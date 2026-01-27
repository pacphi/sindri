//! Docker provider implementation

use crate::templates::{TemplateContext, TemplateRegistry};
use crate::traits::Provider;
use crate::utils::{command_exists, get_command_version};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use sindri_core::config::SindriConfig;
use sindri_core::types::{
    ActionType, ConnectionInfo, DeployOptions, DeployResult, DeploymentPlan, DeploymentState,
    DeploymentStatus, PlannedAction, PlannedResource, Prerequisite, PrerequisiteStatus,
    ResourceUsage,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Docker provider for local development
pub struct DockerProvider {
    /// Template registry for generating docker-compose.yml
    templates: TemplateRegistry,
    /// Output directory for generated files
    output_dir: PathBuf,
}

impl DockerProvider {
    /// Create a new Docker provider
    pub fn new() -> Self {
        Self {
            templates: TemplateRegistry::new().expect("Failed to initialize templates"),
            output_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    /// Create with a specific output directory
    pub fn with_output_dir(output_dir: PathBuf) -> Self {
        Self {
            templates: TemplateRegistry::new().expect("Failed to initialize templates"),
            output_dir,
        }
    }

    /// Check if Docker Compose v2 is available
    fn has_compose_v2(&self) -> bool {
        command_exists("docker") && {
            let output = std::process::Command::new("docker")
                .args(["compose", "version"])
                .output();
            output.map(|o| o.status.success()).unwrap_or(false)
        }
    }

    /// Check if sysbox runtime is available
    fn has_sysbox(&self) -> bool {
        let output = std::process::Command::new("docker")
            .args(["info", "--format", "{{.Runtimes}}"])
            .output();

        output
            .map(|o| {
                let stdout = String::from_utf8_lossy(&o.stdout);
                stdout.contains("sysbox-runc")
            })
            .unwrap_or(false)
    }

    /// Check if NVIDIA runtime is available
    fn has_nvidia_runtime(&self) -> bool {
        let output = std::process::Command::new("docker")
            .args(["info", "--format", "{{.Runtimes}}"])
            .output();

        output
            .map(|o| {
                let stdout = String::from_utf8_lossy(&o.stdout);
                stdout.contains("nvidia")
            })
            .unwrap_or(false)
    }

    /// Detect the best DinD mode based on configuration and host capabilities
    fn detect_dind_mode(&self, config: &SindriConfig) -> String {
        let file = config.inner();

        // Check if DinD is enabled
        let dind_enabled = file
            .providers
            .docker
            .as_ref()
            .and_then(|d| d.dind.as_ref())
            .map(|d| d.enabled)
            .unwrap_or(false);

        if !dind_enabled {
            return "none".to_string();
        }

        // Get requested mode
        let requested_mode = file
            .providers
            .docker
            .as_ref()
            .and_then(|d| d.dind.as_ref())
            .map(|d| format!("{:?}", d.mode).to_lowercase())
            .unwrap_or_else(|| "auto".to_string());

        // Check host capabilities
        let has_sysbox = self.has_sysbox();
        let privileged_allowed = file
            .providers
            .docker
            .as_ref()
            .map(|d| d.privileged)
            .unwrap_or(false);

        match requested_mode.as_str() {
            "sysbox" => {
                if has_sysbox {
                    "sysbox".to_string()
                } else {
                    warn!("Sysbox requested but not available on host");
                    "none".to_string()
                }
            }
            "privileged" => "privileged".to_string(),
            "socket" => "socket".to_string(),
            "auto" => {
                if has_sysbox {
                    info!("Auto-detected Sysbox runtime - using secure DinD");
                    "sysbox".to_string()
                } else if privileged_allowed {
                    info!("Sysbox not available - using privileged mode");
                    "privileged".to_string()
                } else {
                    warn!("DinD enabled but no secure runtime available");
                    "none".to_string()
                }
            }
            _ => "none".to_string(),
        }
    }

    /// Generate docker-compose.yml from config
    fn generate_compose(&self, config: &SindriConfig, output_dir: &Path) -> Result<PathBuf> {
        let dind_mode = self.detect_dind_mode(config);
        let context = TemplateContext::from_config(config, &dind_mode);

        let compose_content = self.templates.render("docker-compose.yml", &context)?;
        let compose_path = output_dir.join("docker-compose.yml");

        std::fs::create_dir_all(output_dir)?;
        std::fs::write(&compose_path, compose_content)?;

        info!("Generated docker-compose.yml at {}", compose_path.display());
        Ok(compose_path)
    }

    /// Run docker compose command
    async fn docker_compose(
        &self,
        args: &[&str],
        compose_file: &Path,
    ) -> Result<std::process::Output> {
        let mut cmd = Command::new("docker");
        cmd.arg("compose")
            .arg("-f")
            .arg(compose_file)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        debug!(
            "Running: docker compose -f {} {}",
            compose_file.display(),
            args.join(" ")
        );
        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Docker compose command failed: {}", stderr);
        }

        Ok(output)
    }

    /// Check if a container is running
    async fn is_container_running(&self, name: &str) -> bool {
        let output = Command::new("docker")
            .args(["ps", "--format", "{{.Names}}"])
            .output()
            .await;

        output
            .map(|o| {
                let stdout = String::from_utf8_lossy(&o.stdout);
                stdout.lines().any(|line| line.trim() == name)
            })
            .unwrap_or(false)
    }

    /// Check if a container exists (running or stopped)
    async fn container_exists(&self, name: &str) -> bool {
        let output = Command::new("docker")
            .args(["ps", "-a", "--format", "{{.Names}}"])
            .output()
            .await;

        output
            .map(|o| {
                let stdout = String::from_utf8_lossy(&o.stdout);
                stdout.lines().any(|line| line.trim() == name)
            })
            .unwrap_or(false)
    }

    /// Get container state
    async fn get_container_state(&self, name: &str) -> DeploymentState {
        let output = Command::new("docker")
            .args(["inspect", "-f", "{{.State.Status}}", name])
            .output()
            .await;

        match output {
            Ok(o) if o.status.success() => {
                let state = String::from_utf8_lossy(&o.stdout).trim().to_lowercase();
                match state.as_str() {
                    "running" => DeploymentState::Running,
                    "exited" | "stopped" => DeploymentState::Stopped,
                    "paused" => DeploymentState::Paused,
                    "created" => DeploymentState::Creating,
                    "dead" | "removing" => DeploymentState::Destroying,
                    _ => DeploymentState::Unknown,
                }
            }
            _ => DeploymentState::NotDeployed,
        }
    }

    /// Get container resource usage
    async fn get_resource_usage(&self, name: &str) -> Option<ResourceUsage> {
        let output = Command::new("docker")
            .args([
                "stats",
                "--no-stream",
                "--format",
                "{{.CPUPerc}},{{.MemUsage}}",
                name,
            ])
            .output()
            .await
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = stdout.trim().split(',').collect();

        if parts.len() >= 2 {
            let cpu = parts[0].trim_end_matches('%').parse::<f64>().ok();
            // Memory is in format "123.4MiB / 4GiB"
            let mem_parts: Vec<&str> = parts[1].split('/').collect();
            let mem_bytes = if let Some(used) = mem_parts.first() {
                parse_docker_memory(used.trim())
            } else {
                None
            };
            let mem_limit = if mem_parts.len() > 1 {
                parse_docker_memory(mem_parts[1].trim())
            } else {
                None
            };

            Some(ResourceUsage {
                cpu_percent: cpu,
                memory_bytes: mem_bytes,
                memory_limit: mem_limit,
                disk_bytes: None,
                disk_limit: None,
            })
        } else {
            None
        }
    }

    /// Build Docker image from Dockerfile
    ///
    /// This method is used when no pre-built image is specified in the config.
    /// It builds a local image using the specified Dockerfile and context directory.
    ///
    /// Note: For on-demand builds, the binary should be pre-built with cargo and
    /// placed in v3/bin/ before calling this method. The Dockerfile's builder-local
    /// stage will pick it up from there.
    async fn build_image(
        &self,
        tag: &str,
        dockerfile: &Path,
        context_dir: &Path,
        force: bool,
    ) -> Result<()> {
        let mut args = vec!["build", "-t", tag, "-f"];
        let dockerfile_str = dockerfile.to_string_lossy();
        args.push(&dockerfile_str);

        if force {
            args.push("--no-cache");
        }

        let context_str = context_dir.to_string_lossy();
        args.push(&context_str);

        info!("Building Docker image: {}", tag);
        let output = Command::new("docker")
            .args(&args)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await?;

        if !output.success() {
            return Err(anyhow!("Docker build failed"));
        }

        Ok(())
    }

    /// Clean up volumes for a deployment
    async fn cleanup_volumes(&self, name: &str, project_name: &str) -> Result<()> {
        // Find volumes matching our patterns
        let output = Command::new("docker")
            .args(["volume", "ls", "--format", "{{.Name}}"])
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let patterns = [
            format!("{}_home", name),
            format!("{}_{}_home", project_name, name),
            format!("{}_docker", name),
            format!("{}_{}_docker", project_name, name),
        ];

        for vol in stdout.lines() {
            let vol = vol.trim();
            if patterns.iter().any(|p| vol.ends_with(p) || vol == p) {
                info!("Removing volume: {}", vol);
                let _ = Command::new("docker")
                    .args(["volume", "rm", "-f", vol])
                    .output()
                    .await;
            }
        }

        Ok(())
    }

    /// Clean up networks for a deployment
    async fn cleanup_networks(&self, name: &str, project_name: &str) -> Result<()> {
        // Find networks matching our patterns
        let output = Command::new("docker")
            .args(["network", "ls", "--format", "{{.Name}}"])
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let patterns = [
            format!("{}_default", name),
            format!("{}_default", project_name),
            "sindri_".to_string(),
        ];

        for net in stdout.lines() {
            let net = net.trim();
            if patterns.iter().any(|p| net.contains(p)) {
                info!("Removing network: {}", net);
                let _ = Command::new("docker")
                    .args(["network", "rm", net])
                    .output()
                    .await;
            }
        }

        Ok(())
    }
}

impl Default for DockerProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for DockerProvider {
    fn name(&self) -> &'static str {
        "docker"
    }

    fn check_prerequisites(&self) -> Result<PrerequisiteStatus> {
        let mut missing = Vec::new();
        let mut available = Vec::new();

        // Check Docker
        if command_exists("docker") {
            let version = get_command_version("docker", "--version")
                .unwrap_or_else(|_| "unknown".to_string());
            available.push(Prerequisite {
                name: "docker".to_string(),
                description: "Docker Engine".to_string(),
                install_hint: None,
                version: Some(version),
            });
        } else {
            missing.push(Prerequisite {
                name: "docker".to_string(),
                description: "Docker Engine".to_string(),
                install_hint: Some(
                    "Install Docker: https://docs.docker.com/get-docker/".to_string(),
                ),
                version: None,
            });
        }

        // Check Docker Compose v2
        if self.has_compose_v2() {
            available.push(Prerequisite {
                name: "docker-compose-v2".to_string(),
                description: "Docker Compose v2 (docker compose)".to_string(),
                install_hint: None,
                version: None,
            });
        } else {
            missing.push(Prerequisite {
                name: "docker-compose-v2".to_string(),
                description: "Docker Compose v2".to_string(),
                install_hint: Some("Docker Compose v2 is included in Docker Desktop. For Linux: https://docs.docker.com/compose/install/".to_string()),
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
        let file = config.inner();
        let name = config.name().to_string();
        info!("Deploying {} with Docker provider", name);

        // Check prerequisites
        let prereqs = self.check_prerequisites()?;
        if !prereqs.satisfied {
            let missing_names: Vec<_> = prereqs.missing.iter().map(|p| p.name.as_str()).collect();
            return Err(anyhow!(
                "Missing prerequisites: {}",
                missing_names.join(", ")
            ));
        }

        // Resolve image: use specified image OR build from Sindri repository
        let has_image_specified =
            file.deployment.image.is_some() || file.deployment.image_config.is_some();

        let image = if has_image_specified {
            // Use resolve_image() for full image_config support
            config.resolve_image().await?
        } else {
            // Fetch official Sindri v3 build context from GitHub and build
            let cache_dir = dirs::cache_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("sindri")
                .join("repos");

            let v3_dir = crate::utils::fetch_sindri_build_context(&cache_dir, None)
                .await
                .map_err(|e| {
                    anyhow!(
                        "Failed to fetch Sindri build context from GitHub: {}. \
                        Ensure git is installed and you have network access. \
                        You can also specify a pre-built image using 'deployment.image' \
                        or 'deployment.image_config' in sindri.yaml",
                        e
                    )
                })?;

            // Get the git SHA for unique tagging
            let repo_dir = v3_dir.parent().unwrap();
            let git_sha = crate::utils::get_git_sha(repo_dir)
                .await
                .unwrap_or_else(|_| "unknown".to_string());

            // Tag format: sindri:{cli_version}-{gitsha}
            // Example: sindri:3.0.0-a1b2c3d
            let cli_version = env!("CARGO_PKG_VERSION");
            let tag = format!("sindri:{}-{}", cli_version, git_sha);

            let dockerfile = v3_dir.join("Dockerfile");

            info!(
                "No image specified, building Sindri v3 image {} from {} (commit: {})",
                tag,
                v3_dir.display(),
                git_sha
            );

            // Build and prepare the binary (compile with cargo, copy to v3/bin/)
            crate::utils::build_and_prepare_binary(&v3_dir).await?;

            // Build Docker image from the repository root as context
            // The Dockerfile will use the binary from v3/bin/ (builder-local stage)
            self.build_image(&tag, &dockerfile, repo_dir, opts.force)
                .await?;
            tag
        };

        debug!("Using image: {}", image);

        // Generate docker-compose.yml
        let compose_path = self.generate_compose(config, &self.output_dir)?;

        if opts.dry_run {
            return Ok(DeployResult {
                success: true,
                name: name.clone(),
                provider: "docker".to_string(),
                instance_id: None,
                connection: None,
                messages: vec![format!(
                    "Would deploy {} using image '{}' and docker-compose.yml at {}",
                    name,
                    image,
                    compose_path.display()
                )],
                warnings: vec![],
            });
        }

        // Check if container already exists
        if self.container_exists(&name).await && !opts.force {
            return Err(anyhow!(
                "Container '{}' already exists. Use --force to recreate.",
                name
            ));
        }

        // Stop existing container if force
        if opts.force && self.container_exists(&name).await {
            info!("Removing existing container...");
            let _ = self.docker_compose(&["down", "-v"], &compose_path).await;
        }

        // Start container
        info!("Starting container...");
        let output = self.docker_compose(&["up", "-d"], &compose_path).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to start container: {}", stderr));
        }

        // Wait for container to be running
        if opts.wait {
            let timeout = opts.timeout.unwrap_or(60);
            let start = std::time::Instant::now();
            while start.elapsed().as_secs() < timeout {
                if self.is_container_running(&name).await {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }

            if !self.is_container_running(&name).await {
                return Err(anyhow!(
                    "Container failed to start within {} seconds",
                    timeout
                ));
            }
        }

        Ok(DeployResult {
            success: true,
            name: name.clone(),
            provider: "docker".to_string(),
            instance_id: Some(name.clone()),
            connection: Some(ConnectionInfo {
                ssh_command: Some(format!(
                    "docker exec -it {} /docker/scripts/entrypoint.sh /bin/bash",
                    name
                )),
                http_url: None,
                https_url: None,
                instructions: Some(format!(
                    "Connect with: sindri connect\nOr: docker exec -it {} /docker/scripts/entrypoint.sh /bin/bash",
                    name
                )),
            }),
            messages: vec![format!(
                "Container '{}' deployed successfully with image '{}'",
                name, image
            )],
            warnings: vec![],
        })
    }

    async fn connect(&self, config: &SindriConfig) -> Result<()> {
        let name = config.name();
        info!("Connecting to {} via Docker", name);

        if !self.is_container_running(name).await {
            return Err(anyhow!(
                "Container '{}' is not running. Deploy first: sindri deploy",
                name
            ));
        }

        // Run docker exec interactively
        let status = Command::new("docker")
            .args([
                "exec",
                "-it",
                name,
                "/docker/scripts/entrypoint.sh",
                "/bin/bash",
            ])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await?;

        if !status.success() {
            return Err(anyhow!("Failed to connect to container"));
        }

        Ok(())
    }

    async fn status(&self, config: &SindriConfig) -> Result<DeploymentStatus> {
        let name = config.name().to_string();
        info!("Getting status for {}", name);

        let state = self.get_container_state(&name).await;
        let resources = if state == DeploymentState::Running {
            self.get_resource_usage(&name).await
        } else {
            None
        };

        // Get container ID if exists
        let instance_id = if self.container_exists(&name).await {
            let output = Command::new("docker")
                .args(["inspect", "-f", "{{.Id}}", &name])
                .output()
                .await
                .ok();

            output.and_then(|o| {
                if o.status.success() {
                    let id = String::from_utf8_lossy(&o.stdout).trim().to_string();
                    if id.len() > 12 {
                        Some(id[..12].to_string())
                    } else {
                        Some(id)
                    }
                } else {
                    None
                }
            })
        } else {
            None
        };

        // Get created/started timestamps
        let timestamps = if self.container_exists(&name).await {
            let output = Command::new("docker")
                .args(["inspect", "-f", "{{.Created}},{{.State.StartedAt}}", &name])
                .output()
                .await
                .ok();

            if let Some(o) = output {
                if o.status.success() {
                    let stdout = String::from_utf8_lossy(&o.stdout);
                    let parts: Vec<&str> = stdout.trim().split(',').collect();

                    let created = parts
                        .first()
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                        .map(|dt| dt.with_timezone(&chrono::Utc));

                    let started = parts
                        .get(1)
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                        .map(|dt| dt.with_timezone(&chrono::Utc));

                    sindri_core::types::DeploymentTimestamps {
                        created_at: created,
                        started_at: started,
                        stopped_at: None,
                        updated_at: None,
                    }
                } else {
                    Default::default()
                }
            } else {
                Default::default()
            }
        } else {
            Default::default()
        };

        // Resolve image for status display using the image_config priority chain
        let image = config.resolve_image().await.ok();

        Ok(DeploymentStatus {
            name,
            provider: "docker".to_string(),
            state,
            instance_id,
            image,
            addresses: vec![],
            resources,
            timestamps,
            details: HashMap::new(),
        })
    }

    async fn destroy(&self, config: &SindriConfig, force: bool) -> Result<()> {
        let name = config.name().to_string();
        info!("Destroying {} (force: {})", name, force);

        let compose_path = self.output_dir.join("docker-compose.yml");
        let project_name = self
            .output_dir
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "sindri".to_string());

        // Run docker compose down first
        if compose_path.exists() {
            info!("Running docker compose down...");
            let _ = self
                .docker_compose(&["down", "--volumes", "--remove-orphans"], &compose_path)
                .await;
        }

        // Manual container cleanup as fallback
        if self.container_exists(&name).await {
            info!("Stopping container...");
            let _ = Command::new("docker").args(["stop", &name]).output().await;
            info!("Removing container...");
            let _ = Command::new("docker").args(["rm", &name]).output().await;
        }

        // Clean up volumes
        self.cleanup_volumes(&name, &project_name).await?;

        // Clean up networks
        self.cleanup_networks(&name, &project_name).await?;

        // Remove generated files
        if compose_path.exists() {
            std::fs::remove_file(&compose_path)?;
        }
        let secrets_path = self.output_dir.join(".env.secrets");
        if secrets_path.exists() {
            std::fs::remove_file(secrets_path)?;
        }

        info!("Container and volumes destroyed");
        Ok(())
    }

    async fn plan(&self, config: &SindriConfig) -> Result<DeploymentPlan> {
        let name = config.name().to_string();
        info!("Planning deployment for {}", name);

        let file = config.inner();
        let dind_mode = self.detect_dind_mode(config);

        // Resolve image: use specified image OR build from Sindri Dockerfile
        let has_image_specified =
            file.deployment.image.is_some() || file.deployment.image_config.is_some();

        let image = if has_image_specified {
            // Use resolve_image() for full image_config support
            config.resolve_image().await.map_err(|e| anyhow!("{}", e))?
        } else {
            // Will build from Sindri repository fetched from GitHub
            // Tag format: sindri:{cli_version}-{gitsha} (e.g., sindri:3.0.0-a1b2c3d)
            // Note: We use a placeholder for planning; actual SHA determined at build time
            let cli_version = env!("CARGO_PKG_VERSION");
            format!("sindri:{}-SOURCE", cli_version)
        };

        let mut actions = vec![PlannedAction {
            action: ActionType::Create,
            resource: "docker-compose.yml".to_string(),
            description: "Generate Docker Compose configuration".to_string(),
        }];

        // Check if image needs to be built (only for local untagged images)
        if image == "sindri:latest" || (image.ends_with(":latest") && !image.contains('/')) {
            actions.push(PlannedAction {
                action: ActionType::Create,
                resource: format!("image:{}", image),
                description: "Build Docker image".to_string(),
            });
        }

        actions.push(PlannedAction {
            action: ActionType::Create,
            resource: format!("volume:{}_home", name),
            description: "Create persistent volume for home directory".to_string(),
        });

        if dind_mode == "privileged" {
            actions.push(PlannedAction {
                action: ActionType::Create,
                resource: format!("volume:{}_docker", name),
                description: "Create volume for Docker daemon storage".to_string(),
            });
        }

        actions.push(PlannedAction {
            action: ActionType::Start,
            resource: format!("container:{}", name),
            description: "Start development container".to_string(),
        });

        let mut resources = vec![
            PlannedResource {
                resource_type: "container".to_string(),
                name: name.clone(),
                config: {
                    let mut m = HashMap::new();
                    m.insert("image".to_string(), serde_json::json!(image));
                    m.insert(
                        "memory".to_string(),
                        serde_json::json!(file
                            .deployment
                            .resources
                            .memory
                            .as_deref()
                            .unwrap_or("4GB")),
                    );
                    m.insert(
                        "cpus".to_string(),
                        serde_json::json!(file.deployment.resources.cpus.unwrap_or(2)),
                    );
                    m
                },
            },
            PlannedResource {
                resource_type: "volume".to_string(),
                name: format!("{}_home", name),
                config: HashMap::new(),
            },
        ];

        if dind_mode == "privileged" {
            resources.push(PlannedResource {
                resource_type: "volume".to_string(),
                name: format!("{}_docker", name),
                config: HashMap::new(),
            });
        }

        Ok(DeploymentPlan {
            provider: "docker".to_string(),
            actions,
            resources,
            estimated_cost: None, // Docker is free
        })
    }

    async fn start(&self, config: &SindriConfig) -> Result<()> {
        let name = config.name();
        info!("Starting {}", name);

        let compose_path = self.output_dir.join("docker-compose.yml");
        if !compose_path.exists() {
            return Err(anyhow!(
                "No docker-compose.yml found. Deploy first: sindri deploy"
            ));
        }

        let output = self.docker_compose(&["start"], &compose_path).await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to start container: {}", stderr));
        }

        Ok(())
    }

    async fn stop(&self, config: &SindriConfig) -> Result<()> {
        let name = config.name();
        info!("Stopping {}", name);

        let compose_path = self.output_dir.join("docker-compose.yml");
        if !compose_path.exists() {
            return Err(anyhow!(
                "No docker-compose.yml found. Deploy first: sindri deploy"
            ));
        }

        let output = self.docker_compose(&["stop"], &compose_path).await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to stop container: {}", stderr));
        }

        Ok(())
    }

    fn supports_gpu(&self) -> bool {
        self.has_nvidia_runtime()
    }
}

/// Parse Docker memory format (e.g., "123.4MiB", "2GiB")
fn parse_docker_memory(s: &str) -> Option<u64> {
    let s = s.trim();

    // Try MiB
    if let Some(num) = s.strip_suffix("MiB") {
        return num
            .trim()
            .parse::<f64>()
            .ok()
            .map(|n| (n * 1024.0 * 1024.0) as u64);
    }

    // Try GiB
    if let Some(num) = s.strip_suffix("GiB") {
        return num
            .trim()
            .parse::<f64>()
            .ok()
            .map(|n| (n * 1024.0 * 1024.0 * 1024.0) as u64);
    }

    // Try KiB
    if let Some(num) = s.strip_suffix("KiB") {
        return num.trim().parse::<f64>().ok().map(|n| (n * 1024.0) as u64);
    }

    // Try MB (base 10)
    if let Some(num) = s.strip_suffix("MB") {
        return num
            .trim()
            .parse::<f64>()
            .ok()
            .map(|n| (n * 1000.0 * 1000.0) as u64);
    }

    // Try GB (base 10)
    if let Some(num) = s.strip_suffix("GB") {
        return num
            .trim()
            .parse::<f64>()
            .ok()
            .map(|n| (n * 1000.0 * 1000.0 * 1000.0) as u64);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_docker_memory() {
        assert_eq!(parse_docker_memory("123.4MiB"), Some(129_394_278));
        assert_eq!(parse_docker_memory("2GiB"), Some(2_147_483_648));
        assert_eq!(parse_docker_memory("512KiB"), Some(524_288));
        assert_eq!(parse_docker_memory("1GB"), Some(1_000_000_000));
    }

    #[test]
    fn test_docker_provider_creation() {
        let provider = DockerProvider::new();
        assert_eq!(provider.name(), "docker");
    }
}
