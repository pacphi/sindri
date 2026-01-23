//! Kind (Kubernetes IN Docker) provider implementation
//!
//! This module implements the ClusterProvider trait for kind,
//! enabling creation and management of local Kubernetes clusters
//! using Docker containers.
//!
//! # Features
//!
//! - Single and multi-node cluster support
//! - Custom Kubernetes version selection
//! - Custom kind configuration file support
//! - Kubeconfig export
//!
//! # Context Naming
//!
//! Kind uses the convention `kind-{cluster-name}` for kubectl contexts.

use crate::config::{ClusterConfig, ClusterInfo, ClusterState, ClusterStatus, NodeInfo, NodeRole};
use crate::traits::ClusterProvider;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::Deserialize;
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Kind cluster provider
///
/// Manages local Kubernetes clusters using kind (Kubernetes IN Docker).
/// Kind creates clusters by running Kubernetes nodes as Docker containers.
pub struct KindProvider {
    /// Path to kind binary (if not in PATH)
    binary_path: Option<String>,
}

impl KindProvider {
    /// Create a new Kind provider
    pub fn new() -> Self {
        Self { binary_path: None }
    }

    /// Create a Kind provider with a specific binary path
    pub fn with_binary_path(path: impl Into<String>) -> Self {
        Self {
            binary_path: Some(path.into()),
        }
    }

    /// Get the kind command
    fn kind_cmd(&self) -> String {
        self.binary_path
            .clone()
            .unwrap_or_else(|| "kind".to_string())
    }

    /// Generate kind cluster configuration YAML for multi-node clusters
    fn generate_cluster_config(&self, config: &ClusterConfig) -> String {
        let mut yaml = String::from(
            "kind: Cluster\n\
             apiVersion: kind.x-k8s.io/v1alpha4\n\
             nodes:\n\
             - role: control-plane\n",
        );

        // Add worker nodes
        for _ in 1..config.nodes {
            yaml.push_str("- role: worker\n");
        }

        yaml
    }

    /// Get the node image for the specified Kubernetes version
    fn get_node_image(&self, config: &ClusterConfig) -> Option<String> {
        // Check for custom image in kind config
        if let Some(kind_config) = &config.provider_config.kind {
            if let Some(image) = &kind_config.image {
                return Some(image.clone());
            }
        }

        // Use version to construct default image
        let version = &config.version;
        if version.starts_with('v') {
            Some(format!("kindest/node:{}", version))
        } else {
            Some(format!("kindest/node:v{}", version))
        }
    }

    /// Parse node information from kubectl output
    async fn get_node_info(&self, context: &str) -> Vec<NodeInfo> {
        let output = Command::new("kubectl")
            .args(["--context", context, "get", "nodes", "-o", "json"])
            .output()
            .await;

        match output {
            Ok(o) if o.status.success() => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                if let Ok(node_list) = serde_json::from_str::<KubeNodeList>(&stdout) {
                    node_list
                        .items
                        .into_iter()
                        .map(|node| {
                            let role = if node
                                .metadata
                                .labels
                                .contains_key("node-role.kubernetes.io/control-plane")
                            {
                                NodeRole::ControlPlane
                            } else {
                                NodeRole::Worker
                            };

                            let status = node
                                .status
                                .conditions
                                .iter()
                                .find(|c| c.condition_type == "Ready")
                                .map(|c| {
                                    if c.status == "True" {
                                        "Ready"
                                    } else {
                                        "NotReady"
                                    }
                                })
                                .unwrap_or("Unknown")
                                .to_string();

                            let internal_ip = node
                                .status
                                .addresses
                                .iter()
                                .find(|a| a.address_type == "InternalIP")
                                .map(|a| a.address.clone());

                            NodeInfo {
                                name: node.metadata.name,
                                role,
                                status,
                                version: node.status.node_info.map(|i| i.kubelet_version),
                                internal_ip,
                            }
                        })
                        .collect()
                } else {
                    Vec::new()
                }
            }
            _ => Vec::new(),
        }
    }
}

