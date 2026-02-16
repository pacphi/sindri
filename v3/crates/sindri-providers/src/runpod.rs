//! RunPod provider implementation
//!
//! Deploys Sindri development environments on RunPod's GPU cloud.
//! Uses the RunPod REST API (v1) at `https://rest.runpod.io/v1` for pod lifecycle management.
//! No external CLI tool installation required -- only a `RUNPOD_API_KEY` environment variable.

use crate::traits::Provider;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use sindri_core::config::SindriConfig;
use sindri_core::types::{
    ActionType, Address, AddressType, ConnectionInfo, CostEstimate, DeployOptions, DeployResult,
    DeploymentPlan, DeploymentState, DeploymentStatus, DeploymentTimestamps, PlannedAction,
    PlannedResource, Prerequisite, PrerequisiteStatus, ResourceUsage,
};
use sindri_secrets::{ResolutionContext, SecretResolver};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// Default RunPod REST API base URL
const RUNPOD_API_BASE: &str = "https://rest.runpod.io/v1";

/// Persistent state for a RunPod deployment
#[derive(Debug, Serialize, Deserialize)]
pub struct RunpodState {
    /// RunPod pod ID
    pub pod_id: String,
    /// Application name (from sindri config)
    pub app_name: String,
    /// GPU type used
    pub gpu_type: String,
    /// Number of GPUs
    pub gpu_count: u32,
    /// Container image used
    pub image: Option<String>,
    /// Timestamp when the pod was created
    pub created_at: String,
}

/// RunPod provider for GPU cloud deployment
pub struct RunpodProvider {
    /// Output directory for generated files
    #[allow(dead_code)]
    output_dir: PathBuf,
    /// HTTP client for RunPod REST API
    client: reqwest::Client,
    /// API base URL (overridable for testing)
    api_base: String,
}

impl RunpodProvider {
    /// Create a new RunPod provider
    pub fn new() -> Result<Self> {
        let client = build_http_client()?;
        Ok(Self {
            output_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            client,
            api_base: RUNPOD_API_BASE.to_string(),
        })
    }

    /// Create with a specific output directory
    pub fn with_output_dir(output_dir: PathBuf) -> Result<Self> {
        let client = build_http_client()?;
        Ok(Self {
            output_dir,
            client,
            api_base: RUNPOD_API_BASE.to_string(),
        })
    }

    /// Create with a custom API base URL and HTTP client (for testing)
    #[doc(hidden)]
    pub fn with_client(client: reqwest::Client, api_base: String) -> Self {
        Self {
            output_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            client,
            api_base,
        }
    }

