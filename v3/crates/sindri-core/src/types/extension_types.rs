//! Extension type definitions matching extension.schema.json

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Extension definition from extension.yaml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extension {
    /// Extension metadata
    pub metadata: ExtensionMetadata,

    /// System requirements
    #[serde(default)]
    pub requirements: Option<ExtensionRequirements>,

    /// Installation configuration
    pub install: InstallConfig,

    /// Post-install configuration
    #[serde(default)]
    pub configure: Option<ConfigureConfig>,

    /// Validation rules
    pub validate: ValidateConfig,

    /// Removal configuration
    #[serde(default)]
    pub remove: Option<RemoveConfig>,

    /// Upgrade configuration
    #[serde(default)]
    pub upgrade: Option<UpgradeConfig>,

    /// Capabilities (project-init, auth, hooks, mcp)
    #[serde(default)]
    pub capabilities: Option<CapabilitiesConfig>,

    /// Bill of Materials
    #[serde(default)]
    pub bom: Option<BomConfig>,
}

/// Extension metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionMetadata {
    /// Extension name (lowercase, hyphens allowed)
    pub name: String,

    /// Semantic version
    pub version: String,

    /// Short description
    pub description: String,

    /// Extension category
    pub category: ExtensionCategory,

    /// Author name
    #[serde(default)]
    pub author: Option<String>,

    /// Homepage URL
    #[serde(default)]
    pub homepage: Option<String>,

    /// Dependencies (other extension names)
    #[serde(default)]
    pub dependencies: Vec<String>,
}

/// Extension categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExtensionCategory {
    AiAgents,
    AiDev,
    Claude,
    Cloud,
    Desktop,
    Devops,
    Documentation,
    Languages,
    Mcp,
    Productivity,
    Research,
    Testing,
}

impl std::fmt::Display for ExtensionCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtensionCategory::AiAgents => write!(f, "ai-agents"),
            ExtensionCategory::AiDev => write!(f, "ai-dev"),
            ExtensionCategory::Claude => write!(f, "claude"),
            ExtensionCategory::Cloud => write!(f, "cloud"),
            ExtensionCategory::Desktop => write!(f, "desktop"),
            ExtensionCategory::Devops => write!(f, "devops"),
            ExtensionCategory::Documentation => write!(f, "documentation"),
            ExtensionCategory::Languages => write!(f, "languages"),
            ExtensionCategory::Mcp => write!(f, "mcp"),
            ExtensionCategory::Productivity => write!(f, "productivity"),
            ExtensionCategory::Research => write!(f, "research"),
            ExtensionCategory::Testing => write!(f, "testing"),
        }
    }
}

/// Extension requirements
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtensionRequirements {
    /// Required network domains
    #[serde(default)]
    pub domains: Vec<String>,

    /// Disk space in MB
    #[serde(default, rename = "diskSpace")]
    pub disk_space: Option<u32>,

    /// Memory in MB
    #[serde(default)]
    pub memory: Option<u32>,

    /// Estimated install time in seconds
    #[serde(default, rename = "installTime")]
    pub install_time: Option<u32>,

    /// Install timeout in seconds
    #[serde(default = "default_install_timeout", rename = "installTimeout")]
    pub install_timeout: u32,

    /// Validation timeout in seconds
    #[serde(default = "default_validation_timeout", rename = "validationTimeout")]
    pub validation_timeout: u32,

    /// Required secrets/environment variables
    #[serde(default)]
    pub secrets: Vec<String>,

    /// GPU requirements
    #[serde(default)]
    pub gpu: Option<GpuRequirements>,
}

fn default_install_timeout() -> u32 {
    300
}

fn default_validation_timeout() -> u32 {
    30
}

/// GPU requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuRequirements {
    /// GPU is mandatory
    #[serde(default)]
    pub required: bool,

    /// GPU is recommended
    #[serde(default)]
    pub recommended: bool,

    /// GPU vendor type
    #[serde(default = "default_gpu_req_type")]
    pub r#type: GpuRequirementType,

    /// Minimum GPU count
    #[serde(default = "default_min_gpu", rename = "minCount")]
    pub min_count: u32,

    /// Minimum GPU memory in MB
    #[serde(default, rename = "minMemory")]
    pub min_memory: Option<u32>,

    /// Minimum CUDA version
    #[serde(default, rename = "cudaVersion")]
    pub cuda_version: Option<String>,
}

fn default_gpu_req_type() -> GpuRequirementType {
    GpuRequirementType::Nvidia
}

