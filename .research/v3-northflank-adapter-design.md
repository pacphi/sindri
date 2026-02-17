# Northflank Adapter Design for Sindri v3

## 1. Overview

The Northflank adapter implements the Sindri v3 `Provider` trait to deploy development environments
on Northflank's Kubernetes-based PaaS. Northflank has a two-level resource hierarchy for Sindri:
**Project** (organizational container) and **Service** (the actual deployment).

**Key design decisions:**

- GPU support is available (returns `true` from `supports_gpu()`)
- Auto-suspend via pause/resume (returns `true` from `supports_auto_suspend()`)
- `stop()` maps to `northflank pause`, `start()` maps to `northflank resume`
- Authentication via `NORTHFLANK_API_TOKEN` env var or `northflank login` (browser-based)
- CLI tool: `northflank` (Node.js-based, installed via npm)
- Resource hierarchy: Project -> Deployment Service
- Persistent volumes are managed separately and attached to services
- Connect via `northflank exec` (like `kubectl exec`)

---

## 2. Struct Definition

```rust
// v3/crates/sindri-providers/src/northflank.rs

use crate::traits::Provider;
use crate::utils::{command_exists, get_command_version};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use sindri_core::config::SindriConfig;
use sindri_core::types::{
    ActionType, Address, AddressType, ConnectionInfo, DeployOptions, DeployResult,
    DeploymentPlan, DeploymentState, DeploymentStatus, DeploymentTimestamps,
    PlannedAction, PlannedResource, Prerequisite, PrerequisiteStatus, ResourceUsage,
};
use sindri_secrets::{ResolutionContext, SecretResolver};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::process::Command;
use tracing::{debug, info, warn};

pub struct NorthflankProvider {
    output_dir: PathBuf,
}

impl NorthflankProvider {
    pub fn new() -> Result<Self> {
        Ok(Self {
            output_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        })
    }

    pub fn with_output_dir(output_dir: PathBuf) -> Result<Self> {
        Ok(Self { output_dir })
    }
}
```

Note: Like RunPod, no `TemplateRegistry` field is needed. Northflank creates services via
JSON input to the CLI, not via config file generation. The JSON payloads are constructed
inline in the deploy method.

---

## 3. Internal Config Struct

```rust
/// Northflank deployment configuration extracted from SindriConfig
struct NorthflankDeployConfig<'a> {
    name: &'a str,
    project_name: String,     // Northflank project (default: "sindri-{name}")
    service_name: String,     // Northflank service name (default: name)
    compute_plan: String,     // e.g., "nf-compute-20", "nf-compute-50"
    instances: u32,           // number of instances (default: 1)
    gpu_type: Option<String>, // e.g., "nvidia-a100"
    gpu_count: u32,           // default 0
    volume_size_gb: u32,      // persistent volume size
    volume_mount_path: String, // mount path in container
    region: Option<String>,   // datacenter region
    ports: Vec<NorthflankPort>, // port configuration
    health_check: Option<NorthflankHealthCheck>,
    auto_scaling: Option<NorthflankAutoScaling>,
    cpus: u32,
    memory_mb: u32,
    image: String,            // resolved Docker image
}

struct NorthflankPort {
    name: String,
    internal_port: u16,
    public: bool,
    protocol: String,         // "TCP" or "HTTP"
}

struct NorthflankHealthCheck {
    path: String,             // e.g., "/health"
    port: u16,
    interval_secs: u32,
    timeout_secs: u32,
}

struct NorthflankAutoScaling {
    min_instances: u32,
    max_instances: u32,
    cpu_target_percent: u32,
}
```

### Config Extraction Pattern

