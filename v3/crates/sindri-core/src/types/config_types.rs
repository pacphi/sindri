//! Configuration types for sindri.yaml

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Root sindri.yaml configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SindriConfigFile {
    /// Configuration schema version (e.g., "3.0")
    pub version: String,

    /// Deployment name (lowercase, hyphens allowed)
    pub name: String,

    /// Deployment configuration
    pub deployment: DeploymentConfig,

    /// Extension configuration
    pub extensions: ExtensionsConfig,

    /// Optional secrets configuration
    #[serde(default)]
    pub secrets: Vec<SecretConfig>,

    /// Provider-specific configurations
    #[serde(default)]
    pub providers: ProvidersConfig,
}

/// Deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentConfig {
    /// Deployment provider
    pub provider: Provider,

    /// Docker image to deploy
    #[serde(default)]
    pub image: Option<String>,

    /// Resource configuration
    #[serde(default)]
    pub resources: ResourcesConfig,

    /// Volume configuration
    #[serde(default)]
    pub volumes: VolumesConfig,
}

/// Available deployment providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Provider {
    Docker,
    #[serde(alias = "docker-compose")]
    DockerCompose,
    Fly,
    Devpod,
    E2b,
    Kubernetes,
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Provider::Docker => write!(f, "docker"),
            Provider::DockerCompose => write!(f, "docker-compose"),
            Provider::Fly => write!(f, "fly"),
            Provider::Devpod => write!(f, "devpod"),
            Provider::E2b => write!(f, "e2b"),
            Provider::Kubernetes => write!(f, "kubernetes"),
        }
    }
}

impl Provider {
    /// Get the normalized provider name (docker-compose → docker)
    pub fn normalized(&self) -> &str {
        match self {
            Provider::Docker | Provider::DockerCompose => "docker",
            Provider::Fly => "fly",
            Provider::Devpod => "devpod",
            Provider::E2b => "e2b",
            Provider::Kubernetes => "kubernetes",
        }
    }

    /// Check if the provider supports GPU
    pub fn supports_gpu(&self) -> bool {
        matches!(
            self,
            Provider::Docker
                | Provider::DockerCompose
                | Provider::Fly
                | Provider::Devpod
                | Provider::Kubernetes
        )
    }
}

/// Resource configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourcesConfig {
    /// Memory allocation (e.g., "4GB", "512MB")
    #[serde(default)]
    pub memory: Option<String>,

    /// CPU count
    #[serde(default)]
    pub cpus: Option<u32>,

    /// GPU configuration
    #[serde(default)]
    pub gpu: Option<GpuConfig>,
}

/// GPU configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuConfig {
    /// Enable GPU support
    #[serde(default)]
    pub enabled: bool,

    /// GPU vendor type
    #[serde(default = "default_gpu_type")]
    pub r#type: GpuType,

    /// Number of GPUs
    #[serde(default = "default_gpu_count")]
    pub count: u32,

    /// GPU tier for automatic instance selection
    #[serde(default)]
    pub tier: Option<GpuTier>,

    /// Minimum GPU memory
    #[serde(default)]
    pub memory: Option<String>,
}

fn default_gpu_type() -> GpuType {
    GpuType::Nvidia
}

fn default_gpu_count() -> u32 {
    1
}

/// GPU vendor types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GpuType {
    #[default]
    Nvidia,
    Amd,
}

/// GPU tier levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GpuTier {
    GpuSmall,
    GpuMedium,
    GpuLarge,
    GpuXlarge,
}

/// Volume configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolumesConfig {
    /// Workspace volume
    #[serde(default)]
    pub workspace: Option<WorkspaceVolume>,
}

/// Workspace volume configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceVolume {
    /// Container path for workspace volume
    #[serde(default = "default_workspace_path")]
    pub path: String,

    /// Volume size
    #[serde(default = "default_workspace_size")]
    pub size: String,
}

fn default_workspace_path() -> String {
    "/alt/home/developer/workspace".to_string()
}

fn default_workspace_size() -> String {
    "10GB".to_string()
}

/// Extensions configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionsConfig {
    /// Extension profile to use
    #[serde(default)]
    pub profile: Option<String>,

    /// Explicit list of extensions
    #[serde(default)]
    pub active: Option<Vec<String>>,

    /// Additional extensions on top of profile
    #[serde(default)]
    pub additional: Option<Vec<String>>,

    /// Auto-install extensions on startup
    #[serde(default = "default_auto_install")]
    pub auto_install: bool,
}

fn default_auto_install() -> bool {
    true
}

