//! # sindri-projects
//!
//! Project management library for the Sindri CLI providing:
//! - Git operations (init, clone, fork, config, remotes)
//! - Project scaffolding and templates
//! - Repository management
//!
//! This crate is part of the Sindri v3 Rust migration (Phase 7: Project Management).
//!
//! # Examples
//!
//! ## Initialize a new git repository
//!
//! ```no_run
//! use sindri_projects::git::{init_repository, InitOptions};
//! use camino::Utf8Path;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let path = Utf8Path::new("/tmp/my-project");
//! let options = InitOptions::default();
//! init_repository(path, &options).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Clone a repository with options
//!
//! ```no_run
//! use sindri_projects::git::{clone_repository, CloneOptions};
//! use camino::Utf8Path;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let options = CloneOptions {
//!     depth: Some(1),  // Shallow clone
//!     branch: Some("main".to_string()),
//!     git_name: Some("John Doe".to_string()),
//!     git_email: Some("john@example.com".to_string()),
//!     ..Default::default()
//! };
//!
//! clone_repository(
//!     "https://github.com/user/repo.git",
//!     Utf8Path::new("/tmp/my-clone"),
//!     &options
//! ).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Fork a repository
//!
//! ```no_run
//! use sindri_projects::git::{fork_repository, ForkOptions};
//! use camino::Utf8Path;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let options = ForkOptions {
//!     setup_aliases: true,
//!     feature_branch: Some("add-new-feature".to_string()),
//!     ..Default::default()
//! };
//!
//! fork_repository(
//!     "https://github.com/original/repo.git",
//!     Utf8Path::new("/tmp/my-fork"),
//!     &options
//! ).await?;
//! # Ok(())
//! # }
//! ```

pub mod error;
pub mod git;
pub mod templates;
pub mod types;
pub mod enhancement;

pub use error::{Error, Result};

// Re-export template types for convenience
pub use templates::{
    TemplateLoader, TemplateManager, TemplateRenderer, TemplateVars, TypeDetector,
};
