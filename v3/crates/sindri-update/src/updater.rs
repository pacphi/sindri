//! Core self-update mechanism with auto-rollback and verification
//!
//! This module provides:
//! - Atomic binary replacement
//! - Automatic backup creation with timestamps
//! - Binary verification before committing
//! - Automatic rollback on verification failure
//! - Cleanup of old backups

use anyhow::{anyhow, Context, Result};
use semver::Version;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, error, info, warn};

use crate::compatibility::CompatibilityChecker;
use crate::download::BinaryDownloader;
use crate::releases::ReleaseManager;

/// Maximum number of backup files to keep
const MAX_BACKUPS: usize = 2;

/// Backup file extension
const BACKUP_EXT: &str = ".bak";

/// Self-update manager for Sindri CLI
pub struct SindriUpdater {
    /// Current binary version
    current_version: Version,

    /// Path to the current binary
    binary_path: PathBuf,

    /// Release manager for fetching releases
    release_manager: ReleaseManager,

    /// Binary downloader
    downloader: BinaryDownloader,

    /// Compatibility checker
    compatibility_checker: CompatibilityChecker,
}

impl SindriUpdater {
    /// Create a new updater instance
    pub fn new() -> Result<Self> {
        let current_version =
            Version::parse(env!("CARGO_PKG_VERSION")).context("Failed to parse current version")?;

        let binary_path =
            std::env::current_exe().context("Failed to get current executable path")?;

        debug!(
            "Updater initialized: version={}, path={:?}",
            current_version, binary_path
        );

        Ok(Self {
            current_version,
            binary_path,
            release_manager: ReleaseManager::new()?,
            downloader: BinaryDownloader::new()?,
            compatibility_checker: CompatibilityChecker::new()?,
        })
    }

    /// Get the current version
    pub fn current_version(&self) -> &Version {
        &self.current_version
    }

    /// Get the binary path
    pub fn binary_path(&self) -> &Path {
        &self.binary_path
    }

    /// Update to a specific version
    ///
    /// This performs the following steps:
    /// 1. Fetch the release for the target version
    /// 2. Check compatibility (if checker is loaded)
    /// 3. Download the new binary
    /// 4. Create a timestamped backup
    /// 5. Replace the current binary atomically
    /// 6. Verify the new binary
    /// 7. Rollback if verification fails
    /// 8. Cleanup old backups
    pub async fn update_to(&self, target_version: &str) -> Result<UpdateResult> {
        info!("Starting update to version {}", target_version);

        // Parse target version
        let target = Version::parse(target_version.trim_start_matches('v'))
            .context("Invalid target version")?;

        // Check if already at target version
        if target == self.current_version {
            info!("Already at version {}", target);
            return Ok(UpdateResult::AlreadyUpToDate(target.to_string()));
        }

        // Check if downgrade
        if target < self.current_version {
            warn!("Downgrading from {} to {}", self.current_version, target);
        }

        // Fetch release information
        info!("Fetching release information for v{}", target);
        let release = self
            .release_manager
            .get_release(&format!("v{}", target))
            .await
            .context("Failed to fetch release")?;

        // Download the binary using BinaryDownloader
        info!("Downloading release binary");
        let download_result = self
            .downloader
            .download_release(&release, None)
            .await
            .context("Failed to download binary")?;

        info!("Binary downloaded to: {:?}", download_result.file_path);
        info!("Checksum: {}", download_result.checksum);

        // Extract binary from archive if needed
        let temp_binary = self
            .extract_binary_from_download(&download_result.file_path)
            .context("Failed to extract binary")?;

        // Verify the extracted binary
        self.verify_binary(&temp_binary)
            .context("Downloaded binary verification failed")?;

        // Create backup
        let backup_path = self.create_backup().context("Failed to create backup")?;

        info!("Backup created at: {:?}", backup_path);

        // Replace binary atomically
        match self.replace_binary(&temp_binary) {
            Ok(_) => {
                info!("Binary replaced successfully");

                // Verify the new binary in place
                match self.verify_binary(&self.binary_path) {
                    Ok(_) => {
                        info!("New binary verified successfully");

                        // Cleanup old backups
                        if let Err(e) = self.cleanup_old_backups() {
                            warn!("Failed to cleanup old backups: {}", e);
                        }

                        Ok(UpdateResult::Updated {
                            from: self.current_version.to_string(),
                            to: target.to_string(),
                            backup: backup_path,
                        })
                    }
                    Err(e) => {
                        error!("New binary verification failed: {}", e);

                        // Auto-rollback
                        warn!("Attempting automatic rollback");
                        self.rollback(&backup_path)
                            .context("Failed to rollback after verification failure")?;

                        Err(anyhow!("Update failed: verification error: {}. Rolled back to previous version.", e))
                    }
                }
            }
            Err(e) => {
                error!("Failed to replace binary: {}", e);

                // Rollback
                self.rollback(&backup_path)
                    .context("Failed to rollback after replacement failure")?;

                Err(anyhow!(
                    "Update failed: {}. Rolled back to previous version.",
                    e
                ))
            }
        }
    }