fn default_min_gpu() -> u32 {
    1
}

/// GPU requirement types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GpuRequirementType {
    #[default]
    Nvidia,
    Amd,
    Any,
}

/// Installation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallConfig {
    /// Installation method
    pub method: InstallMethod,

    /// mise configuration
    #[serde(default)]
    pub mise: Option<MiseInstallConfig>,

    /// APT configuration
    #[serde(default)]
    pub apt: Option<AptInstallConfig>,

    /// Binary download configuration
    #[serde(default)]
    pub binary: Option<BinaryInstallConfig>,

    /// NPM global configuration
    #[serde(default)]
    pub npm: Option<NpmInstallConfig>,

    /// Script configuration
    #[serde(default)]
    pub script: Option<ScriptConfig>,
}

/// Installation methods
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstallMethod {
    Mise,
    Apt,
    Binary,
    Npm,
    NpmGlobal,
    Script,
    Hybrid,
}

/// mise installation config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiseInstallConfig {
    /// Path to mise.toml config file
    #[serde(default, rename = "configFile")]
    pub config_file: Option<String>,

    /// Reshim after install
    #[serde(default = "default_true", rename = "reshimAfterInstall")]
    pub reshim_after_install: bool,
}

fn default_true() -> bool {
    true
}

/// APT installation config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AptInstallConfig {
    /// Custom repositories
    #[serde(default)]
    pub repositories: Vec<AptRepository>,

    /// Packages to install
    #[serde(default)]
    pub packages: Vec<String>,

    /// Run apt update first
    #[serde(default = "default_true", rename = "updateFirst")]
    pub update_first: bool,
}

/// APT repository definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AptRepository {
    /// Repository name
    #[serde(default)]
    pub name: Option<String>,

    /// GPG key URL
    #[serde(rename = "gpgKey")]
    pub gpg_key: String,

    /// Sources list entry
    pub sources: String,
}

/// Binary download config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryInstallConfig {
    /// Downloads to perform
    #[serde(default)]
    pub downloads: Vec<BinaryDownload>,
}

/// Binary download definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryDownload {
    /// Binary name
    pub name: String,

    /// Download source
    pub source: DownloadSource,

    /// Destination path
    #[serde(default)]
    pub destination: Option<String>,

    /// Extract archive
    #[serde(default)]
    pub extract: bool,
}

/// Download source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadSource {
    /// Source type
    pub r#type: DownloadSourceType,

    /// URL or repo
    pub url: String,

    /// Asset pattern for GitHub releases
    #[serde(default)]
    pub asset: Option<String>,

    /// Version (or "latest")
    #[serde(default = "default_version")]
    pub version: String,
}

fn default_version() -> String {
    "latest".to_string()
}

/// Download source types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DownloadSourceType {
    GithubRelease,
    DirectUrl,
}

/// NPM global install config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpmInstallConfig {
    /// NPM package name with optional version
    pub package: String,
}

/// Script execution config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptConfig {
    /// Script path
    pub path: String,

    /// Script arguments
    #[serde(default)]
    pub args: Vec<String>,

    /// Timeout in seconds
    #[serde(default = "default_script_timeout")]
    pub timeout: u32,
}

fn default_script_timeout() -> u32 {
    600
}

/// Configuration phase
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConfigureConfig {
    /// Template files to process
    #[serde(default)]
    pub templates: Vec<TemplateConfig>,

    /// Environment variables to set
    #[serde(default)]
    pub environment: Vec<EnvironmentConfig>,
}

/// Template configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateConfig {
    /// Source template path
    pub source: String,

    /// Destination path
    pub destination: String,

    /// Template mode
    #[serde(default = "default_template_mode")]
    pub mode: TemplateMode,

    /// Optional condition for template selection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<TemplateCondition>,
}

fn default_template_mode() -> TemplateMode {
    TemplateMode::Overwrite
}

/// Template application modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TemplateMode {
    #[default]
    Overwrite,
    Append,
    Merge,
    SkipIfExists,
}

/// Template condition for environment-based selection
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TemplateCondition {
    /// Environment variable conditions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<EnvCondition>,

    /// Platform conditions (OS, architecture)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<PlatformCondition>,

    /// Logical OR - at least one condition must match
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any: Option<Vec<TemplateCondition>>,

    /// Logical AND - all conditions must match
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all: Option<Vec<TemplateCondition>>,

    /// Logical NOT - invert the nested condition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not: Option<Box<TemplateCondition>>,
}

