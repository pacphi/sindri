//! K3d (K3s in Docker) provider implementation
//!
//! This module implements the ClusterProvider trait for k3d,
//! enabling creation and management of lightweight K3s Kubernetes
//! clusters using Docker containers.
//!
//! # Features
//!
//! - Lightweight K3s distribution
//! - Single and multi-node cluster support
//! - Local Docker registry integration
//! - Custom K3s version selection
//! - JSON output parsing for reliable state detection
//!
//! # Context Naming
//!
//! K3d uses the convention `k3d-{cluster-name}` for kubectl contexts.

use crate::config::{ClusterConfig, ClusterInfo, ClusterState, ClusterStatus, NodeInfo, NodeRole};
use crate::traits::ClusterProvider;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::Deserialize;
use tokio::process::Command;
use tracing::{debug, info, warn};

/// K3d cluster provider
///
/// Manages local Kubernetes clusters using k3d (K3s in Docker).
/// K3d creates lightweight clusters by running K3s nodes as Docker containers.
pub struct K3dProvider {
    /// Path to k3d binary (if not in PATH)
    binary_path: Option<String>,
}

impl K3dProvider {
    /// Create a new K3d provider
    pub fn new() -> Self {
        Self { binary_path: None }
    }

    /// Create a K3d provider with a specific binary path
    pub fn with_binary_path(path: impl Into<String>) -> Self {
        Self {
            binary_path: Some(path.into()),
        }
    }

    /// Get the k3d command
    fn k3d_cmd(&self) -> String {
        self.binary_path
            .clone()
            .unwrap_or_else(|| "k3d".to_string())
    }

    /// Get the K3s image for the specified Kubernetes version
    fn get_k3s_image(&self, config: &ClusterConfig) -> Option<String> {
        // Check for custom image in k3d config
        if let Some(k3d_config) = &config.provider_config.k3d {
            if let Some(image) = &k3d_config.image {
                return Some(image.clone());
            }
        }

        // Use version to construct default image
        let version = &config.version;
        let v = version.strip_prefix('v').unwrap_or(version);
        Some(format!("rancher/k3s:v{}-k3s1", v))
    }

    /// Parse cluster list from k3d JSON output
    async fn parse_cluster_list(&self) -> Result<Vec<K3dClusterInfo>> {
        let output = Command::new(self.k3d_cmd())
            .args(["cluster", "list", "-o", "json"])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to list k3d clusters: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Handle empty output
        if stdout.trim().is_empty() || stdout.trim() == "null" {
            return Ok(Vec::new());
        }

        let clusters: Vec<K3dClusterInfo> = serde_json::from_str(&stdout)
            .map_err(|e| anyhow!("Failed to parse k3d cluster list: {}", e))?;

        Ok(clusters)
    }

    /// Get node information from kubectl
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
                            // K3d uses different label patterns
                            let role = if node
                                .metadata
                                .labels
                                .contains_key("node-role.kubernetes.io/control-plane")
                                || node
                                    .metadata
                                    .labels
                                    .contains_key("node-role.kubernetes.io/master")
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

impl Default for K3dProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ClusterProvider for K3dProvider {
    fn name(&self) -> &'static str {
        "k3d"
    }

    fn check_installed(&self) -> bool {
        which::which(self.k3d_cmd()).is_ok()
    }

