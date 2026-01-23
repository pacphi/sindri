//! Cluster provider trait definitions
//!
//! This module defines the core trait for local Kubernetes cluster
//! management providers (kind, k3d).

use crate::config::{ClusterConfig, ClusterInfo, ClusterStatus};
use anyhow::Result;
use async_trait::async_trait;

/// Trait for local Kubernetes cluster providers
///
/// This trait defines the interface for managing local Kubernetes clusters
/// using tools like kind (Kubernetes IN Docker) or k3d (K3s in Docker).
///
/// # Example
///
/// ```ignore
/// use sindri_clusters::{ClusterProvider, KindProvider, ClusterConfig};
///
/// let provider = KindProvider::new();
/// let config = ClusterConfig::new("my-cluster");
///
/// // Create cluster
/// let info = provider.create(&config).await?;
/// println!("Created cluster with context: {}", info.context);
///
/// // List clusters
/// let clusters = provider.list().await?;
///
/// // Destroy cluster
/// provider.destroy("my-cluster", true).await?;
/// ```
#[async_trait]
pub trait ClusterProvider: Send + Sync {
    /// Get the provider name (e.g., "kind", "k3d")
    fn name(&self) -> &'static str;

    /// Check if the provider CLI tool is installed
    fn check_installed(&self) -> bool;

    /// Get the installed version of the provider CLI tool
    fn get_version(&self) -> Option<String>;

    /// Install the provider CLI tool
    ///
    /// This will attempt to install the provider using the best method
    /// for the current platform (e.g., Homebrew on macOS, binary download on Linux).
    async fn install(&self) -> Result<()>;

    /// Create a new Kubernetes cluster
    ///
    /// # Arguments
    ///
    /// * `config` - Cluster configuration including name, version, and node count
    ///
    /// # Returns
    ///
    /// Returns cluster information including the kubectl context name
    async fn create(&self, config: &ClusterConfig) -> Result<ClusterInfo>;

    /// Destroy an existing cluster
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the cluster to destroy
    /// * `force` - If true, skip confirmation prompts
    async fn destroy(&self, name: &str, force: bool) -> Result<()>;

    /// Check if a cluster exists
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the cluster to check
    async fn exists(&self, name: &str) -> bool;

    /// List all clusters managed by this provider
    async fn list(&self) -> Result<Vec<ClusterInfo>>;

    /// Get the status of a cluster
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the cluster
    async fn status(&self, name: &str) -> Result<ClusterStatus>;

    /// Get the kubeconfig for a cluster
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the cluster
    ///
    /// # Returns
    ///
    /// Returns the kubeconfig YAML content as a string
    async fn get_kubeconfig(&self, name: &str) -> Result<String>;

    /// Get the kubectl context name for a cluster
    ///
    /// # Arguments
    ///
    /// * `cluster_name` - The name of the cluster
    ///
    /// # Returns
    ///
    /// Returns the context name (e.g., "kind-my-cluster" or "k3d-my-cluster")
    fn context_name(&self, cluster_name: &str) -> String;

    /// Check if Docker is available and running
    fn check_docker(&self) -> bool {
        // Check if docker command exists
        if which::which("docker").is_err() {
            return false;
        }

        // Check if docker daemon is running
        std::process::Command::new("docker")
            .args(["info"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

/// Factory trait for creating cluster providers
pub trait ClusterProviderFactory {
    /// Create a new provider instance
    fn create() -> Box<dyn ClusterProvider>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test that the trait is object-safe
    fn _assert_object_safe(_: &dyn ClusterProvider) {}
}