/// Environment variable condition (untagged for flexible syntax)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EnvCondition {
    /// Simple key-value: { CI: "true" }
    Simple(HashMap<String, String>),

    /// Complex with operators: { CI: { equals: "true" } }
    Complex(HashMap<String, EnvConditionExpr>),

    /// Logical operators: { any: [...], all: [...], not_any: [...] }
    Logical(EnvConditionLogical),
}

/// Environment variable condition expression
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct EnvConditionExpr {
    /// Value must equal this string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub equals: Option<String>,

    /// Value must not equal this string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_equals: Option<String>,

    /// Variable must exist (true) or not exist (false)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exists: Option<bool>,

    /// Value must match regex pattern
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matches: Option<String>,

    /// Value must be in this list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_list: Option<Vec<String>>,
}

/// Environment variable logical operators
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct EnvConditionLogical {
    /// At least one condition must match
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any: Option<Vec<HashMap<String, String>>>,

    /// All conditions must match
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all: Option<Vec<HashMap<String, String>>>,

    /// None of these conditions must match
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_any: Option<Vec<HashMap<String, String>>>,

    /// Not all of these conditions must match
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_all: Option<Vec<HashMap<String, String>>>,
}

/// Platform condition (OS and/or architecture)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlatformCondition {
    /// Operating systems: ["linux", "macos", "windows"]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<Vec<String>>,

    /// Architectures: ["x86_64", "aarch64", "arm64"]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arch: Option<Vec<String>>,
}

/// Environment variable configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    /// Variable name
    pub key: String,

    /// Variable value
    pub value: String,

    /// Scope
    #[serde(default = "default_env_scope")]
    pub scope: EnvironmentScope,
}

fn default_env_scope() -> EnvironmentScope {
    EnvironmentScope::Bashrc
}

/// Environment variable scopes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnvironmentScope {
    #[default]
    Bashrc,
    Profile,
    Session,
}

/// Validation configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidateConfig {
    /// Commands to validate
    #[serde(default)]
    pub commands: Vec<CommandValidation>,

    /// mise tool validation
    #[serde(default)]
    pub mise: Option<MiseValidation>,
}

/// Command validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandValidation {
    /// Command name
    pub name: String,

    /// Version flag
    #[serde(default = "default_version_flag", rename = "versionFlag")]
    pub version_flag: String,

    /// Expected output pattern (regex)
    #[serde(default, rename = "expectedPattern")]
    pub expected_pattern: Option<String>,
}

fn default_version_flag() -> String {
    "--version".to_string()
}

/// mise validation config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiseValidation {
    /// Tools to check
    #[serde(default)]
    pub tools: Vec<String>,

    /// Minimum tool count
    #[serde(default, rename = "minToolCount")]
    pub min_tool_count: Option<u32>,
}

/// Removal configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RemoveConfig {
    /// Require confirmation
    #[serde(default = "default_true")]
    pub confirmation: bool,

    /// mise removal config
    #[serde(default)]
    pub mise: Option<MiseRemoveConfig>,

    /// APT removal config
    #[serde(default)]
    pub apt: Option<AptRemoveConfig>,

    /// Script removal config
    #[serde(default)]
    pub script: Option<ScriptRemoveConfig>,

    /// Paths to remove
    #[serde(default)]
    pub paths: Vec<String>,
}

/// mise removal config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiseRemoveConfig {
    /// Remove mise config
    #[serde(default = "default_true", rename = "removeConfig")]
    pub remove_config: bool,

    /// Tools to remove
    #[serde(default)]
    pub tools: Vec<String>,
}

/// APT removal config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AptRemoveConfig {
    /// Packages to remove
    #[serde(default)]
    pub packages: Vec<String>,

    /// Use purge instead of remove
    #[serde(default)]
    pub purge: bool,
}

/// Script removal config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptRemoveConfig {
    /// Script path
    #[serde(default)]
    pub path: Option<String>,

    /// Timeout
    #[serde(default = "default_remove_timeout")]
    pub timeout: u32,
}

fn default_remove_timeout() -> u32 {
    120
}

/// Upgrade configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeConfig {
    /// Upgrade strategy
    #[serde(default = "default_upgrade_strategy")]
    pub strategy: UpgradeStrategy,

    /// mise upgrade config
    #[serde(default)]
    pub mise: Option<MiseUpgradeConfig>,

    /// APT upgrade config
    #[serde(default)]
    pub apt: Option<AptUpgradeConfig>,

    /// Script upgrade config
    #[serde(default)]
    pub script: Option<ScriptConfig>,
}

