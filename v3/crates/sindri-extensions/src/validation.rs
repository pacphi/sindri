//! Validation configuration and PATH management
//!
//! This module provides configurable PATH setup for extension validation,
//! ensuring commands installed via various methods (mise, npm, go, cargo, etc.)
//! are discoverable during validation.
//!
//! # Environment Variables
//!
//! - `SINDRI_VALIDATION_EXTRA_PATHS`: Colon-separated list of additional paths
//!   to include during validation (relative to home directory).
//!   Example: `SINDRI_VALIDATION_EXTRA_PATHS=.custom/bin:.other/tools`
//!
//! # Default Paths
//!
//! The following paths are included by default (relative to $HOME):
//! - `.local/share/mise/shims` - mise-managed tools
//! - `.local/bin` - user-installed binaries (uv, goose, etc.)
//! - `workspace/bin` - workspace scripts
//! - `go/bin` - Go-installed tools
//! - `.cargo/bin` - Rust/Cargo tools
//! - `.fly/bin` - Fly.io CLI
//! - `.npm-global/bin` - npm global packages (via NPM_CONFIG_PREFIX)

use std::path::{Path, PathBuf};
use tracing::debug;

/// Default validation paths relative to home directory
///
/// These paths cover tools installed via:
/// - mise (shims for node, python, etc.)
/// - npm global packages (when using NPM_CONFIG_PREFIX)
/// - Go binaries
/// - Cargo/Rust binaries
/// - Fly.io CLI
/// - User-local binaries
pub const DEFAULT_VALIDATION_PATHS: &[&str] = &[
    ".local/share/mise/shims", // mise-managed tools (node, npm, python, etc.)
    ".local/bin",              // User binaries (uv, goose, claude-monitor, etc.)
    "workspace/bin",           // Workspace scripts
    "go/bin",                  // Go-installed tools
    ".cargo/bin",              // Rust/Cargo tools
    ".fly/bin",                // Fly.io CLI
    ".npm-global/bin",         // npm global packages (NPM_CONFIG_PREFIX)
];

/// Environment variable for additional validation paths
pub const VALIDATION_EXTRA_PATHS_ENV: &str = "SINDRI_VALIDATION_EXTRA_PATHS";

/// Configuration for validation PATH setup
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Home directory
    home_dir: PathBuf,

    /// Workspace directory
    workspace_dir: PathBuf,

    /// Additional paths from configuration or environment
    extra_paths: Vec<String>,
}

impl ValidationConfig {
    /// Create a new validation configuration
    pub fn new(home_dir: impl Into<PathBuf>, workspace_dir: impl Into<PathBuf>) -> Self {
        let extra_paths = std::env::var(VALIDATION_EXTRA_PATHS_ENV)
            .map(|v| v.split(':').map(String::from).collect())
            .unwrap_or_default();

        Self {
            home_dir: home_dir.into(),
            workspace_dir: workspace_dir.into(),
            extra_paths,
        }
    }

    /// Add extra paths programmatically
    pub fn with_extra_paths(mut self, paths: Vec<String>) -> Self {
        self.extra_paths.extend(paths);
        self
    }

    /// Build the complete PATH for validation
    ///
    /// Returns a colon-separated PATH string that includes:
    /// 1. Default validation paths (resolved to absolute)
    /// 2. Extra paths from environment/configuration
    /// 3. Current PATH
    pub fn build_validation_path(&self) -> String {
        let current_path = std::env::var("PATH").unwrap_or_default();
        let mut paths: Vec<String> = Vec::new();

        // Add default paths
        for path in DEFAULT_VALIDATION_PATHS {
            let resolved = self.resolve_path(path);
            if resolved.exists() && !self.path_in_list(&resolved, &current_path) {
                debug!("Adding validation path: {}", resolved.display());
                paths.push(resolved.to_string_lossy().to_string());
            }
        }

        // Add extra paths from environment/config
        for path in &self.extra_paths {
            let resolved = self.resolve_path(path);
            if resolved.exists() && !self.path_in_list(&resolved, &current_path) {
                debug!("Adding extra validation path: {}", resolved.display());
                paths.push(resolved.to_string_lossy().to_string());
            }
        }

        // Combine with current PATH
        if paths.is_empty() {
            current_path
        } else {
            format!("{}:{}", paths.join(":"), current_path)
        }
    }

    /// Resolve a path pattern to an absolute path
    ///
    /// Handles:
    /// - `~` or paths starting with `.` -> relative to home_dir
    /// - `workspace/` -> relative to workspace_dir
    /// - Absolute paths -> as-is
    fn resolve_path(&self, path: &str) -> PathBuf {
        if path.starts_with("workspace/") || path.starts_with("workspace\\") {
            self.workspace_dir
                .join(path.strip_prefix("workspace/").unwrap_or(path))
        } else if path.starts_with('/') {
            PathBuf::from(path)
        } else {
            // Relative to home directory
            self.home_dir.join(path)
        }
    }

    /// Check if a path is already in the PATH string
    fn path_in_list(&self, path: &Path, path_var: &str) -> bool {
        let path_str = path.to_string_lossy();
        path_var.split(':').any(|p| p == path_str)
    }

    /// Get the list of all validation paths (for debugging/logging)
    pub fn get_all_paths(&self) -> Vec<PathBuf> {
        let mut all_paths: Vec<PathBuf> = DEFAULT_VALIDATION_PATHS
            .iter()
            .map(|p| self.resolve_path(p))
            .collect();

        all_paths.extend(self.extra_paths.iter().map(|p| self.resolve_path(p)));

        all_paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_paths() {
        assert!(!DEFAULT_VALIDATION_PATHS.is_empty());
        assert!(DEFAULT_VALIDATION_PATHS.contains(&".local/share/mise/shims"));
        assert!(DEFAULT_VALIDATION_PATHS.contains(&".cargo/bin"));
    }

    #[test]
    fn test_resolve_path_home_relative() {
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        let workspace = temp.path().join("workspace");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&workspace).unwrap();

        let config = ValidationConfig::new(&home, &workspace);

        let resolved = config.resolve_path(".local/bin");
        assert_eq!(resolved, home.join(".local/bin"));
    }

    #[test]
    fn test_resolve_path_workspace() {
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        let workspace = temp.path().join("workspace");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&workspace).unwrap();

        let config = ValidationConfig::new(&home, &workspace);

        let resolved = config.resolve_path("workspace/bin");
        assert_eq!(resolved, workspace.join("bin"));
    }

    #[test]
    fn test_extra_paths() {
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        let workspace = temp.path().join("workspace");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&workspace).unwrap();

        let config = ValidationConfig::new(&home, &workspace)
            .with_extra_paths(vec![".custom/bin".to_string()]);

        let all_paths = config.get_all_paths();
        assert!(all_paths.iter().any(|p| p.ends_with(".custom/bin")));
    }

    #[test]
    fn test_build_validation_path_with_existing_dirs() {
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        let workspace = temp.path().join("workspace");

        // Create some directories that should be included
        let local_bin = home.join(".local/bin");
        std::fs::create_dir_all(&local_bin).unwrap();

        let config = ValidationConfig::new(&home, &workspace);
        let path = config.build_validation_path();

        assert!(path.contains(&local_bin.to_string_lossy().to_string()));
    }
}
