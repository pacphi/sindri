//! Azure Packer provider implementation
//!
//! Builds Azure managed images using the `azure-arm` Packer builder.

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use tokio::process::Command;
use tracing::info;

use crate::templates::TemplateRegistry;
use crate::traits::{
    BuildOptions, BuildResult, CloudPrerequisiteStatus, DeployFromImageResult, ImageInfo,
    ImageState, PackerProvider, ValidationResult,
};
use crate::utils;
use sindri_core::types::packer_config::PackerConfig;

/// Azure Packer provider for building managed images
pub struct AzurePackerProvider {
    templates: TemplateRegistry,
    output_dir: PathBuf,
}

impl AzurePackerProvider {
    /// Create a new Azure Packer provider
    pub fn new() -> Self {
        Self {
            templates: TemplateRegistry::new().expect("Failed to load templates"),
            output_dir: utils::default_output_dir().join("azure"),
        }
    }

    /// Check if Azure CLI is installed
    fn check_azure_cli(&self) -> Result<Option<String>> {
        utils::check_cli_installed("az", &["--version"])
    }

    /// Check if Azure credentials are configured
    fn check_azure_credentials(&self) -> bool {
        let result = std::process::Command::new("az")
            .args(["account", "show"])
            .output();

        match result {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }
}

impl Default for AzurePackerProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackerProvider for AzurePackerProvider {
    fn cloud_name(&self) -> &'static str {
        "azure"
    }

    async fn build_image(&self, config: &PackerConfig, opts: BuildOptions) -> Result<BuildResult> {
        info!("Building Azure managed image: {}", config.image_name);

        utils::ensure_dir(&self.output_dir)?;

        let template_content = self.generate_template(config)?;
        let template_path = self.output_dir.join("azure.pkr.hcl");
        utils::write_file(&template_path, &template_content)?;

        // Generate provisioning scripts
        let scripts = self.templates.render_scripts(config)?;
        let scripts_dir = self.output_dir.join("scripts");
        utils::ensure_dir(&scripts_dir)?;
        for (name, content) in scripts {
            utils::write_file(&scripts_dir.join(&name), &content)?;
        }

        let init_output = utils::packer_init(&template_path).await?;
        if !init_output.status.success() {
            return Err(anyhow!(
                "Packer init failed: {}",
                String::from_utf8_lossy(&init_output.stderr)
            ));
        }

        let validate_output = utils::packer_validate(&template_path, false).await?;
        if !validate_output.status.success() {
            return Err(anyhow!(
                "Packer validation failed: {}",
                String::from_utf8_lossy(&validate_output.stderr)
            ));
        }

        let mut cmd = Command::new("packer");
        cmd.arg("build");

        if opts.force {
            cmd.arg("-force");
        }

        if opts.debug {
            cmd.env("PACKER_LOG", "1");
        }

        cmd.arg(&template_path);

        let start = Instant::now();
        let output = cmd.output().await.context("Failed to run Packer build")?;
        let build_time = start.elapsed();

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let success = output.status.success();
        let image_id = if success {
            utils::parse_azure_image_id(&stdout).unwrap_or_else(|| "unknown".to_string())
        } else {
            String::new()
        };

        let location = config
            .azure
            .as_ref()
            .map(|a| a.location.clone())
            .unwrap_or_else(|| "westus2".to_string());

        Ok(BuildResult {
            success,
            image_id,
            image_name: config.image_name.clone(),
            provider: "azure".to_string(),
            region: location,
            build_time,
            artifact_size: None,
            manifest: None,
            logs: vec![stdout, stderr],
            metadata: HashMap::new(),
        })
    }

    async fn list_images(&self, config: &PackerConfig) -> Result<Vec<ImageInfo>> {
        let azure = config
            .azure
            .as_ref()
            .ok_or_else(|| anyhow!("Azure configuration required"))?;

        let output = Command::new("az")
            .args([
                "image",
                "list",
                "--resource-group",
                &azure.resource_group,
                "--query",
                &format!("[?starts_with(name, '{}')]", config.image_name),
                "--output",
                "json",
            ])
            .output()
            .await
            .context("Failed to list Azure images")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to list Azure images: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let images: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout)?;
        let result = images
            .into_iter()
            .map(|img| ImageInfo {
                id: img["id"].as_str().unwrap_or("").to_string(),
                name: img["name"].as_str().unwrap_or("").to_string(),
                description: None,
                state: match img["provisioningState"].as_str() {
                    Some("Succeeded") => ImageState::Available,
                    Some("Creating") => ImageState::Pending,
                    Some("Failed") => ImageState::Failed,
                    _ => ImageState::Unknown,
                },
                created_at: None,
                size: None,
                sindri_version: None,
                extensions: Vec::new(),
                profile: None,
                tags: HashMap::new(),
                metadata: HashMap::new(),
            })
            .collect();

