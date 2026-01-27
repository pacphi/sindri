//! E2B provider implementation

use crate::templates::{TemplateContext, TemplateRegistry};
use crate::traits::Provider;
use crate::utils::{
    command_exists, copy_dir_recursive, fetch_sindri_build_context, get_command_version,
};
use anyhow::{anyhow, Context, Result};
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

/// E2B provider for cloud sandboxes
pub struct E2bProvider {
    /// Template registry for generating E2B templates
    templates: TemplateRegistry,
    /// Output directory for generated files
    output_dir: PathBuf,
}

impl E2bProvider {
    /// Create a new E2B provider
    pub fn new() -> Self {
        Self {
            templates: TemplateRegistry::new().expect("Failed to initialize templates"),
            output_dir: std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(".e2b"),
        }
    }

    /// Create with a specific output directory
    pub fn with_output_dir(output_dir: PathBuf) -> Self {
        Self {
            templates: TemplateRegistry::new().expect("Failed to initialize templates"),
            output_dir,
        }
    }

    /// Check if E2B API key is set
    fn has_api_key(&self) -> bool {
        std::env::var("E2B_API_KEY").is_ok()
    }

    /// Get E2B configuration from SindriConfig
    fn get_e2b_config(&self, config: &SindriConfig) -> E2bDeployConfig {
        let file = config.inner();
        let e2b = file.providers.e2b.as_ref();

        // Generate template alias from name (lowercase, alphanumeric with hyphens)
        let template_alias = e2b
            .and_then(|e| e.template_alias.clone())
            .unwrap_or_else(|| {
                file.name
                    .to_lowercase()
                    .replace(|c: char| !c.is_alphanumeric() && c != '-', "-")
            });

        // Memory in MB (convert from GB/MB string)
        let memory_raw = file.deployment.resources.memory.as_deref().unwrap_or("2GB");
        let memory_mb = parse_memory_to_mb(memory_raw).unwrap_or(2048);

        let cpus = file.deployment.resources.cpus.unwrap_or(2);

        // Profile
        let profile = file
            .extensions
            .profile
            .clone()
            .unwrap_or_else(|| "base".to_string());

        // Custom extensions (comma-separated)
        let custom_extensions = file
            .extensions
            .active
            .as_ref()
            .map(|exts| exts.join(","))
            .unwrap_or_default();

        // Additional extensions (comma-separated)
        let additional_extensions = file
            .extensions
            .additional
            .as_ref()
            .map(|exts| exts.join(","))
            .unwrap_or_default();

        E2bDeployConfig {
            name: file.name.clone(),
            template_alias,
            profile,
            custom_extensions,
            additional_extensions,
            skip_auto_install: !file.extensions.auto_install,
            cpus,
            memory_mb,
            timeout: e2b.map(|e| e.timeout).unwrap_or(300),
            auto_pause: e2b.map(|e| e.auto_pause).unwrap_or(true),
            auto_resume: e2b.map(|e| e.auto_resume).unwrap_or(true),
            reuse_template: e2b.map(|e| e.reuse_template).unwrap_or(true),
            build_on_deploy: e2b.map(|e| e.build_on_deploy).unwrap_or(false),
            team: e2b.and_then(|e| e.team.clone()),
            metadata: e2b.map(|e| e.metadata.clone()).unwrap_or_default(),
        }
    }

    /// Find sandbox by name (using metadata)
    async fn find_sandbox_by_name(&self, name: &str) -> Option<String> {
        let output = Command::new("e2b")
            .args(["sandbox", "list", "--json"])
            .output()
            .await
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let sandboxes: Vec<E2bSandbox> = serde_json::from_str(&stdout).ok()?;

        sandboxes
            .into_iter()
            .find(|s| {
                s.metadata
                    .as_ref()
                    .and_then(|m| m.get("sindri_name"))
                    .map(|n| n == name)
                    .unwrap_or(false)
            })
            .map(|s| s.sandbox_id)
    }

    /// Get sandbox state
    async fn get_sandbox_state(&self, sandbox_id: &str) -> Option<E2bSandbox> {
        let output = Command::new("e2b")
            .args(["sandbox", "list", "--json"])
            .output()
            .await
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let sandboxes: Vec<E2bSandbox> = serde_json::from_str(&stdout).ok()?;

        sandboxes.into_iter().find(|s| s.sandbox_id == sandbox_id)
    }

