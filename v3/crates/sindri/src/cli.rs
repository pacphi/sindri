//! CLI argument parsing with clap

use camino::Utf8PathBuf;
use clap::{Args, Parser, Subcommand};

// Re-export command types for convenience
pub use crate::commands::backup::BackupArgs;
pub use crate::commands::restore::RestoreArgs;
pub use crate::commands::secrets::SecretsCommands;

/// Sindri - Declarative cloud development environments
#[derive(Parser, Debug)]
#[command(name = "sindri")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Suppress output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Path to sindri.yaml config file
    #[arg(short, long, global = true)]
    pub config: Option<Utf8PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Show version information
    Version(VersionArgs),

    /// Configuration management
    #[command(subcommand)]
    Config(ConfigCommands),

    /// Deploy the environment
    Deploy(DeployArgs),

    /// Connect to a deployed environment
    Connect(ConnectArgs),

    /// Show deployment status
    Status(StatusArgs),

    /// Destroy the deployment
    Destroy(DestroyArgs),

    /// Extension management
    #[command(subcommand)]
    Extension(ExtensionCommands),

    /// Profile management
    #[command(subcommand)]
    Profile(ProfileCommands),

    /// Upgrade the CLI
    #[command(disable_version_flag = true)]
    Upgrade(UpgradeArgs),

    /// Secrets management
    #[command(subcommand)]
    Secrets(SecretsCommands),

    /// Backup workspace
    Backup(BackupArgs),

    /// Restore workspace
    Restore(RestoreArgs),

    /// Project management
    #[command(subcommand)]
    Project(ProjectCommands),

    /// Check system for required tools and dependencies
    Doctor(DoctorArgs),

    /// Local Kubernetes cluster management (kind/k3d)
    #[command(subcommand)]
    K8s(K8sCommands),

    /// Container image management
    #[command(subcommand)]
    Image(ImageCommands),

    /// Build VM images with HashiCorp Packer
    #[command(subcommand, alias = "packer")]
    Vm(VmCommands),

    /// Bill of Materials management
    #[command(subcommand)]
    Bom(BomCommands),

    /// Generate shell completions
    Completions(CompletionsArgs),
}

// Version command
#[derive(Args, Debug)]
pub struct VersionArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

// Config commands
#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Initialize a new sindri.yaml
    Init(ConfigInitArgs),

    /// Validate the configuration
    Validate(ConfigValidateArgs),

    /// Show resolved configuration
    Show(ConfigShowArgs),
}

#[derive(Args, Debug)]
pub struct ConfigInitArgs {
    /// Project name
    #[arg(short, long)]
    pub name: Option<String>,

    /// Provider to use
    #[arg(short, long, default_value = "docker")]
    pub provider: String,

    /// Extension profile
    #[arg(long, default_value = "minimal")]
    pub profile: String,

    /// Output file path
    #[arg(short, long, default_value = "sindri.yaml")]
    pub output: Utf8PathBuf,

    /// Overwrite existing file
    #[arg(short, long)]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct ConfigValidateArgs {
    /// Path to config file (default: find sindri.yaml)
    #[arg(short, long)]
    pub file: Option<Utf8PathBuf>,

    /// Check extensions exist
    #[arg(long)]
    pub check_extensions: bool,
}

#[derive(Args, Debug)]
pub struct ConfigShowArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

// Deploy command
#[derive(Args, Debug)]
pub struct DeployArgs {
    /// Force recreation
    #[arg(short, long)]
    pub force: bool,

    /// Dry run (show what would happen)
    #[arg(long)]
    pub dry_run: bool,

    /// Wait for deployment
    #[arg(short, long, default_value = "true")]
    pub wait: bool,

    /// Timeout in seconds
    #[arg(short, long, default_value = "600")]
    pub timeout: u64,

    /// Skip validation
    #[arg(long)]
    pub skip_validation: bool,

