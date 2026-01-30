//! Packer configuration types
//!
//! This module defines the configuration structures for the unified Packer provider,
//! supporting VM image building and deployment across AWS, Azure, GCP, OCI, and Alibaba Cloud.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Cloud provider enumeration for Packer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CloudProvider {
    Aws,
    Azure,
    Gcp,
    Oci,
    Alibaba,
}

impl std::fmt::Display for CloudProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CloudProvider::Aws => write!(f, "aws"),
            CloudProvider::Azure => write!(f, "azure"),
            CloudProvider::Gcp => write!(f, "gcp"),
            CloudProvider::Oci => write!(f, "oci"),
            CloudProvider::Alibaba => write!(f, "alibaba"),
        }
    }
}

impl std::str::FromStr for CloudProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "aws" => Ok(CloudProvider::Aws),
            "azure" => Ok(CloudProvider::Azure),
            "gcp" | "google" => Ok(CloudProvider::Gcp),
            "oci" | "oracle" => Ok(CloudProvider::Oci),
            "alibaba" | "alicloud" => Ok(CloudProvider::Alibaba),
            _ => Err(format!("Unknown cloud provider: {}", s)),
        }
    }
}

/// Main Packer configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackerConfig {
    /// Target cloud platform (required)
    pub cloud: CloudProvider,

    /// Use existing image ID (skip build)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_id: Option<String>,

    /// Image name prefix for built images
    #[serde(default = "default_image_name")]
    pub image_name: String,

    /// Image description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Build configuration (used when image_id is not set)
    #[serde(default)]
    pub build: BuildConfig,

    /// AWS-specific configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aws: Option<AwsConfig>,

    /// Azure-specific configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub azure: Option<AzureConfig>,

    /// GCP-specific configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gcp: Option<GcpConfig>,

    /// OCI-specific configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oci: Option<OciConfig>,

    /// Alibaba Cloud-specific configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alibaba: Option<AlibabaConfig>,

    /// Resource tags applied to all resources
    #[serde(default)]
    pub tags: HashMap<String, String>,
}

fn default_image_name() -> String {
    "sindri-dev".to_string()
}

impl Default for PackerConfig {
    fn default() -> Self {
        Self {
            cloud: CloudProvider::Aws,
            image_id: None,
            image_name: default_image_name(),
            description: None,
            build: BuildConfig::default(),
            aws: None,
            azure: None,
            gcp: None,
            oci: None,
            alibaba: None,
            tags: HashMap::new(),
        }
    }
}

/// Build configuration for creating new images
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Extensions to pre-install in the image
    #[serde(default)]
    pub extensions: Vec<String>,

    /// Extension profile to install
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,

    /// Sindri version to install
    #[serde(default = "default_sindri_version")]
    pub sindri_version: String,

    /// Cache behavior: reuse existing image if config matches
    #[serde(default = "default_cache")]
    pub cache: bool,

    /// Name prefix for built images
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_prefix: Option<String>,

    /// Security hardening options
    #[serde(default)]
    pub security: SecurityConfig,

    /// Custom provisioning scripts to run
    #[serde(default)]
    pub scripts: Vec<String>,

    /// Ansible playbook path (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ansible_playbook: Option<PathBuf>,

    /// Environment variables for provisioning
    #[serde(default)]
    pub environment: HashMap<String, String>,

    /// Files to upload during build
    #[serde(default)]
    pub file_uploads: Vec<FileUpload>,

    /// Maximum parallel builds (0 = unlimited)
    #[serde(default)]
    pub parallel_builds: u32,

    /// SSH timeout for build
    #[serde(default = "default_ssh_timeout")]
    pub ssh_timeout: String,
}

fn default_sindri_version() -> String {
    "latest".to_string()
}

fn default_cache() -> bool {
    true
}

fn default_ssh_timeout() -> String {
    "20m".to_string()
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            extensions: Vec::new(),
            profile: None,
            sindri_version: default_sindri_version(),
            cache: default_cache(),
            name_prefix: None,
            security: SecurityConfig::default(),
            scripts: Vec::new(),
            ansible_playbook: None,
            environment: HashMap::new(),
            file_uploads: Vec::new(),
            parallel_builds: 0,
            ssh_timeout: default_ssh_timeout(),
        }
    }
}