    /// Check if template exists
    async fn template_exists(&self, alias: &str) -> bool {
        let output = Command::new("e2b")
            .args(["template", "list", "--json"])
            .output()
            .await;

        if let Ok(o) = output {
            if o.status.success() {
                let stdout = String::from_utf8_lossy(&o.stdout);
                if let Ok(templates) = serde_json::from_str::<Vec<E2bTemplate>>(&stdout) {
                    return templates
                        .iter()
                        .any(|t| t.template_id == alias || t.alias.as_deref() == Some(alias));
                }
            }
        }

        false
    }

    /// Get template ID from alias
    async fn get_template_id(&self, alias: &str) -> Option<String> {
        let output = Command::new("e2b")
            .args(["template", "list", "--json"])
            .output()
            .await
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let templates: Vec<E2bTemplate> = serde_json::from_str(&stdout).ok()?;

        templates
            .into_iter()
            .find(|t| t.alias.as_deref() == Some(alias))
            .map(|t| t.template_id)
    }

    /// Generate E2B Dockerfile from Sindri Dockerfile
    async fn generate_e2b_dockerfile(
        &self,
        _config: &SindriConfig,
        e2b_config: &E2bDeployConfig,
        output_dir: &Path,
    ) -> Result<PathBuf> {
        // Fetch Sindri v3 build context from GitHub (ADR-034, ADR-037)
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sindri")
            .join("repos");

        let (v3_dir, git_ref_used) = fetch_sindri_build_context(&cache_dir, None).await?;
        let dockerfile_path = v3_dir.join("Dockerfile");

        // Determine which git ref to use for Docker build
        let sindri_version = _config
            .inner()
            .deployment
            .build_from_source
            .as_ref()
            .and_then(|b| b.git_ref.clone())
            .unwrap_or(git_ref_used);

        let dockerfile_content =
            std::fs::read_to_string(&dockerfile_path).context("Failed to read Dockerfile")?;

        let template_dir = output_dir.join("template");
        std::fs::create_dir_all(&template_dir)?;

        // Copy v3 directory to template_dir to preserve COPY statement paths
        // E2B builds from template_dir, so we need v3/docker, v3/bin, etc. to exist there
        // The binary is already built and placed in v3/bin/ by build_and_prepare_binary()
        let dest_v3_dir = template_dir.join("v3");
        if dest_v3_dir.exists() {
            std::fs::remove_dir_all(&dest_v3_dir)?;
        }
        copy_dir_recursive(&v3_dir, &dest_v3_dir)?;

        let mut e2b_dockerfile = String::from("# E2B Template Dockerfile for Sindri\n");
        e2b_dockerfile
            .push_str("# Generated from Sindri Dockerfile with E2B-specific configuration\n\n");

        // Set BUILD_FROM_SOURCE and SINDRI_VERSION for source builds
        let dockerfile_with_args = dockerfile_content
            .replace("ARG BUILD_FROM_SOURCE=false", "ARG BUILD_FROM_SOURCE=true")
            .replace(
                "ARG SINDRI_VERSION=3.0.0",
                &format!("ARG SINDRI_VERSION={}", sindri_version),
            );

        e2b_dockerfile.push_str(&dockerfile_with_args);

        // Add E2B-specific environment variables
        e2b_dockerfile.push_str("\n# E2B-specific configuration\n");
        e2b_dockerfile.push_str("ENV E2B_PROVIDER=true\n");
        e2b_dockerfile.push_str(&format!("ENV INSTALL_PROFILE=\"{}\"\n", e2b_config.profile));
        e2b_dockerfile.push_str(&format!(
            "ENV CUSTOM_EXTENSIONS=\"{}\"\n",
            e2b_config.custom_extensions
        ));
        e2b_dockerfile.push_str(&format!(
            "ENV ADDITIONAL_EXTENSIONS=\"{}\"\n",
            e2b_config.additional_extensions
        ));
        e2b_dockerfile.push_str(&format!(
            "ENV SKIP_AUTO_INSTALL=\"{}\"\n",
            e2b_config.skip_auto_install
        ));
        e2b_dockerfile.push_str("ENV INIT_WORKSPACE=true\n");

        // Add NPM_TOKEN if set (for CI)
        if let Ok(npm_token) = std::env::var("NPM_TOKEN") {
            e2b_dockerfile = e2b_dockerfile.replace(
                "WORKDIR /alt/home/developer/workspace",
                &format!(
                    "ENV NPM_TOKEN=\"{}\"\nWORKDIR /alt/home/developer/workspace",
                    npm_token
                ),
            );
        }

        e2b_dockerfile.push_str("\n# Set working directory for E2B\n");
        e2b_dockerfile.push_str("WORKDIR /alt/home/developer/workspace\n\n");
        e2b_dockerfile.push_str("# Switch to developer user\n");
        e2b_dockerfile.push_str("USER developer\n\n");

        let e2b_dockerfile_path = template_dir.join("e2b.Dockerfile");
        std::fs::write(&e2b_dockerfile_path, e2b_dockerfile)?;

        info!(
            "Generated e2b.Dockerfile at {}",
            e2b_dockerfile_path.display()
        );
        Ok(e2b_dockerfile_path)
    }