fn default_upgrade_strategy() -> UpgradeStrategy {
    UpgradeStrategy::Automatic
}

/// Upgrade strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpgradeStrategy {
    #[default]
    Automatic,
    Manual,
    None,
    Reinstall,
    InPlace,
}

/// mise upgrade config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiseUpgradeConfig {
    /// Upgrade all tools
    #[serde(default = "default_true", rename = "upgradeAll")]
    pub upgrade_all: bool,

    /// Specific tools to upgrade
    #[serde(default)]
    pub tools: Vec<String>,
}

/// APT upgrade config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AptUpgradeConfig {
    /// Packages to upgrade
    #[serde(default)]
    pub packages: Vec<String>,

    /// Run apt update first
    #[serde(default = "default_true", rename = "updateFirst")]
    pub update_first: bool,
}

/// Capabilities configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilitiesConfig {
    /// Project initialization capability
    #[serde(default, rename = "project-init")]
    pub project_init: Option<ProjectInitCapability>,

    /// Authentication requirements
    #[serde(default)]
    pub auth: Option<AuthCapability>,

    /// Lifecycle hooks
    #[serde(default)]
    pub hooks: Option<HooksCapability>,

    /// MCP server integration
    #[serde(default)]
    pub mcp: Option<McpCapability>,

    /// Project context capability
    #[serde(default, rename = "project-context")]
    pub project_context: Option<ProjectContextCapability>,

    /// Feature flags
    #[serde(default)]
    pub features: Option<FeaturesConfig>,

    /// Collision handling
    #[serde(default, rename = "collision-handling")]
    pub collision_handling: Option<CollisionHandlingConfig>,
}

/// Project initialization capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInitCapability {
    /// Enabled
    pub enabled: bool,

    /// Initialization priority
    #[serde(default = "default_priority")]
    pub priority: u32,

    /// Commands to run
    #[serde(default)]
    pub commands: Vec<ProjectInitCommand>,

    /// State markers
    #[serde(default, rename = "state-markers")]
    pub state_markers: Vec<StateMarker>,

    /// Validation
    #[serde(default)]
    pub validation: Option<ProjectInitValidation>,
}

fn default_priority() -> u32 {
    100
}

/// Project init command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInitCommand {
    /// Command to run
    pub command: String,

    /// Description
    pub description: String,

    /// Required authentication
    #[serde(default = "default_auth_none", rename = "requiresAuth")]
    pub requires_auth: AuthProvider,

    /// Conditional execution
    #[serde(default)]
    pub conditional: bool,
}

fn default_auth_none() -> AuthProvider {
    AuthProvider::None
}

/// State marker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMarker {
    /// Path to check
    pub path: String,

    /// Type of marker
    pub r#type: StateMarkerType,

    /// Description
    #[serde(default)]
    pub description: Option<String>,
}

/// State marker types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StateMarkerType {
    Directory,
    File,
    Symlink,
}

/// Project init validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInitValidation {
    /// Command to run
    pub command: String,

    /// Expected pattern
    #[serde(default, rename = "expectedPattern")]
    pub expected_pattern: Option<String>,

    /// Expected exit code
    #[serde(default, rename = "expectedExitCode")]
    pub expected_exit_code: i32,
}

/// Authentication capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthCapability {
    /// Auth provider
    pub provider: AuthProvider,

    /// Required
    #[serde(default)]
    pub required: bool,

    /// Accepted methods
    #[serde(default)]
    pub methods: Vec<AuthMethod>,

    /// Environment variables
    #[serde(default, rename = "envVars")]
    pub env_vars: Vec<String>,

    /// Validator command
    #[serde(default)]
    pub validator: Option<AuthValidator>,

    /// Feature-level auth
    #[serde(default)]
    pub features: Vec<AuthFeature>,
}

/// Auth providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthProvider {
    Anthropic,
    Openai,
    Github,
    Custom,
    #[default]
    None,
}

/// Auth methods
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuthMethod {
    ApiKey,
    CliAuth,
}

/// Auth validator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthValidator {
    /// Command to run
    pub command: String,

    /// Expected exit code
    #[serde(default, rename = "expectedExitCode")]
    pub expected_exit_code: i32,
}

