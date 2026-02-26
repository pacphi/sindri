//! Sindri CLI - Declarative cloud development environments
//!
//! This is the main entry point for the Sindri command-line interface.

mod cli;
mod commands;
mod output;
pub mod utils;
mod version;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize rustls crypto provider (required for rustls 0.23+)
    // This must be done before any TLS operations
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    // Parse CLI args
    let cli = Cli::parse();

    // Detect JSON mode from subcommand flags — must happen before any output
    let json_mode = has_json_flag(&cli.command);
    if json_mode {
        output::set_json_mode(true);
    }

    // Initialize tracing (suppress to error-only in JSON mode so tracing
    // INFO/WARN lines don't contaminate stdout — tracing-subscriber's
    // fmt layer writes to stderr by default, but we suppress anyway to
    // keep stderr clean for structured consumers).
    init_tracing(cli.verbose, cli.quiet || json_mode);

    // Run command
    match cli.command {
        Commands::Version(args) => commands::version::run(args),
        Commands::Config(args) => commands::config::run(args).await,
        Commands::Deploy(args) => commands::deploy::run(args, cli.config.as_deref()).await,
        Commands::Connect(args) => commands::connect::run(args, cli.config.as_deref()).await,
        Commands::Status(args) => commands::status::run(args, cli.config.as_deref()).await,
        Commands::Destroy(args) => commands::destroy::run(args, cli.config.as_deref()).await,
        Commands::Start(args) => commands::start::run(args, cli.config.as_deref()).await,
        Commands::Stop(args) => commands::stop::run(args, cli.config.as_deref()).await,
        Commands::Extension(args) => commands::extension::run(args).await,
        Commands::Profile(args) => commands::profile::run(args).await,
        Commands::Upgrade(args) => commands::upgrade::run(args).await,
        Commands::Secrets(args) => commands::secrets::run(args).await,
        Commands::Backup(args) => commands::backup::run(args).await,
        Commands::Restore(args) => commands::restore::run(args).await,
        Commands::Project(args) => commands::project::run(args).await,
        Commands::Doctor(args) => commands::doctor::run(args).await,
        Commands::K8s(args) => commands::k8s::run(args).await,
        Commands::Image(args) => commands::image::execute(args).await,
        Commands::Vm(args) => commands::vm::run(args).await,
        Commands::Bom(args) => commands::bom::run(args).await,
        Commands::Ledger(args) => commands::ledger::handle_ledger_command(args).await,
        Commands::Completions(args) => commands::completions::run(args),
    }
}

/// Initialize tracing with appropriate verbosity
fn init_tracing(verbose: u8, quiet: bool) {
    let filter = if quiet {
        EnvFilter::new("error")
    } else {
        match verbose {
            // Default to info level to show deployment progress (like v2)
            // Use --quiet to suppress, or -v/-vv for more detail
            0 => EnvFilter::new("info"),
            1 => EnvFilter::new("debug"),
            _ => EnvFilter::new("trace"),
        }
    };

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(false))
        .with(filter)
        .init();
}

/// Inspect the parsed command tree and return `true` if any subcommand
/// has its `--json` flag set.  This avoids touching individual command
/// files — the detection happens once in main before dispatch.
fn has_json_flag(cmd: &Commands) -> bool {
    use cli::*;
    match cmd {
        // Top-level commands with json field
        Commands::Version(a) => a.json,
        Commands::Status(a) => a.json,

        // Config subcommands
        Commands::Config(ConfigCommands::Show(a)) => a.json,

        // Extension subcommands
        Commands::Extension(ExtensionCommands::List(a)) => a.json,
        Commands::Extension(ExtensionCommands::Status(a)) => a.json,
        Commands::Extension(ExtensionCommands::Info(a)) => a.json,
        Commands::Extension(ExtensionCommands::Versions(a)) => a.json,
        Commands::Extension(ExtensionCommands::Check(a)) => a.json,
        Commands::Extension(ExtensionCommands::Log(a)) => a.json,

        // Profile subcommands
        Commands::Profile(ProfileCommands::List(a)) => a.json,
        Commands::Profile(ProfileCommands::Info(a)) => a.json,
        Commands::Profile(ProfileCommands::Status(a)) => a.json,

        // K8s subcommands
        Commands::K8s(K8sCommands::Create(a)) => a.json,
        Commands::K8s(K8sCommands::List(a)) => a.json,
        Commands::K8s(K8sCommands::Status(a)) => a.json,

        // Image subcommands
        Commands::Image(ImageCommands::List(a)) => a.json,
        Commands::Image(ImageCommands::Inspect(a)) => a.json,
        Commands::Image(ImageCommands::Current(a)) => a.json,

        // VM subcommands
        Commands::Vm(VmCommands::Build(a)) => a.json,
        Commands::Vm(VmCommands::Validate(a)) => a.json,
        Commands::Vm(VmCommands::List(a)) => a.json,
        Commands::Vm(VmCommands::Doctor(a)) => a.json,
        Commands::Vm(VmCommands::Deploy(a)) => a.json,

        // BOM subcommands
        Commands::Bom(BomCommands::Generate(a)) => a.json,
        Commands::Bom(BomCommands::Show(a)) => a.json,
        Commands::Bom(BomCommands::List(a)) => a.json,

        // Secrets subcommands
        Commands::Secrets(SecretsCommands::List(a)) => a.json,
        Commands::Secrets(SecretsCommands::TestVault(a)) => a.json,

        // Everything else has no json flag
        _ => false,
    }
}
