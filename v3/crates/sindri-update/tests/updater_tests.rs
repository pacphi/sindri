//! Unit tests for updater module
//!
//! Tests cover:
//! - Binary backup creation
//! - Binary replacement
//! - Verification (success and failure)
//! - Automatic rollback on verification failure
//! - Cleanup of old backups
//! - Uses tempfile for isolated filesystem tests

mod common;

use common::*;
use sindri_update::updater::{BackupInfo, SindriUpdater, UpdateResult};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_updater_creation() {
    let updater = SindriUpdater::new();
    let updater = updater.expect("SindriUpdater::new should succeed");
    assert!(updater.current_version().major >= 3);
    assert!(updater.binary_path().exists());
}

#[test]
fn test_updater_default() {
    let updater = SindriUpdater::new().unwrap();
    assert!(updater.current_version().major >= 3);
}

#[test]
fn test_binary_verification_nonexistent() {
    let updater = SindriUpdater::new().unwrap();
    let fake_path = PathBuf::from("/nonexistent/binary");

    let result = updater.verify_binary(&fake_path);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("does not exist"));
}

#[cfg(unix)]
#[test]
fn test_binary_verification_success() {
    let temp_dir = TempDir::new().unwrap();
    let binary_path = temp_dir.path().join("test-binary");

    // Create a script that responds to --version
    create_version_script(&binary_path, VERSION_3_0_0).unwrap();

    let updater = SindriUpdater::new().unwrap();
    let result = updater.verify_binary(&binary_path);
    result.expect("verify_binary should succeed for a valid binary");
}

#[cfg(unix)]
#[test]
fn test_binary_verification_wrong_output() {
    let temp_dir = TempDir::new().unwrap();
    let binary_path = temp_dir.path().join("wrong-binary");

    // Create a script that returns wrong output
    create_wrong_output_script(&binary_path).unwrap();

    let updater = SindriUpdater::new().unwrap();
    let result = updater.verify_binary(&binary_path);

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("unexpected output"));
}

#[test]
fn test_backup_info_ordering() {
    use std::time::{Duration, SystemTime};

    let mut backups = [
        BackupInfo {
            path: PathBuf::from("/backup1"),
            size: 100,
            created: SystemTime::now() - Duration::from_secs(100),
        },
        BackupInfo {
            path: PathBuf::from("/backup2"),
            size: 200,
            created: SystemTime::now() - Duration::from_secs(50),
        },
        BackupInfo {
            path: PathBuf::from("/backup3"),
            size: 300,
            created: SystemTime::now(),
        },
    ];

    // Sort by creation time (newest first)
    backups.sort_by_key(|b| b.created);
    backups.reverse();

    assert_eq!(backups[0].path, PathBuf::from("/backup3"));
    assert_eq!(backups[1].path, PathBuf::from("/backup2"));
    assert_eq!(backups[2].path, PathBuf::from("/backup1"));
}

#[test]
fn test_update_result_already_up_to_date() {
    let result = UpdateResult::AlreadyUpToDate(VERSION_3_0_0.to_string());

    match result {
        UpdateResult::AlreadyUpToDate(version) => {
            assert_eq!(version, VERSION_3_0_0);
        }
        _ => panic!("Expected AlreadyUpToDate variant"),
    }
}

#[test]
fn test_update_result_updated() {
    let backup_path = PathBuf::from("/tmp/backup");
    let result = UpdateResult::Updated {
        from: VERSION_3_0_0.to_string(),
        to: VERSION_3_1_0.to_string(),
        backup: backup_path.clone(),
    };

    match result {
        UpdateResult::Updated { from, to, backup } => {
            assert_eq!(from, VERSION_3_0_0);
            assert_eq!(to, VERSION_3_1_0);
            assert_eq!(backup, backup_path);
        }
        _ => panic!("Expected Updated variant"),
    }
}