/// Secret configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretConfig {
    /// Environment variable name
    pub name: String,

    /// Secret source type
    pub source: SecretSource,

    /// Read from file (for env source)
    #[serde(default, rename = "fromFile")]
    pub from_file: Option<String>,

    /// Whether the secret is required
    #[serde(default)]
    pub required: bool,

    /// File path (for file source)
    #[serde(default)]
    pub path: Option<String>,

    /// Mount path in container (for file source)
    #[serde(default, rename = "mountPath")]
    pub mount_path: Option<String>,

    /// File permissions (for file source)
    #[serde(default = "default_permissions")]
    pub permissions: String,

    /// Vault path (for vault source)
    #[serde(default, rename = "vaultPath")]
    pub vault_path: Option<String>,

    /// Vault key (for vault source)
    #[serde(default, rename = "vaultKey")]
    pub vault_key: Option<String>,

    /// Vault mount point (for vault source)
    #[serde(default = "default_vault_mount", rename = "vaultMount")]
    pub vault_mount: String,

    /// S3 path (for s3 source)
    #[serde(default, rename = "s3Path")]
    pub s3_path: Option<String>,
}

fn default_permissions() -> String {
    "0644".to_string()
}

fn default_vault_mount() -> String {
    "secret".to_string()
}

/// Secret source types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SecretSource {
    Env,
    File,
    Vault,
    S3,
}

impl std::fmt::Display for SecretSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecretSource::Env => write!(f, "env"),
            SecretSource::File => write!(f, "file"),
            SecretSource::Vault => write!(f, "vault"),
            SecretSource::S3 => write!(f, "s3"),
        }
    }
}

/// Provider-specific configurations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProvidersConfig {
    /// Fly.io specific config
    #[serde(default)]
    pub fly: Option<FlyProviderConfig>,

    /// Docker specific config
    #[serde(default)]
    pub docker: Option<DockerProviderConfig>,

    /// Kubernetes specific config
    #[serde(default)]
    pub kubernetes: Option<KubernetesProviderConfig>,

    /// DevPod specific config
    #[serde(default)]
    pub devpod: Option<DevpodProviderConfig>,

    /// Local K8s (kind/k3d) specific config
    #[serde(default)]
    pub k8s: Option<LocalK8sConfig>,

    /// E2B specific config
    #[serde(default)]
    pub e2b: Option<E2bProviderConfig>,
}

/// Fly.io provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlyProviderConfig {
    /// Fly.io region
    #[serde(default = "default_fly_region")]
    pub region: String,

    /// Auto-stop machines when idle
    #[serde(default = "default_true", rename = "autoStopMachines")]
    pub auto_stop_machines: bool,

    /// Auto-start machines on connections
    #[serde(default = "default_true", rename = "autoStartMachines")]
    pub auto_start_machines: bool,

    /// CPU type
    #[serde(default = "default_cpu_kind", rename = "cpuKind")]
    pub cpu_kind: CpuKind,

    /// SSH port
    #[serde(default = "default_ssh_port", rename = "sshPort")]
    pub ssh_port: u16,

    /// Organization name
    #[serde(default)]
    pub organization: Option<String>,

    /// High availability mode
    #[serde(default, rename = "highAvailability")]
    pub high_availability: bool,
}

fn default_fly_region() -> String {
    "sjc".to_string()
}

fn default_true() -> bool {
    true
}

fn default_cpu_kind() -> CpuKind {
    CpuKind::Shared
}

fn default_ssh_port() -> u16 {
    10022
}

/// CPU type for Fly.io
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CpuKind {
    #[default]
    Shared,
    Performance,
}

/// Docker provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerProviderConfig {
    /// Network mode
    #[serde(default = "default_network")]
    pub network: NetworkMode,

    /// Restart policy
    #[serde(default = "default_restart")]
    pub restart: RestartPolicy,

    /// Additional port mappings
    #[serde(default)]
    pub ports: Vec<String>,

    /// Privileged mode
    #[serde(default)]
    pub privileged: bool,

    /// Extra hosts
    #[serde(default, rename = "extraHosts")]
    pub extra_hosts: Vec<String>,

    /// Container runtime
    #[serde(default = "default_runtime")]
    pub runtime: Runtime,

    /// Docker-in-Docker configuration
    #[serde(default)]
    pub dind: Option<DindConfig>,
}

fn default_network() -> NetworkMode {
    NetworkMode::Bridge
}

fn default_restart() -> RestartPolicy {
    RestartPolicy::UnlessStopped
}

fn default_runtime() -> Runtime {
    Runtime::Auto
}

/// Network mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NetworkMode {
    #[default]
    Bridge,
    Host,
    None,
}

/// Restart policy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RestartPolicy {
    No,
    Always,
    OnFailure,
    #[default]
    UnlessStopped,
}

/// Container runtime
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Runtime {
    Runc,
    SysboxRunc,
    #[default]
    Auto,
}

