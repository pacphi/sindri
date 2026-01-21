//! Restore modes: safe, merge, and full
//!
//! This module defines the three restore modes and their behavior when encountering existing files.

use anyhow::Result;
use async_trait::async_trait;
use camino::{Utf8Path, Utf8PathBuf};
use tokio::fs;
use tracing::{debug, info, warn};

/// Restore mode determines how to handle existing files
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreMode {
    /// Never overwrite existing files
    Safe,
    /// Backup existing files to .bak, then overwrite
    Merge,
    /// Overwrite all files (except system markers)
    Full,
}

impl Default for RestoreMode {
    fn default() -> Self {
        Self::Safe
    }
}

/// Action to take for a file during restore
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestoreAction {
    /// Skip the file, do not restore
    Skip { reason: String },
    /// Overwrite the file (optionally backing up first)
    Overwrite { backed_up_to: Option<Utf8PathBuf> },
    /// File was merged (for special cases like shell configs)
    Merged,
}

/// Trait for mode-specific restore behavior
#[async_trait]
pub trait RestoreModeHandler: Send + Sync {
    /// Determine what action to take for an existing file
    async fn handle_existing_file(
        &self,
        target: &Utf8Path,
        backup_content: &[u8],
    ) -> Result<RestoreAction>;

    /// Determine what action to take for a new file
    async fn handle_new_file(&self, target: &Utf8Path) -> Result<RestoreAction> {
        // All modes restore new files
        Ok(RestoreAction::Overwrite {
            backed_up_to: None,
        })
    }
}

/// Handler for Safe mode
pub struct SafeModeHandler;

#[async_trait]
impl RestoreModeHandler for SafeModeHandler {
    async fn handle_existing_file(
        &self,
        target: &Utf8Path,
        _backup_content: &[u8],
    ) -> Result<RestoreAction> {
        debug!("Safe mode: skipping existing file: {}", target);
        Ok(RestoreAction::Skip {
            reason: "File already exists (safe mode)".to_string(),
        })
    }
}

/// Handler for Merge mode
pub struct MergeModeHandler;

#[async_trait]
impl RestoreModeHandler for MergeModeHandler {
    async fn handle_existing_file(
        &self,
        target: &Utf8Path,
        _backup_content: &[u8],
    ) -> Result<RestoreAction> {
        // Create .bak file
        let backup_path = Utf8PathBuf::from(format!("{}.bak", target));

        debug!("Merge mode: backing up {} to {}", target, backup_path);

        // Copy existing file to .bak
        fs::copy(target, &backup_path).await?;

        info!("Backed up existing file: {} -> {}", target, backup_path);

        Ok(RestoreAction::Overwrite {
            backed_up_to: Some(backup_path),
        })
    }
}

/// Handler for Full mode
pub struct FullModeHandler;

#[async_trait]
impl RestoreModeHandler for FullModeHandler {
    async fn handle_existing_file(
        &self,
        target: &Utf8Path,
        _backup_content: &[u8],
    ) -> Result<RestoreAction> {
        warn!("Full mode: overwriting existing file: {}", target);
        Ok(RestoreAction::Overwrite {
            backed_up_to: None,
        })
    }
}

impl RestoreMode {
    /// Get the handler for this mode
    pub fn handler(&self) -> Box<dyn RestoreModeHandler> {
        match self {
            Self::Safe => Box::new(SafeModeHandler),
            Self::Merge => Box::new(MergeModeHandler),
            Self::Full => Box::new(FullModeHandler),
        }
    }

    /// Parse mode from string
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "safe" => Ok(Self::Safe),
            "merge" => Ok(Self::Merge),
            "full" => Ok(Self::Full),
            _ => Err(anyhow::anyhow!(
                "Invalid restore mode: {}. Valid modes: safe, merge, full",
                s
            )),
        }
    }

    /// Get mode name as string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Merge => "merge",
            Self::Full => "full",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_safe_mode_skips_existing() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = Utf8PathBuf::from_path_buf(temp_dir.path().join("test.txt"))
            .unwrap();

        // Create existing file
        fs::write(&test_file, b"existing content").await.unwrap();

        let handler = SafeModeHandler;
        let action = handler
            .handle_existing_file(&test_file, b"new content")
            .await
            .unwrap();

        assert!(matches!(action, RestoreAction::Skip { .. }));

        // Verify file was not modified
        let content = fs::read(&test_file).await.unwrap();
        assert_eq!(content, b"existing content");
    }

    #[tokio::test]
    async fn test_merge_mode_creates_backup() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = Utf8PathBuf::from_path_buf(temp_dir.path().join("test.txt"))
            .unwrap();

        // Create existing file
        fs::write(&test_file, b"existing content").await.unwrap();

        let handler = MergeModeHandler;
        let action = handler
            .handle_existing_file(&test_file, b"new content")
            .await
            .unwrap();

        match action {
            RestoreAction::Overwrite { backed_up_to } => {
                assert!(backed_up_to.is_some());
                let backup_path = backed_up_to.unwrap();

                // Verify backup was created
                assert!(backup_path.exists());
                let backup_content = fs::read(&backup_path).await.unwrap();
                assert_eq!(backup_content, b"existing content");
            }
            _ => panic!("Expected Overwrite action"),
        }
    }

    #[tokio::test]
    async fn test_full_mode_overwrites() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = Utf8PathBuf::from_path_buf(temp_dir.path().join("test.txt"))
            .unwrap();

        // Create existing file
        fs::write(&test_file, b"existing content").await.unwrap();

        let handler = FullModeHandler;
        let action = handler
            .handle_existing_file(&test_file, b"new content")
            .await
            .unwrap();

        match action {
            RestoreAction::Overwrite { backed_up_to } => {
                assert!(backed_up_to.is_none());
            }
            _ => panic!("Expected Overwrite action"),
        }
    }

    #[test]
    fn test_mode_parsing() {
        assert_eq!(RestoreMode::from_str("safe").unwrap(), RestoreMode::Safe);
        assert_eq!(RestoreMode::from_str("merge").unwrap(), RestoreMode::Merge);
        assert_eq!(RestoreMode::from_str("full").unwrap(), RestoreMode::Full);
        assert_eq!(RestoreMode::from_str("SAFE").unwrap(), RestoreMode::Safe);

        assert!(RestoreMode::from_str("invalid").is_err());
    }

    #[test]
    fn test_mode_as_str() {
        assert_eq!(RestoreMode::Safe.as_str(), "safe");
        assert_eq!(RestoreMode::Merge.as_str(), "merge");
        assert_eq!(RestoreMode::Full.as_str(), "full");
    }
}
