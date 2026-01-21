//! Sindri CLI - Declarative cloud development environments
//!
//! This is the main entry point for the Sindri command-line interface.

mod cli;
mod commands;
mod output;
mod version;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI args
    let cli = Cli::parse();

    // Initialize tracing
    init_tracing(cli.verbose, cli.quiet);

    // Run command
    match cli.command {
        Commands::Version(args) => commands::version::run(args),
        Commands::Config(args) => commands::config::run(args).await,
        Commands::Deploy(args) => commands::deploy::run(args).await,
        Commands::Connect(args) => commands::connect::run(args).await,
        Commands::Status(args) => commands::status::run(args).await,
        Commands::Destroy(args) => commands::destroy::run(args).await,
        Commands::Extension(args) => commands::extension::run(args).await,
        Commands::Profile(args) => commands::profile::run(args).await,
        Commands::Upgrade(args) => commands::upgrade::run(args).await,
        Commands::Secrets(args) => commands::secrets::run(args).await,
        Commands::Backup(args) => commands::backup::run(args).await,
        Commands::Restore(args) => commands::restore::run(args).await,
    }
}

/// Initialize tracing with appropriate verbosity
fn init_tracing(verbose: u8, quiet: bool) {
    let filter = if quiet {
        EnvFilter::new("error")
    } else {
        match verbose {
            0 => EnvFilter::new("warn"),
            1 => EnvFilter::new("info"),
            2 => EnvFilter::new("debug"),
            _ => EnvFilter::new("trace"),
        }
    };

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(false))
        .with(filter)
        .init();
}
