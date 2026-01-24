//! Cluster configuration types
//!
//! This module defines the configuration and status types for
//! local Kubernetes cluster management.

use serde::{Deserialize, Serialize};

/// Default Kubernetes version to use for clusters
pub const DEFAULT_K8S_VERSION: &str = "v1.35.0";

/// Default cluster name
pub const DEFAULT_CLUSTER_NAME: &str = "sindri-local";

/// Configuration for creating a local Kubernetes cluster
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    /// Name of the cluster
    pub name: String,

    /// Kubernetes version (e.g., "v1.35.0")
    #[serde(default = "default_version")]
    pub version: String,

    /// Number of nodes (1 = single node, >1 = 1 control-plane + N-1 workers)
    #[serde(default = "default_nodes")]
    pub nodes: u32,

    /// Provider-specific configuration
    #[serde(default)]
    pub provider_config: ProviderSpecificConfig,
}

fn default_version() -> String {
    DEFAULT_K8S_VERSION.to_string()
}

fn default_nodes() -> u32 {
    1
}

impl ClusterConfig {
    /// Create a new cluster configuration with defaults
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: DEFAULT_K8S_VERSION.to_string(),
            nodes: 1,
            provider_config: ProviderSpecificConfig::default(),
        }
    }

    /// Create configuration with a specific version
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Create configuration with a specific node count
    pub fn with_nodes(mut self, nodes: u32) -> Self {
        self.nodes = nodes;
        self
    }

    /// Create configuration with kind-specific settings
    pub fn with_kind_config(mut self, kind: KindConfig) -> Self {
        self.provider_config.kind = Some(kind);
        self
    }

    /// Create configuration with k3d-specific settings
    pub fn with_k3d_config(mut self, k3d: K3dConfig) -> Self {
        self.provider_config.k3d = Some(k3d);
        self
    }
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self::new(DEFAULT_CLUSTER_NAME)
    }
}

/// Provider-specific configuration options
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderSpecificConfig {
    /// Kind-specific configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<KindConfig>,

    /// K3d-specific configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub k3d: Option<K3dConfig>,
}

/// Kind-specific cluster configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KindConfig {
    /// Custom node image (e.g., "kindest/node:v1.35.0")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,

    /// Path to custom kind configuration file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_file: Option<String>,
}

/// K3d-specific cluster configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct K3dConfig {
    /// Custom k3s image (e.g., "rancher/k3s:v1.35.0-k3s1")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,

    /// Local Docker registry configuration
    #[serde(default)]
    pub registry: K3dRegistryConfig,
}

/// K3d local registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K3dRegistryConfig {
    /// Enable local registry
    #[serde(default)]
    pub enabled: bool,

    /// Registry name
    #[serde(default = "default_registry_name")]
    pub name: String,

    /// Registry port
    #[serde(default = "default_registry_port")]
    pub port: u16,
}

fn default_registry_name() -> String {
    "k3d-registry".to_string()
}

fn default_registry_port() -> u16 {
    5000
}

impl Default for K3dRegistryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            name: default_registry_name(),
            port: default_registry_port(),
        }
    }
}

/// Information about a created/existing cluster
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterInfo {
    /// Cluster name
    pub name: String,

    /// Provider type (kind, k3d)
    pub provider: String,

    /// Kubectl context name
    pub context: String,

    /// Kubernetes version
    pub version: Option<String>,

    /// Number of nodes
    pub node_count: u32,

    /// Registry URL if available (k3d only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry_url: Option<String>,

    /// Creation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl ClusterInfo {
    /// Create new cluster info
    pub fn new(
        name: impl Into<String>,
        provider: impl Into<String>,
        context: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            provider: provider.into(),
            context: context.into(),
            version: None,
            node_count: 1,
            registry_url: None,
            created_at: None,
        }
    }

    /// Set the Kubernetes version
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Set the node count
    pub fn with_node_count(mut self, count: u32) -> Self {
        self.node_count = count;
        self
    }

    /// Set the registry URL
    pub fn with_registry(mut self, url: impl Into<String>) -> Self {
        self.registry_url = Some(url.into());
        self
    }

    /// Set the creation timestamp
    pub fn with_created_at(mut self, created_at: chrono::DateTime<chrono::Utc>) -> Self {
        self.created_at = Some(created_at);
        self
    }
}

/// Cluster status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStatus {
    /// Cluster name
    pub name: String,

    /// Provider type
    pub provider: String,

    /// Kubectl context name
    pub context: String,

    /// Whether the cluster is running and accessible
    pub ready: bool,

    /// Current state
    pub state: ClusterState,

    /// Node information
    pub nodes: Vec<NodeInfo>,

    /// Additional status messages
    #[serde(default)]
    pub messages: Vec<String>,
}

