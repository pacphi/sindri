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
#[command(propagate_version = true)]
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
    Upgrade(UpgradeArgs),

    /// Secrets management
    #[command(subcommand)]
    Secrets(SecretsCommands),

    /// Backup workspace
    Backup(BackupArgs),

    /// Restore workspace
    Restore(RestoreArgs),
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
}

// Connect command
#[derive(Args, Debug)]
pub struct ConnectArgs {
    /// Command to run instead of shell
    #[arg(short = 'c', long)]
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
}

#[derive(Args, Debug)]
pub struct ExtensionInstallArgs {
    /// Extension name (with optional @version)
    pub name: Option<String>,

    /// Specific version to install
    #[arg(short, long)]
    pub version: Option<String>,

    /// Install extensions from sindri.yaml config file
    #[arg(long, conflicts_with_all = ["name", "profile"])]
    pub from_config: Option<Utf8PathBuf>,

    /// Install all extensions from a profile
    #[arg(long, conflicts_with_all = ["name", "from_config"])]
    pub profile: Option<String>,

    /// Force reinstall if already installed
    #[arg(short, long)]
    pub force: bool,

    /// Skip dependency installation
    #[arg(long)]
    pub no_deps: bool,

    /// Skip confirmation prompt (for profile installation)
    #[arg(short = 'y', long)]
    pub yes: bool,
}

#[derive(Args, Debug)]
pub struct ExtensionListArgs {
    /// Filter by category
    #[arg(short, long)]
    pub category: Option<String>,

    /// Show installed only
    #[arg(long)]
    pub installed: bool,

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
    #[arg(short, long)]
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

// Upgrade command
#[derive(Args, Debug)]
pub struct UpgradeArgs {
    /// Check for updates only
    #[arg(long)]
    pub check: bool,

    /// List available versions
    #[arg(long)]
    pub list: bool,

    /// Install specific version
    #[arg(long)]
    pub version: Option<String>,

    /// Check extension compatibility
    #[arg(long)]
    pub compat: Option<String>,

    /// Include prereleases
    #[arg(long)]
    pub prerelease: bool,
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