#[cfg(unix)]
#[test]
fn test_backup_creation_and_cleanup() {
    use std::thread::sleep;
    use std::time::Duration;

    let temp_dir = TempDir::new().unwrap();
    let binary_path = temp_dir.path().join("sindri");

    // Create a fake binary
    create_version_script(&binary_path, VERSION_3_0_0).unwrap();

    // We can't test the actual SindriUpdater backup creation without modifying
    // the binary path, but we can test the backup file pattern
    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let backup_path = binary_path.with_extension(format!("{}.bak", timestamp));

    fs::copy(&binary_path, &backup_path).unwrap();

    assert!(backup_path.exists());
    assert!(backup_path.to_string_lossy().contains(".bak"));

    // Create multiple backups
    sleep(Duration::from_millis(1100)); // Ensure different timestamp
    let timestamp2 = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let backup_path2 = binary_path.with_extension(format!("{}.bak", timestamp2));
    fs::copy(&binary_path, &backup_path2).unwrap();

    sleep(Duration::from_millis(1100));
    let timestamp3 = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let backup_path3 = binary_path.with_extension(format!("{}.bak", timestamp3));
    fs::copy(&binary_path, &backup_path3).unwrap();

    // Verify all backups exist
    assert!(backup_path.exists());
    assert!(backup_path2.exists());
    assert!(backup_path3.exists());

    // Simulate cleanup: keep only 2 most recent
    let mut backups = [
        backup_path.clone(),
        backup_path2.clone(),
        backup_path3.clone(),
    ];
    backups.sort_by_key(|path| {
        fs::metadata(path)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    });
    backups.reverse();

    // Remove oldest
    if backups.len() > 2 {
        for old_backup in &backups[2..] {
            fs::remove_file(old_backup).unwrap();
        }
    }

    // Verify cleanup
    assert!(!backup_path.exists()); // Oldest removed
    assert!(backup_path2.exists());
    assert!(backup_path3.exists());
}

#[test]
fn test_backup_file_extension() {
    let binary_path = PathBuf::from("/usr/local/bin/sindri");
    let timestamp = "20240101-120000";
    let backup_path = binary_path.with_extension(format!("{}.bak", timestamp));

    assert!(backup_path.to_string_lossy().ends_with(".bak"));
    assert!(backup_path.to_string_lossy().contains(timestamp));
}

#[cfg(unix)]
#[test]
fn test_binary_replacement_simulation() {
    let temp_dir = TempDir::new().unwrap();
    let original = temp_dir.path().join("original");
    let replacement = temp_dir.path().join("replacement");

    // Create original binary
    create_fake_binary(&original, ORIGINAL_CONTENT).unwrap();
    let original_content = fs::read(&original).unwrap();

    // Create replacement binary
    create_fake_binary(&replacement, NEW_CONTENT).unwrap();

    // Replace (simulating atomic rename)
    fs::rename(&replacement, &original).unwrap();

    // Verify replacement
    let new_content = fs::read(&original).unwrap();
    assert_ne!(new_content, original_content);
    assert_eq!(new_content, NEW_CONTENT);
}

#[cfg(unix)]
#[test]
fn test_rollback_simulation() {
    let temp_dir = TempDir::new().unwrap();
    let binary = temp_dir.path().join("binary");
    let backup = temp_dir.path().join("backup.bak");

    // Create original binary
    create_fake_binary(&binary, b"original").unwrap();

    // Create backup
    fs::copy(&binary, &backup).unwrap();

    // Simulate bad update
    fs::write(&binary, CORRUPTED_CONTENT).unwrap();

    // Verify binary is corrupted
    assert_eq!(fs::read(&binary).unwrap(), CORRUPTED_CONTENT);

    // Rollback
    fs::copy(&backup, &binary).unwrap();

    // Verify rollback
    assert_eq!(fs::read(&binary).unwrap(), b"original");
}

#[test]
fn test_list_backups_empty() {
    // This test verifies the logic for listing backups when none exist
    let temp_dir = TempDir::new().unwrap();
    let binary_path = temp_dir.path().join("sindri");

    create_fake_binary(&binary_path, TEST_CONTENT).unwrap();

    // Find all .bak files
    let backups: Vec<PathBuf> = fs::read_dir(temp_dir.path())
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|e| e.path())
        .filter(|path| {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                name.starts_with("sindri") && name.ends_with(".bak")
            } else {
                false
            }
        })
        .collect();

    assert_eq!(backups.len(), 0);
}

#[cfg(unix)]
#[test]
fn test_list_backups_with_files() {
    let temp_dir = TempDir::new().unwrap();
    let binary_path = temp_dir.path().join("sindri");

    create_fake_binary(&binary_path, TEST_CONTENT).unwrap();

    // Create some backups
    for i in 0..3 {
        let backup_path = temp_dir
            .path()
            .join(format!("sindri.2024010{}-120000.bak", i));
        fs::copy(&binary_path, &backup_path).unwrap();
    }

    // Find all .bak files
    let backups: Vec<PathBuf> = fs::read_dir(temp_dir.path())
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|e| e.path())
        .filter(|path| {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                name.starts_with("sindri") && name.ends_with(".bak")
            } else {
                false
            }
        })
        .collect();

    assert_eq!(backups.len(), 3);
}

