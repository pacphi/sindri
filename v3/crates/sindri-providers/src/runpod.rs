//! RunPod provider implementation
//!
//! Deploys Sindri development environments on RunPod's GPU cloud.
//! Uses `runpodctl` CLI for pod lifecycle management.

use crate::traits::Provider;
use crate::utils::{command_exists, get_command_version};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use sindri_core::config::SindriConfig;
use sindri_core::types::{
    ActionType, Address, AddressType, ConnectionInfo, CostEstimate, DeployOptions, DeployResult,
    DeploymentPlan, DeploymentState, DeploymentStatus, DeploymentTimestamps, PlannedAction,
    PlannedResource, Prerequisite, PrerequisiteStatus, ResourceUsage,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, info};

/// RunPod provider for GPU cloud deployment
pub struct RunpodProvider {
    /// Output directory for generated files
    #[allow(dead_code)]
    output_dir: PathBuf,
}

impl RunpodProvider {
    /// Create a new RunPod provider
    pub fn new() -> Result<Self> {
        Ok(Self {
            output_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        })
    }

    /// Create with a specific output directory
    pub fn with_output_dir(output_dir: PathBuf) -> Result<Self> {
        Ok(Self { output_dir })
    }

    /// Check if RunPod API key is configured
    fn is_authenticated(&self) -> bool {
        // Check env var first
        if std::env::var("RUNPOD_API_KEY").is_ok() {
            return true;
        }
        // Try listing pods to verify auth
        let output = std::process::Command::new("runpodctl")
            .args(["get", "pod"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        output.map(|s| s.success()).unwrap_or(false)
    }

    /// Find a pod by name, return its ID
    async fn find_pod_by_name(&self, name: &str) -> Option<String> {
        let output = Command::new("runpodctl")
            .args(["get", "pod", "--json"])
            .output()
            .await
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let pods: Vec<RunpodPod> = serde_json::from_str(&stdout).ok()?;

        pods.into_iter().find(|p| p.name == name).map(|p| p.id)
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
        let pods: Vec<RunpodPod> =
            serde_json::from_str(&stdout).context("Failed to parse RunPod pod list")?;

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
        let v: serde_json::Value =
            serde_json::from_str(output).context("Failed to parse runpodctl create output")?;

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
                    pod_id,
                    timeout_secs
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

    /// Get RunPod configuration from SindriConfig
    fn get_runpod_config<'a>(&self, config: &'a SindriConfig) -> RunpodDeployConfig<'a> {
        let file = config.inner();
        let runpod = file.providers.runpod.as_ref();

        // Memory in MB
        let memory_raw = file.deployment.resources.memory.as_deref().unwrap_or("2GB");
        let memory_mb = parse_memory_to_mb(memory_raw).unwrap_or(2048);
        let cpus = file.deployment.resources.cpus.unwrap_or(2);

        // GPU configuration
        let gpu_count = file
            .deployment
            .resources
            .gpu
            .as_ref()
            .map(|g| if g.enabled { g.count.max(1) } else { 0 })
            .unwrap_or(0);

        // GPU type: prefer explicit gpuTypeId, then derive from tier
        let gpu_type = runpod
            .and_then(|r| r.gpu_type_id.clone())
            .unwrap_or_else(|| {
                file.deployment
                    .resources
                    .gpu
                    .as_ref()
                    .and_then(|g| g.tier.as_ref())
                    .map(runpod_gpu_from_tier)
                    .unwrap_or_else(|| "NVIDIA RTX A4000".to_string())
            });

        // Volume size
        let volume_size_gb = file
            .deployment
            .volumes
            .workspace
            .as_ref()
            .map(|v| parse_size_to_gb(&v.size).unwrap_or(50))
            .unwrap_or(50);

        RunpodDeployConfig {
            name: &file.name,
            gpu_type,
            gpu_count,
            container_disk_gb: runpod.map(|r| r.container_disk_gb).unwrap_or(20),
            volume_size_gb,
            cloud_type: runpod
                .map(|r| r.cloud_type.clone())
                .unwrap_or_else(|| "COMMUNITY".to_string()),
            region: runpod.and_then(|r| r.region.clone()),
            expose_ports: runpod.map(|r| r.expose_ports.clone()).unwrap_or_default(),
            spot_bid: runpod.and_then(|r| r.spot_bid),
            cpus,
            memory_mb,
        }
    }
}

/// RunPod deployment configuration extracted from SindriConfig
struct RunpodDeployConfig<'a> {
    name: &'a str,
    gpu_type: String,
    gpu_count: u32,
    container_disk_gb: u32,
    volume_size_gb: u32,
    cloud_type: String,
    region: Option<String>,
    expose_ports: Vec<String>,
    #[allow(dead_code)]
    spot_bid: Option<f64>,
    #[allow(dead_code)]
    cpus: u32,
    #[allow(dead_code)]
    memory_mb: u32,
}

