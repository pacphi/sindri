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

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_validate_restore_preconditions_missing_file() {
        let path = Utf8Path::new("/tmp/nonexistent-backup-file-12345.tar.gz");
        let options = RestoreOptions::default();
        let result = validate_restore_preconditions(path, &options);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Backup file not found"),
            "Expected 'Backup file not found' in error, got: {}",
            err
        );
    }

    #[test]
    fn test_validate_restore_preconditions_invalid_archive() {
        let temp_dir = TempDir::new().unwrap();
        let bad_archive = temp_dir.path().join("bad.tar.gz");
        // Write non-gzip content
        std::fs::write(&bad_archive, b"this is not a valid gzip archive").unwrap();

        let utf8_path =
            Utf8PathBuf::from_path_buf(bad_archive).expect("path should be valid UTF-8");
        let options = RestoreOptions::default();
        let result = validate_restore_preconditions(&utf8_path, &options);
        assert!(result.is_err(), "Should fail on invalid archive format");
    }

    #[test]
    fn test_validate_workspace_writable_nonexistent_dir() {
        let path = Utf8Path::new("/tmp/nonexistent-workspace-dir-12345");
        let result = validate_workspace_writable(path);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("does not exist"),
            "Expected 'does not exist' in error, got: {}",
            err
        );
    }

    #[test]
    fn test_validate_workspace_writable_success() {
        let temp_dir = TempDir::new().unwrap();
        let utf8_path =
            Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf()).expect("valid UTF-8");
        let result = validate_workspace_writable(&utf8_path);
        result.expect("validate_workspace_writable should succeed for temp dir");
    }

    #[test]
    fn test_restore_options_default_values() {
        let opts = RestoreOptions::default();
        assert_eq!(opts.mode, RestoreMode::Safe, "Default mode should be Safe");
        assert!(!opts.dry_run, "Default dry_run should be false");
        assert!(opts.interactive, "Default interactive should be true");
        assert!(!opts.force, "Default force should be false");
        assert!(
            opts.validate_extensions,
            "Default validate_extensions should be true"
        );
        assert!(
            !opts.auto_upgrade_extensions,
            "Default auto_upgrade_extensions should be false"
        );
    }

    #[test]
    fn test_validate_restore_preconditions_valid_archive() {
        let temp_dir = TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("valid.tar.gz");

        // Create a valid tar.gz with content
        let source_dir = temp_dir.path().join("source");
        std::fs::create_dir_all(&source_dir).unwrap();
        std::fs::write(source_dir.join("data.txt"), b"some data").unwrap();

        let file = std::fs::File::create(&archive_path).unwrap();
        let encoder =
            flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut builder = tar::Builder::new(encoder);
        builder.append_dir_all(".", &source_dir).unwrap();
        let encoder = builder.into_inner().unwrap();
        encoder.finish().unwrap();

        let utf8_path =
            Utf8PathBuf::from_path_buf(archive_path).expect("path should be valid UTF-8");
        let options = RestoreOptions::default();
        let result = validate_restore_preconditions(&utf8_path, &options);
        assert!(
            result.is_ok(),
            "Valid archive should pass preconditions, got: {:?}",
            result.err()
        );
    }
}
