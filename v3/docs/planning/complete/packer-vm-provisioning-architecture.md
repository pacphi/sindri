# Sindri v3 HashiCorp Packer VM Provisioning Architecture

**Status:** Implemented
**Date:** 2026-01-24 (Planning) / 2026-01-25 (Implemented)
**Author:** Sindri Core Team
**Related:** [ADR-002: Provider Abstraction Layer](../../architecture/adr/002-provider-abstraction-layer.md), [ADR-003: Template-Based Configuration](../../architecture/adr/003-template-based-configuration.md), [ADR-031: Packer VM Provisioning Architecture](../../architecture/adr/031-packer-vm-provisioning-architecture.md)

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Background & Motivation](#background--motivation)
3. [Research Findings](#research-findings)
4. [Architecture Design](#architecture-design)
5. [Provider Implementations](#provider-implementations)
6. [Template System](#template-system)
7. [GitHub Actions Integration](#github-actions-integration)
8. [Security Architecture](#security-architecture)
9. [Testing Strategy](#testing-strategy)
10. [Implementation Phases](#implementation-phases)
11. [File Structure](#file-structure)
12. [Configuration Schema](#configuration-schema)
13. [CLI Commands](#cli-commands)
14. [Success Criteria](#success-criteria)
15. [Risk Assessment](#risk-assessment)
16. [References](#references)

---

## Executive Summary

This document outlines the architecture for integrating HashiCorp Packer support into Sindri v3 as a **unified `packer` provider**, enabling users to build and provision VM images across five major cloud providers using the familiar `sindri deploy` workflow:

- **Amazon Web Services (AWS)** - EC2 AMI images
- **Microsoft Azure** - Managed images with Shared Image Gallery
- **Google Cloud Platform (GCP)** - Compute Engine images
- **Oracle Cloud Infrastructure (OCI)** - Custom images
- **Alibaba Cloud** - ECS custom images

### Design Principles

1. **Consistent CLI Experience**: Uses `sindri deploy/connect/status/destroy` - same pattern as Docker, Fly.io, and DevPod providers
2. **Single Provider, Multiple Clouds**: One `packer` provider with a `cloud` attribute (aws, azure, gcp, oci, alibaba) - similar to DevPod's multi-backend approach
3. **Optional Pre-built Images**: Use `image_id` to skip building and deploy from existing images
4. **Config-Driven with Rich Documentation**: Generated YAML includes inline comments with citations to official cloud documentation

The implementation follows Sindri v3's established provider abstraction pattern, using:

- Async trait-based provider interface
- Tera template-driven configuration generation
- HCL2 Packer templates for multi-cloud consistency
- GitHub Actions workflows for CI/CD integration

**Key Deliverables:**

1. `sindri-packer` crate with unified multi-cloud Packer provider
2. HCL2 templates for all 5 cloud providers
3. Generated `sindri.yaml` templates with rich inline comments and citations
4. Per-cloud reference documentation (AWS, Azure, GCP, OCI, Alibaba)
5. GitHub Actions reusable workflows
6. Comprehensive test suite with InSpec validation

---

## Background & Motivation

### Current State

Sindri v3 currently supports 5 deployment providers:

- **Docker** - Local container-based development
- **Fly.io** - Cloud VMs with auto-suspend
- **DevPod** - Multi-cloud dev environments via SSH
- **E2B** - Cloud sandboxes with pause/resume
- **Kubernetes** - Container orchestration

### Gap Analysis

| Capability               | Current          | With Packer         |
| ------------------------ | ---------------- | ------------------- |
| Pre-built VM images      | No               | Yes                 |
| Multi-cloud consistency  | Partial (DevPod) | Full                |
| Golden image pipelines   | No               | Yes                 |
| Immutable infrastructure | No               | Yes                 |
| Enterprise cloud support | Limited          | Complete            |
| Offline deployments      | No               | Yes (cached images) |

### Business Drivers

1. **Enterprise Adoption** - Large organizations require VM-based deployments with compliance requirements
2. **Consistency** - Pre-built images ensure identical environments across deployments
3. **Performance** - Eliminates container image pull time (images pre-cached)
4. **Compliance** - CIS-hardened images for regulated industries
5. **Cost Optimization** - Spot/preemptible instances with known-good images

---

## Research Findings

### Clicktruck Organization Analysis

The [clicktruck](https://github.com/clicktruck) GitHub organization provides excellent reference implementations for multi-cloud Packer deployments. Key learnings:

#### Common Patterns Identified

1. **Unified Toolset Image**
   - Same development tools across all clouds
   - ~60GB disk allocation (varies by provider)
   - Ubuntu 24.04 LTS base image
   - SSH access with configurable user

2. **Build Variants**
   - `standard` - Core tools only
   - `with-tanzu` - Includes additional binaries in `dist/` directory
   - Applicable to Sindri: `base`, `with-extensions`, `custom`

3. **Provisioning Script Architecture**

   ```
   scripts/
   ├── init.sh           # Common initialization
   ├── tools/
   │   ├── kubernetes.sh # kubectl, helm, k9s
   │   ├── cloud-cli.sh  # AWS, Azure, GCP CLIs
   │   ├── languages.sh  # Node, Python, Go, Rust
   │   └── devtools.sh   # Git, Docker, utilities
   └── cleanup.sh        # Image cleanup
   ```

4. **Multi-Region Support**
   - AWS: 20 regions
   - Azure: 8 regions via Shared Image Gallery
   - GCP: 80+ zones
   - OCI: Region + availability domain

#### Cloud-Specific Configurations

| Provider | Builder         | Base Image               | Disk             | Auth Method       |
| -------- | --------------- | ------------------------ | ---------------- | ----------------- |
| AWS      | `amazon-ebs`    | Ubuntu 24.04 (Canonical) | 60GB gp2         | Access keys       |
| Azure    | `azure-arm`     | Ubuntu 24.04 (Canonical) | 60GB Premium_LRS | Service principal |
| GCP      | `googlecompute` | Ubuntu 24.04 minimal     | 60GB SSD         | Service account   |
| OCI      | `oracle-oci`    | Ubuntu 24.04             | 80GB             | API signing key   |
| Alibaba  | `alicloud-ecs`  | Ubuntu 24.04             | 60GB             | Access key        |

### HashiCorp Packer Best Practices (2025-2026)

#### HCL2 Template Standards

```hcl
packer {
  required_plugins {
    amazon = {
      version = ">= 1.14.0"
      source  = "github.com/hashicorp/amazon"
    }
  }
}

variable "image_name" {
  type    = string
  default = "sindri-v3-image"
}

source "amazon-ebs" "base" {
  ami_name      = "${var.image_name}-{{timestamp}}"
  instance_type = var.instance_type
  region        = var.region

  source_ami_filter {
    filters = {
      name                = "ubuntu/images/*ubuntu-jammy-24.04-amd64-server-*"
      root-device-type    = "ebs"
      virtualization-type = "hvm"
    }
    most_recent = true
    owners      = ["099720109477"] # Canonical
  }

  ssh_username = "ubuntu"
}

build {
  sources = ["source.amazon-ebs.base"]

  provisioner "shell" {
    scripts = [
      "scripts/init.sh",
      "scripts/install-sindri.sh",
      "scripts/cleanup.sh"
    ]
  }

  post-processor "manifest" {
    output     = "manifest.json"
    strip_path = true
  }
}
```

#### Security Best Practices

1. **CIS Benchmark Compliance** - Automated hardening during build
2. **OpenSCAP Integration** - Security compliance scanning
3. **30-Day Repave Cycle** - HashiCorp-validated vulnerability management
4. **Envelope Encryption** - For sensitive provisioning data
5. **Least Privilege** - Minimal IAM/RBAC permissions

#### CI/CD Integration

1. **GitHub Actions** - Primary CI/CD platform
2. **HCP Packer** - Central image registry and version management
3. **Channel-Based Promotion** - development → test → production
4. **Terraform Integration** - Automatic image consumption via data sources

---

## Architecture Design

### Provider Trait Extension

The Packer provider implements the existing `Provider` trait with additional capabilities:

```rust
// crates/sindri-packer/src/traits.rs

use sindri_providers::Provider;

/// Extended trait for Packer-based providers
#[async_trait]
pub trait PackerProvider: Provider {
    /// Build VM image using Packer
    async fn build_image(&self, config: &PackerConfig, opts: BuildOptions) -> Result<BuildResult>;

    /// List available images
    async fn list_images(&self, config: &PackerConfig) -> Result<Vec<ImageInfo>>;

    /// Delete an image
    async fn delete_image(&self, config: &PackerConfig, image_id: &str) -> Result<()>;

    /// Get image details
    async fn get_image(&self, config: &PackerConfig, image_id: &str) -> Result<ImageInfo>;

    /// Validate Packer template
    async fn validate_template(&self, config: &PackerConfig) -> Result<ValidationResult>;

    /// Check cloud provider prerequisites
    fn check_cloud_prerequisites(&self) -> Result<CloudPrerequisiteStatus>;
}

/// Build options for Packer
pub struct BuildOptions {
    pub force: bool,
    pub only: Option<Vec<String>>,  // Build specific sources only
    pub except: Option<Vec<String>>, // Exclude specific sources
    pub var_files: Vec<PathBuf>,
    pub variables: HashMap<String, String>,
    pub debug: bool,
    pub on_error: OnErrorBehavior,
    pub parallel_builds: u32,
}

/// Build result
pub struct BuildResult {
    pub success: bool,
    pub image_id: String,
    pub image_name: String,
    pub provider: String,
    pub region: String,
    pub build_time: Duration,
    pub artifact_size: Option<u64>,
    pub manifest: Option<PackerManifest>,
    pub logs: Vec<String>,
}
```

### Multi-Cloud Provider Factory

```rust
// crates/sindri-packer/src/lib.rs

pub mod aws;
pub mod azure;
pub mod gcp;
pub mod oci;
pub mod alibaba;
pub mod templates;
pub mod traits;
mod utils;

pub use traits::{PackerProvider, BuildOptions, BuildResult};

use anyhow::Result;
use sindri_core::types::CloudProvider;

/// Create a Packer provider instance by cloud
pub fn create_packer_provider(cloud: CloudProvider) -> Result<Box<dyn PackerProvider>> {
    match cloud {
        CloudProvider::Aws => Ok(Box::new(aws::AwsPackerProvider::new())),
        CloudProvider::Azure => Ok(Box::new(azure::AzurePackerProvider::new())),
        CloudProvider::Gcp => Ok(Box::new(gcp::GcpPackerProvider::new())),
        CloudProvider::Oci => Ok(Box::new(oci::OciPackerProvider::new())),
        CloudProvider::Alibaba => Ok(Box::new(alibaba::AlibabaPackerProvider::new())),
    }
}

/// Build images for multiple clouds in parallel
pub async fn build_multi_cloud(
    clouds: &[CloudProvider],
    config: &PackerConfig,
    opts: BuildOptions,
) -> Result<Vec<BuildResult>> {
    use futures::future::join_all;

    let futures = clouds.iter().map(|cloud| {
        let provider = create_packer_provider(*cloud)?;
        let config = config.clone();
        let opts = opts.clone();

        async move {
            provider.build_image(&config, opts).await
        }
    });

    let results = join_all(futures).await;
    results.into_iter().collect()
}
```

### Configuration Types

```rust
// crates/sindri-core/src/types/packer_config.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Cloud provider enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CloudProvider {
    Aws,
    Azure,
    Gcp,
    Oci,
    Alibaba,
}

/// Packer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackerConfig {
    /// Image name prefix
    pub image_name: String,

    /// Image description
    pub description: Option<String>,

    /// Sindri version to install
    pub sindri_version: String,

    /// Extensions to pre-install
    pub extensions: Vec<String>,

    /// Profile to install
    pub profile: Option<String>,

    /// Cloud-specific configurations
    pub clouds: CloudConfigs,

    /// Build configuration
    pub build: BuildConfig,

    /// Provisioning scripts
    pub provisioning: ProvisioningConfig,

    /// Security configuration
    pub security: SecurityConfig,
}

/// Cloud-specific configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudConfigs {
    pub aws: Option<AwsConfig>,
    pub azure: Option<AzureConfig>,
    pub gcp: Option<GcpConfig>,
    pub oci: Option<OciConfig>,
    pub alibaba: Option<AlibabaConfig>,
}

/// AWS-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsConfig {
    /// Target regions for AMI
    pub regions: Vec<String>,

    /// Instance type for building
    pub instance_type: String,

    /// VPC configuration
    pub vpc_id: Option<String>,
    pub subnet_id: Option<String>,

    /// EBS configuration
    pub volume_size: u32,
    pub volume_type: String,
    pub encrypt_boot: bool,

    /// AMI sharing
    pub ami_users: Vec<String>,
    pub ami_groups: Vec<String>,

    /// Tags
    pub tags: HashMap<String, String>,
}

/// Azure-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureConfig {
    /// Subscription and resource group
    pub subscription_id: String,
    pub resource_group: String,

    /// Managed image location
    pub location: String,

    /// VM size for building
    pub vm_size: String,

    /// Shared Image Gallery configuration
    pub gallery: Option<SharedImageGalleryConfig>,

    /// Disk configuration
    pub os_disk_size_gb: u32,
    pub storage_account_type: String,
}

/// Shared Image Gallery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedImageGalleryConfig {
    pub gallery_name: String,
    pub image_name: String,
    pub image_version: String,
    pub replication_regions: Vec<String>,
}

/// GCP-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcpConfig {
    /// Project ID
    pub project_id: String,

    /// Zone for building
    pub zone: String,

    /// Machine type
    pub machine_type: String,

    /// Network configuration
    pub network: Option<String>,
    pub subnetwork: Option<String>,

    /// Disk configuration
    pub disk_size: u32,
    pub disk_type: String,

    /// Image family
    pub image_family: Option<String>,

    /// Enable Secure Boot
    pub enable_secure_boot: bool,
}

/// OCI-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciConfig {
    /// Compartment OCID
    pub compartment_ocid: String,

    /// Availability domain
    pub availability_domain: String,

    /// Shape (VM type)
    pub shape: String,

    /// Subnet OCID
    pub subnet_ocid: String,

    /// Boot volume size
    pub boot_volume_size_gb: u32,
}

/// Alibaba Cloud-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlibabaConfig {
    /// Region
    pub region: String,

    /// Instance type
    pub instance_type: String,

    /// VSwitch ID
    pub vswitch_id: Option<String>,

    /// System disk
    pub system_disk_size_gb: u32,
    pub system_disk_category: String,
}

/// Build configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Maximum parallel builds
    pub parallel_builds: u32,

    /// SSH timeout
    pub ssh_timeout: String,

    /// Packer log level
    pub log_level: String,

    /// Output directory
    pub output_dir: PathBuf,
}

/// Provisioning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisioningConfig {
    /// Base scripts to run
    pub scripts: Vec<String>,

    /// Ansible playbook (optional)
    pub ansible_playbook: Option<PathBuf>,

    /// Environment variables for provisioning
    pub environment: HashMap<String, String>,

    /// Files to upload
    pub file_uploads: Vec<FileUpload>,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable CIS benchmark hardening
    pub cis_hardening: bool,

    /// Run OpenSCAP scan
    pub openscap_scan: bool,

    /// Clean sensitive data
    pub clean_sensitive_data: bool,

    /// Remove SSH host keys
    pub remove_ssh_keys: bool,
}
```

---

## Provider Implementations

### AWS Packer Provider

```rust
// crates/sindri-packer/src/aws.rs

use async_trait::async_trait;
use anyhow::{Context, Result};
use crate::traits::{PackerProvider, BuildOptions, BuildResult};
use crate::templates::TemplateRegistry;
use sindri_core::config::SindriConfig;
use sindri_core::types::{DeployOptions, DeployResult, DeploymentStatus, PrerequisiteStatus};
use std::path::PathBuf;
use tokio::process::Command;
use tracing::{debug, info, warn};

pub struct AwsPackerProvider {
    templates: TemplateRegistry,
    output_dir: PathBuf,
}

impl AwsPackerProvider {
    pub fn new() -> Self {
        Self {
            templates: TemplateRegistry::new().expect("Failed to load templates"),
            output_dir: PathBuf::from(".sindri/packer"),
        }
    }

    /// Check AWS CLI is installed and configured
    fn check_aws_cli(&self) -> Result<bool> {
        let output = std::process::Command::new("aws")
            .args(["--version"])
            .output()
            .context("AWS CLI not found")?;
        Ok(output.status.success())
    }

    /// Get AWS account ID
    async fn get_account_id(&self) -> Result<String> {
        let output = Command::new("aws")
            .args(["sts", "get-caller-identity", "--query", "Account", "--output", "text"])
            .output()
            .await
            .context("Failed to get AWS account ID")?;

        let account_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(account_id)
    }

    /// Find latest Ubuntu AMI
    async fn find_base_ami(&self, region: &str) -> Result<String> {
        let output = Command::new("aws")
            .args([
                "ec2", "describe-images",
                "--region", region,
                "--owners", "099720109477",
                "--filters",
                "Name=name,Values=ubuntu/images/hvm-ssd/ubuntu-jammy-24.04-amd64-server-*",
                "Name=state,Values=available",
                "--query", "sort_by(Images, &CreationDate)[-1].ImageId",
                "--output", "text",
            ])
            .output()
            .await
            .context("Failed to find base AMI")?;

        let ami_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(ami_id)
    }

    /// Generate HCL2 template for AWS
    fn generate_template(&self, config: &PackerConfig) -> Result<String> {
        let context = self.templates.create_aws_context(config)?;
        self.templates.render("aws.pkr.hcl", &context)
    }

    /// Run Packer build
    async fn run_packer_build(&self, template_path: &Path, opts: &BuildOptions) -> Result<BuildResult> {
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

        cmd.arg(template_path);

        let start = std::time::Instant::now();
        let output = cmd.output().await.context("Failed to run Packer build")?;
        let build_time = start.elapsed();

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Parse manifest for image ID
        let manifest_path = self.output_dir.join("manifest.json");
        let manifest = if manifest_path.exists() {
            Some(serde_json::from_str(&std::fs::read_to_string(&manifest_path)?)?)
        } else {
            None
        };

        Ok(BuildResult {
            success: output.status.success(),
            image_id: self.extract_ami_id(&stdout)?,
            image_name: config.image_name.clone(),
            provider: "aws".to_string(),
            region: config.clouds.aws.as_ref().map(|a| a.regions[0].clone()).unwrap_or_default(),
            build_time,
            artifact_size: None,
            manifest,
            logs: vec![stdout.to_string(), stderr.to_string()],
        })
    }

    fn extract_ami_id(&self, output: &str) -> Result<String> {
        // Parse Packer output for AMI ID
        for line in output.lines() {
            if line.contains("ami-") {
                if let Some(ami_start) = line.find("ami-") {
                    let ami_end = line[ami_start..].find(|c: char| !c.is_alphanumeric() && c != '-')
                        .unwrap_or(line[ami_start..].len());
                    return Ok(line[ami_start..ami_start + ami_end].to_string());
                }
            }
        }
        Err(anyhow::anyhow!("Could not extract AMI ID from Packer output"))
    }
}

#[async_trait]
impl PackerProvider for AwsPackerProvider {
    async fn build_image(&self, config: &PackerConfig, opts: BuildOptions) -> Result<BuildResult> {
        info!("Building AWS AMI: {}", config.image_name);

        // Generate template
        let template_content = self.generate_template(config)?;
        let template_path = self.output_dir.join("aws.pkr.hcl");
        std::fs::create_dir_all(&self.output_dir)?;
        std::fs::write(&template_path, &template_content)?;

        // Initialize Packer plugins
        let init_output = Command::new("packer")
            .args(["init", template_path.to_str().unwrap()])
            .output()
            .await?;

        if !init_output.status.success() {
            return Err(anyhow::anyhow!("Packer init failed: {}",
                String::from_utf8_lossy(&init_output.stderr)));
        }

        // Validate template
        let validate_output = Command::new("packer")
            .args(["validate", template_path.to_str().unwrap()])
            .output()
            .await?;

        if !validate_output.status.success() {
            return Err(anyhow::anyhow!("Packer validation failed: {}",
                String::from_utf8_lossy(&validate_output.stderr)));
        }

        // Run build
        self.run_packer_build(&template_path, &opts).await
    }

    async fn list_images(&self, config: &PackerConfig) -> Result<Vec<ImageInfo>> {
        let aws_config = config.clouds.aws.as_ref()
            .ok_or_else(|| anyhow::anyhow!("AWS configuration required"))?;

        let output = Command::new("aws")
            .args([
                "ec2", "describe-images",
                "--region", &aws_config.regions[0],
                "--owners", "self",
                "--filters", &format!("Name=name,Values={}*", config.image_name),
                "--query", "Images[*].{Id:ImageId,Name:Name,Created:CreationDate,State:State}",
                "--output", "json",
            ])
            .output()
            .await?;

        let images: Vec<ImageInfo> = serde_json::from_slice(&output.stdout)?;
        Ok(images)
    }

    async fn delete_image(&self, config: &PackerConfig, image_id: &str) -> Result<()> {
        let aws_config = config.clouds.aws.as_ref()
            .ok_or_else(|| anyhow::anyhow!("AWS configuration required"))?;

        // Deregister AMI
        Command::new("aws")
            .args([
                "ec2", "deregister-image",
                "--region", &aws_config.regions[0],
                "--image-id", image_id,
            ])
            .output()
            .await?;

        info!("Deleted AMI: {}", image_id);
        Ok(())
    }

    async fn get_image(&self, config: &PackerConfig, image_id: &str) -> Result<ImageInfo> {
        let aws_config = config.clouds.aws.as_ref()
            .ok_or_else(|| anyhow::anyhow!("AWS configuration required"))?;

        let output = Command::new("aws")
            .args([
                "ec2", "describe-images",
                "--region", &aws_config.regions[0],
                "--image-ids", image_id,
                "--output", "json",
            ])
            .output()
            .await?;

        let response: DescribeImagesResponse = serde_json::from_slice(&output.stdout)?;
        response.images.into_iter().next()
            .ok_or_else(|| anyhow::anyhow!("Image not found: {}", image_id))
    }

    async fn validate_template(&self, config: &PackerConfig) -> Result<ValidationResult> {
        let template_content = self.generate_template(config)?;
        let template_path = self.output_dir.join("aws.pkr.hcl");
        std::fs::create_dir_all(&self.output_dir)?;
        std::fs::write(&template_path, &template_content)?;

        let output = Command::new("packer")
            .args(["validate", "-syntax-only", template_path.to_str().unwrap()])
            .output()
            .await?;

        Ok(ValidationResult {
            valid: output.status.success(),
            errors: if output.status.success() {
                vec![]
            } else {
                vec![String::from_utf8_lossy(&output.stderr).to_string()]
            },
            warnings: vec![],
        })
    }

    fn check_cloud_prerequisites(&self) -> Result<CloudPrerequisiteStatus> {
        let mut status = CloudPrerequisiteStatus::default();

        // Check AWS CLI
        status.cli_installed = self.check_aws_cli().unwrap_or(false);

        // Check Packer
        status.packer_installed = std::process::Command::new("packer")
            .args(["--version"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        // Check credentials
        status.credentials_configured = std::process::Command::new("aws")
            .args(["sts", "get-caller-identity"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        Ok(status)
    }
}

// Implement standard Provider trait for deployment
// Note: This is the internal AWS backend; users configure via `provider: packer` + `packer.cloud: aws`
#[async_trait]
impl Provider for AwsPackerProvider {
    fn name(&self) -> &'static str {
        "packer"  // Unified provider name
    }

    fn check_prerequisites(&self) -> Result<PrerequisiteStatus> {
        let cloud_status = self.check_cloud_prerequisites()?;

        let mut missing = vec![];
        if !cloud_status.packer_installed {
            missing.push("packer".to_string());
        }
        if !cloud_status.cli_installed {
            missing.push("aws-cli".to_string());
        }

        Ok(PrerequisiteStatus {
            satisfied: missing.is_empty(),
            missing,
            available: vec!["packer".to_string(), "aws-cli".to_string()],
            hints: vec![
                "Install Packer: https://developer.hashicorp.com/packer/install".to_string(),
                "Install AWS CLI: https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html".to_string(),
            ],
        })
    }

    async fn deploy(&self, config: &SindriConfig, opts: DeployOptions) -> Result<DeployResult> {
        // Build image first if not exists, then launch EC2 instance
        todo!("Implement EC2 instance launch from pre-built AMI")
    }

    async fn connect(&self, config: &SindriConfig) -> Result<()> {
        // SSH to EC2 instance
        todo!("Implement SSH connection to EC2 instance")
    }

    async fn status(&self, config: &SindriConfig) -> Result<DeploymentStatus> {
        todo!("Implement EC2 instance status check")
    }

    async fn destroy(&self, config: &SindriConfig, force: bool) -> Result<()> {
        todo!("Implement EC2 instance termination")
    }

    async fn plan(&self, config: &SindriConfig) -> Result<DeploymentPlan> {
        todo!("Implement deployment plan generation")
    }

    async fn start(&self, config: &SindriConfig) -> Result<()> {
        todo!("Implement EC2 instance start")
    }

    async fn stop(&self, config: &SindriConfig) -> Result<()> {
        todo!("Implement EC2 instance stop")
    }

    fn supports_gpu(&self) -> bool {
        true // AWS supports GPU instances (p3, p4, g4)
    }
}
```

### Similar implementations for Azure, GCP, OCI, and Alibaba...

_(See File Structure section for complete provider files)_

---

## Template System

### HCL2 Template Architecture

```
v3/crates/sindri-packer/src/templates/
├── mod.rs                    # Template registry
├── context.rs                # Template context builders
├── aws.pkr.hcl.tera         # AWS AMI template
├── azure.pkr.hcl.tera       # Azure managed image template
├── gcp.pkr.hcl.tera         # GCP image template
├── oci.pkr.hcl.tera         # Oracle Cloud image template
├── alibaba.pkr.hcl.tera     # Alibaba Cloud image template
├── common/
│   ├── variables.pkr.hcl.tera   # Common variables
│   ├── provisioners.pkr.hcl.tera # Shared provisioners
│   └── post-processors.pkr.hcl.tera # Shared post-processors
└── scripts/
    ├── init.sh.tera         # Initialization script
    ├── install-sindri.sh.tera # Sindri installation
    ├── install-extensions.sh.tera # Extension installation
    ├── security-hardening.sh.tera # CIS hardening
    └── cleanup.sh.tera      # Image cleanup
```

### AWS Template Example

```hcl
{# aws.pkr.hcl.tera #}
packer {
  required_plugins {
    amazon = {
      version = ">= 1.14.0"
      source  = "github.com/hashicorp/amazon"
    }
  }
}

variable "image_name" {
  type    = string
  default = "{{ image_name }}"
}

variable "sindri_version" {
  type    = string
  default = "{{ sindri_version }}"
}

variable "instance_type" {
  type    = string
  default = "{{ instance_type | default(value="t3.large") }}"
}

variable "region" {
  type    = string
  default = "{{ region | default(value="us-west-2") }}"
}

variable "volume_size" {
  type    = number
  default = {{ volume_size | default(value=60) }}
}

source "amazon-ebs" "sindri" {
  ami_name      = "${var.image_name}-{{timestamp}}"
  ami_description = "Sindri v3 development environment - {{ description | default(value="") }}"
  instance_type = var.instance_type
  region        = var.region

  source_ami_filter {
    filters = {
      name                = "ubuntu/images/hvm-ssd/ubuntu-jammy-24.04-amd64-server-*"
      root-device-type    = "ebs"
      virtualization-type = "hvm"
    }
    most_recent = true
    owners      = ["099720109477"] # Canonical
  }

  ssh_username = "ubuntu"
  ssh_timeout  = "{{ ssh_timeout | default(value="20m") }}"

  launch_block_device_mappings {
    device_name           = "/dev/sda1"
    volume_size           = var.volume_size
    volume_type           = "{{ volume_type | default(value="gp3") }}"
    delete_on_termination = true
    encrypted             = {{ encrypt_boot | default(value=true) | lower }}
  }

{% if vpc_id %}
  vpc_id    = "{{ vpc_id }}"
  subnet_id = "{{ subnet_id }}"
{% endif %}

{% if ami_regions | length > 0 %}
  ami_regions = [{% for r in ami_regions %}"{{ r }}"{% if not loop.last %}, {% endif %}{% endfor %}]
{% endif %}

{% if ami_users | length > 0 %}
  ami_users = [{% for u in ami_users %}"{{ u }}"{% if not loop.last %}, {% endif %}{% endfor %}]
{% endif %}

  tags = {
    Name          = var.image_name
    SindriVersion = var.sindri_version
    BuildDate     = "{{timestamp}}"
    BuiltBy       = "packer"
{% for key, value in tags %}
    {{ key }} = "{{ value }}"
{% endfor %}
  }
}

build {
  name    = "sindri-aws"
  sources = ["source.amazon-ebs.sindri"]

  # Upload provisioning scripts
  provisioner "file" {
    source      = "scripts/"
    destination = "/tmp/sindri-scripts/"
  }

{% if file_uploads | length > 0 %}
  # Upload additional files
{% for upload in file_uploads %}
  provisioner "file" {
    source      = "{{ upload.source }}"
    destination = "{{ upload.destination }}"
  }
{% endfor %}
{% endif %}

  # Run initialization
  provisioner "shell" {
    inline = [
      "chmod +x /tmp/sindri-scripts/*.sh",
      "sudo /tmp/sindri-scripts/init.sh"
    ]
    environment_vars = [
      "SINDRI_VERSION={{ sindri_version }}",
{% for key, value in environment %}
      "{{ key }}={{ value }}",
{% endfor %}
    ]
  }

  # Install Sindri
  provisioner "shell" {
    script = "/tmp/sindri-scripts/install-sindri.sh"
    environment_vars = [
      "SINDRI_VERSION={{ sindri_version }}",
      "INSTALL_PROFILE={{ profile | default(value="base") }}",
      "EXTENSIONS={{ extensions | join(sep=",") }}"
    ]
  }

{% if cis_hardening %}
  # CIS Benchmark Hardening
  provisioner "shell" {
    script = "/tmp/sindri-scripts/security-hardening.sh"
  }
{% endif %}

{% if ansible_playbook %}
  # Ansible provisioning
  provisioner "ansible" {
    playbook_file = "{{ ansible_playbook }}"
  }
{% endif %}

  # Cleanup
  provisioner "shell" {
    script = "/tmp/sindri-scripts/cleanup.sh"
    environment_vars = [
      "CLEAN_SENSITIVE_DATA={{ clean_sensitive_data | default(value=true) | lower }}",
      "REMOVE_SSH_KEYS={{ remove_ssh_keys | default(value=true) | lower }}"
    ]
  }

  # Generate manifest
  post-processor "manifest" {
    output     = "manifest.json"
    strip_path = true
    custom_data = {
      sindri_version = var.sindri_version
      build_time     = "{{timestamp}}"
    }
  }

{% if checksum_types | length > 0 %}
  # Generate checksums
  post-processor "checksum" {
    checksum_types = [{% for t in checksum_types %}"{{ t }}"{% if not loop.last %}, {% endif %}{% endfor %}]
    output         = "checksums.txt"
  }
{% endif %}
}
```

### Provisioning Scripts

```bash
#!/bin/bash
# scripts/install-sindri.sh.tera

set -euo pipefail

SINDRI_VERSION="${SINDRI_VERSION:-latest}"
INSTALL_PROFILE="${INSTALL_PROFILE:-base}"
EXTENSIONS="${EXTENSIONS:-}"

echo "=== Installing Sindri v${SINDRI_VERSION} ==="

# Determine architecture
ARCH=$(uname -m)
case $ARCH in
    x86_64)  ARCH="x86_64" ;;
    aarch64) ARCH="aarch64" ;;
    arm64)   ARCH="aarch64" ;;
    *)       echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

# Download Sindri binary
SINDRI_URL="https://github.com/{{ github_repo }}/releases/download/v${SINDRI_VERSION}/sindri-${SINDRI_VERSION}-linux-${ARCH}.tar.gz"

echo "Downloading from: $SINDRI_URL"
curl -fsSL "$SINDRI_URL" -o /tmp/sindri.tar.gz
tar -xzf /tmp/sindri.tar.gz -C /tmp
sudo mv /tmp/sindri /usr/local/bin/sindri
sudo chmod +x /usr/local/bin/sindri

# Verify installation
sindri --version

# Create Sindri directories
mkdir -p ~/.sindri/{state,extensions,cache}

# Install profile if specified
if [ -n "$INSTALL_PROFILE" ] && [ "$INSTALL_PROFILE" != "none" ]; then
    echo "Installing profile: $INSTALL_PROFILE"
    sindri profile install "$INSTALL_PROFILE" --yes
fi

# Install additional extensions
if [ -n "$EXTENSIONS" ]; then
    IFS=',' read -ra EXT_ARRAY <<< "$EXTENSIONS"
    for ext in "${EXT_ARRAY[@]}"; do
        echo "Installing extension: $ext"
        sindri extension install "$ext" --yes
    done
fi

echo "=== Sindri installation complete ==="
sindri doctor
```

---

## GitHub Actions Integration

### Reusable Workflow for Packer Builds

```yaml
# .github/workflows/packer-build.yml
name: Build Sindri VM Images

on:
  workflow_dispatch:
    inputs:
      clouds:
        description: "Target clouds (comma-separated: aws,azure,gcp,oci,alibaba)"
        required: true
        default: "aws"
      sindri_version:
        description: "Sindri version to install"
        required: true
        default: "latest"
      profile:
        description: "Extension profile to install"
        required: false
        default: "base"
      extensions:
        description: "Additional extensions (comma-separated)"
        required: false
        default: ""

  push:
    branches:
      - main
    paths:
      - "v3/packer/**"
      - ".github/workflows/packer-build.yml"

env:
  PACKER_VERSION: "1.10.0"

jobs:
  validate:
    name: Validate Packer Templates
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Packer
        uses: hashicorp/setup-packer@main
        with:
          version: ${{ env.PACKER_VERSION }}

      - name: Initialize Packer
        run: packer init v3/packer/

      - name: Validate Templates
        run: packer validate v3/packer/

  build-aws:
    name: Build AWS AMI
    needs: validate
    if: contains(github.event.inputs.clouds, 'aws') || github.event_name == 'push'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Configure AWS Credentials
        uses: aws-actions/configure-aws-credentials@v4
        with:
          aws-access-key-id: ${{ secrets.AWS_ACCESS_KEY_ID }}
          aws-secret-access-key: ${{ secrets.AWS_SECRET_ACCESS_KEY }}
          aws-region: us-west-2

      - name: Setup Packer
        uses: hashicorp/setup-packer@main
        with:
          version: ${{ env.PACKER_VERSION }}

      - name: Initialize Packer
        run: packer init v3/packer/aws/

      - name: Build AMI
        run: |
          packer build \
            -var "sindri_version=${{ github.event.inputs.sindri_version || 'latest' }}" \
            -var "profile=${{ github.event.inputs.profile || 'base' }}" \
            -var "extensions=${{ github.event.inputs.extensions || '' }}" \
            v3/packer/aws/

      - name: Upload Manifest
        uses: actions/upload-artifact@v4
        with:
          name: aws-manifest
          path: manifest.json

  build-azure:
    name: Build Azure Image
    needs: validate
    if: contains(github.event.inputs.clouds, 'azure')
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Azure Login
        uses: azure/login@v2
        with:
          creds: ${{ secrets.AZURE_CREDENTIALS }}

      - name: Setup Packer
        uses: hashicorp/setup-packer@main
        with:
          version: ${{ env.PACKER_VERSION }}

      - name: Initialize Packer
        run: packer init v3/packer/azure/

      - name: Build Azure Image
        env:
          ARM_CLIENT_ID: ${{ secrets.AZURE_CLIENT_ID }}
          ARM_CLIENT_SECRET: ${{ secrets.AZURE_CLIENT_SECRET }}
          ARM_SUBSCRIPTION_ID: ${{ secrets.AZURE_SUBSCRIPTION_ID }}
          ARM_TENANT_ID: ${{ secrets.AZURE_TENANT_ID }}
        run: |
          packer build \
            -var "sindri_version=${{ github.event.inputs.sindri_version || 'latest' }}" \
            -var "profile=${{ github.event.inputs.profile || 'base' }}" \
            v3/packer/azure/

      - name: Upload Manifest
        uses: actions/upload-artifact@v4
        with:
          name: azure-manifest
          path: manifest.json

  build-gcp:
    name: Build GCP Image
    needs: validate
    if: contains(github.event.inputs.clouds, 'gcp')
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Authenticate to GCP
        uses: google-github-actions/auth@v2
        with:
          credentials_json: ${{ secrets.GCP_SA_KEY }}

      - name: Setup Packer
        uses: hashicorp/setup-packer@main
        with:
          version: ${{ env.PACKER_VERSION }}

      - name: Initialize Packer
        run: packer init v3/packer/gcp/

      - name: Build GCP Image
        run: |
          packer build \
            -var "project_id=${{ secrets.GCP_PROJECT_ID }}" \
            -var "sindri_version=${{ github.event.inputs.sindri_version || 'latest' }}" \
            -var "profile=${{ github.event.inputs.profile || 'base' }}" \
            v3/packer/gcp/

      - name: Upload Manifest
        uses: actions/upload-artifact@v4
        with:
          name: gcp-manifest
          path: manifest.json

  build-oci:
    name: Build Oracle Cloud Image
    needs: validate
    if: contains(github.event.inputs.clouds, 'oci')
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup OCI CLI
        uses: oracle-actions/configure-oci-cli@v1
        with:
          user: ${{ secrets.OCI_USER_OCID }}
          fingerprint: ${{ secrets.OCI_FINGERPRINT }}
          tenancy: ${{ secrets.OCI_TENANCY_OCID }}
          region: ${{ secrets.OCI_REGION }}
          api_key: ${{ secrets.OCI_API_KEY }}

      - name: Setup Packer
        uses: hashicorp/setup-packer@main
        with:
          version: ${{ env.PACKER_VERSION }}

      - name: Initialize Packer
        run: packer init v3/packer/oci/

      - name: Build OCI Image
        run: |
          packer build \
            -var "compartment_ocid=${{ secrets.OCI_COMPARTMENT_OCID }}" \
            -var "sindri_version=${{ github.event.inputs.sindri_version || 'latest' }}" \
            -var "profile=${{ github.event.inputs.profile || 'base' }}" \
            v3/packer/oci/

      - name: Upload Manifest
        uses: actions/upload-artifact@v4
        with:
          name: oci-manifest
          path: manifest.json

  build-alibaba:
    name: Build Alibaba Cloud Image
    needs: validate
    if: contains(github.event.inputs.clouds, 'alibaba')
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Configure Alibaba Cloud
        uses: aliyun/configure-aliyun-credentials-action@v1
        with:
          access-key-id: ${{ secrets.ALICLOUD_ACCESS_KEY }}
          access-key-secret: ${{ secrets.ALICLOUD_SECRET_KEY }}
          region-id: cn-hangzhou

      - name: Setup Packer
        uses: hashicorp/setup-packer@main
        with:
          version: ${{ env.PACKER_VERSION }}

      - name: Initialize Packer
        run: packer init v3/packer/alibaba/

      - name: Build Alibaba Image
        env:
          ALICLOUD_ACCESS_KEY: ${{ secrets.ALICLOUD_ACCESS_KEY }}
          ALICLOUD_SECRET_KEY: ${{ secrets.ALICLOUD_SECRET_KEY }}
        run: |
          packer build \
            -var "sindri_version=${{ github.event.inputs.sindri_version || 'latest' }}" \
            -var "profile=${{ github.event.inputs.profile || 'base' }}" \
            v3/packer/alibaba/

      - name: Upload Manifest
        uses: actions/upload-artifact@v4
        with:
          name: alibaba-manifest
          path: manifest.json

  aggregate-results:
    name: Aggregate Build Results
    needs: [build-aws, build-azure, build-gcp, build-oci, build-alibaba]
    if: always()
    runs-on: ubuntu-latest
    steps:
      - name: Download All Manifests
        uses: actions/download-artifact@v4
        with:
          path: manifests/

      - name: Create Summary
        run: |
          echo "## Packer Build Results" >> $GITHUB_STEP_SUMMARY
          echo "" >> $GITHUB_STEP_SUMMARY
          for manifest in manifests/*/manifest.json; do
            if [ -f "$manifest" ]; then
              cloud=$(dirname "$manifest" | xargs basename | cut -d'-' -f1)
              echo "### $cloud" >> $GITHUB_STEP_SUMMARY
              cat "$manifest" | jq -r '.builds[] | "- Image: \(.artifact_id)\n- Build Time: \(.build_time)"' >> $GITHUB_STEP_SUMMARY
              echo "" >> $GITHUB_STEP_SUMMARY
            fi
          done
```

---

## Security Architecture

### Credential Management

```yaml
# Secrets required per cloud provider
secrets:
  aws:
    - AWS_ACCESS_KEY_ID
    - AWS_SECRET_ACCESS_KEY
    - AWS_SESSION_TOKEN (optional, for assumed roles)

  azure:
    - AZURE_CLIENT_ID
    - AZURE_CLIENT_SECRET
    - AZURE_SUBSCRIPTION_ID
    - AZURE_TENANT_ID

  gcp:
    - GCP_SA_KEY (service account JSON)
    - GCP_PROJECT_ID

  oci:
    - OCI_USER_OCID
    - OCI_FINGERPRINT
    - OCI_TENANCY_OCID
    - OCI_REGION
    - OCI_API_KEY (private key)
    - OCI_COMPARTMENT_OCID

  alibaba:
    - ALICLOUD_ACCESS_KEY
    - ALICLOUD_SECRET_KEY
```

### CIS Hardening Script

```bash
#!/bin/bash
# scripts/security-hardening.sh

set -euo pipefail

echo "=== CIS Benchmark Hardening ==="

# 1. Filesystem Configuration
echo "Configuring filesystem..."
# Disable unused filesystems
cat >> /etc/modprobe.d/cis.conf << EOF
install cramfs /bin/true
install freevxfs /bin/true
install jffs2 /bin/true
install hfs /bin/true
install hfsplus /bin/true
install squashfs /bin/true
install udf /bin/true
EOF

# 2. SSH Hardening
echo "Hardening SSH..."
sed -i 's/#PermitRootLogin.*/PermitRootLogin no/' /etc/ssh/sshd_config
sed -i 's/#PasswordAuthentication.*/PasswordAuthentication no/' /etc/ssh/sshd_config
sed -i 's/X11Forwarding yes/X11Forwarding no/' /etc/ssh/sshd_config
echo "MaxAuthTries 4" >> /etc/ssh/sshd_config
echo "ClientAliveInterval 300" >> /etc/ssh/sshd_config
echo "ClientAliveCountMax 3" >> /etc/ssh/sshd_config

# 3. Network Configuration
echo "Hardening network..."
cat >> /etc/sysctl.d/99-cis.conf << EOF
# IP Forwarding
net.ipv4.ip_forward = 0
net.ipv6.conf.all.forwarding = 0

# ICMP Redirects
net.ipv4.conf.all.send_redirects = 0
net.ipv4.conf.default.send_redirects = 0
net.ipv4.conf.all.accept_redirects = 0
net.ipv4.conf.default.accept_redirects = 0
net.ipv6.conf.all.accept_redirects = 0
net.ipv6.conf.default.accept_redirects = 0

# Source Routing
net.ipv4.conf.all.accept_source_route = 0
net.ipv4.conf.default.accept_source_route = 0
net.ipv6.conf.all.accept_source_route = 0
net.ipv6.conf.default.accept_source_route = 0

# Log Suspicious Packets
net.ipv4.conf.all.log_martians = 1
net.ipv4.conf.default.log_martians = 1

# TCP SYN Cookies
net.ipv4.tcp_syncookies = 1
EOF

sysctl --system

# 4. Audit Configuration
echo "Configuring auditd..."
apt-get install -y auditd audispd-plugins
systemctl enable auditd

# 5. Firewall (UFW)
echo "Configuring firewall..."
apt-get install -y ufw
ufw default deny incoming
ufw default allow outgoing
ufw allow ssh
ufw --force enable

# 6. Remove unnecessary packages
echo "Removing unnecessary packages..."
apt-get purge -y telnet rsh-client rsh-redone-client

# 7. Set permissions on sensitive files
echo "Setting file permissions..."
chmod 600 /etc/shadow
chmod 600 /etc/gshadow
chmod 644 /etc/passwd
chmod 644 /etc/group

# 8. Configure login banners
cat > /etc/issue << EOF
***************************************************************************
                         AUTHORIZED USE ONLY
***************************************************************************
This system is for authorized use only. Unauthorized access is prohibited.
All activities may be monitored and recorded.
***************************************************************************
EOF

cat > /etc/issue.net << EOF
***************************************************************************
                         AUTHORIZED USE ONLY
***************************************************************************
EOF

echo "=== CIS Hardening Complete ==="
```

---

## Testing Strategy

### Unit Tests

```rust
// crates/sindri-packer/src/tests/aws_tests.rs

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_aws_template_generation() {
        let config = PackerConfig {
            image_name: "test-sindri".to_string(),
            sindri_version: "3.0.0".to_string(),
            clouds: CloudConfigs {
                aws: Some(AwsConfig {
                    regions: vec!["us-west-2".to_string()],
                    instance_type: "t3.large".to_string(),
                    volume_size: 60,
                    volume_type: "gp3".to_string(),
                    encrypt_boot: true,
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        };

        let provider = AwsPackerProvider::new();
        let template = provider.generate_template(&config).unwrap();

        assert!(template.contains("amazon-ebs"));
        assert!(template.contains("test-sindri"));
        assert!(template.contains("us-west-2"));
        assert!(template.contains("t3.large"));
        assert!(template.contains("encrypted = true"));
    }

    #[test]
    fn test_aws_prerequisites() {
        let provider = AwsPackerProvider::new();
        let prereqs = provider.check_prerequisites().unwrap();

        // May or may not be satisfied depending on environment
        assert!(prereqs.available.contains(&"packer".to_string()));
        assert!(prereqs.available.contains(&"aws-cli".to_string()));
    }

    #[tokio::test]
    async fn test_ami_id_extraction() {
        let provider = AwsPackerProvider::new();
        let output = r#"
==> amazon-ebs.sindri: Creating AMI from instance i-1234567890abcdef0
    amazon-ebs.sindri: AMI: ami-0123456789abcdef0
==> amazon-ebs.sindri: Waiting for AMI to become ready...
        "#;

        let ami_id = provider.extract_ami_id(output).unwrap();
        assert_eq!(ami_id, "ami-0123456789abcdef0");
    }
}
```

### Integration Tests

```rust
// crates/sindri-packer/tests/integration_tests.rs

#[cfg(test)]
mod integration_tests {
    use sindri_packer::*;
    use std::env;

    /// Only run when AWS credentials are available
    #[tokio::test]
    #[ignore]
    async fn test_aws_build_dry_run() {
        if env::var("AWS_ACCESS_KEY_ID").is_err() {
            return;
        }

        let config = PackerConfig {
            image_name: "integration-test-sindri".to_string(),
            sindri_version: "3.0.0".to_string(),
            clouds: CloudConfigs {
                aws: Some(AwsConfig::default()),
                ..Default::default()
            },
            ..Default::default()
        };

        let provider = create_packer_provider(CloudProvider::Aws).unwrap();
        let validation = provider.validate_template(&config).await.unwrap();

        assert!(validation.valid, "Template validation failed: {:?}", validation.errors);
    }

    #[tokio::test]
    #[ignore]
    async fn test_multi_cloud_validation() {
        let config = PackerConfig::default();
        let clouds = vec![
            CloudProvider::Aws,
            CloudProvider::Azure,
            CloudProvider::Gcp,
        ];

        for cloud in clouds {
            let provider = create_packer_provider(cloud).unwrap();
            let result = provider.validate_template(&config).await;
            assert!(result.is_ok(), "Validation failed for {:?}", cloud);
        }
    }
}
```

### InSpec Tests for Built Images

```ruby
# test/integration/sindri/controls/sindri.rb

title 'Sindri VM Image Validation'

control 'sindri-1.0' do
  impact 1.0
  title 'Sindri Installation'
  desc 'Verify Sindri is properly installed'

  describe command('sindri --version') do
    its('exit_status') { should eq 0 }
    its('stdout') { should match /sindri \d+\.\d+\.\d+/ }
  end

  describe file('/usr/local/bin/sindri') do
    it { should exist }
    it { should be_executable }
  end

  describe directory('/home/ubuntu/.sindri') do
    it { should exist }
    its('owner') { should eq 'ubuntu' }
  end
end

control 'sindri-1.1' do
  impact 0.7
  title 'Extension Installation'
  desc 'Verify extensions are installed'

  describe command('sindri extension list --json') do
    its('exit_status') { should eq 0 }
  end
end

control 'sindri-2.0' do
  impact 1.0
  title 'SSH Configuration'
  desc 'Verify SSH is properly configured'

  describe sshd_config do
    its('PermitRootLogin') { should eq 'no' }
    its('PasswordAuthentication') { should eq 'no' }
  end

  describe service('ssh') do
    it { should be_enabled }
    it { should be_running }
  end
end

control 'sindri-3.0' do
  impact 0.9
  title 'CIS Hardening'
  desc 'Verify CIS benchmark compliance'

  describe kernel_parameter('net.ipv4.ip_forward') do
    its('value') { should eq 0 }
  end

  describe kernel_parameter('net.ipv4.tcp_syncookies') do
    its('value') { should eq 1 }
  end

  describe service('ufw') do
    it { should be_enabled }
    it { should be_running }
  end
end

control 'sindri-4.0' do
  impact 0.8
  title 'Development Tools'
  desc 'Verify development tools are installed'

  %w[git docker curl wget].each do |pkg|
    describe command("which #{pkg}") do
      its('exit_status') { should eq 0 }
    end
  end

  describe docker do
    it { should exist }
    it { should be_running }
  end
end
```

---

## Implementation Phases

### Phase 1: Core Infrastructure (Week 1-2)

**Deliverables:**

1. `sindri-packer` crate structure
2. `PackerProvider` trait definition
3. Configuration types in `sindri-core`
4. Template registry infrastructure

**Files:**

- `v3/crates/sindri-packer/Cargo.toml`
- `v3/crates/sindri-packer/src/lib.rs`
- `v3/crates/sindri-packer/src/traits.rs`
- `v3/crates/sindri-core/src/types/packer_config.rs`

### Phase 2: AWS Provider (Week 2-3)

**Deliverables:**

1. AWS Packer provider implementation
2. HCL2 template for AWS AMI
3. Provisioning scripts
4. Unit tests

**Files:**

- `v3/crates/sindri-packer/src/aws.rs`
- `v3/crates/sindri-packer/src/templates/aws.pkr.hcl.tera`
- `v3/packer/scripts/install-sindri.sh`
- `v3/crates/sindri-packer/src/tests/aws_tests.rs`

### Phase 3: Azure Provider (Week 3-4)

**Deliverables:**

1. Azure Packer provider implementation
2. HCL2 template for Azure managed images
3. Shared Image Gallery support
4. Unit tests

**Files:**

- `v3/crates/sindri-packer/src/azure.rs`
- `v3/crates/sindri-packer/src/templates/azure.pkr.hcl.tera`

### Phase 4: GCP Provider (Week 4-5)

**Deliverables:**

1. GCP Packer provider implementation
2. HCL2 template for Compute Engine images
3. Image family support
4. Unit tests

**Files:**

- `v3/crates/sindri-packer/src/gcp.rs`
- `v3/crates/sindri-packer/src/templates/gcp.pkr.hcl.tera`

### Phase 5: OCI & Alibaba Providers (Week 5-6)

**Deliverables:**

1. OCI Packer provider implementation
2. Alibaba Cloud Packer provider implementation
3. HCL2 templates for both
4. Unit tests

**Files:**

- `v3/crates/sindri-packer/src/oci.rs`
- `v3/crates/sindri-packer/src/alibaba.rs`
- `v3/crates/sindri-packer/src/templates/oci.pkr.hcl.tera`
- `v3/crates/sindri-packer/src/templates/alibaba.pkr.hcl.tera`

### Phase 6: CLI Integration (Week 6-7)

**Deliverables:**

1. Integrate `packer` provider with `sindri deploy` command
2. Add packer-specific flags (`--rebuild`, `--build-only`, `--list-images`)
3. `sindri init --provider packer --cloud <target>` template generation
4. Per-cloud documentation with citations

**Files:**

- `v3/crates/sindri/src/commands/deploy.rs` (extend for packer)
- `v3/crates/sindri/src/commands/init.rs` (add packer templates)
- `v3/crates/sindri/src/templates/packer/` (per-cloud YAML templates)
- `v3/docs/providers/packer/README.md`
- `v3/docs/providers/packer/aws.md`
- `v3/docs/providers/packer/azure.md`
- `v3/docs/providers/packer/gcp.md`
- `v3/docs/providers/packer/oci.md`
- `v3/docs/providers/packer/alibaba.md`

### Phase 7: GitHub Actions & CI/CD (Week 7-8)

**Deliverables:**

1. Reusable Packer build workflow
2. Multi-cloud build pipeline
3. InSpec test integration
4. HCP Packer integration (optional)

**Files:**

- `.github/workflows/packer-build.yml`
- `.github/workflows/packer-test.yml`
- `v3/test/integration/sindri/`

### Phase 8: Security & Hardening (Week 8-9)

**Deliverables:**

1. CIS benchmark hardening scripts
2. OpenSCAP integration
3. Security documentation
4. Compliance reports

**Files:**

- `v3/packer/scripts/security-hardening.sh`
- `v3/packer/scripts/openscap-scan.sh`
- `v3/docs/SECURITY.md`

---

## File Structure

```
v3/
├── crates/
│   ├── sindri-packer/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── traits.rs
│   │   │   ├── aws.rs
│   │   │   ├── azure.rs
│   │   │   ├── gcp.rs
│   │   │   ├── oci.rs
│   │   │   ├── alibaba.rs
│   │   │   ├── utils.rs
│   │   │   ├── templates/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── context.rs
│   │   │   │   ├── aws.pkr.hcl.tera
│   │   │   │   ├── azure.pkr.hcl.tera
│   │   │   │   ├── gcp.pkr.hcl.tera
│   │   │   │   ├── oci.pkr.hcl.tera
│   │   │   │   ├── alibaba.pkr.hcl.tera
│   │   │   │   └── common/
│   │   │   │       ├── variables.pkr.hcl.tera
│   │   │   │       ├── provisioners.pkr.hcl.tera
│   │   │   │       └── post-processors.pkr.hcl.tera
│   │   │   └── tests/
│   │   │       ├── mod.rs
│   │   │       ├── aws_tests.rs
│   │   │       ├── azure_tests.rs
│   │   │       ├── gcp_tests.rs
│   │   │       ├── oci_tests.rs
│   │   │       └── alibaba_tests.rs
│   │   └── tests/
│   │       └── integration_tests.rs
│   │
│   ├── sindri-core/
│   │   └── src/types/
│   │       └── packer_config.rs  # New file
│   │
│   └── sindri/
│       └── src/commands/
│           └── packer.rs  # New file
│
├── packer/
│   ├── scripts/
│   │   ├── init.sh
│   │   ├── install-sindri.sh
│   │   ├── install-extensions.sh
│   │   ├── security-hardening.sh
│   │   ├── openscap-scan.sh
│   │   └── cleanup.sh
│   ├── aws/
│   │   ├── sindri.pkr.hcl
│   │   └── variables.pkrvars.hcl
│   ├── azure/
│   │   ├── sindri.pkr.hcl
│   │   └── variables.pkrvars.hcl
│   ├── gcp/
│   │   ├── sindri.pkr.hcl
│   │   └── variables.pkrvars.hcl
│   ├── oci/
│   │   ├── sindri.pkr.hcl
│   │   └── variables.pkrvars.hcl
│   └── alibaba/
│       ├── sindri.pkr.hcl
│       └── variables.pkrvars.hcl
│
├── test/
│   └── integration/
│       └── sindri/
│           ├── inspec.yml
│           └── controls/
│               ├── sindri.rb
│               ├── security.rb
│               └── compliance.rb
│
├── docs/
│   ├── providers/
│   │   └── packer/
│   │       ├── README.md              # Overview and quick start
│   │       ├── aws.md                 # AWS-specific reference
│   │       ├── azure.md               # Azure-specific reference
│   │       ├── gcp.md                 # GCP-specific reference
│   │       ├── oci.md                 # Oracle Cloud-specific reference
│   │       ├── alibaba.md             # Alibaba Cloud-specific reference
│   │       ├── image-building.md      # Image build workflow
│   │       ├── security.md            # Security hardening guide
│   │       └── troubleshooting.md     # Common issues
│   └── planning/
│       └── active/
│           └── packer-vm-provisioning-architecture.md  # This document
│
└── schemas/
    └── packer-config.schema.json
```

---

## Configuration Schema

### Unified Packer Provider Design

The `packer` provider follows the same pattern as other Sindri providers (Docker, Fly.io, DevPod) but adds a `cloud` attribute to specify the target cloud platform. This keeps the CLI experience consistent while supporting multi-cloud VM deployments.

**Key Configuration Options:**

| Option     | Required | Description                                           |
| ---------- | -------- | ----------------------------------------------------- |
| `cloud`    | Yes      | Target cloud: `aws`, `azure`, `gcp`, `oci`, `alibaba` |
| `image_id` | No       | Use existing image (skip build)                       |
| `build.*`  | No       | Image build settings (when `image_id` not set)        |

### sindri.yaml Schema

```yaml
# sindri.yaml
name: my-sindri-env
version: "3.0.0"
provider: packer # Unified packer provider

packer:
  # Target cloud platform (required)
  # Options: aws, azure, gcp, oci, alibaba
  cloud: aws

  # Cloud-specific settings (vary by cloud target)
  region: us-west-2
  instance_type: t3.large
  volume_size: 80

  # === IMAGE SOURCE (choose one) ===

  # Option A: Use a pre-built image (skip build entirely)
  # image_id: ami-0123456789abcdef0

  # Option B: Build image with these settings (used when image_id is NOT set)
  build:
    # Extensions to bake into the image
    extensions:
      - python
      - node
      - rust
      - claude-code

    # Extension profile to install
    profile: ai-dev

    # Cache behavior: reuse existing image if config matches
    cache: true

    # Image naming prefix for identification
    name_prefix: sindri-dev

    # Security hardening options
    security:
      cis_hardening: true
      clean_sensitive_data: true
```

### Image Source Logic

| `image_id` | `build.cache` | Behavior                                                  |
| ---------- | ------------- | --------------------------------------------------------- |
| Set        | -             | Use specified image directly, no build                    |
| Not set    | `true`        | Look for cached image matching config, build if not found |
| Not set    | `false`       | Always build fresh image                                  |

---

## CLI Commands

### Consistent Deploy Workflow

The `packer` provider uses the **same CLI commands** as all other Sindri providers, maintaining a consistent user experience:

```
# Standard deployment workflow (same as docker, fly, devpod)
sindri deploy              # Build image (if needed) + deploy VM
sindri connect             # SSH into the running VM
sindri status              # Show VM status
sindri stop                # Stop VM (for cost savings)
sindri start               # Resume stopped VM
sindri destroy             # Terminate VM

# Packer-specific flags for image management
sindri deploy --rebuild    # Force fresh image build before deploy
sindri deploy --build-only # Only build image, don't deploy
sindri deploy --list-images # List available images for current cloud
```

### Usage Examples

```bash
# Deploy to AWS (builds image automatically if needed)
sindri deploy

# Deploy to a different cloud (override config)
sindri deploy --set packer.cloud=azure

# Use a specific pre-built image
sindri deploy --set packer.image_id=ami-0123456789abcdef0

# Force rebuild the image before deploying
sindri deploy --rebuild

# Build image only (for CI/CD pipelines)
sindri deploy --build-only
# Output: Built image: ami-0abc123... (save this for production config)

# List available images
sindri deploy --list-images

# Delete an old image
sindri deploy --delete-image ami-old123
```

### Command Implementation

```rust
// crates/sindri/src/commands/deploy.rs (extended for packer provider)

use sindri_packer::{PackerProvider, BuildOptions};

impl DeployCommand {
    async fn run_packer_deploy(&self, config: &SindriConfig) -> Result<DeployResult> {
        let packer_config = config.packer.as_ref()
            .ok_or_else(|| anyhow!("Packer configuration required"))?;

        let provider = PackerProvider::new(packer_config.cloud)?;

        // Check for existing image or build new one
        let image_id = if let Some(id) = &packer_config.image_id {
            // Use specified image directly
            info!("Using pre-built image: {}", id);
            id.clone()
        } else if self.rebuild {
            // Force rebuild
            info!("Building fresh image...");
            let result = provider.build_image(packer_config, BuildOptions::default()).await?;
            result.image_id
        } else if packer_config.build.cache {
            // Try to find cached image, build if not found
            match provider.find_cached_image(packer_config).await? {
                Some(id) => {
                    info!("Using cached image: {}", id);
                    id
                }
                None => {
                    info!("No cached image found, building...");
                    let result = provider.build_image(packer_config, BuildOptions::default()).await?;
                    result.image_id
                }
            }
        } else {
            // Always build
            info!("Building image...");
            let result = provider.build_image(packer_config, BuildOptions::default()).await?;
            result.image_id
        };

        // Deploy VM from image
        provider.deploy_from_image(&image_id, packer_config).await
    }
}
```

---

## Generated YAML Templates

When users run `sindri init --provider packer --cloud <target>`, Sindri generates a fully documented `sindri.yaml` with inline comments and citations to official documentation. Each cloud target has its own template with cloud-specific guidance.

### AWS Template (Generated)

```yaml
# sindri.yaml - AWS Packer Configuration
# Generated by: sindri init --provider packer --cloud aws
# Documentation: https://sindri.dev/docs/providers/packer/aws

name: my-dev-environment
version: "3.0.0"
provider: packer

packer:
  # ═══════════════════════════════════════════════════════════════════════════
  # TARGET CLOUD PLATFORM
  # ═══════════════════════════════════════════════════════════════════════════
  cloud: aws

  # ═══════════════════════════════════════════════════════════════════════════
  # AWS REGION
  # Where to build and deploy the VM instance.
  # Ref: https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/using-regions-availability-zones.html
  # Common regions: us-east-1 (N. Virginia), us-west-2 (Oregon), eu-west-1 (Ireland)
  # ═══════════════════════════════════════════════════════════════════════════
  region: us-west-2

  # ═══════════════════════════════════════════════════════════════════════════
  # EC2 INSTANCE TYPE
  # Determines CPU, memory, and network performance.
  # Ref: https://aws.amazon.com/ec2/instance-types/
  #
  # Recommended for development:
  #   t3.medium  - 2 vCPU, 4 GB RAM   (~$0.042/hr) - Light development
  #   t3.large   - 2 vCPU, 8 GB RAM   (~$0.083/hr) - Standard development
  #   t3.xlarge  - 4 vCPU, 16 GB RAM  (~$0.166/hr) - Heavy workloads
  #   m6i.xlarge - 4 vCPU, 16 GB RAM  (~$0.192/hr) - Consistent performance
  #
  # For GPU workloads (AI/ML):
  #   g4dn.xlarge - 4 vCPU, 16 GB RAM, 1 T4 GPU (~$0.526/hr)
  #   p3.2xlarge  - 8 vCPU, 61 GB RAM, 1 V100 GPU (~$3.06/hr)
  # ═══════════════════════════════════════════════════════════════════════════
  instance_type: t3.large

  # ═══════════════════════════════════════════════════════════════════════════
  # EBS VOLUME SIZE (GB)
  # Root volume size for the VM. Sindri base image requires minimum 40GB.
  # Ref: https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/ebs-volume-types.html
  # ═══════════════════════════════════════════════════════════════════════════
  volume_size: 80

  # ═══════════════════════════════════════════════════════════════════════════
  # EBS VOLUME TYPE
  # Options: gp3 (recommended), gp2, io1, io2
  # Ref: https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/ebs-volume-types.html
  #   gp3 - General Purpose SSD, 3000 IOPS baseline, best price/performance
  #   gp2 - General Purpose SSD, IOPS scales with size
  #   io1/io2 - Provisioned IOPS, for high-performance workloads
  # ═══════════════════════════════════════════════════════════════════════════
  volume_type: gp3

  # ═══════════════════════════════════════════════════════════════════════════
  # BOOT VOLUME ENCRYPTION
  # Encrypt the EBS root volume using AWS KMS.
  # Ref: https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/EBSEncryption.html
  # Recommended: true for compliance (HIPAA, SOC2, etc.)
  # ═══════════════════════════════════════════════════════════════════════════
  encrypt_boot: true

  # ═══════════════════════════════════════════════════════════════════════════
  # IMAGE SOURCE
  # Choose ONE of the following options:
  # ═══════════════════════════════════════════════════════════════════════════

  # Option A: Use an existing AMI (skip build, fastest deployment)
  # Find AMIs: aws ec2 describe-images --owners self --query 'Images[*].[ImageId,Name]'
  # image_id: ami-0123456789abcdef0

  # Option B: Build a new image with these settings
  build:
    # Extensions to pre-install in the image
    # Available: python, node, rust, go, java, claude-code, aider, cursor
    extensions:
      - python
      - node
      - rust

    # Extension profile (predefined extension sets)
    # Options: base, web-dev, ai-dev, data-science, devops
    profile: ai-dev

    # Reuse existing image if configuration matches (recommended)
    # Set to false to always build fresh
    cache: true

    # Prefix for AMI names (for identification in AWS Console)
    name_prefix: sindri-dev

    # Security hardening during image build
    security:
      # Apply CIS Benchmark hardening
      # Ref: https://www.cisecurity.org/benchmark/amazon_linux
      cis_hardening: true

      # Remove sensitive data before image capture
      clean_sensitive_data: true

  # ═══════════════════════════════════════════════════════════════════════════
  # OPTIONAL: VPC CONFIGURATION
  # Required if deploying to a private subnet or custom VPC.
  # Ref: https://docs.aws.amazon.com/vpc/latest/userguide/what-is-amazon-vpc.html
  # ═══════════════════════════════════════════════════════════════════════════
  # vpc_id: vpc-0123456789abcdef0
  # subnet_id: subnet-0123456789abcdef0

  # ═══════════════════════════════════════════════════════════════════════════
  # OPTIONAL: RESOURCE TAGS
  # Applied to all AWS resources (instance, volumes, AMI).
  # Ref: https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/Using_Tags.html
  # ═══════════════════════════════════════════════════════════════════════════
  tags:
    Environment: development
    Project: my-project
    ManagedBy: sindri
```

### Azure Template (Generated)

```yaml
# sindri.yaml - Azure Packer Configuration
# Generated by: sindri init --provider packer --cloud azure
# Documentation: https://sindri.dev/docs/providers/packer/azure

name: my-dev-environment
version: "3.0.0"
provider: packer

packer:
  cloud: azure

  # ═══════════════════════════════════════════════════════════════════════════
  # AZURE SUBSCRIPTION
  # Your Azure subscription ID. Find it in Azure Portal or with:
  #   az account show --query id -o tsv
  # Ref: https://learn.microsoft.com/en-us/azure/azure-portal/get-subscription-tenant-id
  # ═══════════════════════════════════════════════════════════════════════════
  subscription_id: "${AZURE_SUBSCRIPTION_ID}"

  # ═══════════════════════════════════════════════════════════════════════════
  # RESOURCE GROUP
  # Azure resource group for the VM and related resources.
  # Create with: az group create --name sindri-rg --location westus2
  # Ref: https://learn.microsoft.com/en-us/azure/azure-resource-manager/management/manage-resource-groups-portal
  # ═══════════════════════════════════════════════════════════════════════════
  resource_group: sindri-rg

  # ═══════════════════════════════════════════════════════════════════════════
  # AZURE REGION
  # Ref: https://azure.microsoft.com/en-us/explore/global-infrastructure/geographies/
  # Common: eastus, westus2, westeurope, northeurope, southeastasia
  # ═══════════════════════════════════════════════════════════════════════════
  location: westus2

  # ═══════════════════════════════════════════════════════════════════════════
  # VM SIZE
  # Determines CPU, memory, and pricing.
  # Ref: https://learn.microsoft.com/en-us/azure/virtual-machines/sizes
  #
  # General purpose:
  #   Standard_D2s_v4  - 2 vCPU, 8 GB RAM   (~$0.096/hr)
  #   Standard_D4s_v4  - 4 vCPU, 16 GB RAM  (~$0.192/hr)
  #   Standard_D8s_v4  - 8 vCPU, 32 GB RAM  (~$0.384/hr)
  #
  # GPU (AI/ML):
  #   Standard_NC6s_v3 - 6 vCPU, 112 GB RAM, 1 V100 GPU (~$3.06/hr)
  # ═══════════════════════════════════════════════════════════════════════════
  vm_size: Standard_D4s_v4

  # ═══════════════════════════════════════════════════════════════════════════
  # OS DISK SIZE (GB)
  # Ref: https://learn.microsoft.com/en-us/azure/virtual-machines/managed-disks-overview
  # ═══════════════════════════════════════════════════════════════════════════
  os_disk_size_gb: 80

  # ═══════════════════════════════════════════════════════════════════════════
  # STORAGE ACCOUNT TYPE
  # Ref: https://learn.microsoft.com/en-us/azure/virtual-machines/disks-types
  # Options:
  #   Premium_LRS   - Premium SSD, locally redundant (recommended)
  #   StandardSSD_LRS - Standard SSD
  #   Standard_LRS  - Standard HDD (cheapest, slowest)
  # ═══════════════════════════════════════════════════════════════════════════
  storage_account_type: Premium_LRS

  # ═══════════════════════════════════════════════════════════════════════════
  # IMAGE SOURCE
  # ═══════════════════════════════════════════════════════════════════════════

  # Option A: Use existing image
  # image_id: /subscriptions/.../resourceGroups/.../providers/Microsoft.Compute/images/sindri-dev

  # Option B: Build settings
  build:
    extensions:
      - python
      - node
    profile: ai-dev
    cache: true
    name_prefix: sindri-dev
    security:
      cis_hardening: true
      clean_sensitive_data: true

  # ═══════════════════════════════════════════════════════════════════════════
  # OPTIONAL: SHARED IMAGE GALLERY
  # Distribute images across regions for faster deployment.
  # Ref: https://learn.microsoft.com/en-us/azure/virtual-machines/shared-image-galleries
  # ═══════════════════════════════════════════════════════════════════════════
  # gallery:
  #   gallery_name: sindri_gallery
  #   image_name: sindri-dev
  #   image_version: "1.0.0"
  #   replication_regions:
  #     - westus2
  #     - eastus
```

### GCP Template (Generated)

```yaml
# sindri.yaml - GCP Packer Configuration
# Generated by: sindri init --provider packer --cloud gcp
# Documentation: https://sindri.dev/docs/providers/packer/gcp

name: my-dev-environment
version: "3.0.0"
provider: packer

packer:
  cloud: gcp

  # ═══════════════════════════════════════════════════════════════════════════
  # GCP PROJECT ID
  # Your GCP project ID (not the project number).
  # Find with: gcloud config get-value project
  # Ref: https://cloud.google.com/resource-manager/docs/creating-managing-projects
  # ═══════════════════════════════════════════════════════════════════════════
  project_id: "${GCP_PROJECT_ID}"

  # ═══════════════════════════════════════════════════════════════════════════
  # ZONE
  # GCP zone for the VM. Format: region-zone (e.g., us-west1-a)
  # Ref: https://cloud.google.com/compute/docs/regions-zones
  # ═══════════════════════════════════════════════════════════════════════════
  zone: us-west1-a

  # ═══════════════════════════════════════════════════════════════════════════
  # MACHINE TYPE
  # Ref: https://cloud.google.com/compute/docs/machine-resource
  #
  # General purpose (E2 - cost-optimized):
  #   e2-medium   - 2 vCPU, 4 GB RAM   (~$0.034/hr)
  #   e2-standard-4 - 4 vCPU, 16 GB RAM (~$0.134/hr)
  #
  # General purpose (N2 - balanced):
  #   n2-standard-4 - 4 vCPU, 16 GB RAM (~$0.194/hr)
  #   n2-standard-8 - 8 vCPU, 32 GB RAM (~$0.388/hr)
  #
  # GPU (AI/ML):
  #   n1-standard-4 + nvidia-tesla-t4 (~$0.35/hr + $0.35/hr GPU)
  # ═══════════════════════════════════════════════════════════════════════════
  machine_type: e2-standard-4

  # ═══════════════════════════════════════════════════════════════════════════
  # DISK SIZE (GB)
  # Ref: https://cloud.google.com/compute/docs/disks
  # ═══════════════════════════════════════════════════════════════════════════
  disk_size: 80

  # ═══════════════════════════════════════════════════════════════════════════
  # DISK TYPE
  # Ref: https://cloud.google.com/compute/docs/disks#disk-types
  # Options:
  #   pd-ssd      - SSD persistent disk (recommended)
  #   pd-balanced - Balanced persistent disk
  #   pd-standard - Standard persistent disk (HDD)
  # ═══════════════════════════════════════════════════════════════════════════
  disk_type: pd-ssd

  # ═══════════════════════════════════════════════════════════════════════════
  # SHIELDED VM / SECURE BOOT
  # Ref: https://cloud.google.com/compute/shielded-vm/docs/shielded-vm
  # ═══════════════════════════════════════════════════════════════════════════
  enable_secure_boot: true

  # ═══════════════════════════════════════════════════════════════════════════
  # IMAGE SOURCE
  # ═══════════════════════════════════════════════════════════════════════════

  # Option A: Use existing image
  # image_id: projects/my-project/global/images/sindri-dev-v1

  # Option B: Build settings
  build:
    extensions:
      - python
      - node
    profile: ai-dev
    cache: true
    name_prefix: sindri-dev

    # Image family for automatic latest version resolution
    # Ref: https://cloud.google.com/compute/docs/images/image-families-best-practices
    image_family: sindri-dev

    security:
      cis_hardening: true
      clean_sensitive_data: true

  # ═══════════════════════════════════════════════════════════════════════════
  # OPTIONAL: NETWORK CONFIGURATION
  # Ref: https://cloud.google.com/vpc/docs/vpc
  # ═══════════════════════════════════════════════════════════════════════════
  # network: projects/my-project/global/networks/my-vpc
  # subnetwork: projects/my-project/regions/us-west1/subnetworks/my-subnet
```

### OCI Template (Generated)

```yaml
# sindri.yaml - Oracle Cloud Infrastructure Packer Configuration
# Generated by: sindri init --provider packer --cloud oci
# Documentation: https://sindri.dev/docs/providers/packer/oci

name: my-dev-environment
version: "3.0.0"
provider: packer

packer:
  cloud: oci

  # ═══════════════════════════════════════════════════════════════════════════
  # COMPARTMENT OCID
  # The OCID of the compartment where resources will be created.
  # Find with: oci iam compartment list --query 'data[*].{name:name,id:id}'
  # Ref: https://docs.oracle.com/en-us/iaas/Content/Identity/Tasks/managingcompartments.htm
  # ═══════════════════════════════════════════════════════════════════════════
  compartment_ocid: "${OCI_COMPARTMENT_OCID}"

  # ═══════════════════════════════════════════════════════════════════════════
  # AVAILABILITY DOMAIN
  # Format: <region-identifier>:<availability-domain-number>
  # List with: oci iam availability-domain list
  # Ref: https://docs.oracle.com/en-us/iaas/Content/General/Concepts/regions.htm
  # ═══════════════════════════════════════════════════════════════════════════
  availability_domain: "Uocm:US-ASHBURN-AD-1"

  # ═══════════════════════════════════════════════════════════════════════════
  # SHAPE (VM TYPE)
  # Ref: https://docs.oracle.com/en-us/iaas/Content/Compute/References/computeshapes.htm
  #
  # Flexible shapes (recommended - pay for what you use):
  #   VM.Standard.E4.Flex - AMD EPYC, configurable OCPUs
  #   VM.Standard.A1.Flex - Arm-based Ampere, best price/performance
  #
  # Fixed shapes:
  #   VM.Standard.E4.Flex (1 OCPU, 16 GB) - ~$0.025/hr
  #   VM.Standard.E4.Flex (4 OCPU, 64 GB) - ~$0.10/hr
  #
  # GPU:
  #   VM.GPU3.1 - 1 V100 GPU - ~$2.95/hr
  # ═══════════════════════════════════════════════════════════════════════════
  shape: VM.Standard.E4.Flex

  # For flexible shapes, specify OCPUs and memory
  shape_config:
    ocpus: 4
    memory_in_gbs: 64

  # ═══════════════════════════════════════════════════════════════════════════
  # SUBNET OCID
  # The subnet for the VM's VNIC.
  # List with: oci network subnet list --compartment-id $COMPARTMENT
  # Ref: https://docs.oracle.com/en-us/iaas/Content/Network/Tasks/managingVCNs.htm
  # ═══════════════════════════════════════════════════════════════════════════
  subnet_ocid: "${OCI_SUBNET_OCID}"

  # ═══════════════════════════════════════════════════════════════════════════
  # BOOT VOLUME SIZE (GB)
  # Ref: https://docs.oracle.com/en-us/iaas/Content/Block/Concepts/bootvolumes.htm
  # ═══════════════════════════════════════════════════════════════════════════
  boot_volume_size_gb: 80

  # ═══════════════════════════════════════════════════════════════════════════
  # IMAGE SOURCE
  # ═══════════════════════════════════════════════════════════════════════════

  # Option A: Use existing image
  # image_id: ocid1.image.oc1.iad.aaaa...

  # Option B: Build settings
  build:
    extensions:
      - python
      - node
    profile: ai-dev
    cache: true
    name_prefix: sindri-dev
    security:
      cis_hardening: true
      clean_sensitive_data: true
```

### Alibaba Cloud Template (Generated)

```yaml
# sindri.yaml - Alibaba Cloud Packer Configuration
# Generated by: sindri init --provider packer --cloud alibaba
# Documentation: https://sindri.dev/docs/providers/packer/alibaba

name: my-dev-environment
version: "3.0.0"
provider: packer

packer:
  cloud: alibaba

  # ═══════════════════════════════════════════════════════════════════════════
  # REGION
  # Alibaba Cloud region ID.
  # Ref: https://www.alibabacloud.com/help/en/ecs/product-overview/regions-and-zones
  # Common: cn-hangzhou, cn-shanghai, cn-beijing, us-west-1, eu-central-1
  # ═══════════════════════════════════════════════════════════════════════════
  region: cn-hangzhou

  # ═══════════════════════════════════════════════════════════════════════════
  # INSTANCE TYPE
  # Ref: https://www.alibabacloud.com/help/en/ecs/user-guide/overview-of-instance-families
  #
  # General purpose:
  #   ecs.g6.large   - 2 vCPU, 8 GB RAM
  #   ecs.g6.xlarge  - 4 vCPU, 16 GB RAM
  #   ecs.g6.2xlarge - 8 vCPU, 32 GB RAM
  #
  # GPU (AI/ML):
  #   ecs.gn6i-c4g1.xlarge - 4 vCPU, 15 GB, 1 T4 GPU
  # ═══════════════════════════════════════════════════════════════════════════
  instance_type: ecs.g6.xlarge

  # ═══════════════════════════════════════════════════════════════════════════
  # SYSTEM DISK SIZE (GB)
  # Ref: https://www.alibabacloud.com/help/en/ecs/user-guide/block-storage-overview
  # ═══════════════════════════════════════════════════════════════════════════
  system_disk_size_gb: 80

  # ═══════════════════════════════════════════════════════════════════════════
  # SYSTEM DISK CATEGORY
  # Ref: https://www.alibabacloud.com/help/en/ecs/user-guide/block-storage-overview
  # Options:
  #   cloud_essd     - Enhanced SSD (recommended)
  #   cloud_ssd      - Standard SSD
  #   cloud_efficiency - Ultra disk
  # ═══════════════════════════════════════════════════════════════════════════
  system_disk_category: cloud_essd

  # ═══════════════════════════════════════════════════════════════════════════
  # IMAGE SOURCE
  # ═══════════════════════════════════════════════════════════════════════════

  # Option A: Use existing image
  # image_id: m-bp1234567890abcdef

  # Option B: Build settings
  build:
    extensions:
      - python
      - node
    profile: ai-dev
    cache: true
    name_prefix: sindri-dev
    security:
      cis_hardening: true
      clean_sensitive_data: true

  # ═══════════════════════════════════════════════════════════════════════════
  # OPTIONAL: VSWITCH
  # For deploying in a specific VPC/VSwitch.
  # Ref: https://www.alibabacloud.com/help/en/vpc/user-guide/create-and-manage-a-vswitch
  # ═══════════════════════════════════════════════════════════════════════════
  # vswitch_id: vsw-bp1234567890abcdef
```

---

## Per-Cloud Documentation Structure

Documentation for the Packer provider is organized into individual reference guides per cloud:

```
v3/docs/providers/packer/
├── README.md           # Overview and quick start
├── aws.md              # AWS-specific reference
├── azure.md            # Azure-specific reference
├── gcp.md              # GCP-specific reference
├── oci.md              # Oracle Cloud-specific reference
├── alibaba.md          # Alibaba Cloud-specific reference
├── image-building.md   # Image build workflow details
├── security.md         # Security hardening guide
└── troubleshooting.md  # Common issues and solutions
```

Each cloud reference document includes:

1. **Prerequisites** - Required CLI tools, authentication setup
2. **Configuration Reference** - All options with descriptions and defaults
3. **Pricing Guide** - Instance type recommendations with cost estimates
4. **Networking** - VPC/subnet configuration examples
5. **Image Management** - List, share, delete operations
6. **Troubleshooting** - Cloud-specific error resolution

---

## Success Criteria

### Functional Requirements

- [ ] Build AWS AMIs with Sindri pre-installed
- [ ] Build Azure managed images with Shared Image Gallery support
- [ ] Build GCP images with image family support
- [ ] Build OCI custom images
- [ ] Build Alibaba Cloud images
- [ ] Support multi-cloud parallel builds
- [ ] Integrate with existing extension/profile system
- [ ] CIS benchmark hardening
- [ ] GitHub Actions CI/CD workflows

### Performance Requirements

- [ ] AWS AMI build time < 15 minutes
- [ ] Azure image build time < 20 minutes
- [ ] GCP image build time < 15 minutes
- [ ] Multi-cloud parallel build < 25 minutes
- [ ] Template validation < 5 seconds

### Quality Requirements

- [ ] Unit test coverage > 80%
- [ ] Integration tests for each cloud
- [ ] InSpec compliance tests passing
- [ ] Documentation complete
- [ ] No critical security vulnerabilities

---

## Risk Assessment

| Risk                        | Probability | Impact   | Mitigation                           |
| --------------------------- | ----------- | -------- | ------------------------------------ |
| Cloud API rate limits       | Medium      | Medium   | Implement exponential backoff        |
| Packer plugin compatibility | Low         | High     | Pin plugin versions                  |
| Build timeout               | Medium      | Medium   | Configurable timeouts, checkpoints   |
| Credential exposure         | Low         | Critical | Use environment variables, never log |
| Template syntax errors      | Medium      | Low      | Validation in CI, syntax tests       |
| Multi-cloud inconsistency   | Medium      | Medium   | Shared provisioning scripts          |
| Image size inflation        | Medium      | Low      | Cleanup scripts, size monitoring     |

---

## References

### HashiCorp Documentation

- [Packer HCL2 Templates](https://developer.hashicorp.com/packer/docs/templates/hcl_templates)
- [Packer AWS Builder](https://developer.hashicorp.com/packer/integrations/hashicorp/amazon)
- [Packer Azure Builder](https://developer.hashicorp.com/packer/integrations/hashicorp/azure)
- [Packer GCP Builder](https://developer.hashicorp.com/packer/integrations/hashicorp/googlecompute)
- [Packer Oracle Builder](https://developer.hashicorp.com/packer/integrations/hashicorp/oracle)
- [Packer Alibaba Builder](https://developer.hashicorp.com/packer/integrations/hashicorp/alicloud)
- [GitHub Actions for Packer](https://developer.hashicorp.com/packer/tutorials/cloud-production/github-actions)

### Clicktruck Reference Implementations

- [gha-workflows-with-gitops-for-tanzu-application-platform](https://github.com/clicktruck/gha-workflows-with-gitops-for-tanzu-application-platform)
- [aws-actions](https://github.com/clicktruck/aws-actions)
- [azure-actions](https://github.com/clicktruck/azure-actions)
- [google-actions](https://github.com/clicktruck/google-actions)

### Security Standards

- [CIS Benchmarks](https://www.cisecurity.org/cis-benchmarks)
- [OpenSCAP](https://www.open-scap.org/)
- [HashiCorp Security Best Practices](https://developer.hashicorp.com/packer/docs/security)

### Sindri v3 Architecture

- [ADR-002: Provider Abstraction Layer](../../architecture/adr/002-provider-abstraction-layer.md)
- [ADR-003: Template-Based Configuration](../../architecture/adr/003-template-based-configuration.md)
- [ADR-005: Provider-Specific Implementations](../../architecture/adr/005-provider-specific-implementations.md)

---

## Document History

| Version | Date       | Author           | Changes                                                                                                                                                                                                        |
| ------- | ---------- | ---------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1.0.0   | 2026-01-24 | Sindri Core Team | Initial architecture document                                                                                                                                                                                  |
| 1.1.0   | 2026-01-25 | Sindri Core Team | Revised to unified `packer` provider approach: single provider with `cloud` attribute, `image_id` for pre-built images, consistent `sindri deploy` CLI, generated YAML with rich inline comments and citations |

---

**Next Steps:**

1. Review and approve architecture
2. Create `sindri-packer` crate scaffold
3. Implement per-cloud YAML template generators with inline documentation
4. Begin Phase 1 implementation
5. Set up CI/CD infrastructure
