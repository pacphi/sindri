//! Provider adapters for Sindri deployments
//!
//! This crate provides the provider abstraction layer for deploying
//! Sindri environments to different platforms:
//!
//! - Docker (local development)
//! - Fly.io (cloud deployment with auto-suspend)
//! - DevPod (multi-cloud development environments)
//! - E2B (cloud sandboxes)
//! - Kubernetes (container orchestration)
//! - RunPod (GPU cloud)
//! - Northflank (Kubernetes PaaS)

pub mod devpod;
pub mod docker;
pub mod e2b;
pub mod fly;
pub mod kubernetes;
pub mod northflank;
pub mod runpod;
pub mod templates;
pub mod traits;
mod utils;

pub use traits::{Provider, ProviderFactory};

use anyhow::Result;
use sindri_core::types::Provider as ProviderType;

/// Create a provider instance by name
pub fn create_provider(provider: ProviderType) -> Result<Box<dyn Provider>> {
    match provider {
        ProviderType::Docker | ProviderType::DockerCompose => {
            Ok(Box::new(docker::DockerProvider::new()?))
        }
        ProviderType::Fly => Ok(Box::new(fly::FlyProvider::new()?)),
        ProviderType::Devpod => Ok(Box::new(devpod::DevPodProvider::new()?)),
        ProviderType::E2b => Ok(Box::new(e2b::E2bProvider::new()?)),
        ProviderType::Kubernetes => Ok(Box::new(kubernetes::KubernetesProvider::new()?)),
        ProviderType::Runpod => Ok(Box::new(runpod::RunpodProvider::new()?)),
        ProviderType::Northflank => Ok(Box::new(northflank::NorthflankProvider::new()?)),
    }
}