    /// Get the state file path for a given app name
    fn state_file_path(app_name: &str) -> PathBuf {
        let base = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".sindri")
            .join("state");
        base.join(format!("runpod-{}.json", app_name))
    }

    /// Save deployment state to disk
    fn save_state(state: &RunpodState) -> Result<()> {
        let path = Self::state_file_path(&state.app_name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create .sindri/state directory")?;
        }
        let json =
            serde_json::to_string_pretty(state).context("Failed to serialize RunPod state")?;
        std::fs::write(&path, json)
            .with_context(|| format!("Failed to write state file: {}", path.display()))?;
        debug!("Saved RunPod state to {}", path.display());
        Ok(())
    }

    /// Load deployment state from disk
    fn load_state(app_name: &str) -> Option<RunpodState> {
        let path = Self::state_file_path(app_name);
        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<RunpodState>(&content) {
                Ok(state) => Some(state),
                Err(e) => {
                    warn!(
                        "Corrupted RunPod state file at {}: {}. Ignoring.",
                        path.display(),
                        e
                    );
                    None
                }
            },
            Err(_) => None,
        }
    }

    /// Remove state file from disk
    fn remove_state(app_name: &str) {
        let path = Self::state_file_path(app_name);
        if path.exists() {
            if let Err(e) = std::fs::remove_file(&path) {
                warn!("Failed to remove state file {}: {}", path.display(), e);
            } else {
                debug!("Removed RunPod state file: {}", path.display());
            }
        }
    }

    /// Get the API key from the environment
    fn get_api_key() -> Option<String> {
        std::env::var("RUNPOD_API_KEY")
            .ok()
            .filter(|k| !k.is_empty())
    }

    /// Check if RunPod API key is configured
    fn is_authenticated(&self) -> bool {
        Self::get_api_key().is_some()
    }

    /// List all pods via REST API (GET /v1/pods)
    async fn list_pods(&self) -> Result<Vec<RunpodPod>> {
        let url = format!("{}/pods", self.api_base);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to list RunPod pods")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Failed to list pods (HTTP {}): {}", status, body));
        }

        let pods: Vec<RunpodPod> = resp
            .json()
            .await
            .context("Failed to parse RunPod pod list")?;
        Ok(pods)
    }

    /// Find a pod by name, return its ID
    async fn find_pod_by_name(&self, name: &str) -> Option<String> {
        let pods = self.list_pods().await.ok()?;
        pods.into_iter().find(|p| p.name == name).map(|p| p.id)
    }

    /// Find full pod details by name
    async fn find_pod_details(&self, name: &str) -> Result<Option<RunpodPod>> {
        let pods = self.list_pods().await?;
        Ok(pods.into_iter().find(|p| p.name == name))
    }

    /// Get a single pod by ID via REST API (GET /v1/pods/{podId})
    async fn get_pod(&self, pod_id: &str) -> Result<RunpodPod> {
        let url = format!("{}/pods/{}", self.api_base, pod_id);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to get RunPod pod")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Failed to get pod {} (HTTP {}): {}",
                pod_id,
                status,
                body
            ));
        }

        let pod: RunpodPod = resp
            .json()
            .await
            .context("Failed to parse RunPod pod details")?;
        Ok(pod)
    }

    /// Create a pod via REST API (POST /v1/pods)
    async fn create_pod_api(&self, request: &CreatePodRequest) -> Result<RunpodPod> {
        let url = format!("{}/pods", self.api_base);
        let resp = self
            .client
            .post(&url)
            .json(request)
            .send()
            .await
            .context("Failed to create RunPod pod")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Failed to create RunPod pod (HTTP {}): {}",
                status,
                body
            ));
        }

        let pod: RunpodPod = resp
            .json()
            .await
            .context("Failed to parse RunPod create pod response")?;
        Ok(pod)
    }

    /// Terminate (delete) a pod by ID via REST API (DELETE /v1/pods/{podId})
    async fn terminate_pod(&self, pod_id: &str) -> Result<()> {
        let url = format!("{}/pods/{}", self.api_base, pod_id);
        let resp = self
            .client
            .delete(&url)
            .send()
            .await
            .context("Failed to terminate RunPod pod")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Failed to terminate pod {} (HTTP {}): {}",
                pod_id,
                status,
                body
            ));
        }
        Ok(())
    }

    /// Start a pod by ID via REST API (POST /v1/pods/{podId}/start)
    async fn start_pod_api(&self, pod_id: &str) -> Result<()> {
        let url = format!("{}/pods/{}/start", self.api_base, pod_id);
        let resp = self
            .client
            .post(&url)
            .send()
            .await
            .context("Failed to start RunPod pod")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Failed to start pod {} (HTTP {}): {}",
                pod_id,
                status,
                body
            ));
        }
        Ok(())
    }

    /// Stop a pod by ID via REST API (POST /v1/pods/{podId}/stop)
    async fn stop_pod_api(&self, pod_id: &str) -> Result<()> {
        let url = format!("{}/pods/{}/stop", self.api_base, pod_id);
        let resp = self
            .client
            .post(&url)
            .send()
            .await
            .context("Failed to stop RunPod pod")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Failed to stop pod {} (HTTP {}): {}",
                pod_id,
                status,
                body
            ));
        }
        Ok(())
    }

    /// Wait for pod to reach RUNNING state by polling the REST API
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

            if let Ok(pod) = self.get_pod(pod_id).await {
                let status_str = pod
                    .status
                    .as_deref()
                    .or(pod.desired_status.as_deref())
                    .unwrap_or("");
                if status_str == "RUNNING" {
                    return Ok(());
                }
            }

            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    }

    /// Get RunPod configuration from SindriConfig
    fn get_runpod_config<'a>(&self, config: &'a SindriConfig) -> RunpodDeployConfig<'a> {
        let file = config.inner();
        let runpod = file.providers.runpod.as_ref();

        let memory_raw = file.deployment.resources.memory.as_deref().unwrap_or("2GB");
        let memory_mb = parse_memory_to_mb(memory_raw).unwrap_or(2048);
        let cpus = file.deployment.resources.cpus.unwrap_or(2);

        let gpu_count = file
            .deployment
            .resources
            .gpu
            .as_ref()
            .map(|g| if g.enabled { g.count.max(1) } else { 0 })
            .unwrap_or(0);

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
                .map(|r| format!("{:?}", r.cloud_type))
                .unwrap_or_else(|| "COMMUNITY".to_string()),
            region: runpod.and_then(|r| r.region.clone()),
            expose_ports: runpod
                .map(|r| r.expose_ports.iter().map(|p| p.to_string()).collect())
                .unwrap_or_default(),
            spot_bid: runpod.and_then(|r| r.spot_bid),
            cpus,
            memory_mb,
        }
    }

    /// Resolve secrets from config and return as KEY=VALUE pairs for env injection
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

        let context = ResolutionContext::new(config_dir).with_custom_env_file(custom_env_file);

        let resolver = SecretResolver::new(context);
        let resolved = resolver.resolve_all(secrets).await?;

        let mut env_vars = HashMap::new();
        for (name, secret) in &resolved {
            if let Some(value) = secret.value.as_string() {
                env_vars.insert(name.clone(), value.to_string());
            } else {
                warn!(
                    "RunPod provider currently only supports environment variable secrets. \
                     File secret '{}' will be skipped.",
                    name
                );
            }
        }

        info!("Resolved {} environment variable secrets", env_vars.len());
        Ok(env_vars)
    }
}