/// Security hardening configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Apply CIS benchmark hardening
    #[serde(default)]
    pub cis_hardening: bool,

    /// Run OpenSCAP security scan
    #[serde(default)]
    pub openscap_scan: bool,

    /// Clean sensitive data before image capture
    #[serde(default = "default_clean_sensitive")]
    pub clean_sensitive_data: bool,

    /// Remove SSH host keys before image capture
    #[serde(default = "default_remove_ssh_keys")]
    pub remove_ssh_keys: bool,
}

fn default_clean_sensitive() -> bool {
    true
}

fn default_remove_ssh_keys() -> bool {
    true
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            cis_hardening: false,
            openscap_scan: false,
            clean_sensitive_data: default_clean_sensitive(),
            remove_ssh_keys: default_remove_ssh_keys(),
        }
    }
}

/// File upload configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileUpload {
    /// Source path (local)
    pub source: PathBuf,
    /// Destination path (on VM)
    pub destination: String,
}

// =============================================================================
// AWS Configuration
// =============================================================================

/// AWS-specific configuration for Packer builds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsConfig {
    /// AWS region for building and deploying
    #[serde(default = "default_aws_region")]
    pub region: String,

    /// EC2 instance type for building
    #[serde(default = "default_aws_instance_type")]
    pub instance_type: String,

    /// EBS volume size in GB
    #[serde(default = "default_volume_size")]
    pub volume_size: u32,

    /// EBS volume type
    #[serde(default = "default_aws_volume_type")]
    pub volume_type: String,

    /// Encrypt boot volume
    #[serde(default = "default_encrypt")]
    pub encrypt_boot: bool,

    /// VPC ID (optional, uses default VPC if not specified)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vpc_id: Option<String>,

    /// Subnet ID (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subnet_id: Option<String>,

    /// Additional regions to copy AMI to
    #[serde(default)]
    pub ami_regions: Vec<String>,

    /// AWS account IDs to share AMI with
    #[serde(default)]
    pub ami_users: Vec<String>,

    /// AWS groups to share AMI with (e.g., "all" for public)
    #[serde(default)]
    pub ami_groups: Vec<String>,
}

fn default_aws_region() -> String {
    "us-west-2".to_string()
}

fn default_aws_instance_type() -> String {
    "t3.large".to_string()
}

fn default_volume_size() -> u32 {
    80
}

fn default_aws_volume_type() -> String {
    "gp3".to_string()
}

fn default_encrypt() -> bool {
    true
}

impl Default for AwsConfig {
    fn default() -> Self {
        Self {
            region: default_aws_region(),
            instance_type: default_aws_instance_type(),
            volume_size: default_volume_size(),
            volume_type: default_aws_volume_type(),
            encrypt_boot: default_encrypt(),
            vpc_id: None,
            subnet_id: None,
            ami_regions: Vec::new(),
            ami_users: Vec::new(),
            ami_groups: Vec::new(),
        }
    }
}

// =============================================================================
// Azure Configuration
// =============================================================================

/// Azure-specific configuration for Packer builds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureConfig {
    /// Azure subscription ID
    pub subscription_id: String,

    /// Resource group name
    pub resource_group: String,

    /// Azure region/location
    #[serde(default = "default_azure_location")]
    pub location: String,

    /// VM size for building
    #[serde(default = "default_azure_vm_size")]
    pub vm_size: String,

    /// OS disk size in GB
    #[serde(default = "default_volume_size")]
    pub os_disk_size_gb: u32,

    /// Storage account type
    #[serde(default = "default_azure_storage_type")]
    pub storage_account_type: String,

    /// Shared Image Gallery configuration (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gallery: Option<SharedImageGalleryConfig>,
}

fn default_azure_location() -> String {
    "westus2".to_string()
}

fn default_azure_vm_size() -> String {
    "Standard_D4s_v4".to_string()
}

fn default_azure_storage_type() -> String {
    "Premium_LRS".to_string()
}

impl Default for AzureConfig {
    fn default() -> Self {
        Self {
            subscription_id: String::new(),
            resource_group: String::new(),
            location: default_azure_location(),
            vm_size: default_azure_vm_size(),
            os_disk_size_gb: default_volume_size(),
            storage_account_type: default_azure_storage_type(),
            gallery: None,
        }
    }
}

