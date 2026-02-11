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

    // Initialize tracing
    init_tracing(cli.verbose, cli.quiet);

    // Run command
    match cli.command {
        Commands::Version(args) => commands::version::run(args),
        Commands::Config(args) => commands::config::run(args).await,
        Commands::Deploy(args) => commands::deploy::run(args, cli.config.as_deref()).await,
        Commands::Connect(args) => commands::connect::run(args, cli.config.as_deref()).await,
        Commands::Status(args) => commands::status::run(args, cli.config.as_deref()).await,
        Commands::Destroy(args) => commands::destroy::run(args, cli.config.as_deref()).await,
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