    /// Generate e2b.toml configuration
    fn generate_e2b_toml(
        &self,
        config: &SindriConfig,
        e2b_config: &E2bDeployConfig,
        output_dir: &Path,
    ) -> Result<PathBuf> {
        let template_dir = output_dir.join("template");
        std::fs::create_dir_all(&template_dir)?;

        // Create template context
        let dind_mode = "none"; // E2B doesn't support DinD
        let mut context = TemplateContext::from_config(config, dind_mode);

        // Add E2B-specific variables
        context.env_vars.insert(
            "e2b_template_alias".to_string(),
            e2b_config.template_alias.clone(),
        );
        context.env_vars.insert(
            "e2b_memory_mb".to_string(),
            e2b_config.memory_mb.to_string(),
        );

        // Render template
        let toml_content = self.templates.render("e2b.toml", &context)?;

        let e2b_toml_path = template_dir.join("e2b.toml");
        std::fs::write(&e2b_toml_path, toml_content)?;

        info!("Generated e2b.toml at {}", e2b_toml_path.display());
        Ok(e2b_toml_path)
    }

    /// Build E2B template
    async fn build_template(
        &self,
        config: &SindriConfig,
        e2b_config: &E2bDeployConfig,
        _force: bool,
    ) -> Result<()> {
        info!("Building E2B template: {}", &e2b_config.template_alias);

        // Generate template files
        self.generate_e2b_dockerfile(config, e2b_config, &self.output_dir)
            .await?;
        self.generate_e2b_toml(config, e2b_config, &self.output_dir)?;

        let template_dir = self.output_dir.join("template");

        // Build template using E2B CLI
        info!("Building template with E2B (this may take 2-5 minutes)...");

        let cpus_str = e2b_config.cpus.to_string();
        let memory_str = e2b_config.memory_mb.to_string();

        let mut args = vec![
            "template",
            "build",
            "--name",
            &e2b_config.template_alias,
            "--dockerfile",
            "e2b.Dockerfile",
            "--cpu-count",
            &cpus_str,
            "--memory-mb",
            &memory_str,
        ];

        if let Some(ref team) = e2b_config.team {
            args.push("--team");
            args.push(team);
        }

        let status = Command::new("e2b")
            .args(&args)
            .current_dir(&template_dir)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await?;

        if !status.success() {
            return Err(anyhow!("E2B template build failed"));
        }

        info!("Template built: {}", &e2b_config.template_alias);
        Ok(())
    }

    /// Create sandbox from template
    async fn create_sandbox(
        &self,
        template_id: &str,
        e2b_config: &E2bDeployConfig,
        ephemeral: bool,
    ) -> Result<String> {
        info!("Creating sandbox from template: {}", template_id);

        let timeout_ms = e2b_config.timeout * 1000;
        let timeout_str = timeout_ms.to_string();

        // Build metadata JSON
        let mut metadata = e2b_config.metadata.clone();
        metadata.insert("sindri_name".to_string(), e2b_config.name.to_string());
        metadata.insert("sindri_profile".to_string(), e2b_config.profile.clone());

        let metadata_json = serde_json::to_string(&metadata)?;

        // Build create command arguments
        let mut args = vec![
            "sandbox",
            "create",
            template_id,
            "--timeout",
            &timeout_str,
            "--metadata",
            &metadata_json,
            "--json",
        ];

        let on_timeout;
        if e2b_config.auto_pause && !ephemeral {
            on_timeout = "pause".to_string();
            args.push("--on-timeout");
            args.push(&on_timeout);
        }

        if let Some(ref team) = e2b_config.team {
            args.push("--team");
            args.push(team);
        }

        let output = Command::new("e2b").args(&args).output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to create sandbox: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result: E2bCreateResponse =
            serde_json::from_str(&stdout).context("Failed to parse E2B sandbox create response")?;

        Ok(result.sandbox_id)
    }

