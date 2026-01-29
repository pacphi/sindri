//! Fly.io provider implementation

use crate::templates::{TemplateContext, TemplateRegistry};
use crate::traits::Provider;
use crate::utils::{command_exists, fetch_sindri_build_context, get_command_version};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::Deserialize;
use sindri_core::config::SindriConfig;
use sindri_core::types::{
    ActionType, ConnectionInfo, DeployOptions, DeployResult, DeploymentPlan, DeploymentState,
    DeploymentStatus, PlannedAction, PlannedResource, Prerequisite, PrerequisiteStatus,
};
use sindri_secrets::{ResolutionContext, SecretResolver};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Fly.io provider for cloud deployment
pub struct FlyProvider {
    /// Template registry for generating fly.toml
    templates: TemplateRegistry,
    /// Output directory for generated files
    output_dir: PathBuf,
}

impl FlyProvider {
    /// Create a new Fly.io provider
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

    /// Check if flyctl is authenticated
    fn is_authenticated(&self) -> bool {
        let output = std::process::Command::new("flyctl")
            .args(["auth", "whoami"])
            .output();

        output.map(|o| o.status.success()).unwrap_or(false)
    }

    /// Get Fly.io configuration from SindriConfig
    fn get_fly_config<'a>(&self, config: &'a SindriConfig) -> FlyDeployConfig<'a> {
        let file = config.inner();

        let fly = file.providers.fly.as_ref();

        // Memory in MB (convert from GB/MB string)
        let memory_raw = file.deployment.resources.memory.as_deref().unwrap_or("2GB");
        let memory_mb = parse_memory_to_mb(memory_raw).unwrap_or(2048);

        let cpus = file.deployment.resources.cpus.unwrap_or(2);

        // Swap is half of memory, minimum 2GB
        let swap_mb = std::cmp::max(memory_mb / 2, 2048);

        // GPU configuration
        let (gpu_enabled, gpu_tier) = file
            .deployment
            .resources
            .gpu
            .as_ref()
            .map(|g| (g.enabled, g.tier.as_ref()))
            .unwrap_or((false, None));

        let gpu_tier_str = gpu_tier
            .map(|t| format!("{:?}", t).to_lowercase().replace("gpu", "gpu-"))
            .unwrap_or_else(|| "gpu-small".to_string());

        // Volume size
        let volume_size = file
            .deployment
            .volumes
            .workspace
            .as_ref()
            .map(|v| parse_size_to_gb(&v.size).unwrap_or(10))
            .unwrap_or(10);

        FlyDeployConfig {
            name: &file.name,
            region: fly.map(|f| f.region.as_str()).unwrap_or("sjc"),
            organization: fly
                .and_then(|f| f.organization.as_deref())
                .unwrap_or("personal"),
            cpu_kind: fly
                .map(|f| format!("{:?}", f.cpu_kind).to_lowercase())
                .unwrap_or_else(|| "shared".to_string()),
            cpus,
            memory_mb,
            swap_mb,
            ssh_port: fly.map(|f| f.ssh_port).unwrap_or(10022),
            auto_stop: fly.map(|f| f.auto_stop_machines).unwrap_or(true),
            auto_start: fly.map(|f| f.auto_start_machines).unwrap_or(true),
            volume_size,
            gpu_enabled,
            gpu_tier: gpu_tier_str,
            image: file.deployment.image.as_deref().unwrap_or("sindri:latest"),
        }
    }

    /// Generate fly.toml from config
    async fn generate_fly_toml(
        &self,
        config: &SindriConfig,
        output_dir: &Path,
        ci_mode: bool,
    ) -> Result<PathBuf> {
        let fly_config = self.get_fly_config(config);
        let dind_mode = "none"; // Fly.io doesn't support DinD
        let mut context = TemplateContext::from_config(config, dind_mode);

        // Set CI mode
        context.ci_mode = ci_mode;

        // Determine which git ref to clone for getting the Dockerfile
        let version_to_fetch = config
            .inner()
            .deployment
            .build_from_source
            .as_ref()
            .and_then(|b| b.git_ref.as_deref());

        // Fetch Sindri v3 build context from GitHub (ADR-034, ADR-037)
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sindri")
            .join("repos");

        let (v3_dir, git_ref_used) =
            fetch_sindri_build_context(&cache_dir, version_to_fetch).await?;
        let repo_dir = v3_dir.parent().unwrap();

        // Determine which git ref to use for Docker build
        let sindri_version = config
            .inner()
            .deployment
            .build_from_source
            .as_ref()
            .and_then(|b| b.git_ref.clone())
            .unwrap_or(git_ref_used);

        // Select appropriate Dockerfile based on build mode
        let should_build_from_source = config
            .inner()
            .deployment
            .build_from_source
            .as_ref()
            .map(|b| b.enabled)
            .unwrap_or(false);

        let dockerfile_name = if should_build_from_source {
            "Dockerfile.dev"
        } else {
            "Dockerfile"
        };

        // For Fly, we need to generate fly.toml in the repo root and use relative paths
        // because flyctl uses the fly.toml directory as the build context
        let dockerfile_path = if output_dir == Path::new(".") || output_dir.is_relative() {
            // Generate fly.toml in repo root for correct build context
            format!("v3/{}", dockerfile_name)
        } else {
            // Use absolute path if custom output_dir specified
            v3_dir.join(dockerfile_name).to_string_lossy().to_string()
        };

        context
            .env_vars
            .insert("dockerfile_path".to_string(), dockerfile_path);

        // Store repo_dir and sindri_version for later use
        context.env_vars.insert(
            "repo_dir".to_string(),
            repo_dir.to_string_lossy().to_string(),
        );
        context
            .env_vars
            .insert("sindri_version".to_string(), sindri_version);

        // Add Fly.io specific context variables
        context
            .env_vars
            .insert("fly_region".to_string(), fly_config.region.to_string());
        context
            .env_vars
            .insert("fly_cpu_kind".to_string(), fly_config.cpu_kind.clone());
        context
            .env_vars
            .insert("fly_ssh_port".to_string(), fly_config.ssh_port.to_string());
        context.env_vars.insert(
            "fly_memory_mb".to_string(),
            fly_config.memory_mb.to_string(),
        );
        context
            .env_vars
            .insert("fly_swap_mb".to_string(), fly_config.swap_mb.to_string());
        context.env_vars.insert(
            "fly_volume_size".to_string(),
            fly_config.volume_size.to_string(),
        );

        let auto_stop_mode = if fly_config.auto_stop {
            "suspend"
        } else {
            "off"
        };
        context
            .env_vars
            .insert("fly_auto_stop_mode".to_string(), auto_stop_mode.to_string());
        context.env_vars.insert(
            "fly_auto_start".to_string(),
            fly_config.auto_start.to_string(),
        );

        // Add GPU-specific context if enabled
        if fly_config.gpu_enabled {
            let (guest_type, gpu_cpus, gpu_memory) = get_fly_gpu_config(&fly_config.gpu_tier);
            context
                .env_vars
                .insert("fly_gpu_guest_type".to_string(), guest_type.to_string());
            context
                .env_vars
                .insert("fly_gpu_cpus".to_string(), gpu_cpus.to_string());
            context
                .env_vars
                .insert("fly_gpu_memory".to_string(), gpu_memory.to_string());
        }

        // Render template
        let fly_toml_content = self.templates.render("fly.toml", &context)?;

        // Determine where to save fly.toml
        // If building from source (repo_dir in env_vars), save in repo root for correct build context
        // Otherwise save in output_dir
        let fly_toml_path = if let Some(repo_dir_str) = context.env_vars.get("repo_dir") {
            let repo_path = PathBuf::from(repo_dir_str);
            std::fs::create_dir_all(&repo_path)?;
            repo_path.join("fly.toml")
        } else {
            std::fs::create_dir_all(output_dir)?;
            output_dir.join("fly.toml")
        };

        std::fs::write(&fly_toml_path, fly_toml_content)?;

        info!("Generated fly.toml at {}", fly_toml_path.display());
        Ok(fly_toml_path)
    }

    /// Check if app exists on Fly.io
    async fn app_exists(&self, name: &str) -> bool {
        let output = Command::new("flyctl")
            .args(["apps", "list", "--json"])
            .output()
            .await;

        output
            .map(|o| {
                if o.status.success() {
                    let stdout = String::from_utf8_lossy(&o.stdout);
                    stdout.contains(&format!("\"{}\"", name))
                } else {
                    false
                }
            })
            .unwrap_or(false)
    }

    /// Create app on Fly.io
    async fn create_app(&self, name: &str, org: &str) -> Result<()> {
        info!("Creating Fly.io app: {}", name);
        let output = Command::new("flyctl")
            .args(["apps", "create", name, "--org", org])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Ignore "already exists" error
            if !stderr.contains("already exists") {
                return Err(anyhow!("Failed to create app: {}", stderr));
            }
        }

        Ok(())
    }

    /// Check if volume exists
    async fn volume_exists(&self, app_name: &str, volume_name: &str) -> bool {
        let output = Command::new("flyctl")
            .args(["volumes", "list", "-a", app_name, "--json"])
            .output()
            .await;

        output
            .map(|o| {
                if o.status.success() {
                    let stdout = String::from_utf8_lossy(&o.stdout);
                    stdout.contains(volume_name)
                } else {
                    false
                }
            })
            .unwrap_or(false)
    }

    /// Create volume on Fly.io
    async fn create_volume(
        &self,
        app_name: &str,
        volume_name: &str,
        size_gb: u32,
        region: &str,
    ) -> Result<()> {
        info!("Creating Fly.io volume: {} ({}GB)", volume_name, size_gb);
        let output = Command::new("flyctl")
            .args([
                "volumes",
                "create",
                volume_name,
                "-s",
                &size_gb.to_string(),
                "-r",
                region,
                "-a",
                app_name,
                "--yes",
            ])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to create volume: {}", stderr));
        }

        Ok(())
    }

    /// Get machine state
    async fn get_machine_state(&self, app_name: &str) -> Result<(Option<String>, DeploymentState)> {
        let output = Command::new("flyctl")
            .args(["machines", "list", "-a", app_name, "--json"])
            .output()
            .await?;

        if !output.status.success() {
            return Ok((None, DeploymentState::NotDeployed));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let machines: Vec<FlyMachine> = serde_json::from_str(&stdout).unwrap_or_default();

        if let Some(machine) = machines.first() {
            let state = match machine.state.as_str() {
                "started" | "running" => DeploymentState::Running,
                "stopped" | "stopping" => DeploymentState::Stopped,
                "suspended" => DeploymentState::Suspended,
                "created" | "starting" => DeploymentState::Creating,
                "destroying" => DeploymentState::Destroying,
                _ => DeploymentState::Unknown,
            };
            Ok((Some(machine.id.clone()), state))
        } else {
            Ok((None, DeploymentState::NotDeployed))
        }
    }

    /// Start a machine
    async fn start_machine(&self, app_name: &str, machine_id: &str) -> Result<()> {
        info!("Starting machine: {}", machine_id);
        let output = Command::new("flyctl")
            .args(["machine", "start", machine_id, "-a", app_name])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to start machine: {}", stderr));
        }

        Ok(())
    }

    /// Stop a machine
    async fn stop_machine(&self, app_name: &str, machine_id: &str) -> Result<()> {
        info!("Stopping machine: {}", machine_id);
        let output = Command::new("flyctl")
            .args(["machine", "stop", machine_id, "-a", app_name])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to stop machine: {}", stderr));
        }

        Ok(())
    }

    /// Run flyctl deploy
    async fn flyctl_deploy(&self, fly_toml_path: &Path, rebuild: bool) -> Result<()> {
        let mut args = vec!["deploy", "--ha=false", "--wait-timeout", "600"];

        if rebuild {
            args.push("--no-cache");
        }

        info!("Deploying to Fly.io...");
        let status = Command::new("flyctl")
            .args(&args)
            .current_dir(fly_toml_path.parent().unwrap_or(Path::new(".")))
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await?;

        if !status.success() {
            return Err(anyhow!("flyctl deploy failed"));
        }

        Ok(())
    }

    /// Deploy using a pre-built image (skip Dockerfile build)
    async fn flyctl_deploy_image(
        &self,
        image: &str,
        fly_toml_path: &Path,
        config: &FlyDeployConfig<'_>,
    ) -> Result<()> {
        info!("Deploying pre-built image to Fly.io: {}", image);

        let mut args = vec![
            "deploy",
            "--image",
            image,
            "--ha=false",
            "--wait-timeout",
            "600",
        ];

        // Add organization if specified (and not "personal")
        let org_string: String;
        if config.organization != "personal" {
            org_string = config.organization.to_string();
            args.push("--org");
            args.push(&org_string);
        }

        let status = Command::new("flyctl")
            .args(&args)
            .current_dir(fly_toml_path.parent().unwrap_or(Path::new(".")))
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await?;

        if !status.success() {
            return Err(anyhow!("flyctl deploy with image failed"));
        }

        Ok(())
    }

    /// Check if a pre-built image should be used instead of building from Dockerfile
    fn should_use_prebuilt_image(&self, config: &SindriConfig) -> bool {
        let file = config.inner();
        file.deployment.image.is_some() || file.deployment.image_config.is_some()
    }

    /// Resolve secrets and set them using flyctl secrets
    async fn resolve_and_set_secrets(
        &self,
        config: &SindriConfig,
        app_name: &str,
        custom_env_file: Option<PathBuf>,
    ) -> Result<()> {
        let secrets = config.secrets();

        // If no secrets configured, skip
        if secrets.is_empty() {
            debug!("No secrets configured, skipping secrets resolution");
            return Ok(());
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

        // Prepare secrets for flyctl
        let mut secret_pairs = Vec::new();
        for (name, secret) in &resolved {
            if let Some(value) = secret.value.as_string() {
                // This is an environment variable secret
                secret_pairs.push(format!("{}={}", name, value));
            } else {
                warn!("Fly.io provider currently only supports environment variable secrets. File secret '{}' will be skipped.", name);
            }
        }

        if secret_pairs.is_empty() {
            debug!("No environment variable secrets to set");
            return Ok(());
        }

        info!("Setting {} secrets via flyctl...", secret_pairs.len());

        // Use flyctl secrets import to set all secrets at once
        let secrets_input = secret_pairs.join("\n");

        let mut child = Command::new("flyctl")
            .args(["secrets", "import", "-a", app_name])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Write secrets to stdin
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(secrets_input.as_bytes()).await?;
            stdin.flush().await?;
        }

        let output = child.wait_with_output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to set secrets: {}", stderr));
        }

        info!("Secrets set successfully");
        Ok(())
    }
}