/// Build an HTTP client with the RunPod API key from the environment.
fn build_http_client() -> Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    if let Some(api_key) = RunpodProvider::get_api_key() {
        let auth_value = format!("Bearer {}", api_key);
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&auth_value)
                .context("Invalid RUNPOD_API_KEY value for HTTP header")?,
        );
    }

    reqwest::Client::builder()
        .default_headers(headers)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("Failed to build HTTP client for RunPod API")
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

/// Request body for POST /v1/pods (RunPod REST API)
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePodRequest {
    pub name: String,
    pub image_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_type_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compute_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloud_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_disk_in_gb: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_in_gb: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_mount_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ports: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_center_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interruptible: Option<bool>,
}

/// RunPod pod response from the REST API
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunpodPod {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub desired_status: Option<String>,
    #[serde(default, alias = "image")]
    pub image_name: Option<String>,
    #[serde(default)]
    pub gpu: Option<RunpodGpuInfo>,
    #[serde(default)]
    pub public_ip: Option<String>,
    #[serde(default)]
    pub machine: Option<RunpodMachineInfo>,
    #[serde(default)]
    pub port_mappings: Option<Vec<RunpodPortMapping>>,
    #[serde(default)]
    pub volume_in_gb: Option<u32>,
    #[serde(default)]
    pub container_disk_in_gb: Option<u32>,
    #[serde(default)]
    pub cost_per_hr: Option<f64>,
    #[serde(default)]
    pub runtime: Option<RunpodRuntime>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunpodGpuInfo {
    #[serde(default, alias = "type")]
    pub gpu_type: Option<String>,
    #[serde(default)]
    pub count: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunpodMachineInfo {
    #[serde(default)]
    pub id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunpodPortMapping {
    #[serde(default)]
    pub private_port: Option<u16>,
    #[serde(default)]
    pub public_port: Option<u16>,
    #[serde(default)]
    pub ip: Option<String>,
    #[serde(default, alias = "type")]
    pub protocol: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunpodRuntime {
    #[serde(default)]
    pub cpu_percent: Option<f64>,
    #[serde(default)]
    pub memory_bytes: Option<u64>,
    #[serde(default)]
    pub memory_limit: Option<u64>,
    #[serde(default)]
    pub disk_bytes: Option<u64>,
    #[serde(default)]
    pub disk_limit: Option<u64>,
}

fn runpod_gpu_from_tier(tier: &sindri_core::types::GpuTier) -> String {
    use sindri_core::types::GpuTier;
    match tier {
        GpuTier::GpuSmall => "NVIDIA RTX A4000".to_string(),
        GpuTier::GpuMedium => "NVIDIA RTX A5000".to_string(),
        GpuTier::GpuLarge => "NVIDIA A100 80GB PCIe".to_string(),
        GpuTier::GpuXlarge => "NVIDIA H100 80GB HBM3".to_string(),
    }
}

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

        // RUNPOD_API_KEY environment variable
        // No CLI tool installation required -- all operations use the REST API
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
                    "Set the RUNPOD_API_KEY environment variable with your RunPod API key.\n\
                     Get your key from https://www.runpod.io/console/user/settings"
                        .to_string(),
                ),
                version: None,
            });
        }

        // Docker (needed for image builds when not using --skip-build)
        if crate::utils::command_exists("docker") {
            let version = crate::utils::get_command_version("docker", "--version")
                .unwrap_or_else(|_| "unknown".to_string());
            available.push(Prerequisite {
                name: "docker".to_string(),
                description: "Docker Engine (for image builds)".to_string(),
                install_hint: None,
                version: Some(version),
            });
        } else {
            // Docker is optional -- only needed when building from source
            available.push(Prerequisite {
                name: "docker".to_string(),
                description: "Docker not found (needed only for --skip-build=false)".to_string(),
                install_hint: Some(
                    "Install Docker: https://docs.docker.com/get-docker/".to_string(),
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

        // Check for existing pod via REST API
        if let Some(existing_id) = self.find_pod_by_name(runpod_config.name).await {
            if opts.force {
                info!("Force flag set, terminating existing pod: {}", existing_id);
                self.terminate_pod(&existing_id).await?;
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            } else {
                return Err(anyhow!(
                    "Pod '{}' already exists (id: {}). Use --force to recreate.",
                    runpod_config.name,
                    existing_id
                ));
            }
        }

        // Resolve image: build and push if necessary, or use pre-built
        info!("Resolving deployment image...");
        let image = crate::utils::resolve_and_build_image(config, opts.skip_build, opts.force)
            .await
            .context("Failed to resolve deployment image for RunPod")?;
        info!("Using image: {}", image);

        // Build REST API create request
        info!("Creating RunPod pod via REST API: {}", runpod_config.name);

        let mut ports = vec!["22/tcp".to_string()];
        for port in &runpod_config.expose_ports {
            ports.push(format!("{}/http", port));
        }

        // Resolve and inject secrets as environment variables
        let secret_env_vars = self.resolve_secrets(config, None).await?;
        let env = if secret_env_vars.is_empty() {
            None
        } else {
            Some(secret_env_vars.clone())
        };

        let request = CreatePodRequest {
            name: runpod_config.name.to_string(),
            image_name: image.clone(),
            gpu_type_ids: if runpod_config.gpu_count > 0 {
                Some(vec![runpod_config.gpu_type.clone()])
            } else {
                None
            },
            gpu_count: if runpod_config.gpu_count > 0 {
                Some(runpod_config.gpu_count)
            } else {
                None
            },
            compute_type: Some(if runpod_config.gpu_count > 0 {
                "GPU".to_string()
            } else {
                "CPU".to_string()
            }),
            cloud_type: Some(runpod_config.cloud_type.clone()),
            container_disk_in_gb: Some(runpod_config.container_disk_gb),
            volume_in_gb: Some(runpod_config.volume_size_gb),
            volume_mount_path: Some("/workspace".to_string()),
            ports: Some(ports),
            data_center_ids: runpod_config.region.as_ref().map(|r| vec![r.clone()]),
            env,
            interruptible: runpod_config.spot_bid.map(|bid| bid > 0.0),
        };

        debug!("RunPod create request: {:?}", request);

        let pod = self.create_pod_api(&request).await?;
        let pod_id = pod.id.clone();

        if opts.wait {
            let timeout = opts.timeout.unwrap_or(300);
            self.wait_for_running(&pod_id, timeout).await?;
        }

        // Save state
        let state = RunpodState {
            pod_id: pod_id.clone(),
            app_name: runpod_config.name.to_string(),
            gpu_type: runpod_config.gpu_type.clone(),
            gpu_count: runpod_config.gpu_count,
            image: Some(image.clone()),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        if let Err(e) = Self::save_state(&state) {
            warn!("Failed to save deployment state: {}. Deployment succeeded but state tracking may be unavailable.", e);
        }

        let connection = ConnectionInfo {
            ssh_command: Some(format!(
                "ssh root@ssh.runpod.io -i ~/.ssh/id_ed25519 (pod: {})",
                pod_id
            )),
            http_url: runpod_config
                .expose_ports
                .first()
                .map(|port| format!("https://{}-{}.proxy.runpod.net", pod_id, port)),
            https_url: None,
            instructions: Some(format!(
                "SSH: ssh root@ssh.runpod.io (pod: {})\nWeb: https://www.runpod.io/console/pods/{}",
                pod_id, pod_id
            )),
        };

        Ok(DeployResult {
            success: true,
            name: runpod_config.name.to_string(),
            provider: "runpod".to_string(),
            instance_id: Some(pod_id),
            connection: Some(connection),
            messages: {
                let mut msgs = vec![format!(
                    "Pod deployed with {} GPU(s)",
                    runpod_config.gpu_count
                )];
                if !secret_env_vars.is_empty() {
                    msgs.push(format!(
                        "Injected {} secret(s) as environment variables",
                        secret_env_vars.len()
                    ));
                }
                msgs
            },
            warnings: vec![],
        })
    }

    async fn connect(&self, config: &SindriConfig) -> Result<()> {
        let file = config.inner();
        let name = &file.name;

        let pod_id = match self.find_pod_by_name(name).await {
            Some(id) => id,
            None => Self::load_state(name)
                .map(|s| s.pod_id)
                .ok_or_else(|| anyhow!("No RunPod pod found for '{}'. Deploy first.", name))?,
        };

        info!("Connecting to RunPod pod: {}", pod_id);

        // Use SSH proxy -- available on all RunPod pods
        let status = tokio::process::Command::new("ssh")
            .args(["root@ssh.runpod.io", "-i", "~/.ssh/id_ed25519"])
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .await
            .context("Failed to connect to RunPod pod via SSH")?;

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
                // Fall back to saved state for basic info
                if let Some(saved) = Self::load_state(name) {
                    warn!(
                        "Pod not found via API, returning saved state for '{}'",
                        name
                    );
                    let mut details = HashMap::new();
                    details.insert("gpu_type".to_string(), saved.gpu_type.clone());
                    details.insert("gpu_count".to_string(), saved.gpu_count.to_string());
                    details.insert("source".to_string(), "saved_state".to_string());
                    return Ok(DeploymentStatus {
                        name: name.to_string(),
                        provider: "runpod".to_string(),
                        state: DeploymentState::Unknown,
                        instance_id: Some(saved.pod_id),
                        image: saved.image,
                        addresses: vec![],
                        resources: None,
                        timestamps: DeploymentTimestamps::default(),
                        details,
                    });
                }
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

        let status_str = pod
            .status
            .as_deref()
            .or(pod.desired_status.as_deref())
            .unwrap_or("UNKNOWN");

        let state = match status_str {
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

        if let Some(ref mappings) = pod.port_mappings {
            for mapping in mappings {
                if let Some(private_port) = mapping.private_port {
                    addresses.push(Address {
                        r#type: AddressType::Https,
                        value: format!("{}-{}.proxy.runpod.net", pod.id, private_port),
                        port: Some(private_port),
                    });
                }
            }
        }

        let mut details = HashMap::new();
        if let Some(ref gpu) = pod.gpu {
            if let Some(ref gpu_type) = gpu.gpu_type {
                details.insert("gpu_type".to_string(), gpu_type.clone());
            }
            if let Some(count) = gpu.count {
                details.insert("gpu_count".to_string(), count.to_string());
            }
        }
        if let Some(ref machine) = pod.machine {
            if let Some(ref machine_id) = machine.id {
                details.insert("machine_id".to_string(), machine_id.clone());
            }
        }

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

        let pod_id = match self.find_pod_by_name(name).await {
            Some(id) => id,
            None => Self::load_state(name)
                .map(|s| s.pod_id)
                .ok_or_else(|| anyhow!("No RunPod pod found for '{}'", name))?,
        };

        info!("Destroying RunPod pod: {} ({})", name, pod_id);
        self.terminate_pod(&pod_id).await?;
        Self::remove_state(name);
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

        let mut actions = Vec::new();

        let file = config.inner();
        let needs_build = file
            .deployment
            .build_from_source
            .as_ref()
            .map(|b| b.enabled)
            .unwrap_or(false)
            || file.deployment.image.is_none();

        if needs_build {
            actions.push(PlannedAction {
                action: ActionType::Create,
                resource: "docker-image".to_string(),
                description: "Build Docker image from Sindri source".to_string(),
            });
            actions.push(PlannedAction {
                action: ActionType::Create,
                resource: "registry-push".to_string(),
                description: "Push Docker image to container registry".to_string(),
            });
        }

        actions.push(PlannedAction {
            action: ActionType::Create,
            resource: "runpod-pod".to_string(),
            description: format!(
                "Create RunPod pod '{}' with {} x {}",
                runpod_config.name, runpod_config.gpu_count, runpod_config.gpu_type
            ),
        });

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
        let name = &file.name;

        let pod_id = match self.find_pod_by_name(name).await {
            Some(id) => id,
            None => Self::load_state(name)
                .map(|s| s.pod_id)
                .ok_or_else(|| anyhow!("No RunPod pod found for '{}'", name))?,
        };

        info!("Starting RunPod pod: {}", pod_id);
        self.start_pod_api(&pod_id).await
    }

    async fn stop(&self, config: &SindriConfig) -> Result<()> {
        let file = config.inner();
        let name = &file.name;

        let pod_id = match self.find_pod_by_name(name).await {
            Some(id) => id,
            None => Self::load_state(name)
                .map(|s| s.pod_id)
                .ok_or_else(|| anyhow!("No RunPod pod found for '{}'", name))?,
        };

        info!("Stopping RunPod pod: {}", pod_id);
        self.stop_pod_api(&pod_id).await
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
    fn test_check_prerequisites_requires_only_api_key() {
        let provider = RunpodProvider::new().unwrap();
        let status = provider.check_prerequisites().unwrap();
        // Should NOT require any CLI tool -- only RUNPOD_API_KEY
        for missing in &status.missing {
            assert_ne!(
                missing.name, "runpodctl",
                "Should not require runpodctl CLI"
            );
        }
    }

    #[test]
    fn test_create_pod_request_serialization() {
        let request = CreatePodRequest {
            name: "test-pod".to_string(),
            image_name: "ghcr.io/org/sindri:latest".to_string(),
            gpu_type_ids: Some(vec!["NVIDIA RTX A4000".to_string()]),
            gpu_count: Some(1),
            compute_type: Some("GPU".to_string()),
            cloud_type: Some("COMMUNITY".to_string()),
            container_disk_in_gb: Some(20),
            volume_in_gb: Some(50),
            volume_mount_path: Some("/workspace".to_string()),
            ports: Some(vec!["22/tcp".to_string(), "8080/http".to_string()]),
            data_center_ids: None,
            env: None,
            interruptible: None,
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["name"], "test-pod");
        assert_eq!(json["imageName"], "ghcr.io/org/sindri:latest");
        assert_eq!(json["gpuTypeIds"][0], "NVIDIA RTX A4000");
        assert_eq!(json["gpuCount"], 1);
        assert_eq!(json["computeType"], "GPU");
        assert_eq!(json["cloudType"], "COMMUNITY");
        assert_eq!(json["containerDiskInGb"], 20);
        assert_eq!(json["volumeInGb"], 50);
        assert_eq!(json["volumeMountPath"], "/workspace");
    }

    #[test]
    fn test_create_pod_request_cpu_only_omits_gpu() {
        let request = CreatePodRequest {
            name: "cpu-pod".to_string(),
            image_name: "sindri:latest".to_string(),
            gpu_type_ids: None,
            gpu_count: None,
            compute_type: Some("CPU".to_string()),
            cloud_type: Some("COMMUNITY".to_string()),
            container_disk_in_gb: Some(20),
            volume_in_gb: None,
            volume_mount_path: None,
            ports: None,
            data_center_ids: None,
            env: None,
            interruptible: None,
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["computeType"], "CPU");
        assert!(json.get("gpuTypeIds").is_none());
        assert!(json.get("gpuCount").is_none());
    }

    #[test]
    fn test_create_pod_request_with_env_vars() {
        let mut env = HashMap::new();
        env.insert("DB_PASSWORD".to_string(), "secret123".to_string());
        env.insert("API_KEY".to_string(), "key456".to_string());

        let request = CreatePodRequest {
            name: "env-pod".to_string(),
            image_name: "sindri:latest".to_string(),
            gpu_type_ids: None,
            gpu_count: None,
            compute_type: None,
            cloud_type: None,
            container_disk_in_gb: None,
            volume_in_gb: None,
            volume_mount_path: None,
            ports: None,
            data_center_ids: None,
            env: Some(env),
            interruptible: None,
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["env"]["DB_PASSWORD"], "secret123");
        assert_eq!(json["env"]["API_KEY"], "key456");
    }

    #[test]
    fn test_pod_response_deserialization() {
        let json = r#"{
            "id": "abc123",
            "name": "my-pod",
            "status": "RUNNING",
            "desiredStatus": "RUNNING",
            "image": "ghcr.io/org/sindri:latest",
            "gpu": { "type": "NVIDIA RTX A4000", "count": 1 },
            "publicIp": "1.2.3.4",
            "machine": { "id": "m-xyz" },
            "portMappings": [{ "privatePort": 8080, "publicPort": 8080, "type": "http" }],
            "volumeInGb": 50,
            "containerDiskInGb": 20,
            "costPerHr": 0.20,
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
        assert_eq!(pod.status.as_deref(), Some("RUNNING"));
        assert!(pod.gpu.is_some());
        assert!(pod.runtime.is_some());
    }

    #[test]
    fn test_pod_minimal_response_deserialization() {
        let json = r#"{ "id": "min-001", "name": "minimal-pod" }"#;

        let pod: RunpodPod = serde_json::from_str(json).unwrap();
        assert_eq!(pod.id, "min-001");
        assert!(pod.status.is_none());
        assert!(pod.gpu.is_none());
    }

    #[test]
    fn test_pod_list_deserialization() {
        let json = r#"[
            { "id": "p1", "name": "pod-one", "status": "RUNNING" },
            { "id": "p2", "name": "pod-two", "status": "EXITED" }
        ]"#;

        let pods: Vec<RunpodPod> = serde_json::from_str(json).unwrap();
        assert_eq!(pods.len(), 2);
        assert_eq!(pods[0].status.as_deref(), Some("RUNNING"));
        assert_eq!(pods[1].status.as_deref(), Some("EXITED"));
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

    #[test]
    fn test_state_serialization_roundtrip() {
        let state = RunpodState {
            pod_id: "pod-test-123".to_string(),
            app_name: "my-app".to_string(),
            gpu_type: "NVIDIA RTX A4000".to_string(),
            gpu_count: 1,
            image: Some("sindri:latest".to_string()),
            created_at: "2026-02-16T10:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: RunpodState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.pod_id, "pod-test-123");
        assert_eq!(deserialized.app_name, "my-app");
    }

    #[test]
    fn test_state_file_path() {
        let path = RunpodProvider::state_file_path("myapp");
        let path_str = path.to_string_lossy();
        assert!(path_str.contains(".sindri"));
        assert!(path_str.contains("state"));
        assert!(path_str.ends_with("runpod-myapp.json"));
    }

    #[test]
    fn test_load_state_missing_file() {
        // Loading state for a non-existent app should return None
        let result = RunpodProvider::load_state("nonexistent-app-xyz-12345");
        assert!(result.is_none());
    }
}
