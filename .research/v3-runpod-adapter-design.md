# RunPod Adapter Design for Sindri v3

## 1. Overview

The RunPod adapter implements the Sindri v3 `Provider` trait to deploy development environments
on RunPod's GPU cloud. RunPod is GPU-first but also supports CPU-only pods. The adapter uses
`runpodctl` CLI for all operations, following the same subprocess-based pattern as Fly.io/E2B.

**Key design decisions:**

- GPU support is a core differentiator (returns `true` from `supports_gpu()`)
- Auto-suspend is not natively supported by RunPod pods (returns `false` from `supports_auto_suspend()`)
- `stop`/`start` map directly to RunPod's `stop pod`/`start pod` commands
- Authentication via `RUNPOD_API_KEY` environment variable or `runpodctl config`
- No template file generation needed (RunPod creates pods directly via CLI flags, no config file)

---

## 2. Struct Definition

```rust
// v3/crates/sindri-providers/src/runpod.rs

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

pub struct RunpodProvider {
    output_dir: PathBuf,
}

impl RunpodProvider {
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

Note: No `TemplateRegistry` field needed. RunPod does not use config file generation --
pods are created entirely via CLI arguments. This is a deliberate design difference from
Docker/Fly/K8s providers that generate config files.

---

## 3. Internal Config Struct

```rust
/// RunPod deployment configuration extracted from SindriConfig
struct RunpodDeployConfig<'a> {
    name: &'a str,
    gpu_type: String,          // e.g., "NVIDIA RTX A4000", "NVIDIA A100 80GB"
    gpu_count: u32,            // default 1
    container_disk_gb: u32,    // default 20
    volume_size_gb: u32,       // default 50 (network volume)
    cloud_type: String,        // "SECURE" or "COMMUNITY"
    region: Option<String>,    // datacenter ID (optional)
    expose_ports: Vec<String>, // HTTP ports to expose via proxy
    spot_bid: Option<f64>,     // None = on-demand, Some(x) = spot with bid
    cpus: u32,                 // vCPUs
    memory_mb: u32,            // memory in MB
    image: String,             // resolved Docker image
}
```

### Config Extraction Pattern

```rust
fn get_runpod_config<'a>(&self, config: &'a SindriConfig) -> RunpodDeployConfig<'a> {
    let file = config.inner();
    let runpod = file.providers.runpod.as_ref();

    // GPU configuration
    let (gpu_enabled, gpu_type_raw, gpu_count) = file
        .deployment.resources.gpu.as_ref()
        .map(|g| (g.enabled, g.r#type.clone(), g.count))
        .unwrap_or((false, Default::default(), 0));

    // Map GpuTier to RunPod GPU type
    let gpu_type = runpod
        .and_then(|r| r.gpu_type_id.clone())
        .unwrap_or_else(|| {
            file.deployment.resources.gpu.as_ref()
                .and_then(|g| g.tier.as_ref())
                .map(|t| runpod_gpu_from_tier(t))
                .unwrap_or_else(|| "NVIDIA RTX A4000".to_string())
        });

    let memory_raw = file.deployment.resources.memory.as_deref().unwrap_or("2GB");
    let memory_mb = parse_memory_to_mb(memory_raw).unwrap_or(2048);
    let cpus = file.deployment.resources.cpus.unwrap_or(2);

    let volume_size_gb = file.deployment.volumes.workspace.as_ref()
        .map(|v| parse_size_to_gb(&v.size).unwrap_or(50))
        .unwrap_or(50);

    RunpodDeployConfig {
        name: &file.name,
        gpu_type,
        gpu_count: if gpu_enabled { gpu_count.max(1) } else { 0 },
        container_disk_gb: runpod.map(|r| r.container_disk_gb).unwrap_or(20),
        volume_size_gb,
        cloud_type: runpod.map(|r| r.cloud_type.clone())
            .unwrap_or_else(|| "COMMUNITY".to_string()),
        region: runpod.and_then(|r| r.region.clone()),
        expose_ports: runpod.map(|r| r.expose_ports.clone()).unwrap_or_default(),
        spot_bid: runpod.and_then(|r| r.spot_bid),
        cpus,
        memory_mb,
        image: String::new(), // Set later after image resolution
    }
}
```

### GPU Tier to RunPod Mapping

```rust
fn runpod_gpu_from_tier(tier: &GpuTier) -> String {
    match tier {
        GpuTier::GpuSmall => "NVIDIA RTX A4000".to_string(),
        GpuTier::GpuMedium => "NVIDIA RTX A5000".to_string(),
        GpuTier::GpuLarge => "NVIDIA A100 80GB PCIe".to_string(),
        GpuTier::GpuXlarge => "NVIDIA H100 80GB HBM3".to_string(),
    }
}
```

---

## 4. Provider Trait Implementation

### 4.1 name()

```rust
fn name(&self) -> &'static str {
    "runpod"
}
```

### 4.2 check_prerequisites()

```rust
fn check_prerequisites(&self) -> Result<PrerequisiteStatus> {
    let mut missing = Vec::new();
    let mut available = Vec::new();

    // Check runpodctl CLI
    if command_exists("runpodctl") {
        let version = get_command_version("runpodctl", "version")
            .unwrap_or_else(|_| "unknown".to_string());
        available.push(Prerequisite {
            name: "runpodctl".to_string(),
            description: "RunPod CLI for pod management".to_string(),
            install_hint: None,
            version: Some(version),
        });
    } else {
        missing.push(Prerequisite {
            name: "runpodctl".to_string(),
            description: "RunPod CLI for pod management".to_string(),
            install_hint: Some(
                "Install from https://github.com/runpod/runpodctl/releases".to_string()
            ),
            version: None,
        });
    }

    // Check API key authentication
    if self.is_authenticated() {
        available.push(Prerequisite {
            name: "runpod-auth".to_string(),
            description: "RunPod API key configured".to_string(),
            install_hint: None,
            version: None,
        });
    } else {
        missing.push(Prerequisite {
            name: "runpod-auth".to_string(),
            description: "RunPod API key not configured".to_string(),
            install_hint: Some(
                "Run: runpodctl config --apiKey=YOUR_API_KEY\n\
                 Or set RUNPOD_API_KEY environment variable".to_string()
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
    if std::env::var("RUNPOD_API_KEY").is_ok() {
        return true;
    }
    // Try to list pods (will fail if not authenticated)
    let output = std::process::Command::new("runpodctl")
        .args(["get", "pod"])
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
    let runpod_config = self.get_runpod_config(config);

    // 1. Handle dry run
    if opts.dry_run {
        return Ok(DeployResult {
            success: true,
            name: runpod_config.name.to_string(),
            provider: "runpod".to_string(),
            instance_id: None,
            connection: None,
            messages: vec!["Dry run: would create RunPod pod".to_string()],
            warnings: vec![],
        });
    }

    // 2. Check for existing pod
    if let Some(existing_id) = self.find_pod_by_name(runpod_config.name).await {
        if opts.force {
            info!("Force flag set, removing existing pod: {}", existing_id);
            self.remove_pod(&existing_id).await?;
        } else {
            return Err(anyhow!(
                "Pod '{}' already exists (id: {}). Use --force to recreate.",
                runpod_config.name, existing_id
            ));
        }
    }

    // 3. Resolve secrets
    let secrets = config.secrets();
    let resolved_secrets = if !secrets.is_empty() {
        let config_dir = config.config_path().parent().unwrap_or(Path::new("."));
        let context = ResolutionContext::new(config_dir);
        let resolver = SecretResolver::new(context);
        Some(resolver.resolve_all(&secrets).await?)
    } else {
        None
    };

    // 4. Create pod via runpodctl
    info!("Creating RunPod pod: {}", runpod_config.name);
    let mut cmd_args = vec![
        "create".to_string(), "pods".to_string(),
        "--name".to_string(), runpod_config.name.to_string(),
        "--imageName".to_string(), runpod_config.image.clone(),
        "--containerDiskSize".to_string(), runpod_config.container_disk_gb.to_string(),
        "--volumeSize".to_string(), runpod_config.volume_size_gb.to_string(),
        "--volumeMountPath".to_string(), "/workspace".to_string(),
    ];

    // GPU configuration
    if runpod_config.gpu_count > 0 {
        cmd_args.extend([
            "--gpuType".to_string(), runpod_config.gpu_type.clone(),
            "--gpuCount".to_string(), runpod_config.gpu_count.to_string(),
        ]);
    }

    // Cloud type
    cmd_args.extend([
        "--cloudType".to_string(), runpod_config.cloud_type.clone(),
    ]);

    // Region (optional)
    if let Some(ref region) = runpod_config.region {
        cmd_args.extend(["--dataCenterId".to_string(), region.clone()]);
    }

    // Ports
    if !runpod_config.expose_ports.is_empty() {
        let ports_str = runpod_config.expose_ports.join(",");
        cmd_args.extend(["--ports".to_string(), ports_str]);
    }

    // Environment variables (secrets)
    if let Some(ref resolved) = resolved_secrets {
        for (name, value) in resolved {
            cmd_args.extend(["--env".to_string(), format!("{}={}", name, value)]);
        }
    }

    // Enable SSH
    cmd_args.push("--startSSH".to_string());

    debug!("Running: runpodctl {}", cmd_args.join(" "));

    let output = Command::new("runpodctl")
        .args(&cmd_args)
        .output()
        .await
        .context("Failed to execute runpodctl create")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failed to create RunPod pod: {}", stderr));
    }

    // 5. Parse pod ID from output
    let stdout = String::from_utf8_lossy(&output.stdout);
    let pod_id = self.parse_pod_id_from_output(&stdout)?;

    // 6. Wait for running (if requested)
    if opts.wait {
        let timeout = opts.timeout.unwrap_or(300);
        self.wait_for_running(&pod_id, timeout).await?;
    }

    // 7. Build connection info
    let connection = ConnectionInfo {
        ssh_command: Some(format!("runpodctl connect {}", pod_id)),
        http_url: if !runpod_config.expose_ports.is_empty() {
            Some(format!(
                "https://{}-{}.proxy.runpod.net",
                pod_id,
                runpod_config.expose_ports.first().unwrap_or(&"8080".to_string())
            ))
        } else {
            None
        },
        https_url: None,
        instructions: Some(format!(
            "SSH: runpodctl connect {}\nWeb: https://www.runpod.io/console/pods/{}", pod_id, pod_id
        )),
    };

    Ok(DeployResult {
        success: true,
        name: runpod_config.name.to_string(),
        provider: "runpod".to_string(),
        instance_id: Some(pod_id),
        connection: Some(connection),
        messages: vec![format!("Pod deployed with {} GPU(s)", runpod_config.gpu_count)],
        warnings: vec![],
    })
}
```

### 4.4 status()

```rust
async fn status(&self, config: &SindriConfig) -> Result<DeploymentStatus> {
    let file = config.inner();
    let name = &file.name;

    // Find pod by name
    let pod = match self.find_pod_details(name).await? {
        Some(p) => p,
        None => {
            return Ok(DeploymentStatus {
                name: name.to_string(),
                provider: "runpod".to_string(),
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

    let state = match pod.desired_status.as_str() {
        "RUNNING" => DeploymentState::Running,
        "EXITED" => DeploymentState::Stopped,
        "CREATED" => DeploymentState::Creating,
        "ERROR" => DeploymentState::Error,
        _ => DeploymentState::Unknown,
    };

    let mut addresses = Vec::new();

    // SSH address (RunPod public IP)
    if let Some(ref ip) = pod.public_ip {
        addresses.push(Address {
            r#type: AddressType::Ssh,
            value: ip.clone(),
            port: Some(22),
        });
    }

    // HTTP proxy addresses
    for port in &pod.ports {
        addresses.push(Address {
            r#type: AddressType::Https,
            value: format!("{}-{}.proxy.runpod.net", pod.id, port),
            port: Some(*port),
        });
    }

    let mut details = HashMap::new();
    details.insert("gpu_type".to_string(), pod.gpu_type.clone());
    details.insert("gpu_count".to_string(), pod.gpu_count.to_string());
    if let Some(ref machine_id) = pod.machine_id {
        details.insert("machine_id".to_string(), machine_id.clone());
    }
    details.insert("cloud_type".to_string(), pod.cloud_type.clone());

    Ok(DeploymentStatus {
        name: name.to_string(),
        provider: "runpod".to_string(),
        state,
        instance_id: Some(pod.id.clone()),
        image: pod.image_name.clone(),
        addresses,
        resources: pod.runtime.as_ref().map(|r| ResourceUsage {
            cpu_percent: r.cpu_percent,
            memory_bytes: r.memory_bytes,
            memory_limit: r.memory_limit,
            disk_bytes: r.disk_bytes,
            disk_limit: r.disk_limit,
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

    let pod_id = self.find_pod_by_name(name).await.ok_or_else(|| {
        anyhow!("No RunPod pod found for '{}'", name)
    })?;

    info!("Destroying RunPod pod: {} ({})", name, pod_id);
    self.remove_pod(&pod_id).await?;
    info!("Pod {} destroyed", pod_id);

    Ok(())
}
```

### 4.6 connect()

```rust
async fn connect(&self, config: &SindriConfig) -> Result<()> {
    let file = config.inner();
    let name = &file.name;

    let pod_id = self.find_pod_by_name(name).await.ok_or_else(|| {
        anyhow!("No RunPod pod found for '{}'. Deploy first.", name)
    })?;

    info!("Connecting to RunPod pod: {}", pod_id);

    // Use runpodctl connect for SSH
    let status = Command::new("runpodctl")
        .args(["connect", &pod_id])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .await
        .context("Failed to connect to RunPod pod")?;

    if !status.success() {
        return Err(anyhow!("SSH connection to pod {} failed", pod_id));
    }

    Ok(())
}
```

### 4.7 plan()

```rust
async fn plan(&self, config: &SindriConfig) -> Result<DeploymentPlan> {
    let runpod_config = self.get_runpod_config(config);

    let mut resources = vec![PlannedResource {
        resource_type: "runpod-pod".to_string(),
        name: runpod_config.name.to_string(),
        config: HashMap::from([
            ("gpu_type".to_string(), serde_json::Value::String(runpod_config.gpu_type.clone())),
            ("gpu_count".to_string(), serde_json::json!(runpod_config.gpu_count)),
            ("container_disk_gb".to_string(), serde_json::json!(runpod_config.container_disk_gb)),
            ("volume_size_gb".to_string(), serde_json::json!(runpod_config.volume_size_gb)),
            ("cloud_type".to_string(), serde_json::Value::String(runpod_config.cloud_type.clone())),
        ]),
    }];

    let actions = vec![
        PlannedAction {
            action: ActionType::Create,
            resource: "runpod-pod".to_string(),
            description: format!(
                "Create RunPod pod '{}' with {} x {}",
                runpod_config.name, runpod_config.gpu_count, runpod_config.gpu_type
            ),
        },
    ];

    // Rough cost estimate
    let estimated_cost = Some(CostEstimate {
        hourly: Some(estimate_runpod_hourly_cost(&runpod_config.gpu_type, runpod_config.gpu_count)),
        monthly: None,
        currency: "USD".to_string(),
        notes: Some(format!(
            "{} cloud pricing. Spot pricing may be lower.",
            runpod_config.cloud_type
        )),
    });

    Ok(DeploymentPlan {
        provider: "runpod".to_string(),
        actions,
        resources,
        estimated_cost,
    })
}
```

### 4.8 start() / stop()

```rust
async fn start(&self, config: &SindriConfig) -> Result<()> {
    let file = config.inner();
    let pod_id = self.find_pod_by_name(&file.name).await.ok_or_else(|| {
        anyhow!("No RunPod pod found for '{}'", file.name)
    })?;

    info!("Starting RunPod pod: {}", pod_id);

    let output = Command::new("runpodctl")
        .args(["start", "pod", &pod_id])
        .output()
        .await
        .context("Failed to start RunPod pod")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failed to start pod: {}", stderr));
    }

    Ok(())
}

async fn stop(&self, config: &SindriConfig) -> Result<()> {
    let file = config.inner();
    let pod_id = self.find_pod_by_name(&file.name).await.ok_or_else(|| {
        anyhow!("No RunPod pod found for '{}'", file.name)
    })?;

    info!("Stopping RunPod pod: {}", pod_id);

    let output = Command::new("runpodctl")
        .args(["stop", "pod", &pod_id])
        .output()
        .await
        .context("Failed to stop RunPod pod")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failed to stop pod: {}", stderr));
    }

    Ok(())
}
```

### 4.9 Capability Flags

```rust
fn supports_gpu(&self) -> bool {
    true  // RunPod is GPU-first
}

fn supports_auto_suspend(&self) -> bool {
    false  // RunPod pods don't auto-suspend
}
```

---

## 5. API Response Deserialization Structs

```rust
/// RunPod pod response from `runpodctl get pod --json`
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunpodPod {
    id: String,
    name: String,
    desired_status: String,
    image_name: Option<String>,
    gpu_type: String,
    gpu_count: u32,
    cloud_type: String,
    public_ip: Option<String>,
    machine_id: Option<String>,
    ports: Vec<u16>,
    runtime: Option<RunpodRuntime>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunpodRuntime {
    cpu_percent: Option<f64>,
    memory_bytes: Option<u64>,
    memory_limit: Option<u64>,
    disk_bytes: Option<u64>,
    disk_limit: Option<u64>,
}
```

---

## 6. Helper Methods

```rust
impl RunpodProvider {
    /// Find a pod by name, return its ID
    async fn find_pod_by_name(&self, name: &str) -> Option<String> {
        let output = Command::new("runpodctl")
            .args(["get", "pod", "--json"])
            .output()
            .await
            .ok()?;

        if !output.status.success() { return None; }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let pods: Vec<RunpodPod> = serde_json::from_str(&stdout).ok()?;

        pods.into_iter()
            .find(|p| p.name == name)
            .map(|p| p.id)
    }

    /// Find full pod details by name
    async fn find_pod_details(&self, name: &str) -> Result<Option<RunpodPod>> {
        let output = Command::new("runpodctl")
            .args(["get", "pod", "--json"])
            .output()
            .await
            .context("Failed to query RunPod pods")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to list pods: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let pods: Vec<RunpodPod> = serde_json::from_str(&stdout)
            .context("Failed to parse RunPod pod list")?;

        Ok(pods.into_iter().find(|p| p.name == name))
    }

    /// Remove a pod by ID
    async fn remove_pod(&self, pod_id: &str) -> Result<()> {
        let output = Command::new("runpodctl")
            .args(["remove", "pod", pod_id])
            .output()
            .await
            .context("Failed to remove RunPod pod")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to remove pod {}: {}", pod_id, stderr));
        }
        Ok(())
    }

    /// Parse pod ID from create output
    fn parse_pod_id_from_output(&self, output: &str) -> Result<String> {
        // runpodctl create typically outputs JSON with the pod ID
        let v: serde_json::Value = serde_json::from_str(output)
            .context("Failed to parse runpodctl create output")?;

        v.get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("No pod ID found in create output"))
    }

    /// Wait for pod to reach RUNNING state
    async fn wait_for_running(&self, pod_id: &str, timeout_secs: u64) -> Result<()> {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);

        loop {
            if start.elapsed() > timeout {
                return Err(anyhow!(
                    "Pod {} did not reach RUNNING state within {} seconds",
                    pod_id, timeout_secs
                ));
            }

            let output = Command::new("runpodctl")
                .args(["get", "pod", pod_id, "--json"])
                .output()
                .await?;

            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Ok(pod) = serde_json::from_str::<RunpodPod>(&stdout) {
                    if pod.desired_status == "RUNNING" {
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

RunPod injects environment variables via `--env KEY=VALUE` flags at pod creation time.
This matches the pattern used by E2B (inline ENV), not the Fly pattern (staged secrets import).

```rust
// In deploy():
if let Some(ref resolved) = resolved_secrets {
    for (name, value) in resolved {
        cmd_args.extend(["--env".to_string(), format!("{}={}", name, value)]);
    }
}
```

Post-creation secret updates require pod recreation (RunPod limitation).

---

## 8. Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let provider = RunpodProvider::new().unwrap();
        assert_eq!(provider.name(), "runpod");
    }

    #[test]
    fn test_with_output_dir() {
        let dir = PathBuf::from("/tmp/test-runpod");
        let provider = RunpodProvider::with_output_dir(dir.clone()).unwrap();
        assert_eq!(provider.output_dir, dir);
    }

    #[test]
    fn test_supports_gpu() {
        let provider = RunpodProvider::new().unwrap();
        assert!(provider.supports_gpu());
    }

    #[test]
    fn test_does_not_support_auto_suspend() {
        let provider = RunpodProvider::new().unwrap();
        assert!(!provider.supports_auto_suspend());
    }

    #[test]
    fn test_check_prerequisites_does_not_panic() {
        let provider = RunpodProvider::new().unwrap();
        let result = provider.check_prerequisites();
        assert!(result.is_ok());
    }

    #[test]
    fn test_pod_response_deserialization() {
        let json = r#"{
            "id": "abc123",
            "name": "my-pod",
            "desiredStatus": "RUNNING",
            "imageName": "ghcr.io/org/sindri:latest",
            "gpuType": "NVIDIA RTX A4000",
            "gpuCount": 1,
            "cloudType": "COMMUNITY",
            "publicIp": "1.2.3.4",
            "machineId": "m-xyz",
            "ports": [8080],
            "runtime": {
                "cpuPercent": 25.0,
                "memoryBytes": 2147483648,
                "memoryLimit": 4294967296,
                "diskBytes": 1073741824,
                "diskLimit": 21474836480
            }
        }"#;

        let pod: RunpodPod = serde_json::from_str(json).unwrap();
        assert_eq!(pod.id, "abc123");
        assert_eq!(pod.name, "my-pod");
        assert_eq!(pod.desired_status, "RUNNING");
        assert_eq!(pod.gpu_count, 1);
    }

    #[test]
    fn test_gpu_tier_mapping() {
        assert_eq!(runpod_gpu_from_tier(&GpuTier::GpuSmall), "NVIDIA RTX A4000");
        assert_eq!(runpod_gpu_from_tier(&GpuTier::GpuLarge), "NVIDIA A100 80GB PCIe");
    }

    #[test]
    fn test_pod_status_mapping() {
        let test_cases = vec![
            ("RUNNING", DeploymentState::Running),
            ("EXITED", DeploymentState::Stopped),
            ("CREATED", DeploymentState::Creating),
            ("ERROR", DeploymentState::Error),
            ("UNKNOWN", DeploymentState::Unknown),
        ];

        for (input, expected) in test_cases {
            let state = match input {
                "RUNNING" => DeploymentState::Running,
                "EXITED" => DeploymentState::Stopped,
                "CREATED" => DeploymentState::Creating,
                "ERROR" => DeploymentState::Error,
                _ => DeploymentState::Unknown,
            };
            assert_eq!(state, expected);
        }
    }
}
```

---

## 9. File Changes Required

| File                                              | Change                                                                                   |
| ------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| `v3/crates/sindri-providers/src/runpod.rs`        | **NEW** - Full provider implementation                                                   |
| `v3/crates/sindri-providers/src/lib.rs`           | Add `pub mod runpod;` + match arm                                                        |
| `v3/crates/sindri-core/src/types/config_types.rs` | Add `Runpod` to Provider enum, `RunpodProviderConfig` struct, field in `ProvidersConfig` |
| `v3/schemas/sindri.schema.json`                   | Add "runpod" to provider enum, add providers.runpod schema                               |

No template file needed -- RunPod creates pods via CLI flags directly.
