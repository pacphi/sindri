//! Helper functions for updater testing
//!
//! Provides utilities for creating fake binaries and scripts for testing
//! the binary update and verification process.

use std::fs;
use std::io::Write;
use std::path::Path;

/// Create a fake binary file with the given content
pub fn create_fake_binary(path: &Path, content: &[u8]) -> std::io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(content)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms)?;
    }

    Ok(())
}

/// Create a test binary that responds to --version with the given version
#[cfg(unix)]
pub fn create_version_script(path: &Path, version: &str) -> std::io::Result<()> {
    let script = format!(
        r#"#!/bin/bash
if [ "$1" = "--version" ]; then
    echo "sindri {}"
    exit 0
else
    exit 1
fi
"#,
        version
    );

    let mut file = fs::File::create(path)?;
    file.write_all(script.as_bytes())?;

    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms)?;

    Ok(())
}

/// Create a test binary that returns unexpected output (for verification failure tests)
#[cfg(unix)]
pub fn create_wrong_output_script(path: &Path) -> std::io::Result<()> {
    let script = r#"#!/bin/bash
if [ "$1" = "--version" ]; then
    echo "some other tool v1.0.0"
    exit 0
fi
"#;

    let mut file = fs::File::create(path)?;
    file.write_all(script.as_bytes())?;

    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms)?;

    Ok(())
}

/// Test content for fake binaries
pub const ORIGINAL_CONTENT: &[u8] = b"original content";
pub const NEW_CONTENT: &[u8] = b"new content";
pub const CORRUPTED_CONTENT: &[u8] = b"corrupted";
pub const TEST_CONTENT: &[u8] = b"test";
