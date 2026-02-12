//! System marker protection
//!
//! This module protects critical system markers from being restored, which would break
//! the initialization flow and cause duplicate setup.

use camino::{Utf8Path, Utf8PathBuf};
use tracing::debug;

/// System markers that should NEVER be restored
pub const NEVER_RESTORE: &[&str] = &[
    ".initialized",
    ".welcome_shown",
    "workspace/.system/bootstrap.yaml",
    "workspace/.system/installed",
    "workspace/.system/install-status",
];

/// Check if a path is a system marker
pub fn is_system_marker(path: &Utf8Path) -> bool {
    for marker in NEVER_RESTORE {
        let marker_path = Utf8Path::new(marker);
        if path.ends_with(marker_path) || path == marker_path || path.starts_with(marker_path) {
            debug!("Path {} is a system marker", path);
            return true;
        }
    }
    false
}

/// Filter system markers from a list of paths
pub fn filter_system_markers(paths: Vec<Utf8PathBuf>) -> Vec<Utf8PathBuf> {
    paths
        .into_iter()
        .filter(|path| !is_system_marker(path))
        .collect()
}