    /// Skip image signature and provenance verification
    #[arg(long)]
    pub skip_image_verification: bool,

    /// Build from source instead of using pre-built image (for Sindri developers)
    #[arg(long, alias = "fs")]
    pub from_source: bool,

    /// Path to .env file (default: look for .env/.env.local in config directory)
    #[arg(long)]
    pub env_file: Option<Utf8PathBuf>,
}

// Connect command
#[derive(Args, Debug)]
pub struct ConnectArgs {
    /// Command to run instead of shell
    #[arg(long)]
    pub command: Option<String>,
}

// Status command
#[derive(Args, Debug)]
pub struct StatusArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Watch status (refresh every N seconds)
    #[arg(short, long)]
    pub watch: Option<u64>,
}

// Destroy command
#[derive(Args, Debug)]
pub struct DestroyArgs {
    /// Skip confirmation
    #[arg(short, long)]
    pub force: bool,

    /// Also remove volumes
    #[arg(long)]
    pub volumes: bool,
}

// Extension commands
#[derive(Subcommand, Debug)]
pub enum ExtensionCommands {
    /// Install an extension
    Install(ExtensionInstallArgs),

    /// List extensions
    List(ExtensionListArgs),

    /// Validate an extension
    Validate(ExtensionValidateArgs),

    /// Show extension status
    Status(ExtensionStatusArgs),

    /// Show extension information
    Info(ExtensionInfoArgs),

    /// Upgrade an extension
    Upgrade(ExtensionUpgradeArgs),

    /// Remove an extension
    Remove(ExtensionRemoveArgs),

    /// Show available versions
    Versions(ExtensionVersionsArgs),

    /// Check for extension updates
    Check(ExtensionCheckArgs),

    /// Rollback to previous version
    Rollback(ExtensionRollbackArgs),

    /// Update support files (common.sh, compatibility-matrix.yaml, extension-source.yaml)
    UpdateSupportFiles(UpdateSupportFilesArgs),

    /// Generate documentation for an extension
    Docs(ExtensionDocsArgs),
}

#[derive(Args, Debug)]
pub struct ExtensionDocsArgs {
    /// Extension name
    pub name: String,
}

#[derive(Args, Debug)]
#[command(disable_version_flag = true)]
pub struct ExtensionInstallArgs {
    /// Extension name (with optional @version)
    #[arg(conflicts_with_all = ["from_config", "profile"])]
    pub name: Option<String>,

    /// Specific version to install
    #[arg(short = 'V', long, conflicts_with_all = ["from_config", "profile"])]
    pub version: Option<String>,

    /// Install extensions from sindri.yaml config file
    #[arg(long, value_name = "PATH", conflicts_with_all = ["name", "profile", "version"])]
    pub from_config: Option<Utf8PathBuf>,

    /// Install all extensions from a profile
    #[arg(long, value_name = "NAME", conflicts_with_all = ["name", "from_config", "version"])]
    pub profile: Option<String>,

    /// Force reinstall if already installed
    #[arg(short, long)]
    pub force: bool,

    /// Skip dependency installation
    #[arg(long)]
    pub no_deps: bool,

    /// Skip confirmation prompt (for profile/config installation)
    #[arg(short = 'y', long)]
    pub yes: bool,
}

#[derive(Args, Debug)]
pub struct ExtensionListArgs {
    /// Filter by category
    #[arg(short = 'C', long)]
    pub category: Option<String>,

    /// Show installed only
    #[arg(long)]
    pub installed: bool,

