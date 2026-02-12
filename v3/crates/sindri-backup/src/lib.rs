//! Sindri Backup System
//!
//! This crate provides backup and restore functionality for Sindri workspaces.
//! It supports three backup profiles (user-data, standard, full) and implements
//! streaming tar+gzip compression with SHA256 checksums.
//!
//! # Features
//!
//! ## Backup
//! - **Three backup profiles**: user-data (migration), standard (default), full (disaster recovery)
//! - **Streaming compression**: Handles large workspaces efficiently
//! - **Smart filtering**: Excludes caches, build artifacts, regenerable files
//! - **System marker protection**: Never backs up or restores initialization markers
//! - **Progress reporting**: Visual feedback for long operations
//! - **Manifest format**: JSON metadata with checksums and statistics
//!
//! # Examples
//!
//! ```no_run
//! use sindri_backup::{ArchiveBuilder, ArchiveConfig, BackupProfile, SourceInfo};
//! use std::path::Path;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let source_info = SourceInfo {
//!         instance_name: "my-sindri".to_string(),
//!         provider: "docker".to_string(),
//!         hostname: "localhost".to_string(),
//!     };
//!
//!     let config = ArchiveConfig::new(BackupProfile::Standard, source_info)?;
//!     let builder = ArchiveBuilder::new(config);
//!
//!     let result = builder.create(
//!         Path::new("/alt/home/developer"),
//!         Path::new("backup.tar.gz"),
//!     ).await?;
//!
//!     println!("Backup created: {} bytes", result.size_bytes);
//!     Ok(())
//! }
//! ```

pub mod archive;
pub mod compression;
pub mod filters;
pub mod manifest;
pub mod profile;
pub mod progress;
pub mod restore;

// Re-export commonly used types
pub use archive::{ArchiveBuilder, ArchiveConfig, BackupResult};
pub use compression::{calculate_checksum, CompressionStats, DEFAULT_COMPRESSION_LEVEL};
pub use filters::{ExclusionConfig, RestoreFilter, ALWAYS_EXCLUDE, NEVER_RESTORE};
pub use manifest::{
    BackupManifest, BackupStatistics, ChecksumInfo, ExtensionInfo, SourceInfo, MANIFEST_FILENAME,
    MANIFEST_VERSION,
};
pub use profile::BackupProfile;
pub use progress::{BackupProgress, RestoreProgress, SpinnerProgress};

/// Library version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_manifest_constants() {
        assert_eq!(MANIFEST_FILENAME, ".backup-manifest.json");
        assert_eq!(MANIFEST_VERSION, "1.0.0");
    }

    #[test]
    fn test_compression_level() {
        assert_eq!(DEFAULT_COMPRESSION_LEVEL, 6);
    }

    #[test]
    fn test_always_exclude_not_empty() {
        assert!(!ALWAYS_EXCLUDE.is_empty());
        assert!(ALWAYS_EXCLUDE.contains(&".cache"));
        assert!(ALWAYS_EXCLUDE.contains(&"**/node_modules"));
    }

    #[test]
    fn test_never_restore_not_empty() {
        assert!(!NEVER_RESTORE.is_empty());
        assert!(NEVER_RESTORE.contains(&".initialized"));
        assert!(NEVER_RESTORE.contains(&".welcome_shown"));
    }
}
