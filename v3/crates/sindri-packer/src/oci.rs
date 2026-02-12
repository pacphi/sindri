//! OCI (Oracle Cloud Infrastructure) Packer provider implementation
//!
//! Builds Oracle Cloud custom images using the `oracle-oci` Packer builder.

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

/// OCI Packer provider for building Oracle Cloud custom images
pub struct OciPackerProvider {
    templates: TemplateRegistry,
    output_dir: PathBuf,
}

impl OciPackerProvider {
    /// Create a new OCI Packer provider
    pub fn new() -> Result<Self> {
        Ok(Self {
            templates: TemplateRegistry::new().context("Failed to load templates")?,
            output_dir: utils::default_output_dir().join("oci"),
        })
    }

    /// Check if OCI CLI is installed
    fn check_oci_cli(&self) -> Result<Option<String>> {
        utils::check_cli_installed("oci", &["--version"])
    }

    /// Check if OCI credentials are configured
    fn check_oci_credentials(&self) -> bool {
        let result = std::process::Command::new("oci")
            .args(["iam", "region", "list"])
            .output();

        match result {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }
}

impl Default for OciPackerProvider {
    fn default() -> Self {
        Self::new().expect("Failed to create default OciPackerProvider")
    }
}

#[async_trait]
impl PackerProvider for OciPackerProvider {
    fn cloud_name(&self) -> &'static str {
        "oci"
    }

    async fn build_image(&self, config: &PackerConfig, opts: BuildOptions) -> Result<BuildResult> {
        info!("Building OCI custom image: {}", config.image_name);

        utils::ensure_dir(&self.output_dir)?;

        let template_content = self.generate_template(config)?;
        let template_path = self.output_dir.join("oci.pkr.hcl");
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
            utils::parse_oci_image_id(&stdout).unwrap_or_else(|| "unknown".to_string())
        } else {
            String::new()
        };

        let ad = config
            .oci
            .as_ref()
            .map(|o| o.availability_domain.clone())
            .unwrap_or_default();

