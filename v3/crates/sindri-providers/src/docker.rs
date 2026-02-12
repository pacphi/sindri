//! Docker provider implementation

use crate::templates::{TemplateContext, TemplateRegistry};
use crate::traits::Provider;
use crate::utils::{command_exists, get_command_version};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use sindri_core::config::SindriConfig;
use sindri_core::types::{
    ActionType, ConnectionInfo, DeployOptions, DeployResult, DeploymentPlan, DeploymentState,
    DeploymentStatus, PlannedAction, PlannedResource, Prerequisite, PrerequisiteStatus,
    ResourceUsage,
};
use sindri_secrets::{ResolutionContext, SecretResolver};
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
    pub fn new() -> Result<Self> {
        Ok(Self {
            templates: TemplateRegistry::new().context("Failed to initialize templates")?,
            output_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        })
    }

    /// Create with a specific output directory
    pub fn with_output_dir(output_dir: PathBuf) -> Result<Self> {
        Ok(Self {
            templates: TemplateRegistry::new().context("Failed to initialize templates")?,
            output_dir,
        })
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
    fn generate_compose(
        &self,
        config: &SindriConfig,
        output_dir: &Path,
        image_override: Option<&str>,
    ) -> Result<PathBuf> {
        let dind_mode = self.detect_dind_mode(config);
        let mut context = TemplateContext::from_config(config, &dind_mode);

        // Apply image override if provided
        if let Some(image) = image_override {
            context.image = image.to_string();
        }

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

    /// Run docker compose command with explicit project name
    async fn docker_compose_with_project(
        &self,
        args: &[&str],
        compose_file: &Path,
        project_name: &str,
    ) -> Result<std::process::Output> {
        let mut cmd = Command::new("docker");
        cmd.arg("compose")
            .arg("-f")
            .arg(compose_file)
            .arg("--project-name")
            .arg(project_name)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        debug!(
            "Running: docker compose -f {} --project-name {} {}",
            compose_file.display(),
            project_name,
            args.join(" ")
        );
        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(
                "Docker compose command failed (project: {}): {}",
                project_name, stderr
            );
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

    /// Resolve secrets and write them to .env.secrets file
    async fn resolve_secrets(
        &self,
        config: &SindriConfig,
        custom_env_file: Option<PathBuf>,
    ) -> Result<Option<PathBuf>> {
        let secrets = config.secrets();

        // If no secrets configured, skip
        if secrets.is_empty() {
            debug!("No secrets configured, skipping secrets resolution");
            return Ok(None);
        }

        info!("Resolving {} secrets...", secrets.len());

        // Create resolution context from config directory
        let config_dir = config
            .config_path
            .parent()
            .map(|p| p.to_path_buf().into())
            .unwrap_or_else(|| PathBuf::from("."));

        let context = ResolutionContext::new(config_dir).with_custom_env_file(custom_env_file);

        // Resolve all secrets
        let resolver = SecretResolver::new(context);
        let resolved = resolver.resolve_all(secrets).await?;

        // Write environment variable secrets to .env.secrets file
        let secrets_file = self.output_dir.join(".env.secrets");
        let mut env_content = String::new();
        env_content.push_str("# Auto-generated secrets file - DO NOT COMMIT\n");
        env_content.push_str("# Generated by Sindri CLI\n\n");

        let mut has_env_secrets = false;
        for (name, secret) in &resolved {
            if let Some(value) = secret.value.as_string() {
                // This is an environment variable secret
                env_content.push_str(&format!("{}={}\n", name, value));
                has_env_secrets = true;
            }
        }

        if has_env_secrets {
            std::fs::write(&secrets_file, env_content)?;
            info!(
                "Wrote {} environment secrets to {}",
                resolved.len(),
                secrets_file.display()
            );

            // Set restrictive permissions (0600 = owner read/write only)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&secrets_file)?.permissions();
                perms.set_mode(0o600);
                std::fs::set_permissions(&secrets_file, perms)?;
            }

            Ok(Some(secrets_file))
        } else {
            debug!("No environment variable secrets to write");
            Ok(None)
        }
    }

    /// Clean up secrets file
    fn cleanup_secrets_file(&self, secrets_file: Option<&PathBuf>) {
        if let Some(path) = secrets_file {
            if path.exists() {
                debug!("Removing secrets file: {}", path.display());
                let _ = std::fs::remove_file(path);
            }
        }
    }
}

impl Default for DockerProvider {
    fn default() -> Self {
        Self::new().expect("Failed to create default DockerProvider")
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

        let should_build_from_source = file
            .deployment
            .build_from_source
            .as_ref()
            .map(|b| b.enabled)
            .unwrap_or(false);

        let image = if has_image_specified && !should_build_from_source {
            // Use resolve_image() for full image_config support
            config.resolve_image(None).await?
        } else if should_build_from_source || !has_image_specified {
            // Determine which git ref to clone for getting the Dockerfile
            // Use the gitRef from config if specified, otherwise use CLI version
            let version_to_fetch = file
                .deployment
                .build_from_source
                .as_ref()
                .and_then(|b| b.git_ref.as_deref());

            // Fetch official Sindri v3 build context from GitHub and build
            let cache_dir = dirs::cache_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("sindri")
                .join("repos");

            let (v3_dir, git_ref_used) =
                crate::utils::fetch_sindri_build_context(&cache_dir, version_to_fetch)
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

            // Select appropriate Dockerfile based on build mode
            let dockerfile_name = if should_build_from_source {
                "Dockerfile.dev"
            } else {
                "Dockerfile"
            };
            let dockerfile = v3_dir.join(dockerfile_name);

            info!(
                "Building {} image {} from {} using {} (commit: {})",
                if should_build_from_source {
                    "development"
                } else {
                    "production"
                },
                tag,
                v3_dir.display(),
                dockerfile_name,
                git_sha
            );

            // Determine which git ref to use for Docker build
            // Priority: YAML config > git ref we actually cloned from
            let sindri_version = file
                .deployment
                .build_from_source
                .as_ref()
                .and_then(|b| b.git_ref.clone())
                .unwrap_or_else(|| git_ref_used.clone());

            // Build Docker image using selected Dockerfile
            // Dockerfile choice (production vs development) is implicit in the file selected
            let mut args = vec!["build", "-t", &tag, "-f"];
            let dockerfile_str = dockerfile.to_string_lossy();
            args.push(&dockerfile_str);

            if opts.force {
                args.push("--no-cache");
            }

            // Pass SINDRI_VERSION build arg (used by both Dockerfiles)
            args.push("--build-arg");
            let sindri_version_arg = format!("SINDRI_VERSION={}", sindri_version);
            args.push(&sindri_version_arg);

            let context_str = repo_dir.to_string_lossy();
            args.push(&context_str);

            info!(
                "Building Docker image from source (ref: {}) - this will take 3-5 minutes...",
                sindri_version
            );
            let output = Command::new("docker")
                .args(&args)
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .await?;

            if !output.success() {
                return Err(anyhow!("Docker build failed"));
            }

            tag
        } else {
            // Neither image specified nor build_from_source enabled
            return Err(anyhow!(
                "No image configured. Please specify:\n\
                1. deployment.image or deployment.image_config in sindri.yaml, OR\n\
                2. Enable deployment.buildFromSource.enabled in sindri.yaml, OR\n\
                3. Use --from-source flag when deploying"
            ));
        };

        debug!("Using image: {}", image);

        // Resolve secrets and write to .env.secrets
        let secrets_file = match self.resolve_secrets(config, None).await {
            Ok(path) => path,
            Err(e) => {
                warn!("Failed to resolve secrets: {}", e);
                return Err(anyhow!("Secret resolution failed: {}", e));
            }
        };

        // Generate docker-compose.yml with the resolved image
        let compose_path = self.generate_compose(config, &self.output_dir, Some(&image))?;

        if opts.dry_run {
            self.cleanup_secrets_file(secrets_file.as_ref());
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
            self.cleanup_secrets_file(secrets_file.as_ref());
            return Err(anyhow!(
                "Container '{}' already exists. Use --force to recreate.",
                name
            ));
        }

        // Derive project name from output directory for consistent volume naming
        let project_name = self
            .output_dir
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "sindri".to_string());

        // Stop existing container if force
        if opts.force && self.container_exists(&name).await {
            info!(
                "Removing existing container with project name '{}'...",
                project_name
            );
            let _ = self
                .docker_compose_with_project(&["down", "-v"], &compose_path, &project_name)
                .await;
        }

        // Start container
        info!("Starting container with project name '{}'...", project_name);
        let output = self
            .docker_compose_with_project(&["up", "-d"], &compose_path, &project_name)
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            self.cleanup_secrets_file(secrets_file.as_ref());
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
                self.cleanup_secrets_file(secrets_file.as_ref());
                return Err(anyhow!(
                    "Container failed to start within {} seconds",
                    timeout
                ));
            }
        }

        // Container is running, clean up secrets file
        // Note: Keep the file for a brief moment to ensure container has fully read it
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        self.cleanup_secrets_file(secrets_file.as_ref());

        Ok(DeployResult {
            success: true,
            name: name.clone(),
            provider: "docker".to_string(),
            instance_id: Some(name.clone()),
            connection: Some(ConnectionInfo {
                ssh_command: Some(format!(
                    "docker exec -it -e HOME=/alt/home/developer -u developer -w /alt/home/developer {} bash -l",
                    name
                )),
                http_url: None,
                https_url: None,
                instructions: Some(format!(
                    "Connect with: sindri connect\nOr: docker exec -it -e HOME=/alt/home/developer -u developer -w /alt/home/developer {} bash -l",
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
        let _status = Command::new("docker")
            .args([
                "exec",
                "-it",
                "-e",
                "HOME=/alt/home/developer",
                "-u",
                "developer",
                "-w",
                "/alt/home/developer",
                name,
                "bash",
                "-l",
            ])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await?;

        // Note: We don't check exit status here because exiting from an interactive
        // shell is the normal way to disconnect. Any exit (whether code 0 or non-zero)
        // indicates the user intentionally left the session, not a connection failure.
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
        let image = config.resolve_image(None).await.ok();

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
            info!(
                "Running docker compose down with project name '{}'...",
                project_name
            );
            match self
                .docker_compose_with_project(
                    &["down", "--volumes", "--remove-orphans"],
                    &compose_path,
                    &project_name,
                )
                .await
            {
                Ok(output) => {
                    if output.status.success() {
                        info!("Docker compose down completed successfully");
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        warn!("Docker compose down had issues: {}", stderr);
                        warn!("Will attempt manual cleanup as fallback");
                    }
                }
                Err(e) => {
                    warn!("Docker compose down failed: {}", e);
                    warn!("Will attempt manual cleanup as fallback");
                }
            }
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
            config
                .resolve_image(None)
                .await
                .map_err(|e| anyhow!("{}", e))?
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
    fn test_parse_docker_memory_mb_base10() {
        assert_eq!(parse_docker_memory("500MB"), Some(500_000_000));
    }

    #[test]
    fn test_parse_docker_memory_invalid() {
        assert_eq!(parse_docker_memory("invalid"), None);
        assert_eq!(parse_docker_memory(""), None);
        assert_eq!(parse_docker_memory("123TB"), None);
    }

    #[test]
    fn test_parse_docker_memory_whitespace() {
        // Input is trimmed in the function
        assert_eq!(parse_docker_memory("  2GiB  "), Some(2_147_483_648));
        assert_eq!(parse_docker_memory("  512KiB  "), Some(524_288));
    }

    #[test]
    fn test_docker_provider_creation() {
        let provider = DockerProvider::new().unwrap();
        assert_eq!(provider.name(), "docker");
    }

    #[test]
    fn test_docker_provider_with_output_dir() {
        let dir = std::path::PathBuf::from("/tmp/test-docker");
        let provider = DockerProvider::with_output_dir(dir.clone()).unwrap();
        assert_eq!(provider.output_dir, dir);
        assert_eq!(provider.name(), "docker");
    }

    #[test]
    fn test_docker_provider_check_prerequisites() {
        let provider = DockerProvider::new().unwrap();
        let result = provider.check_prerequisites();
        assert!(result.is_ok(), "check_prerequisites should not error");
        let status = result.unwrap();
        // In CI, docker may or may not exist, but the function should not panic
        assert!(
            !status.available.is_empty() || !status.missing.is_empty(),
            "should have at least one available or missing prerequisite"
        );
    }

    #[test]
    fn test_docker_provider_does_not_support_auto_suspend() {
        let provider = DockerProvider::new().unwrap();
        assert!(
            !provider.supports_auto_suspend(),
            "Docker provider should not support auto-suspend"
        );
    }
}
