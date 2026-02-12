//! Project management commands (new, clone)
//!
//! Implements project scaffolding with:
//! - Template-based project creation from project-templates.yaml
//! - Intelligent project type detection from names
//! - Extension activation for language/framework support
//! - Dependency detection and installation
//!
//! References ADR-024: Template-Based Project Scaffolding

mod clone;
mod enhance;
mod new;
mod template;

use anyhow::Result;

use crate::cli::ProjectCommands;

/// Run project subcommands
pub async fn run(cmd: ProjectCommands) -> Result<()> {
    match cmd {
        ProjectCommands::New(args) => new::run(args).await,
        ProjectCommands::Clone(args) => clone::run(args).await,
    }
}