        Ok(BuildResult {
            success,
            image_id,
            image_name: config.image_name.clone(),
            provider: "oci".to_string(),
            region: ad,
            build_time,
            artifact_size: None,
            manifest: None,
            logs: vec![stdout, stderr],
            metadata: HashMap::new(),
        })
    }

    async fn list_images(&self, config: &PackerConfig) -> Result<Vec<ImageInfo>> {
        let oci = config
            .oci
            .as_ref()
            .ok_or_else(|| anyhow!("OCI configuration required"))?;

        let output = Command::new("oci")
            .args([
                "compute",
                "image",
                "list",
                "--compartment-id",
                &oci.compartment_ocid,
                "--display-name",
                &config.image_name,
                "--output",
                "json",
            ])
            .output()
            .await
            .context("Failed to list OCI images")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to list OCI images: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let response: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        let images = response["data"].as_array().cloned().unwrap_or_default();

        let result = images
            .into_iter()
            .map(|img| {
                let mut freeform_tags = HashMap::new();
                if let Some(tag_obj) = img["freeform-tags"].as_object() {
                    for (key, value) in tag_obj {
                        if let Some(v) = value.as_str() {
                            freeform_tags.insert(key.clone(), v.to_string());
                        }
                    }
                }

                ImageInfo {
                    id: img["id"].as_str().unwrap_or("").to_string(),
                    name: img["display-name"].as_str().unwrap_or("").to_string(),
                    description: None,
                    state: match img["lifecycle-state"].as_str() {
                        Some("AVAILABLE") => ImageState::Available,
                        Some("PROVISIONING") | Some("IMPORTING") => ImageState::Pending,
                        Some("DISABLED") => ImageState::Deregistered,
                        _ => ImageState::Unknown,
                    },
                    created_at: img["time-created"]
                        .as_str()
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                        .map(|dt| dt.with_timezone(&chrono::Utc)),
                    size: img["size-in-mbs"].as_u64(),
                    sindri_version: freeform_tags.get("SindriVersion").cloned(),
                    extensions: Vec::new(),
                    profile: None,
                    tags: freeform_tags,
                    metadata: HashMap::new(),
                }
            })
            .collect();

        Ok(result)
    }

    async fn delete_image(&self, _config: &PackerConfig, image_id: &str) -> Result<()> {
        info!("Deleting OCI image: {}", image_id);

        let output = Command::new("oci")
            .args([
                "compute",
                "image",
                "delete",
                "--image-id",
                image_id,
                "--force",
            ])
            .output()
            .await
            .context("Failed to delete OCI image")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to delete OCI image: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        info!("Deleted OCI image: {}", image_id);
        Ok(())
    }

    async fn get_image(&self, _config: &PackerConfig, image_id: &str) -> Result<ImageInfo> {
        let output = Command::new("oci")
            .args([
                "compute",
                "image",
                "get",
                "--image-id",
                image_id,
                "--output",
                "json",
            ])
            .output()
            .await
            .context("Failed to get OCI image")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to get OCI image: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let response: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        let img = &response["data"];

        let mut freeform_tags = HashMap::new();
        if let Some(tag_obj) = img["freeform-tags"].as_object() {
            for (key, value) in tag_obj {
                if let Some(v) = value.as_str() {
                    freeform_tags.insert(key.clone(), v.to_string());
                }
            }
        }

        Ok(ImageInfo {
            id: img["id"].as_str().unwrap_or("").to_string(),
            name: img["display-name"].as_str().unwrap_or("").to_string(),
            description: None,
            state: match img["lifecycle-state"].as_str() {
                Some("AVAILABLE") => ImageState::Available,
                Some("PROVISIONING") | Some("IMPORTING") => ImageState::Pending,
                Some("DISABLED") => ImageState::Deregistered,
                _ => ImageState::Unknown,
            },
            created_at: img["time-created"]
                .as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc)),
            size: img["size-in-mbs"].as_u64(),
            sindri_version: freeform_tags.get("SindriVersion").cloned(),
            extensions: Vec::new(),
            profile: None,
            tags: freeform_tags,
            metadata: HashMap::new(),
        })
    }

    async fn validate_template(&self, config: &PackerConfig) -> Result<ValidationResult> {
        let template_content = self.generate_template(config)?;

        utils::ensure_dir(&self.output_dir)?;
        let template_path = self.output_dir.join("oci.pkr.hcl");
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
        let cli_version = self.check_oci_cli()?;
        let credentials_configured = self.check_oci_credentials();

        let mut missing = Vec::new();
        let mut hints = Vec::new();

        if packer_version.is_none() {
            missing.push("packer".to_string());
            hints
                .push("Install Packer: https://developer.hashicorp.com/packer/install".to_string());
        }

        if cli_version.is_none() {
            missing.push("oci-cli".to_string());
            hints.push("Install OCI CLI: https://docs.oracle.com/en-us/iaas/Content/API/SDKDocs/cliinstall.htm".to_string());
        }

        if !credentials_configured {
            missing.push("oci-credentials".to_string());
            hints.push("Configure OCI CLI: oci setup config".to_string());
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
        let oci = config
            .oci
            .as_ref()
            .ok_or_else(|| anyhow!("OCI configuration required"))?;

        info!("Launching OCI compute instance from image: {}", image_id);

        let instance_name = format!("{}-{}", config.image_name, utils::generate_build_id());

        let mut args = vec![
            "compute",
            "instance",
            "launch",
            "--compartment-id",
            &oci.compartment_ocid,
            "--availability-domain",
            &oci.availability_domain,
            "--shape",
            &oci.shape,
            "--subnet-id",
            &oci.subnet_ocid,
            "--image-id",
            image_id,
            "--display-name",
            &instance_name,
            "--output",
            "json",
        ];

        // Add shape config for flexible shapes
        let shape_config_str: String;
        if let Some(shape_config) = &oci.shape_config {
            shape_config_str = serde_json::json!({
                "ocpus": shape_config.ocpus,
                "memoryInGBs": shape_config.memory_in_gbs
            })
            .to_string();
            args.push("--shape-config");
            args.push(&shape_config_str);
        }

        let output = Command::new("oci")
            .args(&args)
            .output()
            .await
            .context("Failed to launch OCI instance")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to launch OCI instance: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let response: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        let instance_id = response["data"]["id"].as_str().unwrap_or("").to_string();

        // OCI doesn't immediately provide IP addresses
        Ok(DeployFromImageResult {
            success: true,
            instance_id,
            public_ip: None,
            private_ip: None,
            ssh_command: None,
            messages: vec![
                "OCI instance launched successfully".to_string(),
                "Use 'oci compute instance list-vnics' to get IP addresses".to_string(),
            ],
        })
    }

    fn generate_template(&self, config: &PackerConfig) -> Result<String> {
        let context = self.templates.create_oci_context(config)?;
        self.templates.render("oci.pkr.hcl.tera", &context)
    }
}
