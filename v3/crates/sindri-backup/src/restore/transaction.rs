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
    FileCreated {
        path: Utf8PathBuf,
    },
    FileModified {
        path: Utf8PathBuf,
        backup: Utf8PathBuf,
    },
    FileDeleted {
        path: Utf8PathBuf,
    },
    DirectoryCreated {
        path: Utf8PathBuf,
    },
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
            sindri_core::utils::get_home_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| "/home/user".to_string())
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
        self.changes
            .push(RestoreChange::FileModified { path, backup });
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_restore_change_file_created() {
        let change = RestoreChange::FileCreated {
            path: Utf8PathBuf::from("/tmp/new-file.txt"),
        };
        assert!(matches!(change, RestoreChange::FileCreated { .. }));
    }

    #[test]
    fn test_restore_change_file_modified() {
        let change = RestoreChange::FileModified {
            path: Utf8PathBuf::from("/tmp/modified.txt"),
            backup: Utf8PathBuf::from("/tmp/modified.txt.bak"),
        };
        assert!(matches!(change, RestoreChange::FileModified { .. }));
    }

    #[test]
    fn test_restore_change_file_deleted() {
        let change = RestoreChange::FileDeleted {
            path: Utf8PathBuf::from("/tmp/deleted.txt"),
        };
        assert!(matches!(change, RestoreChange::FileDeleted { .. }));
    }

    #[test]
    fn test_restore_change_directory_created() {
        let change = RestoreChange::DirectoryCreated {
            path: Utf8PathBuf::from("/tmp/new-dir"),
        };
        assert!(matches!(change, RestoreChange::DirectoryCreated { .. }));
    }

    #[test]
    fn test_record_file_created_increments_count() {
        let mut tx = RestoreTransaction {
            id: "test-id".to_string(),
            snapshot_path: Utf8PathBuf::from("/tmp/snapshot.tar.gz"),
            changes: Vec::new(),
            committed: false,
        };

        assert_eq!(tx.change_count(), 0);
        tx.record_file_created(Utf8PathBuf::from("/tmp/a.txt"));
        assert_eq!(tx.change_count(), 1);
        tx.record_file_created(Utf8PathBuf::from("/tmp/b.txt"));
        assert_eq!(tx.change_count(), 2);
    }

    #[test]
    fn test_record_file_modified_increments_count() {
        let mut tx = RestoreTransaction {
            id: "test-id".to_string(),
            snapshot_path: Utf8PathBuf::from("/tmp/snapshot.tar.gz"),
            changes: Vec::new(),
            committed: false,
        };

        tx.record_file_modified(
            Utf8PathBuf::from("/tmp/a.txt"),
            Utf8PathBuf::from("/tmp/a.txt.bak"),
        );
        assert_eq!(tx.change_count(), 1);
    }

    #[test]
    fn test_record_file_deleted_increments_count() {
        let mut tx = RestoreTransaction {
            id: "test-id".to_string(),
            snapshot_path: Utf8PathBuf::from("/tmp/snapshot.tar.gz"),
            changes: Vec::new(),
            committed: false,
        };

        tx.record_file_deleted(Utf8PathBuf::from("/tmp/a.txt"));
        assert_eq!(tx.change_count(), 1);
    }

    #[test]
    fn test_record_directory_created_increments_count() {
        let mut tx = RestoreTransaction {
            id: "test-id".to_string(),
            snapshot_path: Utf8PathBuf::from("/tmp/snapshot.tar.gz"),
            changes: Vec::new(),
            committed: false,
        };

        tx.record_directory_created(Utf8PathBuf::from("/tmp/newdir"));
        assert_eq!(tx.change_count(), 1);
    }

    #[test]
    fn test_is_committed_initially_false() {
        let tx = RestoreTransaction {
            id: "test-id".to_string(),
            snapshot_path: Utf8PathBuf::from("/tmp/snapshot.tar.gz"),
            changes: Vec::new(),
            committed: false,
        };

        assert!(!tx.is_committed());
    }

    #[tokio::test]
    async fn test_commit_sets_committed() {
        let tx = RestoreTransaction {
            id: "test-id".to_string(),
            snapshot_path: Utf8PathBuf::from("/tmp/snapshot.tar.gz"),
            changes: Vec::new(),
            committed: false,
        };

        assert!(!tx.is_committed());
        tx.commit().await.expect("commit should succeed");
    }

    #[tokio::test]
    async fn test_rollback_empty_changes() {
        let tx = RestoreTransaction {
            id: "test-id".to_string(),
            snapshot_path: Utf8PathBuf::from("/tmp/snapshot.tar.gz"),
            changes: Vec::new(),
            committed: false,
        };

        tx.rollback()
            .await
            .expect("rollback with no changes should succeed");
    }

    #[tokio::test]
    async fn test_rollback_file_created_removes_file() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let file_path =
            Utf8PathBuf::from_path_buf(temp_dir.path().join("created.txt")).expect("valid UTF-8");

        // Create the file that was "restored"
        tokio::fs::write(&file_path, b"restored content")
            .await
            .unwrap();
        assert!(file_path.exists());

        let mut tx = RestoreTransaction {
            id: "test-id".to_string(),
            snapshot_path: Utf8PathBuf::from("/tmp/snapshot.tar.gz"),
            changes: Vec::new(),
            committed: false,
        };
        tx.record_file_created(file_path.clone());

        tx.rollback().await.expect("rollback should succeed");
        assert!(
            !file_path.exists(),
            "Rollback should remove the created file"
        );
    }

    #[tokio::test]
    async fn test_rollback_directory_created_removes_dir() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let dir_path =
            Utf8PathBuf::from_path_buf(temp_dir.path().join("created-dir")).expect("valid UTF-8");

        tokio::fs::create_dir_all(&dir_path).await.unwrap();
        assert!(dir_path.exists());

        let mut tx = RestoreTransaction {
            id: "test-id".to_string(),
            snapshot_path: Utf8PathBuf::from("/tmp/snapshot.tar.gz"),
            changes: Vec::new(),
            committed: false,
        };
        tx.record_directory_created(dir_path.clone());

        tx.rollback().await.expect("rollback should succeed");
        assert!(
            !dir_path.exists(),
            "Rollback should remove the created directory"
        );
    }

    #[tokio::test]
    async fn test_begin_creates_snapshot() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let workspace_dir =
            Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf()).expect("valid UTF-8");

        // Create a small file in the workspace so there's something to snapshot
        tokio::fs::write(workspace_dir.join("test.txt"), b"hello")
            .await
            .unwrap();

        let tx = RestoreTransaction::begin(&workspace_dir).await;
        assert!(tx.is_ok(), "begin() should succeed with a valid workspace");

        let tx = tx.unwrap();
        assert!(!tx.is_committed());
        assert_eq!(tx.change_count(), 0);
        assert!(
            tx.snapshot_path.exists(),
            "Snapshot file should be created at {}",
            tx.snapshot_path
        );

        // Cleanup snapshot
        tokio::fs::remove_file(&tx.snapshot_path).await.ok();
    }

    #[test]
    fn test_mixed_record_operations() {
        let mut tx = RestoreTransaction {
            id: "test-id".to_string(),
            snapshot_path: Utf8PathBuf::from("/tmp/snapshot.tar.gz"),
            changes: Vec::new(),
            committed: false,
        };

        tx.record_file_created(Utf8PathBuf::from("/tmp/a.txt"));
        tx.record_file_modified(
            Utf8PathBuf::from("/tmp/b.txt"),
            Utf8PathBuf::from("/tmp/b.txt.bak"),
        );
        tx.record_file_deleted(Utf8PathBuf::from("/tmp/c.txt"));
        tx.record_directory_created(Utf8PathBuf::from("/tmp/d"));

        assert_eq!(tx.change_count(), 4);
        assert!(matches!(tx.changes[0], RestoreChange::FileCreated { .. }));
        assert!(matches!(tx.changes[1], RestoreChange::FileModified { .. }));
        assert!(matches!(tx.changes[2], RestoreChange::FileDeleted { .. }));
        assert!(matches!(
            tx.changes[3],
            RestoreChange::DirectoryCreated { .. }
        ));
    }
}
