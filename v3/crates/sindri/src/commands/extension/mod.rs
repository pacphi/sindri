//! Extension management commands
//!
//! Implements extension management CLI commands:
//! - install: Install an extension with optional version
//! - list: List available extensions with filtering
//! - validate: Validate extension against schema
//! - status: Show installation status
//! - info: Show detailed extension information
//! - upgrade: Upgrade extension to newer version
//! - remove: Remove an installed extension
//! - versions: Show available versions with compatibility
//! - check: Check for extension updates
//! - rollback: Rollback to previous version
//! - support: Update support files
//! - docs: Generate extension documentation
//! - verify: Verify installed extensions
//! - log: View extension event log

mod check;
mod common;
mod docs;
mod info;
mod install;
mod list;
mod log;
mod remove;
mod rollback;
mod status;
mod support;
mod upgrade;
mod validate;
mod verify;
mod versions;

use anyhow::Result;

use crate::cli::ExtensionCommands;

/// Main entry point for extension subcommands
pub async fn run(cmd: ExtensionCommands) -> Result<()> {
    match cmd {
        ExtensionCommands::Install(args) => install::run(args).await,
        ExtensionCommands::List(args) => list::run(args).await,
        ExtensionCommands::Validate(args) => validate::run(args).await,
        ExtensionCommands::Status(args) => status::run(args).await,
        ExtensionCommands::Info(args) => info::run(args).await,
        ExtensionCommands::Upgrade(args) => upgrade::run(args).await,
        ExtensionCommands::Remove(args) => remove::run(args).await,
        ExtensionCommands::Versions(args) => versions::run(args).await,
        ExtensionCommands::Check(args) => check::run(args).await,
        ExtensionCommands::Rollback(args) => rollback::run(args).await,
        ExtensionCommands::UpdateSupportFiles(args) => support::run(args).await,
        ExtensionCommands::Docs(args) => docs::run(args).await,
        ExtensionCommands::Verify(args) => verify::run(args).await,
        ExtensionCommands::Log(args) => log::run(args).await,
    }
}