    /// Resume a paused sandbox
    async fn resume_sandbox(&self, sandbox_id: &str) -> Result<()> {
        info!("Resuming sandbox: {}", sandbox_id);

        let output = Command::new("e2b")
            .args(["sandbox", "resume", sandbox_id])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to resume sandbox: {}", stderr));
        }

        info!("Sandbox resumed");
        Ok(())
    }

    /// Pause a running sandbox
    async fn pause_sandbox(&self, sandbox_id: &str) -> Result<()> {
        info!("Pausing sandbox: {}", sandbox_id);
        info!("Note: Pause takes ~4 seconds per 1 GiB of RAM");

        let output = Command::new("e2b")
            .args(["sandbox", "pause", sandbox_id])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to pause sandbox: {}", stderr));
        }

        info!("Sandbox paused");
        Ok(())
    }

    /// Connect via PTY (WebSocket terminal)
    async fn connect_pty(&self, sandbox_id: &str) -> Result<()> {
        info!("Connecting to sandbox: {}", sandbox_id);

        let status = Command::new("e2b")
            .args(["sandbox", "terminal", sandbox_id, "--shell", "/bin/bash"])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await?;

        if !status.success() {
            return Err(anyhow!("Failed to connect to sandbox"));
        }

        Ok(())
    }
}