impl Default for KindProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ClusterProvider for KindProvider {
    fn name(&self) -> &'static str {
        "kind"
    }

    fn check_installed(&self) -> bool {
        which::which(self.kind_cmd()).is_ok()
    }

    fn get_version(&self) -> Option<String> {
        let output = std::process::Command::new(self.kind_cmd())
            .args(["version"])
            .output()
            .ok()?;

        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            // Parse "kind v0.25.0 ..." format
            Some(version.trim().to_string())
        } else {
            None
        }
    }

    async fn install(&self) -> Result<()> {
        crate::installer::install_kind().await
    }

    async fn create(&self, config: &ClusterConfig) -> Result<ClusterInfo> {
        // Check prerequisites
        if !self.check_installed() {
            return Err(anyhow!(
                "kind is not installed. Run: sindri k8s install kind"
            ));
        }

        if !self.check_docker() {
            return Err(anyhow!(
                "Docker is not running. Please start Docker and try again."
            ));
        }

        // Check if cluster already exists
        if self.exists(&config.name).await {
            let context = self.context_name(&config.name);
            warn!("Cluster '{}' already exists", config.name);
            return Ok(ClusterInfo::new(&config.name, "kind", &context)
                .with_version(&config.version)
                .with_node_count(config.nodes));
        }

        info!("Creating kind cluster: {}", config.name);

        let mut args = vec![
            "create".to_string(),
            "cluster".to_string(),
            "--name".to_string(),
            config.name.clone(),
        ];

        // Add custom image if specified
        if let Some(image) = self.get_node_image(config) {
            args.push("--image".to_string());
            args.push(image);
        }

        // Check for custom config file
        let use_generated_config = if let Some(kind_config) = &config.provider_config.kind {
            if let Some(config_file) = &kind_config.config_file {
                args.push("--config".to_string());
                args.push(config_file.clone());
                false
            } else {
                config.nodes > 1
            }
        } else {
            config.nodes > 1
        };

        // Generate config for multi-node clusters without custom config
        let temp_config_path;
        if use_generated_config {
            let config_content = self.generate_cluster_config(config);
            debug!("Generated kind config:\n{}", config_content);

            temp_config_path =
                std::env::temp_dir().join(format!("kind-config-{}.yaml", config.name));
            std::fs::write(&temp_config_path, &config_content)?;
            args.push("--config".to_string());
            args.push(temp_config_path.to_string_lossy().to_string());
        }

        // Execute kind create cluster
        let output = Command::new(self.kind_cmd()).args(&args).output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to create kind cluster: {}", stderr));
        }

        let context = self.context_name(&config.name);
        info!("Cluster '{}' created successfully", config.name);
        info!("kubectl context: {}", context);

        Ok(ClusterInfo::new(&config.name, "kind", &context)
            .with_version(&config.version)
            .with_node_count(config.nodes)
            .with_created_at(chrono::Utc::now()))
    }

    async fn destroy(&self, name: &str, force: bool) -> Result<()> {
        if !self.exists(name).await {
            if force {
                warn!("Cluster '{}' does not exist", name);
                return Ok(());
            }
            return Err(anyhow!("Cluster '{}' does not exist", name));
        }

        info!("Deleting kind cluster: {}", name);

        let output = Command::new(self.kind_cmd())
            .args(["delete", "cluster", "--name", name])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to delete kind cluster: {}", stderr));
        }

        info!("Cluster '{}' deleted", name);
        Ok(())
    }

    async fn exists(&self, name: &str) -> bool {
        let output = Command::new(self.kind_cmd())
            .args(["get", "clusters"])
            .output()
            .await;

        match output {
            Ok(o) if o.status.success() => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                stdout.lines().any(|line| line.trim() == name)
            }
            _ => false,
        }
    }

    async fn list(&self) -> Result<Vec<ClusterInfo>> {
        let output = Command::new(self.kind_cmd())
            .args(["get", "clusters"])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to list kind clusters: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let clusters: Vec<ClusterInfo> = stdout
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|name| {
                let name = name.trim().to_string();
                let context = self.context_name(&name);
                ClusterInfo::new(&name, "kind", &context)
            })
            .collect();

        Ok(clusters)
    }

    async fn status(&self, name: &str) -> Result<ClusterStatus> {
        let context = self.context_name(name);
        let mut status = ClusterStatus::new(name, "kind", &context);

        // Check if cluster exists
        if !self.exists(name).await {
            return Ok(status.with_state(ClusterState::NotFound));
        }

        // Check if cluster is accessible
        let cluster_info_output = Command::new("kubectl")
            .args(["--context", &context, "cluster-info"])
            .output()
            .await;

        match cluster_info_output {
            Ok(o) if o.status.success() => {
                let nodes = self.get_node_info(&context).await;
                status = status
                    .with_ready(true)
                    .with_state(ClusterState::Running)
                    .with_nodes(nodes);
            }
            _ => {
                status = status
                    .with_ready(false)
                    .with_state(ClusterState::Stopped)
                    .with_message("Cluster not accessible. Docker may be stopped.");
            }
        }

        Ok(status)
    }

    async fn get_kubeconfig(&self, name: &str) -> Result<String> {
        if !self.exists(name).await {
            return Err(anyhow!("Cluster '{}' does not exist", name));
        }

        let output = Command::new(self.kind_cmd())
            .args(["get", "kubeconfig", "--name", name])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to get kubeconfig: {}", stderr));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn context_name(&self, cluster_name: &str) -> String {
        format!("kind-{}", cluster_name)
    }
}

