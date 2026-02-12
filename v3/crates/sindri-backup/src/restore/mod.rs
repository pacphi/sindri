//! Restore system

use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use flate2::read::GzDecoder;
use std::fs::File;
use std::time::{Duration, Instant};
use tar::Archive;
use tokio::fs;
use tracing::{debug, info, warn};

pub mod analysis;
pub mod compatibility;
pub mod markers;
pub mod modes;
pub mod transaction;
pub mod validation;

pub use analysis::{BackupAnalysis, BackupAnalyzer};
pub use compatibility::VersionCompatibility;
pub use markers::{filter_system_markers, is_system_marker, NEVER_RESTORE};
pub use modes::{RestoreAction, RestoreMode, RestoreModeHandler};
pub use transaction::{RestoreChange, RestoreTransaction};
pub use validation::{validate_restore_preconditions, RestoreOptions};

#[derive(Debug, Clone)]
pub struct RestoreResult {
    pub restored: usize,
    pub skipped: usize,
    pub backed_up: usize,
    pub duration: Duration,
}

pub struct RestoreManager {
    mode: RestoreMode,
}

impl RestoreManager {
    pub fn new(mode: RestoreMode) -> Self {
        Self { mode }
    }

    pub async fn restore(
        &self,
        archive_path: &Utf8Path,
        destination: &Utf8Path,
        options: RestoreOptions,
    ) -> Result<RestoreResult> {
        let start = Instant::now();

        info!(
            "Starting restore: archive={}, mode={}",
            archive_path,
            self.mode.as_str()
        );

        // Stage 1: Validation
        info!("Stage 1/5: Validating preconditions");
        validate_restore_preconditions(archive_path, &options)?;
        validation::validate_workspace_writable(destination)?;

        // Stage 2: Analysis
        info!("Stage 2/5: Analyzing backup");
        let analyzer = BackupAnalyzer;
        let analysis = analyzer.analyze(archive_path).await?;

        info!(
            "Backup: {} files, version {}",
            analysis.file_count, analysis.manifest.version
        );

        if !analysis.compatibility.compatible && !options.force {
            anyhow::bail!("Backup incompatible: {}", analysis.compatibility.message());
        }

        // Stage 3: Snapshot
        info!("Stage 3/5: Creating pre-restore snapshot");
        let mut transaction = RestoreTransaction::begin(destination).await?;

        // Stage 4: Restore
        info!("Stage 4/5: Extracting and restoring files");
        let result = match self
            .restore_files(archive_path, destination, &options, &mut transaction)
            .await
        {
            Ok(result) => result,
            Err(e) => {
                warn!("Restore failed: {}", e);
                transaction.rollback().await?;
                return Err(e);
            }
        };

        // Stage 5: Commit
        info!("Stage 5/5: Committing restore");
        transaction.commit().await?;

        let duration = start.elapsed();

        info!(
            "Restore complete: restored={}, skipped={}, duration={:?}",
            result.restored, result.skipped, duration
        );

        Ok(RestoreResult {
            restored: result.restored,
            skipped: result.skipped,
            backed_up: result.backed_up,
            duration,
        })
    }

    async fn restore_files(
        &self,
        archive_path: &Utf8Path,
        destination: &Utf8Path,
        options: &RestoreOptions,
        transaction: &mut RestoreTransaction,
    ) -> Result<RestoreResult> {
        let file = File::open(archive_path.as_std_path())?;
        let decoder = GzDecoder::new(file);
        let mut archive = Archive::new(decoder);

        let handler = self.mode.handler();
        let mut restored = 0;
        let mut skipped = 0;
        let mut backed_up = 0;

        for entry in archive.entries()? {
            let mut entry = entry?;
            let entry_path = entry.path()?;
            let entry_path_str = entry_path.to_str().context("Invalid UTF-8 in path")?;
            let entry_path_buf = Utf8PathBuf::from(entry_path_str);

            if entry_path_buf.as_str().ends_with(".backup-manifest.json") {
                continue;
            }

            if is_system_marker(&entry_path_buf) {
                debug!("Skipping system marker: {}", entry_path_buf);
                skipped += 1;
                continue;
            }

            let target_path = destination.join(&entry_path_buf);
            let mut content = Vec::new();
            std::io::Read::read_to_end(&mut entry, &mut content)?;

            let action = if target_path.exists() {
                handler.handle_existing_file(&target_path, &content).await?
            } else {
                handler.handle_new_file(&target_path).await?
            };

            match action {
                RestoreAction::Skip { reason: _ } => {
                    skipped += 1;
                }
                RestoreAction::Overwrite { backed_up_to } => {
                    if !options.dry_run {
                        if let Some(parent) = target_path.parent() {
                            fs::create_dir_all(parent).await?;
                        }
                        fs::write(&target_path, &content).await?;

                        if let Some(backup_path) = backed_up_to {
                            transaction.record_file_modified(target_path.clone(), backup_path);
                            backed_up += 1;
                        } else {
                            transaction.record_file_created(target_path.clone());
                        }
                    }
                    restored += 1;
                }
                RestoreAction::Merged => {
                    restored += 1;
                }
            }
        }

        Ok(RestoreResult {
            restored,
            skipped,
            backed_up,
            duration: Duration::from_secs(0),
        })
    }
}