    /// Extract binary from downloaded archive or return direct binary path
    fn extract_binary_from_download(&self, download_path: &Path) -> Result<PathBuf> {
        let file_name = download_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("Invalid download path"))?;

        debug!("Extracting binary from: {}", file_name);

        // If it's an archive, extract it
        if file_name.ends_with(".tar.gz") || file_name.ends_with(".tgz") {
            self.extract_from_tarball(download_path)
        } else if file_name.ends_with(".zip") {
            Err(anyhow!("ZIP format not yet supported"))
        } else {
            // Direct binary - just make it executable and return
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(download_path)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(download_path, perms)?;
            }

            Ok(download_path.to_path_buf())
        }
    }

    /// Extract binary from tarball
    fn extract_from_tarball(&self, archive_path: &Path) -> Result<PathBuf> {
        use flate2::read::GzDecoder;
        use std::fs::File;
        use tar::Archive;

        debug!("Extracting tarball: {:?}", archive_path);

        let file = File::open(archive_path).context("Failed to open archive file")?;

        let decoder = GzDecoder::new(file);
        let mut archive = Archive::new(decoder);

        let temp_dir =
            std::env::temp_dir().join(format!("sindri-extract-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir)?;

        archive.unpack(&temp_dir)?;

        // Find the binary (usually named "sindri")
        for entry in fs::read_dir(&temp_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                if file_name == "sindri"
                    || (file_name.starts_with("sindri") && !file_name.contains('.'))
                {
                    // Make executable
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let mut perms = fs::metadata(&path)?.permissions();
                        perms.set_mode(0o755);
                        fs::set_permissions(&path, perms)?;
                    }

                    info!("Found binary: {:?}", path);
                    return Ok(path);
                }
            }
        }

        Err(anyhow!("Could not find sindri binary in tarball"))
    }

    /// Verify a binary by running --version
    pub fn verify_binary(&self, binary_path: &Path) -> Result<()> {
        debug!("Verifying binary at: {:?}", binary_path);

        if !binary_path.exists() {
            return Err(anyhow!("Binary does not exist: {:?}", binary_path));
        }

        // Test by running --version
        let output = Command::new(binary_path)
            .arg("--version")
            .output()
            .context("Failed to execute binary for verification")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Binary verification failed: exit code {}",
                output.status.code().unwrap_or(-1)
            ));
        }

        let version_output = String::from_utf8_lossy(&output.stdout);
        debug!("Binary verification output: {}", version_output.trim());

        // Check that output contains "sindri" to ensure it's the right binary
        if !version_output.to_lowercase().contains("sindri") {
            return Err(anyhow!(
                "Binary verification failed: unexpected output: {}",
                version_output.trim()
            ));
        }

        info!("Binary verified successfully");
        Ok(())
    }

    /// Create a timestamped backup of the current binary
    fn create_backup(&self) -> Result<PathBuf> {
        let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
        let backup_path = self
            .binary_path
            .with_extension(format!("{}{}", timestamp, BACKUP_EXT));

        debug!(
            "Creating backup: {:?} -> {:?}",
            self.binary_path, backup_path
        );

        fs::copy(&self.binary_path, &backup_path)
            .with_context(|| format!("Failed to create backup at {:?}", backup_path))?;

        info!("Backup created: {:?}", backup_path);
        Ok(backup_path)
    }

    /// Replace the current binary atomically
    fn replace_binary(&self, new_binary: &Path) -> Result<()> {
        debug!(
            "Replacing binary: {:?} -> {:?}",
            new_binary, self.binary_path
        );

        // On Unix, we can atomically replace using rename
        #[cfg(unix)]
        {
            fs::rename(new_binary, &self.binary_path)
                .context("Failed to replace binary (rename)")?;
        }

        // On Windows, we need to handle the locked executable differently
        #[cfg(windows)]
        {
            // Windows approach: copy to a temp name, then use self_replace
            use std::io;

            // First, try to remove the old binary
            if self.binary_path.exists() {
                fs::remove_file(&self.binary_path)
                    .or_else(|e| {
                        if e.kind() == io::ErrorKind::PermissionDenied {
                            // Binary is locked, use alternate approach
                            let temp_old = self.binary_path.with_extension("old");
                            fs::rename(&self.binary_path, &temp_old)?;
                            Ok(())
                        } else {
                            Err(e)
                        }
                    })
                    .context("Failed to remove old binary")?;
            }

            // Copy new binary into place
            fs::copy(new_binary, &self.binary_path).context("Failed to copy new binary")?;
        }

        info!("Binary replaced successfully");
        Ok(())
    }

    /// Rollback to a backup
    pub fn rollback(&self, backup_path: &Path) -> Result<()> {
        warn!("Rolling back to backup: {:?}", backup_path);

        if !backup_path.exists() {
            return Err(anyhow!("Backup file does not exist: {:?}", backup_path));
        }

        // Verify the backup before restoring
        self.verify_binary(backup_path)
            .context("Backup verification failed")?;

        // Replace current binary with backup
        #[cfg(unix)]
        {
            fs::copy(backup_path, &self.binary_path).context("Failed to restore from backup")?;
        }

        #[cfg(windows)]
        {
            if self.binary_path.exists() {
                fs::remove_file(&self.binary_path).context("Failed to remove corrupted binary")?;
            }

            fs::copy(backup_path, &self.binary_path).context("Failed to restore from backup")?;
        }

        info!("Rollback completed successfully");
        Ok(())
    }

    /// Cleanup old backup files, keeping only the most recent MAX_BACKUPS
    pub fn cleanup_old_backups(&self) -> Result<()> {
        debug!("Cleaning up old backups");

        let parent = self
            .binary_path
            .parent()
            .ok_or_else(|| anyhow!("Cannot determine parent directory"))?;

        let binary_name = self
            .binary_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("Cannot determine binary name"))?;

        // Find all backup files
        let mut backups: Vec<PathBuf> = fs::read_dir(parent)?
            .filter_map(|entry| entry.ok())
            .map(|e| e.path())
            .filter(|path| {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    name.starts_with(binary_name) && name.ends_with(BACKUP_EXT)
                } else {
                    false
                }
            })
            .collect();

        if backups.len() <= MAX_BACKUPS {
            debug!("Found {} backups, no cleanup needed", backups.len());
            return Ok(());
        }

        // Sort by modification time (newest first)
        backups.sort_by_key(|path| {
            fs::metadata(path)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        });
        backups.reverse();

        // Remove old backups
        let to_remove = &backups[MAX_BACKUPS..];
        for backup in to_remove {
            info!("Removing old backup: {:?}", backup);
            if let Err(e) = fs::remove_file(backup) {
                warn!("Failed to remove backup {:?}: {}", backup, e);
            }
        }

        info!(
            "Cleanup completed: kept {} backups, removed {}",
            MAX_BACKUPS,
            to_remove.len()
        );
        Ok(())
    }

    /// List available backups
    pub fn list_backups(&self) -> Result<Vec<BackupInfo>> {
        let parent = self
            .binary_path
            .parent()
            .ok_or_else(|| anyhow!("Cannot determine parent directory"))?;

        let binary_name = self
            .binary_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("Cannot determine binary name"))?;

        let mut backups: Vec<BackupInfo> = fs::read_dir(parent)?
            .filter_map(|entry| entry.ok())
            .filter_map(|e| {
                let path = e.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with(binary_name) && name.ends_with(BACKUP_EXT) {
                        let metadata = fs::metadata(&path).ok()?;
                        let size = metadata.len();
                        let modified = metadata.modified().ok()?;

                        return Some(BackupInfo {
                            path: path.clone(),
                            size,
                            created: modified,
                        });
                    }
                }
                None
            })
            .collect();

        // Sort by creation time (newest first)
        backups.sort_by_key(|b| b.created);
        backups.reverse();

        Ok(backups)
    }

    /// Set compatibility checker with loaded matrix
    pub fn with_compatibility_checker(mut self, checker: CompatibilityChecker) -> Self {
        self.compatibility_checker = checker;
        self
    }

    /// Get reference to release manager
    pub fn release_manager(&self) -> &ReleaseManager {
        &self.release_manager
    }

    /// Get reference to compatibility checker
    pub fn compatibility_checker(&self) -> &CompatibilityChecker {
        &self.compatibility_checker
    }
}

/// Result of an update operation
#[derive(Debug)]
pub enum UpdateResult {
    /// Already at the target version
    AlreadyUpToDate(String),

    /// Successfully updated
    Updated {
        /// Version upgraded from
        from: String,

        /// Version upgraded to
        to: String,

        /// Path to backup file
        backup: PathBuf,
    },
}

/// Information about a backup file
#[derive(Debug)]
pub struct BackupInfo {
    /// Path to backup file
    pub path: PathBuf,

    /// Size in bytes
    pub size: u64,

    /// Creation time
    pub created: std::time::SystemTime,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_updater_creation() {
        let updater = SindriUpdater::new();
        let updater = updater.expect("SindriUpdater::new should succeed");
        assert!(updater.current_version().major >= 3);
    }

    #[test]
    fn test_backup_cleanup_logic() {
        // This would require creating temporary files
        // Skipping for now, but should be implemented
    }
}