/// Docker-in-Docker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DindConfig {
    /// Enable DinD
    #[serde(default)]
    pub enabled: bool,

    /// DinD mode
    #[serde(default = "default_dind_mode")]
    pub mode: DindMode,

    /// Storage driver
    #[serde(default = "default_storage_driver", rename = "storageDriver")]
    pub storage_driver: StorageDriver,

    /// Storage size
    #[serde(default = "default_storage_size", rename = "storageSize")]
    pub storage_size: String,
}

fn default_dind_mode() -> DindMode {
    DindMode::Auto
}

fn default_storage_driver() -> StorageDriver {
    StorageDriver::Auto
}

fn default_storage_size() -> String {
    "20GB".to_string()
}

/// DinD mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DindMode {
    Sysbox,
    Privileged,
    Socket,
    #[default]
    Auto,
}

/// Storage driver
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StorageDriver {
    #[default]
    Auto,
    Overlay2,
    FuseOverlayfs,
    Vfs,
}

/// Kubernetes provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KubernetesProviderConfig {
    /// Namespace
    #[serde(default = "default_namespace")]
    pub namespace: String,

    /// Storage class
    #[serde(default, rename = "storageClass")]
    pub storage_class: Option<String>,

    /// Ingress configuration
    #[serde(default)]
    pub ingress: Option<IngressConfig>,
}

fn default_namespace() -> String {
    "default".to_string()
}

/// Ingress configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngressConfig {
    /// Enable ingress
    #[serde(default)]
    pub enabled: bool,

    /// Hostname
    #[serde(default)]
    pub hostname: Option<String>,
}

/// DevPod provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevpodProviderConfig {
    /// DevPod provider type
    pub r#type: DevpodType,

    /// Build repository for image push
    #[serde(default, rename = "buildRepository")]
    pub build_repository: Option<String>,

    /// AWS specific config
    #[serde(default)]
    pub aws: Option<AwsConfig>,

    /// GCP specific config
    #[serde(default)]
    pub gcp: Option<GcpConfig>,

    /// Azure specific config
    #[serde(default)]
    pub azure: Option<AzureConfig>,

    /// DigitalOcean specific config
    #[serde(default)]
    pub digitalocean: Option<DigitalOceanConfig>,

    /// Kubernetes specific config
    #[serde(default)]
    pub kubernetes: Option<DevpodK8sConfig>,

    /// SSH specific config
    #[serde(default)]
    pub ssh: Option<SshConfig>,

    /// Docker specific config
    #[serde(default)]
    pub docker: Option<DevpodDockerConfig>,
}

/// DevPod provider types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DevpodType {
    Aws,
    Gcp,
    Azure,
    Digitalocean,
    Kubernetes,
    Ssh,
    Docker,
}

/// AWS configuration for DevPod
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsConfig {
    #[serde(default = "default_aws_region")]
    pub region: String,
    #[serde(default = "default_aws_instance", rename = "instanceType")]
    pub instance_type: String,
    #[serde(default = "default_disk_size", rename = "diskSize")]
    pub disk_size: u32,
    #[serde(default, rename = "useSpot")]
    pub use_spot: bool,
    #[serde(default, rename = "subnetId")]
    pub subnet_id: Option<String>,
    #[serde(default, rename = "securityGroupId")]
    pub security_group_id: Option<String>,
}

fn default_aws_region() -> String {
    "us-west-2".to_string()
}

fn default_aws_instance() -> String {
    "c5.xlarge".to_string()
}

fn default_disk_size() -> u32 {
    40
}

/// GCP configuration for DevPod
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcpConfig {
    #[serde(default)]
    pub project: Option<String>,
    #[serde(default = "default_gcp_zone")]
    pub zone: String,
    #[serde(default = "default_gcp_machine", rename = "machineType")]
    pub machine_type: String,
    #[serde(default = "default_disk_size", rename = "diskSize")]
    pub disk_size: u32,
    #[serde(default = "default_gcp_disk_type", rename = "diskType")]
    pub disk_type: String,
}

fn default_gcp_zone() -> String {
    "us-central1-a".to_string()
}

fn default_gcp_machine() -> String {
    "e2-standard-4".to_string()
}

fn default_gcp_disk_type() -> String {
    "pd-balanced".to_string()
}

/// Azure configuration for DevPod
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureConfig {
    #[serde(default)]
    pub subscription: Option<String>,
    #[serde(default = "default_azure_rg", rename = "resourceGroup")]
    pub resource_group: String,
    #[serde(default = "default_azure_location")]
    pub location: String,
    #[serde(default = "default_azure_vm", rename = "vmSize")]
    pub vm_size: String,
    #[serde(default = "default_disk_size", rename = "diskSize")]
    pub disk_size: u32,
}

fn default_azure_rg() -> String {
    "devpod-resources".to_string()
}

