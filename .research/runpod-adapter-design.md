# RunPod Provider Adapter -- Architecture Design Document

> Date: 2026-02-16
> Status: Implementation-Ready
> Scope: `v3/crates/sindri-providers/src/runpod.rs` and related integration points

---

## Table of Contents

1. [Overview](#1-overview)
2. [Architectural Decision: HTTP Client vs CLI Subprocess](#2-architectural-decision-http-client-vs-cli-subprocess)
3. [Module Structure](#3-module-structure)
4. [Configuration Schema](#4-configuration-schema)
5. [API Response Structs](#5-api-response-structs)
6. [Provider Trait Implementation](#6-provider-trait-implementation)
7. [HTTP Client Layer](#7-http-client-layer)
8. [Deploy Lifecycle](#8-deploy-lifecycle)
9. [Connect Strategy](#9-connect-strategy)
10. [Status Implementation](#10-status-implementation)
11. [Start/Stop Implementation](#11-startstop-implementation)
12. [Secrets Injection](#12-secrets-injection)
13. [Storage Strategy](#13-storage-strategy)
14. [Error Handling](#14-error-handling)
15. [Testing Strategy](#15-testing-strategy)
16. [Integration Points](#16-integration-points)
17. [Example sindri.yaml](#17-example-sindriyaml)
18. [Implementation Checklist](#18-implementation-checklist)

---

## 1. Overview

The RunPod provider adapter enables Sindri v3 to deploy GPU/CPU development environments on RunPod's cloud infrastructure. RunPod is a GPU cloud platform offering persistent pods with SSH access, making it well-suited for GPU-accelerated development workloads.

### Key Differentiators from Other Providers

| Aspect          | Other Providers                             | RunPod Provider                                        |
| --------------- | ------------------------------------------- | ------------------------------------------------------ |
| API interaction | CLI subprocess (`flyctl`, `e2b`, `kubectl`) | Direct HTTP REST API via `reqwest`                     |
| GPU support     | Limited/varied                              | First-class, extensive GPU catalog                     |
| Pricing model   | Varied                                      | Per-second billing, spot/on-demand                     |
| Auto-suspend    | Fly.io suspend, E2B pause                   | Stop (retains volume data)                             |
| Storage         | Provider-specific                           | Three-tier: container disk, pod volume, network volume |

### Capability Flags

- `supports_gpu()` = `true` -- RunPod's primary value proposition
- `supports_auto_suspend()` = `true` -- Pods can be stopped (volume data persists) and restarted

---

## 2. Architectural Decision: HTTP Client vs CLI Subprocess

### ADR: Use `reqwest` HTTP Client Instead of `runpodctl` CLI

**Context**: All existing Sindri v3 providers use CLI subprocess calls (`tokio::process::Command`) to interact with their platforms (flyctl, e2b, kubectl, docker). RunPod provides both a CLI (`runpodctl`) and a REST API.

**Decision**: Use `reqwest` HTTP client for direct REST API calls.

**Rationale**:

1. **`runpodctl` limitations**: The CLI is primarily a file-transfer and pod-management tool. It lacks the full API surface needed (e.g., GPU availability queries, detailed pod configuration, network volume management).

2. **REST API maturity**: RunPod's REST API v1 at `https://rest.runpod.io/v1` is well-documented with an OpenAPI spec, actively developed, and covers all operations Sindri needs.

3. **Fewer external dependencies**: Users do not need to install `runpodctl` separately. The only prerequisite is a `RUNPOD_API_KEY` environment variable.

4. **Better error handling**: Direct HTTP responses provide structured error information, unlike parsing CLI stderr output.

5. **Performance**: No subprocess spawn overhead for each API call.

**Consequences**:

- Must add `reqwest` to `sindri-providers` Cargo.toml dependencies
- `check_prerequisites()` checks for API key only, not CLI tool
- Different coding pattern from other providers (HTTP calls vs subprocess)
- Easier to unit test with HTTP mocking

**New Dependency** (`v3/crates/sindri-providers/Cargo.toml`):

```toml
[dependencies]
# ... existing dependencies ...
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }

[dev-dependencies]
# ... existing dependencies ...
mockito = "1.5"
```

Note: `reqwest` with `rustls-tls` avoids OpenSSL system dependency issues. The `json` feature enables `.json()` request/response methods.

---

## 3. Module Structure

### File: `v3/crates/sindri-providers/src/runpod.rs`

```rust
//! RunPod provider implementation
//!
//! Deploys Sindri environments to RunPod GPU/CPU pods using the REST API v1.
//! Unlike other providers that shell out to CLI tools, this provider uses
//! `reqwest` for direct HTTP API calls to https://rest.runpod.io/v1.

use crate::templates::TemplateRegistry;
use crate::traits::Provider;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use reqwest::{header, Client, StatusCode};
use serde::{Deserialize, Serialize};
use sindri_core::config::SindriConfig;
use sindri_core::types::{
    ActionType, ConnectionInfo, DeployOptions, DeployResult, DeploymentPlan,
    DeploymentState, DeploymentStatus, PlannedAction, PlannedResource,
    Prerequisite, PrerequisiteStatus,
};
use sindri_secrets::{ResolutionContext, SecretResolver};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{debug, info, warn};
```

### Primary Struct: `RunpodProvider`

```rust
/// RunPod provider for GPU/CPU cloud pods
pub struct RunpodProvider {
    /// Template registry (kept for consistency, minimal template usage)
    templates: TemplateRegistry,
    /// Output directory for any generated files
    output_dir: PathBuf,
}
```

### Constructors

```rust
impl RunpodProvider {
    /// Base URL for RunPod REST API v1
    const API_BASE: &'static str = "https://rest.runpod.io/v1";

    /// Default poll interval when waiting for pod to become RUNNING
    const POLL_INTERVAL: Duration = Duration::from_secs(5);

    /// Maximum number of poll attempts (5 seconds * 120 = 10 minutes)
    const MAX_POLL_ATTEMPTS: u32 = 120;

    /// Create a new RunPod provider
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

### Internal Config Struct: `RunpodDeployConfig`

Extracted from `SindriConfig` with RunPod-specific defaults:

```rust
/// RunPod deployment configuration extracted from SindriConfig
struct RunpodDeployConfig<'a> {
    /// Deployment name (used as pod name)
    name: &'a str,
    /// Container image to deploy
    image: String,
    /// Compute type: "GPU" or "CPU"
    compute_type: &'static str,
    /// GPU type IDs (priority list), e.g. ["NVIDIA GeForce RTX 4090"]
    gpu_type_ids: Vec<String>,
    /// Number of GPUs
    gpu_count: u32,
    /// Number of vCPUs (CPU pods only)
    vcpu_count: u32,
    /// Container disk size in GB (ephemeral)
    container_disk_gb: u32,
    /// Pod volume size in GB (persists on stop)
    volume_gb: u32,
    /// Volume mount path
    volume_mount_path: String,
    /// Cloud type: "SECURE", "COMMUNITY", or "ALL"
    cloud_type: String,
    /// Data center IDs (optional, for region selection)
    data_center_ids: Vec<String>,
    /// Whether pod is interruptible (spot pricing)
    interruptible: bool,
    /// Port mappings (e.g. ["8888/http", "22/tcp"])
    ports: Vec<String>,
    /// Network volume ID (optional, for persistent storage)
    network_volume_id: Option<String>,
    /// Whether to request a public IP
    support_public_ip: bool,
}
```

---

## 4. Configuration Schema

### Rust Config Struct

**File**: `v3/crates/sindri-core/src/types/config_types.rs`

Add to `ProvidersConfig`:

```rust
/// Provider-specific configurations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProvidersConfig {
    // ... existing fields ...

    /// RunPod specific config
    #[serde(default)]
    pub runpod: Option<RunpodProviderConfig>,
}
```

New struct:

```rust
/// RunPod provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunpodProviderConfig {
    /// GPU type identifier or pool ID
    /// Examples: "NVIDIA GeForce RTX 4090", "ADA_24", "AMPERE_80"
    #[serde(default, rename = "gpuType")]
    pub gpu_type: Option<String>,

    /// Number of GPUs (default: 1)
    #[serde(default = "default_gpu_count_1", rename = "gpuCount")]
    pub gpu_count: u32,

    /// Enable CPU-only mode (no GPU)
    #[serde(default, rename = "cpuOnly")]
    pub cpu_only: bool,

    /// Number of vCPUs for CPU pods (default: 4)
    #[serde(default = "default_vcpu_count", rename = "vcpuCount")]
    pub vcpu_count: u32,

    /// Container disk size in GB (ephemeral, default: 50)
    #[serde(default = "default_container_disk", rename = "containerDiskGb")]
    pub container_disk_gb: u32,

    /// Pod volume size in GB (persists on stop, default: 20)
    #[serde(default = "default_volume_size", rename = "volumeSizeGb")]
    pub volume_size_gb: u32,

    /// Volume mount path (default: "/workspace")
    #[serde(default = "default_volume_mount", rename = "volumeMountPath")]
    pub volume_mount_path: String,

    /// Cloud type: "SECURE" or "COMMUNITY" (default: "SECURE")
    #[serde(default = "default_cloud_type", rename = "cloudType")]
    pub cloud_type: String,

    /// Data center/region ID (e.g., "US-CA-2", "EU-RO-1")
    #[serde(default)]
    pub region: Option<String>,

    /// Enable spot/interruptible pricing (default: false)
    #[serde(default)]
    pub spot: bool,

    /// Exposed port mappings (default: ["8888/http", "22/tcp"])
    #[serde(default = "default_runpod_ports")]
    pub ports: Vec<String>,

    /// Network volume ID for persistent storage
    #[serde(default, rename = "networkVolumeId")]
    pub network_volume_id: Option<String>,

    /// Request a public IP for full SSH/SCP support
    #[serde(default, rename = "publicIp")]
    pub public_ip: bool,
}

fn default_gpu_count_1() -> u32 { 1 }
fn default_vcpu_count() -> u32 { 4 }
fn default_container_disk() -> u32 { 50 }
fn default_volume_size() -> u32 { 20 }
fn default_volume_mount() -> String { "/workspace".to_string() }
fn default_cloud_type() -> String { "SECURE".to_string() }
fn default_runpod_ports() -> Vec<String> {
    vec!["8888/http".to_string(), "22/tcp".to_string()]
}
```

### Provider Enum Extension

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
    Runpod,     // <-- NEW
}
```

Update `Display`, `normalized()`, and `supports_gpu()` implementations:

```rust
impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // ... existing ...
            Provider::Runpod => write!(f, "runpod"),
        }
    }
}

impl Provider {
    pub fn normalized(&self) -> &str {
        match self {
            // ... existing ...
            Provider::Runpod => "runpod",
        }
    }

    pub fn supports_gpu(&self) -> bool {
        matches!(
            self,
            Provider::Docker
                | Provider::DockerCompose
                | Provider::Fly
                | Provider::Devpod
                | Provider::Kubernetes
                | Provider::Runpod  // <-- ADD
        )
    }
}
```

### JSON Schema Extension

**File**: `v3/schemas/sindri.schema.json`

Add `"runpod"` to the provider enum:

```json
"provider": {
  "type": "string",
  "enum": ["fly", "kubernetes", "docker-compose", "docker", "devpod", "e2b", "runpod"]
}
```

Add RunPod provider config under `providers`:

```json
"runpod": {
  "type": "object",
  "properties": {
    "gpuType": {
      "type": "string",
      "description": "GPU type ID or pool ID (e.g., 'NVIDIA GeForce RTX 4090', 'ADA_24')"
    },
    "gpuCount": {
      "type": "integer",
      "minimum": 1,
      "maximum": 16,
      "default": 1,
      "description": "Number of GPUs"
    },
    "cpuOnly": {
      "type": "boolean",
      "default": false,
      "description": "CPU-only mode (no GPU)"
    },
    "vcpuCount": {
      "type": "integer",
      "minimum": 1,
      "default": 4,
      "description": "Number of vCPUs (CPU pods)"
    },
    "containerDiskGb": {
      "type": "integer",
      "minimum": 1,
      "default": 50,
      "description": "Container disk size in GB (ephemeral)"
    },
    "volumeSizeGb": {
      "type": "integer",
      "minimum": 0,
      "default": 20,
      "description": "Pod volume size in GB (persists on stop)"
    },
    "volumeMountPath": {
      "type": "string",
      "default": "/workspace",
      "description": "Volume mount path inside container"
    },
    "cloudType": {
      "type": "string",
      "enum": ["SECURE", "COMMUNITY"],
      "default": "SECURE",
      "description": "Cloud security tier"
    },
    "region": {
      "type": "string",
      "description": "Data center ID (e.g., 'US-CA-2', 'EU-RO-1')"
    },
    "spot": {
      "type": "boolean",
      "default": false,
      "description": "Use spot/interruptible pricing"
    },
    "ports": {
      "type": "array",
      "items": { "type": "string" },
      "default": ["8888/http", "22/tcp"],
      "description": "Port mappings (format: 'port/protocol')"
    },
    "networkVolumeId": {
      "type": "string",
      "description": "Network volume ID for persistent storage"
    },
    "publicIp": {
      "type": "boolean",
      "default": false,
      "description": "Request a public IP for SSH/SCP"
    }
  },
  "additionalProperties": false
}
```

---

## 5. API Response Structs

### Pod Response (from `GET /v1/pods/{id}` and `POST /v1/pods`)

```rust
/// RunPod pod response from REST API
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PodResponse {
    /// Pod unique identifier
    id: String,
    /// Pod name
    name: String,
    /// Container image
    #[serde(default)]
    image: Option<String>,
    /// Pod status string
    #[serde(default)]
    status: Option<String>,
    /// Desired status
    #[serde(default)]
    desired_status: Option<String>,
    /// Public IP address (if available)
    #[serde(default)]
    public_ip: Option<String>,
    /// Port mappings
    #[serde(default)]
    port_mappings: Option<Vec<PortMapping>>,
    /// Cost per hour in USD
    #[serde(default)]
    cost_per_hr: Option<f64>,
    /// GPU information
    #[serde(default)]
    gpu: Option<PodGpu>,
    /// Volume size in GB
    #[serde(default)]
    volume_in_gb: Option<u32>,
    /// Container disk size in GB
    #[serde(default)]
    container_disk_in_gb: Option<u32>,
    /// Machine details
    #[serde(default)]
    machine: Option<PodMachine>,
}

/// Port mapping from RunPod API
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PortMapping {
    /// Private container port
    #[serde(default)]
    private_port: Option<u16>,
    /// Public mapped port
    #[serde(default)]
    public_port: Option<u16>,
    /// Protocol (tcp, http)
    #[serde(default)]
    r#type: Option<String>,
    /// Public IP for this mapping
    #[serde(default)]
    ip: Option<String>,
}

/// GPU information from RunPod API
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PodGpu {
    /// GPU type name
    #[serde(default, alias = "type")]
    gpu_type: Option<String>,
    /// Number of GPUs
    #[serde(default)]
    count: Option<u32>,
}

/// Machine information from RunPod API
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PodMachine {
    /// Machine/host ID
    #[serde(default)]
    pod_host_id: Option<String>,
}
```

### Create Pod Request

```rust
/// Request body for creating a RunPod pod (POST /v1/pods)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreatePodRequest {
    /// Pod name
    name: String,
    /// Container image
    image_name: String,
    /// Compute type: "GPU" or "CPU"
    #[serde(skip_serializing_if = "Option::is_none")]
    compute_type: Option<String>,
    /// GPU type IDs (priority list)
    #[serde(skip_serializing_if = "Option::is_none")]
    gpu_type_ids: Option<Vec<String>>,
    /// Number of GPUs
    #[serde(skip_serializing_if = "Option::is_none")]
    gpu_count: Option<u32>,
    /// Number of vCPUs (CPU pods only)
    #[serde(skip_serializing_if = "Option::is_none")]
    vcpu_count: Option<u32>,
    /// Container disk size in GB
    #[serde(skip_serializing_if = "Option::is_none")]
    container_disk_in_gb: Option<u32>,
    /// Pod volume size in GB
    #[serde(skip_serializing_if = "Option::is_none")]
    volume_in_gb: Option<u32>,
    /// Volume mount path
    #[serde(skip_serializing_if = "Option::is_none")]
    volume_mount_path: Option<String>,
    /// Cloud type: "SECURE" or "COMMUNITY"
    #[serde(skip_serializing_if = "Option::is_none")]
    cloud_type: Option<String>,
    /// Data center IDs
    #[serde(skip_serializing_if = "Option::is_none")]
    data_center_ids: Option<Vec<String>>,
    /// Interruptible (spot) flag
    #[serde(skip_serializing_if = "Option::is_none")]
    interruptible: Option<bool>,
    /// Port mappings
    #[serde(skip_serializing_if = "Option::is_none")]
    ports: Option<Vec<String>>,
    /// Environment variables
    #[serde(skip_serializing_if = "Option::is_none")]
    env: Option<HashMap<String, String>>,
    /// Network volume ID
    #[serde(skip_serializing_if = "Option::is_none")]
    network_volume_id: Option<String>,
    /// Support public IP
    #[serde(skip_serializing_if = "Option::is_none")]
    support_public_ip: Option<bool>,
}
```

### Error Response

```rust
/// RunPod API error response
#[derive(Debug, Clone, Deserialize)]
struct ApiError {
    /// Error message
    #[serde(default)]
    message: Option<String>,
    /// Error code
    #[serde(default)]
    error: Option<String>,
}
```

### Pod List Response

```rust
/// Response from GET /v1/pods (list all pods)
/// The API returns an array of PodResponse objects directly.
type PodListResponse = Vec<PodResponse>;
```

---

## 6. Provider Trait Implementation

### Complete Trait Implementation Skeleton

```rust
#[async_trait]
impl Provider for RunpodProvider {
    fn name(&self) -> &'static str {
        "runpod"
    }

    fn check_prerequisites(&self) -> Result<PrerequisiteStatus> {
        // See Section 6.1
    }

    async fn deploy(
        &self,
        config: &SindriConfig,
        opts: DeployOptions,
    ) -> Result<DeployResult> {
        // See Section 8
    }

    async fn connect(&self, config: &SindriConfig) -> Result<()> {
        // See Section 9
    }

    async fn status(&self, config: &SindriConfig) -> Result<DeploymentStatus> {
        // See Section 10
    }

    async fn destroy(&self, config: &SindriConfig, force: bool) -> Result<()> {
        // See Section 6.2
    }

    async fn plan(&self, config: &SindriConfig) -> Result<DeploymentPlan> {
        // See Section 6.3
    }

    async fn start(&self, config: &SindriConfig) -> Result<()> {
        // See Section 11
    }

    async fn stop(&self, config: &SindriConfig) -> Result<()> {
        // See Section 11
    }

    fn supports_gpu(&self) -> bool {
        true
    }

    fn supports_auto_suspend(&self) -> bool {
        true // stop preserves pod volume data
    }
}
```

### 6.1 check_prerequisites()

Unlike other providers that check for CLI tools, RunPod only requires an API key:

```rust
fn check_prerequisites(&self) -> Result<PrerequisiteStatus> {
    let mut missing = Vec::new();
    let mut available = Vec::new();

    // Check RUNPOD_API_KEY environment variable
    if std::env::var("RUNPOD_API_KEY").is_ok() {
        available.push(Prerequisite {
            name: "runpod-api-key".to_string(),
            description: "RunPod API key (RUNPOD_API_KEY)".to_string(),
            install_hint: None,
            version: None,
        });
    } else {
        missing.push(Prerequisite {
            name: "runpod-api-key".to_string(),
            description: "RunPod API key".to_string(),
            install_hint: Some(
                "Set RUNPOD_API_KEY environment variable. \
                 Get key at https://www.runpod.io/console/user/settings"
                    .to_string(),
            ),
            version: None,
        });
    }

    // Optional: check for ssh (needed for connect)
    if crate::utils::command_exists("ssh") {
        available.push(Prerequisite {
            name: "ssh".to_string(),
            description: "SSH client (for pod connection)".to_string(),
            install_hint: None,
            version: None,
        });
    } else {
        // SSH is optional -- connect can still show instructions
        available.push(Prerequisite {
            name: "ssh".to_string(),
            description: "SSH client (not found, connection instructions will be shown)"
                .to_string(),
            install_hint: Some("Install OpenSSH client".to_string()),
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

### 6.2 destroy()

```rust
async fn destroy(&self, config: &SindriConfig, force: bool) -> Result<()> {
    let name = config.name();
    info!("Destroying {} on RunPod (force: {})", name, force);

    // Find pod by name
    let pod = self.find_pod_by_name(name).await?;

    match pod {
        Some(pod) => {
            info!("Terminating pod: {} ({})", pod.name, pod.id);
            self.terminate_pod(&pod.id).await?;
            info!("Pod '{}' terminated", name);
        }
        None => {
            warn!("Pod '{}' not found on RunPod", name);
        }
    }

    Ok(())
}
```

### 6.3 plan()

```rust
async fn plan(&self, config: &SindriConfig) -> Result<DeploymentPlan> {
    let deploy_config = self.get_runpod_config(config).await?;
    let name = deploy_config.name.to_string();
    info!("Planning RunPod deployment for {}", name);

    let mut actions = Vec::new();

    // Check if pod already exists
    let existing = self.find_pod_by_name(&name).await?;
    if existing.is_some() {
        actions.push(PlannedAction {
            action: ActionType::Update,
            resource: format!("pod:{}", name),
            description: "Pod already exists (use --force to recreate)".to_string(),
        });
    } else {
        actions.push(PlannedAction {
            action: ActionType::Create,
            resource: format!("pod:{}", name),
            description: format!(
                "Create RunPod {} pod with image {}",
                deploy_config.compute_type, deploy_config.image
            ),
        });
    }

    let mut resource_config = HashMap::new();
    resource_config.insert(
        "compute_type".to_string(),
        serde_json::json!(deploy_config.compute_type),
    );
    resource_config.insert(
        "image".to_string(),
        serde_json::json!(deploy_config.image),
    );

    if deploy_config.compute_type == "GPU" {
        resource_config.insert(
            "gpu_type_ids".to_string(),
            serde_json::json!(deploy_config.gpu_type_ids),
        );
        resource_config.insert(
            "gpu_count".to_string(),
            serde_json::json!(deploy_config.gpu_count),
        );
    } else {
        resource_config.insert(
            "vcpu_count".to_string(),
            serde_json::json!(deploy_config.vcpu_count),
        );
    }

    resource_config.insert(
        "container_disk_gb".to_string(),
        serde_json::json!(deploy_config.container_disk_gb),
    );
    resource_config.insert(
        "volume_gb".to_string(),
        serde_json::json!(deploy_config.volume_gb),
    );
    resource_config.insert(
        "cloud_type".to_string(),
        serde_json::json!(deploy_config.cloud_type),
    );

    let resources = vec![PlannedResource {
        resource_type: "pod".to_string(),
        name: name.clone(),
        config: resource_config,
    }];

    Ok(DeploymentPlan {
        provider: "runpod".to_string(),
        actions,
        resources,
        estimated_cost: None, // Could query GPU pricing in future
    })
}
```

---

## 7. HTTP Client Layer

### API Client Helper Methods

```rust
impl RunpodProvider {
    /// Get the API key from environment
    fn api_key() -> Result<String> {
        std::env::var("RUNPOD_API_KEY")
            .context("RUNPOD_API_KEY environment variable not set. \
                      Get your API key at https://www.runpod.io/console/user/settings")
    }

    /// Build a configured reqwest client
    fn build_client() -> Result<Client> {
        let api_key = Self::api_key()?;
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {}", api_key))
                .context("Invalid API key format")?,
        );
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );

        Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to build HTTP client")
    }

    /// Create a pod via POST /v1/pods
    async fn create_pod(&self, request: &CreatePodRequest) -> Result<PodResponse> {
        let client = Self::build_client()?;
        let url = format!("{}/pods", Self::API_BASE);

        debug!("Creating pod: POST {}", url);
        debug!("Request body: {:?}", request);

        let response = client
            .post(&url)
            .json(request)
            .send()
            .await
            .context("Failed to send create pod request to RunPod API")?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(self.map_api_error(status, &error_body));
        }

        let pod: PodResponse = response
            .json()
            .await
            .context("Failed to parse RunPod create pod response")?;

        info!("Pod created: {} ({})", pod.name, pod.id);
        Ok(pod)
    }

    /// Get pod details via GET /v1/pods/{podId}
    async fn get_pod(&self, pod_id: &str) -> Result<PodResponse> {
        let client = Self::build_client()?;
        let url = format!("{}/pods/{}", Self::API_BASE, pod_id);

        debug!("Getting pod: GET {}", url);

        let response = client
            .get(&url)
            .send()
            .await
            .context("Failed to query RunPod API")?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(self.map_api_error(status, &error_body));
        }

        response
            .json()
            .await
            .context("Failed to parse pod response")
    }

    /// List all pods via GET /v1/pods
    async fn list_pods(&self) -> Result<Vec<PodResponse>> {
        let client = Self::build_client()?;
        let url = format!("{}/pods", Self::API_BASE);

        debug!("Listing pods: GET {}", url);

        let response = client
            .get(&url)
            .send()
            .await
            .context("Failed to list pods from RunPod API")?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(self.map_api_error(status, &error_body));
        }

        response
            .json()
            .await
            .context("Failed to parse pod list response")
    }

    /// Start a stopped pod via POST /v1/pods/{podId}/start
    async fn start_pod(&self, pod_id: &str) -> Result<()> {
        let client = Self::build_client()?;
        let url = format!("{}/pods/{}/start", Self::API_BASE, pod_id);

        info!("Starting pod: {}", pod_id);

        let response = client
            .post(&url)
            .send()
            .await
            .context("Failed to start pod")?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(self.map_api_error(status, &error_body));
        }

        Ok(())
    }

    /// Stop a running pod via POST /v1/pods/{podId}/stop
    async fn stop_pod(&self, pod_id: &str) -> Result<()> {
        let client = Self::build_client()?;
        let url = format!("{}/pods/{}/stop", Self::API_BASE, pod_id);

        info!("Stopping pod: {}", pod_id);

        let response = client
            .post(&url)
            .send()
            .await
            .context("Failed to stop pod")?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(self.map_api_error(status, &error_body));
        }

        Ok(())
    }

    /// Terminate a pod via DELETE /v1/pods/{podId}
    async fn terminate_pod(&self, pod_id: &str) -> Result<()> {
        let client = Self::build_client()?;
        let url = format!("{}/pods/{}", Self::API_BASE, pod_id);

        info!("Terminating pod: {}", pod_id);

        let response = client
            .delete(&url)
            .send()
            .await
            .context("Failed to terminate pod")?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(self.map_api_error(status, &error_body));
        }

        Ok(())
    }

    /// Find a pod by its name (linear scan of all pods)
    async fn find_pod_by_name(&self, name: &str) -> Result<Option<PodResponse>> {
        let pods = self.list_pods().await?;
        Ok(pods.into_iter().find(|p| p.name == name))
    }

    /// Poll pod until it reaches RUNNING state or timeout
    async fn wait_for_running(&self, pod_id: &str) -> Result<PodResponse> {
        for attempt in 0..Self::MAX_POLL_ATTEMPTS {
            let pod = self.get_pod(pod_id).await?;
            let status = pod.status.as_deref().unwrap_or("UNKNOWN");

            debug!(
                "Pod {} status: {} (attempt {}/{})",
                pod_id,
                status,
                attempt + 1,
                Self::MAX_POLL_ATTEMPTS
            );

            match status {
                "RUNNING" => return Ok(pod),
                "EXITED" | "TERMINATED" | "ERROR" => {
                    return Err(anyhow!(
                        "Pod entered unexpected state '{}' while waiting for RUNNING",
                        status
                    ));
                }
                _ => {
                    // CREATED, STARTING, etc. -- keep polling
                    tokio::time::sleep(Self::POLL_INTERVAL).await;
                }
            }
        }

        Err(anyhow!(
            "Timed out waiting for pod '{}' to reach RUNNING state \
             (waited {} seconds)",
            pod_id,
            Self::POLL_INTERVAL.as_secs() * Self::MAX_POLL_ATTEMPTS as u64
        ))
    }
}
```

---

## 8. Deploy Lifecycle

### Sequence Diagram

```
User                    RunpodProvider                  RunPod REST API
  |                          |                               |
  |-- sindri deploy -------->|                               |
  |                          |-- check_prerequisites() ----->|
  |                          |   (check RUNPOD_API_KEY)      |
  |                          |                               |
  |                          |-- resolve_image() ----------->|
  |                          |                               |
  |                          |-- [if dry_run] plan() ------->|
  |                          |<- return plan ----------------|
  |                          |                               |
  |                          |-- find_pod_by_name() -------->|
  |                          |   GET /v1/pods                |
  |                          |<- pod list -------------------|
  |                          |                               |
  |                          |-- [if exists && force] ------>|
  |                          |   DELETE /v1/pods/{id}        |
  |                          |<- 200 OK --------------------|
  |                          |                               |
  |                          |-- resolve_secrets() --------->|
  |                          |                               |
  |                          |-- create_pod() -------------->|
  |                          |   POST /v1/pods               |
  |                          |<- 201 PodResponse ------------|
  |                          |                               |
  |                          |-- wait_for_running() -------->|
  |                          |   GET /v1/pods/{id} (poll)    |
  |                          |<- RUNNING --------------------|
  |                          |                               |
  |<- DeployResult ----------|                               |
```

### deploy() Implementation

```rust
async fn deploy(
    &self,
    config: &SindriConfig,
    opts: DeployOptions,
) -> Result<DeployResult> {
    let deploy_config = self.get_runpod_config(config).await?;
    let name = deploy_config.name.to_string();
    info!("Deploying {} to RunPod", name);

    // 1. Check prerequisites
    let prereqs = self.check_prerequisites()?;
    if !prereqs.satisfied {
        let missing_names: Vec<_> = prereqs.missing.iter().map(|p| p.name.as_str()).collect();
        return Err(anyhow!(
            "Missing prerequisites: {}",
            missing_names.join(", ")
        ));
    }

    // 2. Handle dry-run
    if opts.dry_run {
        let plan = self.plan(config).await?;
        return Ok(DeployResult {
            success: true,
            name: name.clone(),
            provider: "runpod".to_string(),
            instance_id: None,
            connection: None,
            messages: plan
                .actions
                .iter()
                .map(|a| format!("{:?}: {} - {}", a.action, a.resource, a.description))
                .collect(),
            warnings: vec![],
        });
    }

    // 3. Check for existing pod
    if let Some(existing_pod) = self.find_pod_by_name(&name).await? {
        if opts.force {
            info!("Force flag set, terminating existing pod: {}", existing_pod.id);
            self.terminate_pod(&existing_pod.id).await?;
            // Wait briefly for termination to propagate
            tokio::time::sleep(Duration::from_secs(3)).await;
        } else {
            let status = existing_pod.status.as_deref().unwrap_or("UNKNOWN");
            return Err(anyhow!(
                "Pod '{}' already exists (status: {}). \
                 Use --force to recreate.",
                name,
                status
            ));
        }
    }

    // 4. Resolve secrets
    let env_vars = self.resolve_secrets(config, None).await?;

    // 5. Build create request
    let request = self.build_create_request(&deploy_config, env_vars)?;

    // 6. Create pod
    let pod = self.create_pod(&request).await?;
    let pod_id = pod.id.clone();

    // 7. Wait for RUNNING state (if requested)
    let final_pod = if opts.wait {
        self.wait_for_running(&pod_id).await?
    } else {
        // Brief wait to let pod initialize
        tokio::time::sleep(Duration::from_secs(2)).await;
        self.get_pod(&pod_id).await.unwrap_or(pod)
    };

    // 8. Build connection info
    let connection = self.build_connection_info(&final_pod);

    // 9. Build cost message
    let mut messages = vec![format!("Pod '{}' created successfully on RunPod", name)];
    if let Some(cost) = final_pod.cost_per_hr {
        messages.push(format!("Cost: ${:.3}/hr", cost));
    }
    if let Some(ref gpu) = final_pod.gpu {
        if let Some(ref gpu_type) = gpu.gpu_type {
            messages.push(format!(
                "GPU: {}x {}",
                gpu.count.unwrap_or(1),
                gpu_type
            ));
        }
    }

    Ok(DeployResult {
        success: true,
        name,
        provider: "runpod".to_string(),
        instance_id: Some(pod_id),
        connection: Some(connection),
        messages,
        warnings: vec![],
    })
}
```

### Config Extraction Helper

```rust
impl RunpodProvider {
    /// Extract RunPod-specific config from SindriConfig
    async fn get_runpod_config<'a>(
        &self,
        config: &'a SindriConfig,
    ) -> Result<RunpodDeployConfig<'a>> {
        let file = config.inner();
        let runpod = file.providers.runpod.as_ref();

        // Resolve image
        let image = config
            .resolve_image(None)
            .await
            .unwrap_or_else(|_| "runpod/pytorch:latest".to_string());

        // Determine compute type
        let cpu_only = runpod.map(|r| r.cpu_only).unwrap_or(false);
        let gpu_enabled = file
            .deployment
            .resources
            .gpu
            .as_ref()
            .map(|g| g.enabled)
            .unwrap_or(true); // RunPod defaults to GPU

        let compute_type = if cpu_only || !gpu_enabled {
            "CPU"
        } else {
            "GPU"
        };

        // GPU configuration
        let gpu_type = runpod
            .and_then(|r| r.gpu_type.clone())
            .or_else(|| {
                // Fall back to generic tier mapping
                file.deployment
                    .resources
                    .gpu
                    .as_ref()
                    .and_then(|g| g.tier.as_ref())
                    .map(|t| match t {
                        sindri_core::types::GpuTier::GpuSmall => "ADA_24".to_string(),
                        sindri_core::types::GpuTier::GpuMedium => "AMPERE_48".to_string(),
                        sindri_core::types::GpuTier::GpuLarge => "AMPERE_80".to_string(),
                        sindri_core::types::GpuTier::GpuXlarge => "ADA_80_PRO".to_string(),
                    })
            });

        let gpu_type_ids = if compute_type == "GPU" {
            gpu_type
                .map(|t| vec![t])
                .unwrap_or_else(|| vec!["NVIDIA GeForce RTX 4090".to_string()])
        } else {
            vec![]
        };

        let gpu_count = runpod.map(|r| r.gpu_count).unwrap_or(
            file.deployment
                .resources
                .gpu
                .as_ref()
                .map(|g| g.count)
                .unwrap_or(1),
        );

        let vcpu_count = runpod.map(|r| r.vcpu_count).unwrap_or(4);

        // Storage
        let container_disk_gb = runpod.map(|r| r.container_disk_gb).unwrap_or(50);
        let volume_gb = runpod.map(|r| r.volume_size_gb).unwrap_or(20);
        let volume_mount_path = runpod
            .map(|r| r.volume_mount_path.clone())
            .unwrap_or_else(|| "/workspace".to_string());

        // Network
        let cloud_type = runpod
            .map(|r| r.cloud_type.clone())
            .unwrap_or_else(|| "SECURE".to_string());

        let data_center_ids = runpod
            .and_then(|r| r.region.as_ref())
            .map(|r| vec![r.clone()])
            .unwrap_or_default();

        let interruptible = runpod.map(|r| r.spot).unwrap_or(false);

        let ports = runpod
            .map(|r| r.ports.clone())
            .unwrap_or_else(|| vec!["8888/http".to_string(), "22/tcp".to_string()]);

        let network_volume_id = runpod.and_then(|r| r.network_volume_id.clone());
        let support_public_ip = runpod.map(|r| r.public_ip).unwrap_or(false);

        Ok(RunpodDeployConfig {
            name: &file.name,
            image,
            compute_type,
            gpu_type_ids,
            gpu_count,
            vcpu_count,
            container_disk_gb,
            volume_gb,
            volume_mount_path,
            cloud_type,
            data_center_ids,
            interruptible,
            ports,
            network_volume_id,
            support_public_ip,
        })
    }

    /// Build the CreatePodRequest from deploy config and resolved secrets
    fn build_create_request(
        &self,
        config: &RunpodDeployConfig<'_>,
        env_vars: HashMap<String, String>,
    ) -> Result<CreatePodRequest> {
        let env = if env_vars.is_empty() {
            None
        } else {
            Some(env_vars)
        };

        let is_gpu = config.compute_type == "GPU";

        Ok(CreatePodRequest {
            name: config.name.to_string(),
            image_name: config.image.clone(),
            compute_type: Some(config.compute_type.to_string()),
            gpu_type_ids: if is_gpu {
                Some(config.gpu_type_ids.clone())
            } else {
                None
            },
            gpu_count: if is_gpu {
                Some(config.gpu_count)
            } else {
                None
            },
            vcpu_count: if !is_gpu {
                Some(config.vcpu_count)
            } else {
                None
            },
            container_disk_in_gb: Some(config.container_disk_gb),
            volume_in_gb: Some(config.volume_gb),
            volume_mount_path: Some(config.volume_mount_path.clone()),
            cloud_type: Some(config.cloud_type.clone()),
            data_center_ids: if config.data_center_ids.is_empty() {
                None
            } else {
                Some(config.data_center_ids.clone())
            },
            interruptible: if config.interruptible {
                Some(true)
            } else {
                None
            },
            ports: Some(config.ports.clone()),
            env,
            network_volume_id: config.network_volume_id.clone(),
            support_public_ip: if config.support_public_ip {
                Some(true)
            } else {
                None
            },
        })
    }
}
```

---

## 9. Connect Strategy

RunPod pods can be accessed via SSH in two ways:

1. **Proxy SSH** (all pods): `ssh root@ssh.runpod.io -i ~/.ssh/id_ed25519`
   - Limited to terminal only, no SCP/SFTP
2. **Public IP SSH** (pods with public IP): `ssh root@<PUBLIC_IP> -p <PORT>`
   - Full SSH capabilities

### connect() Implementation

```rust
async fn connect(&self, config: &SindriConfig) -> Result<()> {
    let name = config.name();
    info!("Connecting to {} on RunPod", name);

    // Find the pod
    let pod = self
        .find_pod_by_name(name)
        .await?
        .ok_or_else(|| {
            anyhow!(
                "Pod '{}' not found on RunPod. Deploy first: sindri deploy",
                name
            )
        })?;

    // Check pod status
    let status = pod.status.as_deref().unwrap_or("UNKNOWN");
    match status {
        "RUNNING" => { /* good, connect */ }
        "EXITED" | "STOPPED" => {
            info!("Pod is stopped, starting...");
            self.start_pod(&pod.id).await?;
            self.wait_for_running(&pod.id).await?;
        }
        other => {
            return Err(anyhow!(
                "Pod is in state '{}', cannot connect. Wait for RUNNING state.",
                other
            ));
        }
    }

    // Determine SSH connection method
    let connection = self.build_connection_info(&pod);

    if let Some(ref ssh_cmd) = connection.ssh_command {
        info!("Connecting via SSH...");

        // Parse the SSH command to extract components
        // Use tokio::process::Command to run SSH interactively
        let status = tokio::process::Command::new("ssh")
            .args(self.parse_ssh_args(&pod))
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .await?;

        if !status.success() {
            return Err(anyhow!("SSH connection failed"));
        }
    } else {
        // No SSH available, show proxy URL instructions
        println!("Connect to your pod via the RunPod web terminal:");
        println!(
            "  https://www.runpod.io/console/pods/{}/terminal",
            pod.id
        );

        if let Some(ref instructions) = connection.instructions {
            println!("\n{}", instructions);
        }
    }

    Ok(())
}
```

### SSH Argument Builder

```rust
impl RunpodProvider {
    /// Build SSH arguments for connecting to a pod
    fn parse_ssh_args(&self, pod: &PodResponse) -> Vec<String> {
        // Prefer public IP with mapped port for full SSH capabilities
        if let Some(ref port_mappings) = pod.port_mappings {
            for mapping in port_mappings {
                if mapping.private_port == Some(22) {
                    if let (Some(ref ip), Some(public_port)) =
                        (&mapping.ip, mapping.public_port)
                    {
                        return vec![
                            format!("root@{}", ip),
                            "-p".to_string(),
                            public_port.to_string(),
                            "-o".to_string(),
                            "StrictHostKeyChecking=no".to_string(),
                            "-o".to_string(),
                            "UserKnownHostsFile=/dev/null".to_string(),
                        ];
                    }
                }
            }
        }

        // Fallback to proxy SSH
        vec![
            "root@ssh.runpod.io".to_string(),
            "-o".to_string(),
            "StrictHostKeyChecking=no".to_string(),
            "-o".to_string(),
            "UserKnownHostsFile=/dev/null".to_string(),
        ]
    }

    /// Build ConnectionInfo from a pod response
    fn build_connection_info(&self, pod: &PodResponse) -> ConnectionInfo {
        let pod_id = &pod.id;

        // Try to build SSH command from port mappings (public IP)
        let ssh_command = pod.port_mappings.as_ref().and_then(|mappings| {
            mappings.iter().find_map(|m| {
                if m.private_port == Some(22) {
                    if let (Some(ref ip), Some(public_port)) = (&m.ip, m.public_port) {
                        return Some(format!(
                            "ssh root@{} -p {}",
                            ip, public_port
                        ));
                    }
                }
                None
            })
        });

        // Fallback SSH command via proxy
        let ssh_command = ssh_command.or_else(|| {
            Some("ssh root@ssh.runpod.io".to_string())
        });

        // Proxy HTTP URL
        let http_url = Some(format!(
            "https://{}-8888.proxy.runpod.net",
            pod_id
        ));

        let instructions = format!(
            "Connect with:\n\
             \x20 sindri connect\n\
             \x20 {}\n\n\
             Web terminal:\n\
             \x20 https://www.runpod.io/console/pods/{}/terminal\n\n\
             Jupyter/HTTP:\n\
             \x20 https://{}-8888.proxy.runpod.net",
            ssh_command.as_deref().unwrap_or("(SSH not available)"),
            pod_id,
            pod_id
        );

        ConnectionInfo {
            ssh_command,
            http_url,
            https_url: None,
            instructions: Some(instructions),
        }
    }
}
```

---

## 10. Status Implementation

### State Mapping

| RunPod API Status | DeploymentState |
| ----------------- | --------------- |
| `CREATED`         | `Creating`      |
| `STARTING`        | `Creating`      |
| `RUNNING`         | `Running`       |
| `EXITED`          | `Stopped`       |
| `STOPPED`         | `Stopped`       |
| `TERMINATED`      | `NotDeployed`   |
| `ERROR`           | `Error`         |
| (not found)       | `NotDeployed`   |
| (anything else)   | `Unknown`       |

### status() Implementation

```rust
async fn status(&self, config: &SindriConfig) -> Result<DeploymentStatus> {
    let name = config.name().to_string();
    info!("Getting RunPod status for {}", name);

    let pod = self.find_pod_by_name(&name).await?;

    let (state, instance_id, image, details) = match pod {
        Some(pod) => {
            let state = self.map_pod_status(&pod);
            let mut details = HashMap::new();

            if let Some(ref gpu) = pod.gpu {
                if let Some(ref gpu_type) = gpu.gpu_type {
                    details.insert("gpu_type".to_string(), gpu_type.clone());
                }
                if let Some(count) = gpu.count {
                    details.insert("gpu_count".to_string(), count.to_string());
                }
            }
            if let Some(cost) = pod.cost_per_hr {
                details.insert("cost_per_hr".to_string(), format!("${:.3}", cost));
            }
            if let Some(ref status_str) = pod.status {
                details.insert("runpod_status".to_string(), status_str.clone());
            }

            let image = pod.image.clone();
            (state, Some(pod.id), image, details)
        }
        None => (
            DeploymentState::NotDeployed,
            None,
            config.resolve_image(None).await.ok(),
            HashMap::new(),
        ),
    };

    Ok(DeploymentStatus {
        name,
        provider: "runpod".to_string(),
        state,
        instance_id,
        image,
        addresses: vec![],
        resources: None,
        timestamps: Default::default(),
        details,
    })
}
```

### Status Mapping Helper

```rust
impl RunpodProvider {
    /// Map RunPod pod status string to DeploymentState
    fn map_pod_status(&self, pod: &PodResponse) -> DeploymentState {
        match pod.status.as_deref().unwrap_or("UNKNOWN") {
            "CREATED" | "STARTING" => DeploymentState::Creating,
            "RUNNING" => DeploymentState::Running,
            "EXITED" | "STOPPED" => DeploymentState::Stopped,
            "TERMINATED" => DeploymentState::NotDeployed,
            "ERROR" => DeploymentState::Error,
            _ => DeploymentState::Unknown,
        }
    }
}
```

---

## 11. Start/Stop Implementation

RunPod supports stopping pods (preserving volume data) and starting them again. This maps directly to the `start()` and `stop()` trait methods.

```rust
async fn start(&self, config: &SindriConfig) -> Result<()> {
    let name = config.name();
    info!("Starting {} on RunPod", name);

    let pod = self
        .find_pod_by_name(name)
        .await?
        .ok_or_else(|| anyhow!("Pod '{}' not found on RunPod", name))?;

    let status = pod.status.as_deref().unwrap_or("UNKNOWN");
    match status {
        "RUNNING" => {
            info!("Pod is already running");
            return Ok(());
        }
        "EXITED" | "STOPPED" => {
            self.start_pod(&pod.id).await?;
            info!("Pod start initiated. Use 'sindri status' to check progress.");
        }
        other => {
            return Err(anyhow!(
                "Pod is in state '{}', cannot start. \
                 Only stopped/exited pods can be started.",
                other
            ));
        }
    }

    Ok(())
}

async fn stop(&self, config: &SindriConfig) -> Result<()> {
    let name = config.name();
    info!("Stopping {} on RunPod", name);

    let pod = self
        .find_pod_by_name(name)
        .await?
        .ok_or_else(|| anyhow!("Pod '{}' not found on RunPod", name))?;

    let status = pod.status.as_deref().unwrap_or("UNKNOWN");
    match status {
        "RUNNING" => {
            self.stop_pod(&pod.id).await?;
            info!(
                "Pod stopped. Volume data ({} GB) is preserved. \
                 Use 'sindri start' to resume.",
                pod.volume_in_gb.unwrap_or(0)
            );
        }
        "EXITED" | "STOPPED" => {
            info!("Pod is already stopped");
        }
        other => {
            return Err(anyhow!(
                "Pod is in state '{}', cannot stop.",
                other
            ));
        }
    }

    Ok(())
}
```

---

## 12. Secrets Injection

RunPod pods accept environment variables via the `env` object in the create pod request. This is the injection mechanism for Sindri secrets.

```rust
impl RunpodProvider {
    /// Resolve secrets from config and return as env var HashMap
    async fn resolve_secrets(
        &self,
        config: &SindriConfig,
        custom_env_file: Option<PathBuf>,
    ) -> Result<HashMap<String, String>> {
        let secrets = config.secrets();

        if secrets.is_empty() {
            debug!("No secrets configured, skipping secrets resolution");
            return Ok(HashMap::new());
        }

        info!("Resolving {} secrets...", secrets.len());

        let config_dir = config
            .config_path
            .parent()
            .map(|p| p.to_path_buf().into())
            .unwrap_or_else(|| PathBuf::from("."));

        let context =
            ResolutionContext::new(config_dir).with_custom_env_file(custom_env_file);

        let resolver = SecretResolver::new(context);
        let resolved = resolver.resolve_all(secrets).await?;

        let mut env_vars = HashMap::new();
        for (name, secret) in &resolved {
            if let Some(value) = secret.value.as_string() {
                env_vars.insert(name.clone(), value.to_string());
            } else {
                warn!(
                    "RunPod provider only supports environment variable secrets. \
                     File secret '{}' will be skipped.",
                    name
                );
            }
        }

        info!("Resolved {} environment variable secrets", env_vars.len());
        Ok(env_vars)
    }
}
```

**Note**: Secrets are set at pod creation time via the `env` field in the API request. They cannot be updated after creation without terminating and recreating the pod. This is different from Fly.io which supports `flyctl secrets import` without redeployment.

---

## 13. Storage Strategy

### Three-Tier Storage Model

```
+-------------------+     +-------------------+     +-------------------+
| Container Disk    |     | Pod Volume        |     | Network Volume    |
| (Ephemeral)       |     | (Stop-persistent) |     | (Fully persistent)|
|                   |     |                   |     |                   |
| - Default: 50 GB  |     | - Default: 20 GB  |     | - User-managed    |
| - Lost on stop    |     | - Mount: /workspace|     | - Mount: /workspace|
| - $0.10/GB/month  |     | - Survives stop   |     | - Survives terminate|
| - Scratch/temp    |     | - Lost on terminate|    | - $0.07/GB/month  |
+-------------------+     +-------------------+     +-------------------+
```

### Configuration Mapping

| sindri.yaml field        | RunPod API field    | Default      |
| ------------------------ | ------------------- | ------------ |
| `runpod.containerDiskGb` | `containerDiskInGb` | 50           |
| `runpod.volumeSizeGb`    | `volumeInGb`        | 20           |
| `runpod.volumeMountPath` | `volumeMountPath`   | "/workspace" |
| `runpod.networkVolumeId` | `networkVolumeId`   | (none)       |

### Behavior by Lifecycle Event

| Event            | Container Disk | Pod Volume    | Network Volume |
| ---------------- | -------------- | ------------- | -------------- |
| Pod running      | Available      | Available     | Available      |
| `sindri stop`    | **Lost**       | **Preserved** | **Preserved**  |
| `sindri start`   | Fresh          | Reattached    | Reattached     |
| `sindri destroy` | **Lost**       | **Lost**      | **Preserved**  |

### Network Volume Constraint

Network volumes must be in the same data center as the pod. If a user specifies `networkVolumeId`, the `dataCenterIds` should either be unset (auto-select) or match the volume's data center. The provider should warn if there's a potential mismatch.

---

## 14. Error Handling

### HTTP Status Code Mapping

```rust
impl RunpodProvider {
    /// Map HTTP error status codes to user-friendly error messages
    fn map_api_error(&self, status: StatusCode, body: &str) -> anyhow::Error {
        // Try to parse structured error response
        let api_error: Option<ApiError> = serde_json::from_str(body).ok();
        let detail = api_error
            .and_then(|e| e.message.or(e.error))
            .unwrap_or_else(|| body.to_string());

        match status {
            StatusCode::BAD_REQUEST => anyhow!(
                "RunPod API error (400 Bad Request): {}\n\
                 Check your pod configuration (GPU type, region, etc.)",
                detail
            ),
            StatusCode::UNAUTHORIZED => anyhow!(
                "RunPod API authentication failed (401 Unauthorized).\n\
                 Check that RUNPOD_API_KEY is set correctly.\n\
                 Get your API key at https://www.runpod.io/console/user/settings"
            ),
            StatusCode::FORBIDDEN => anyhow!(
                "RunPod API access denied (403 Forbidden): {}\n\
                 Your API key may lack permissions for this operation.",
                detail
            ),
            StatusCode::NOT_FOUND => anyhow!(
                "RunPod resource not found (404): {}",
                detail
            ),
            StatusCode::TOO_MANY_REQUESTS => anyhow!(
                "RunPod API rate limit exceeded (429). \
                 Please wait a moment and try again."
            ),
            StatusCode::INTERNAL_SERVER_ERROR
            | StatusCode::BAD_GATEWAY
            | StatusCode::SERVICE_UNAVAILABLE => anyhow!(
                "RunPod API server error ({}): {}\n\
                 Check https://status.runpod.io for service status.",
                status.as_u16(),
                detail
            ),
            _ => anyhow!(
                "RunPod API error ({}): {}",
                status.as_u16(),
                detail
            ),
        }
    }
}
```

### Retry Strategy for Rate Limiting

While not implemented in the initial version, the architecture supports adding retry logic with exponential backoff. A future enhancement could wrap the HTTP calls:

```rust
/// Retry configuration for transient failures
const MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF: Duration = Duration::from_secs(1);

// Future enhancement: wrap API calls with retry logic
// for status codes 429 (rate limited) and 5xx (server errors)
```

### GPU Availability Errors

If GPU pod creation fails due to insufficient capacity, the error from the API will indicate this. The provider should surface this clearly:

```rust
// In create_pod error handling:
if detail.contains("insufficient") || detail.contains("out of stock") {
    return Err(anyhow!(
        "GPU type '{}' is currently unavailable in the requested region.\n\
         Suggestions:\n\
         - Try a different GPU type\n\
         - Use a GPU pool ID (e.g., 'ADA_24') for flexible selection\n\
         - Remove the region constraint\n\
         - Use spot pricing (spot: true) for wider availability",
        config.gpu_type_ids.first().unwrap_or(&"unknown".to_string())
    ));
}
```

---

## 15. Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // --- Provider creation tests ---

    #[test]
    fn test_runpod_provider_creation() {
        let provider = RunpodProvider::new().unwrap();
        assert_eq!(provider.name(), "runpod");
    }

    #[test]
    fn test_runpod_provider_with_output_dir() {
        let dir = PathBuf::from("/tmp/test-runpod");
        let provider = RunpodProvider::with_output_dir(dir.clone()).unwrap();
        assert_eq!(provider.output_dir, dir);
        assert_eq!(provider.name(), "runpod");
    }

    // --- Capability flag tests ---

    #[test]
    fn test_supports_gpu() {
        let provider = RunpodProvider::new().unwrap();
        assert!(provider.supports_gpu(), "RunPod should support GPU");
    }

    #[test]
    fn test_supports_auto_suspend() {
        let provider = RunpodProvider::new().unwrap();
        assert!(
            provider.supports_auto_suspend(),
            "RunPod should support auto-suspend (stop/start)"
        );
    }

    // --- Prerequisite check tests ---

    #[test]
    fn test_check_prerequisites_no_api_key() {
        // Temporarily unset the API key
        let original = std::env::var("RUNPOD_API_KEY").ok();
        std::env::remove_var("RUNPOD_API_KEY");

        let provider = RunpodProvider::new().unwrap();
        let status = provider.check_prerequisites().unwrap();

        assert!(!status.satisfied);
        assert!(
            status.missing.iter().any(|p| p.name == "runpod-api-key"),
            "Should report missing API key"
        );

        // Restore
        if let Some(key) = original {
            std::env::set_var("RUNPOD_API_KEY", key);
        }
    }

    #[test]
    fn test_check_prerequisites_with_api_key() {
        std::env::set_var("RUNPOD_API_KEY", "test-key-12345");

        let provider = RunpodProvider::new().unwrap();
        let status = provider.check_prerequisites().unwrap();

        assert!(status.satisfied);
        assert!(
            status.available.iter().any(|p| p.name == "runpod-api-key"),
            "Should report API key as available"
        );

        std::env::remove_var("RUNPOD_API_KEY");
    }

    // --- API response deserialization tests ---

    #[test]
    fn test_pod_response_deserialization() {
        let json = r#"{
            "id": "pod-abc123",
            "name": "my-pod",
            "image": "runpod/pytorch:latest",
            "status": "RUNNING",
            "publicIp": "1.2.3.4",
            "costPerHr": 0.44,
            "gpu": { "type": "NVIDIA GeForce RTX 4090", "count": 1 },
            "volumeInGb": 20,
            "containerDiskInGb": 50
        }"#;
        let pod: PodResponse = serde_json::from_str(json).unwrap();
        assert_eq!(pod.id, "pod-abc123");
        assert_eq!(pod.name, "my-pod");
        assert_eq!(pod.status.as_deref(), Some("RUNNING"));
        assert_eq!(pod.public_ip.as_deref(), Some("1.2.3.4"));
        assert_eq!(pod.cost_per_hr, Some(0.44));
        assert!(pod.gpu.is_some());
        let gpu = pod.gpu.unwrap();
        assert_eq!(gpu.gpu_type.as_deref(), Some("NVIDIA GeForce RTX 4090"));
        assert_eq!(gpu.count, Some(1));
    }

    #[test]
    fn test_pod_response_deserialization_minimal() {
        let json = r#"{"id": "pod-xyz", "name": "test"}"#;
        let pod: PodResponse = serde_json::from_str(json).unwrap();
        assert_eq!(pod.id, "pod-xyz");
        assert_eq!(pod.name, "test");
        assert!(pod.status.is_none());
        assert!(pod.gpu.is_none());
    }

    #[test]
    fn test_pod_list_deserialization() {
        let json = r#"[
            {"id": "pod-1", "name": "gpu-pod", "status": "RUNNING"},
            {"id": "pod-2", "name": "cpu-pod", "status": "EXITED"}
        ]"#;
        let pods: Vec<PodResponse> = serde_json::from_str(json).unwrap();
        assert_eq!(pods.len(), 2);
        assert_eq!(pods[0].id, "pod-1");
        assert_eq!(pods[0].status.as_deref(), Some("RUNNING"));
        assert_eq!(pods[1].status.as_deref(), Some("EXITED"));
    }

    #[test]
    fn test_pod_response_with_port_mappings() {
        let json = r#"{
            "id": "pod-pm",
            "name": "port-test",
            "portMappings": [
                {"privatePort": 22, "publicPort": 43215, "type": "tcp", "ip": "1.2.3.4"},
                {"privatePort": 8888, "publicPort": 43216, "type": "http", "ip": "1.2.3.4"}
            ]
        }"#;
        let pod: PodResponse = serde_json::from_str(json).unwrap();
        assert!(pod.port_mappings.is_some());
        let mappings = pod.port_mappings.unwrap();
        assert_eq!(mappings.len(), 2);
        assert_eq!(mappings[0].private_port, Some(22));
        assert_eq!(mappings[0].public_port, Some(43215));
    }

    // --- Create request serialization tests ---

    #[test]
    fn test_create_pod_request_gpu() {
        let request = CreatePodRequest {
            name: "test-pod".to_string(),
            image_name: "runpod/pytorch:latest".to_string(),
            compute_type: Some("GPU".to_string()),
            gpu_type_ids: Some(vec!["NVIDIA GeForce RTX 4090".to_string()]),
            gpu_count: Some(1),
            vcpu_count: None,
            container_disk_in_gb: Some(50),
            volume_in_gb: Some(20),
            volume_mount_path: Some("/workspace".to_string()),
            cloud_type: Some("SECURE".to_string()),
            data_center_ids: None,
            interruptible: None,
            ports: Some(vec!["8888/http".to_string(), "22/tcp".to_string()]),
            env: None,
            network_volume_id: None,
            support_public_ip: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"gpuTypeIds\""));
        assert!(json.contains("\"gpuCount\":1"));
        assert!(!json.contains("\"vcpuCount\"")); // Should be skipped
        assert!(!json.contains("\"interruptible\"")); // None should be skipped
    }

    #[test]
    fn test_create_pod_request_cpu() {
        let request = CreatePodRequest {
            name: "cpu-pod".to_string(),
            image_name: "runpod/stack:latest".to_string(),
            compute_type: Some("CPU".to_string()),
            gpu_type_ids: None,
            gpu_count: None,
            vcpu_count: Some(4),
            container_disk_in_gb: Some(50),
            volume_in_gb: Some(20),
            volume_mount_path: Some("/workspace".to_string()),
            cloud_type: Some("SECURE".to_string()),
            data_center_ids: None,
            interruptible: None,
            ports: Some(vec!["22/tcp".to_string()]),
            env: Some(HashMap::from([
                ("KEY".to_string(), "value".to_string()),
            ])),
            network_volume_id: None,
            support_public_ip: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"vcpuCount\":4"));
        assert!(!json.contains("\"gpuTypeIds\"")); // Should be skipped
        assert!(json.contains("\"env\""));
    }

    // --- Status mapping tests ---

    #[test]
    fn test_status_mapping() {
        let provider = RunpodProvider::new().unwrap();

        let test_cases = vec![
            ("CREATED", DeploymentState::Creating),
            ("STARTING", DeploymentState::Creating),
            ("RUNNING", DeploymentState::Running),
            ("EXITED", DeploymentState::Stopped),
            ("STOPPED", DeploymentState::Stopped),
            ("TERMINATED", DeploymentState::NotDeployed),
            ("ERROR", DeploymentState::Error),
            ("SOMETHING_ELSE", DeploymentState::Unknown),
        ];

        for (runpod_status, expected_state) in test_cases {
            let pod = PodResponse {
                id: "test".to_string(),
                name: "test".to_string(),
                image: None,
                status: Some(runpod_status.to_string()),
                desired_status: None,
                public_ip: None,
                port_mappings: None,
                cost_per_hr: None,
                gpu: None,
                volume_in_gb: None,
                container_disk_in_gb: None,
                machine: None,
            };
            assert_eq!(
                provider.map_pod_status(&pod),
                expected_state,
                "RunPod status '{}' should map to {:?}",
                runpod_status,
                expected_state
            );
        }
    }

    // --- Error response deserialization tests ---

    #[test]
    fn test_api_error_deserialization() {
        let json = r#"{"message": "GPU type not available"}"#;
        let error: ApiError = serde_json::from_str(json).unwrap();
        assert_eq!(error.message.as_deref(), Some("GPU type not available"));
    }

    #[test]
    fn test_api_error_deserialization_with_error_field() {
        let json = r#"{"error": "Unauthorized"}"#;
        let error: ApiError = serde_json::from_str(json).unwrap();
        assert_eq!(error.error.as_deref(), Some("Unauthorized"));
    }

    #[test]
    fn test_api_error_deserialization_empty() {
        let json = r#"{}"#;
        let error: ApiError = serde_json::from_str(json).unwrap();
        assert!(error.message.is_none());
        assert!(error.error.is_none());
    }

    // --- Error mapping tests ---

    #[test]
    fn test_error_mapping_401() {
        let provider = RunpodProvider::new().unwrap();
        let error = provider.map_api_error(StatusCode::UNAUTHORIZED, "{}");
        let msg = error.to_string();
        assert!(msg.contains("401"));
        assert!(msg.contains("RUNPOD_API_KEY"));
    }

    #[test]
    fn test_error_mapping_429() {
        let provider = RunpodProvider::new().unwrap();
        let error = provider.map_api_error(StatusCode::TOO_MANY_REQUESTS, "{}");
        let msg = error.to_string();
        assert!(msg.contains("rate limit"));
    }

    #[test]
    fn test_error_mapping_400_with_message() {
        let provider = RunpodProvider::new().unwrap();
        let body = r#"{"message": "Invalid GPU type"}"#;
        let error = provider.map_api_error(StatusCode::BAD_REQUEST, body);
        let msg = error.to_string();
        assert!(msg.contains("Invalid GPU type"));
    }

    // --- Connection info tests ---

    #[test]
    fn test_build_connection_info_with_public_ip() {
        let provider = RunpodProvider::new().unwrap();
        let pod = PodResponse {
            id: "pod-test".to_string(),
            name: "test".to_string(),
            image: None,
            status: Some("RUNNING".to_string()),
            desired_status: None,
            public_ip: Some("1.2.3.4".to_string()),
            port_mappings: Some(vec![PortMapping {
                private_port: Some(22),
                public_port: Some(43215),
                r#type: Some("tcp".to_string()),
                ip: Some("1.2.3.4".to_string()),
            }]),
            cost_per_hr: None,
            gpu: None,
            volume_in_gb: None,
            container_disk_in_gb: None,
            machine: None,
        };

        let conn = provider.build_connection_info(&pod);
        assert!(conn.ssh_command.is_some());
        let ssh = conn.ssh_command.unwrap();
        assert!(ssh.contains("1.2.3.4"));
        assert!(ssh.contains("43215"));
    }

    #[test]
    fn test_build_connection_info_proxy_fallback() {
        let provider = RunpodProvider::new().unwrap();
        let pod = PodResponse {
            id: "pod-proxy".to_string(),
            name: "test".to_string(),
            image: None,
            status: Some("RUNNING".to_string()),
            desired_status: None,
            public_ip: None,
            port_mappings: None,
            cost_per_hr: None,
            gpu: None,
            volume_in_gb: None,
            container_disk_in_gb: None,
            machine: None,
        };

        let conn = provider.build_connection_info(&pod);
        assert!(conn.ssh_command.is_some());
        let ssh = conn.ssh_command.unwrap();
        assert!(ssh.contains("ssh.runpod.io"));
    }
}
```

### Integration Tests with HTTP Mocking

In a separate test file or with the `mockito` crate:

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use mockito::{Server, Mock};

    // These tests require the mockito crate in dev-dependencies
    // and would override the API_BASE URL for testing.

    // Example structure (actual implementation depends on how
    // API_BASE is made configurable for testing):

    #[tokio::test]
    async fn test_create_pod_success() {
        // Set up mock server
        let mut server = Server::new_async().await;
        let mock = server.mock("POST", "/v1/pods")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(r#"{
                "id": "pod-mock",
                "name": "test-pod",
                "status": "CREATED"
            }"#)
            .create_async()
            .await;

        // ... test create_pod with mock URL ...
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_create_pod_401_error() {
        // ... test unauthorized response handling ...
    }

    #[tokio::test]
    async fn test_create_pod_rate_limited() {
        // ... test 429 response handling ...
    }
}
```

**Note**: To support mocking, the `API_BASE` constant could be made configurable via a private field or cfg attribute:

```rust
impl RunpodProvider {
    #[cfg(not(test))]
    const API_BASE: &'static str = "https://rest.runpod.io/v1";

    #[cfg(test)]
    fn api_base(&self) -> &str {
        // Allow override for testing
        &self.api_base_override.as_deref()
            .unwrap_or("https://rest.runpod.io/v1")
    }
}
```

Or alternatively, accept the base URL as a parameter for testing:

```rust
pub struct RunpodProvider {
    templates: TemplateRegistry,
    output_dir: PathBuf,
    /// API base URL (overridable for testing)
    api_base: String,
}
```

---

## 16. Integration Points

### 16.1 lib.rs Changes

**File**: `v3/crates/sindri-providers/src/lib.rs`

```rust
pub mod devpod;
pub mod docker;
pub mod e2b;
pub mod fly;
pub mod kubernetes;
pub mod runpod;     // <-- ADD
pub mod templates;
pub mod traits;
mod utils;

// In create_provider():
pub fn create_provider(provider: ProviderType) -> Result<Box<dyn Provider>> {
    match provider {
        // ... existing ...
        ProviderType::Runpod => Ok(Box::new(runpod::RunpodProvider::new()?)),
    }
}
```

### 16.2 Cargo.toml Changes

**File**: `v3/crates/sindri-providers/Cargo.toml`

```toml
[dependencies]
# ... existing ...
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }

[dev-dependencies]
# ... existing ...
mockito = "1.5"
```

### 16.3 JSON Schema Changes

**File**: `v3/schemas/sindri.schema.json`

1. Add `"runpod"` to `deployment.provider` enum
2. Add `runpod` object under `providers` (see Section 4)

### 16.4 config_types.rs Changes

**File**: `v3/crates/sindri-core/src/types/config_types.rs`

1. Add `Runpod` variant to `Provider` enum
2. Add `RunpodProviderConfig` struct
3. Add `runpod: Option<RunpodProviderConfig>` to `ProvidersConfig`
4. Update `Display`, `normalized()`, `supports_gpu()` for `Runpod`

---

## 17. Example sindri.yaml

### GPU Pod (Default RTX 4090)

```yaml
version: "3.0"
name: my-gpu-env
deployment:
  provider: runpod
  image: ghcr.io/pacphi/sindri:3.0.0
  resources:
    memory: 16GB
    cpus: 4
    gpu:
      enabled: true
extensions:
  profile: full
providers:
  runpod:
    gpuType: "NVIDIA GeForce RTX 4090"
    gpuCount: 1
    containerDiskGb: 100
    volumeSizeGb: 50
    cloudType: SECURE
    ports:
      - "8888/http"
      - "22/tcp"
```

### GPU Pod with Pool ID (Flexible Selection)

```yaml
version: "3.0"
name: ml-training
deployment:
  provider: runpod
  image: ghcr.io/pacphi/sindri:3.0.0
  resources:
    memory: 32GB
    cpus: 8
    gpu:
      enabled: true
      tier: gpu-large
extensions:
  profile: ml
providers:
  runpod:
    gpuType: "AMPERE_80"
    gpuCount: 1
    containerDiskGb: 200
    volumeSizeGb: 100
    region: US-CA-2
    spot: true
```

### CPU Pod

```yaml
version: "3.0"
name: api-server
deployment:
  provider: runpod
  image: ghcr.io/pacphi/sindri:3.0.0
  resources:
    memory: 8GB
    cpus: 4
extensions:
  profile: base
providers:
  runpod:
    cpuOnly: true
    vcpuCount: 4
    containerDiskGb: 50
    volumeSizeGb: 20
    cloudType: SECURE
```

### With Network Volume and Secrets

```yaml
version: "3.0"
name: persistent-dev
deployment:
  provider: runpod
  image: ghcr.io/pacphi/sindri:3.0.0
  resources:
    gpu:
      enabled: true
extensions:
  profile: full
secrets:
  - name: HF_TOKEN
    source: env
    required: true
  - name: WANDB_API_KEY
    source: env
providers:
  runpod:
    gpuType: "ADA_24"
    networkVolumeId: "vol-abc123"
    publicIp: true
    region: US-CA-2
```

---

## 18. Implementation Checklist

### Phase 1: Core Types and Configuration

- [ ] Add `Runpod` variant to `Provider` enum in `config_types.rs`
- [ ] Add `RunpodProviderConfig` struct to `config_types.rs`
- [ ] Add `runpod` field to `ProvidersConfig` struct
- [ ] Update `Display`, `normalized()`, `supports_gpu()` for `Provider::Runpod`
- [ ] Update JSON schema (`sindri.schema.json`)

### Phase 2: Provider Implementation

- [ ] Create `v3/crates/sindri-providers/src/runpod.rs`
- [ ] Implement `RunpodProvider` struct with constructors
- [ ] Implement `RunpodDeployConfig` extraction
- [ ] Implement API request/response structs with serde
- [ ] Implement HTTP client helper methods
- [ ] Implement `Provider` trait (all 9 required + 2 optional methods)
- [ ] Implement error mapping

### Phase 3: Integration

- [ ] Add `pub mod runpod;` and match arm in `lib.rs`
- [ ] Add `reqwest` to `Cargo.toml` dependencies
- [ ] Add `mockito` to `Cargo.toml` dev-dependencies

### Phase 4: Testing

- [ ] Unit tests: provider creation, capability flags
- [ ] Unit tests: prerequisite checks
- [ ] Unit tests: API response deserialization
- [ ] Unit tests: create request serialization
- [ ] Unit tests: status mapping
- [ ] Unit tests: error mapping
- [ ] Unit tests: connection info building
- [ ] Integration tests: mocked HTTP API calls (optional, phase 2)

### Phase 5: Documentation

- [ ] Create `v3/docs/providers/RUNPOD.md`
- [ ] Add RunPod to provider comparison table
- [ ] Example sindri.yaml configurations

---

## Appendix A: RunPod API Quick Reference

### Authentication

```
Authorization: Bearer <RUNPOD_API_KEY>
```

### Pod Endpoints

| Method   | Path                     | Description     |
| -------- | ------------------------ | --------------- |
| `POST`   | `/v1/pods`               | Create pod      |
| `GET`    | `/v1/pods`               | List all pods   |
| `GET`    | `/v1/pods/{podId}`       | Get pod details |
| `POST`   | `/v1/pods/{podId}/start` | Start pod       |
| `POST`   | `/v1/pods/{podId}/stop`  | Stop pod        |
| `DELETE` | `/v1/pods/{podId}`       | Terminate pod   |

### Pod Statuses

| Status       | Description                  |
| ------------ | ---------------------------- |
| `CREATED`    | Pod created, not yet running |
| `STARTING`   | Pod is starting up           |
| `RUNNING`    | Pod is running               |
| `EXITED`     | Pod has exited/stopped       |
| `STOPPED`    | Pod explicitly stopped       |
| `TERMINATED` | Pod terminated/deleted       |
| `ERROR`      | Pod in error state           |

## Appendix B: GPU Pool ID Reference

| Pool ID      | Included Models                  | VRAM (GB) |
| ------------ | -------------------------------- | --------- |
| `AMPERE_16`  | A4000, A4500, RTX 4000, RTX 2000 | 16        |
| `AMPERE_24`  | L4, A5000, RTX 3090              | 24        |
| `ADA_24`     | RTX 4090                         | 24        |
| `AMPERE_48`  | A6000, A40                       | 48        |
| `ADA_48_PRO` | L40, L40S, RTX 6000 Ada          | 48        |
| `AMPERE_80`  | A100                             | 80        |
| `ADA_80_PRO` | H100                             | 80        |
| `HOPPER_141` | H200                             | 141       |

## Appendix C: Comparison with Existing Providers

| Feature               | Docker             | Fly               | E2B            | RunPod                    |
| --------------------- | ------------------ | ----------------- | -------------- | ------------------------- |
| API Interface         | CLI subprocess     | CLI subprocess    | CLI subprocess | **HTTP REST API**         |
| GPU Support           | Runtime check      | Yes (A100, L40s)  | No             | **Yes (40+ GPU types)**   |
| Auto-suspend          | No                 | Yes (suspend)     | Yes (pause)    | **Yes (stop/start)**      |
| Storage Persistence   | Docker volumes     | Fly volumes       | No             | **3-tier storage**        |
| SSH Access            | docker exec        | flyctl ssh        | N/A            | **SSH proxy + public IP** |
| Secrets               | .env.secrets file  | flyctl secrets    | Dockerfile ENV | **Pod env vars**          |
| Auth Method           | N/A                | flyctl auth login | E2B_API_KEY    | **RUNPOD_API_KEY**        |
| Template File         | docker-compose.yml | fly.toml          | e2b.toml       | **None (API-driven)**     |
| External CLI Required | docker             | flyctl            | e2b            | **None**                  |
| Spot/Preemptible      | No                 | No                | No             | **Yes**                   |
| Cost Visibility       | Free (local)       | API query         | API query      | **Per-pod cost_per_hr**   |