/// Azure Shared Image Gallery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedImageGalleryConfig {
    /// Gallery name
    pub gallery_name: String,

    /// Image definition name
    pub image_name: String,

    /// Image version
    pub image_version: String,

    /// Regions to replicate to
    #[serde(default)]
    pub replication_regions: Vec<String>,
}

// =============================================================================
// GCP Configuration
// =============================================================================

/// GCP-specific configuration for Packer builds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcpConfig {
    /// GCP project ID
    pub project_id: String,

    /// GCP zone for building
    #[serde(default = "default_gcp_zone")]
    pub zone: String,

    /// Machine type for building
    #[serde(default = "default_gcp_machine_type")]
    pub machine_type: String,

    /// Disk size in GB
    #[serde(default = "default_volume_size")]
    pub disk_size: u32,

    /// Disk type
    #[serde(default = "default_gcp_disk_type")]
    pub disk_type: String,

    /// Enable Shielded VM secure boot
    #[serde(default)]
    pub enable_secure_boot: bool,

    /// Image family for versioning
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_family: Option<String>,

    /// Network (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,

    /// Subnetwork (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subnetwork: Option<String>,
}

fn default_gcp_zone() -> String {
    "us-west1-a".to_string()
}

fn default_gcp_machine_type() -> String {
    "e2-standard-4".to_string()
}

fn default_gcp_disk_type() -> String {
    "pd-ssd".to_string()
}

impl Default for GcpConfig {
    fn default() -> Self {
        Self {
            project_id: String::new(),
            zone: default_gcp_zone(),
            machine_type: default_gcp_machine_type(),
            disk_size: default_volume_size(),
            disk_type: default_gcp_disk_type(),
            enable_secure_boot: false,
            image_family: None,
            network: None,
            subnetwork: None,
        }
    }
}

// =============================================================================
// OCI Configuration
// =============================================================================

/// Oracle Cloud Infrastructure configuration for Packer builds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciConfig {
    /// Compartment OCID
    pub compartment_ocid: String,

    /// Availability domain
    pub availability_domain: String,

    /// Compute shape
    #[serde(default = "default_oci_shape")]
    pub shape: String,

    /// Flexible shape configuration (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape_config: Option<OciShapeConfig>,

    /// Subnet OCID
    pub subnet_ocid: String,

    /// Boot volume size in GB
    #[serde(default = "default_volume_size")]
    pub boot_volume_size_gb: u32,
}

fn default_oci_shape() -> String {
    "VM.Standard.E4.Flex".to_string()
}

impl Default for OciConfig {
    fn default() -> Self {
        Self {
            compartment_ocid: String::new(),
            availability_domain: String::new(),
            shape: default_oci_shape(),
            shape_config: None,
            subnet_ocid: String::new(),
            boot_volume_size_gb: default_volume_size(),
        }
    }
}

/// OCI flexible shape configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciShapeConfig {
    /// Number of OCPUs
    pub ocpus: u32,
    /// Memory in GB
    pub memory_in_gbs: u32,
}

// =============================================================================
// Alibaba Cloud Configuration
// =============================================================================

/// Alibaba Cloud configuration for Packer builds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlibabaConfig {
    /// Alibaba Cloud region
    #[serde(default = "default_alibaba_region")]
    pub region: String,

    /// ECS instance type
    #[serde(default = "default_alibaba_instance_type")]
    pub instance_type: String,

    /// System disk size in GB
    #[serde(default = "default_volume_size")]
    pub system_disk_size_gb: u32,

    /// System disk category
    #[serde(default = "default_alibaba_disk_category")]
    pub system_disk_category: String,

    /// VSwitch ID (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vswitch_id: Option<String>,
}

fn default_alibaba_region() -> String {
    "cn-hangzhou".to_string()
}

fn default_alibaba_instance_type() -> String {
    "ecs.g6.xlarge".to_string()
}

fn default_alibaba_disk_category() -> String {
    "cloud_essd".to_string()
}

impl Default for AlibabaConfig {
    fn default() -> Self {
        Self {
            region: default_alibaba_region(),
            instance_type: default_alibaba_instance_type(),
            system_disk_size_gb: default_volume_size(),
            system_disk_category: default_alibaba_disk_category(),
            vswitch_id: None,
        }
    }
}
