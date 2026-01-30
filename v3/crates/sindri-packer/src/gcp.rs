//! GCP Packer provider implementation
//!
//! Builds Google Compute Engine images using the `googlecompute` Packer builder.

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

/// GCP Packer provider for building Compute Engine images
pub struct GcpPackerProvider {
    templates: TemplateRegistry,
    output_dir: PathBuf,
}

impl GcpPackerProvider {
    /// Create a new GCP Packer provider
    pub fn new() -> Self {
        Self {
            templates: TemplateRegistry::new().expect("Failed to load templates"),
            output_dir: utils::default_output_dir().join("gcp"),
        }
    }

    /// Check if gcloud CLI is installed
    fn check_gcloud_cli(&self) -> Result<Option<String>> {
        utils::check_cli_installed("gcloud", &["--version"])
    }

    /// Check if GCP credentials are configured
    fn check_gcp_credentials(&self) -> bool {
        let result = std::process::Command::new("gcloud")
            .args(["auth", "print-access-token"])
            .output();

        match result {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }
}

impl Default for GcpPackerProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackerProvider for GcpPackerProvider {
    fn cloud_name(&self) -> &'static str {
        "gcp"
    }

    async fn build_image(&self, config: &PackerConfig, opts: BuildOptions) -> Result<BuildResult> {
        info!("Building GCP Compute Engine image: {}", config.image_name);

        utils::ensure_dir(&self.output_dir)?;

        let template_content = self.generate_template(config)?;
        let template_path = self.output_dir.join("gcp.pkr.hcl");
        utils::write_file(&template_path, &template_content)?;

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
            utils::parse_gcp_image(&stdout).unwrap_or_else(|| "unknown".to_string())
        } else {
            String::new()
        };

        let zone = config
            .gcp
            .as_ref()
            .map(|g| g.zone.clone())
            .unwrap_or_else(|| "us-west1-a".to_string());

