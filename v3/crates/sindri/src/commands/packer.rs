//! Packer command - Build VM images with HashiCorp Packer
//!
//! This command provides multi-cloud VM image building using the sindri-packer crate.

use anyhow::{anyhow, Result};
use sindri_core::types::packer_config::{
    AlibabaConfig, AwsConfig, AzureConfig, BuildConfig, CloudProvider, GcpConfig, OciConfig,
    PackerConfig, SecurityConfig,
};
use sindri_packer::{
    check_packer_installed, create_packer_provider, BuildOptions, OnErrorBehavior,
};

use crate::cli::{
    PackerBuildArgs, PackerCommands, PackerDeleteArgs, PackerDeployArgs, PackerDoctorArgs,
    PackerInitArgs, PackerListArgs, PackerValidateArgs,
};
use crate::output;

pub async fn run(command: PackerCommands) -> Result<()> {
    match command {
        PackerCommands::Build(args) => build(args).await,
        PackerCommands::Validate(args) => validate(args).await,
        PackerCommands::List(args) => list(args).await,
        PackerCommands::Delete(args) => delete(args).await,
        PackerCommands::Doctor(args) => doctor(args).await,
        PackerCommands::Init(args) => init(args).await,
        PackerCommands::Deploy(args) => deploy(args).await,
    }
}

/// Parse cloud provider from string
fn parse_cloud(cloud: &str) -> Result<CloudProvider> {
    match cloud.to_lowercase().as_str() {
        "aws" | "amazon" => Ok(CloudProvider::Aws),
        "azure" | "microsoft" => Ok(CloudProvider::Azure),
        "gcp" | "google" => Ok(CloudProvider::Gcp),
        "oci" | "oracle" => Ok(CloudProvider::Oci),
        "alibaba" | "alicloud" => Ok(CloudProvider::Alibaba),
        _ => Err(anyhow!(
            "Unknown cloud provider: {}. Supported: aws, azure, gcp, oci, alibaba",
            cloud
        )),
    }
}

/// Arguments for building a Packer configuration
struct BuildConfigArgs {
    cloud: CloudProvider,
    name: Option<String>,
    sindri_version: String,
    profile: Option<String>,
    extensions: Option<String>,
    region: Option<String>,
    instance_type: Option<String>,
    disk_size: Option<u32>,
    cis_hardening: bool,
}