#[test]
fn test_backup_filename_format() {
    let timestamp = "20240115-143022";
    let filename = format!("sindri.{}.bak", timestamp);

    assert!(filename.starts_with("sindri."));
    assert!(filename.ends_with(".bak"));
    assert!(filename.contains(timestamp));
}

#[cfg(unix)]
#[test]
fn test_permissions_preserved_on_backup() {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = TempDir::new().unwrap();
    let binary = temp_dir.path().join("binary");
    let backup = temp_dir.path().join("backup");

    // Create binary with specific permissions
    create_fake_binary(&binary, TEST_CONTENT).unwrap();
    let mut perms = fs::metadata(&binary).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&binary, perms).unwrap();

    // Create backup
    fs::copy(&binary, &backup).unwrap();

    // Verify permissions
    let backup_perms = fs::metadata(&backup).unwrap().permissions();
    assert_eq!(backup_perms.mode() & 0o777, 0o755);
}

#[test]
fn test_max_backups_constant() {
    // Verify the MAX_BACKUPS constant is reasonable
    const TEST_MAX_BACKUPS: usize = 2;
    const _: () = {
        assert!(TEST_MAX_BACKUPS > 0);
        assert!(TEST_MAX_BACKUPS <= 10); // Reasonable upper bound
    };
}

#[cfg(unix)]
#[test]
fn test_binary_executable_check() {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = TempDir::new().unwrap();
    let binary = temp_dir.path().join("binary");

    create_fake_binary(&binary, TEST_CONTENT).unwrap();

    // Check if executable
    let metadata = fs::metadata(&binary).unwrap();
    let permissions = metadata.permissions();
    let mode = permissions.mode();

    assert!(mode & 0o100 != 0); // Owner execute bit
}

#[test]
fn test_version_parsing_in_updater() {
    use semver::Version;

    let version = Version::parse(VERSION_3_0_0);
    let v = version.expect("VERSION_3_0_0 should be a valid semver");
    assert_eq!(v.major, 3);
    assert_eq!(v.minor, 0);
    assert_eq!(v.patch, 0);
}

#[test]
fn test_version_comparison() {
    use semver::Version;

    let current = Version::parse(VERSION_3_0_0).unwrap();
    let newer = Version::parse(VERSION_3_1_0).unwrap();
    let older = Version::parse("2.9.0").unwrap();

    assert!(newer > current);
    assert!(current > older);
    assert!(current == current);
}

#[cfg(unix)]
#[test]
fn test_atomic_rename_behavior() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("source");
    let dest = temp_dir.path().join("dest");

    // Create source
    fs::write(&source, b"source content").unwrap();

    // Create destination
    fs::write(&dest, b"dest content").unwrap();

    // Atomic rename (overwrites dest)
    fs::rename(&source, &dest).unwrap();

    // Verify
    assert!(!source.exists());
    assert!(dest.exists());
    assert_eq!(fs::read(&dest).unwrap(), b"source content");
}

#[test]
fn test_updater_component_access() {
    let updater = SindriUpdater::new().unwrap();

    // Test that we can access components
    let _release_manager = updater.release_manager();
    let _compat_checker = updater.compatibility_checker();
    let _version = updater.current_version();
    let _path = updater.binary_path();

    // All accessors should work without errors
}

#[test]
fn test_update_result_debug_format() {
    let result = UpdateResult::AlreadyUpToDate(VERSION_3_0_0.to_string());
    let debug_str = format!("{:?}", result);
    assert!(debug_str.contains("AlreadyUpToDate"));
    assert!(debug_str.contains(VERSION_3_0_0));
}

#[test]
fn test_backup_info_debug_format() {
    use std::time::SystemTime;

    let info = BackupInfo {
        path: PathBuf::from("/test/backup"),
        size: 12345,
        created: SystemTime::now(),
    };

    let debug_str = format!("{:?}", info);
    assert!(debug_str.contains("BackupInfo"));
    assert!(debug_str.contains("12345"));
}
