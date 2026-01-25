//! Packer provider trait definitions
//!
//! This module defines the `PackerProvider` trait that extends the base `Provider`
//! trait with image building capabilities specific to HashiCorp Packer.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sindri_core::types::packer_config::PackerConfig;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Extended trait for Packer-based providers
///
/// Implementors must also implement the base `Provider` trait from `sindri_providers`.
/// This trait adds image building and management capabilities.
#[async_trait]
pub trait PackerProvider: Send + Sync {
    /// Get the cloud provider name (aws, azure, gcp, oci, alibaba)
    fn cloud_name(&self) -> &'static str;

    /// Build VM image using Packer
    ///
    /// This method:
    /// 1. Generates the HCL2 template from configuration
    /// 2. Initializes Packer plugins
    /// 3. Validates the template
    /// 4. Runs the build process
    /// 5. Returns build results including image ID
    async fn build_image(&self, config: &PackerConfig, opts: BuildOptions) -> Result<BuildResult>;

    /// List available images matching the configuration
    async fn list_images(&self, config: &PackerConfig) -> Result<Vec<ImageInfo>>;

    /// Delete an image by ID
    async fn delete_image(&self, config: &PackerConfig, image_id: &str) -> Result<()>;

    /// Get detailed information about a specific image
    async fn get_image(&self, config: &PackerConfig, image_id: &str) -> Result<ImageInfo>;

    /// Validate the Packer template without building
    async fn validate_template(&self, config: &PackerConfig) -> Result<ValidationResult>;

    /// Check cloud provider prerequisites (CLI tools, credentials, etc.)
    fn check_cloud_prerequisites(&self) -> Result<CloudPrerequisiteStatus>;

    /// Find a cached image matching the current configuration
    ///
    /// Returns the image ID if a matching cached image exists
    async fn find_cached_image(&self, config: &PackerConfig) -> Result<Option<String>>;

    /// Deploy a VM instance from an existing image
    async fn deploy_from_image(
        &self,
        image_id: &str,
        config: &PackerConfig,
    ) -> Result<DeployFromImageResult>;

    /// Generate the HCL2 template content for inspection
    fn generate_template(&self, config: &PackerConfig) -> Result<String>;
}

/// Build options for Packer
#[derive(Debug, Clone, Default)]
pub struct BuildOptions {
    /// Force rebuild even if cached image exists
    pub force: bool,

    /// Build only specific sources (by name)
    pub only: Option<Vec<String>>,

    /// Exclude specific sources from build
    pub except: Option<Vec<String>>,

    /// Additional variable files to load
    pub var_files: Vec<PathBuf>,

    /// Additional variables to pass
    pub variables: HashMap<String, String>,

    /// Enable debug mode (PACKER_LOG=1)
    pub debug: bool,

    /// Behavior on build error
    pub on_error: OnErrorBehavior,

    /// Maximum parallel builds (0 = unlimited)
    pub parallel_builds: u32,

    /// Output directory for artifacts
    pub output_dir: Option<PathBuf>,

    /// Timeout for the build process
    pub timeout: Option<Duration>,
}

/// Behavior when a build error occurs
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OnErrorBehavior {
    /// Cleanup resources and fail
    #[default]
    Cleanup,
    /// Abort immediately without cleanup
    Abort,
    /// Pause for debugging (ask user to continue)
    AskUser,
    /// Run cleanup provisioner if defined
    RunCleanupProvisioner,
}

impl OnErrorBehavior {
    pub fn as_packer_flag(&self) -> &'static str {
        match self {
            OnErrorBehavior::Cleanup => "cleanup",
            OnErrorBehavior::Abort => "abort",
            OnErrorBehavior::AskUser => "ask",
            OnErrorBehavior::RunCleanupProvisioner => "run-cleanup-provisioner",
        }
    }
}

/// Result of a Packer build
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildResult {
    /// Whether the build succeeded
    pub success: bool,

    /// The created image ID (AMI ID, image resource ID, etc.)
    pub image_id: String,

    /// The image name
    pub image_name: String,

    /// Cloud provider (aws, azure, gcp, oci, alibaba)
    pub provider: String,

    /// Region/location where image was built
    pub region: String,

    /// Total build time
    #[serde(with = "serde_duration")]
    pub build_time: Duration,

    /// Artifact size in bytes (if available)
    pub artifact_size: Option<u64>,

    /// Packer manifest content
    pub manifest: Option<PackerManifest>,

    /// Build logs
    pub logs: Vec<String>,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Packer manifest structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackerManifest {
    pub builds: Vec<PackerManifestBuild>,
    pub last_run_uuid: String,
}

/// Individual build entry in the manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackerManifestBuild {
    pub name: String,
    pub builder_type: String,
    pub build_time: i64,
    pub artifact_id: String,
    pub packer_run_uuid: String,
    pub custom_data: HashMap<String, String>,
}

/// Image information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInfo {
    /// Image ID
    pub id: String,

    /// Image name
    pub name: String,

    /// Image description
    pub description: Option<String>,

    /// Image state (available, pending, failed)
    pub state: ImageState,

    /// Creation timestamp
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,

    /// Image size in bytes
    pub size: Option<u64>,

    /// Sindri version baked into image
    pub sindri_version: Option<String>,

    /// Extensions installed in image
    pub extensions: Vec<String>,

    /// Profile installed in image
    pub profile: Option<String>,

    /// Tags/labels
    pub tags: HashMap<String, String>,

    /// Cloud-specific metadata
    pub metadata: HashMap<String, String>,
}

/// Image state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageState {
    Available,
    Pending,
    Failed,
    Deregistered,
    Unknown,
}

impl std::fmt::Display for ImageState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageState::Available => write!(f, "available"),
            ImageState::Pending => write!(f, "pending"),
            ImageState::Failed => write!(f, "failed"),
            ImageState::Deregistered => write!(f, "deregistered"),
            ImageState::Unknown => write!(f, "unknown"),
        }
    }
}

/// Template validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the template is valid
    pub valid: bool,

    /// Validation errors
    pub errors: Vec<String>,

    /// Validation warnings
    pub warnings: Vec<String>,

    /// Formatted template content (for debugging)
    pub template_content: Option<String>,
}

/// Cloud prerequisite status
#[derive(Debug, Clone, Default)]
pub struct CloudPrerequisiteStatus {
    /// Whether Packer is installed
    pub packer_installed: bool,

    /// Packer version
    pub packer_version: Option<String>,

    /// Whether cloud CLI is installed (aws, az, gcloud, oci, aliyun)
    pub cli_installed: bool,

    /// CLI version
    pub cli_version: Option<String>,

    /// Whether credentials are configured
    pub credentials_configured: bool,

    /// Missing prerequisites
    pub missing: Vec<String>,

    /// Installation hints
    pub hints: Vec<String>,

    /// Whether all prerequisites are satisfied
    pub satisfied: bool,
}

/// Result of deploying from an existing image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployFromImageResult {
    /// Whether deployment succeeded
    pub success: bool,

    /// Instance/VM ID
    pub instance_id: String,

    /// Public IP address (if assigned)
    pub public_ip: Option<String>,

    /// Private IP address
    pub private_ip: Option<String>,

    /// SSH connection command
    pub ssh_command: Option<String>,

    /// Deployment messages
    pub messages: Vec<String>,
}

/// Serialization helper for Duration
mod serde_duration {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}