impl Default for E2bProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for E2bProvider {
    fn name(&self) -> &'static str {
        "e2b"
    }

    fn check_prerequisites(&self) -> Result<PrerequisiteStatus> {
        let mut missing = Vec::new();
        let mut available = Vec::new();

        // Check e2b CLI
        if command_exists("e2b") {
            let version =
                get_command_version("e2b", "--version").unwrap_or_else(|_| "unknown".to_string());
            available.push(Prerequisite {
                name: "e2b".to_string(),
                description: "E2B CLI".to_string(),
                install_hint: None,
                version: Some(version),
            });
        } else {
            missing.push(Prerequisite {
                name: "e2b".to_string(),
                description: "E2B CLI".to_string(),
                install_hint: Some("Install: npm install -g @e2b/cli".to_string()),
                version: None,
            });
        }

        // Check API key
        if self.has_api_key() {
            available.push(Prerequisite {
                name: "e2b-api-key".to_string(),
                description: "E2B API key (E2B_API_KEY)".to_string(),
                install_hint: None,
                version: None,
            });
        } else {
            missing.push(Prerequisite {
                name: "e2b-api-key".to_string(),
                description: "E2B API key".to_string(),
                install_hint: Some(
                    "Set E2B_API_KEY environment variable. Get key at https://e2b.dev/dashboard"
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
        let e2b_config = self.get_e2b_config(config);
        let name = e2b_config.name.to_string();
        info!("Deploying {} to E2B", name);

        // E2B explicitly does not support GPU
        if let Some(gpu) = config.resources().gpu.as_ref() {
            if gpu.enabled {
                return Err(anyhow!(
                    "GPU is not supported on E2B provider\n\n\
                     E2B sandboxes do not support GPU workloads.\n\
                     For GPU support, use one of these providers:\n\
                     - fly: Fly.io with GPU machines\n\
                     - devpod: DevPod with cloud GPU providers\n\
                     - docker: Local Docker with nvidia-container-toolkit"
                ));
            }
        }

        // Check prerequisites
        let prereqs = self.check_prerequisites()?;
        if !prereqs.satisfied {
            let missing_names: Vec<_> = prereqs.missing.iter().map(|p| p.name.as_str()).collect();
            return Err(anyhow!(
                "Missing prerequisites: {}",
                missing_names.join(", ")
            ));
        }

        if opts.dry_run {
            return Ok(DeployResult {
                success: true,
                name: name.clone(),
                provider: "e2b".to_string(),
                instance_id: None,
                connection: None,
                messages: vec![format!(
                    "Would deploy {} as E2B sandbox with template {}",
                    name, &e2b_config.template_alias
                )],
                warnings: vec![],
            });
        }

        // Check for existing sandbox
        if let Some(existing_sandbox) = self.find_sandbox_by_name(&name).await {
            if let Some(sandbox_info) = self.get_sandbox_state(&existing_sandbox).await {
                match sandbox_info.status.as_str() {
                    "running" => {
                        return Ok(DeployResult {
                            success: true,
                            name: name.clone(),
                            provider: "e2b".to_string(),
                            instance_id: Some(existing_sandbox.clone()),
                            connection: Some(ConnectionInfo {
                                ssh_command: None,
                                http_url: None,
                                https_url: None,
                                instructions: Some(format!(
                                    "Sandbox '{}' is already running.\n\
                                     Connect with: sindri connect\n\
                                     Or: e2b sandbox terminal {}",
                                    name, existing_sandbox
                                )),
                            }),
                            messages: vec![format!("Sandbox '{}' already running", name)],
                            warnings: vec![],
                        });
                    }
                    "paused" => {
                        if e2b_config.auto_resume {
                            info!("Found paused sandbox, resuming...");
                            self.resume_sandbox(&existing_sandbox).await?;
                            return Ok(DeployResult {
                                success: true,
                                name: name.clone(),
                                provider: "e2b".to_string(),
                                instance_id: Some(existing_sandbox.clone()),
                                connection: Some(ConnectionInfo {
                                    ssh_command: None,
                                    http_url: None,
                                    https_url: None,
                                    instructions: Some(
                                        "Sandbox resumed.\nConnect with: sindri connect"
                                            .to_string(),
                                    ),
                                }),
                                messages: vec![format!("Sandbox '{}' resumed", name)],
                                warnings: vec![],
                            });
                        } else {
                            return Ok(DeployResult {
                                success: true,
                                name: name.clone(),
                                provider: "e2b".to_string(),
                                instance_id: Some(existing_sandbox.clone()),
                                connection: None,
                                messages: vec![],
                                warnings: vec![format!(
                                    "Sandbox '{}' is paused. Resume with: sindri connect (if autoResume is enabled) or e2b sandbox resume {}",
                                    name, existing_sandbox
                                )],
                            });
                        }
                    }
                    _ => {}
                }
            }
        }

        // Determine if we need to build template
        let need_build = e2b_config.build_on_deploy
            || opts.force
            || !self.template_exists(&e2b_config.template_alias).await
            || !e2b_config.reuse_template;

        // Build template if needed
        if need_build {
            self.build_template(config, &e2b_config, opts.force).await?;
        }

        // Get template ID
        let template_id = self
            .get_template_id(&e2b_config.template_alias)
            .await
            .unwrap_or_else(|| e2b_config.template_alias.to_string());

        // Create sandbox
        let sandbox_id = self
            .create_sandbox(&template_id, &e2b_config, false)
            .await?;

        Ok(DeployResult {
            success: true,
            name: name.clone(),
            provider: "e2b".to_string(),
            instance_id: Some(sandbox_id.clone()),
            connection: Some(ConnectionInfo {
                ssh_command: None,
                http_url: None,
                https_url: None,
                instructions: Some(format!(
                    "Connect:\n  sindri connect\n  e2b sandbox terminal {}\n\n\
                     Manage:\n  sindri status\n  sindri stop       # Pause sandbox (preserve state)\n  sindri destroy     # Kill sandbox",
                    sandbox_id
                )),
            }),
            messages: vec![format!("Sandbox '{}' deployed successfully", name)],
            warnings: vec![],
        })
    }

    async fn connect(&self, config: &SindriConfig) -> Result<()> {
        let e2b_config = self.get_e2b_config(config);
        let name = config.name();
        info!("Connecting to {} on E2B", name);

        // Check prerequisites
        let prereqs = self.check_prerequisites()?;
        if !prereqs.satisfied {
            let missing_names: Vec<_> = prereqs.missing.iter().map(|p| p.name.as_str()).collect();
            return Err(anyhow!(
                "Missing prerequisites: {}",
                missing_names.join(", ")
            ));
        }

        let sandbox_id = self.find_sandbox_by_name(name).await.ok_or_else(|| {
            anyhow!(
                "Sandbox '{}' not found. Deploy first: sindri deploy --provider e2b",
                name
            )
        })?;

        // Check state and resume if paused
        if let Some(sandbox_info) = self.get_sandbox_state(&sandbox_id).await {
            match sandbox_info.status.as_str() {
                "paused" => {
                    if e2b_config.auto_resume {
                        info!("Sandbox is paused, resuming...");
                        self.resume_sandbox(&sandbox_id).await?;
                    } else {
                        return Err(anyhow!(
                            "Sandbox is paused.\n\
                             Enable auto-resume in sindri.yaml or resume manually:\n  e2b sandbox resume {}",
                            sandbox_id
                        ));
                    }
                }
                "running" => {
                    // Already running, connect directly
                }
                state => {
                    return Err(anyhow!("Sandbox is in unexpected state: {}", state));
                }
            }
        }

        // Connect via PTY
        self.connect_pty(&sandbox_id).await
    }

    async fn status(&self, config: &SindriConfig) -> Result<DeploymentStatus> {
        let name = config.name().to_string();
        info!("Getting E2B status for {}", name);

        // Check prerequisites
        let prereqs = self.check_prerequisites()?;
        if !prereqs.satisfied {
            return Ok(DeploymentStatus {
                name,
                provider: "e2b".to_string(),
                state: DeploymentState::Unknown,
                instance_id: None,
                image: config.image().map(|s| s.to_string()),
                addresses: vec![],
                resources: None,
                timestamps: Default::default(),
                details: Default::default(),
            });
        }

        let sandbox_id = self.find_sandbox_by_name(&name).await;

        let (state, instance_id, details) = if let Some(id) = sandbox_id {
            if let Some(sandbox_info) = self.get_sandbox_state(&id).await {
                let state = match sandbox_info.status.as_str() {
                    "running" => DeploymentState::Running,
                    "paused" => DeploymentState::Paused,
                    "stopped" => DeploymentState::Stopped,
                    _ => DeploymentState::Unknown,
                };

                let mut details = HashMap::new();
                if let Some(template_id) = sandbox_info.template_id {
                    details.insert("template_id".to_string(), template_id);
                }
                if let Some(started_at) = sandbox_info.started_at {
                    details.insert("started_at".to_string(), started_at);
                }

                (state, Some(id), details)
            } else {
                (DeploymentState::Unknown, Some(id), HashMap::new())
            }
        } else {
            (DeploymentState::NotDeployed, None, HashMap::new())
        };

        Ok(DeploymentStatus {
            name,
            provider: "e2b".to_string(),
            state,
            instance_id,
            image: config.image().map(|s| s.to_string()),
            addresses: vec![],
            resources: None,
            timestamps: Default::default(),
            details,
        })
    }

    async fn destroy(&self, config: &SindriConfig, force: bool) -> Result<()> {
        let name = config.name();
        info!("Destroying {} on E2B (force: {})", name, force);

        // Check prerequisites
        let prereqs = self.check_prerequisites()?;
        if !prereqs.satisfied {
            let missing_names: Vec<_> = prereqs.missing.iter().map(|p| p.name.as_str()).collect();
            return Err(anyhow!(
                "Missing prerequisites: {}",
                missing_names.join(", ")
            ));
        }

        if let Some(sandbox_id) = self.find_sandbox_by_name(name).await {
            info!("Killing sandbox: {}", sandbox_id);
            let output = Command::new("e2b")
                .args(["sandbox", "kill", &sandbox_id])
                .output()
                .await?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow!("Failed to kill sandbox: {}", stderr));
            }

            info!("Sandbox destroyed");
        } else {
            warn!("Sandbox '{}' not found", name);
        }

        // Clean up local files
        if self.output_dir.exists() {
            std::fs::remove_dir_all(&self.output_dir)?;
        }

        Ok(())
    }

    async fn plan(&self, config: &SindriConfig) -> Result<DeploymentPlan> {
        let e2b_config = self.get_e2b_config(config);
        let name = e2b_config.name.to_string();
        info!("Planning E2B deployment for {}", name);

        let mut actions = vec![
            PlannedAction {
                action: ActionType::Create,
                resource: "e2b.Dockerfile".to_string(),
                description: "Generate E2B Dockerfile".to_string(),
            },
            PlannedAction {
                action: ActionType::Create,
                resource: "e2b.toml".to_string(),
                description: "Generate E2B template configuration".to_string(),
            },
        ];

        // Check prerequisites first
        let prereqs = self.check_prerequisites().ok();
        let has_prereqs = prereqs.as_ref().map(|p| p.satisfied).unwrap_or(false);

        if has_prereqs {
            if !self.template_exists(&e2b_config.template_alias).await {
                actions.push(PlannedAction {
                    action: ActionType::Create,
                    resource: format!("template:{}", &e2b_config.template_alias),
                    description: "Build E2B template (2-5 minutes)".to_string(),
                });
            } else {
                actions.push(PlannedAction {
                    action: ActionType::Update,
                    resource: format!("template:{}", &e2b_config.template_alias),
                    description: "Use existing E2B template".to_string(),
                });
            }
        }

        actions.push(PlannedAction {
            action: ActionType::Create,
            resource: format!("sandbox:{}", name),
            description: "Create E2B sandbox from template".to_string(),
        });

        if e2b_config.auto_pause {
            actions.push(PlannedAction {
                action: ActionType::Update,
                resource: format!("sandbox:{}", name),
                description: "Configure auto-pause on timeout".to_string(),
            });
        }

        let resources = vec![
            PlannedResource {
                resource_type: "template".to_string(),
                name: e2b_config.template_alias.to_string(),
                config: {
                    let mut m = HashMap::new();
                    m.insert("cpus".to_string(), serde_json::json!(e2b_config.cpus));
                    m.insert(
                        "memory_mb".to_string(),
                        serde_json::json!(e2b_config.memory_mb),
                    );
                    m.insert("profile".to_string(), serde_json::json!(e2b_config.profile));
                    m
                },
            },
            PlannedResource {
                resource_type: "sandbox".to_string(),
                name: name.clone(),
                config: {
                    let mut m = HashMap::new();
                    m.insert("timeout".to_string(), serde_json::json!(e2b_config.timeout));
                    m.insert(
                        "auto_pause".to_string(),
                        serde_json::json!(e2b_config.auto_pause),
                    );
                    m.insert(
                        "auto_resume".to_string(),
                        serde_json::json!(e2b_config.auto_resume),
                    );
                    m
                },
            },
        ];

        Ok(DeploymentPlan {
            provider: "e2b".to_string(),
            actions,
            resources,
            estimated_cost: None,
        })
    }

    async fn start(&self, config: &SindriConfig) -> Result<()> {
        let name = config.name();
        info!("Starting (resuming) {} on E2B", name);

        let sandbox_id = self
            .find_sandbox_by_name(name)
            .await
            .ok_or_else(|| anyhow!("Sandbox '{}' not found", name))?;

        self.resume_sandbox(&sandbox_id).await
    }

    async fn stop(&self, config: &SindriConfig) -> Result<()> {
        let name = config.name();
        info!("Stopping (pausing) {} on E2B", name);

        let sandbox_id = self
            .find_sandbox_by_name(name)
            .await
            .ok_or_else(|| anyhow!("Sandbox '{}' not found", name))?;

        self.pause_sandbox(&sandbox_id).await
    }

    fn supports_gpu(&self) -> bool {
        false // E2B explicitly does not support GPU
    }

    fn supports_auto_suspend(&self) -> bool {
        true // E2B supports pause/resume
    }
}

/// E2B deployment configuration
struct E2bDeployConfig {
    name: String,
    template_alias: String,
    profile: String,
    custom_extensions: String,
    additional_extensions: String,
    skip_auto_install: bool,
    cpus: u32,
    memory_mb: u32,
    timeout: u32,
    auto_pause: bool,
    auto_resume: bool,
    reuse_template: bool,
    build_on_deploy: bool,
    team: Option<String>,
    metadata: HashMap<String, String>,
}

/// E2B sandbox info from JSON API
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct E2bSandbox {
    sandbox_id: String,
    status: String,
    template_id: Option<String>,
    started_at: Option<String>,
    metadata: Option<HashMap<String, String>>,
}

/// E2B template info from JSON API
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct E2bTemplate {
    template_id: String,
    alias: Option<String>,
}

/// E2B create sandbox response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct E2bCreateResponse {
    sandbox_id: String,
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
    fn test_e2b_provider_creation() {
        let provider = E2bProvider::new();
        assert_eq!(provider.name(), "e2b");
    }

    #[test]
    fn test_supports_gpu() {
        let provider = E2bProvider::new();
        assert!(!provider.supports_gpu());
    }

    #[test]
    fn test_supports_auto_suspend() {
        let provider = E2bProvider::new();
        assert!(provider.supports_auto_suspend());
    }
}
