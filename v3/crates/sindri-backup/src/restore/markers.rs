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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_system_markers_detected() {
        assert!(is_system_marker(Utf8Path::new(".initialized")));
        assert!(is_system_marker(Utf8Path::new(".welcome_shown")));
        assert!(is_system_marker(Utf8Path::new(
            "workspace/.system/bootstrap.yaml"
        )));
        assert!(is_system_marker(Utf8Path::new(
            "workspace/.system/installed"
        )));
        assert!(is_system_marker(Utf8Path::new(
            "workspace/.system/install-status"
        )));
    }

    #[test]
    fn test_non_marker_paths_not_detected() {
        assert!(!is_system_marker(Utf8Path::new("config.yaml")));
        assert!(!is_system_marker(Utf8Path::new("src/main.rs")));
        assert!(!is_system_marker(Utf8Path::new(
            "workspace/project/data.json"
        )));
        assert!(!is_system_marker(Utf8Path::new(".gitignore")));
        assert!(!is_system_marker(Utf8Path::new("README.md")));
    }

    #[test]
    fn test_paths_ending_with_marker_names_detected() {
        assert!(is_system_marker(Utf8Path::new("some/prefix/.initialized")));
        assert!(is_system_marker(Utf8Path::new(
            "deep/nested/path/.welcome_shown"
        )));
        assert!(is_system_marker(Utf8Path::new(
            "other/workspace/.system/bootstrap.yaml"
        )));
    }

    #[test]
    fn test_filter_system_markers_removes_markers() {
        let paths = vec![
            Utf8PathBuf::from("config.yaml"),
            Utf8PathBuf::from(".initialized"),
            Utf8PathBuf::from("src/main.rs"),
            Utf8PathBuf::from(".welcome_shown"),
            Utf8PathBuf::from("workspace/.system/bootstrap.yaml"),
        ];

        let filtered = filter_system_markers(paths);
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0], Utf8PathBuf::from("config.yaml"));
        assert_eq!(filtered[1], Utf8PathBuf::from("src/main.rs"));
    }

    #[test]
    fn test_filter_system_markers_keeps_all_non_markers() {
        let paths = vec![
            Utf8PathBuf::from("a.txt"),
            Utf8PathBuf::from("b/c.rs"),
            Utf8PathBuf::from("d/e/f.yaml"),
        ];

        let filtered = filter_system_markers(paths.clone());
        assert_eq!(filtered, paths);
    }

    #[test]
    fn test_filter_system_markers_empty_input() {
        let paths: Vec<Utf8PathBuf> = vec![];
        let filtered = filter_system_markers(paths);
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_filter_system_markers_all_markers() {
        let paths = vec![
            Utf8PathBuf::from(".initialized"),
            Utf8PathBuf::from(".welcome_shown"),
            Utf8PathBuf::from("workspace/.system/installed"),
        ];

        let filtered = filter_system_markers(paths);
        assert!(
            filtered.is_empty(),
            "All system markers should be filtered out"
        );
    }
}