/// Build a Packer configuration from CLI args
fn build_config(args: BuildConfigArgs) -> PackerConfig {
    let cloud = args.cloud;
    let image_name = args.name.unwrap_or_else(|| "sindri-dev".to_string());
    let extensions: Vec<String> = args
        .extensions
        .map(|e| e.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    let mut config = PackerConfig {
        cloud,
        image_name,
        description: Some("Sindri development environment".to_string()),
        build: BuildConfig {
            sindri_version: args.sindri_version,
            extensions,
            profile: args.profile,
            ssh_timeout: "20m".to_string(),
            security: SecurityConfig {
                cis_hardening: args.cis_hardening,
                openscap_scan: false,
                clean_sensitive_data: true,
                remove_ssh_keys: true,
            },
            ..Default::default()
        },
        ..Default::default()
    };

    // Set cloud-specific configuration
    match cloud {
        CloudProvider::Aws => {
            config.aws = Some(AwsConfig {
                region: args.region.unwrap_or_else(|| "us-west-2".to_string()),
                instance_type: args.instance_type.unwrap_or_else(|| "t3.large".to_string()),
                volume_size: args.disk_size.unwrap_or(60),
                ..Default::default()
            });
        }
        CloudProvider::Azure => {
            config.azure = Some(AzureConfig {
                location: args.region.unwrap_or_else(|| "eastus".to_string()),
                vm_size: args
                    .instance_type
                    .unwrap_or_else(|| "Standard_D2s_v3".to_string()),
                os_disk_size_gb: args.disk_size.unwrap_or(60),
                subscription_id: std::env::var("AZURE_SUBSCRIPTION_ID").unwrap_or_default(),
                resource_group: std::env::var("AZURE_RESOURCE_GROUP")
                    .unwrap_or_else(|_| "sindri-packer".to_string()),
                ..Default::default()
            });
        }
        CloudProvider::Gcp => {
            config.gcp = Some(GcpConfig {
                project_id: std::env::var("GCP_PROJECT_ID").unwrap_or_default(),
                zone: args.region.unwrap_or_else(|| "us-central1-a".to_string()),
                machine_type: args
                    .instance_type
                    .unwrap_or_else(|| "e2-standard-2".to_string()),
                disk_size: args.disk_size.unwrap_or(60),
                ..Default::default()
            });
        }
        CloudProvider::Oci => {
            config.oci = Some(OciConfig {
                compartment_ocid: std::env::var("OCI_COMPARTMENT_OCID").unwrap_or_default(),
                availability_domain: args.region.unwrap_or_default(),
                shape: args
                    .instance_type
                    .unwrap_or_else(|| "VM.Standard.E4.Flex".to_string()),
                boot_volume_size_gb: args.disk_size.unwrap_or(60),
                subnet_ocid: std::env::var("OCI_SUBNET_OCID").unwrap_or_default(),
                ..Default::default()
            });
        }
        CloudProvider::Alibaba => {
            config.alibaba = Some(AlibabaConfig {
                region: args.region.unwrap_or_else(|| "cn-hangzhou".to_string()),
                instance_type: args
                    .instance_type
                    .unwrap_or_else(|| "ecs.g6.xlarge".to_string()),
                system_disk_size_gb: args.disk_size.unwrap_or(80),
                ..Default::default()
            });
        }
    }

    config
}

async fn build(args: PackerBuildArgs) -> Result<()> {
    let cloud = parse_cloud(&args.cloud)?;

    output::header(&format!("Building {} VM image", args.cloud.to_uppercase()));

    // Check prerequisites first
    let provider = create_packer_provider(cloud)?;
    let prereqs = provider.check_cloud_prerequisites()?;

    if !prereqs.packer_installed {
        output::error("Packer is not installed");
        for hint in &prereqs.hints {
            output::info(hint);
        }
        return Err(anyhow!("Prerequisites not satisfied"));
    }

    if !prereqs.satisfied {
        output::warning("Some prerequisites are missing:");
        for missing in &prereqs.missing {
            output::kv("Missing", missing);
        }
        for hint in &prereqs.hints {
            output::info(hint);
        }
        output::info("");
    }

    // Build configuration
    let config = build_config(BuildConfigArgs {
        cloud,
        name: args.name.clone(),
        sindri_version: args.sindri_version.clone(),
        profile: args.profile.clone(),
        extensions: args.extensions.clone(),
        region: args.region.clone(),
        instance_type: args.instance_type.clone(),
        disk_size: args.disk_size,
        cis_hardening: args.cis_hardening,
    });

    output::kv("Image name", &config.image_name);
    output::kv("Sindri version", &config.build.sindri_version);
    if let Some(profile) = &config.build.profile {
        output::kv("Profile", profile);
    }
    if !config.build.extensions.is_empty() {
        output::kv("Extensions", &config.build.extensions.join(", "));
    }
    output::info("");

    // Dry run - just generate and validate template
    if args.dry_run {
        let spinner = output::spinner("Generating template...");
        let template = provider.generate_template(&config)?;
        spinner.finish_and_clear();

        output::success("Generated Packer template:");
        output::info("");
        println!("{}", template);
        return Ok(());
    }

    // Check for cached image
    if !args.force {
        let spinner = output::spinner("Checking for cached image...");
        if let Ok(Some(image_id)) = provider.find_cached_image(&config).await {
            spinner.finish_and_clear();
            output::success(&format!("Found cached image: {}", image_id));
            output::info("Use --force to rebuild");
            return Ok(());
        }
        spinner.finish_and_clear();
    }

    // Build the image
    let spinner = output::spinner("Building image...");
    let opts = BuildOptions {
        force: args.force,
        debug: args.debug,
        on_error: OnErrorBehavior::Cleanup,
        var_files: args.var_file.map(|p| vec![p.into()]).unwrap_or_default(),
        ..Default::default()
    };

    let result = provider.build_image(&config, opts).await?;
    spinner.finish_and_clear();

    if result.success {
        output::success("Image build complete");
        output::kv("Image ID", &result.image_id);
        output::kv("Region", &result.region);
        output::kv(
            "Build time",
            &format!("{:.1}s", result.build_time.as_secs_f64()),
        );

        if args.json {
            let json = serde_json::json!({
                "success": true,
                "image_id": result.image_id,
                "image_name": result.image_name,
                "provider": result.provider,
                "region": result.region,
                "build_time_secs": result.build_time.as_secs_f64(),
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
    } else {
        output::error("Image build failed");
        for log in &result.logs {
            if !log.is_empty() {
                output::info(log);
            }
        }
        return Err(anyhow!("Image build failed"));
    }

    Ok(())
}

async fn validate(args: PackerValidateArgs) -> Result<()> {
    let cloud = parse_cloud(&args.cloud)?;

    output::header(&format!(
        "Validating {} Packer template",
        args.cloud.to_uppercase()
    ));

    let provider = create_packer_provider(cloud)?;

    let config = build_config(BuildConfigArgs {
        cloud,
        name: args.name,
        sindri_version: args.sindri_version,
        profile: None,
        extensions: None,
        region: None,
        instance_type: None,
        disk_size: None,
        cis_hardening: false,
    });

    let spinner = output::spinner("Validating template...");
    let result = provider.validate_template(&config).await?;
    spinner.finish_and_clear();

    if result.valid {
        output::success("Template is valid");

        if args.json {
            let json = serde_json::json!({
                "valid": true,
                "warnings": result.warnings,
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
    } else {
        output::error("Template validation failed:");
        for error in &result.errors {
            output::info(&format!("  {}", error));
        }

        if args.json {
            let json = serde_json::json!({
                "valid": false,
                "errors": result.errors,
                "warnings": result.warnings,
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }

        return Err(anyhow!("Template validation failed"));
    }

    Ok(())
}

async fn list(args: PackerListArgs) -> Result<()> {
    let cloud = parse_cloud(&args.cloud)?;

    output::header(&format!("Listing {} images", args.cloud.to_uppercase()));

    let provider = create_packer_provider(cloud)?;

    let config = build_config(BuildConfigArgs {
        cloud,
        name: args.name,
        sindri_version: "latest".to_string(),
        profile: None,
        extensions: None,
        region: args.region,
        instance_type: None,
        disk_size: None,
        cis_hardening: false,
    });

    let spinner = output::spinner("Fetching images...");
    let images = provider.list_images(&config).await?;
    spinner.finish_and_clear();

    if images.is_empty() {
        output::info("No images found");
        return Ok(());
    }

    if args.json {
        let json: Vec<_> = images
            .iter()
            .map(|img| {
                serde_json::json!({
                    "id": img.id,
                    "name": img.name,
                    "state": format!("{:?}", img.state),
                    "created_at": img.created_at.map(|d| d.to_rfc3339()),
                    "sindri_version": img.sindri_version,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        output::info(&format!("Found {} image(s):", images.len()));
        output::info("");
        for img in &images {
            output::kv("ID", &img.id);
            output::kv("Name", &img.name);
            output::kv("State", &format!("{:?}", img.state));
            if let Some(version) = &img.sindri_version {
                output::kv("Sindri version", version);
            }
            if let Some(created) = &img.created_at {
                output::kv("Created", &created.to_rfc3339());
            }
            output::info("");
        }
    }

    Ok(())
}

async fn delete(args: PackerDeleteArgs) -> Result<()> {
    let cloud = parse_cloud(&args.cloud)?;

    output::header(&format!("Deleting {} image", args.cloud.to_uppercase()));

    if !args.force {
        output::warning(&format!(
            "This will permanently delete image: {}",
            args.image_id
        ));
        let confirm = dialoguer::Confirm::new()
            .with_prompt("Are you sure?")
            .default(false)
            .interact()?;

        if !confirm {
            output::info("Cancelled");
            return Ok(());
        }
    }

    let provider = create_packer_provider(cloud)?;

    let config = build_config(BuildConfigArgs {
        cloud,
        name: None,
        sindri_version: "latest".to_string(),
        profile: None,
        extensions: None,
        region: args.region,
        instance_type: None,
        disk_size: None,
        cis_hardening: false,
    });

    let spinner = output::spinner("Deleting image...");
    provider.delete_image(&config, &args.image_id).await?;
    spinner.finish_and_clear();

    output::success(&format!("Deleted image: {}", args.image_id));

    Ok(())
}

async fn doctor(args: PackerDoctorArgs) -> Result<()> {
    output::header("Packer Prerequisites Check");

    // Check Packer first
    let packer_version = check_packer_installed()?;
    if let Some(version) = &packer_version {
        output::success(&format!("Packer installed: {}", version));
    } else {
        output::error("Packer not installed");
        output::info("  Install: https://developer.hashicorp.com/packer/install");
    }
    output::info("");

    // Check cloud-specific prerequisites
    let clouds: Vec<CloudProvider> = if let Some(cloud) = &args.cloud {
        if cloud == "all" {
            vec![
                CloudProvider::Aws,
                CloudProvider::Azure,
                CloudProvider::Gcp,
                CloudProvider::Oci,
                CloudProvider::Alibaba,
            ]
        } else {
            vec![parse_cloud(cloud)?]
        }
    } else {
        // Default to checking all clouds
        vec![
            CloudProvider::Aws,
            CloudProvider::Azure,
            CloudProvider::Gcp,
            CloudProvider::Oci,
            CloudProvider::Alibaba,
        ]
    };

    let mut all_results = Vec::new();

    for cloud in &clouds {
        let provider = create_packer_provider(*cloud)?;
        let prereqs = provider.check_cloud_prerequisites()?;

        output::kv(
            &format!("{} Prerequisites", provider.cloud_name().to_uppercase()),
            "",
        );

        if prereqs.cli_installed {
            output::success(&format!(
                "  CLI installed: {}",
                prereqs.cli_version.as_deref().unwrap_or("unknown version")
            ));
        } else {
            output::error("  CLI not installed");
        }

        if prereqs.credentials_configured {
            output::success("  Credentials configured");
        } else {
            output::warning("  Credentials not configured");
        }

        if !prereqs.hints.is_empty() {
            for hint in &prereqs.hints {
                output::info(&format!("  {}", hint));
            }
        }

        output::info("");

        all_results.push(serde_json::json!({
            "cloud": provider.cloud_name(),
            "packer_installed": prereqs.packer_installed,
            "cli_installed": prereqs.cli_installed,
            "cli_version": prereqs.cli_version,
            "credentials_configured": prereqs.credentials_configured,
            "satisfied": prereqs.satisfied,
            "missing": prereqs.missing,
            "hints": prereqs.hints,
        }));
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&all_results)?);
    }

    Ok(())
}

async fn init(args: PackerInitArgs) -> Result<()> {
    let cloud = parse_cloud(&args.cloud)?;

    output::header(&format!(
        "Initializing {} Packer configuration",
        args.cloud.to_uppercase()
    ));

    let provider = create_packer_provider(cloud)?;

    let config = build_config(BuildConfigArgs {
        cloud,
        name: Some("sindri-dev".to_string()),
        sindri_version: "latest".to_string(),
        profile: None,
        extensions: None,
        region: None,
        instance_type: None,
        disk_size: None,
        cis_hardening: false,
    });

    // Generate template
    let template = provider.generate_template(&config)?;

    // Determine output path
    let output_dir = args
        .output
        .map(|p| p.into_std_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));

    let template_path = output_dir.join(format!("{}.pkr.hcl", args.cloud.to_lowercase()));

    // Check if file exists
    if template_path.exists() && !args.force {
        output::error(&format!("File already exists: {}", template_path.display()));
        output::info("Use --force to overwrite");
        return Err(anyhow!("File already exists"));
    }

    // Write template
    std::fs::create_dir_all(&output_dir)?;
    std::fs::write(&template_path, template)?;

    output::success(&format!("Created: {}", template_path.display()));
    output::info("");
    output::info("Next steps:");
    output::info(&format!("  1. Edit {} as needed", template_path.display()));
    output::info(&format!(
        "  2. Run: packer init {}",
        template_path.display()
    ));
    output::info(&format!(
        "  3. Run: packer build {}",
        template_path.display()
    ));

    Ok(())
}

async fn deploy(args: PackerDeployArgs) -> Result<()> {
    let cloud = parse_cloud(&args.cloud)?;

    output::header(&format!(
        "Deploying {} instance from image",
        args.cloud.to_uppercase()
    ));

    let provider = create_packer_provider(cloud)?;

    let config = build_config(BuildConfigArgs {
        cloud,
        name: None,
        sindri_version: "latest".to_string(),
        profile: None,
        extensions: None,
        region: args.region.clone(),
        instance_type: args.instance_type.clone(),
        disk_size: None,
        cis_hardening: false,
    });

    output::kv("Image ID", &args.image_id);
    if let Some(region) = &args.region {
        output::kv("Region", region);
    }
    if let Some(instance_type) = &args.instance_type {
        output::kv("Instance type", instance_type);
    }
    output::info("");

    let spinner = output::spinner("Launching instance...");
    let result = provider.deploy_from_image(&args.image_id, &config).await?;
    spinner.finish_and_clear();

    if result.success {
        output::success("Instance launched successfully");
        output::kv("Instance ID", &result.instance_id);
        if let Some(public_ip) = &result.public_ip {
            output::kv("Public IP", public_ip);
        }
        if let Some(private_ip) = &result.private_ip {
            output::kv("Private IP", private_ip);
        }
        if let Some(ssh) = &result.ssh_command {
            output::kv("SSH", ssh);
        }

        if args.json {
            let json = serde_json::json!({
                "success": true,
                "instance_id": result.instance_id,
                "public_ip": result.public_ip,
                "private_ip": result.private_ip,
                "ssh_command": result.ssh_command,
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
    } else {
        output::error("Instance launch failed");
        for msg in &result.messages {
            output::info(msg);
        }
        return Err(anyhow!("Instance launch failed"));
    }

    Ok(())
}