        Ok(result)
    }

    async fn delete_image(&self, _config: &PackerConfig, image_id: &str) -> Result<()> {
        info!("Deleting Azure image: {}", image_id);

        let output = Command::new("az")
            .args(["image", "delete", "--ids", image_id])
            .output()
            .await
            .context("Failed to delete Azure image")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to delete Azure image: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        info!("Deleted Azure image: {}", image_id);
        Ok(())
    }

    async fn get_image(&self, _config: &PackerConfig, image_id: &str) -> Result<ImageInfo> {
        let output = Command::new("az")
            .args(["image", "show", "--ids", image_id, "--output", "json"])
            .output()
            .await
            .context("Failed to get Azure image")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to get Azure image: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let img: serde_json::Value = serde_json::from_slice(&output.stdout)?;

        let mut tags = HashMap::new();
        if let Some(tag_obj) = img["tags"].as_object() {
            for (key, value) in tag_obj {
                if let Some(v) = value.as_str() {
                    tags.insert(key.clone(), v.to_string());
                }
            }
        }

        Ok(ImageInfo {
            id: img["id"].as_str().unwrap_or("").to_string(),
            name: img["name"].as_str().unwrap_or("").to_string(),
            description: None,
            state: match img["provisioningState"].as_str() {
                Some("Succeeded") => ImageState::Available,
                Some("Creating") => ImageState::Pending,
                Some("Failed") => ImageState::Failed,
                _ => ImageState::Unknown,
            },
            created_at: None,
            size: None,
            sindri_version: tags.get("SindriVersion").cloned(),
            extensions: Vec::new(),
            profile: None,
            tags,
            metadata: HashMap::new(),
        })
    }

    async fn validate_template(&self, config: &PackerConfig) -> Result<ValidationResult> {
        let template_content = self.generate_template(config)?;

        utils::ensure_dir(&self.output_dir)?;
        let template_path = self.output_dir.join("azure.pkr.hcl");
        utils::write_file(&template_path, &template_content)?;

        let output = utils::packer_validate(&template_path, true).await?;

        Ok(ValidationResult {
            valid: output.status.success(),
            errors: if output.status.success() {
                vec![]
            } else {
                vec![String::from_utf8_lossy(&output.stderr).to_string()]
            },
            warnings: vec![],
            template_content: Some(template_content),
        })
    }

    fn check_cloud_prerequisites(&self) -> Result<CloudPrerequisiteStatus> {
        let packer_version = utils::check_packer_installed()?;
        let cli_version = self.check_azure_cli()?;
        let credentials_configured = self.check_azure_credentials();

        let mut missing = Vec::new();
        let mut hints = Vec::new();

        if packer_version.is_none() {
            missing.push("packer".to_string());
            hints
                .push("Install Packer: https://developer.hashicorp.com/packer/install".to_string());
        }

        if cli_version.is_none() {
            missing.push("azure-cli".to_string());
            hints.push(
                "Install Azure CLI: https://learn.microsoft.com/en-us/cli/azure/install-azure-cli"
                    .to_string(),
            );
        }

        if !credentials_configured {
            missing.push("azure-credentials".to_string());
            hints.push("Login to Azure: az login".to_string());
        }

        let satisfied = missing.is_empty();

        Ok(CloudPrerequisiteStatus {
            packer_installed: packer_version.is_some(),
            packer_version,
            cli_installed: cli_version.is_some(),
            cli_version,
            credentials_configured,
            missing,
            hints,
            satisfied,
        })
    }

    async fn find_cached_image(&self, config: &PackerConfig) -> Result<Option<String>> {
        let images = self.list_images(config).await?;
        let config_hash = utils::config_hash(&config.build);

        for image in images {
            if image.state == ImageState::Available {
                if let Some(hash) = image.tags.get("ConfigHash") {
                    if hash == &config_hash {
                        return Ok(Some(image.id));
                    }
                }
            }
        }

        Ok(None)
    }

    async fn deploy_from_image(
        &self,
        image_id: &str,
        config: &PackerConfig,
    ) -> Result<DeployFromImageResult> {
        let azure = config
            .azure
            .as_ref()
            .ok_or_else(|| anyhow!("Azure configuration required"))?;

        info!("Creating Azure VM from image: {}", image_id);

        let vm_name = format!("{}-{}", config.image_name, utils::generate_build_id());

        let output = Command::new("az")
            .args([
                "vm",
                "create",
                "--resource-group",
                &azure.resource_group,
                "--name",
                &vm_name,
                "--image",
                image_id,
                "--size",
                &azure.vm_size,
                "--admin-username",
                "sindri",
                "--generate-ssh-keys",
                "--output",
                "json",
            ])
            .output()
            .await
            .context("Failed to create Azure VM")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to create Azure VM: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let response: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        let public_ip = response["publicIpAddress"].as_str().map(|s| s.to_string());
        let private_ip = response["privateIpAddress"].as_str().map(|s| s.to_string());

        let ssh_command = public_ip.as_ref().map(|ip| format!("ssh sindri@{}", ip));

        Ok(DeployFromImageResult {
            success: true,
            instance_id: vm_name,
            public_ip,
            private_ip,
            ssh_command,
            messages: vec!["Azure VM created successfully".to_string()],
        })
    }

    fn generate_template(&self, config: &PackerConfig) -> Result<String> {
        let context = self.templates.create_azure_context(config)?;
        self.templates.render("azure.pkr.hcl.tera", &context)
    }
}
