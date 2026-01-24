//! Local Kubernetes cluster lifecycle management for Sindri
//!
//! This crate provides the cluster management abstraction layer for
//! creating and managing local Kubernetes clusters using:
//!
//! - **Kind** (Kubernetes IN Docker) - Full Kubernetes clusters in Docker containers
//! - **K3d** (K3s in Docker) - Lightweight K3s clusters with local registry support
//!
//! # Features
//!
//! - Create/destroy local Kubernetes clusters
//! - Multi-node cluster support (control-plane + workers)
//! - K3d local Docker registry integration
//! - Auto-detection of installed tools (kind/k3d)
//! - Cross-platform installation assistance (macOS, Linux)
//!
//! # Example
//!
//! ```ignore
//! use sindri_clusters::{create_cluster_provider, ClusterConfig, ClusterProviderType};
//!
//! // Create a provider
//! let provider = create_cluster_provider(ClusterProviderType::Kind)?;
//!
//! // Configure cluster
//! let config = ClusterConfig::new("my-cluster")
//!     .with_version("v1.35.0")
//!     .with_nodes(2);
//!
//! // Create cluster
//! let info = provider.create(&config).await?;
//! println!("Cluster created with context: {}", info.context);
//!
//! // List clusters
//! let clusters = provider.list().await?;
//! for cluster in clusters {
//!     println!("  - {} ({})", cluster.name, cluster.context);
//! }
//!
//! // Destroy cluster
//! provider.destroy("my-cluster", true).await?;
//! ```
//!
//! # Architecture
//!
//! The crate uses a provider pattern similar to `sindri-providers`, but focused
//! on cluster lifecycle rather than workload deployment:
//!
//! ```text
//! ClusterProvider (trait)
//! ├── KindProvider
//! └── K3dProvider
//! ```
//!
//! Each provider implements the same interface for cluster operations, allowing
//! the CLI to work uniformly with either backend.

pub mod config;
pub mod installer;
pub mod k3d;
pub mod kind;
pub mod platform;
pub mod traits;

// Re-export main types for convenience
pub use config::{
    ClusterConfig, ClusterInfo, ClusterProviderType, ClusterState, ClusterStatus, K3dConfig,
    K3dRegistryConfig, KindConfig, NodeInfo, NodeRole, DEFAULT_CLUSTER_NAME, DEFAULT_K8S_VERSION,
};
pub use k3d::K3dProvider;
pub use kind::KindProvider;
pub use traits::ClusterProvider;

use anyhow::{anyhow, Result};

/// Create a cluster provider by type
///
/// # Arguments
///
/// * `provider_type` - The type of cluster provider to create
///
/// # Returns
///
/// Returns a boxed cluster provider implementing the ClusterProvider trait
///
/// # Example
///
/// ```ignore
/// use sindri_clusters::{create_cluster_provider, ClusterProviderType};
///
/// let provider = create_cluster_provider(ClusterProviderType::Kind)?;
/// println!("Using provider: {}", provider.name());
/// ```
pub fn create_cluster_provider(
    provider_type: ClusterProviderType,
) -> Result<Box<dyn ClusterProvider>> {
    match provider_type {
        ClusterProviderType::Kind => Ok(Box::new(KindProvider::new())),
        ClusterProviderType::K3d => Ok(Box::new(K3dProvider::new())),
    }
}

/// Auto-detect the best available cluster provider
///
/// This function checks for installed cluster tools and returns
/// the first available provider, preferring kind over k3d.
///
/// # Returns
///
/// Returns a boxed cluster provider if one is available, or an error
/// if no cluster tools are installed.
///
/// # Example
///
/// ```ignore
/// use sindri_clusters::detect_cluster_provider;
///
/// match detect_cluster_provider() {
///     Ok(provider) => println!("Found provider: {}", provider.name()),
///     Err(e) => println!("No cluster provider available: {}", e),
/// }
/// ```
pub fn detect_cluster_provider() -> Result<Box<dyn ClusterProvider>> {
    // Check kind first (more commonly used)
    let kind = KindProvider::new();
    if kind.check_installed() {
        return Ok(Box::new(kind));
    }

    // Check k3d
    let k3d = K3dProvider::new();
    if k3d.check_installed() {
        return Ok(Box::new(k3d));
    }

    Err(anyhow!(
        "No cluster provider found. Install kind or k3d:\n\
         - kind: https://kind.sigs.k8s.io/\n\
         - k3d: https://k3d.io/"
    ))
}

/// Get all available cluster providers
///
/// Returns a list of all cluster providers that are currently installed.
///
/// # Example
///
/// ```ignore
/// use sindri_clusters::get_available_providers;
///
/// let providers = get_available_providers();
/// for provider in providers {
///     println!("Available: {} ({})", provider.name(),
///         provider.get_version().unwrap_or_default());
/// }
/// ```
pub fn get_available_providers() -> Vec<Box<dyn ClusterProvider>> {
    let mut providers: Vec<Box<dyn ClusterProvider>> = Vec::new();

    let kind = KindProvider::new();
    if kind.check_installed() {
        providers.push(Box::new(kind));
    }

    let k3d = K3dProvider::new();
    if k3d.check_installed() {
        providers.push(Box::new(k3d));
    }

    providers
}

/// Check if any cluster provider is available
pub fn has_cluster_provider() -> bool {
    KindProvider::new().check_installed() || K3dProvider::new().check_installed()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_kind_provider() {
        let provider = create_cluster_provider(ClusterProviderType::Kind).unwrap();
        assert_eq!(provider.name(), "kind");
    }

    #[test]
    fn test_create_k3d_provider() {
        let provider = create_cluster_provider(ClusterProviderType::K3d).unwrap();
        assert_eq!(provider.name(), "k3d");
    }
}
