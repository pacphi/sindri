//! Northflank provider implementation
//!
//! Deploys Sindri development environments on Northflank's Kubernetes-based PaaS.
//! Uses `northflank` CLI for project and service lifecycle management.

use crate::traits::Provider;
use crate::utils::{command_exists, get_command_version};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use sindri_core::config::SindriConfig;
use sindri_core::types::{
    ActionType, Address, AddressType, ConnectionInfo, DeployOptions, DeployResult, DeploymentPlan,
    DeploymentState, DeploymentStatus, DeploymentTimestamps, PlannedAction, PlannedResource,
    Prerequisite, PrerequisiteStatus, ResourceUsage,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Northflank provider for Kubernetes PaaS deployment
pub struct NorthflankProvider {
    /// Output directory for generated files
    #[allow(dead_code)]
    output_dir: PathBuf,
}

impl NorthflankProvider {
    /// Create a new Northflank provider
    pub fn new() -> Result<Self> {
        Ok(Self {
            output_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        })
    }

    /// Create with a specific output directory
    pub fn with_output_dir(output_dir: PathBuf) -> Result<Self> {
        Ok(Self { output_dir })
    }

    /// Check if Northflank is authenticated
    fn is_authenticated(&self) -> bool {
        // Check env var first
        if std::env::var("NORTHFLANK_API_TOKEN").is_ok() {
            return true;
        }
        // Try listing projects to verify auth
        let output = std::process::Command::new("northflank")
            .args(["list", "projects", "--output", "json"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        output.map(|s| s.success()).unwrap_or(false)
    }

    /// Ensure a project exists (create if not)
    async fn ensure_project(&self, project_name: &str) -> Result<()> {
        // Check if project exists
        let output = Command::new("northflank")
            .args([
                "get",
                "project",
                "--project",
                project_name,
                "--output",
                "json",
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
                "create",
                "project",
                "--input",
                &project_def.to_string(),
                "--output",
                "json",
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
                "list",
                "services",
                "--project",
                project_name,
                "--output",
                "json",
            ])
            .output()
            .await
            .context("Failed to list Northflank services")?;

        if !output.status.success() {
            return Ok(None);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let services: Vec<NorthflankService> = serde_json::from_str(&stdout).unwrap_or_default();

        Ok(services.into_iter().find(|s| s.name == service_name))
    }

    /// Delete a service
    async fn delete_service(&self, project_name: &str, service_id: &str) -> Result<()> {
        let output = Command::new("northflank")
            .args([
                "delete",
                "service",
                "--project",
                project_name,
                "--service",
                service_id,
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

    /// Pause a service
    async fn pause_service(&self, project_name: &str, service_id: &str) -> Result<()> {
        let output = Command::new("northflank")
            .args(["pause", "--project", project_name, "--service", service_id])
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
            .args(["resume", "--project", project_name, "--service", service_id])
            .output()
            .await
            .context("Failed to resume service")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to resume service: {}", stderr));
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
            "size": size_gb * 1024,
            "mountPath": mount_path
        });

        let output = Command::new("northflank")
            .args([
                "create",
                "volume",
                "--project",
                project_name,
                "--service",
                service_id,
                "--input",
                &volume_def.to_string(),
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
                    service_id,
                    timeout_secs
                ));
            }

            let output = Command::new("northflank")
                .args([
                    "get",
                    "service",
                    "--project",
                    project_name,
                    "--service",
                    service_id,
                    "--output",
                    "json",
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

    /// Resolve project name and service ID from config
    async fn resolve_service_ids(&self, config: &SindriConfig) -> Result<(String, String)> {
        let file = config.inner();
        let nf = file.providers.northflank.as_ref();

        let project_name = nf
            .and_then(|n| n.project_name.clone())
            .unwrap_or_else(|| format!("sindri-{}", file.name));
        let service_name = nf
            .and_then(|n| n.service_name.clone())
            .unwrap_or_else(|| file.name.clone());

        let service = self
            .find_service(&project_name, &service_name)
            .await?
            .ok_or_else(|| anyhow!("No Northflank service found for '{}'", file.name))?;

        Ok((project_name, service.id))
    }

    /// Get Northflank config from SindriConfig
    fn get_northflank_config<'a>(&self, config: &'a SindriConfig) -> InternalDeployConfig<'a> {
        let file = config.inner();
        let nf = file.providers.northflank.as_ref();

        let memory_raw = file.deployment.resources.memory.as_deref().unwrap_or("2GB");
        let memory_mb = parse_memory_to_mb(memory_raw).unwrap_or(2048);
        let cpus = file.deployment.resources.cpus.unwrap_or(2);

        let volume_size_gb = file
            .deployment
            .volumes
            .workspace
            .as_ref()
            .map(|v| parse_size_to_gb(&v.size).unwrap_or(10))
            .unwrap_or(10);

        let volume_mount_path = file
            .deployment
            .volumes
            .workspace
            .as_ref()
            .map(|v| v.path.clone())
            .unwrap_or_else(|| "/workspace".to_string());

        let compute_plan = nf
            .and_then(|n| n.compute_plan.clone())
            .unwrap_or_else(|| compute_plan_from_resources(cpus, memory_mb));

        InternalDeployConfig {
            name: &file.name,
            project_name: nf
                .and_then(|n| n.project_name.clone())
                .unwrap_or_else(|| format!("sindri-{}", file.name)),
            service_name: nf
                .and_then(|n| n.service_name.clone())
                .unwrap_or_else(|| file.name.clone()),
            compute_plan,
            instances: nf.map(|n| n.instances).unwrap_or(1),
            volume_size_gb,
            volume_mount_path,
            cpus,
            memory_mb,
        }
    }

    /// Build JSON service definition for northflank create
    pub fn build_service_definition<'a>(
        &self,
        config: &NorthflankDeployConfig<'a>,
    ) -> Result<String> {
        let image = &config.image;

        let mut service_def = serde_json::json!({
            "name": config.service_name,
            "description": format!("Sindri development environment: {}", config.name),
            "billing": {
                "deploymentPlan": config.compute_plan
            },
            "deployment": {
                "instances": config.instances,
                "external": {
                    "imagePath": image
                },
                "docker": {
                    "configType": "default"
                }
            }
        });

        // Add ports
        let ports_json: Vec<serde_json::Value> = if config.ports.is_empty() {
            vec![serde_json::json!({
                "name": "ssh",
                "internalPort": 22,
                "public": false,
                "protocol": "TCP"
            })]
        } else {
            config
                .ports
                .iter()
                .map(|p| {
                    serde_json::json!({
                        "name": p.name,
                        "internalPort": p.internal_port,
                        "public": p.public,
                        "protocol": p.protocol
                    })
                })
                .collect()
        };
        service_def["ports"] = serde_json::Value::Array(ports_json);

        // Add health checks if configured
        if let Some(ref hc) = config.health_check {
            service_def["healthChecks"] = serde_json::json!([{
                "path": hc.path,
                "port": hc.port,
                "intervalSeconds": hc.interval_secs,
                "timeoutSeconds": hc.timeout_secs
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

    /// Create a secret group for a service
    pub async fn create_secret_group(
        &self,
        project: &str,
        service: &str,
        secrets: &HashMap<String, String>,
    ) -> Result<()> {
        let secret_def = serde_json::json!({
            "name": format!("{}-secrets", service),
            "secrets": secrets,
        });

        let output = Command::new("northflank")
            .args([
                "create",
                "secret",
                "--project",
                project,
                "--input",
                &secret_def.to_string(),
            ])
            .output()
            .await
            .context("Failed to create secret group")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to create secret group: {}", stderr));
        }

        Ok(())
    }
}

/// Northflank deployment configuration
pub struct NorthflankDeployConfig<'a> {
    pub name: &'a str,
    pub project_name: String,
    pub service_name: String,
    pub compute_plan: String,
    pub instances: u32,
    pub gpu_type: Option<String>,
    pub gpu_count: u32,
    pub volume_size_gb: u32,
    pub volume_mount_path: String,
    pub region: Option<String>,
    pub ports: Vec<NorthflankPort>,
    pub health_check: Option<NorthflankHealthCheck>,
    pub auto_scaling: Option<NorthflankAutoScaling>,
    pub cpus: u32,
    pub memory_mb: u32,
    pub image: String,
}

/// Internal deploy config extracted from SindriConfig (holds a borrow)
struct InternalDeployConfig<'a> {
    name: &'a str,
    project_name: String,
    service_name: String,
    compute_plan: String,
    instances: u32,
    volume_size_gb: u32,
    volume_mount_path: String,
    cpus: u32,
    memory_mb: u32,
}

/// Northflank port configuration
#[derive(Debug, Clone)]
pub struct NorthflankPort {
    pub name: String,
    pub internal_port: u16,
    pub public: bool,
    pub protocol: String,
}

/// Northflank health check configuration
#[derive(Debug, Clone)]
pub struct NorthflankHealthCheck {
    pub path: String,
    pub port: u16,
    pub interval_secs: u32,
    pub timeout_secs: u32,
}

/// Northflank auto-scaling configuration
#[derive(Debug, Clone)]
pub struct NorthflankAutoScaling {
    pub min_instances: u32,
    pub max_instances: u32,
    pub cpu_target_percent: u32,
}

/// Northflank service response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NorthflankService {
    pub id: String,
    pub name: String,
    pub status: String,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default = "default_compute_plan")]
    pub compute_plan: String,
    #[serde(default = "default_one")]
    pub instances: u32,
    #[serde(default)]
    pub ports: Vec<NorthflankServicePort>,
    #[serde(default)]
    pub metrics: Option<NorthflankMetrics>,
}

fn default_compute_plan() -> String {
    "unknown".to_string()
}

fn default_one() -> u32 {
    1
}

/// Northflank service port in API response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NorthflankServicePort {
    pub name: String,
    pub internal_port: u16,
    #[serde(default)]
    pub public: bool,
    #[serde(default)]
    pub dns: Option<String>,
}

/// Northflank metrics in API response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NorthflankMetrics {
    pub cpu_percent: Option<f64>,
    pub memory_bytes: Option<u64>,
    pub memory_limit: Option<u64>,
    pub disk_bytes: Option<u64>,
    pub disk_limit: Option<u64>,
}

/// Northflank project in API response
#[derive(Debug, Deserialize)]
pub struct NorthflankProject {
    pub id: String,
    pub name: String,
}

/// Map a Northflank service status string to DeploymentState
pub fn map_service_status(status: &str) -> DeploymentState {
    match status {
        "running" => DeploymentState::Running,
        "paused" => DeploymentState::Paused,
        "creating" | "pending" => DeploymentState::Creating,
        "error" | "failed" => DeploymentState::Error,
        "stopped" => DeploymentState::Stopped,
        _ => DeploymentState::Unknown,
    }
}

/// Map a GPU tier name to Northflank GPU type identifier
pub fn northflank_gpu_from_tier(tier: Option<&str>) -> &'static str {
    match tier {
        Some("gpu-large") | Some("gpu-xlarge") => "nvidia-a100",
        _ => "nvidia-a10g",
    }
}

