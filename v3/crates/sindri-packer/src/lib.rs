//! HashiCorp Packer provider for Sindri v3
//!
//! This crate provides a unified `packer` provider for building and deploying
//! VM images across multiple cloud platforms:
//!
//! - **AWS** - EC2 AMI images via `amazon-ebs` builder
//! - **Azure** - Managed images with Shared Image Gallery support
//! - **GCP** - Compute Engine images via `googlecompute` builder
//! - **OCI** - Oracle Cloud Infrastructure custom images
//! - **Alibaba** - Alibaba Cloud ECS custom images
//!
//! # Architecture
//!
//! The Packer provider follows Sindri's established provider abstraction pattern:
//!
//! - Implements the standard `Provider` trait for `sindri deploy/connect/status/destroy`
//! - Extends with `PackerProvider` trait for image building operations
//! - Uses HCL2 templates (Tera-rendered) for multi-cloud consistency
//! - Supports both building new images and deploying from pre-built images
//!
//! # Usage
//!
//! ```yaml
//! # sindri.yaml
//! provider: packer
//! packer:
//!   cloud: aws           # Target cloud: aws, azure, gcp, oci, alibaba
//!   region: us-west-2
//!   instance_type: t3.large
//!   build:
//!     extensions:
//!       - python
//!       - node
//!     profile: anthropic-dev
//! ```

pub mod alibaba;
pub mod aws;
pub mod azure;
pub mod gcp;
pub mod oci;
pub mod templates;
pub mod traits;
mod utils;

#[cfg(test)]
mod tests;

pub use traits::{
    BuildOptions, BuildResult, CloudPrerequisiteStatus, ImageInfo, OnErrorBehavior, PackerProvider,
    ValidationResult,
};

// Re-export utility functions
pub use utils::check_packer_installed;

use anyhow::Result;
use sindri_core::types::packer_config::{CloudProvider, PackerConfig};

/// Create a Packer provider instance for the specified cloud
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
) -> Result<Vec<Result<BuildResult>>> {
    use futures::future::join_all;

    let futures: Vec<_> = clouds
        .iter()
        .map(|cloud| {
            let config = config.clone();
            let opts = opts.clone();
            let cloud = *cloud;

            async move {
                let provider = create_packer_provider(cloud)?;
                provider.build_image(&config, opts).await
            }
        })
        .collect();

    Ok(join_all(futures).await)
}

/// Validate configurations for multiple clouds
pub async fn validate_multi_cloud(
    clouds: &[CloudProvider],
    config: &PackerConfig,
) -> Result<Vec<(CloudProvider, ValidationResult)>> {
    use futures::future::join_all;

    let futures: Vec<_> = clouds
        .iter()
        .map(|cloud| {
            let config = config.clone();
            let cloud = *cloud;

            async move {
                let provider = create_packer_provider(cloud)?;
                let result = provider.validate_template(&config).await?;
                Ok::<_, anyhow::Error>((cloud, result))
            }
        })
        .collect();

    let results = join_all(futures).await;
    results.into_iter().collect()
}

/// Check prerequisites for a specific cloud
pub fn check_prerequisites(cloud: CloudProvider) -> Result<CloudPrerequisiteStatus> {
    let provider = create_packer_provider(cloud)?;
    provider.check_cloud_prerequisites()
}

/// Check prerequisites for all supported clouds
pub fn check_all_prerequisites() -> Vec<(CloudProvider, Result<CloudPrerequisiteStatus>)> {
    let clouds = [
        CloudProvider::Aws,
        CloudProvider::Azure,
        CloudProvider::Gcp,
        CloudProvider::Oci,
        CloudProvider::Alibaba,
    ];

    clouds
        .iter()
        .map(|cloud| {
            let result = check_prerequisites(*cloud);
            (*cloud, result)
        })
        .collect()
}
