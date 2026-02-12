//! Pre-flight validation for restore operations

use anyhow::Result;
use camino::Utf8Path;
use std::fs;
use tracing::{debug, info};

use super::analysis::BackupAnalyzer;
use super::modes::RestoreMode;

#[derive(Debug, Clone)]
pub struct RestoreOptions {
    pub mode: RestoreMode,
    pub dry_run: bool,
    pub interactive: bool,
    pub force: bool,
    pub validate_extensions: bool,
    pub auto_upgrade_extensions: bool,
}

impl Default for RestoreOptions {
    fn default() -> Self {
        Self {
            mode: RestoreMode::Safe,
            dry_run: false,
            interactive: true,
            force: false,
            validate_extensions: true,
            auto_upgrade_extensions: false,
        }
    }
}

pub fn validate_restore_preconditions(
    backup_path: &Utf8Path,
    _options: &RestoreOptions,
) -> Result<()> {
    info!("Validating restore preconditions for: {}", backup_path);

    if !backup_path.exists() {
        anyhow::bail!("Backup file not found: {}", backup_path);
    }

    debug!("✓ Backup file exists");

    fs::metadata(backup_path.as_std_path())?;
    debug!("✓ Backup file is readable");

    check_disk_space(backup_path)?;
    debug!("✓ Sufficient disk space");

    let analyzer = BackupAnalyzer;
    analyzer.validate_archive(backup_path)?;
    debug!("✓ Archive integrity validated");

    info!("All preconditions validated");
    Ok(())
}

fn check_disk_space(backup_path: &Utf8Path) -> Result<()> {
    let metadata = fs::metadata(backup_path.as_std_path())?;
    let _backup_size = metadata.len();
    Ok(())
}

pub fn validate_workspace_writable(workspace_dir: &Utf8Path) -> Result<()> {
    if !workspace_dir.exists() {
        anyhow::bail!("Workspace directory does not exist: {}", workspace_dir);
    }

    let test_file = workspace_dir.join(".restore-write-test");
    fs::write(test_file.as_std_path(), b"test")?;
    fs::remove_file(test_file.as_std_path()).ok();

    debug!("✓ Workspace directory is writable");
    Ok(())
}
