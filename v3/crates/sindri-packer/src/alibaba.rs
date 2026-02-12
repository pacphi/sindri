//! Alibaba Cloud Packer provider implementation
//!
//! Builds Alibaba Cloud ECS custom images using the `alicloud-ecs` Packer builder.

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

/// Alibaba Cloud Packer provider for building ECS custom images
pub struct AlibabaPackerProvider {
    templates: TemplateRegistry,
    output_dir: PathBuf,
}

impl AlibabaPackerProvider {
    /// Create a new Alibaba Cloud Packer provider
    pub fn new() -> Result<Self> {
        Ok(Self {
            templates: TemplateRegistry::new().context("Failed to load templates")?,
            output_dir: utils::default_output_dir().join("alibaba"),
        })
    }

    /// Check if Alibaba Cloud CLI (aliyun) is installed
    fn check_aliyun_cli(&self) -> Result<Option<String>> {
        utils::check_cli_installed("aliyun", &["--version"])
    }

    /// Check if Alibaba Cloud credentials are configured
    fn check_alibaba_credentials(&self) -> bool {
        // Check for environment variables or CLI config
        std::env::var("ALICLOUD_ACCESS_KEY").is_ok()
            || std::env::var("ALIBABA_CLOUD_ACCESS_KEY_ID").is_ok()
            || {
                let result = std::process::Command::new("aliyun")
                    .args(["configure", "get"])
                    .output();
                match result {
                    Ok(output) => output.status.success(),
                    Err(_) => false,
                }
            }
    }
}

