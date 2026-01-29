//! AWS Packer provider implementation
//!
//! Builds EC2 AMI images using the `amazon-ebs` Packer builder.

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use tokio::process::Command;
use tracing::{debug, info};

use crate::templates::TemplateRegistry;
use crate::traits::{
    BuildOptions, BuildResult, CloudPrerequisiteStatus, DeployFromImageResult, ImageInfo,
    ImageState, PackerProvider, ValidationResult,
};
use crate::utils;
use sindri_core::types::packer_config::PackerConfig;

/// AWS Packer provider for building EC2 AMIs
pub struct AwsPackerProvider {
    templates: TemplateRegistry,
    output_dir: PathBuf,
}

impl AwsPackerProvider {
    /// Create a new AWS Packer provider
    pub fn new() -> Self {
        Self {
            templates: TemplateRegistry::new().expect("Failed to load templates"),
            output_dir: utils::default_output_dir().join("aws"),
        }
    }

    /// Check if AWS CLI is installed and configured
    fn check_aws_cli(&self) -> Result<Option<String>> {
        utils::check_cli_installed("aws", &["--version"])
    }

    /// Check if AWS credentials are configured
    fn check_aws_credentials(&self) -> bool {
        let result = std::process::Command::new("aws")
            .args(["sts", "get-caller-identity"])
            .output();

        match result {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    /// Get AWS account ID
    pub async fn get_account_id(&self) -> Result<String> {
        let output = Command::new("aws")
            .args([
                "sts",
                "get-caller-identity",
                "--query",
                "Account",
                "--output",
                "text",
            ])
            .output()
            .await
            .context("Failed to get AWS account ID")?;

        if !output.status.success() {
            return Err(anyhow!("Failed to get AWS account ID: not authenticated"));
        }

        let account_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(account_id)
    }

    /// Find the latest Ubuntu 24.04 AMI
    pub async fn find_base_ami(&self, region: &str) -> Result<String> {
        let output = Command::new("aws")
            .args([
                "ec2",
                "describe-images",
                "--region",
                region,
                "--owners",
                "099720109477",
                "--filters",
                "Name=name,Values=ubuntu/images/hvm-ssd/ubuntu-jammy-24.04-amd64-server-*",
                "Name=state,Values=available",
                "--query",
                "sort_by(Images, &CreationDate)[-1].ImageId",
                "--output",
                "text",
            ])
            .output()
            .await
            .context("Failed to find base AMI")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to find base AMI: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let ami_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if ami_id.is_empty() || ami_id == "None" {
            return Err(anyhow!("No Ubuntu 24.04 AMI found in region {}", region));
        }

        Ok(ami_id)
    }

    /// Run Packer build
    async fn run_packer_build(
        &self,
        template_path: &std::path::Path,
        config: &PackerConfig,
        opts: &BuildOptions,
    ) -> Result<BuildResult> {
        let mut cmd = Command::new("packer");
        cmd.arg("build");

        if opts.force {
            cmd.arg("-force");
        }

        if opts.debug {
            cmd.env("PACKER_LOG", "1");
        }

        if let Some(only) = &opts.only {
            cmd.arg(format!("-only={}", only.join(",")));
        }

        if let Some(except) = &opts.except {
            cmd.arg(format!("-except={}", except.join(",")));
        }

        for var_file in &opts.var_files {
            cmd.arg(format!("-var-file={}", var_file.display()));
        }

        for (key, value) in &opts.variables {
            cmd.arg(format!("-var={}={}", key, value));
        }

        cmd.arg(format!("-on-error={}", opts.on_error.as_packer_flag()));

        cmd.arg(template_path);

        let start = Instant::now();
        let output = cmd.output().await.context("Failed to run Packer build")?;
        let build_time = start.elapsed();

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let success = output.status.success();
        let image_id = if success {
            utils::parse_ami_id(&stdout).unwrap_or_else(|| "unknown".to_string())
        } else {
            String::new()
        };

        let region = config
            .aws
            .as_ref()
            .map(|a| a.region.clone())
            .unwrap_or_else(|| "us-west-2".to_string());

        // Try to read manifest
        let manifest_path = self.output_dir.join("manifest.json");
        let manifest = if manifest_path.exists() {
            serde_json::from_str(&std::fs::read_to_string(&manifest_path)?).ok()
        } else {
            None
        };

        Ok(BuildResult {
            success,
            image_id,
            image_name: config.image_name.clone(),
            provider: "aws".to_string(),
            region,
            build_time,
            artifact_size: None,
            manifest,
            logs: vec![stdout, stderr],
            metadata: HashMap::new(),
        })
    }
}

impl Default for AwsPackerProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackerProvider for AwsPackerProvider {
    fn cloud_name(&self) -> &'static str {
        "aws"
    }

    async fn build_image(&self, config: &PackerConfig, opts: BuildOptions) -> Result<BuildResult> {
        info!("Building AWS AMI: {}", config.image_name);

        // Ensure output directory exists
        utils::ensure_dir(&self.output_dir)?;

        // Generate template
        let template_content = self.generate_template(config)?;
        let template_path = self.output_dir.join("aws.pkr.hcl");
        utils::write_file(&template_path, &template_content)?;

        // Generate provisioning scripts
        let scripts = self.templates.render_scripts(config)?;
        let scripts_dir = self.output_dir.join("scripts");
        utils::ensure_dir(&scripts_dir)?;
        for (name, content) in scripts {
            utils::write_file(&scripts_dir.join(&name), &content)?;
        }

        // Initialize Packer plugins
        let init_output = utils::packer_init(&template_path).await?;
        if !init_output.status.success() {
            return Err(anyhow!(
                "Packer init failed: {}",
                String::from_utf8_lossy(&init_output.stderr)
            ));
        }

        // Validate template
        let validate_output = utils::packer_validate(&template_path, false).await?;
        if !validate_output.status.success() {
            return Err(anyhow!(
                "Packer validation failed: {}",
                String::from_utf8_lossy(&validate_output.stderr)
            ));
        }

        // Run build
        self.run_packer_build(&template_path, config, &opts).await
    }

    async fn list_images(&self, config: &PackerConfig) -> Result<Vec<ImageInfo>> {
        let aws = config
            .aws
            .as_ref()
            .ok_or_else(|| anyhow!("AWS configuration required"))?;

        let output = Command::new("aws")
            .args([
                "ec2", "describe-images",
                "--region", &aws.region,
                "--owners", "self",
                "--filters", &format!("Name=name,Values={}*", config.image_name),
                "--query", "Images[*].{Id:ImageId,Name:Name,Created:CreationDate,State:State,Description:Description}",
                "--output", "json",
            ])
            .output()
            .await
            .context("Failed to list AMIs")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to list AMIs: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let images: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout)?;
        let result = images
            .into_iter()
            .map(|img| ImageInfo {
                id: img["Id"].as_str().unwrap_or("").to_string(),
                name: img["Name"].as_str().unwrap_or("").to_string(),
                description: img["Description"].as_str().map(|s| s.to_string()),
                state: match img["State"].as_str() {
                    Some("available") => ImageState::Available,
                    Some("pending") => ImageState::Pending,
                    Some("failed") => ImageState::Failed,
                    Some("deregistered") => ImageState::Deregistered,
                    _ => ImageState::Unknown,
                },
                created_at: img["Created"]
                    .as_str()
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc)),
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

    async fn delete_image(&self, config: &PackerConfig, image_id: &str) -> Result<()> {
        let aws = config
            .aws
            .as_ref()
            .ok_or_else(|| anyhow!("AWS configuration required"))?;

        info!("Deregistering AMI: {}", image_id);

        // First, get snapshot IDs associated with the AMI
        let describe_output = Command::new("aws")
            .args([
                "ec2",
                "describe-images",
                "--region",
                &aws.region,
                "--image-ids",
                image_id,
                "--query",
                "Images[0].BlockDeviceMappings[*].Ebs.SnapshotId",
                "--output",
                "json",
            ])
            .output()
            .await?;

        let snapshot_ids: Vec<String> = if describe_output.status.success() {
            serde_json::from_slice(&describe_output.stdout).unwrap_or_default()
        } else {
            Vec::new()
        };

        // Deregister the AMI
        let output = Command::new("aws")
            .args([
                "ec2",
                "deregister-image",
                "--region",
                &aws.region,
                "--image-id",
                image_id,
            ])
            .output()
            .await
            .context("Failed to deregister AMI")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to deregister AMI: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        // Delete associated snapshots
        for snapshot_id in snapshot_ids {
            if !snapshot_id.is_empty() {
                debug!("Deleting snapshot: {}", snapshot_id);
                let _ = Command::new("aws")
                    .args([
                        "ec2",
                        "delete-snapshot",
                        "--region",
                        &aws.region,
                        "--snapshot-id",
                        &snapshot_id,
                    ])
                    .output()
                    .await;
            }
        }

        info!("Deleted AMI: {}", image_id);
        Ok(())
    }

    async fn get_image(&self, config: &PackerConfig, image_id: &str) -> Result<ImageInfo> {
        let aws = config
            .aws
            .as_ref()
            .ok_or_else(|| anyhow!("AWS configuration required"))?;

        let output = Command::new("aws")
            .args([
                "ec2",
                "describe-images",
                "--region",
                &aws.region,
                "--image-ids",
                image_id,
                "--output",
                "json",
            ])
            .output()
            .await
            .context("Failed to describe AMI")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to describe AMI: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let response: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        let images = response["Images"]
            .as_array()
            .ok_or_else(|| anyhow!("Invalid response format"))?;

        if images.is_empty() {
            return Err(anyhow!("Image not found: {}", image_id));
        }

        let img = &images[0];

        // Extract tags
        let mut tags = HashMap::new();
        if let Some(tag_arr) = img["Tags"].as_array() {
            for tag in tag_arr {
                if let (Some(key), Some(value)) = (tag["Key"].as_str(), tag["Value"].as_str()) {
                    tags.insert(key.to_string(), value.to_string());
                }
            }
        }

        Ok(ImageInfo {
            id: img["ImageId"].as_str().unwrap_or("").to_string(),
            name: img["Name"].as_str().unwrap_or("").to_string(),
            description: img["Description"].as_str().map(|s| s.to_string()),
            state: match img["State"].as_str() {
                Some("available") => ImageState::Available,
                Some("pending") => ImageState::Pending,
                Some("failed") => ImageState::Failed,
                Some("deregistered") => ImageState::Deregistered,
                _ => ImageState::Unknown,
            },
            created_at: img["CreationDate"]
                .as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc)),
            size: None,
            sindri_version: tags.get("SindriVersion").cloned(),
            extensions: Vec::new(),
            profile: None,
            tags,
            metadata: HashMap::new(),
        })
    }

    async fn validate_template(&self, config: &PackerConfig) -> Result<ValidationResult> {
        // Generate template
        let template_content = self.generate_template(config)?;

        // Write to temp location
        utils::ensure_dir(&self.output_dir)?;
        let template_path = self.output_dir.join("aws.pkr.hcl");
        utils::write_file(&template_path, &template_content)?;

        // Run packer validate
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
        let cli_version = self.check_aws_cli()?;
        let credentials_configured = self.check_aws_credentials();

        let mut missing = Vec::new();
        let mut hints = Vec::new();

        if packer_version.is_none() {
            missing.push("packer".to_string());
            hints
                .push("Install Packer: https://developer.hashicorp.com/packer/install".to_string());
        }

        if cli_version.is_none() {
            missing.push("aws-cli".to_string());
            hints.push("Install AWS CLI: https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html".to_string());
        }

        if !credentials_configured {
            missing.push("aws-credentials".to_string());
            hints.push("Configure AWS credentials: aws configure".to_string());
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

        // Find an image that matches the configuration
        let config_hash = utils::config_hash(&config.build);

        for image in images {
            if image.state == ImageState::Available {
                // Check if tags match our config
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
        let aws = config
            .aws
            .as_ref()
            .ok_or_else(|| anyhow!("AWS configuration required"))?;

        info!("Launching EC2 instance from AMI: {}", image_id);

        // Generate a unique instance name
        let instance_name = format!("{}-{}", config.image_name, utils::generate_build_id());
        let tag_spec = format!(
            "ResourceType=instance,Tags=[{{Key=Name,Value={}}}]",
            instance_name
        );

        let mut args = vec![
            "ec2",
            "run-instances",
            "--region",
            &aws.region,
            "--image-id",
            image_id,
            "--instance-type",
            &aws.instance_type,
            "--count",
            "1",
            "--tag-specifications",
            tag_spec.as_str(),
        ];

        // Add subnet if specified
        if let Some(subnet_id) = &aws.subnet_id {
            args.push("--subnet-id");
            args.push(subnet_id);
        }

        let output = Command::new("aws")
            .args(&args)
            .args(["--output", "json"])
            .output()
            .await
            .context("Failed to launch EC2 instance")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to launch EC2 instance: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let response: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        let instance_id = response["Instances"][0]["InstanceId"]
            .as_str()
            .unwrap_or("")
            .to_string();

        // Wait for instance to be running and get IP
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        let describe_output = Command::new("aws")
            .args([
                "ec2", "describe-instances",
                "--region", &aws.region,
                "--instance-ids", &instance_id,
                "--query", "Reservations[0].Instances[0].{PublicIp:PublicIpAddress,PrivateIp:PrivateIpAddress}",
                "--output", "json",
            ])
            .output()
            .await?;

        let ips: serde_json::Value = serde_json::from_slice(&describe_output.stdout)?;
        let public_ip = ips["PublicIp"].as_str().map(|s| s.to_string());
        let private_ip = ips["PrivateIp"].as_str().map(|s| s.to_string());

        let ssh_command = public_ip.as_ref().map(|ip| format!("ssh ubuntu@{}", ip));

        Ok(DeployFromImageResult {
            success: true,
            instance_id,
            public_ip,
            private_ip,
            ssh_command,
            messages: vec!["EC2 instance launched successfully".to_string()],
        })
    }

    fn generate_template(&self, config: &PackerConfig) -> Result<String> {
        let context = self.templates.create_aws_context(config)?;
        self.templates.render("aws.pkr.hcl.tera", &context)
    }
}
