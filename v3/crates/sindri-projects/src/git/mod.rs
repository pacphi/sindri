//! Git operations module
//!
//! This module provides async Git operations for project management including:
//! - Repository initialization
//! - Cloning and forking repositories
//! - Git configuration management
//! - Remote repository management
//! - Branch operations
//!
//! # Examples
//!
//! ## Initialize a new repository
//!
//! ```no_run
//! use sindri_projects::git::{init_repository, InitOptions};
//! use sindri_core::types::GitWorkflowConfig;
//! use camino::Utf8Path;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let path = Utf8Path::new("/tmp/my-repo");
//! let options = InitOptions::default();
//! let git_config = GitWorkflowConfig::default();
//! init_repository(path, &options, &git_config).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Clone a repository
//!
//! ```no_run
//! use sindri_projects::git::{clone_repository, CloneOptions};
//! use camino::Utf8Path;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let url = "https://github.com/user/repo.git";
//! let dest = Utf8Path::new("/tmp/my-clone");
//! let options = CloneOptions {
//!     depth: Some(1),
//!     branch: Some("main".to_string()),
//!     ..Default::default()
//! };
//! clone_repository(url, dest, &options).await?;
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
//! let url = "https://github.com/original/repo.git";
//! let dest = Utf8Path::new("/tmp/my-fork");
//! let options = ForkOptions {
//!     setup_aliases: true,
//!     feature_branch: Some("my-feature".to_string()),
//!     ..Default::default()
//! };
//! fork_repository(url, dest, &options).await?;
//! # Ok(())
//! # }
//! ```

mod clone;
mod config;
mod init;
mod remote;

// Re-export public API
pub use clone::{clone_repository, fork_repository, CloneOptions, ForkOptions};
pub use config::{
    branch_exists, configure_user, detect_main_branch, get_config_value, get_current_branch,
    set_config_value, setup_fork_aliases, setup_fork_aliases_with_config, ConfigScope,
};
pub use init::{checkout_branch, create_branch, init_repository, InitOptions};
pub use remote::{
    add_remote, fetch_remote, get_remote_url, list_remotes, remote_exists, remove_remote,
    setup_fork_remotes, setup_fork_remotes_with_config,
};