        Ok(BuildResult {
            success,
            image_id,
            image_name: config.image_name.clone(),
            provider: "gcp".to_string(),
            region: zone,
            build_time,
            artifact_size: None,
            manifest: None,
            logs: vec![stdout, stderr],
            metadata: HashMap::new(),
        })
    }

    async fn list_images(&self, config: &PackerConfig) -> Result<Vec<ImageInfo>> {
        let gcp = config
            .gcp
            .as_ref()
            .ok_or_else(|| anyhow!("GCP configuration required"))?;

        let output = Command::new("gcloud")
            .args([
                "compute",
                "images",
                "list",
                "--project",
                &gcp.project_id,
                "--filter",
                &format!("name~^{}", config.image_name),
                "--format",
                "json",
            ])
            .output()
            .await
            .context("Failed to list GCP images")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to list GCP images: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let images: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout)?;
        let result = images
            .into_iter()
            .map(|img| {
                let mut labels = HashMap::new();
                if let Some(label_obj) = img["labels"].as_object() {
                    for (key, value) in label_obj {
                        if let Some(v) = value.as_str() {
                            labels.insert(key.clone(), v.to_string());
                        }
                    }
                }

                ImageInfo {
                    id: img["selfLink"].as_str().unwrap_or("").to_string(),
                    name: img["name"].as_str().unwrap_or("").to_string(),
                    description: img["description"].as_str().map(|s| s.to_string()),
                    state: match img["status"].as_str() {
                        Some("READY") => ImageState::Available,
                        Some("PENDING") => ImageState::Pending,
                        Some("FAILED") => ImageState::Failed,
                        _ => ImageState::Unknown,
                    },
                    created_at: img["creationTimestamp"]
                        .as_str()
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                        .map(|dt| dt.with_timezone(&chrono::Utc)),
                    size: img["diskSizeGb"]
                        .as_str()
                        .and_then(|s| s.parse::<u64>().ok()),
                    sindri_version: labels.get("sindri-version").cloned(),
                    extensions: Vec::new(),
                    profile: None,
                    tags: labels,
                    metadata: HashMap::new(),
                }
            })
            .collect();

        Ok(result)
    }

    async fn delete_image(&self, config: &PackerConfig, image_id: &str) -> Result<()> {
        let gcp = config
            .gcp
            .as_ref()
            .ok_or_else(|| anyhow!("GCP configuration required"))?;

        info!("Deleting GCP image: {}", image_id);

        // Extract image name from selfLink if needed
        let image_name = if image_id.contains('/') {
            image_id.rsplit('/').next().unwrap_or(image_id)
        } else {
            image_id
        };

        let output = Command::new("gcloud")
            .args([
                "compute",
                "images",
                "delete",
                image_name,
                "--project",
                &gcp.project_id,
                "--quiet",
            ])
            .output()
            .await
            .context("Failed to delete GCP image")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to delete GCP image: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        info!("Deleted GCP image: {}", image_id);
        Ok(())
    }

    async fn get_image(&self, config: &PackerConfig, image_id: &str) -> Result<ImageInfo> {
        let gcp = config
            .gcp
            .as_ref()
            .ok_or_else(|| anyhow!("GCP configuration required"))?;

        let image_name = if image_id.contains('/') {
            image_id.rsplit('/').next().unwrap_or(image_id)
        } else {
            image_id
        };

        let output = Command::new("gcloud")
            .args([
                "compute",
                "images",
                "describe",
                image_name,
                "--project",
                &gcp.project_id,
                "--format",
                "json",
            ])
            .output()
            .await
            .context("Failed to describe GCP image")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to describe GCP image: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let img: serde_json::Value = serde_json::from_slice(&output.stdout)?;

        let mut labels = HashMap::new();
        if let Some(label_obj) = img["labels"].as_object() {
            for (key, value) in label_obj {
                if let Some(v) = value.as_str() {
                    labels.insert(key.clone(), v.to_string());
                }
            }
        }

        Ok(ImageInfo {
            id: img["selfLink"].as_str().unwrap_or("").to_string(),
            name: img["name"].as_str().unwrap_or("").to_string(),
            description: img["description"].as_str().map(|s| s.to_string()),
            state: match img["status"].as_str() {
                Some("READY") => ImageState::Available,
                Some("PENDING") => ImageState::Pending,
                Some("FAILED") => ImageState::Failed,
                _ => ImageState::Unknown,
            },
            created_at: img["creationTimestamp"]
                .as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc)),
            size: img["diskSizeGb"]
                .as_str()
                .and_then(|s| s.parse::<u64>().ok()),
            sindri_version: labels.get("sindri-version").cloned(),
            extensions: Vec::new(),
            profile: None,
            tags: labels,
            metadata: HashMap::new(),
        })
    }

    async fn validate_template(&self, config: &PackerConfig) -> Result<ValidationResult> {
        let template_content = self.generate_template(config)?;

        utils::ensure_dir(&self.output_dir)?;
        let template_path = self.output_dir.join("gcp.pkr.hcl");
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
        let cli_version = self.check_gcloud_cli()?;
        let credentials_configured = self.check_gcp_credentials();

        let mut missing = Vec::new();
        let mut hints = Vec::new();

        if packer_version.is_none() {
            missing.push("packer".to_string());
            hints
                .push("Install Packer: https://developer.hashicorp.com/packer/install".to_string());
        }

        if cli_version.is_none() {
            missing.push("gcloud-cli".to_string());
            hints.push("Install gcloud: https://cloud.google.com/sdk/docs/install".to_string());
        }

        if !credentials_configured {
            missing.push("gcp-credentials".to_string());
            hints.push("Login to GCP: gcloud auth login".to_string());
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
                if let Some(hash) = image.tags.get("config-hash") {
                    if hash == &config_hash {
                        return Ok(Some(image.name));
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
        let gcp = config
            .gcp
            .as_ref()
            .ok_or_else(|| anyhow!("GCP configuration required"))?;

        info!("Creating GCP VM from image: {}", image_id);

        let vm_name = format!(
            "{}-{}",
            utils::sanitize_name(&config.image_name),
            utils::generate_build_id()
        );

        let image_name = if image_id.contains('/') {
            image_id.rsplit('/').next().unwrap_or(image_id)
        } else {
            image_id
        };

        let output = Command::new("gcloud")
            .args([
                "compute",
                "instances",
                "create",
                &vm_name,
                "--project",
                &gcp.project_id,
                "--zone",
                &gcp.zone,
                "--machine-type",
                &gcp.machine_type,
                "--image",
                image_name,
                "--image-project",
                &gcp.project_id,
                "--boot-disk-size",
                &format!("{}GB", gcp.disk_size),
                "--boot-disk-type",
                &gcp.disk_type,
                "--format",
                "json",
            ])
            .output()
            .await
            .context("Failed to create GCP VM")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to create GCP VM: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let response: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout)?;

        let public_ip = response
            .first()
            .and_then(|v| v["networkInterfaces"][0]["accessConfigs"][0]["natIP"].as_str())
            .map(|s| s.to_string());

        let private_ip = response
            .first()
            .and_then(|v| v["networkInterfaces"][0]["networkIP"].as_str())
            .map(|s| s.to_string());

        let ssh_command = Some(format!(
            "gcloud compute ssh {} --project {} --zone {}",
            vm_name, gcp.project_id, gcp.zone
        ));

        Ok(DeployFromImageResult {
            success: true,
            instance_id: vm_name,
            public_ip,
            private_ip,
            ssh_command,
            messages: vec!["GCP VM created successfully".to_string()],
        })
    }

    fn generate_template(&self, config: &PackerConfig) -> Result<String> {
        let context = self.templates.create_gcp_context(config)?;
        self.templates.render("gcp.pkr.hcl.tera", &context)
    }
}