// Kubernetes API types for JSON parsing

#[derive(Debug, Deserialize)]
struct KubeNodeList {
    items: Vec<KubeNode>,
}

#[derive(Debug, Deserialize)]
struct KubeNode {
    metadata: KubeNodeMetadata,
    status: KubeNodeStatus,
}

#[derive(Debug, Deserialize)]
struct KubeNodeMetadata {
    name: String,
    #[serde(default)]
    labels: std::collections::HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct KubeNodeStatus {
    #[serde(default)]
    conditions: Vec<KubeNodeCondition>,
    #[serde(default)]
    addresses: Vec<KubeNodeAddress>,
    #[serde(rename = "nodeInfo")]
    node_info: Option<KubeNodeInfo>,
}

#[derive(Debug, Deserialize)]
struct KubeNodeCondition {
    #[serde(rename = "type")]
    condition_type: String,
    status: String,
}

#[derive(Debug, Deserialize)]
struct KubeNodeAddress {
    #[serde(rename = "type")]
    address_type: String,
    address: String,
}

#[derive(Debug, Deserialize)]
struct KubeNodeInfo {
    #[serde(rename = "kubeletVersion")]
    kubelet_version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kind_provider_name() {
        let provider = KindProvider::new();
        assert_eq!(provider.name(), "kind");
    }

    #[test]
    fn test_context_name() {
        let provider = KindProvider::new();
        assert_eq!(provider.context_name("my-cluster"), "kind-my-cluster");
        assert_eq!(provider.context_name("test"), "kind-test");
    }

    #[test]
    fn test_generate_single_node_config() {
        let provider = KindProvider::new();
        let config = ClusterConfig::new("test").with_nodes(1);
        let yaml = provider.generate_cluster_config(&config);

        assert!(yaml.contains("kind: Cluster"));
        assert!(yaml.contains("role: control-plane"));
        assert!(!yaml.contains("role: worker"));
    }

    #[test]
    fn test_generate_multi_node_config() {
        let provider = KindProvider::new();
        let config = ClusterConfig::new("test").with_nodes(3);
        let yaml = provider.generate_cluster_config(&config);

        assert!(yaml.contains("kind: Cluster"));
        assert!(yaml.contains("role: control-plane"));
        // Should have 2 workers
        assert_eq!(yaml.matches("role: worker").count(), 2);
    }

    #[test]
    fn test_get_node_image() {
        let provider = KindProvider::new();

        // Default version
        let config = ClusterConfig::new("test").with_version("v1.35.0");
        assert_eq!(
            provider.get_node_image(&config),
            Some("kindest/node:v1.35.0".to_string())
        );

        // Without 'v' prefix
        let config = ClusterConfig::new("test").with_version("1.35.0");
        assert_eq!(
            provider.get_node_image(&config),
            Some("kindest/node:v1.35.0".to_string())
        );

        // Custom image
        let config = ClusterConfig::new("test").with_kind_config(crate::KindConfig {
            image: Some("my-custom-image:latest".to_string()),
            config_file: None,
        });
        assert_eq!(
            provider.get_node_image(&config),
            Some("my-custom-image:latest".to_string())
        );
    }
}
