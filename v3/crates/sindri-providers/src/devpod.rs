//! DevPod provider implementation

use crate::templates::{TemplateContext, TemplateRegistry};
use crate::traits::Provider;
use crate::utils::{command_exists, find_dockerfile_or_error, get_command_version};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::Deserialize;
use sindri_core::config::SindriConfig;
use sindri_core::types::{
    ActionType, ConnectionInfo, DeployOptions, DeployResult, DeploymentPlan, DeploymentState,
    DeploymentStatus, PlannedAction, PlannedResource, Prerequisite, PrerequisiteStatus,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tracing::{info, warn};

/// DevPod provider for multi-cloud development environments
pub struct DevPodProvider {
    /// Template registry for generating devcontainer.json
    templates: TemplateRegistry,
    /// Output directory for generated files
    output_dir: PathBuf,
}

impl DevPodProvider {
    /// Create a new DevPod provider
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

    /// Get DevPod provider type from config
    fn get_devpod_provider(&self, config: &SindriConfig) -> String {
        config
            .inner()
            .providers
            .devpod
            .as_ref()
            .map(|d| format!("{:?}", d.r#type).to_lowercase())
            .unwrap_or_else(|| "docker".to_string())
    }

    /// Get build repository from config
    fn get_build_repository(&self, config: &SindriConfig) -> Option<String> {
        config
            .inner()
            .providers
            .devpod
            .as_ref()
            .and_then(|d| d.build_repository.clone())
    }

    /// Detect if running against a local Kubernetes cluster (kind or k3d)
    async fn detect_local_k8s_cluster(&self, context: Option<&str>) -> Option<LocalCluster> {
        // Check for kind cluster
        if command_exists("kind") {
            let current_context = self.get_k8s_current_context(context).await.ok()?;

            if current_context.starts_with("kind-") {
                let cluster_name = current_context.strip_prefix("kind-")?;
                // Verify cluster exists
                if self.kind_cluster_exists(cluster_name).await {
                    return Some(LocalCluster {
                        cluster_type: LocalClusterType::Kind,
                        name: cluster_name.to_string(),
                    });
                }
            }
        }

        // Check for k3d cluster
        if command_exists("k3d") {
            let current_context = self.get_k8s_current_context(context).await.ok()?;

            if current_context.starts_with("k3d-") {
                let cluster_name = current_context.strip_prefix("k3d-")?;
                if self.k3d_cluster_exists(cluster_name).await {
                    return Some(LocalCluster {
                        cluster_type: LocalClusterType::K3d,
                        name: cluster_name.to_string(),
                    });
                }
            }
        }

        None
    }

    /// Get current Kubernetes context
    async fn get_k8s_current_context(&self, context: Option<&str>) -> Result<String> {
        if let Some(ctx) = context {
            return Ok(ctx.to_string());
        }

        let output = Command::new("kubectl")
            .args(["config", "current-context"])
            .output()
            .await?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(anyhow!("Failed to get Kubernetes context"))
        }
    }

    /// Check if kind cluster exists
    async fn kind_cluster_exists(&self, name: &str) -> bool {
        let output = Command::new("kind")
            .args(["get", "clusters"])
            .output()
            .await;

        output
            .map(|o| {
                let stdout = String::from_utf8_lossy(&o.stdout);
                stdout.lines().any(|line| line.trim() == name)
            })
            .unwrap_or(false)
    }

    /// Check if k3d cluster exists
    async fn k3d_cluster_exists(&self, name: &str) -> bool {
        let output = Command::new("k3d").args(["cluster", "list"]).output().await;

        output
            .map(|o| {
                let stdout = String::from_utf8_lossy(&o.stdout);
                stdout.contains(name)
            })
            .unwrap_or(false)
    }

    /// Get Docker credentials from environment or files
    fn get_docker_credentials(&self) -> Option<DockerCredentials> {
        // Check environment variables first
        if let (Ok(username), Ok(password)) = (
            std::env::var("DOCKER_USERNAME"),
            std::env::var("DOCKER_PASSWORD"),
        ) {
            return Some(DockerCredentials {
                username,
                password,
                registry: std::env::var("DOCKER_REGISTRY").ok(),
            });
        }

        // Check .env.local
        if let Ok(content) = std::fs::read_to_string(".env.local") {
            if let Some(creds) = Self::parse_env_file(&content) {
                return Some(creds);
            }
        }

        // Check .env
        if let Ok(content) = std::fs::read_to_string(".env") {
            if let Some(creds) = Self::parse_env_file(&content) {
                return Some(creds);
            }
        }

        None
    }

    /// Parse .env file for Docker credentials
    fn parse_env_file(content: &str) -> Option<DockerCredentials> {
        let mut username = None;
        let mut password = None;
        let mut registry = None;

        for line in content.lines() {
            if let Some(value) = line.strip_prefix("DOCKER_USERNAME=") {
                username = Some(value.trim_matches('"').to_string());
            } else if let Some(value) = line.strip_prefix("DOCKER_PASSWORD=") {
                password = Some(value.trim_matches('"').to_string());
            } else if let Some(value) = line.strip_prefix("DOCKER_REGISTRY=") {
                registry = Some(value.trim_matches('"').to_string());
            }
        }

        if let (Some(u), Some(p)) = (username, password) {
            Some(DockerCredentials {
                username: u,
                password: p,
                registry,
            })
        } else {
            None
        }
    }

    /// Login to Docker registry
    async fn docker_login(&self, registry: &str, creds: &DockerCredentials) -> Result<()> {
        info!("Logging in to Docker registry: {}", registry);

        let mut child = Command::new("docker")
            .args(["login", registry, "-u", &creds.username, "--password-stdin"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(creds.password.as_bytes()).await?;
            stdin.flush().await?;
        }

        let output = child.wait_with_output().await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Docker login failed: {}", stderr));
        }

        Ok(())
    }

    /// Build Docker image
    async fn build_image(&self, tag: &str, dockerfile: &Path, context_dir: &Path) -> Result<()> {
        info!("Building Docker image: {}", tag);

        let status = Command::new("docker")
            .args([
                "build",
                "-t",
                tag,
                "-f",
                &dockerfile.to_string_lossy(),
                &context_dir.to_string_lossy(),
            ])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await?;

        if !status.success() {
            return Err(anyhow!("Docker build failed"));
        }

        Ok(())
    }

    /// Push Docker image to registry
    async fn push_image(&self, tag: &str) -> Result<()> {
        info!("Pushing image to registry: {}", tag);

        let status = Command::new("docker")
            .args(["push", tag])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await?;

        if !status.success() {
            return Err(anyhow!("Docker push failed"));
        }

        Ok(())
    }

    /// Load image into local Kubernetes cluster
    async fn load_image_to_local_cluster(&self, image: &str, cluster: &LocalCluster) -> Result<()> {
        info!(
            "Loading image into {} cluster: {}",
            cluster.cluster_type, cluster.name
        );

        match cluster.cluster_type {
            LocalClusterType::Kind => {
                let status = Command::new("kind")
                    .args(["load", "docker-image", image, "--name", &cluster.name])
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .status()
                    .await?;

                if !status.success() {
                    return Err(anyhow!("Failed to load image into kind cluster"));
                }
            }
            LocalClusterType::K3d => {
                let status = Command::new("k3d")
                    .args(["image", "import", image, "--cluster", &cluster.name])
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .status()
                    .await?;

                if !status.success() {
                    return Err(anyhow!("Failed to load image into k3d cluster"));
                }
            }
        }

        Ok(())
    }

    /// Prepare image for DevPod deployment
    async fn prepare_image(
        &self,
        config: &SindriConfig,
        provider_type: &str,
    ) -> Result<Option<String>> {
        // Find Dockerfile using standard search paths (ADR-035)
        let dockerfile = find_dockerfile_or_error()?;
        let base_dir = dockerfile.parent().unwrap_or(Path::new("."));

        // For docker provider, use Dockerfile directly
        if provider_type == "docker" {
            return Ok(None);
        }

        // Check for local K8s cluster
        let k8s_context = config
            .inner()
            .providers
            .devpod
            .as_ref()
            .and_then(|d| d.kubernetes.as_ref())
            .and_then(|k| k.context.as_deref());

        if provider_type == "kubernetes" {
            if let Some(local_cluster) = self.detect_local_k8s_cluster(k8s_context).await {
                // Build and load into local cluster
                let image_tag = "sindri:latest";
                self.build_image(image_tag, &dockerfile, base_dir).await?;
                self.load_image_to_local_cluster(image_tag, &local_cluster)
                    .await?;
                return Ok(Some(image_tag.to_string()));
            }
        }

        // For cloud providers, require build repository
        if matches!(provider_type, "kubernetes" | "aws" | "gcp" | "azure") {
            let build_repo = self.get_build_repository(config).ok_or_else(|| {
                anyhow!(
                    "Build repository required for {} provider. Set providers.devpod.buildRepository in sindri.yaml",
                    provider_type
                )
            })?;

            let image_tag = format!("{}:latest", build_repo);
            let registry_host = build_repo.split('/').next().unwrap_or("docker.io");

            // Get credentials and login
            if let Some(creds) = self.get_docker_credentials() {
                let registry = creds.registry.as_deref().unwrap_or(registry_host);
                self.docker_login(registry, &creds).await?;
            }

            // Build and push
            self.build_image(&image_tag, &dockerfile, base_dir).await?;
            self.push_image(&image_tag).await?;

            return Ok(Some(image_tag));
        }

        Ok(None)
    }

    /// Generate devcontainer.json
    fn generate_devcontainer(
        &self,
        config: &SindriConfig,
        image_tag: Option<&str>,
    ) -> Result<PathBuf> {
        let devcontainer_dir = self.output_dir.join(".devcontainer");
        std::fs::create_dir_all(&devcontainer_dir)?;

        let mut context = TemplateContext::from_config(config, "none");

        // Set image or use Dockerfile
        if let Some(image) = image_tag {
            context.image = image.to_string();
        }

        let content = self.templates.render("devcontainer.json", &context)?;
        let devcontainer_path = devcontainer_dir.join("devcontainer.json");
        std::fs::write(&devcontainer_path, content)?;

        info!(
            "Generated devcontainer.json at {}",
            devcontainer_path.display()
        );
        Ok(devcontainer_path)
    }

    /// Check if DevPod provider is added
    async fn provider_exists(&self, provider: &str) -> bool {
        let output = Command::new("devpod")
            .args(["provider", "list"])
            .output()
            .await;

        output
            .map(|o| {
                let stdout = String::from_utf8_lossy(&o.stdout);
                stdout.lines().any(|line| line.starts_with(provider))
            })
            .unwrap_or(false)
    }

    /// Add DevPod provider
    async fn add_provider(&self, provider: &str) -> Result<()> {
        info!("Adding DevPod provider: {}", provider);

        let status = Command::new("devpod")
            .args(["provider", "add", provider])
            .status()
            .await?;

        if !status.success() {
            return Err(anyhow!("Failed to add DevPod provider: {}", provider));
        }

        Ok(())
    }

    /// Configure Kubernetes provider options
    async fn configure_k8s_provider(&self, config: &SindriConfig) -> Result<()> {
        let k8s_config = config
            .inner()
            .providers
            .devpod
            .as_ref()
            .and_then(|d| d.kubernetes.as_ref());

        if let Some(k8s) = k8s_config {
            if let Some(context) = &k8s.context {
                info!("Setting Kubernetes context: {}", context);
                let _ = Command::new("devpod")
                    .args([
                        "provider",
                        "set-options",
                        "kubernetes",
                        "-o",
                        &format!("KUBERNETES_CONTEXT={}", context),
                    ])
                    .output()
                    .await;
            }

            let namespace = &k8s.namespace;
            info!("Setting Kubernetes namespace: {}", namespace);
            let _ = Command::new("devpod")
                .args([
                    "provider",
                    "set-options",
                    "kubernetes",
                    "-o",
                    &format!("KUBERNETES_NAMESPACE={}", namespace),
                ])
                .output()
                .await;

            // Create namespace if it doesn't exist
            let context_args = if let Some(ctx) = &k8s.context {
                vec!["--context", ctx.as_str()]
            } else {
                vec![]
            };

            let mut cmd = Command::new("kubectl");
            cmd.args(&context_args)
                .args(["get", "namespace", namespace]);

            if !cmd.output().await?.status.success() {
                info!("Creating namespace: {}", namespace);
                let mut create_cmd = Command::new("kubectl");
                create_cmd
                    .args(&context_args)
                    .args(["create", "namespace", namespace]);
                let _ = create_cmd.output().await;
            }

            if let Some(storage_class) = &k8s.storage_class {
                info!("Setting Kubernetes storage class: {}", storage_class);
                let _ = Command::new("devpod")
                    .args([
                        "provider",
                        "set-options",
                        "kubernetes",
                        "-o",
                        &format!("KUBERNETES_STORAGE_CLASS={}", storage_class),
                    ])
                    .output()
                    .await;
            }
        }

        Ok(())
    }

    /// Check if workspace exists
    async fn workspace_exists(&self, name: &str) -> bool {
        let output = Command::new("devpod")
            .args(["list", "--output", "json"])
            .output()
            .await;

        output
            .map(|o| {
                if o.status.success() {
                    let stdout = String::from_utf8_lossy(&o.stdout);
                    stdout.contains(&format!("\"id\":\"{}\"", name))
                } else {
                    false
                }
            })
            .unwrap_or(false)
    }

    /// Get workspace status
    async fn get_workspace_status(&self, name: &str) -> Result<(Option<String>, DeploymentState)> {
        let output = Command::new("devpod")
            .args(["status", name, "--output", "json"])
            .output()
            .await?;

        if !output.status.success() {
            return Ok((None, DeploymentState::NotDeployed));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let status: DevPodStatus = serde_json::from_str(&stdout).unwrap_or_default();

        let state = match status.state.as_str() {
            "Running" => DeploymentState::Running,
            "Stopped" => DeploymentState::Stopped,
            "NotFound" => DeploymentState::NotDeployed,
            _ => DeploymentState::Unknown,
        };

        Ok((Some(name.to_string()), state))
    }

    /// Deploy DevPod workspace
    async fn devpod_up(&self, name: &str, provider: &str, devcontainer_dir: &Path) -> Result<()> {
        info!("Running: devpod up");

        let status = Command::new("devpod")
            .args([
                "up",
                ".",
                "--provider",
                provider,
                "--id",
                name,
                "--ide",
                "none",
            ])
            .current_dir(devcontainer_dir.parent().unwrap_or(Path::new(".")))
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await?;

        if !status.success() {
            return Err(anyhow!("DevPod deployment failed"));
        }

        Ok(())
    }
}

impl Default for DevPodProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for DevPodProvider {
    fn name(&self) -> &'static str {
        "devpod"
    }

    fn check_prerequisites(&self) -> Result<PrerequisiteStatus> {
        let mut missing = Vec::new();
        let mut available = Vec::new();

        // Check devpod
        if command_exists("devpod") {
            let version =
                get_command_version("devpod", "version").unwrap_or_else(|_| "unknown".to_string());
            available.push(Prerequisite {
                name: "devpod".to_string(),
                description: "DevPod CLI".to_string(),
                install_hint: None,
                version: Some(version),
            });
        } else {
            missing.push(Prerequisite {
                name: "devpod".to_string(),
                description: "DevPod CLI".to_string(),
                install_hint: Some(
                    "Install: https://devpod.sh/docs/getting-started/install".to_string(),
                ),
                version: None,
            });
        }

        // Check docker (required for local DevPod and image building)
        if command_exists("docker") {
            available.push(Prerequisite {
                name: "docker".to_string(),
                description: "Docker (for local DevPod provider)".to_string(),
                install_hint: None,
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
        let name = config.name().to_string();
        let provider_type = self.get_devpod_provider(config);

        info!("Deploying {} with DevPod provider: {}", name, provider_type);

        // Check prerequisites
        let prereqs = self.check_prerequisites()?;
        if !prereqs.satisfied {
            let missing_names: Vec<_> = prereqs.missing.iter().map(|p| p.name.as_str()).collect();
            return Err(anyhow!(
                "Missing prerequisites: {}",
                missing_names.join(", ")
            ));
        }

        // Prepare image (build/push/load as needed)
        let image_tag = self.prepare_image(config, &provider_type).await?;

        // Generate devcontainer.json
        let devcontainer_path = self.generate_devcontainer(config, image_tag.as_deref())?;

        if opts.dry_run {
            return Ok(DeployResult {
                success: true,
                name: name.clone(),
                provider: "devpod".to_string(),
                instance_id: None,
                connection: None,
                messages: vec![format!(
                    "Would deploy {} using devcontainer.json at {}",
                    name,
                    devcontainer_path.display()
                )],
                warnings: vec![],
            });
        }

        // Ensure provider is added
        if !self.provider_exists(&provider_type).await {
            self.add_provider(&provider_type).await?;
        }

        // Configure Kubernetes provider if needed
        if provider_type == "kubernetes" {
            self.configure_k8s_provider(config).await?;
        }

        // Deploy workspace
        self.devpod_up(&name, &provider_type, &devcontainer_path)
            .await?;

        Ok(DeployResult {
            success: true,
            name: name.clone(),
            provider: "devpod".to_string(),
            instance_id: Some(name.clone()),
            connection: Some(ConnectionInfo {
                ssh_command: Some(format!("devpod ssh {}", name)),
                http_url: None,
                https_url: None,
                instructions: Some(format!(
                    "Connect with:\n  sindri connect\n  devpod ssh {}",
                    name
                )),
            }),
            messages: vec![format!("DevPod workspace '{}' deployed successfully", name)],
            warnings: vec![],
        })
    }

    async fn connect(&self, config: &SindriConfig) -> Result<()> {
        let name = config.name();
        info!("Connecting to {} via DevPod", name);

        if !self.workspace_exists(name).await {
            return Err(anyhow!(
                "Workspace '{}' not found. Deploy first: sindri deploy",
                name
            ));
        }

        let status = Command::new("devpod")
            .args(["ssh", name])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await?;

        if !status.success() {
            return Err(anyhow!("Failed to connect to DevPod workspace"));
        }

        Ok(())
    }

    async fn status(&self, config: &SindriConfig) -> Result<DeploymentStatus> {
        let name = config.name().to_string();
        info!("Getting DevPod status for {}", name);

        let (instance_id, state) = self.get_workspace_status(&name).await?;

        Ok(DeploymentStatus {
            name,
            provider: "devpod".to_string(),
            state,
            instance_id,
            image: config.image().map(|s| s.to_string()),
            addresses: vec![],
            resources: None,
            timestamps: Default::default(),
            details: Default::default(),
        })
    }

    async fn destroy(&self, config: &SindriConfig, force: bool) -> Result<()> {
        let name = config.name();
        info!("Destroying DevPod workspace: {} (force: {})", name, force);

        if !self.workspace_exists(name).await {
            warn!("Workspace '{}' not found", name);
            return Ok(());
        }

        // Stop workspace
        let _ = Command::new("devpod").args(["stop", name]).output().await;

        // Delete workspace
        let mut args = vec!["delete", name];
        if force {
            args.push("--force");
        }

        let output = Command::new("devpod").args(&args).output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to destroy workspace: {}", stderr));
        }

        // Remove devcontainer directory
        let devcontainer_dir = self.output_dir.join(".devcontainer");
        if devcontainer_dir.exists() {
            std::fs::remove_dir_all(devcontainer_dir)?;
        }

        info!("Workspace destroyed");
        Ok(())
    }

    async fn plan(&self, config: &SindriConfig) -> Result<DeploymentPlan> {
        let name = config.name().to_string();
        let provider_type = self.get_devpod_provider(config);

        info!("Planning DevPod deployment for {}", name);

        let file = config.inner();
        let mut actions = vec![PlannedAction {
            action: ActionType::Create,
            resource: "devcontainer.json".to_string(),
            description: "Generate DevPod devcontainer configuration".to_string(),
        }];

        // Check if image needs to be built/pushed
        if provider_type != "docker" {
            let build_repo = self.get_build_repository(config);
            if build_repo.is_some() {
                actions.push(PlannedAction {
                    action: ActionType::Create,
                    resource: "image:sindri".to_string(),
                    description: "Build and push Docker image".to_string(),
                });
            }
        }

        if !self.provider_exists(&provider_type).await {
            actions.push(PlannedAction {
                action: ActionType::Create,
                resource: format!("provider:{}", provider_type),
                description: format!("Add DevPod provider: {}", provider_type),
            });
        }

        actions.push(PlannedAction {
            action: ActionType::Create,
            resource: format!("workspace:{}", name),
            description: "Create DevPod workspace".to_string(),
        });

        let resources = vec![PlannedResource {
            resource_type: "workspace".to_string(),
            name: name.clone(),
            config: {
                let mut m = HashMap::new();
                m.insert("provider".to_string(), serde_json::json!(provider_type));
                m.insert(
                    "memory".to_string(),
                    serde_json::json!(file.deployment.resources.memory.as_deref().unwrap_or("4GB")),
                );
                m.insert(
                    "cpus".to_string(),
                    serde_json::json!(file.deployment.resources.cpus.unwrap_or(2)),
                );
                m
            },
        }];

        Ok(DeploymentPlan {
            provider: "devpod".to_string(),
            actions,
            resources,
            estimated_cost: None,
        })
    }

    async fn start(&self, config: &SindriConfig) -> Result<()> {
        let name = config.name();
        info!("Starting DevPod workspace: {}", name);

        let status = Command::new("devpod").args(["up", name]).status().await?;

        if !status.success() {
            return Err(anyhow!("Failed to start workspace"));
        }

        Ok(())
    }

    async fn stop(&self, config: &SindriConfig) -> Result<()> {
        let name = config.name();
        info!("Stopping DevPod workspace: {}", name);

        let status = Command::new("devpod").args(["stop", name]).status().await?;

        if !status.success() {
            return Err(anyhow!("Failed to stop workspace"));
        }

        Ok(())
    }

    fn supports_gpu(&self) -> bool {
        true // DevPod can deploy to GPU instances on AWS/GCP/Azure
    }
}

/// Local Kubernetes cluster detection
struct LocalCluster {
    cluster_type: LocalClusterType,
    name: String,
}

enum LocalClusterType {
    Kind,
    K3d,
}

impl std::fmt::Display for LocalClusterType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LocalClusterType::Kind => write!(f, "kind"),
            LocalClusterType::K3d => write!(f, "k3d"),
        }
    }
}

/// Docker registry credentials
struct DockerCredentials {
    username: String,
    password: String,
    registry: Option<String>,
}

/// DevPod workspace status from JSON
#[derive(Debug, Default, Deserialize)]
struct DevPodStatus {
    #[serde(default)]
    state: String,
}