impl Default for FlyProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for FlyProvider {
    fn name(&self) -> &'static str {
        "fly"
    }

    fn check_prerequisites(&self) -> Result<PrerequisiteStatus> {
        let mut missing = Vec::new();
        let mut available = Vec::new();

        // Check flyctl
        if command_exists("flyctl") {
            let version =
                get_command_version("flyctl", "version").unwrap_or_else(|_| "unknown".to_string());

            if self.is_authenticated() {
                available.push(Prerequisite {
                    name: "flyctl".to_string(),
                    description: "Fly.io CLI (authenticated)".to_string(),
                    install_hint: None,
                    version: Some(version),
                });
            } else {
                missing.push(Prerequisite {
                    name: "flyctl-auth".to_string(),
                    description: "Fly.io authentication".to_string(),
                    install_hint: Some("Run: flyctl auth login".to_string()),
                    version: None,
                });
            }
        } else {
            missing.push(Prerequisite {
                name: "flyctl".to_string(),
                description: "Fly.io CLI".to_string(),
                install_hint: Some("Install: curl -L https://fly.io/install.sh | sh".to_string()),
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
        let fly_config = self.get_fly_config(config);
        let name = fly_config.name.to_string();
        info!("Deploying {} to Fly.io", name);

        // Check prerequisites
        let prereqs = self.check_prerequisites()?;
        if !prereqs.satisfied {
            let missing_names: Vec<_> = prereqs.missing.iter().map(|p| p.name.as_str()).collect();
            return Err(anyhow!(
                "Missing prerequisites: {}",
                missing_names.join(", ")
            ));
        }

        // Generate fly.toml
        let fly_toml_path = self
            .generate_fly_toml(config, &self.output_dir, false)
            .await?;

        if opts.dry_run {
            return Ok(DeployResult {
                success: true,
                name: name.clone(),
                provider: "fly".to_string(),
                instance_id: None,
                connection: None,
                messages: vec![format!(
                    "Would deploy {} using fly.toml at {}",
                    name,
                    fly_toml_path.display()
                )],
                warnings: vec![],
            });
        }

        // Create app if it doesn't exist
        if !self.app_exists(&name).await {
            self.create_app(&name, fly_config.organization).await?;
        }

        // Resolve and set secrets
        self.resolve_and_set_secrets(config, &name, None).await?;

        // Create volume if it doesn't exist
        if !self.volume_exists(&name, "home_data").await {
            self.create_volume(
                &name,
                "home_data",
                fly_config.volume_size,
                fly_config.region,
            )
            .await?;
        }

        // Check if we should use a pre-built image
        if self.should_use_prebuilt_image(config) {
            // Resolve and deploy using pre-built image
            let image = config
                .resolve_image()
                .await
                .map_err(|e| anyhow!("Failed to resolve image: {}", e))?;
            info!("Using pre-built image: {}", image);
            self.flyctl_deploy_image(&image, &fly_toml_path, &fly_config)
                .await?;
        } else {
            // Build from Dockerfile (existing behavior)
            info!("No image specified, building from Dockerfile");
            self.flyctl_deploy(&fly_toml_path, opts.force).await?;
        }

        let ssh_host = format!("{}.fly.dev", name);

        Ok(DeployResult {
            success: true,
            name: name.clone(),
            provider: "fly".to_string(),
            instance_id: Some(name.clone()),
            connection: Some(ConnectionInfo {
                ssh_command: Some(format!(
                    "ssh developer@{} -p {}",
                    ssh_host, fly_config.ssh_port
                )),
                http_url: None,
                https_url: None,
                instructions: Some(format!(
                    "Connect with:\n  sindri connect\n  flyctl ssh console -a {}\n  ssh developer@{} -p {}",
                    name, ssh_host, fly_config.ssh_port
                )),
            }),
            messages: vec![format!("App '{}' deployed successfully to Fly.io", name)],
            warnings: vec![],
        })
    }

    async fn connect(&self, config: &SindriConfig) -> Result<()> {
        let name = config.name();
        info!("Connecting to {} on Fly.io", name);

        // Check if app exists
        if !self.app_exists(name).await {
            return Err(anyhow!(
                "App '{}' not found on Fly.io. Deploy first: sindri deploy",
                name
            ));
        }

        // Check machine state and wake if suspended
        let (machine_id, state) = self.get_machine_state(name).await?;
        if matches!(state, DeploymentState::Suspended | DeploymentState::Stopped) {
            if let Some(id) = &machine_id {
                info!("Machine is {:?}, waking up...", state);
                self.start_machine(name, id).await?;
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }

        // Connect via flyctl ssh console
        let status = Command::new("flyctl")
            .args([
                "ssh",
                "console",
                "-a",
                name,
                "--pty",
                "-C",
                "sh -c 'cat /etc/motd 2>/dev/null; exec su - developer'",
            ])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await?;

        if !status.success() {
            return Err(anyhow!("Failed to connect to Fly.io"));
        }

        Ok(())
    }

    async fn status(&self, config: &SindriConfig) -> Result<DeploymentStatus> {
        let name = config.name().to_string();
        info!("Getting Fly.io status for {}", name);

        let (machine_id, state) = self.get_machine_state(&name).await?;

        // Resolve image using the image_config priority chain
        let image = config.resolve_image().await.ok();

        Ok(DeploymentStatus {
            name,
            provider: "fly".to_string(),
            state,
            instance_id: machine_id,
            image,
            addresses: vec![],
            resources: None,
            timestamps: Default::default(),
            details: Default::default(),
        })
    }

    async fn destroy(&self, config: &SindriConfig, force: bool) -> Result<()> {
        let name = config.name();
        info!("Destroying {} on Fly.io (force: {})", name, force);

        if !self.app_exists(name).await {
            warn!("App '{}' not found on Fly.io", name);
            return Ok(());
        }

        let output = Command::new("flyctl")
            .args(["apps", "destroy", name, "--yes"])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to destroy app: {}", stderr));
        }

        // Remove fly.toml
        let fly_toml_path = self.output_dir.join("fly.toml");
        if fly_toml_path.exists() {
            std::fs::remove_file(fly_toml_path)?;
        }

        info!("App '{}' destroyed", name);
        Ok(())
    }

    async fn plan(&self, config: &SindriConfig) -> Result<DeploymentPlan> {
        let fly_config = self.get_fly_config(config);
        let name = fly_config.name.to_string();
        info!("Planning Fly.io deployment for {}", name);

        // Resolve image using the image_config priority chain
        let image = config.resolve_image().await.map_err(|e| anyhow!("{}", e))?;

        let mut actions = vec![PlannedAction {
            action: ActionType::Create,
            resource: "fly.toml".to_string(),
            description: "Generate Fly.io configuration".to_string(),
        }];

        if !self.app_exists(&name).await {
            actions.push(PlannedAction {
                action: ActionType::Create,
                resource: format!("app:{}", name),
                description: format!("Create Fly.io app in {}", fly_config.region),
            });
        }

        if !self.volume_exists(&name, "home_data").await {
            actions.push(PlannedAction {
                action: ActionType::Create,
                resource: "volume:home_data".to_string(),
                description: format!("Create {}GB persistent volume", fly_config.volume_size),
            });
        }

        actions.push(PlannedAction {
            action: ActionType::Create,
            resource: format!("machine:{}", name),
            description: format!("Deploy Fly.io machine with image {}", image),
        });

        let resources = vec![
            PlannedResource {
                resource_type: "app".to_string(),
                name: name.clone(),
                config: {
                    let mut m = HashMap::new();
                    m.insert("region".to_string(), serde_json::json!(fly_config.region));
                    m.insert(
                        "organization".to_string(),
                        serde_json::json!(fly_config.organization),
                    );
                    m
                },
            },
            PlannedResource {
                resource_type: "volume".to_string(),
                name: "home_data".to_string(),
                config: {
                    let mut m = HashMap::new();
                    m.insert(
                        "size_gb".to_string(),
                        serde_json::json!(fly_config.volume_size),
                    );
                    m
                },
            },
            PlannedResource {
                resource_type: "machine".to_string(),
                name: name.clone(),
                config: {
                    let mut m = HashMap::new();
                    m.insert("cpus".to_string(), serde_json::json!(fly_config.cpus));
                    m.insert(
                        "memory_mb".to_string(),
                        serde_json::json!(fly_config.memory_mb),
                    );
                    m.insert("image".to_string(), serde_json::json!(image));
                    m
                },
            },
        ];

        Ok(DeploymentPlan {
            provider: "fly".to_string(),
            actions,
            resources,
            estimated_cost: None,
        })
    }

    async fn start(&self, config: &SindriConfig) -> Result<()> {
        let name = config.name();
        info!("Starting {} on Fly.io", name);

        let (machine_id, _) = self.get_machine_state(name).await?;
        if let Some(id) = machine_id {
            self.start_machine(name, &id).await?;
        } else {
            return Err(anyhow!("No machine found for app '{}'", name));
        }

        Ok(())
    }

    async fn stop(&self, config: &SindriConfig) -> Result<()> {
        let name = config.name();
        info!("Stopping {} on Fly.io", name);

        let (machine_id, _) = self.get_machine_state(name).await?;
        if let Some(id) = machine_id {
            self.stop_machine(name, &id).await?;
        } else {
            return Err(anyhow!("No machine found for app '{}'", name));
        }

        Ok(())
    }

    fn supports_gpu(&self) -> bool {
        true // Fly.io supports A100 and L40s GPUs
    }

    fn supports_auto_suspend(&self) -> bool {
        true // Fly.io machines support auto-suspend
    }
}

/// Fly.io deployment configuration
struct FlyDeployConfig<'a> {
    name: &'a str,
    region: &'a str,
    organization: &'a str,
    cpu_kind: String,
    cpus: u32,
    memory_mb: u32,
    swap_mb: u32,
    ssh_port: u16,
    auto_stop: bool,
    auto_start: bool,
    volume_size: u32,
    gpu_enabled: bool,
    gpu_tier: String,
    #[allow(dead_code)] // Used in plan() for resource details
    image: &'a str,
}

/// Fly.io machine status from JSON API
#[derive(Debug, Deserialize)]
struct FlyMachine {
    id: String,
    state: String,
}

/// Parse memory string to MB
fn parse_memory_to_mb(mem: &str) -> Option<u32> {
    let mem = mem.trim().to_uppercase();

    if let Some(gb) = mem.strip_suffix("GB") {
        gb.parse::<u32>().ok().map(|v| v * 1024)
    } else if let Some(mb) = mem.strip_suffix("MB") {
        mb.parse::<u32>().ok()
    } else {
        None
    }
}

/// Parse size string to GB
fn parse_size_to_gb(size: &str) -> Option<u32> {
    let size = size.trim().to_uppercase();

    if let Some(gb) = size.strip_suffix("GB") {
        gb.parse::<u32>().ok()
    } else if let Some(tb) = size.strip_suffix("TB") {
        tb.parse::<u32>().ok().map(|v| v * 1024)
    } else {
        None
    }
}

/// Get Fly.io GPU configuration for a tier
/// Returns (guest_type, cpus, memory_mb)
fn get_fly_gpu_config(tier: &str) -> (&'static str, u32, u32) {
    match tier {
        "gpu-small" => ("a100-40gb", 8, 32768),
        "gpu-medium" => ("a100-40gb", 16, 65536),
        "gpu-large" => ("l40s", 16, 65536),
        "gpu-xlarge" => ("a100-80gb", 32, 131072),
        _ => ("a100-40gb", 8, 32768),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_memory_to_mb() {
        assert_eq!(parse_memory_to_mb("2GB"), Some(2048));
        assert_eq!(parse_memory_to_mb("512MB"), Some(512));
        assert_eq!(parse_memory_to_mb("4gb"), Some(4096));
    }

    #[test]
    fn test_parse_size_to_gb() {
        assert_eq!(parse_size_to_gb("10GB"), Some(10));
        assert_eq!(parse_size_to_gb("1TB"), Some(1024));
    }

    #[test]
    fn test_get_fly_gpu_config() {
        let (guest, cpus, mem) = get_fly_gpu_config("gpu-small");
        assert_eq!(guest, "a100-40gb");
        assert_eq!(cpus, 8);
        assert_eq!(mem, 32768);
    }
}
