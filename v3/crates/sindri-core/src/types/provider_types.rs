//! Provider-related types for deployment status and results

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Deployment status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentStatus {
    /// Deployment name
    pub name: String,

    /// Provider
    pub provider: String,

    /// Current state
    pub state: DeploymentState,

    /// Container/machine ID (provider-specific)
    pub instance_id: Option<String>,

    /// Deployed image
    pub image: Option<String>,

    /// IP addresses
    pub addresses: Vec<Address>,

    /// Resource usage
    pub resources: Option<ResourceUsage>,

    /// Timestamps
    pub timestamps: DeploymentTimestamps,

    /// Provider-specific details
    pub details: HashMap<String, String>,
}

/// Deployment state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeploymentState {
    /// Not deployed
    NotDeployed,
    /// Creating resources
    Creating,
    /// Running
    Running,
    /// Stopped
    Stopped,
    /// Suspended (e.g., Fly.io machines)
    Suspended,
    /// Paused (e.g., E2B sandboxes)
    Paused,
    /// Error state
    Error,
    /// Destroying resources
    Destroying,
    /// Unknown state
    Unknown,
}

impl std::fmt::Display for DeploymentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeploymentState::NotDeployed => write!(f, "not deployed"),
            DeploymentState::Creating => write!(f, "creating"),
            DeploymentState::Running => write!(f, "running"),
            DeploymentState::Stopped => write!(f, "stopped"),
            DeploymentState::Suspended => write!(f, "suspended"),
            DeploymentState::Paused => write!(f, "paused"),
            DeploymentState::Error => write!(f, "error"),
            DeploymentState::Destroying => write!(f, "destroying"),
            DeploymentState::Unknown => write!(f, "unknown"),
        }
    }
}

/// Network address
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    /// Address type
    pub r#type: AddressType,
    /// Address value
    pub value: String,
    /// Port (if applicable)
    pub port: Option<u16>,
}

/// Address types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AddressType {
    Internal,
    External,
    Ssh,
    Http,
    Https,
}

/// Resource usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU usage percentage
    pub cpu_percent: Option<f64>,
    /// Memory usage in bytes
    pub memory_bytes: Option<u64>,
    /// Memory limit in bytes
    pub memory_limit: Option<u64>,
    /// Disk usage in bytes
    pub disk_bytes: Option<u64>,
    /// Disk limit in bytes
    pub disk_limit: Option<u64>,
}

/// Deployment timestamps
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeploymentTimestamps {
    /// Creation time
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Last start time
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Last stop time
    pub stopped_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Last modified time
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Deployment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployResult {
    /// Success indicator
    pub success: bool,
    /// Deployment name
    pub name: String,
    /// Provider used
    pub provider: String,
    /// Instance/container ID
    pub instance_id: Option<String>,
    /// Connection information
    pub connection: Option<ConnectionInfo>,
    /// Messages
    pub messages: Vec<String>,
    /// Warnings
    pub warnings: Vec<String>,
}

/// Connection information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    /// SSH command
    pub ssh_command: Option<String>,
    /// HTTP URL
    pub http_url: Option<String>,
    /// HTTPS URL
    pub https_url: Option<String>,
    /// Custom connection instructions
    pub instructions: Option<String>,
}

/// Deploy options
#[derive(Debug, Clone, Default)]
pub struct DeployOptions {
    /// Force recreate
    pub force: bool,
    /// Dry run (don't actually deploy)
    pub dry_run: bool,
    /// Wait for deployment to complete
    pub wait: bool,
    /// Timeout in seconds
    pub timeout: Option<u64>,
    /// Skip validation
    pub skip_validation: bool,
    /// Verbose output
    pub verbose: bool,
    /// Skip Docker image build (use pre-built image from config)
    pub skip_build: bool,
}

/// Deployment plan (for dry-run)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentPlan {
    /// Provider
    pub provider: String,
    /// Actions to take
    pub actions: Vec<PlannedAction>,
    /// Resources to create
    pub resources: Vec<PlannedResource>,
    /// Estimated cost (if available)
    pub estimated_cost: Option<CostEstimate>,
}

/// Planned action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedAction {
    /// Action type
    pub action: ActionType,
    /// Resource affected
    pub resource: String,
    /// Description
    pub description: String,
}

/// Action types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionType {
    Create,
    Update,
    Delete,
    Start,
    Stop,
    Restart,
}

/// Planned resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedResource {
    /// Resource type
    pub resource_type: String,
    /// Resource name
    pub name: String,
    /// Configuration
    pub config: HashMap<String, serde_json::Value>,
}

/// Cost estimate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    /// Hourly cost
    pub hourly: Option<f64>,
    /// Monthly cost
    pub monthly: Option<f64>,
    /// Currency
    pub currency: String,
    /// Notes
    pub notes: Option<String>,
}

/// Prerequisite check result
#[derive(Debug, Clone)]
pub struct PrerequisiteStatus {
    /// All prerequisites met
    pub satisfied: bool,
    /// Missing prerequisites
    pub missing: Vec<Prerequisite>,
    /// Available prerequisites
    pub available: Vec<Prerequisite>,
}

/// Prerequisite
#[derive(Debug, Clone)]
pub struct Prerequisite {
    /// Name
    pub name: String,
    /// Description
    pub description: String,
    /// Install instructions
    pub install_hint: Option<String>,
    /// Version (if installed)
    pub version: Option<String>,
}