```rust
fn get_northflank_config<'a>(&self, config: &'a SindriConfig) -> NorthflankDeployConfig<'a> {
    let file = config.inner();
    let nf = file.providers.northflank.as_ref();

    let memory_raw = file.deployment.resources.memory.as_deref().unwrap_or("2GB");
    let memory_mb = parse_memory_to_mb(memory_raw).unwrap_or(2048);
    let cpus = file.deployment.resources.cpus.unwrap_or(2);

    // GPU configuration
    let (gpu_type, gpu_count) = file
        .deployment.resources.gpu.as_ref()
        .filter(|g| g.enabled)
        .map(|g| {
            let gpu_type = nf
                .and_then(|n| n.gpu_type.clone())
                .unwrap_or_else(|| northflank_gpu_from_tier(g.tier.as_ref()));
            (Some(gpu_type), g.count)
        })
        .unwrap_or((None, 0));

    // Volume size
    let volume_size_gb = file.deployment.volumes.workspace.as_ref()
        .map(|v| parse_size_to_gb(&v.size).unwrap_or(10))
        .unwrap_or(10);

    let volume_mount_path = file.deployment.volumes.workspace.as_ref()
        .map(|v| v.path.clone())
        .unwrap_or_else(|| "/workspace".to_string());

    // Default ports: SSH (22) always included
    let mut ports = vec![NorthflankPort {
        name: "ssh".to_string(),
        internal_port: 22,
        public: false,
        protocol: "TCP".to_string(),
    }];

    // Add custom ports from config
    if let Some(ref nf_config) = nf {
        for port in &nf_config.ports {
            ports.push(NorthflankPort {
                name: port.name.clone(),
                internal_port: port.internal_port,
                public: port.public,
                protocol: port.protocol.clone().unwrap_or_else(|| "TCP".to_string()),
            });
        }
    }

    // Compute plan: map from resources to Northflank plan
    let compute_plan = nf
        .and_then(|n| n.compute_plan.clone())
        .unwrap_or_else(|| compute_plan_from_resources(cpus, memory_mb));

    // Health check
    let health_check = nf.and_then(|n| n.health_check.as_ref()).map(|h| {
        NorthflankHealthCheck {
            path: h.path.clone(),
            port: h.port,
            interval_secs: h.interval_secs.unwrap_or(30),
            timeout_secs: h.timeout_secs.unwrap_or(5),
        }
    });

    // Auto-scaling
    let auto_scaling = nf.and_then(|n| n.auto_scaling.as_ref()).map(|a| {
        NorthflankAutoScaling {
            min_instances: a.min_instances.unwrap_or(1),
            max_instances: a.max_instances.unwrap_or(3),
            cpu_target_percent: a.cpu_target_percent.unwrap_or(80),
        }
    });

    NorthflankDeployConfig {
        name: &file.name,
        project_name: nf.and_then(|n| n.project_name.clone())
            .unwrap_or_else(|| format!("sindri-{}", file.name)),
        service_name: nf.and_then(|n| n.service_name.clone())
            .unwrap_or_else(|| file.name.clone()),
        compute_plan,
        instances: nf.map(|n| n.instances).unwrap_or(1),
        gpu_type,
        gpu_count,
        volume_size_gb,
        volume_mount_path,
        region: nf.and_then(|n| n.region.clone()),
        ports,
        health_check,
        auto_scaling,
        cpus,
        memory_mb,
        image: String::new(), // Set later after image resolution
    }
}
```

### Compute Plan Mapping

```rust
/// Map CPU/memory to the closest Northflank compute plan
fn compute_plan_from_resources(cpus: u32, memory_mb: u32) -> String {
    match (cpus, memory_mb) {
        (c, m) if c <= 1 && m <= 512 => "nf-compute-10".to_string(),
        (c, m) if c <= 2 && m <= 2048 => "nf-compute-20".to_string(),
        (c, m) if c <= 4 && m <= 4096 => "nf-compute-50".to_string(),
        (c, m) if c <= 8 && m <= 8192 => "nf-compute-100".to_string(),
        _ => "nf-compute-200".to_string(),
    }
}

/// Map GPU tier to Northflank GPU type
fn northflank_gpu_from_tier(tier: Option<&GpuTier>) -> String {
    match tier {
        Some(GpuTier::GpuSmall) | Some(GpuTier::GpuMedium) => "nvidia-a10g".to_string(),
        Some(GpuTier::GpuLarge) | Some(GpuTier::GpuXlarge) => "nvidia-a100".to_string(),
        None => "nvidia-a10g".to_string(),
    }
}
```

---

## 4. Provider Trait Implementation

### 4.1 name()

```rust
fn name(&self) -> &'static str {
    "northflank"
}
```

### 4.2 check_prerequisites()