/// Feature-level auth
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthFeature {
    /// Feature name
    pub name: String,

    /// Requires API key
    #[serde(rename = "requiresApiKey")]
    pub requires_api_key: bool,

    /// Description
    pub description: String,
}

/// Hooks capability
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HooksCapability {
    /// Pre-install hook
    #[serde(default, rename = "pre-install")]
    pub pre_install: Option<HookConfig>,

    /// Post-install hook
    #[serde(default, rename = "post-install")]
    pub post_install: Option<HookConfig>,

    /// Pre-project-init hook
    #[serde(default, rename = "pre-project-init")]
    pub pre_project_init: Option<HookConfig>,

    /// Post-project-init hook
    #[serde(default, rename = "post-project-init")]
    pub post_project_init: Option<HookConfig>,
}

/// Hook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookConfig {
    /// Command to run
    pub command: String,

    /// Description
    #[serde(default)]
    pub description: Option<String>,
}

/// MCP capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpCapability {
    /// Enabled
    pub enabled: bool,

    /// MCP server config
    #[serde(default)]
    pub server: Option<McpServerConfig>,

    /// Available tools
    #[serde(default)]
    pub tools: Vec<McpTool>,
}

/// MCP server config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Command to run
    pub command: String,

    /// Arguments
    #[serde(default)]
    pub args: Vec<String>,

    /// Environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// MCP tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    /// Tool name
    pub name: String,

    /// Description
    pub description: String,
}

/// Project context capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContextCapability {
    /// Enabled
    pub enabled: bool,

    /// Merge file config
    #[serde(default, rename = "mergeFile")]
    pub merge_file: Option<MergeFileConfig>,
}

/// Merge file config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeFileConfig {
    /// Source file
    pub source: String,

    /// Target file
    pub target: String,

    /// Merge strategy
    pub strategy: MergeStrategy,
}

/// Merge strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MergeStrategy {
    Append,
    Prepend,
    Merge,
    Replace,
    AppendIfMissing,
}

/// Features configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FeaturesConfig {
    /// Core features
    #[serde(default)]
    pub core: Option<CoreFeatures>,

    /// Swarm features
    #[serde(default)]
    pub swarm: Option<SwarmFeatures>,

    /// LLM features
    #[serde(default)]
    pub llm: Option<LlmFeatures>,

    /// Advanced features
    #[serde(default)]
    pub advanced: Option<AdvancedFeatures>,

    /// MCP features
    #[serde(default)]
    pub mcp: Option<McpFeatures>,
}

/// Core features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreFeatures {
    #[serde(default = "default_true")]
    pub daemon_autostart: bool,
    #[serde(default = "default_true")]
    pub flash_attention: bool,
    #[serde(default = "default_true")]
    pub unified_config: bool,
}

/// Swarm features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmFeatures {
    #[serde(default = "default_topology")]
    pub default_topology: String,
    #[serde(default = "default_consensus")]
    pub consensus_algorithm: String,
}

fn default_topology() -> String {
    "hierarchical-mesh".to_string()
}

fn default_consensus() -> String {
    "raft".to_string()
}

/// LLM features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmFeatures {
    #[serde(default = "default_llm_provider")]
    pub default_provider: String,
    #[serde(default)]
    pub load_balancing: bool,
}

fn default_llm_provider() -> String {
    "anthropic".to_string()
}

/// Advanced features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedFeatures {
    #[serde(default)]
    pub sona_learning: bool,
    #[serde(default)]
    pub security_scanning: bool,
    #[serde(default)]
    pub claims_system: bool,
    #[serde(default = "default_true")]
    pub plugin_system: bool,
}

/// MCP features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpFeatures {
    #[serde(default = "default_mcp_transport")]
    pub transport: String,
}

fn default_mcp_transport() -> String {
    "stdio".to_string()
}

/// Collision handling config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollisionHandlingConfig {
    /// Enabled
    pub enabled: bool,

    /// Conflict rules
    #[serde(default, rename = "conflict-rules")]
    pub conflict_rules: Vec<ConflictRule>,

    /// Version markers
    #[serde(default, rename = "version-markers")]
    pub version_markers: Vec<VersionMarker>,

    /// Collision scenarios
    #[serde(default)]
    pub scenarios: Vec<CollisionScenario>,
}

/// Conflict rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictRule {
    /// Path
    pub path: String,

    /// Type
    pub r#type: ConflictResourceType,

    /// On conflict action
    #[serde(rename = "on-conflict")]
    pub on_conflict: OnConflictAction,
}