#[async_trait]
impl PackerProvider for AlibabaPackerProvider {
    fn cloud_name(&self) -> &'static str {
        "alibaba"
    }

    async fn build_image(&self, config: &PackerConfig, opts: BuildOptions) -> Result<BuildResult> {
        info!("Building Alibaba Cloud ECS image: {}", config.image_name);

        utils::ensure_dir(&self.output_dir)?;

        let template_content = self.generate_template(config)?;
        let template_path = self.output_dir.join("alibaba.pkr.hcl");
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
            utils::parse_alicloud_image_id(&stdout).unwrap_or_else(|| "unknown".to_string())
        } else {
            String::new()
        };

        let region = config
            .alibaba
            .as_ref()
            .map(|a| a.region.clone())
            .unwrap_or_else(|| "cn-hangzhou".to_string());

        Ok(BuildResult {
            success,
            image_id,
            image_name: config.image_name.clone(),
            provider: "alibaba".to_string(),
            region,
            build_time,
            artifact_size: None,
            manifest: None,
            logs: vec![stdout, stderr],
            metadata: HashMap::new(),
        })
    }

    async fn list_images(&self, config: &PackerConfig) -> Result<Vec<ImageInfo>> {
        let alibaba = config
            .alibaba
            .as_ref()
            .ok_or_else(|| anyhow!("Alibaba configuration required"))?;

        let output = Command::new("aliyun")
            .args([
                "ecs",
                "DescribeImages",
                "--RegionId",
                &alibaba.region,
                "--ImageOwnerAlias",
                "self",
                "--ImageName",
                &format!("{}*", config.image_name),
                "--output",
                "json",
            ])
            .output()
            .await
            .context("Failed to list Alibaba images")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to list Alibaba images: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let response: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        let images = response["Images"]["Image"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        let result = images
            .into_iter()
            .map(|img| {
                let mut tags = HashMap::new();
                if let Some(tag_arr) = img["Tags"]["Tag"].as_array() {
                    for tag in tag_arr {
                        if let (Some(key), Some(value)) =
                            (tag["TagKey"].as_str(), tag["TagValue"].as_str())
                        {
                            tags.insert(key.to_string(), value.to_string());
                        }
                    }
                }

                ImageInfo {
                    id: img["ImageId"].as_str().unwrap_or("").to_string(),
                    name: img["ImageName"].as_str().unwrap_or("").to_string(),
                    description: img["Description"].as_str().map(|s| s.to_string()),
                    state: match img["Status"].as_str() {
                        Some("Available") => ImageState::Available,
                        Some("Creating") | Some("Waiting") => ImageState::Pending,
                        Some("CreateFailed") => ImageState::Failed,
                        Some("Deprecated") => ImageState::Deregistered,
                        _ => ImageState::Unknown,
                    },
                    created_at: img["CreationTime"]
                        .as_str()
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                        .map(|dt| dt.with_timezone(&chrono::Utc)),
                    size: img["Size"].as_u64(),
                    sindri_version: tags.get("SindriVersion").cloned(),
                    extensions: Vec::new(),
                    profile: None,
                    tags,
                    metadata: HashMap::new(),
                }
            })
            .collect();

        Ok(result)
    }

    async fn delete_image(&self, config: &PackerConfig, image_id: &str) -> Result<()> {
        let alibaba = config
            .alibaba
            .as_ref()
            .ok_or_else(|| anyhow!("Alibaba configuration required"))?;

        info!("Deleting Alibaba image: {}", image_id);

        let output = Command::new("aliyun")
            .args([
                "ecs",
                "DeleteImage",
                "--RegionId",
                &alibaba.region,
                "--ImageId",
                image_id,
                "--Force",
                "true",
            ])
            .output()
            .await
            .context("Failed to delete Alibaba image")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to delete Alibaba image: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        info!("Deleted Alibaba image: {}", image_id);
        Ok(())
    }

    async fn get_image(&self, config: &PackerConfig, image_id: &str) -> Result<ImageInfo> {
        let alibaba = config
            .alibaba
            .as_ref()
            .ok_or_else(|| anyhow!("Alibaba configuration required"))?;

        let output = Command::new("aliyun")
            .args([
                "ecs",
                "DescribeImages",
                "--RegionId",
                &alibaba.region,
                "--ImageId",
                image_id,
                "--output",
                "json",
            ])
            .output()
            .await
            .context("Failed to get Alibaba image")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to get Alibaba image: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let response: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        let images = response["Images"]["Image"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        if images.is_empty() {
            return Err(anyhow!("Image not found: {}", image_id));
        }

        let img = &images[0];

        let mut tags = HashMap::new();
        if let Some(tag_arr) = img["Tags"]["Tag"].as_array() {
            for tag in tag_arr {
                if let (Some(key), Some(value)) = (tag["TagKey"].as_str(), tag["TagValue"].as_str())
                {
                    tags.insert(key.to_string(), value.to_string());
                }
            }
        }

        Ok(ImageInfo {
            id: img["ImageId"].as_str().unwrap_or("").to_string(),
            name: img["ImageName"].as_str().unwrap_or("").to_string(),
            description: img["Description"].as_str().map(|s| s.to_string()),
            state: match img["Status"].as_str() {
                Some("Available") => ImageState::Available,
                Some("Creating") | Some("Waiting") => ImageState::Pending,
                Some("CreateFailed") => ImageState::Failed,
                Some("Deprecated") => ImageState::Deregistered,
                _ => ImageState::Unknown,
            },
            created_at: img["CreationTime"]
                .as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc)),
            size: img["Size"].as_u64(),
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
        let template_path = self.output_dir.join("alibaba.pkr.hcl");
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
        let cli_version = self.check_aliyun_cli()?;
        let credentials_configured = self.check_alibaba_credentials();

        let mut missing = Vec::new();
        let mut hints = Vec::new();

        if packer_version.is_none() {
            missing.push("packer".to_string());
            hints
                .push("Install Packer: https://developer.hashicorp.com/packer/install".to_string());
        }

        if cli_version.is_none() {
            missing.push("aliyun-cli".to_string());
            hints.push(
                "Install Aliyun CLI: https://www.alibabacloud.com/help/en/cli/install-cli"
                    .to_string(),
            );
        }

        if !credentials_configured {
            missing.push("alibaba-credentials".to_string());
            hints.push("Configure credentials: aliyun configure".to_string());
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
        let alibaba = config
            .alibaba
            .as_ref()
            .ok_or_else(|| anyhow!("Alibaba configuration required"))?;

        info!("Creating Alibaba ECS instance from image: {}", image_id);

        let instance_name = format!("{}-{}", config.image_name, utils::generate_build_id());
        let disk_size = alibaba.system_disk_size_gb.to_string();

        let mut args = vec![
            "ecs",
            "RunInstances",
            "--RegionId",
            &alibaba.region,
            "--ImageId",
            image_id,
            "--InstanceType",
            &alibaba.instance_type,
            "--InstanceName",
            &instance_name,
            "--SystemDisk.Size",
            disk_size.as_str(),
            "--SystemDisk.Category",
            &alibaba.system_disk_category,
            "--InternetMaxBandwidthOut",
            "5",
            "--output",
            "json",
        ];

        if let Some(vswitch_id) = &alibaba.vswitch_id {
            args.push("--VSwitchId");
            args.push(vswitch_id);
        }

        let output = Command::new("aliyun")
            .args(&args)
            .output()
            .await
            .context("Failed to create Alibaba ECS instance")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to create Alibaba ECS instance: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let response: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        let instance_id = response["InstanceIdSets"]["InstanceIdSet"][0]
            .as_str()
            .unwrap_or("")
            .to_string();

        // Wait a moment and try to get IPs
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;

        let describe_output = Command::new("aliyun")
            .args([
                "ecs",
                "DescribeInstances",
                "--RegionId",
                &alibaba.region,
                "--InstanceIds",
                &format!("[\"{}\"]", instance_id),
                "--output",
                "json",
            ])
            .output()
            .await?;

        let mut public_ip = None;
        let mut private_ip = None;

        if describe_output.status.success() {
            let desc_response: serde_json::Value = serde_json::from_slice(&describe_output.stdout)?;
            if let Some(instances) = desc_response["Instances"]["Instance"].as_array() {
                if let Some(instance) = instances.first() {
                    public_ip = instance["PublicIpAddress"]["IpAddress"][0]
                        .as_str()
                        .map(|s| s.to_string());
                    private_ip = instance["VpcAttributes"]["PrivateIpAddress"]["IpAddress"][0]
                        .as_str()
                        .map(|s| s.to_string());
                }
            }
        }

        let ssh_command = public_ip.as_ref().map(|ip| format!("ssh root@{}", ip));

        Ok(DeployFromImageResult {
            success: true,
            instance_id,
            public_ip,
            private_ip,
            ssh_command,
            messages: vec!["Alibaba ECS instance created successfully".to_string()],
        })
    }

    fn generate_template(&self, config: &PackerConfig) -> Result<String> {
        let context = self.templates.create_alibaba_context(config)?;
        self.templates.render("alibaba.pkr.hcl.tera", &context)
    }
}