```rust
fn check_prerequisites(&self) -> Result<PrerequisiteStatus> {
    let mut missing = Vec::new();
    let mut available = Vec::new();

    // Check northflank CLI
    if command_exists("northflank") {
        let version = get_command_version("northflank", "--version")
            .unwrap_or_else(|_| "unknown".to_string());
        available.push(Prerequisite {
            name: "northflank".to_string(),
            description: "Northflank CLI for service management".to_string(),
            install_hint: None,
            version: Some(version),
        });
    } else {
        missing.push(Prerequisite {
            name: "northflank".to_string(),
            description: "Northflank CLI for service management".to_string(),
            install_hint: Some(
                "Install: npm install -g @northflank/cli".to_string()
            ),
            version: None,
        });
    }

    // Check authentication
    if self.is_authenticated() {
        available.push(Prerequisite {
            name: "northflank-auth".to_string(),
            description: "Northflank API authentication".to_string(),
            install_hint: None,
            version: None,
        });
    } else {
        missing.push(Prerequisite {
            name: "northflank-auth".to_string(),
            description: "Northflank API authentication not configured".to_string(),
            install_hint: Some(
                "Run: northflank login\n\
                 Or set NORTHFLANK_API_TOKEN environment variable".to_string()
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

Authentication check:

```rust
fn is_authenticated(&self) -> bool {
    // Check env var first
    if std::env::var("NORTHFLANK_API_TOKEN").is_ok() {
        return true;
    }
    // Try to list projects (will fail if not authenticated)
    let output = std::process::Command::new("northflank")
        .args(["list", "projects", "--output", "json"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    output.map(|s| s.success()).unwrap_or(false)
}
```

### 4.3 deploy()

Full deploy lifecycle:

```rust
async fn deploy(&self, config: &SindriConfig, opts: DeployOptions) -> Result<DeployResult> {
    let nf_config = self.get_northflank_config(config);

    // 1. Handle dry run
    if opts.dry_run {
        return Ok(DeployResult {
            success: true,
            name: nf_config.name.to_string(),
            provider: "northflank".to_string(),
            instance_id: None,
            connection: None,
            messages: vec!["Dry run: would create Northflank project and service".to_string()],
            warnings: vec![],
        });
    }

    // 2. Check for existing service
    if let Some(existing) = self.find_service(&nf_config.project_name, &nf_config.service_name).await? {
        if opts.force {
            info!("Force flag set, deleting existing service: {}", existing.id);
            self.delete_service(&nf_config.project_name, &existing.id).await?;
            // Wait for deletion to complete
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        } else {
            return Err(anyhow!(
                "Service '{}' already exists in project '{}'. Use --force to recreate.",
                nf_config.service_name, nf_config.project_name
            ));
        }
    }

    // 3. Ensure project exists
    info!("Ensuring Northflank project '{}' exists", nf_config.project_name);
    self.ensure_project(&nf_config.project_name).await?;

    // 4. Resolve secrets
    let secrets = config.secrets();
    let resolved_secrets = if !secrets.is_empty() {
        let config_dir = config.config_path().parent().unwrap_or(Path::new("."));
        let context = ResolutionContext::new(config_dir);
        let resolver = SecretResolver::new(context);
        Some(resolver.resolve_all(&secrets).await?)
    } else {
        None
    };

    // 5. Create secret group (if secrets exist)
    if let Some(ref resolved) = resolved_secrets {
        self.create_secret_group(
            &nf_config.project_name,
            &nf_config.service_name,
            resolved,
        ).await?;
    }

    // 6. Create deployment service
    info!("Creating Northflank service: {}", nf_config.service_name);
    let service_def = self.build_service_definition(&nf_config)?;

    let output = Command::new("northflank")
        .args([
            "create", "service", "deployment",
            "--project", &nf_config.project_name,
            "--input", &service_def,
            "--output", "json",
        ])
        .output()
        .await
        .context("Failed to create Northflank service")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failed to create Northflank service: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let service: NorthflankService = serde_json::from_str(&stdout)
        .context("Failed to parse service creation response")?;

    // 7. Create and attach volume (if size > 0)
    if nf_config.volume_size_gb > 0 {
        self.create_and_attach_volume(
            &nf_config.project_name,
            &service.id,
            nf_config.volume_size_gb,
            &nf_config.volume_mount_path,
        ).await?;
    }

    // 8. Wait for running (if requested)
    if opts.wait {
        let timeout = opts.timeout.unwrap_or(300);
        self.wait_for_running(&nf_config.project_name, &service.id, timeout).await?;
    }

    // 9. Build connection info
    let connection = ConnectionInfo {
        ssh_command: Some(format!(
            "northflank exec --project {} --service {}",
            nf_config.project_name, service.id
        )),
        http_url: service.ports.iter()
            .find(|p| p.public)
            .map(|p| format!("https://{}", p.dns.as_deref().unwrap_or("pending"))),
        https_url: None,
        instructions: Some(format!(
            "Shell: northflank exec --project {} --service {}\n\
             Port forward: northflank forward --project {} --service {}",
            nf_config.project_name, service.id,
            nf_config.project_name, service.id
        )),
    };

    Ok(DeployResult {
        success: true,
        name: nf_config.name.to_string(),
        provider: "northflank".to_string(),
        instance_id: Some(service.id),
        connection: Some(connection),
        messages: vec![format!(
            "Service deployed on plan {} with {} instance(s)",
            nf_config.compute_plan, nf_config.instances
        )],
        warnings: vec![],
    })
}
```

### 4.4 status()

```rust
async fn status(&self, config: &SindriConfig) -> Result<DeploymentStatus> {
    let file = config.inner();
    let name = &file.name;
    let nf = file.providers.northflank.as_ref();

    let project_name = nf.and_then(|n| n.project_name.clone())
        .unwrap_or_else(|| format!("sindri-{}", name));
    let service_name = nf.and_then(|n| n.service_name.clone())
        .unwrap_or_else(|| name.clone());

    let service = match self.find_service(&project_name, &service_name).await? {
        Some(s) => s,
        None => {
            return Ok(DeploymentStatus {
                name: name.to_string(),
                provider: "northflank".to_string(),
                state: DeploymentState::NotDeployed,
                instance_id: None,
                image: None,
                addresses: vec![],
                resources: None,
                timestamps: DeploymentTimestamps::default(),
                details: HashMap::new(),
            });
        }
    };

    let state = match service.status.as_str() {
        "running" => DeploymentState::Running,
        "paused" => DeploymentState::Paused,
        "creating" | "pending" => DeploymentState::Creating,
        "error" | "failed" => DeploymentState::Error,
        "stopped" => DeploymentState::Stopped,
        _ => DeploymentState::Unknown,
    };

    let mut addresses = Vec::new();
    for port in &service.ports {
        if let Some(ref dns) = port.dns {
            addresses.push(Address {
                r#type: if port.public { AddressType::Https } else { AddressType::Internal },
                value: dns.clone(),
                port: Some(port.internal_port),
            });
        }
    }

    let mut details = HashMap::new();
    details.insert("project".to_string(), project_name.clone());
    details.insert("compute_plan".to_string(), service.compute_plan.clone());
    details.insert("instances".to_string(), service.instances.to_string());

    Ok(DeploymentStatus {
        name: name.to_string(),
        provider: "northflank".to_string(),
        state,
        instance_id: Some(service.id.clone()),
        image: service.image.clone(),
        addresses,
        resources: service.metrics.as_ref().map(|m| ResourceUsage {
            cpu_percent: m.cpu_percent,
            memory_bytes: m.memory_bytes,
            memory_limit: m.memory_limit,
            disk_bytes: m.disk_bytes,
            disk_limit: m.disk_limit,
        }),
        timestamps: DeploymentTimestamps::default(),
        details,
    })
}
```

### 4.5 destroy()

```rust
async fn destroy(&self, config: &SindriConfig, force: bool) -> Result<()> {
    let file = config.inner();
    let name = &file.name;
    let nf = file.providers.northflank.as_ref();

    let project_name = nf.and_then(|n| n.project_name.clone())
        .unwrap_or_else(|| format!("sindri-{}", name));
    let service_name = nf.and_then(|n| n.service_name.clone())
        .unwrap_or_else(|| name.clone());

    let service = self.find_service(&project_name, &service_name).await?
        .ok_or_else(|| anyhow!("No Northflank service found for '{}'", name))?;

    info!("Destroying Northflank service: {} in project {}", service.id, project_name);

    // Delete service (cascading volumes if attached)
    self.delete_service(&project_name, &service.id).await?;

    info!("Service {} destroyed", service.id);
    info!("Note: Project '{}' was preserved. Delete manually if no longer needed.", project_name);

    Ok(())
}
```

### 4.6 connect()

```rust
async fn connect(&self, config: &SindriConfig) -> Result<()> {
    let file = config.inner();
    let name = &file.name;
    let nf = file.providers.northflank.as_ref();

    let project_name = nf.and_then(|n| n.project_name.clone())
        .unwrap_or_else(|| format!("sindri-{}", name));
    let service_name = nf.and_then(|n| n.service_name.clone())
        .unwrap_or_else(|| name.clone());

    let service = self.find_service(&project_name, &service_name).await?
        .ok_or_else(|| anyhow!("No Northflank service found for '{}'. Deploy first.", name))?;

    // Check if paused and auto-resume
    if service.status == "paused" {
        info!("Service is paused, resuming...");
        self.resume_service(&project_name, &service.id).await?;
        // Wait briefly for resume
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }

    info!("Connecting to Northflank service: {}", service.id);

    // Use northflank exec for shell access
    let status = Command::new("northflank")
        .args([
            "exec",
            "--project", &project_name,
            "--service", &service.id,
        ])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .await
        .context("Failed to connect to Northflank service")?;

    if !status.success() {
        return Err(anyhow!("Shell connection to service {} failed", service.id));
    }

    Ok(())
}
```

### 4.7 plan()

```rust
async fn plan(&self, config: &SindriConfig) -> Result<DeploymentPlan> {
    let nf_config = self.get_northflank_config(config);

    let mut actions = vec![
        PlannedAction {
            action: ActionType::Create,
            resource: "northflank-project".to_string(),
            description: format!("Ensure project '{}' exists", nf_config.project_name),
        },
        PlannedAction {
            action: ActionType::Create,
            resource: "northflank-service".to_string(),
            description: format!(
                "Create deployment service '{}' on plan {}",
                nf_config.service_name, nf_config.compute_plan
            ),
        },
    ];

    if nf_config.volume_size_gb > 0 {
        actions.push(PlannedAction {
            action: ActionType::Create,
            resource: "northflank-volume".to_string(),
            description: format!(
                "Create {}GB volume mounted at {}",
                nf_config.volume_size_gb, nf_config.volume_mount_path
            ),
        });
    }

    let resources = vec![
        PlannedResource {
            resource_type: "northflank-service".to_string(),
            name: nf_config.service_name.clone(),
            config: HashMap::from([
                ("project".to_string(), serde_json::Value::String(nf_config.project_name.clone())),
                ("compute_plan".to_string(), serde_json::Value::String(nf_config.compute_plan.clone())),
                ("instances".to_string(), serde_json::json!(nf_config.instances)),
                ("volume_gb".to_string(), serde_json::json!(nf_config.volume_size_gb)),
            ]),
        },
    ];

    Ok(DeploymentPlan {
        provider: "northflank".to_string(),
        actions,
        resources,
        estimated_cost: None, // Northflank pricing is plan-based, hard to estimate
    })
}
```

### 4.8 start() / stop()

```rust
async fn start(&self, config: &SindriConfig) -> Result<()> {
    let (project_name, service_id) = self.resolve_service_ids(config).await?;

    info!("Resuming Northflank service: {}", service_id);
    self.resume_service(&project_name, &service_id).await
}

async fn stop(&self, config: &SindriConfig) -> Result<()> {
    let (project_name, service_id) = self.resolve_service_ids(config).await?;

    info!("Pausing Northflank service: {}", service_id);
    self.pause_service(&project_name, &service_id).await
}
```

### 4.9 Capability Flags

```rust
fn supports_gpu(&self) -> bool {
    true  // Northflank supports GPU workloads
}

fn supports_auto_suspend(&self) -> bool {
    true  // Northflank supports pause/resume
}
```

---

## 5. API Response Deserialization Structs

```rust
/// Northflank service response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NorthflankService {
    id: String,
    name: String,
    status: String,       // "running", "paused", "creating", "error"
    image: Option<String>,
    compute_plan: String,
    instances: u32,
    ports: Vec<NorthflankServicePort>,
    metrics: Option<NorthflankMetrics>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NorthflankServicePort {
    name: String,
    internal_port: u16,
    public: bool,
    dns: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NorthflankMetrics {
    cpu_percent: Option<f64>,
    memory_bytes: Option<u64>,
    memory_limit: Option<u64>,
    disk_bytes: Option<u64>,
    disk_limit: Option<u64>,
}

/// Northflank project response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NorthflankProject {
    id: String,
    name: String,
}
```

---

## 6. Helper Methods

```rust
impl NorthflankProvider {
    /// Ensure a project exists (create if not)
    async fn ensure_project(&self, project_name: &str) -> Result<()> {
        // Check if project exists
        let output = Command::new("northflank")
            .args([
                "get", "project",
                "--project", project_name,
                "--output", "json",
            ])
            .output()
            .await?;

        if output.status.success() {
            debug!("Project '{}' already exists", project_name);
            return Ok(());
        }

        // Create project
        info!("Creating Northflank project: {}", project_name);
        let project_def = serde_json::json!({
            "name": project_name,
            "description": format!("Sindri development environment for {}", project_name)
        });

        let output = Command::new("northflank")
            .args([
                "create", "project",
                "--input", &project_def.to_string(),
                "--output", "json",
            ])
            .output()
            .await
            .context("Failed to create Northflank project")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to create project: {}", stderr));
        }

        Ok(())
    }

    /// Find a service by name in a project
    async fn find_service(
        &self,
        project_name: &str,
        service_name: &str,
    ) -> Result<Option<NorthflankService>> {
        let output = Command::new("northflank")
            .args([
                "list", "services",
                "--project", project_name,
                "--output", "json",
            ])
            .output()
            .await
            .context("Failed to list Northflank services")?;

        if !output.status.success() {
            // Project may not exist
            return Ok(None);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let services: Vec<NorthflankService> = serde_json::from_str(&stdout)
            .unwrap_or_default();

        Ok(services.into_iter().find(|s| s.name == service_name))
    }

    /// Delete a service
    async fn delete_service(&self, project_name: &str, service_id: &str) -> Result<()> {
        let output = Command::new("northflank")
            .args([
                "delete", "service",
                "--project", project_name,
                "--service", service_id,
            ])
            .output()
            .await
            .context("Failed to delete Northflank service")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to delete service: {}", stderr));
        }
        Ok(())
    }

    /// Pause a service (cost savings)
    async fn pause_service(&self, project_name: &str, service_id: &str) -> Result<()> {
        let output = Command::new("northflank")
            .args([
                "pause",
                "--project", project_name,
                "--service", service_id,
            ])
            .output()
            .await
            .context("Failed to pause service")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to pause service: {}", stderr));
        }
        Ok(())
    }

    /// Resume a paused service
    async fn resume_service(&self, project_name: &str, service_id: &str) -> Result<()> {
        let output = Command::new("northflank")
            .args([
                "resume",
                "--project", project_name,
                "--service", service_id,
            ])
            .output()
            .await
            .context("Failed to resume service")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to resume service: {}", stderr));
        }
        Ok(())
    }

    /// Create a secret group for environment variables
    async fn create_secret_group(
        &self,
        project_name: &str,
        service_name: &str,
        secrets: &HashMap<String, String>,
    ) -> Result<()> {
        let secret_group_name = format!("{}-secrets", service_name);
        let env_vars: HashMap<String, serde_json::Value> = secrets.iter()
            .map(|(k, v)| (k.clone(), serde_json::json!({ "value": v, "type": "secret" })))
            .collect();

        let secret_def = serde_json::json!({
            "name": secret_group_name,
            "secretType": "environment",
            "priority": 10,
            "variables": env_vars
        });

        let output = Command::new("northflank")
            .args([
                "create", "secret",
                "--project", project_name,
                "--input", &secret_def.to_string(),
            ])
            .output()
            .await
            .context("Failed to create secret group")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Failed to create secret group (may already exist): {}", stderr);
        }

        Ok(())
    }

    /// Create and attach a persistent volume
    async fn create_and_attach_volume(
        &self,
        project_name: &str,
        service_id: &str,
        size_gb: u32,
        mount_path: &str,
    ) -> Result<()> {
        let volume_def = serde_json::json!({
            "name": format!("{}-data", service_id),
            "size": size_gb * 1024, // MB
            "mountPath": mount_path
        });

        let output = Command::new("northflank")
            .args([
                "create", "volume",
                "--project", project_name,
                "--service", service_id,
                "--input", &volume_def.to_string(),
            ])
            .output()
            .await
            .context("Failed to create volume")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Volume creation warning: {}", stderr);
        }

        Ok(())
    }

    /// Build JSON service definition for northflank create
    fn build_service_definition(&self, config: &NorthflankDeployConfig) -> Result<String> {
        let ports: Vec<serde_json::Value> = config.ports.iter()
            .map(|p| serde_json::json!({
                "name": p.name,
                "internalPort": p.internal_port,
                "public": p.public,
                "protocol": p.protocol,
            }))
            .collect();

        let mut service_def = serde_json::json!({
            "name": config.service_name,
            "description": format!("Sindri development environment: {}", config.name),
            "billing": {
                "deploymentPlan": config.compute_plan
            },
            "deployment": {
                "instances": config.instances,
                "external": {
                    "imagePath": config.image
                },
                "docker": {
                    "configType": "default"
                }
            },
            "ports": ports
        });

        // Add health check if configured
        if let Some(ref hc) = config.health_check {
            service_def["healthChecks"] = serde_json::json!([{
                "protocol": "HTTP",
                "path": hc.path,
                "port": hc.port,
                "initialDelaySeconds": 30,
                "periodSeconds": hc.interval_secs,
                "timeoutSeconds": hc.timeout_secs,
            }]);
        }

        // Add auto-scaling if configured
        if let Some(ref auto) = config.auto_scaling {
            service_def["deployment"]["scaling"] = serde_json::json!({
                "minInstances": auto.min_instances,
                "maxInstances": auto.max_instances,
                "targetCpu": auto.cpu_target_percent
            });
        }

        Ok(service_def.to_string())
    }

    /// Resolve project name and service ID from config
    async fn resolve_service_ids(&self, config: &SindriConfig) -> Result<(String, String)> {
        let file = config.inner();
        let nf = file.providers.northflank.as_ref();

        let project_name = nf.and_then(|n| n.project_name.clone())
            .unwrap_or_else(|| format!("sindri-{}", file.name));
        let service_name = nf.and_then(|n| n.service_name.clone())
            .unwrap_or_else(|| file.name.clone());

        let service = self.find_service(&project_name, &service_name).await?
            .ok_or_else(|| anyhow!("No Northflank service found for '{}'", file.name))?;

        Ok((project_name, service.id))
    }

    /// Wait for service to reach running state
    async fn wait_for_running(
        &self,
        project_name: &str,
        service_id: &str,
        timeout_secs: u64,
    ) -> Result<()> {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);

        loop {
            if start.elapsed() > timeout {
                return Err(anyhow!(
                    "Service {} did not reach running state within {} seconds",
                    service_id, timeout_secs
                ));
            }

            let output = Command::new("northflank")
                .args([
                    "get", "service",
                    "--project", project_name,
                    "--service", service_id,
                    "--output", "json",
                ])
                .output()
                .await?;

            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Ok(service) = serde_json::from_str::<NorthflankService>(&stdout) {
                    if service.status == "running" {
                        return Ok(());
                    }
                }
            }

            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    }
}
```

---

## 7. Secrets Handling

Northflank uses **secret groups** attached to projects. Secrets are environment variables
injected into running containers.

Pattern:

1. Create secret group with `northflank create secret`
2. Link secret group to service (automatic via project scope)
3. Secrets are available as env vars in the container

This is closer to Fly's pattern (platform-managed secrets) than Docker's (file-based).

---

## 8. Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let provider = NorthflankProvider::new().unwrap();
        assert_eq!(provider.name(), "northflank");
    }

    #[test]
    fn test_with_output_dir() {
        let dir = PathBuf::from("/tmp/test-northflank");
        let provider = NorthflankProvider::with_output_dir(dir.clone()).unwrap();
        assert_eq!(provider.output_dir, dir);
    }

    #[test]
    fn test_supports_gpu() {
        let provider = NorthflankProvider::new().unwrap();
        assert!(provider.supports_gpu());
    }

    #[test]
    fn test_supports_auto_suspend() {
        let provider = NorthflankProvider::new().unwrap();
        assert!(provider.supports_auto_suspend());
    }

    #[test]
    fn test_check_prerequisites_does_not_panic() {
        let provider = NorthflankProvider::new().unwrap();
        let result = provider.check_prerequisites();
        assert!(result.is_ok());
    }

    #[test]
    fn test_compute_plan_mapping() {
        assert_eq!(compute_plan_from_resources(1, 512), "nf-compute-10");
        assert_eq!(compute_plan_from_resources(2, 2048), "nf-compute-20");
        assert_eq!(compute_plan_from_resources(4, 4096), "nf-compute-50");
        assert_eq!(compute_plan_from_resources(8, 8192), "nf-compute-100");
        assert_eq!(compute_plan_from_resources(16, 16384), "nf-compute-200");
    }

    #[test]
    fn test_service_response_deserialization() {
        let json = r#"{
            "id": "svc-abc123",
            "name": "my-service",
            "status": "running",
            "image": "ghcr.io/org/sindri:latest",
            "computePlan": "nf-compute-50",
            "instances": 1,
            "ports": [
                {
                    "name": "ssh",
                    "internalPort": 22,
                    "public": false,
                    "dns": null
                },
                {
                    "name": "http",
                    "internalPort": 8080,
                    "public": true,
                    "dns": "my-service.example.northflank.app"
                }
            ],
            "metrics": {
                "cpuPercent": 15.5,
                "memoryBytes": 1073741824,
                "memoryLimit": 4294967296,
                "diskBytes": null,
                "diskLimit": null
            }
        }"#;

        let service: NorthflankService = serde_json::from_str(json).unwrap();
        assert_eq!(service.id, "svc-abc123");
        assert_eq!(service.name, "my-service");
        assert_eq!(service.status, "running");
        assert_eq!(service.instances, 1);
        assert_eq!(service.ports.len(), 2);
        assert!(service.ports[1].public);
    }

    #[test]
    fn test_service_status_mapping() {
        let test_cases = vec![
            ("running", DeploymentState::Running),
            ("paused", DeploymentState::Paused),
            ("creating", DeploymentState::Creating),
            ("pending", DeploymentState::Creating),
            ("error", DeploymentState::Error),
            ("failed", DeploymentState::Error),
            ("stopped", DeploymentState::Stopped),
            ("unknown-state", DeploymentState::Unknown),
        ];

        for (input, expected) in test_cases {
            let state = match input {
                "running" => DeploymentState::Running,
                "paused" => DeploymentState::Paused,
                "creating" | "pending" => DeploymentState::Creating,
                "error" | "failed" => DeploymentState::Error,
                "stopped" => DeploymentState::Stopped,
                _ => DeploymentState::Unknown,
            };
            assert_eq!(state, expected);
        }
    }

    #[test]
    fn test_gpu_tier_mapping() {
        assert_eq!(northflank_gpu_from_tier(Some(&GpuTier::GpuSmall)), "nvidia-a10g");
        assert_eq!(northflank_gpu_from_tier(Some(&GpuTier::GpuLarge)), "nvidia-a100");
        assert_eq!(northflank_gpu_from_tier(None), "nvidia-a10g");
    }
}
```

---

## 9. File Changes Required

| File                                              | Change                                                                                           |
| ------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `v3/crates/sindri-providers/src/northflank.rs`    | **NEW** - Full provider implementation                                                           |
| `v3/crates/sindri-providers/src/lib.rs`           | Add `pub mod northflank;` + match arm                                                            |
| `v3/crates/sindri-core/src/types/config_types.rs` | Add `Northflank` to Provider enum, `NorthflankProviderConfig` struct, field in `ProvidersConfig` |
| `v3/schemas/sindri.schema.json`                   | Add "northflank" to provider enum, add providers.northflank schema                               |

No template file needed -- Northflank service definitions are built as inline JSON.