impl ClusterStatus {
    /// Create new cluster status
    pub fn new(
        name: impl Into<String>,
        provider: impl Into<String>,
        context: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            provider: provider.into(),
            context: context.into(),
            ready: false,
            state: ClusterState::Unknown,
            nodes: Vec::new(),
            messages: Vec::new(),
        }
    }

    /// Set ready status
    pub fn with_ready(mut self, ready: bool) -> Self {
        self.ready = ready;
        self
    }

    /// Set cluster state
    pub fn with_state(mut self, state: ClusterState) -> Self {
        self.state = state;
        self
    }

    /// Set node information
    pub fn with_nodes(mut self, nodes: Vec<NodeInfo>) -> Self {
        self.nodes = nodes;
        self
    }

    /// Add a status message
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.messages.push(message.into());
        self
    }
}

/// Cluster state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClusterState {
    /// Cluster is running and accessible
    Running,
    /// Cluster exists but is not accessible (e.g., Docker stopped)
    Stopped,
    /// Cluster is being created
    Creating,
    /// Cluster is being deleted
    Deleting,
    /// Cluster state is unknown
    Unknown,
    /// Cluster does not exist
    NotFound,
    /// Cluster has errors
    Error,
}

impl std::fmt::Display for ClusterState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClusterState::Running => write!(f, "Running"),
            ClusterState::Stopped => write!(f, "Stopped"),
            ClusterState::Creating => write!(f, "Creating"),
            ClusterState::Deleting => write!(f, "Deleting"),
            ClusterState::Unknown => write!(f, "Unknown"),
            ClusterState::NotFound => write!(f, "Not Found"),
            ClusterState::Error => write!(f, "Error"),
        }
    }
}

/// Information about a cluster node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Node name
    pub name: String,

    /// Node role (control-plane, worker)
    pub role: NodeRole,

    /// Node status (Ready, NotReady)
    pub status: String,

    /// Kubernetes version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Internal IP address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_ip: Option<String>,
}

/// Node role in the cluster
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NodeRole {
    /// Control plane node (master)
    ControlPlane,
    /// Worker node
    Worker,
}

impl std::fmt::Display for NodeRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeRole::ControlPlane => write!(f, "control-plane"),
            NodeRole::Worker => write!(f, "worker"),
        }
    }
}

/// Supported cluster providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClusterProviderType {
    /// Kind (Kubernetes IN Docker)
    Kind,
    /// K3d (K3s in Docker)
    K3d,
}

impl std::fmt::Display for ClusterProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClusterProviderType::Kind => write!(f, "kind"),
            ClusterProviderType::K3d => write!(f, "k3d"),
        }
    }
}

impl std::str::FromStr for ClusterProviderType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "kind" => Ok(ClusterProviderType::Kind),
            "k3d" => Ok(ClusterProviderType::K3d),
            _ => Err(anyhow::anyhow!(
                "Unknown cluster provider: {}. Supported: kind, k3d",
                s
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_config_default() {
        let config = ClusterConfig::default();
        assert_eq!(config.name, DEFAULT_CLUSTER_NAME);
        assert_eq!(config.version, DEFAULT_K8S_VERSION);
        assert_eq!(config.nodes, 1);
    }

    #[test]
    fn test_cluster_config_builder() {
        let config = ClusterConfig::new("test-cluster")
            .with_version("v1.30.0")
            .with_nodes(3);

        assert_eq!(config.name, "test-cluster");
        assert_eq!(config.version, "v1.30.0");
        assert_eq!(config.nodes, 3);
    }

    #[test]
    fn test_cluster_info() {
        let info = ClusterInfo::new("test", "kind", "kind-test")
            .with_version("v1.35.0")
            .with_node_count(2);

        assert_eq!(info.name, "test");
        assert_eq!(info.provider, "kind");
        assert_eq!(info.context, "kind-test");
        assert_eq!(info.version, Some("v1.35.0".to_string()));
        assert_eq!(info.node_count, 2);
    }

    #[test]
    fn test_cluster_provider_type_from_str() {
        assert_eq!(
            "kind".parse::<ClusterProviderType>().unwrap(),
            ClusterProviderType::Kind
        );
        assert_eq!(
            "k3d".parse::<ClusterProviderType>().unwrap(),
            ClusterProviderType::K3d
        );
        assert_eq!(
            "KIND".parse::<ClusterProviderType>().unwrap(),
            ClusterProviderType::Kind
        );
        assert!("invalid".parse::<ClusterProviderType>().is_err());
    }

    #[test]
    fn test_cluster_state_display() {
        assert_eq!(ClusterState::Running.to_string(), "Running");
        assert_eq!(ClusterState::Stopped.to_string(), "Stopped");
        assert_eq!(ClusterState::NotFound.to_string(), "Not Found");
    }
}