/// Conflict resource types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConflictResourceType {
    File,
    Directory,
}

/// On conflict action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnConflictAction {
    /// Action type
    pub action: ConflictActionType,

    /// Separator for append/prepend
    #[serde(default)]
    pub separator: Option<String>,

    /// Backup suffix
    #[serde(default = "default_backup_suffix", rename = "backup-suffix")]
    pub backup_suffix: String,

    /// Backup before action
    #[serde(default)]
    pub backup: bool,

    /// Prompt options
    #[serde(default, rename = "prompt-options")]
    pub prompt_options: Vec<String>,
}

fn default_backup_suffix() -> String {
    ".backup".to_string()
}

/// Conflict action types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConflictActionType {
    Overwrite,
    Append,
    Prepend,
    MergeJson,
    MergeYaml,
    Backup,
    BackupAndReplace,
    Merge,
    Prompt,
    PromptPerFile,
    Skip,
}

/// Version marker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionMarker {
    /// Path
    pub path: String,

    /// Type
    pub r#type: StateMarkerType,

    /// Version
    pub version: String,

    /// Detection config
    pub detection: VersionDetection,
}

/// Version detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDetection {
    /// Detection method
    pub method: DetectionMethod,

    /// Patterns for content-match
    #[serde(default)]
    pub patterns: Vec<String>,

    /// Match any pattern
    #[serde(default, rename = "match-any")]
    pub match_any: bool,

    /// Exclude if paths exist
    #[serde(default, rename = "exclude-if")]
    pub exclude_if: Vec<String>,
}

/// Detection methods
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DetectionMethod {
    FileExists,
    DirectoryExists,
    ContentMatch,
}

/// Collision scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollisionScenario {
    /// Scenario name
    pub name: String,

    /// Detected version
    #[serde(rename = "detected-version")]
    pub detected_version: String,

    /// Installing version
    #[serde(rename = "installing-version")]
    pub installing_version: String,

    /// Action
    pub action: ScenarioAction,

    /// Message
    pub message: String,

    /// Options
    #[serde(default)]
    pub options: Vec<ScenarioOption>,
}

/// Scenario actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScenarioAction {
    Stop,
    Skip,
    Proceed,
    Backup,
    Prompt,
}

/// Scenario option
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioOption {
    /// Label
    pub label: String,

    /// Action
    pub action: String,

    /// Backup suffix
    #[serde(default, rename = "backup-suffix")]
    pub backup_suffix: Option<String>,
}

/// Bill of Materials configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BomConfig {
    /// Tools installed
    #[serde(default)]
    pub tools: Vec<BomTool>,

    /// Files created
    #[serde(default)]
    pub files: Vec<BomFile>,
}

/// BOM tool entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BomTool {
    /// Tool name
    pub name: String,

    /// Version
    #[serde(default)]
    pub version: Option<String>,

    /// Source
    pub source: BomSource,

    /// Type
    #[serde(default)]
    pub r#type: Option<BomToolType>,

    /// License
    #[serde(default)]
    pub license: Option<String>,

    /// Homepage
    #[serde(default)]
    pub homepage: Option<String>,

    /// Download URL
    #[serde(default, rename = "downloadUrl")]
    pub download_url: Option<String>,

    /// Checksum
    #[serde(default)]
    pub checksum: Option<Checksum>,

    /// Package URL
    #[serde(default)]
    pub purl: Option<String>,

    /// CPE
    #[serde(default)]
    pub cpe: Option<String>,
}

/// BOM sources
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BomSource {
    Mise,
    Apt,
    Npm,
    Pip,
    Binary,
    Script,
    GithubRelease,
}

/// BOM tool types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BomToolType {
    Runtime,
    Compiler,
    PackageManager,
    CliTool,
    Library,
    Framework,
    Database,
    Server,
    Utility,
    Application,
}

/// Checksum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checksum {
    /// Algorithm
    pub algorithm: ChecksumAlgorithm,

    /// Value
    pub value: String,
}

/// Checksum algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChecksumAlgorithm {
    Sha256,
    Sha512,
    Md5,
}

/// BOM file entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BomFile {
    /// File path
    pub path: String,

    /// File type
    pub r#type: BomFileType,

    /// Checksum
    #[serde(default)]
    pub checksum: Option<Checksum>,
}

/// BOM file types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BomFileType {
    Config,
    Binary,
    Library,
    Script,
    Data,
}
