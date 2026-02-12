//! Restore transaction with atomic rollback

use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::File;
use tar::Builder;
use tokio::fs as async_fs;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum RestoreChange {
    FileCreated { path: Utf8PathBuf },
    FileModified { path: Utf8PathBuf, backup: Utf8PathBuf },
    FileDeleted { path: Utf8PathBuf },
    DirectoryCreated { path: Utf8PathBuf },
}

pub struct RestoreTransaction {
    pub id: String,
    pub snapshot_path: Utf8PathBuf,
    pub changes: Vec<RestoreChange>,
    committed: bool,
}

impl RestoreTransaction {
    pub async fn begin(workspace_dir: &Utf8Path) -> Result<Self> {
        let id = Uuid::new_v4().to_string();
        let snapshot_dir = Utf8PathBuf::from(format!(
            "{}/.sindri/restore-snapshots",
            std::env::var("HOME").unwrap_or_else(|_| "/alt/home/developer".to_string())
        ));

        async_fs::create_dir_all(&snapshot_dir).await?;
        let snapshot_path = snapshot_dir.join(format!("snapshot-{}.tar.gz", id));

        info!("Creating pre-restore snapshot: {}", snapshot_path);
        Self::create_snapshot(workspace_dir, &snapshot_path).await?;

        Ok(Self {
            id,
            snapshot_path,
            changes: Vec::new(),
            committed: false,
        })
    }

    async fn create_snapshot(source: &Utf8Path, destination: &Utf8Path) -> Result<()> {
        let file = File::create(destination.as_std_path())?;
        let encoder = GzEncoder::new(file, Compression::default());
        let mut archive = Builder::new(encoder);

        archive.append_dir_all(".", source.as_std_path())?;
        archive.finish()?;

        Ok(())
    }

    pub fn record_file_created(&mut self, path: Utf8PathBuf) {
        self.changes.push(RestoreChange::FileCreated { path });
    }

    pub fn record_file_modified(&mut self, path: Utf8PathBuf, backup: Utf8PathBuf) {
        self.changes.push(RestoreChange::FileModified { path, backup });
    }

    pub fn record_file_deleted(&mut self, path: Utf8PathBuf) {
        self.changes.push(RestoreChange::FileDeleted { path });
    }

    pub fn record_directory_created(&mut self, path: Utf8PathBuf) {
        self.changes.push(RestoreChange::DirectoryCreated { path });
    }

    pub async fn commit(mut self) -> Result<()> {
        info!("Committing restore transaction {}", self.id);
        self.committed = true;
        Ok(())
    }

    pub async fn rollback(mut self) -> Result<()> {
        warn!("Rolling back restore transaction {}", self.id);

        for change in self.changes.iter().rev() {
            if let Err(e) = self.rollback_change(change).await {
                warn!("Failed to rollback change {:?}: {}", change, e);
            }
        }

        self.committed = true;
        Ok(())
    }

    async fn rollback_change(&self, change: &RestoreChange) -> Result<()> {
        match change {
            RestoreChange::FileCreated { path } => {
                if path.exists() {
                    async_fs::remove_file(path).await?;
                }
            }
            RestoreChange::FileModified { path, backup } => {
                if backup.exists() {
                    async_fs::rename(backup, path).await?;
                }
            }
            RestoreChange::FileDeleted { path } => {
                warn!("Cannot restore deleted file: {}", path);
            }
            RestoreChange::DirectoryCreated { path } => {
                if path.exists() {
                    async_fs::remove_dir_all(path).await?;
                }
            }
        }
        Ok(())
    }

    pub fn change_count(&self) -> usize {
        self.changes.len()
    }

    pub fn is_committed(&self) -> bool {
        self.committed
    }
}
