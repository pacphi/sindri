//! Self-update functionality for Sindri CLI
//!
//! Provides:
//! - Version checking against GitHub releases
//! - Self-update with checksum verification
//! - Compatibility matrix checking
//! - Binary download with progress tracking
//! - Atomic binary replacement with auto-rollback
//! - Timestamped backup management
//! - Binary verification before committing updates

pub mod compatibility;
pub mod download;
pub mod releases;
pub mod updater;
pub mod version;

pub use compatibility::CompatibilityChecker;
pub use download::{BinaryDownloader, DownloadProgress, DownloadResult};
pub use releases::ReleaseManager;
pub use updater::{BackupInfo, SindriUpdater, UpdateResult};
pub use version::VersionInfo;

/// Current CLI version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// GitHub repository owner
pub const REPO_OWNER: &str = "pacphi";

/// GitHub repository name
pub const REPO_NAME: &str = "sindri";