    fn get_version(&self) -> Option<String> {
        let output = std::process::Command::new(self.k3d_cmd())
            .args(["version"])
            .output()
            .ok()?;

        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            // Parse "k3d version v5.7.4" format
            Some(version.lines().next().unwrap_or("").trim().to_string())
        } else {
            None
        }
    }

    async fn install(&self) -> Result<()> {
        crate::installer::install_k3d().await
    }

    async fn create(&self, config: &ClusterConfig) -> Result<ClusterInfo> {
        // Check prerequisites
        if !self.check_installed() {
            return Err(anyhow!("k3d is not installed. Run: sindri k8s install k3d"));
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

            let mut info = ClusterInfo::new(&config.name, "k3d", &context)
                .with_version(&config.version)
                .with_node_count(config.nodes);

            // Check for registry
            if let Some(k3d_config) = &config.provider_config.k3d {
                if k3d_config.registry.enabled {
                    info = info.with_registry(format!("localhost:{}", k3d_config.registry.port));
                }
            }

            return Ok(info);
        }

        info!("Creating k3d cluster: {}", config.name);

        let mut args = vec![
            "cluster".to_string(),
            "create".to_string(),
            config.name.clone(),
        ];

        // Add custom image if specified
        if let Some(image) = self.get_k3s_image(config) {
            args.push("--image".to_string());
            args.push(image);
        }

        // Add worker nodes (agents)
        if config.nodes > 1 {
            let agents = config.nodes - 1;
            args.push("--agents".to_string());
            args.push(agents.to_string());
            info!("Creating cluster with 1 server and {} agent(s)", agents);
        }

        // Registry configuration
        let mut registry_url = None;
        if let Some(k3d_config) = &config.provider_config.k3d {
            if k3d_config.registry.enabled {
                let registry_spec = format!(
                    "{}:0.0.0.0:{}",
                    k3d_config.registry.name, k3d_config.registry.port
                );
                args.push("--registry-create".to_string());
                args.push(registry_spec);
                registry_url = Some(format!("localhost:{}", k3d_config.registry.port));
                info!(
                    "Creating local registry: {}:{}",
                    k3d_config.registry.name, k3d_config.registry.port
                );
            }
        }

        // Wait for cluster to be ready
        args.push("--wait".to_string());

        debug!("Executing: k3d {}", args.join(" "));

        // Execute k3d cluster create
        let output = Command::new(self.k3d_cmd()).args(&args).output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to create k3d cluster: {}", stderr));
        }

        let context = self.context_name(&config.name);
        info!("Cluster '{}' created successfully", config.name);
        info!("kubectl context: {}", context);

        if let Some(ref url) = registry_url {
            info!("Local registry available at: {}", url);
        }

        let mut info = ClusterInfo::new(&config.name, "k3d", &context)
            .with_version(&config.version)
            .with_node_count(config.nodes)
            .with_created_at(chrono::Utc::now());

        if let Some(url) = registry_url {
            info = info.with_registry(url);
        }

        Ok(info)
    }

    async fn destroy(&self, name: &str, force: bool) -> Result<()> {
        if !self.exists(name).await {
            if force {
                warn!("Cluster '{}' does not exist", name);
                return Ok(());
            }
            return Err(anyhow!("Cluster '{}' does not exist", name));
        }

        info!("Deleting k3d cluster: {}", name);

        let output = Command::new(self.k3d_cmd())
            .args(["cluster", "delete", name])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to delete k3d cluster: {}", stderr));
        }

        info!("Cluster '{}' deleted", name);
        Ok(())
    }

    async fn exists(&self, name: &str) -> bool {
        match self.parse_cluster_list().await {
            Ok(clusters) => clusters.iter().any(|c| c.name == name),
            Err(_) => false,
        }
    }

    async fn list(&self) -> Result<Vec<ClusterInfo>> {
        let k3d_clusters = self.parse_cluster_list().await?;

        let clusters: Vec<ClusterInfo> = k3d_clusters
            .into_iter()
            .map(|c| {
                let context = self.context_name(&c.name);
                let node_count = c.servers_count + c.agents_count;
                ClusterInfo::new(&c.name, "k3d", &context).with_node_count(node_count)
            })
            .collect();

        Ok(clusters)
    }

    async fn status(&self, name: &str) -> Result<ClusterStatus> {
        let context = self.context_name(name);
        let mut status = ClusterStatus::new(name, "k3d", &context);

        // Check if cluster exists using k3d cluster list
        let clusters = self.parse_cluster_list().await?;
        let cluster = clusters.iter().find(|c| c.name == name);

        if cluster.is_none() {
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

        let output = Command::new(self.k3d_cmd())
            .args(["kubeconfig", "get", name])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to get kubeconfig: {}", stderr));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn context_name(&self, cluster_name: &str) -> String {
        format!("k3d-{}", cluster_name)
    }
}

// K3d JSON output types

#[derive(Debug, Deserialize)]
struct K3dClusterInfo {
    name: String,
    #[serde(rename = "serversCount", default)]
    servers_count: u32,
    #[serde(rename = "agentsCount", default)]
    agents_count: u32,
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
    fn test_k3d_provider_name() {
        let provider = K3dProvider::new();
        assert_eq!(provider.name(), "k3d");
    }

    #[test]
    fn test_context_name() {
        let provider = K3dProvider::new();
        assert_eq!(provider.context_name("my-cluster"), "k3d-my-cluster");
        assert_eq!(provider.context_name("test"), "k3d-test");
    }

    #[test]
    fn test_get_k3s_image() {
        let provider = K3dProvider::new();

        // Default version
        let config = ClusterConfig::new("test").with_version("v1.35.0");
        assert_eq!(
            provider.get_k3s_image(&config),
            Some("rancher/k3s:v1.35.0-k3s1".to_string())
        );

        // Without 'v' prefix
        let config = ClusterConfig::new("test").with_version("1.35.0");
        assert_eq!(
            provider.get_k3s_image(&config),
            Some("rancher/k3s:v1.35.0-k3s1".to_string())
        );

        // Custom image
        let config = ClusterConfig::new("test").with_k3d_config(crate::K3dConfig {
            image: Some("my-custom-k3s:latest".to_string()),
            ..Default::default()
        });
        assert_eq!(
            provider.get_k3s_image(&config),
            Some("my-custom-k3s:latest".to_string())
        );
    }

    #[test]
    fn test_parse_k3d_cluster_json() {
        let json = r#"[{"name":"test-cluster","serversCount":1,"agentsCount":2}]"#;
        let clusters: Vec<K3dClusterInfo> = serde_json::from_str(json).unwrap();

        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].name, "test-cluster");
        assert_eq!(clusters[0].servers_count, 1);
        assert_eq!(clusters[0].agents_count, 2);
    }
}
