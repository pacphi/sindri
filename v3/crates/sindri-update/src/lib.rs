//! Self-update functionality for Sindri CLI
//!
//! Provides:
//! - Version checking against GitHub releases
//! - Self-update with checksum verification
//! - Compatibility matrix checking

pub mod compatibility;
pub mod releases;
pub mod version;

pub use compatibility::CompatibilityChecker;
pub use releases::ReleaseManager;
pub use version::VersionInfo;

/// Current CLI version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// GitHub repository owner
pub const REPO_OWNER: &str = "pacphi";

/// GitHub repository name
pub const REPO_NAME: &str = "sindri";