/// RunPod pod response from `runpodctl get pod --json`
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunpodPod {
    id: String,
    name: String,
    desired_status: String,
    image_name: Option<String>,
    #[serde(default)]
    gpu_type: String,
    #[serde(default)]
    gpu_count: u32,
    #[serde(default)]
    cloud_type: String,
    #[serde(default)]
    public_ip: Option<String>,
    #[serde(default)]
    machine_id: Option<String>,
    #[serde(default)]
    ports: Vec<u16>,
    #[serde(default)]
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

/// Map GPU tier to RunPod GPU type identifier
fn runpod_gpu_from_tier(tier: &sindri_core::types::GpuTier) -> String {
    use sindri_core::types::GpuTier;
    match tier {
        GpuTier::GpuSmall => "NVIDIA RTX A4000".to_string(),
        GpuTier::GpuMedium => "NVIDIA RTX A5000".to_string(),
        GpuTier::GpuLarge => "NVIDIA A100 80GB PCIe".to_string(),
        GpuTier::GpuXlarge => "NVIDIA H100 80GB HBM3".to_string(),
    }
}

/// Parse memory string (e.g., "4GB", "512MB") to megabytes
fn parse_memory_to_mb(memory: &str) -> Option<u32> {
    let memory = memory.trim();
    if let Some(gb) = memory.strip_suffix("GB") {
        gb.parse::<u32>().ok().map(|v| v * 1024)
    } else if let Some(mb) = memory.strip_suffix("MB") {
        mb.parse::<u32>().ok()
    } else {
        None
    }
}

/// Parse size string to GB
fn parse_size_to_gb(size: &str) -> Option<u32> {
    let size = size.trim();
    if let Some(gb) = size.strip_suffix("GB") {
        gb.parse::<u32>().ok()
    } else if let Some(mb) = size.strip_suffix("MB") {
        mb.parse::<u32>().ok().map(|v| v / 1024)
    } else {
        None
    }
}

/// Estimate hourly cost for a RunPod GPU type
fn estimate_runpod_hourly_cost(gpu_type: &str, gpu_count: u32) -> f64 {
    let per_gpu = match gpu_type {
        t if t.contains("A4000") => 0.20,
        t if t.contains("A5000") => 0.30,
        t if t.contains("4090") => 0.44,
        t if t.contains("A100") => 1.10,
        t if t.contains("H100") => 2.50,
        _ => 0.30,
    };
    per_gpu * gpu_count as f64
}

#[async_trait]
impl Provider for RunpodProvider {
    fn name(&self) -> &'static str {
        "runpod"
    }

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
                    "Install from https://github.com/runpod/runpodctl/releases".to_string(),
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
                     Or set RUNPOD_API_KEY environment variable"
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

    async fn deploy(&self, config: &SindriConfig, opts: DeployOptions) -> Result<DeployResult> {
        let runpod_config = self.get_runpod_config(config);

        // Handle dry run
        if opts.dry_run {
            let _plan = self.plan(config).await?;
            return Ok(DeployResult {
                success: true,
                name: runpod_config.name.to_string(),
                provider: "runpod".to_string(),
                instance_id: None,
                connection: None,
                messages: vec![format!(
                    "Dry run: would create RunPod pod with {} x {}",
                    runpod_config.gpu_count, runpod_config.gpu_type
                )],
                warnings: vec![],
            });
        }

        // Check for existing pod
        if let Some(existing_id) = self.find_pod_by_name(runpod_config.name).await {
            if opts.force {
                info!("Force flag set, removing existing pod: {}", existing_id);
                self.remove_pod(&existing_id).await?;
                // Wait for cleanup
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            } else {
                return Err(anyhow!(
                    "Pod '{}' already exists (id: {}). Use --force to recreate.",
                    runpod_config.name,
                    existing_id
                ));
            }
        }

        // Resolve image
        let image = config
            .inner()
            .deployment
            .image
            .clone()
            .ok_or_else(|| anyhow!("No image configured for RunPod deployment"))?;

        // Build runpodctl create command
        info!("Creating RunPod pod: {}", runpod_config.name);
        let mut cmd_args = vec![
            "create".to_string(),
            "pods".to_string(),
            "--name".to_string(),
            runpod_config.name.to_string(),
            "--imageName".to_string(),
            image.clone(),
            "--containerDiskSize".to_string(),
            runpod_config.container_disk_gb.to_string(),
            "--volumeSize".to_string(),
            runpod_config.volume_size_gb.to_string(),
            "--volumeMountPath".to_string(),
            "/workspace".to_string(),
        ];

        // GPU configuration
        if runpod_config.gpu_count > 0 {
            cmd_args.extend([
                "--gpuType".to_string(),
                runpod_config.gpu_type.clone(),
                "--gpuCount".to_string(),
                runpod_config.gpu_count.to_string(),
            ]);
        }

        // Cloud type
        cmd_args.extend(["--cloudType".to_string(), runpod_config.cloud_type.clone()]);

        // Region
        if let Some(ref region) = runpod_config.region {
            cmd_args.extend(["--dataCenterId".to_string(), region.clone()]);
        }

        // Ports
        if !runpod_config.expose_ports.is_empty() {
            let ports_str = runpod_config.expose_ports.join(",");
            cmd_args.extend(["--ports".to_string(), ports_str]);
        }

        // SSH
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

        // Parse pod ID
        let stdout = String::from_utf8_lossy(&output.stdout);
        let pod_id = self.parse_pod_id_from_output(&stdout)?;

        // Wait for running
        if opts.wait {
            let timeout = opts.timeout.unwrap_or(300);
            self.wait_for_running(&pod_id, timeout).await?;
        }

        // Build connection info
        let connection = ConnectionInfo {
            ssh_command: Some(format!("runpodctl connect {}", pod_id)),
            http_url: runpod_config
                .expose_ports
                .first()
                .map(|port| format!("https://{}-{}.proxy.runpod.net", pod_id, port)),
            https_url: None,
            instructions: Some(format!(
                "SSH: runpodctl connect {}\nWeb: https://www.runpod.io/console/pods/{}",
                pod_id, pod_id
            )),
        };

        Ok(DeployResult {
            success: true,
            name: runpod_config.name.to_string(),
            provider: "runpod".to_string(),
            instance_id: Some(pod_id),
            connection: Some(connection),
            messages: vec![format!(
                "Pod deployed with {} GPU(s)",
                runpod_config.gpu_count
            )],
            warnings: vec![],
        })
    }

    async fn connect(&self, config: &SindriConfig) -> Result<()> {
        let file = config.inner();
        let name = &file.name;

        let pod_id = self
            .find_pod_by_name(name)
            .await
            .ok_or_else(|| anyhow!("No RunPod pod found for '{}'. Deploy first.", name))?;

        info!("Connecting to RunPod pod: {}", pod_id);

        let status = Command::new("runpodctl")
            .args(["connect", &pod_id])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await
            .context("Failed to connect to RunPod pod")?;

        if !status.success() {
            return Err(anyhow!("SSH connection to pod {} failed", pod_id));
        }

        Ok(())
    }

    async fn status(&self, config: &SindriConfig) -> Result<DeploymentStatus> {
        let file = config.inner();
        let name = &file.name;

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

        if let Some(ref ip) = pod.public_ip {
            addresses.push(Address {
                r#type: AddressType::Ssh,
                value: ip.clone(),
                port: Some(22),
            });
        }

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

    async fn destroy(&self, config: &SindriConfig, _force: bool) -> Result<()> {
        let file = config.inner();
        let name = &file.name;

        let pod_id = self
            .find_pod_by_name(name)
            .await
            .ok_or_else(|| anyhow!("No RunPod pod found for '{}'", name))?;

        info!("Destroying RunPod pod: {} ({})", name, pod_id);
        self.remove_pod(&pod_id).await?;
        info!("Pod {} destroyed", pod_id);

        Ok(())
    }

    async fn plan(&self, config: &SindriConfig) -> Result<DeploymentPlan> {
        let runpod_config = self.get_runpod_config(config);

        let resources = vec![PlannedResource {
            resource_type: "runpod-pod".to_string(),
            name: runpod_config.name.to_string(),
            config: HashMap::from([
                (
                    "gpu_type".to_string(),
                    serde_json::Value::String(runpod_config.gpu_type.clone()),
                ),
                (
                    "gpu_count".to_string(),
                    serde_json::json!(runpod_config.gpu_count),
                ),
                (
                    "container_disk_gb".to_string(),
                    serde_json::json!(runpod_config.container_disk_gb),
                ),
                (
                    "volume_size_gb".to_string(),
                    serde_json::json!(runpod_config.volume_size_gb),
                ),
                (
                    "cloud_type".to_string(),
                    serde_json::Value::String(runpod_config.cloud_type.clone()),
                ),
            ]),
        }];

        let actions = vec![PlannedAction {
            action: ActionType::Create,
            resource: "runpod-pod".to_string(),
            description: format!(
                "Create RunPod pod '{}' with {} x {}",
                runpod_config.name, runpod_config.gpu_count, runpod_config.gpu_type
            ),
        }];

        let estimated_cost = Some(CostEstimate {
            hourly: Some(estimate_runpod_hourly_cost(
                &runpod_config.gpu_type,
                runpod_config.gpu_count,
            )),
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

    async fn start(&self, config: &SindriConfig) -> Result<()> {
        let file = config.inner();
        let pod_id = self
            .find_pod_by_name(&file.name)
            .await
            .ok_or_else(|| anyhow!("No RunPod pod found for '{}'", file.name))?;

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
        let pod_id = self
            .find_pod_by_name(&file.name)
            .await
            .ok_or_else(|| anyhow!("No RunPod pod found for '{}'", file.name))?;

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

    fn supports_gpu(&self) -> bool {
        true
    }

    fn supports_auto_suspend(&self) -> bool {
        false
    }
}

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
        use sindri_core::types::GpuTier;
        assert_eq!(runpod_gpu_from_tier(&GpuTier::GpuSmall), "NVIDIA RTX A4000");
        assert_eq!(
            runpod_gpu_from_tier(&GpuTier::GpuLarge),
            "NVIDIA A100 80GB PCIe"
        );
    }

    #[test]
    fn test_parse_memory_to_mb() {
        assert_eq!(parse_memory_to_mb("4GB"), Some(4096));
        assert_eq!(parse_memory_to_mb("512MB"), Some(512));
        assert_eq!(parse_memory_to_mb("invalid"), None);
    }

    #[test]
    fn test_parse_size_to_gb() {
        assert_eq!(parse_size_to_gb("50GB"), Some(50));
        assert_eq!(parse_size_to_gb("2048MB"), Some(2));
        assert_eq!(parse_size_to_gb("bad"), None);
    }

    #[test]
    fn test_cost_estimation() {
        let cost = estimate_runpod_hourly_cost("NVIDIA RTX A4000", 1);
        assert!(cost > 0.0);

        let cost_2 = estimate_runpod_hourly_cost("NVIDIA A100 80GB PCIe", 2);
        assert!(cost_2 > cost);
    }
}