/// Map CPU/memory to Northflank compute plan
pub fn compute_plan_from_resources(cpus: u32, memory_mb: u32) -> String {
    match (cpus, memory_mb) {
        (c, m) if c <= 1 && m <= 512 => "nf-compute-10".to_string(),
        (c, m) if c <= 2 && m <= 2048 => "nf-compute-20".to_string(),
        (c, m) if c <= 4 && m <= 4096 => "nf-compute-50".to_string(),
        (c, m) if c <= 8 && m <= 8192 => "nf-compute-100".to_string(),
        _ => "nf-compute-200".to_string(),
    }
}

/// Parse memory string to megabytes
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

#[async_trait]
impl Provider for NorthflankProvider {
    fn name(&self) -> &'static str {
        "northflank"
    }

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
                install_hint: Some("Install: npm install -g @northflank/cli".to_string()),
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
                     Or set NORTHFLANK_API_TOKEN environment variable"
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
        let nf_config = self.get_northflank_config(config);

        // Handle dry run
        if opts.dry_run {
            return Ok(DeployResult {
                success: true,
                name: nf_config.name.to_string(),
                provider: "northflank".to_string(),
                instance_id: None,
                connection: None,
                messages: vec![format!(
                    "Dry run: would create Northflank project '{}' and service '{}' on plan {}",
                    nf_config.project_name, nf_config.service_name, nf_config.compute_plan
                )],
                warnings: vec![],
            });
        }

        // Check for existing service
        if let Some(existing) = self
            .find_service(&nf_config.project_name, &nf_config.service_name)
            .await?
        {
            if opts.force {
                info!("Force flag set, deleting existing service: {}", existing.id);
                self.delete_service(&nf_config.project_name, &existing.id)
                    .await?;
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            } else {
                return Err(anyhow!(
                    "Service '{}' already exists in project '{}'. Use --force to recreate.",
                    nf_config.service_name,
                    nf_config.project_name
                ));
            }
        }

        // Ensure project exists
        info!(
            "Ensuring Northflank project '{}' exists",
            nf_config.project_name
        );
        self.ensure_project(&nf_config.project_name).await?;

        // Resolve image
        let image = config
            .inner()
            .deployment
            .image
            .clone()
            .ok_or_else(|| anyhow!("No image configured for Northflank deployment"))?;

        // Create deployment service
        info!("Creating Northflank service: {}", nf_config.service_name);
        let deploy_config = NorthflankDeployConfig {
            name: nf_config.name,
            project_name: nf_config.project_name.clone(),
            service_name: nf_config.service_name.clone(),
            compute_plan: nf_config.compute_plan.clone(),
            instances: nf_config.instances,
            gpu_type: None,
            gpu_count: 0,
            volume_size_gb: nf_config.volume_size_gb,
            volume_mount_path: nf_config.volume_mount_path.clone(),
            region: None,
            ports: vec![NorthflankPort {
                name: "ssh".to_string(),
                internal_port: 22,
                public: false,
                protocol: "TCP".to_string(),
            }],
            health_check: None,
            auto_scaling: None,
            cpus: nf_config.cpus,
            memory_mb: nf_config.memory_mb,
            image: image.clone(),
        };
        let service_def = self.build_service_definition(&deploy_config)?;

        let output = Command::new("northflank")
            .args([
                "create",
                "service",
                "deployment",
                "--project",
                &nf_config.project_name,
                "--input",
                &service_def,
                "--output",
                "json",
            ])
            .output()
            .await
            .context("Failed to create Northflank service")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to create Northflank service: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let service: NorthflankService =
            serde_json::from_str(&stdout).context("Failed to parse service creation response")?;

        // Create and attach volume
        if nf_config.volume_size_gb > 0 {
            self.create_and_attach_volume(
                &nf_config.project_name,
                &service.id,
                nf_config.volume_size_gb,
                &nf_config.volume_mount_path,
            )
            .await?;
        }

        // Wait for running
        if opts.wait {
            let timeout = opts.timeout.unwrap_or(300);
            self.wait_for_running(&nf_config.project_name, &service.id, timeout)
                .await?;
        }

        // Build connection info
        let connection = ConnectionInfo {
            ssh_command: Some(format!(
                "northflank exec --project {} --service {}",
                nf_config.project_name, service.id
            )),
            http_url: service
                .ports
                .iter()
                .find(|p| p.public)
                .and_then(|p| p.dns.clone())
                .map(|dns| format!("https://{}", dns)),
            https_url: None,
            instructions: Some(format!(
                "Shell: northflank exec --project {} --service {}\n\
                 Port forward: northflank forward --project {} --service {}",
                nf_config.project_name, service.id, nf_config.project_name, service.id
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

    async fn connect(&self, config: &SindriConfig) -> Result<()> {
        let file = config.inner();
        let name = &file.name;
        let nf = file.providers.northflank.as_ref();

        let project_name = nf
            .and_then(|n| n.project_name.clone())
            .unwrap_or_else(|| format!("sindri-{}", name));
        let service_name = nf
            .and_then(|n| n.service_name.clone())
            .unwrap_or_else(|| name.clone());

        let service = self
            .find_service(&project_name, &service_name)
            .await?
            .ok_or_else(|| anyhow!("No Northflank service found for '{}'. Deploy first.", name))?;

        // Auto-resume if paused
        if service.status == "paused" {
            info!("Service is paused, resuming...");
            self.resume_service(&project_name, &service.id).await?;
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }

        info!("Connecting to Northflank service: {}", service.id);

        let status = Command::new("northflank")
            .args(["exec", "--project", &project_name, "--service", &service.id])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await
            .context("Failed to connect to Northflank service")?;

        if !status.success() {
            return Err(anyhow!("Shell connection to service {} failed", service.id));
        }

        Ok(())
    }

    async fn status(&self, config: &SindriConfig) -> Result<DeploymentStatus> {
        let file = config.inner();
        let name = &file.name;
        let nf = file.providers.northflank.as_ref();

        let project_name = nf
            .and_then(|n| n.project_name.clone())
            .unwrap_or_else(|| format!("sindri-{}", name));
        let service_name = nf
            .and_then(|n| n.service_name.clone())
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

        let state = map_service_status(&service.status);

        let mut addresses = Vec::new();
        for port in &service.ports {
            if let Some(ref dns) = port.dns {
                addresses.push(Address {
                    r#type: if port.public {
                        AddressType::Https
                    } else {
                        AddressType::Internal
                    },
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

    async fn destroy(&self, config: &SindriConfig, _force: bool) -> Result<()> {
        let file = config.inner();
        let name = &file.name;
        let nf = file.providers.northflank.as_ref();

        let project_name = nf
            .and_then(|n| n.project_name.clone())
            .unwrap_or_else(|| format!("sindri-{}", name));
        let service_name = nf
            .and_then(|n| n.service_name.clone())
            .unwrap_or_else(|| name.clone());

        let service = self
            .find_service(&project_name, &service_name)
            .await?
            .ok_or_else(|| anyhow!("No Northflank service found for '{}'", name))?;

        info!(
            "Destroying Northflank service: {} in project {}",
            service.id, project_name
        );
        self.delete_service(&project_name, &service.id).await?;

        info!("Service {} destroyed", service.id);
        info!(
            "Note: Project '{}' was preserved. Delete manually if no longer needed.",
            project_name
        );

        Ok(())
    }

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

        let resources = vec![PlannedResource {
            resource_type: "northflank-service".to_string(),
            name: nf_config.service_name.clone(),
            config: HashMap::from([
                (
                    "project".to_string(),
                    serde_json::Value::String(nf_config.project_name.clone()),
                ),
                (
                    "compute_plan".to_string(),
                    serde_json::Value::String(nf_config.compute_plan.clone()),
                ),
                (
                    "instances".to_string(),
                    serde_json::json!(nf_config.instances),
                ),
                (
                    "volume_gb".to_string(),
                    serde_json::json!(nf_config.volume_size_gb),
                ),
            ]),
        }];

        Ok(DeploymentPlan {
            provider: "northflank".to_string(),
            actions,
            resources,
            estimated_cost: None,
        })
    }

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

    fn supports_gpu(&self) -> bool {
        true
    }

    fn supports_auto_suspend(&self) -> bool {
        true
    }
}

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
        assert_eq!(
            service.ports[1].dns.as_deref(),
            Some("my-service.example.northflank.app")
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
}