    /// Show all extensions (both installed and available)
    #[arg(long)]
    pub all: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct ExtensionValidateArgs {
    /// Extension name or path to extension.yaml
    pub name: String,

    /// Path to extension.yaml file
    #[arg(short, long)]
    pub file: Option<Utf8PathBuf>,
}

#[derive(Args, Debug)]
pub struct ExtensionStatusArgs {
    /// Extension name (optional, shows all if not specified)
    pub name: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct ExtensionInfoArgs {
    /// Extension name
    pub name: String,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct ExtensionUpgradeArgs {
    /// Extension name
    pub name: String,

    /// Upgrade to specific version
    #[arg(short = 'V', long)]
    pub version: Option<String>,

    /// Skip confirmation prompt
    #[arg(short = 'y', long)]
    pub yes: bool,
}

#[derive(Args, Debug)]
pub struct ExtensionRemoveArgs {
    /// Extension name
    pub name: String,

    /// Skip confirmation prompt
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Force removal even if other extensions depend on it
    #[arg(short, long)]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct ExtensionVersionsArgs {
    /// Extension name
    pub name: String,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct ExtensionCheckArgs {
    /// Specific extensions to check (all if not specified)
    pub extensions: Vec<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct ExtensionRollbackArgs {
    /// Extension name
    pub name: String,

    /// Skip confirmation prompt
    #[arg(short = 'y', long)]
    pub yes: bool,
}

#[derive(Args, Debug)]
pub struct UpdateSupportFilesArgs {
    /// Force update even if version matches
    #[arg(short, long)]
    pub force: bool,

    /// Use bundled files instead of fetching from GitHub
    #[arg(short, long)]
    pub bundled: bool,

    /// Suppress output (for scripts/automation)
    #[arg(short, long)]
    pub quiet: bool,
}

// Upgrade command
#[derive(Args, Debug)]
#[command(disable_version_flag = true)]
pub struct UpgradeArgs {
    /// Check for updates only
    #[arg(long)]
    pub check: bool,

    /// List available versions
    #[arg(long)]
    pub list: bool,

    /// Install specific version
    #[arg(long)]
    pub target_version: Option<String>,

    /// Check extension compatibility
    #[arg(long)]
    pub compat: Option<String>,

    /// Include prereleases
    #[arg(long)]
    pub prerelease: bool,

    /// Allow downgrade to older version
    #[arg(long)]
    pub allow_downgrade: bool,

    /// Skip confirmation prompts
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Force upgrade even if extensions are incompatible
    #[arg(short, long)]
    pub force: bool,
}

// Profile commands
#[derive(Subcommand, Debug)]
pub enum ProfileCommands {
    /// List available profiles
    List(ProfileListArgs),

    /// Install all extensions in a profile
    Install(ProfileInstallArgs),

    /// Reinstall all extensions in a profile
    Reinstall(ProfileReinstallArgs),

    /// Show profile information
    Info(ProfileInfoArgs),

    /// Check profile installation status
    Status(ProfileStatusArgs),
}

#[derive(Args, Debug)]
pub struct ProfileListArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct ProfileInstallArgs {
    /// Profile name
    pub profile: String,

    /// Skip confirmation prompt
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Continue on failures (don't stop)
    #[arg(long, default_value = "true")]
    pub continue_on_error: bool,
}

#[derive(Args, Debug)]
pub struct ProfileReinstallArgs {
    /// Profile name
    pub profile: String,

    /// Skip confirmation prompt
    #[arg(short = 'y', long)]
    pub yes: bool,
}

#[derive(Args, Debug)]
pub struct ProfileInfoArgs {
    /// Profile name
    pub profile: String,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct ProfileStatusArgs {
    /// Profile name
    pub profile: String,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

// Project commands
#[derive(Subcommand, Debug)]
pub enum ProjectCommands {
    /// Create a new project from template
    New(NewProjectArgs),

    /// Clone a project repository
    Clone(CloneProjectArgs),
}

#[derive(Args, Debug)]
pub struct NewProjectArgs {
    /// Project name
    pub name: String,

    /// Project type (auto-detected if not specified)
    #[arg(short = 't', long)]
    pub project_type: Option<String>,

    /// Force interactive type selection
    #[arg(short, long)]
    pub interactive: bool,

    /// Git user name
    #[arg(long)]
    pub git_name: Option<String>,

    /// Git user email
    #[arg(long)]
    pub git_email: Option<String>,

    /// Skip agentic tools
    #[arg(long)]
    pub skip_tools: bool,
}

#[derive(Args, Debug)]
pub struct CloneProjectArgs {
    /// Repository URL
    pub repository: String,

    /// Fork before cloning
    #[arg(short, long)]
    pub fork: bool,

    /// Branch to checkout
    #[arg(short, long)]
    pub branch: Option<String>,

    /// Clone depth
    #[arg(short, long)]
    pub depth: Option<u32>,

    /// Git user name
    #[arg(long)]
    pub git_name: Option<String>,

    /// Git user email
    #[arg(long)]
    pub git_email: Option<String>,

    /// Feature branch to create
    #[arg(long)]
    pub feature: Option<String>,

    /// Skip dependency installation
    #[arg(long)]
    pub no_deps: bool,

    /// Skip agentic tools
    #[arg(long)]
    pub skip_tools: bool,

    /// Skip enhancements
    #[arg(long)]
    pub no_enhance: bool,
}

// Doctor command
#[derive(Args, Debug)]
pub struct DoctorArgs {
    /// Check tools for a specific provider (docker, fly, devpod, e2b, k8s)
    #[arg(short, long)]
    pub provider: Option<String>,

    /// Check tools for a specific command (project, extension, secrets, deploy)
    #[arg(long)]
    pub command: Option<String>,

    /// Check all tools regardless of current usage
    #[arg(short, long)]
    pub all: bool,

    /// Exit with non-zero code if required tools are missing (for CI)
    #[arg(long)]
    pub ci: bool,

    /// Output format: human (default), json, yaml
    #[arg(long, default_value = "human")]
    pub format: String,

    /// Show detailed information including timing
    #[arg(long)]
    pub verbose_output: bool,

    /// Check authentication status for tools that require it
    #[arg(long)]
    pub check_auth: bool,

    /// Attempt to install missing tools
    #[arg(long)]
    pub fix: bool,

    /// Skip confirmation prompts when installing (use with --fix)
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Show what would be installed without actually installing (use with --fix)
    #[arg(long)]
    pub dry_run: bool,

    /// Check tools required by installed extensions
    #[arg(long)]
    pub check_extensions: bool,

    /// Check a specific extension's tool requirements
    #[arg(long, value_name = "NAME")]
    pub extension: Option<String>,
}

// K8s cluster management commands
#[derive(Subcommand, Debug)]
pub enum K8sCommands {
    /// Create a local Kubernetes cluster
    Create(K8sCreateArgs),

    /// Destroy a local Kubernetes cluster
    Destroy(K8sDestroyArgs),

    /// List local Kubernetes clusters
    List(K8sListArgs),

    /// Show cluster status
    Status(K8sStatusArgs),

    /// Show kubeconfig for a cluster
    Config(K8sConfigArgs),

    /// Install cluster management tools (kind/k3d)
    Install(K8sInstallArgs),
}

#[derive(Args, Debug)]
#[command(disable_version_flag = true)]
pub struct K8sCreateArgs {
    /// Cluster provider (kind, k3d)
    #[arg(short, long, default_value = "kind")]
    pub provider: String,

    /// Cluster name
    #[arg(short, long, default_value = "sindri-local")]
    pub name: String,

    /// Number of nodes (1 = single node, >1 = 1 control-plane + N-1 workers)
    #[arg(short = 'N', long, default_value = "1")]
    pub nodes: u32,

    /// Kubernetes version
    #[arg(long = "k8s-version", default_value = "v1.35.0")]
    pub version: String,

    /// Enable local registry (k3d only)
    #[arg(long)]
    pub registry: bool,

    /// Registry port (k3d only, default: 5000)
    #[arg(long, default_value = "5000")]
    pub registry_port: u16,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct K8sDestroyArgs {
    /// Cluster name
    #[arg(short, long, default_value = "sindri-local")]
    pub name: String,

    /// Skip confirmation
    #[arg(short, long)]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct K8sListArgs {
    /// Cluster provider (kind, k3d, or all)
    #[arg(short, long)]
    pub provider: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct K8sStatusArgs {
    /// Cluster name
    #[arg(short, long, default_value = "sindri-local")]
    pub name: String,

    /// Cluster provider (kind, k3d - auto-detected if not specified)
    #[arg(short, long)]
    pub provider: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct K8sConfigArgs {
    /// Cluster name
    #[arg(short, long, default_value = "sindri-local")]
    pub name: String,

    /// Cluster provider (kind, k3d - auto-detected if not specified)
    #[arg(short, long)]
    pub provider: Option<String>,
}

#[derive(Args, Debug)]
pub struct K8sInstallArgs {
    /// Tool to install (kind, k3d)
    pub tool: String,

    /// Skip confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,
}

// Image management commands
#[derive(Subcommand, Debug)]
pub enum ImageCommands {
    /// List available images from registry
    List(ImageListArgs),

    /// Inspect image details
    Inspect(ImageInspectArgs),

    /// Verify image signature and provenance
    Verify(ImageVerifyArgs),

    /// Show version compatibility matrix
    Versions(ImageVersionsArgs),

    /// Show currently deployed image
    Current(ImageCurrentArgs),
}

#[derive(Args, Debug)]
pub struct ImageListArgs {
    /// Registry URL (default: ghcr.io)
    #[arg(long, default_value = "ghcr.io")]
    pub registry: String,

    /// Repository name (default: pacphi/sindri)
    #[arg(long)]
    pub repository: Option<String>,

    /// Filter tags by pattern (regex)
    #[arg(long)]
    pub filter: Option<String>,

    /// Include prerelease versions
    #[arg(long)]
    pub include_prerelease: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct ImageInspectArgs {
    /// Image tag to inspect
    pub tag: String,

    /// Show image digest
    #[arg(long)]
    pub digest: bool,

    /// Download and show SBOM
    #[arg(long)]
    pub sbom: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct ImageVerifyArgs {
    /// Image tag to verify
    pub tag: String,

    /// Skip signature verification
    #[arg(long)]
    pub no_signature: bool,

    /// Skip provenance verification
    #[arg(long)]
    pub no_provenance: bool,
}

#[derive(Args, Debug)]
pub struct ImageVersionsArgs {
    /// CLI version to check compatibility for
    #[arg(long)]
    pub cli_version: Option<String>,

    /// Output format (table, json)
    #[arg(long, default_value = "table")]
    pub format: String,
}

#[derive(Args, Debug)]
pub struct ImageCurrentArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

// VM commands for image building with HashiCorp Packer
#[derive(Subcommand, Debug)]
pub enum VmCommands {
    /// Build a VM image
    Build(VmBuildArgs),

    /// Validate a Packer template
    Validate(VmValidateArgs),

    /// List built images
    List(VmListArgs),

    /// Delete a VM image
    Delete(VmDeleteArgs),

    /// Check Packer prerequisites
    Doctor(VmDoctorArgs),

    /// Initialize Packer configuration
    Init(VmInitArgs),

    /// Deploy an instance from an image
    Deploy(VmDeployArgs),
}

#[derive(Args, Debug)]
pub struct VmBuildArgs {
    /// Target cloud provider (aws, azure, gcp, oci, alibaba)
    #[arg(long)]
    pub cloud: String,

    /// Image name prefix
    #[arg(short, long)]
    pub name: Option<String>,

    /// Sindri version to install in image
    #[arg(long, default_value = "latest")]
    pub sindri_version: String,

    /// Extension profile to install
    #[arg(long)]
    pub profile: Option<String>,

    /// Additional extensions to install (comma-separated)
    #[arg(long)]
    pub extensions: Option<String>,

    /// Cloud region
    #[arg(short, long)]
    pub region: Option<String>,

    /// Instance type / VM size
    #[arg(long)]
    pub instance_type: Option<String>,

    /// Disk size in GB
    #[arg(long)]
    pub disk_size: Option<u32>,

    /// Enable CIS security hardening
    #[arg(long)]
    pub cis_hardening: bool,

    /// Force rebuild even if cached image exists
    #[arg(short, long)]
    pub force: bool,

    /// Dry run - generate template without building
    #[arg(long)]
    pub dry_run: bool,

    /// Enable debug output
    #[arg(long)]
    pub debug: bool,

    /// Path to variable file
    #[arg(long)]
    pub var_file: Option<camino::Utf8PathBuf>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct VmValidateArgs {
    /// Target cloud provider
    #[arg(long)]
    pub cloud: String,

    /// Image name prefix
    #[arg(short, long)]
    pub name: Option<String>,

    /// Sindri version
    #[arg(long, default_value = "latest")]
    pub sindri_version: String,

    /// Syntax check only
    #[arg(long)]
    pub syntax_only: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct VmListArgs {
    /// Target cloud provider
    #[arg(long)]
    pub cloud: String,

    /// Filter by name prefix
    #[arg(short, long)]
    pub name: Option<String>,

    /// Cloud region
    #[arg(short, long)]
    pub region: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct VmDeleteArgs {
    /// Target cloud provider
    #[arg(long)]
    pub cloud: String,

    /// Image ID to delete
    pub image_id: String,

    /// Cloud region
    #[arg(short, long)]
    pub region: Option<String>,

    /// Skip confirmation
    #[arg(short, long)]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct VmDoctorArgs {
    /// Target cloud provider (or all)
    #[arg(long)]
    pub cloud: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct VmInitArgs {
    /// Target cloud provider
    #[arg(long)]
    pub cloud: String,

    /// Output directory for generated files
    #[arg(short, long)]
    pub output: Option<camino::Utf8PathBuf>,

    /// Force overwrite existing files
    #[arg(short, long)]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct VmDeployArgs {
    /// Target cloud provider
    #[arg(long)]
    pub cloud: String,

    /// Image ID to deploy
    pub image_id: String,

    /// Cloud region
    #[arg(short, long)]
    pub region: Option<String>,

    /// Instance type / VM size
    #[arg(long)]
    pub instance_type: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

// BOM commands
#[derive(Subcommand, Debug)]
pub enum BomCommands {
    /// Generate BOM from installed extensions
    Generate(BomGenerateArgs),

    /// Show BOM for specific extension
    Show(BomShowArgs),

    /// List all components
    List(BomListArgs),

    /// Export BOM to file
    Export(BomExportArgs),
}

#[derive(Args, Debug)]
pub struct BomGenerateArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Detect versions by running validation commands
    #[arg(long)]
    pub detect_versions: bool,
}

#[derive(Args, Debug)]
pub struct BomShowArgs {
    /// Extension name
    pub extension: String,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Detect versions
    #[arg(long)]
    pub detect_versions: bool,
}

#[derive(Args, Debug)]
pub struct BomListArgs {
    /// Extension name (optional)
    pub extension: Option<String>,

    /// Filter by type (tool, runtime, library)
    #[arg(short = 't', long)]
    pub component_type: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct BomExportArgs {
    /// Format (json, yaml, cyclonedx, spdx)
    #[arg(short, long, default_value = "json")]
    pub format: String,

    /// Output file
    #[arg(short, long, default_value = "bom.json")]
    pub output: Utf8PathBuf,

    /// Detect versions
    #[arg(long)]
    pub detect_versions: bool,

    /// Force overwrite
    #[arg(long)]
    pub force: bool,
}

// Completions command
#[derive(Args, Debug)]
pub struct CompletionsArgs {
    /// Shell to generate completions for
    #[arg(value_enum)]
    pub shell: clap_complete::Shell,
}
