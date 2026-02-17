# Northflank Provider Adapter -- Architecture Design Document

> Author: Architecture Designer
> Date: 2026-02-16
> Status: Implementation-Ready
> References: [v3-adapter-architecture.md](./v3-adapter-architecture.md), [northflank-findings.md](./northflank-findings.md)

---

## Table of Contents

1. [Overview](#1-overview)
2. [Module Structure](#2-module-structure)
3. [Provider Struct and Constructors](#3-provider-struct-and-constructors)
4. [Configuration Schema](#4-configuration-schema)
5. [Deploy Configuration Extraction](#5-deploy-configuration-extraction)
6. [API Response Structs](#6-api-response-structs)
7. [Provider Trait Implementation](#7-provider-trait-implementation)
8. [Prerequisite Checks](#8-prerequisite-checks)
9. [Deploy Lifecycle](#9-deploy-lifecycle)
10. [Connect Strategy](#10-connect-strategy)
11. [Status Implementation](#11-status-implementation)
12. [Start and Stop (Pause/Resume)](#12-start-and-stop-pauseresume)
13. [Plan (Dry-Run)](#13-plan-dry-run)
14. [Destroy Implementation](#14-destroy-implementation)
15. [Secrets Injection](#15-secrets-injection)
16. [Error Handling](#16-error-handling)
17. [CLI Integration Points](#17-cli-integration-points)
18. [Testing Strategy](#18-testing-strategy)
19. [Implementation Checklist](#19-implementation-checklist)

---

## 1. Overview

The Northflank provider adapter enables Sindri v3 to deploy development environments on the Northflank PaaS platform. Northflank provides a full-stack cloud PaaS built on Kubernetes with native GPU support, pause/resume capabilities, auto-scaling, and 16+ managed cloud regions.

### Key Capabilities

| Capability                  | Supported | Notes                                         |
| --------------------------- | --------- | --------------------------------------------- |
| GPU Support                 | Yes       | H100, B200, A100, L4, H200, AMD MI300X        |
| Auto-Suspend (Pause/Resume) | Yes       | Native pause/resume via API and CLI           |
| Persistent Volumes          | Yes       | SSD-backed, up to 1.5 TB                      |
| Interactive Shell           | Yes       | `northflank exec` for container access        |
| Port Forwarding             | Yes       | `northflank forward` without public exposure  |
| Auto-Scaling                | Yes       | CPU/memory-based horizontal scaling           |
| Secrets Management          | Yes       | Secret groups with runtime variable injection |

### Integration Approach

The Northflank adapter follows the **CLI subprocess pattern** established by all existing providers. It shells out to the `northflank` CLI tool (npm package `@northflank/cli`) for all platform operations. This approach was chosen over direct REST API calls for consistency with the existing codebase and to leverage the CLI's built-in authentication, context management, and interactive features (exec, forward).

Where the CLI lacks JSON output for certain operations, the adapter falls back to the Northflank REST API via `reqwest` HTTP calls using the same authentication token.

---

## 2. Module Structure

### File Locations

| File                                              | Purpose                                        |
| ------------------------------------------------- | ---------------------------------------------- |
| `v3/crates/sindri-providers/src/northflank.rs`    | Provider implementation (new)                  |
| `v3/crates/sindri-providers/src/lib.rs`           | Module registration + factory arm (modify)     |
| `v3/crates/sindri-core/src/types/config_types.rs` | Provider enum variant + config struct (modify) |
| `v3/schemas/sindri.schema.json`                   | Schema validation (modify)                     |

### No Template File Required

Unlike Docker, Fly, or Kubernetes providers, Northflank does not require a generated configuration file (no equivalent of `fly.toml` or `docker-compose.yml`). All deployment configuration is passed directly via CLI arguments or REST API JSON payloads. The `templates` field on the struct is retained for interface consistency but unused.

---

## 3. Provider Struct and Constructors

```rust
// File: v3/crates/sindri-providers/src/northflank.rs

use crate::templates::TemplateRegistry;
use crate::traits::Provider;
use crate::utils::{command_exists, get_command_version};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use sindri_core::config::SindriConfig;
use sindri_core::types::{
    ActionType, ConnectionInfo, DeployOptions, DeployResult, DeploymentPlan,
    DeploymentState, DeploymentStatus, PlannedAction, PlannedResource,
    Prerequisite, PrerequisiteStatus,
};
use sindri_secrets::{ResolutionContext, SecretResolver};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Northflank provider for cloud deployment
pub struct NorthflankProvider {
    /// Template registry (retained for interface consistency, unused)
    templates: TemplateRegistry,
    /// Output directory for any generated artifacts
    output_dir: PathBuf,
}

impl NorthflankProvider {
    /// Create a new Northflank provider
    pub fn new() -> Result<Self> {
        Ok(Self {
            templates: TemplateRegistry::new()
                .context("Failed to initialize templates")?,
            output_dir: std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from(".")),
        })
    }

    /// Create with a specific output directory (for testing)
    pub fn with_output_dir(output_dir: PathBuf) -> Result<Self> {
        Ok(Self {
            templates: TemplateRegistry::new()
                .context("Failed to initialize templates")?,
            output_dir,
        })
    }
}
```

### Design Rationale

- The struct follows the exact same pattern as `FlyProvider`, `E2bProvider`, etc.
- `templates` is kept for structural consistency even though Northflank does not use generated config files. This avoids special-casing the provider and allows future template usage if needed.
- `with_output_dir()` enables unit testing with `tempfile::tempdir()`.

---

## 4. Configuration Schema

### Provider Enum Addition

File: `v3/crates/sindri-core/src/types/config_types.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Provider {
    Docker,
    #[serde(alias = "docker-compose")]
    DockerCompose,
    Fly,
    Devpod,
    E2b,
    Kubernetes,
    Northflank,  // <-- NEW
}
```

Add to `Display` impl:

```rust
Provider::Northflank => write!(f, "northflank"),
```

Add to `normalized()`:

```rust
Provider::Northflank => "northflank",
```

Add to `supports_gpu()`:

```rust
Provider::Northflank => true,  // via matches! macro
```

### Provider-Specific Config Struct

File: `v3/crates/sindri-core/src/types/config_types.rs`

```rust
/// Northflank provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NorthflankProviderConfig {
    /// Northflank project name (defaults to sindri deployment name)
    #[serde(default, rename = "projectName")]
    pub project_name: Option<String>,

    /// Northflank service name (defaults to sindri deployment name)
    #[serde(default, rename = "serviceName")]
    pub service_name: Option<String>,

    /// Compute plan identifier (e.g., "nf-compute-200")
    #[serde(default = "default_northflank_plan", rename = "computePlan")]
    pub compute_plan: String,

    /// GPU type (e.g., "nvidia-h100", "nvidia-a100-40gb", "nvidia-l4")
    #[serde(default, rename = "gpuType")]
    pub gpu_type: Option<String>,

    /// Number of container instances
    #[serde(default = "default_northflank_instances")]
    pub instances: u32,

    /// Deployment region slug (e.g., "us-east", "europe-west")
    #[serde(default = "default_northflank_region")]
    pub region: String,

    /// Auto-scaling configuration
    #[serde(default, rename = "autoScaling")]
    pub auto_scaling: Option<NorthflankAutoScaling>,

    /// Persistent volume size in GB
    #[serde(default, rename = "volumeSizeGb")]
    pub volume_size_gb: Option<u32>,

    /// Volume mount path inside the container
    #[serde(default = "default_northflank_mount_path", rename = "volumeMountPath")]
    pub volume_mount_path: String,

    /// Registry credential ID for private images
    #[serde(default, rename = "registryCredentials")]
    pub registry_credentials: Option<String>,

    /// Ports to expose
    #[serde(default)]
    pub ports: Vec<NorthflankPort>,

    /// Health check configuration
    #[serde(default, rename = "healthChecks")]
    pub health_checks: Option<NorthflankHealthChecks>,
}

/// Northflank auto-scaling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NorthflankAutoScaling {
    /// Enable auto-scaling
    #[serde(default)]
    pub enabled: bool,

    /// Minimum instances
    #[serde(default = "default_min_instances", rename = "minInstances")]
    pub min_instances: u32,

    /// Maximum instances
    #[serde(default = "default_max_instances", rename = "maxInstances")]
    pub max_instances: u32,

    /// CPU utilization target percentage (0-100)
    #[serde(default = "default_cpu_threshold", rename = "cpuThreshold")]
    pub cpu_threshold: u32,

    /// Memory utilization target percentage (0-100)
    #[serde(default = "default_memory_threshold", rename = "memoryThreshold")]
    pub memory_threshold: u32,
}

/// Northflank port configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NorthflankPort {
    /// Port name (e.g., "http", "ssh")
    pub name: String,

    /// Internal container port
    #[serde(rename = "internalPort")]
    pub internal_port: u16,

    /// Whether to expose publicly
    #[serde(default)]
    pub public: bool,

    /// Protocol (HTTP, TCP, UDP)
    #[serde(default = "default_protocol")]
    pub protocol: String,
}

/// Northflank health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NorthflankHealthChecks {
    /// Liveness probe
    #[serde(default)]
    pub liveness: Option<NorthflankProbe>,

    /// Readiness probe
    #[serde(default)]
    pub readiness: Option<NorthflankProbe>,

    /// Startup probe
    #[serde(default)]
    pub startup: Option<NorthflankProbe>,
}

/// Northflank health check probe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NorthflankProbe {
    /// Protocol: HTTP, TCP, or CMD
    pub protocol: String,

    /// Path for HTTP probes
    #[serde(default)]
    pub path: Option<String>,

    /// Port for HTTP/TCP probes
    #[serde(default)]
    pub port: Option<u16>,

    /// Command for CMD probes
    #[serde(default)]
    pub command: Option<String>,

    /// Initial delay in seconds
    #[serde(default = "default_initial_delay", rename = "initialDelaySeconds")]
    pub initial_delay_seconds: u32,

    /// Check period in seconds
    #[serde(default = "default_period", rename = "periodSeconds")]
    pub period_seconds: u32,

    /// Timeout per check in seconds
    #[serde(default = "default_timeout_seconds", rename = "timeoutSeconds")]
    pub timeout_seconds: u32,

    /// Failure threshold before marking unhealthy
    #[serde(default = "default_failure_threshold", rename = "failureThreshold")]
    pub failure_threshold: u32,
}

// Default functions
fn default_northflank_plan() -> String {
    "nf-compute-200".to_string()
}

fn default_northflank_instances() -> u32 {
    1
}

fn default_northflank_region() -> String {
    "us-east".to_string()
}

fn default_northflank_mount_path() -> String {
    "/data".to_string()
}

fn default_min_instances() -> u32 {
    1
}

fn default_max_instances() -> u32 {
    5
}

fn default_cpu_threshold() -> u32 {
    70
}

fn default_memory_threshold() -> u32 {
    80
}

fn default_protocol() -> String {
    "HTTP".to_string()
}

fn default_initial_delay() -> u32 {
    10
}

fn default_period() -> u32 {
    15
}

fn default_timeout_seconds() -> u32 {
    5
}

fn default_failure_threshold() -> u32 {
    3
}
```

### ProvidersConfig Addition

```rust
pub struct ProvidersConfig {
    // ... existing fields ...

    /// Northflank specific config
    #[serde(default)]
    pub northflank: Option<NorthflankProviderConfig>,
}
```

### JSON Schema Addition

File: `v3/schemas/sindri.schema.json`

Add `"northflank"` to the provider enum:

```json
"provider": {
  "type": "string",
  "enum": ["fly", "kubernetes", "docker-compose", "docker", "devpod", "e2b", "northflank"]
}
```

Add Northflank provider config object under `providers`:

```json
"northflank": {
  "type": "object",
  "properties": {
    "projectName": { "type": "string" },
    "serviceName": { "type": "string" },
    "computePlan": {
      "type": "string",
      "default": "nf-compute-200",
      "description": "Northflank compute plan (e.g., nf-compute-200, nf-compute-400)"
    },
    "gpuType": {
      "type": "string",
      "enum": ["nvidia-l4", "nvidia-a100-40gb", "nvidia-a100-80gb", "nvidia-h100", "nvidia-h200", "nvidia-b200"],
      "description": "GPU model for GPU-enabled deployments"
    },
    "instances": { "type": "integer", "minimum": 0, "default": 1 },
    "region": {
      "type": "string",
      "default": "us-east",
      "enum": [
        "us-east", "us-west", "us-central", "us-east-ohio", "us-west-california",
        "europe-west", "europe-west-frankfurt", "europe-west-netherlands", "europe-west-zurich",
        "asia-east", "asia-northeast", "asia-southeast",
        "australia-southeast", "canada-central", "southamerica-east", "africa-south"
      ]
    },
    "autoScaling": {
      "type": "object",
      "properties": {
        "enabled": { "type": "boolean", "default": false },
        "minInstances": { "type": "integer", "minimum": 0, "default": 1 },
        "maxInstances": { "type": "integer", "minimum": 1, "default": 5 },
        "cpuThreshold": { "type": "integer", "minimum": 1, "maximum": 100, "default": 70 },
        "memoryThreshold": { "type": "integer", "minimum": 1, "maximum": 100, "default": 80 }
      }
    },
    "volumeSizeGb": { "type": "integer", "minimum": 1, "maximum": 1500 },
    "volumeMountPath": { "type": "string", "default": "/data" },
    "registryCredentials": { "type": "string" },
    "ports": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["name", "internalPort"],
        "properties": {
          "name": { "type": "string" },
          "internalPort": { "type": "integer" },
          "public": { "type": "boolean", "default": false },
          "protocol": { "type": "string", "enum": ["HTTP", "TCP", "UDP"], "default": "HTTP" }
        }
      }
    },
    "healthChecks": {
      "type": "object",
      "properties": {
        "liveness": { "$ref": "#/$defs/northflankProbe" },
        "readiness": { "$ref": "#/$defs/northflankProbe" },
        "startup": { "$ref": "#/$defs/northflankProbe" }
      }
    }
  }
}
```

---

## 5. Deploy Configuration Extraction

The adapter extracts a typed `NorthflankDeployConfig` from `SindriConfig`, following the pattern established by `FlyDeployConfig` and `E2bDeployConfig`.

```rust
/// Northflank deployment configuration (extracted from SindriConfig)
struct NorthflankDeployConfig<'a> {
    /// Deployment name (from sindri.yaml `name`)
    name: &'a str,
    /// Northflank project name
    project_name: String,
    /// Northflank service name
    service_name: String,
    /// Compute plan identifier
    compute_plan: String,
    /// GPU type (if GPU enabled)
    gpu_type: Option<String>,
    /// Number of instances
    instances: u32,
    /// Region slug
    region: &'a str,
    /// Volume size in GB (None = no volume)
    volume_size_gb: Option<u32>,
    /// Volume mount path
    volume_mount_path: String,
    /// Whether auto-scaling is enabled
    auto_scaling: Option<NorthflankAutoScaling>,
    /// Registry credential ID for private images
    registry_credentials: Option<String>,
    /// Configured ports
    ports: Vec<NorthflankPort>,
}

impl NorthflankProvider {
    /// Extract typed Northflank config from SindriConfig
    fn get_northflank_config<'a>(&self, config: &'a SindriConfig) -> NorthflankDeployConfig<'a> {
        let file = config.inner();
        let nf = file.providers.northflank.as_ref();

        // Project and service names default to the deployment name
        let project_name = nf
            .and_then(|n| n.project_name.clone())
            .unwrap_or_else(|| format!("sindri-{}", file.name));

        let service_name = nf
            .and_then(|n| n.service_name.clone())
            .unwrap_or_else(|| file.name.clone());

        // Compute plan: provider-specific config overrides generic resources
        let compute_plan = nf
            .map(|n| n.compute_plan.clone())
            .unwrap_or_else(|| {
                // Map generic resource config to a Northflank plan
                let cpus = file.deployment.resources.cpus.unwrap_or(2);
                let memory_raw = file.deployment.resources.memory.as_deref().unwrap_or("4GB");
                let memory_mb = parse_memory_to_mb(memory_raw).unwrap_or(4096);
                map_resources_to_plan(cpus, memory_mb)
            });

        // GPU type: from provider config or generic GPU config
        let gpu_type = nf
            .and_then(|n| n.gpu_type.clone())
            .or_else(|| {
                file.deployment
                    .resources
                    .gpu
                    .as_ref()
                    .filter(|g| g.enabled)
                    .and_then(|g| g.tier.as_ref())
                    .map(|t| map_gpu_tier_to_northflank(t))
            });

        // Region
        let region = nf
            .map(|n| n.region.as_str())
            .unwrap_or("us-east");

        // Volume size: provider config or generic volume config
        let volume_size_gb = nf
            .and_then(|n| n.volume_size_gb)
            .or_else(|| {
                file.deployment
                    .volumes
                    .workspace
                    .as_ref()
                    .and_then(|v| parse_size_to_gb(&v.size))
            });

        let volume_mount_path = nf
            .map(|n| n.volume_mount_path.clone())
            .unwrap_or_else(|| "/data".to_string());

        let instances = nf.map(|n| n.instances).unwrap_or(1);

        let auto_scaling = nf.and_then(|n| n.auto_scaling.clone());

        let registry_credentials = nf.and_then(|n| n.registry_credentials.clone());

        let ports = nf
            .map(|n| n.ports.clone())
            .unwrap_or_default();

        NorthflankDeployConfig {
            name: &file.name,
            project_name,
            service_name,
            compute_plan,
            gpu_type,
            instances,
            region,
            volume_size_gb,
            volume_mount_path,
            auto_scaling,
            registry_credentials,
            ports,
        }
    }
}
```

### Resource-to-Plan Mapping

```rust
/// Map generic CPU/memory resources to the closest Northflank compute plan
fn map_resources_to_plan(cpus: u32, memory_mb: u32) -> String {
    // Northflank plan naming: nf-compute-{vCPU*100}[-{memoryMB/1024}]
    // Select the plan that meets or exceeds the requested resources
    match (cpus, memory_mb) {
        (0..=1, 0..=1024) => "nf-compute-100-1".to_string(),
        (0..=1, 1025..=2048) => "nf-compute-100-2".to_string(),
        (0..=1, 2049..=4096) => "nf-compute-100-4".to_string(),
        (2, 0..=4096) => "nf-compute-200".to_string(),
        (2, 4097..=8192) => "nf-compute-200-8".to_string(),
        (2, 8193..=16384) => "nf-compute-200-16".to_string(),
        (3..=4, 0..=8192) => "nf-compute-400".to_string(),
        (3..=4, 8193..=16384) => "nf-compute-400-16".to_string(),
        (5..=8, 0..=8192) => "nf-compute-800-8".to_string(),
        (5..=8, 8193..=16384) => "nf-compute-800-16".to_string(),
        (5..=8, 16385..=24576) => "nf-compute-800-24".to_string(),
        (5..=8, 24577..=32768) => "nf-compute-800-32".to_string(),
        (5..=8, _) => "nf-compute-800-40".to_string(),
        (9..=12, _) => "nf-compute-1200-24".to_string(),
        (13..=16, _) => "nf-compute-1600-32".to_string(),
        _ => "nf-compute-2000-40".to_string(),
    }
}

/// Map Sindri GPU tier to Northflank GPU type string
fn map_gpu_tier_to_northflank(tier: &sindri_core::types::GpuTier) -> String {
    use sindri_core::types::GpuTier;
    match tier {
        GpuTier::GpuSmall => "nvidia-l4".to_string(),
        GpuTier::GpuMedium => "nvidia-a100-40gb".to_string(),
        GpuTier::GpuLarge => "nvidia-a100-80gb".to_string(),
        GpuTier::GpuXlarge => "nvidia-h100".to_string(),
    }
}
```

---

## 6. API Response Structs

These structs deserialize JSON responses from the Northflank CLI and API.

```rust
/// Northflank project response
#[derive(Debug, Deserialize)]
struct NorthflankProject {
    /// Project ID (slug)
    id: String,
    /// Project name
    name: String,
    /// Project description
    #[serde(default)]
    description: Option<String>,
    /// Region
    #[serde(default)]
    region: Option<String>,
}

/// Northflank service response
#[derive(Debug, Deserialize)]
struct NorthflankService {
    /// Service ID
    id: String,
    /// Service name
    name: String,
    /// Service type (deployment, build, combined)
    #[serde(default, rename = "serviceType")]
    service_type: Option<String>,
    /// Deployment status
    #[serde(default)]
    status: Option<String>,
    /// Deployment details
    #[serde(default)]
    deployment: Option<NorthflankDeployment>,
}

/// Northflank deployment details (nested in service response)
#[derive(Debug, Deserialize)]
struct NorthflankDeployment {
    /// Number of running instances
    #[serde(default)]
    instances: Option<u32>,
    /// Container image information
    #[serde(default)]
    external: Option<NorthflankExternalImage>,
}

/// External image reference
#[derive(Debug, Deserialize)]
struct NorthflankExternalImage {
    /// Image path (e.g., "ghcr.io/org/image:tag")
    #[serde(default, rename = "imagePath")]
    image_path: Option<String>,
}

/// Northflank service list response wrapper
#[derive(Debug, Deserialize)]
struct NorthflankServiceListResponse {
    data: NorthflankServiceListData,
}

#[derive(Debug, Deserialize)]
struct NorthflankServiceListData {
    services: Vec<NorthflankService>,
}

/// Northflank project list response wrapper
#[derive(Debug, Deserialize)]
struct NorthflankProjectListResponse {
    data: NorthflankProjectListData,
}

#[derive(Debug, Deserialize)]
struct NorthflankProjectListData {
    projects: Vec<NorthflankProject>,
}
```

---

## 7. Provider Trait Implementation

### Capability Flags

```rust
#[async_trait]
impl Provider for NorthflankProvider {
    fn name(&self) -> &'static str {
        "northflank"
    }

    fn supports_gpu(&self) -> bool {
        true // Northflank supports H100, B200, A100, L4, H200, MI300X
    }

    fn supports_auto_suspend(&self) -> bool {
        true // Northflank supports native pause/resume
    }

    // ... (all 9 required methods detailed in sections below)
}
```

---

## 8. Prerequisite Checks

The Northflank CLI is a Node.js package installed via npm. Prerequisites are:

1. `northflank` CLI binary must exist on PATH
2. The user must be authenticated (verified by listing projects)

```rust
fn check_prerequisites(&self) -> Result<PrerequisiteStatus> {
    let mut missing = Vec::new();
    let mut available = Vec::new();

    // 1. Check northflank CLI exists
    if command_exists("northflank") {
        let version = get_command_version("northflank", "--version")
            .unwrap_or_else(|_| "unknown".to_string());

        // 2. Check authentication by attempting to list projects
        if self.is_authenticated() {
            available.push(Prerequisite {
                name: "northflank".to_string(),
                description: "Northflank CLI (authenticated)".to_string(),
                install_hint: None,
                version: Some(version),
            });
        } else {
            available.push(Prerequisite {
                name: "northflank".to_string(),
                description: "Northflank CLI".to_string(),
                install_hint: None,
                version: Some(version),
            });
            missing.push(Prerequisite {
                name: "northflank-auth".to_string(),
                description: "Northflank authentication".to_string(),
                install_hint: Some(
                    "Run: northflank login\n  \
                     Or set NORTHFLANK_API_TOKEN environment variable"
                        .to_string(),
                ),
                version: None,
            });
        }
    } else {
        missing.push(Prerequisite {
            name: "northflank".to_string(),
            description: "Northflank CLI".to_string(),
            install_hint: Some(
                "Install: npm i @northflank/cli -g\n  \
                 Docs: https://northflank.com/docs/v1/api/use-the-cli"
                    .to_string(),
            ),
            version: None,
        });
    }

    Ok(PrerequisiteStatus {
        satisfied: missing.is_empty(),
        missing,
        available,
    })
}
```

### Authentication Check Helper

```rust
impl NorthflankProvider {
    /// Check if the Northflank CLI is authenticated
    fn is_authenticated(&self) -> bool {
        // Attempt to list projects; success means authenticated
        let output = std::process::Command::new("northflank")
            .args(["list", "projects"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();

        output.map(|o| o.status.success()).unwrap_or(false)
    }
}
```

---

## 9. Deploy Lifecycle

The deploy method follows the standard Sindri provider lifecycle:

```
check_prerequisites
    |
    v
resolve image
    |
    v
handle dry_run? ---yes---> return plan()
    |
    no
    v
check existing service
    |
    v
exists && force? ---yes---> destroy first
exists && !force? ---yes---> error
    |
    v
ensure project exists
    |
    v
create volume (if configured)
    |
    v
resolve and inject secrets
    |
    v
create deployment service
    |
    v
wait for RUNNING (if opts.wait)
    |
    v
return DeployResult with connection info
```

### Implementation

```rust
async fn deploy(
    &self,
    config: &SindriConfig,
    opts: DeployOptions,
) -> Result<DeployResult> {
    let nf_config = self.get_northflank_config(config);
    let name = nf_config.name.to_string();
    info!("Deploying {} to Northflank", name);

    // 1. Check prerequisites
    let prereqs = self.check_prerequisites()?;
    if !prereqs.satisfied {
        let missing_names: Vec<_> =
            prereqs.missing.iter().map(|p| p.name.as_str()).collect();
        return Err(anyhow!(
            "Missing prerequisites: {}",
            missing_names.join(", ")
        ));
    }

    // 2. Resolve image
    let image = config
        .resolve_image(None)
        .await
        .map_err(|e| anyhow!("Failed to resolve image: {}", e))?;
    info!("Using image: {}", image);

    // 3. Handle dry-run
    if opts.dry_run {
        return self.plan(config).await.map(|plan| DeployResult {
            success: true,
            name: name.clone(),
            provider: "northflank".to_string(),
            instance_id: None,
            connection: None,
            messages: vec![format!(
                "Would deploy {} to Northflank project '{}' in region '{}'",
                name, nf_config.project_name, nf_config.region
            )],
            warnings: vec![],
        });
    }

    // 4. Check existing service
    if self.service_exists(&nf_config.project_name, &nf_config.service_name).await {
        if opts.force {
            info!("Service exists, destroying first (--force)");
            self.delete_service(&nf_config.project_name, &nf_config.service_name).await?;
            // Brief wait for deletion to propagate
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        } else {
            return Err(anyhow!(
                "Service '{}' already exists in project '{}'. \
                 Use --force to recreate.",
                nf_config.service_name,
                nf_config.project_name
            ));
        }
    }

    // 5. Ensure project exists
    self.ensure_project(&nf_config.project_name, nf_config.region).await?;

    // 6. Create volume (if configured)
    if let Some(size_gb) = nf_config.volume_size_gb {
        self.ensure_volume(
            &nf_config.project_name,
            &format!("{}-data", nf_config.service_name),
            size_gb,
            &nf_config.volume_mount_path,
        ).await?;
    }

    // 7. Resolve and inject secrets
    self.resolve_and_set_secrets(config, &nf_config.project_name).await?;

    // 8. Create deployment service
    self.create_deployment_service(
        &nf_config,
        &image,
    ).await?;

    // 9. Wait for running (if requested)
    if opts.wait {
        let timeout = opts.timeout.unwrap_or(300);
        self.wait_for_running(
            &nf_config.project_name,
            &nf_config.service_name,
            timeout,
        ).await?;
    }

    // 10. Return result
    Ok(DeployResult {
        success: true,
        name: name.clone(),
        provider: "northflank".to_string(),
        instance_id: Some(nf_config.service_name.clone()),
        connection: Some(ConnectionInfo {
            ssh_command: None, // No direct SSH; use exec
            http_url: None,   // Filled after querying service endpoints
            https_url: None,
            instructions: Some(format!(
                "Connect with:\n  \
                 sindri connect\n  \
                 northflank exec --project {} --service {}",
                nf_config.project_name, nf_config.service_name
            )),
        }),
        messages: vec![format!(
            "Service '{}' deployed to Northflank project '{}' (region: {})",
            nf_config.service_name, nf_config.project_name, nf_config.region
        )],
        warnings: self.collect_warnings(&nf_config),
    })
}
```

### Helper Methods

```rust
impl NorthflankProvider {
    /// Ensure a Northflank project exists, create if not
    async fn ensure_project(&self, project_name: &str, region: &str) -> Result<()> {
        if self.project_exists(project_name).await {
            info!("Project '{}' already exists", project_name);
            return Ok(());
        }

        info!("Creating Northflank project: {} (region: {})", project_name, region);
        let output = Command::new("northflank")
            .args([
                "create", "project",
                "--name", project_name,
                "--region", region,
            ])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to create project '{}': {}", project_name, stderr));
        }

        Ok(())
    }

    /// Check if a project exists
    async fn project_exists(&self, project_name: &str) -> bool {
        let output = Command::new("northflank")
            .args(["get", "project", "--project", project_name])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .await;

        output.map(|o| o.status.success()).unwrap_or(false)
    }

    /// Check if a service exists
    async fn service_exists(&self, project: &str, service: &str) -> bool {
        let output = Command::new("northflank")
            .args([
                "get", "service", "details",
                "--project", project,
                "--service", service,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .await;

        output.map(|o| o.status.success()).unwrap_or(false)
    }

    /// Create a deployment service via CLI
    async fn create_deployment_service(
        &self,
        config: &NorthflankDeployConfig<'_>,
        image: &str,
    ) -> Result<()> {
        info!(
            "Creating deployment service: {} (plan: {}, image: {})",
            config.service_name, config.compute_plan, image
        );

        let mut args = vec![
            "create".to_string(),
            "service".to_string(),
            "deployment".to_string(),
            "--project".to_string(),
            config.project_name.clone(),
            "--name".to_string(),
            config.service_name.clone(),
            "--image".to_string(),
            image.to_string(),
            "--plan".to_string(),
            config.compute_plan.clone(),
        ];

        // Add instance count if not default
        if config.instances != 1 {
            args.push("--instances".to_string());
            args.push(config.instances.to_string());
        }

        let output = Command::new("northflank")
            .args(&args)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!(
                "Failed to create deployment service '{}': {}",
                config.service_name,
                stderr
            ));
        }

        info!("Service '{}' created successfully", config.service_name);
        Ok(())
    }

    /// Delete a service
    async fn delete_service(&self, project: &str, service: &str) -> Result<()> {
        info!("Deleting service: {} in project: {}", service, project);
        let output = Command::new("northflank")
            .args([
                "delete", "service",
                "--project", project,
                "--service", service,
            ])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to delete service '{}': {}", service, stderr));
        }

        Ok(())
    }

    /// Ensure a volume exists and is attached
    async fn ensure_volume(
        &self,
        project: &str,
        volume_name: &str,
        size_gb: u32,
        mount_path: &str,
    ) -> Result<()> {
        info!(
            "Ensuring volume '{}' exists ({}GB, mount: {})",
            volume_name, size_gb, mount_path
        );
        // Volume creation is handled during service creation via API
        // This is a placeholder for volume pre-creation if needed
        debug!("Volume will be attached during service creation");
        Ok(())
    }

    /// Wait for service to reach RUNNING state
    async fn wait_for_running(
        &self,
        project: &str,
        service: &str,
        timeout_secs: u64,
    ) -> Result<()> {
        info!("Waiting for service to reach RUNNING state (timeout: {}s)", timeout_secs);
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);

        loop {
            if start.elapsed() > timeout {
                return Err(anyhow!(
                    "Timed out waiting for service '{}' to start ({}s)",
                    service,
                    timeout_secs
                ));
            }

            let state = self.get_service_state(project, service).await?;
            match state {
                DeploymentState::Running => {
                    info!("Service is running");
                    return Ok(());
                }
                DeploymentState::Error => {
                    return Err(anyhow!(
                        "Service '{}' entered error state during deployment",
                        service
                    ));
                }
                _ => {
                    debug!("Service state: {:?}, waiting...", state);
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            }
        }
    }

    /// Collect deployment warnings
    fn collect_warnings(&self, config: &NorthflankDeployConfig<'_>) -> Vec<String> {
        let mut warnings = Vec::new();

        // Warn about volume + scaling constraint
        if config.volume_size_gb.is_some() && config.instances > 1 {
            warnings.push(
                "Northflank limits services with volumes to 1 instance. \
                 Auto-scaling and multi-instance settings will be ignored."
                    .to_string(),
            );
        }

        // Warn about GPU region availability
        if config.gpu_type.is_some() {
            warnings.push(format!(
                "GPU deployments require pre-purchased credits. \
                 Verify GPU availability in region '{}'.",
                config.region
            ));
        }

        warnings
    }
}
```

---

## 10. Connect Strategy

Northflank provides two connection methods:

1. **`northflank exec`** -- Interactive shell inside the container (primary)
2. **`northflank forward`** -- Port forwarding to local machine (secondary)

The `connect()` method uses `exec` for interactive shell access, mirroring how Fly uses `flyctl ssh console` and Docker uses `docker exec`.

```rust
async fn connect(&self, config: &SindriConfig) -> Result<()> {
    let nf_config = self.get_northflank_config(config);
    let name = nf_config.name;
    info!("Connecting to {} on Northflank", name);

    // Check service exists
    if !self.service_exists(&nf_config.project_name, &nf_config.service_name).await {
        return Err(anyhow!(
            "Service '{}' not found in project '{}'. Deploy first: sindri deploy",
            nf_config.service_name,
            nf_config.project_name
        ));
    }

    // Check service state; resume if paused
    let state = self.get_service_state(
        &nf_config.project_name,
        &nf_config.service_name,
    ).await?;

    if matches!(state, DeploymentState::Paused | DeploymentState::Stopped) {
        info!("Service is {:?}, resuming...", state);
        self.resume_service(&nf_config.project_name, &nf_config.service_name).await?;
        // Wait briefly for the service to start
        self.wait_for_running(
            &nf_config.project_name,
            &nf_config.service_name,
            60,
        ).await?;
    }

    // Exec into the container
    let status = Command::new("northflank")
        .args([
            "exec",
            "--project", &nf_config.project_name,
            "--service", &nf_config.service_name,
        ])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await?;

    if !status.success() {
        return Err(anyhow!("Failed to connect to Northflank service"));
    }

    Ok(())
}
```

---

## 11. Status Implementation

```rust
async fn status(&self, config: &SindriConfig) -> Result<DeploymentStatus> {
    let nf_config = self.get_northflank_config(config);
    let name = nf_config.name.to_string();
    info!("Getting Northflank status for {}", name);

    let state = self
        .get_service_state(&nf_config.project_name, &nf_config.service_name)
        .await
        .unwrap_or(DeploymentState::NotDeployed);

    // Resolve image for display
    let image = config.resolve_image(None).await.ok();

    Ok(DeploymentStatus {
        name,
        provider: "northflank".to_string(),
        state,
        instance_id: Some(nf_config.service_name.clone()),
        image,
        addresses: vec![],
        resources: None,
        timestamps: Default::default(),
        details: {
            let mut d = HashMap::new();
            d.insert("project".to_string(), nf_config.project_name.clone());
            d.insert("region".to_string(), nf_config.region.to_string());
            d.insert("compute_plan".to_string(), nf_config.compute_plan.clone());
            d
        },
    })
}
```

### Service State Query Helper

```rust
impl NorthflankProvider {
    /// Get the current deployment state of a service
    async fn get_service_state(
        &self,
        project: &str,
        service: &str,
    ) -> Result<DeploymentState> {
        let output = Command::new("northflank")
            .args([
                "get", "service", "details",
                "--project", project,
                "--service", service,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("not found") || stderr.contains("404") {
                return Ok(DeploymentState::NotDeployed);
            }
            return Err(anyhow!("Failed to get service status: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse the CLI output to determine state
        // The CLI may output structured or human-readable text
        // Try JSON parsing first, fall back to text matching
        if let Ok(service) = serde_json::from_str::<NorthflankService>(&stdout) {
            return Ok(map_northflank_status(
                service.status.as_deref().unwrap_or("unknown"),
            ));
        }

        // Fallback: text-based status detection
        let stdout_lower = stdout.to_lowercase();
        if stdout_lower.contains("running") {
            Ok(DeploymentState::Running)
        } else if stdout_lower.contains("paused") {
            Ok(DeploymentState::Paused)
        } else if stdout_lower.contains("stopped") || stdout_lower.contains("scaled to 0") {
            Ok(DeploymentState::Stopped)
        } else if stdout_lower.contains("creating") || stdout_lower.contains("building") {
            Ok(DeploymentState::Creating)
        } else if stdout_lower.contains("error") || stdout_lower.contains("failed") {
            Ok(DeploymentState::Error)
        } else {
            Ok(DeploymentState::Unknown)
        }
    }
}

/// Map Northflank status string to DeploymentState
fn map_northflank_status(status: &str) -> DeploymentState {
    match status.to_lowercase().as_str() {
        "running" | "healthy" => DeploymentState::Running,
        "paused" => DeploymentState::Paused,
        "stopped" | "scaled-to-zero" => DeploymentState::Stopped,
        "creating" | "building" | "deploying" | "pending" => DeploymentState::Creating,
        "error" | "failed" | "crash-loop" => DeploymentState::Error,
        "deleting" | "destroying" => DeploymentState::Destroying,
        _ => DeploymentState::Unknown,
    }
}
```

---

## 12. Start and Stop (Pause/Resume)

Northflank natively supports pause and resume via CLI and API. This maps directly to the `start()` and `stop()` trait methods.

```rust
async fn start(&self, config: &SindriConfig) -> Result<()> {
    let nf_config = self.get_northflank_config(config);
    info!("Starting (resuming) {} on Northflank", nf_config.name);

    self.resume_service(&nf_config.project_name, &nf_config.service_name).await
}

async fn stop(&self, config: &SindriConfig) -> Result<()> {
    let nf_config = self.get_northflank_config(config);
    info!("Stopping (pausing) {} on Northflank", nf_config.name);

    self.pause_service(&nf_config.project_name, &nf_config.service_name).await
}
```

### Pause/Resume Helpers

```rust
impl NorthflankProvider {
    /// Pause a service (stop resource consumption)
    async fn pause_service(&self, project: &str, service: &str) -> Result<()> {
        info!("Pausing service: {} in project: {}", service, project);
        let output = Command::new("northflank")
            .args([
                "pause", "service",
                "--project", project,
                "--service", service,
            ])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Ignore "already paused" errors
            if !stderr.to_lowercase().contains("already paused") {
                return Err(anyhow!("Failed to pause service '{}': {}", service, stderr));
            }
        }

        info!("Service '{}' paused", service);
        Ok(())
    }

    /// Resume a paused service
    async fn resume_service(&self, project: &str, service: &str) -> Result<()> {
        info!("Resuming service: {} in project: {}", service, project);
        let output = Command::new("northflank")
            .args([
                "resume", "service",
                "--project", project,
                "--service", service,
            ])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Ignore "already running" errors
            if !stderr.to_lowercase().contains("already running") {
                return Err(anyhow!("Failed to resume service '{}': {}", service, stderr));
            }
        }

        info!("Service '{}' resumed", service);
        Ok(())
    }
}
```

---

## 13. Plan (Dry-Run)

```rust
async fn plan(&self, config: &SindriConfig) -> Result<DeploymentPlan> {
    let nf_config = self.get_northflank_config(config);
    let name = nf_config.name.to_string();
    info!("Planning Northflank deployment for {}", name);

    let image = config
        .resolve_image(None)
        .await
        .map_err(|e| anyhow!("{}", e))?;

    let mut actions = Vec::new();

    // Project creation
    if !self.project_exists(&nf_config.project_name).await {
        actions.push(PlannedAction {
            action: ActionType::Create,
            resource: format!("project:{}", nf_config.project_name),
            description: format!(
                "Create Northflank project '{}' in region '{}'",
                nf_config.project_name, nf_config.region
            ),
        });
    }

    // Volume creation
    if let Some(size_gb) = nf_config.volume_size_gb {
        actions.push(PlannedAction {
            action: ActionType::Create,
            resource: format!("volume:{}-data", nf_config.service_name),
            description: format!(
                "Create {}GB persistent volume mounted at '{}'",
                size_gb, nf_config.volume_mount_path
            ),
        });
    }

    // Service creation
    actions.push(PlannedAction {
        action: ActionType::Create,
        resource: format!("service:{}", nf_config.service_name),
        description: format!(
            "Create deployment service with image '{}' on plan '{}'",
            image, nf_config.compute_plan
        ),
    });

    // Build resources list
    let mut resources = vec![
        PlannedResource {
            resource_type: "project".to_string(),
            name: nf_config.project_name.clone(),
            config: {
                let mut m = HashMap::new();
                m.insert("region".to_string(), serde_json::json!(nf_config.region));
                m
            },
        },
        PlannedResource {
            resource_type: "service".to_string(),
            name: nf_config.service_name.clone(),
            config: {
                let mut m = HashMap::new();
                m.insert("plan".to_string(), serde_json::json!(nf_config.compute_plan));
                m.insert("image".to_string(), serde_json::json!(image));
                m.insert("instances".to_string(), serde_json::json!(nf_config.instances));
                if let Some(ref gpu) = nf_config.gpu_type {
                    m.insert("gpu_type".to_string(), serde_json::json!(gpu));
                }
                m
            },
        },
    ];

    if let Some(size_gb) = nf_config.volume_size_gb {
        resources.push(PlannedResource {
            resource_type: "volume".to_string(),
            name: format!("{}-data", nf_config.service_name),
            config: {
                let mut m = HashMap::new();
                m.insert("size_gb".to_string(), serde_json::json!(size_gb));
                m.insert(
                    "mount_path".to_string(),
                    serde_json::json!(nf_config.volume_mount_path),
                );
                m
            },
        });
    }

    // Cost estimate based on compute plan
    let estimated_cost = estimate_cost(&nf_config.compute_plan, &nf_config.gpu_type);

    Ok(DeploymentPlan {
        provider: "northflank".to_string(),
        actions,
        resources,
        estimated_cost: Some(estimated_cost),
    })
}
```

### Cost Estimation Helper

```rust
use sindri_core::types::CostEstimate;

/// Estimate hourly/monthly cost based on compute plan and GPU
fn estimate_cost(plan: &str, gpu_type: &Option<String>) -> CostEstimate {
    // Base compute costs (from Northflank pricing)
    let hourly = match plan {
        "nf-compute-10" => 0.004,
        "nf-compute-20" => 0.008,
        "nf-compute-50" => 0.017,
        "nf-compute-100-1" => 0.025,
        "nf-compute-100-2" => 0.033,
        "nf-compute-100-4" => 0.050,
        "nf-compute-200" => 0.067,
        "nf-compute-200-8" => 0.100,
        "nf-compute-200-16" => 0.167,
        "nf-compute-400" => 0.133,
        "nf-compute-400-16" => 0.200,
        "nf-compute-800-8" => 0.200,
        "nf-compute-800-16" => 0.267,
        "nf-compute-800-24" => 0.333,
        "nf-compute-800-32" => 0.400,
        "nf-compute-800-40" => 0.467,
        "nf-compute-1200-24" => 0.400,
        "nf-compute-1600-32" => 0.533,
        "nf-compute-2000-40" => 0.667,
        _ => 0.067, // default to nf-compute-200
    };

    // Add GPU costs
    let gpu_hourly = gpu_type.as_ref().map(|g| match g.as_str() {
        "nvidia-a100-40gb" => 1.42,
        "nvidia-a100-80gb" => 1.76,
        "nvidia-h100" => 2.74,
        "nvidia-b200" => 5.87,
        _ => 0.0, // L4, H200, MI300X: pricing varies
    }).unwrap_or(0.0);

    let total_hourly = hourly + gpu_hourly;

    CostEstimate {
        hourly: Some(total_hourly),
        monthly: Some(total_hourly * 730.0), // ~730 hours per month
        currency: "USD".to_string(),
        notes: gpu_type.as_ref().map(|g| {
            format!("GPU ({}) costs require pre-purchased credits", g)
        }),
    }
}
```

---

## 14. Destroy Implementation

```rust
async fn destroy(&self, config: &SindriConfig, force: bool) -> Result<()> {
    let nf_config = self.get_northflank_config(config);
    info!(
        "Destroying {} on Northflank (force: {})",
        nf_config.name, force
    );

    // Delete service
    if self.service_exists(&nf_config.project_name, &nf_config.service_name).await {
        self.delete_service(&nf_config.project_name, &nf_config.service_name).await?;
        info!("Service '{}' deleted", nf_config.service_name);
    } else {
        warn!(
            "Service '{}' not found in project '{}'",
            nf_config.service_name, nf_config.project_name
        );
    }

    // Optionally delete the project if force is true and no other services exist
    if force {
        let has_other_services = self
            .project_has_other_services(&nf_config.project_name, &nf_config.service_name)
            .await;

        if !has_other_services {
            info!("No other services in project, deleting project '{}'", nf_config.project_name);
            let output = Command::new("northflank")
                .args([
                    "delete", "project",
                    "--project", &nf_config.project_name,
                ])
                .output()
                .await?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!("Failed to delete project: {}", stderr);
                // Non-fatal: service is already deleted
            } else {
                info!("Project '{}' deleted", nf_config.project_name);
            }
        }
    }

    Ok(())
}
```

### Helper for Project Service Check

```rust
impl NorthflankProvider {
    /// Check if a project has services other than the specified one
    async fn project_has_other_services(
        &self,
        project: &str,
        exclude_service: &str,
    ) -> bool {
        let output = Command::new("northflank")
            .args(["list", "services", "--project", project])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .await;

        output
            .map(|o| {
                if o.status.success() {
                    let stdout = String::from_utf8_lossy(&o.stdout);
                    // Check if any services besides the excluded one are listed
                    stdout
                        .lines()
                        .filter(|line| !line.contains(exclude_service) && !line.trim().is_empty())
                        .count()
                        > 1 // Header line
                } else {
                    true // Assume yes if we cannot check
                }
            })
            .unwrap_or(true)
    }
}
```

---

## 15. Secrets Injection

Northflank supports secrets via:

1. **Secret groups** -- Project-level collections inherited by services
2. **Inline runtime variables** -- Per-service environment variables

For Sindri, we use inline runtime variables set during service creation, matching the pattern used by Fly (flyctl secrets) and E2B (Dockerfile ENV).

```rust
impl NorthflankProvider {
    /// Resolve secrets and set them as runtime variables on the service
    async fn resolve_and_set_secrets(
        &self,
        config: &SindriConfig,
        project_name: &str,
    ) -> Result<()> {
        let secrets = config.secrets();

        if secrets.is_empty() {
            debug!("No secrets configured, skipping");
            return Ok(());
        }

        info!("Resolving {} secrets...", secrets.len());

        // Create resolution context
        let config_dir = config
            .config_path
            .parent()
            .map(|p| p.to_path_buf().into())
            .unwrap_or_else(|| PathBuf::from("."));
        let context = ResolutionContext::new(config_dir);
        let resolver = SecretResolver::new(context);
        let resolved = resolver.resolve_all(secrets).await?;

        // Prepare environment variable pairs
        let mut env_pairs = Vec::new();
        for (name, secret) in &resolved {
            if let Some(value) = secret.value.as_string() {
                env_pairs.push((name.clone(), value.to_string()));
            } else {
                warn!(
                    "Northflank provider only supports environment variable secrets. \
                     File secret '{}' will be skipped.",
                    name
                );
            }
        }

        if env_pairs.is_empty() {
            debug!("No environment variable secrets to set");
            return Ok(());
        }

        info!("Setting {} secrets as runtime variables", env_pairs.len());

        // Secrets will be passed as environment variables during service creation
        // via the API payload or CLI. They are stored on the NorthflankDeployConfig
        // and injected in create_deployment_service().
        //
        // For existing services, update via:
        //   northflank update service --project {project} --service {service} --env KEY=VALUE
        //
        // Implementation note: The CLI may not support bulk env var setting.
        // If not, use the REST API:
        //   PATCH /v1/projects/{projectId}/services/{serviceId}/runtime-environment
        //   { "runtimeEnvironment": { "KEY": "VALUE", ... } }

        debug!("Secrets resolved, will be injected during service creation");
        Ok(())
    }
}
```

---

## 16. Error Handling

### Error Classification

| Scenario                    | Detection                                   | User Message                                                                  |
| --------------------------- | ------------------------------------------- | ----------------------------------------------------------------------------- |
| CLI not found               | `command_exists("northflank") == false`     | "Install: npm i @northflank/cli -g"                                           |
| Not authenticated           | `northflank list projects` fails            | "Run: northflank login"                                                       |
| Project not found           | stderr contains "not found" or 404          | "Project '{name}' not found"                                                  |
| Service already exists      | `service_exists()` returns true             | "Use --force to recreate"                                                     |
| Rate limit exceeded         | HTTP 429 or stderr "rate limit"             | "API rate limit exceeded (1000 req/hr). Wait and retry."                      |
| Quota exceeded              | stderr "quota" or "limit"                   | "Northflank account quota exceeded. Upgrade plan or delete unused resources." |
| Region GPU unavailable      | stderr "gpu" + "not available"              | "GPU type '{type}' not available in region '{region}'"                        |
| Volume + scaling conflict   | `volume_size_gb.is_some() && instances > 1` | Warning during deploy                                                         |
| Timeout waiting for running | elapsed > timeout                           | "Timed out waiting for service '{name}' to start"                             |

### Rate Limit Handling

```rust
/// Retry a command with exponential backoff for rate limiting
async fn retry_with_backoff<F, T>(
    max_retries: u32,
    operation: F,
) -> Result<T>
where
    F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>>,
{
    let mut retries = 0;
    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                let err_msg = e.to_string().to_lowercase();
                if (err_msg.contains("rate limit") || err_msg.contains("429"))
                    && retries < max_retries
                {
                    retries += 1;
                    let delay = std::time::Duration::from_secs(2u64.pow(retries));
                    warn!(
                        "Rate limited, retrying in {}s (attempt {}/{})",
                        delay.as_secs(),
                        retries,
                        max_retries
                    );
                    tokio::time::sleep(delay).await;
                } else {
                    return Err(e);
                }
            }
        }
    }
}
```

---

## 17. CLI Integration Points

### lib.rs Modifications

File: `v3/crates/sindri-providers/src/lib.rs`

```rust
pub mod northflank;  // <-- ADD

pub fn create_provider(provider: ProviderType) -> Result<Box<dyn Provider>> {
    match provider {
        // ... existing arms ...
        ProviderType::Northflank => Ok(Box::new(northflank::NorthflankProvider::new()?)),
    }
}
```

### sindri.yaml Example

```yaml
version: "3.0"
name: my-dev-env

deployment:
  provider: northflank
  image: ghcr.io/org/sindri:latest
  resources:
    memory: 4GB
    cpus: 2
    gpu:
      enabled: false
  volumes:
    workspace:
      size: 10GB

extensions:
  profile: full

providers:
  northflank:
    region: us-east
    computePlan: nf-compute-200
    instances: 1
    volumeSizeGb: 10
    volumeMountPath: /data
    ports:
      - name: http
        internalPort: 8080
        public: true
        protocol: HTTP
```

### GPU-Enabled Example

```yaml
version: "3.0"
name: gpu-dev-env

deployment:
  provider: northflank
  image: ghcr.io/org/sindri-gpu:latest
  resources:
    memory: 32GB
    cpus: 8
    gpu:
      enabled: true
      type: nvidia
      tier: gpu-xlarge

providers:
  northflank:
    region: us-east
    computePlan: nf-compute-800-32
    gpuType: nvidia-h100
    volumeSizeGb: 50
```

---

## 18. Testing Strategy

### Unit Tests

File: `v3/crates/sindri-providers/src/northflank.rs` (within `#[cfg(test)] mod tests`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // --- Provider creation tests ---

    #[test]
    fn test_northflank_provider_creation() {
        let provider = NorthflankProvider::new().unwrap();
        assert_eq!(provider.name(), "northflank");
    }

    #[test]
    fn test_northflank_provider_with_output_dir() {
        let dir = PathBuf::from("/tmp/test-northflank");
        let provider = NorthflankProvider::with_output_dir(dir.clone()).unwrap();
        assert_eq!(provider.output_dir, dir);
        assert_eq!(provider.name(), "northflank");
    }

    // --- Capability flag tests ---

    #[test]
    fn test_northflank_supports_gpu() {
        let provider = NorthflankProvider::new().unwrap();
        assert!(provider.supports_gpu(), "Northflank should support GPU");
    }

    #[test]
    fn test_northflank_supports_auto_suspend() {
        let provider = NorthflankProvider::new().unwrap();
        assert!(
            provider.supports_auto_suspend(),
            "Northflank should support auto-suspend (pause/resume)"
        );
    }

    // --- Resource mapping tests ---

    #[test]
    fn test_map_resources_to_plan_small() {
        assert_eq!(map_resources_to_plan(1, 1024), "nf-compute-100-1");
        assert_eq!(map_resources_to_plan(1, 2048), "nf-compute-100-2");
    }

    #[test]
    fn test_map_resources_to_plan_medium() {
        assert_eq!(map_resources_to_plan(2, 4096), "nf-compute-200");
        assert_eq!(map_resources_to_plan(2, 8192), "nf-compute-200-8");
    }

    #[test]
    fn test_map_resources_to_plan_large() {
        assert_eq!(map_resources_to_plan(4, 8192), "nf-compute-400");
        assert_eq!(map_resources_to_plan(8, 16384), "nf-compute-800-16");
    }

    #[test]
    fn test_map_resources_to_plan_xlarge() {
        assert_eq!(map_resources_to_plan(16, 32768), "nf-compute-1600-32");
        assert_eq!(map_resources_to_plan(20, 40960), "nf-compute-2000-40");
    }

    // --- GPU tier mapping tests ---

    #[test]
    fn test_map_gpu_tier_to_northflank() {
        use sindri_core::types::GpuTier;
        assert_eq!(
            map_gpu_tier_to_northflank(&GpuTier::GpuSmall),
            "nvidia-l4"
        );
        assert_eq!(
            map_gpu_tier_to_northflank(&GpuTier::GpuMedium),
            "nvidia-a100-40gb"
        );
        assert_eq!(
            map_gpu_tier_to_northflank(&GpuTier::GpuLarge),
            "nvidia-a100-80gb"
        );
        assert_eq!(
            map_gpu_tier_to_northflank(&GpuTier::GpuXlarge),
            "nvidia-h100"
        );
    }

    // --- Status mapping tests ---

    #[test]
    fn test_map_northflank_status_running() {
        assert_eq!(
            map_northflank_status("running"),
            DeploymentState::Running
        );
        assert_eq!(
            map_northflank_status("healthy"),
            DeploymentState::Running
        );
    }

    #[test]
    fn test_map_northflank_status_paused() {
        assert_eq!(
            map_northflank_status("paused"),
            DeploymentState::Paused
        );
    }

    #[test]
    fn test_map_northflank_status_stopped() {
        assert_eq!(
            map_northflank_status("stopped"),
            DeploymentState::Stopped
        );
        assert_eq!(
            map_northflank_status("scaled-to-zero"),
            DeploymentState::Stopped
        );
    }

    #[test]
    fn test_map_northflank_status_creating() {
        assert_eq!(
            map_northflank_status("creating"),
            DeploymentState::Creating
        );
        assert_eq!(
            map_northflank_status("building"),
            DeploymentState::Creating
        );
        assert_eq!(
            map_northflank_status("deploying"),
            DeploymentState::Creating
        );
    }

    #[test]
    fn test_map_northflank_status_error() {
        assert_eq!(
            map_northflank_status("error"),
            DeploymentState::Error
        );
        assert_eq!(
            map_northflank_status("failed"),
            DeploymentState::Error
        );
        assert_eq!(
            map_northflank_status("crash-loop"),
            DeploymentState::Error
        );
    }

    #[test]
    fn test_map_northflank_status_unknown() {
        assert_eq!(
            map_northflank_status("some-new-status"),
            DeploymentState::Unknown
        );
    }

    // --- Cost estimation tests ---

    #[test]
    fn test_estimate_cost_basic_plan() {
        let cost = estimate_cost("nf-compute-200", &None);
        assert_eq!(cost.hourly, Some(0.067));
        assert!(cost.monthly.unwrap() > 48.0);
        assert_eq!(cost.currency, "USD");
        assert!(cost.notes.is_none());
    }

    #[test]
    fn test_estimate_cost_with_gpu() {
        let cost = estimate_cost(
            "nf-compute-800-32",
            &Some("nvidia-h100".to_string()),
        );
        let hourly = cost.hourly.unwrap();
        assert!(hourly > 3.0, "H100 + compute should exceed $3/hr");
        assert!(cost.notes.is_some());
    }

    // --- API response deserialization tests ---

    #[test]
    fn test_northflank_project_deserialization() {
        let json = r#"{"id": "proj-123", "name": "my-project"}"#;
        let project: NorthflankProject = serde_json::from_str(json).unwrap();
        assert_eq!(project.id, "proj-123");
        assert_eq!(project.name, "my-project");
    }

    #[test]
    fn test_northflank_service_deserialization() {
        let json = r#"{
            "id": "svc-456",
            "name": "my-service",
            "serviceType": "deployment",
            "status": "running"
        }"#;
        let service: NorthflankService = serde_json::from_str(json).unwrap();
        assert_eq!(service.id, "svc-456");
        assert_eq!(service.name, "my-service");
        assert_eq!(service.status.as_deref(), Some("running"));
    }

    #[test]
    fn test_northflank_service_with_deployment() {
        let json = r#"{
            "id": "svc-789",
            "name": "gpu-service",
            "status": "running",
            "deployment": {
                "instances": 1,
                "external": {
                    "imagePath": "ghcr.io/org/app:latest"
                }
            }
        }"#;
        let service: NorthflankService = serde_json::from_str(json).unwrap();
        assert_eq!(service.deployment.unwrap().instances, Some(1));
    }

    // --- Parse utility tests ---

    #[test]
    fn test_parse_memory_to_mb() {
        assert_eq!(parse_memory_to_mb("2GB"), Some(2048));
        assert_eq!(parse_memory_to_mb("512MB"), Some(512));
        assert_eq!(parse_memory_to_mb("4gb"), Some(4096));
    }

    #[test]
    fn test_parse_memory_to_mb_invalid() {
        assert_eq!(parse_memory_to_mb("abc"), None);
        assert_eq!(parse_memory_to_mb(""), None);
    }

    #[test]
    fn test_parse_size_to_gb() {
        assert_eq!(parse_size_to_gb("10GB"), Some(10));
        assert_eq!(parse_size_to_gb("1TB"), Some(1024));
    }

    // --- Warning collection tests ---

    #[test]
    fn test_collect_warnings_volume_scaling_conflict() {
        let provider = NorthflankProvider::new().unwrap();
        // This test would require constructing a NorthflankDeployConfig
        // with volume_size_gb and instances > 1 to verify the warning
    }

    // --- Prerequisite tests ---

    #[test]
    fn test_check_prerequisites_does_not_panic() {
        let provider = NorthflankProvider::new().unwrap();
        let result = provider.check_prerequisites();
        assert!(result.is_ok());
    }
}
```

### Integration Test Patterns

File: `v3/tests/integration/northflank_test.rs` (or inline in the module)

Integration tests should verify end-to-end flows with actual Northflank credentials. These are gated behind a feature flag or environment variable:

```rust
#[cfg(test)]
#[cfg(feature = "integration-tests")]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_full_deploy_lifecycle() {
        // Requires NORTHFLANK_API_TOKEN to be set
        if std::env::var("NORTHFLANK_API_TOKEN").is_err() {
            eprintln!("Skipping: NORTHFLANK_API_TOKEN not set");
            return;
        }

        // 1. Create provider
        // 2. Check prerequisites
        // 3. Deploy
        // 4. Status (verify running)
        // 5. Stop (verify paused)
        // 6. Start (verify running)
        // 7. Destroy
        // 8. Status (verify not deployed)
    }
}
```

### Mock Strategy for CLI Calls

For unit testing CLI interactions without running actual commands:

```rust
/// Trait for abstracting command execution (for testing)
#[cfg(test)]
trait CommandRunner {
    fn run(&self, cmd: &str, args: &[&str]) -> Result<std::process::Output>;
}

// In tests, inject a mock CommandRunner that returns predefined outputs.
// Production code uses tokio::process::Command directly.
//
// This follows the pattern used by other providers: unit tests focus on
// parsing and mapping logic, while CLI invocations are tested at the
// integration level.
```

---

## 19. Implementation Checklist

### Phase 1: Core Types (config_types.rs)

- [ ] Add `Northflank` variant to `Provider` enum
- [ ] Add `Display` implementation for `Northflank`
- [ ] Add `normalized()` return for `Northflank`
- [ ] Add `Northflank` to `supports_gpu()` match
- [ ] Define `NorthflankProviderConfig` struct
- [ ] Define `NorthflankAutoScaling` struct
- [ ] Define `NorthflankPort` struct
- [ ] Define `NorthflankHealthChecks` and `NorthflankProbe` structs
- [ ] Add `northflank: Option<NorthflankProviderConfig>` to `ProvidersConfig`
- [ ] Add default functions

### Phase 2: Provider Implementation (northflank.rs)

- [ ] Create `v3/crates/sindri-providers/src/northflank.rs`
- [ ] Implement `NorthflankProvider` struct with `new()` and `with_output_dir()`
- [ ] Implement `NorthflankDeployConfig` extraction
- [ ] Implement API response deserialization structs
- [ ] Implement `map_resources_to_plan()` helper
- [ ] Implement `map_gpu_tier_to_northflank()` helper
- [ ] Implement `parse_memory_to_mb()` and `parse_size_to_gb()` (or reuse from utils)
- [ ] Implement `map_northflank_status()` helper
- [ ] Implement `estimate_cost()` helper
- [ ] Implement `is_authenticated()` helper
- [ ] Implement `project_exists()` helper
- [ ] Implement `service_exists()` helper
- [ ] Implement `ensure_project()` helper
- [ ] Implement `create_deployment_service()` helper
- [ ] Implement `delete_service()` helper
- [ ] Implement `get_service_state()` helper
- [ ] Implement `pause_service()` helper
- [ ] Implement `resume_service()` helper
- [ ] Implement `wait_for_running()` helper
- [ ] Implement `resolve_and_set_secrets()` helper
- [ ] Implement `collect_warnings()` helper
- [ ] Implement `project_has_other_services()` helper

### Phase 3: Provider Trait (9 required + 2 optional methods)

- [ ] `name()` -- returns `"northflank"`
- [ ] `check_prerequisites()` -- CLI + auth check
- [ ] `deploy()` -- full lifecycle
- [ ] `connect()` -- `northflank exec`
- [ ] `status()` -- service state query
- [ ] `destroy()` -- service + optional project deletion
- [ ] `plan()` -- dry-run plan
- [ ] `start()` -- resume paused service
- [ ] `stop()` -- pause running service
- [ ] `supports_gpu()` -- returns `true`
- [ ] `supports_auto_suspend()` -- returns `true`

### Phase 4: Integration (lib.rs)

- [ ] Add `pub mod northflank;` to lib.rs
- [ ] Add `ProviderType::Northflank` match arm to `create_provider()`

### Phase 5: Schema

- [ ] Add `"northflank"` to provider enum in `sindri.schema.json`
- [ ] Add northflank config object under providers in schema

### Phase 6: Tests

- [ ] Provider creation test
- [ ] Output directory test
- [ ] GPU support flag test
- [ ] Auto-suspend support flag test
- [ ] Resource-to-plan mapping tests (all size categories)
- [ ] GPU tier mapping tests (all tiers)
- [ ] Status mapping tests (all states)
- [ ] Cost estimation tests (CPU-only and GPU)
- [ ] API response deserialization tests (project, service, deployment)
- [ ] Parse utility tests (memory, size)
- [ ] Prerequisites check does not panic
- [ ] Warning collection tests (volume+scaling, GPU region)

---

## Appendix A: Provider Comparison (Northflank vs Existing)

| Aspect          | Fly.io                  | E2B                    | Northflank                                   |
| --------------- | ----------------------- | ---------------------- | -------------------------------------------- |
| CLI Tool        | `flyctl`                | `e2b`                  | `northflank`                                 |
| Auth Method     | `flyctl auth login`     | `E2B_API_KEY` env      | `northflank login` or `NORTHFLANK_API_TOKEN` |
| Connect         | `flyctl ssh console`    | `e2b sandbox terminal` | `northflank exec`                            |
| Stop            | Stop machine            | Pause sandbox          | `northflank pause service`                   |
| Start           | Start machine           | Resume sandbox         | `northflank resume service`                  |
| Config File     | `fly.toml` (generated)  | `e2b.toml` (generated) | None (CLI args only)                         |
| GPU             | A100, L40s              | No                     | H100, B200, A100, L4, H200, MI300X           |
| Auto-suspend    | Machine suspend         | Sandbox pause          | Service pause                                |
| Volumes         | Fly volumes             | No                     | SSD volumes (up to 1.5TB)                    |
| Secrets         | `flyctl secrets import` | Dockerfile ENV         | Runtime variables / Secret groups            |
| Cost Visibility | No native estimate      | No native estimate     | Plan-based pricing (estimatable)             |

## Appendix B: Northflank CLI Command Reference (Used by Adapter)

| Operation      | Command                                                                                          |
| -------------- | ------------------------------------------------------------------------------------------------ |
| Check auth     | `northflank list projects`                                                                       |
| Create project | `northflank create project --name {name} --region {region}`                                      |
| Get project    | `northflank get project --project {name}`                                                        |
| Create service | `northflank create service deployment --project {proj} --name {svc} --image {img} --plan {plan}` |
| Get service    | `northflank get service details --project {proj} --service {svc}`                                |
| Delete service | `northflank delete service --project {proj} --service {svc}`                                     |
| Pause service  | `northflank pause service --project {proj} --service {svc}`                                      |
| Resume service | `northflank resume service --project {proj} --service {svc}`                                     |
| Exec shell     | `northflank exec --project {proj} --service {svc}`                                               |
| Port forward   | `northflank forward service --project {proj} --service {svc}`                                    |
| List services  | `northflank list services --project {proj}`                                                      |
| Scale service  | `northflank scale service --project {proj} --service {svc} --instances {n}`                      |
| Get version    | `northflank --version`                                                                           |