fn default_azure_location() -> String {
    "eastus".to_string()
}

fn default_azure_vm() -> String {
    "Standard_D4s_v3".to_string()
}

/// DigitalOcean configuration for DevPod
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigitalOceanConfig {
    #[serde(default = "default_do_region")]
    pub region: String,
    #[serde(default = "default_do_size")]
    pub size: String,
    #[serde(default, rename = "diskSize")]
    pub disk_size: Option<u32>,
}

fn default_do_region() -> String {
    "nyc3".to_string()
}

fn default_do_size() -> String {
    "s-4vcpu-8gb".to_string()
}

/// Kubernetes configuration for DevPod
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevpodK8sConfig {
    #[serde(default = "default_devpod_namespace")]
    pub namespace: String,
    #[serde(default, rename = "storageClass")]
    pub storage_class: Option<String>,
    #[serde(default)]
    pub context: Option<String>,
    #[serde(default, rename = "nodeSelector")]
    pub node_selector: Option<HashMap<String, String>>,
}

fn default_devpod_namespace() -> String {
    "devpod".to_string()
}

/// SSH configuration for DevPod
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConfig {
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default = "default_ssh_user")]
    pub user: String,
    #[serde(default = "default_ssh_port_22")]
    pub port: u16,
    #[serde(default = "default_ssh_key", rename = "keyPath")]
    pub key_path: String,
}

fn default_ssh_user() -> String {
    "root".to_string()
}

fn default_ssh_port_22() -> u16 {
    22
}

fn default_ssh_key() -> String {
    "~/.ssh/id_rsa".to_string()
}

/// Docker configuration for DevPod
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevpodDockerConfig {
    #[serde(default, rename = "dockerHost")]
    pub docker_host: Option<String>,
}

/// Local Kubernetes (kind/k3d) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalK8sConfig {
    /// Provider type
    #[serde(default = "default_local_k8s_provider")]
    pub provider: LocalK8sProvider,

    /// Cluster name
    #[serde(default, rename = "clusterName")]
    pub cluster_name: Option<String>,

    /// Kubernetes version
    #[serde(default = "default_k8s_version")]
    pub version: String,

    /// Number of nodes
    #[serde(default = "default_nodes")]
    pub nodes: u32,

    /// kind-specific config
    #[serde(default)]
    pub kind: Option<KindConfig>,

    /// k3d-specific config
    #[serde(default)]
    pub k3d: Option<K3dConfig>,
}

fn default_local_k8s_provider() -> LocalK8sProvider {
    LocalK8sProvider::Kind
}

fn default_k8s_version() -> String {
    "v1.31.0".to_string()
}

fn default_nodes() -> u32 {
    1
}

/// Local K8s provider types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LocalK8sProvider {
    #[default]
    Kind,
    K3d,
}

/// kind configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KindConfig {
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default, rename = "configFile")]
    pub config_file: Option<String>,
}

/// k3d configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K3dConfig {
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub registry: Option<K3dRegistryConfig>,
}

/// k3d registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K3dRegistryConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_registry_name")]
    pub name: String,
    #[serde(default = "default_registry_port")]
    pub port: u16,
}

fn default_registry_name() -> String {
    "k3d-registry".to_string()
}

fn default_registry_port() -> u16 {
    5000
}

/// E2B provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct E2bProviderConfig {
    /// Template alias
    #[serde(default, rename = "templateAlias")]
    pub template_alias: Option<String>,

    /// Reuse existing template
    #[serde(default = "default_true", rename = "reuseTemplate")]
    pub reuse_template: bool,

    /// Sandbox timeout in seconds
    #[serde(default = "default_e2b_timeout")]
    pub timeout: u32,

    /// Auto-pause on timeout
    #[serde(default = "default_true", rename = "autoPause")]
    pub auto_pause: bool,

    /// Auto-resume paused sandbox
    #[serde(default = "default_true", rename = "autoResume")]
    pub auto_resume: bool,

    /// Enable internet access
    #[serde(default = "default_true", rename = "internetAccess")]
    pub internet_access: bool,

    /// Allowed outbound domains
    #[serde(default, rename = "allowedDomains")]
    pub allowed_domains: Vec<String>,

    /// Blocked outbound domains
    #[serde(default, rename = "blockedDomains")]
    pub blocked_domains: Vec<String>,

    /// Public URL access
    #[serde(default, rename = "publicAccess")]
    pub public_access: bool,

    /// Custom metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,

    /// E2B team
    #[serde(default)]
    pub team: Option<String>,

    /// Force rebuild on deploy
    #[serde(default, rename = "buildOnDeploy")]
    pub build_on_deploy: bool,
}

fn default_e2b_timeout() -> u32 {
    300
}
