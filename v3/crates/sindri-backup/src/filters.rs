//! File filtering for backup operations.
//!
//! This module provides filtering logic for backup operations:
//! - Always excluded patterns (caches, node_modules, etc.)
//! - Never restore patterns (system markers)
//! - Custom exclusion patterns

use globset::{Glob, GlobSet, GlobSetBuilder};
use std::path::Path;

/// Patterns that are ALWAYS excluded from backups, regardless of profile.
/// These are regenerable files that bloat backup size.
pub const ALWAYS_EXCLUDE: &[&str] = &[
    ".cache",
    ".cache/**",
    ".local/share/mise/installs",
    ".local/share/mise/installs/**",
    ".local/state/mise",
    ".local/state/mise/**",
    ".local/state",
    ".local/state/**",
    "workspace/.system/logs",
    "workspace/.system/logs/**",
    "**/node_modules",
    "**/node_modules/**",
    "**/.venv",
    "**/.venv/**",
    "**/__pycache__",
    "**/__pycache__/**",
    "**/target/debug",
    "**/target/debug/**",
    "**/target/release",
    "**/target/release/**",
    "**/.next",
    "**/.next/**",
    "**/.gradle",
    "**/.gradle/**",
    "**/.turbo",
    "**/.turbo/**",
    "**/dist",
    "**/build",
];

/// Patterns that should NEVER be restored from backups.
/// These are system markers that control initialization flow.
/// Restoring these breaks the entrypoint initialization.
pub const NEVER_RESTORE: &[&str] = &[
    ".initialized",
    ".welcome_shown",
    "workspace/.system/bootstrap.yaml",
    "workspace/.system/installed",
    "workspace/.system/install-status",
];

/// Exclusion configuration for backup operations.
#[derive(Debug, Clone)]
pub struct ExclusionConfig {
    /// The compiled globset for matching excluded files
    globset: GlobSet,
}

impl ExclusionConfig {
    /// Creates a new exclusion configuration with always-excluded patterns
    /// and optional additional patterns.
    pub fn new(additional_patterns: Vec<String>) -> anyhow::Result<Self> {
        let mut builder = GlobSetBuilder::new();

        // Add always-excluded patterns
        for pattern in ALWAYS_EXCLUDE {
            let glob = Glob::new(pattern)
                .map_err(|e| anyhow::anyhow!("Invalid exclusion pattern '{}': {}", pattern, e))?;
            builder.add(glob);
        }

        // Add user-provided patterns
        for pattern in additional_patterns {
            let glob = Glob::new(&pattern)
                .map_err(|e| anyhow::anyhow!("Invalid exclusion pattern '{}': {}", pattern, e))?;
            builder.add(glob);
        }

        let globset = builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build exclusion globset: {}", e))?;

        Ok(Self { globset })
    }

    /// Checks if a path should be excluded from backup.
    pub fn should_exclude(&self, path: &Path) -> bool {
        self.globset.is_match(path)
    }

    /// Returns true if the path is a system marker that should never be restored.
    pub fn is_system_marker(path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        NEVER_RESTORE
            .iter()
            .any(|marker| path_str == *marker || path_str.starts_with(&format!("{}/", marker)))
    }
}

/// Filter for restore operations that protects system markers.
#[derive(Debug, Clone)]
pub struct RestoreFilter {
    /// Patterns to exclude from restore
    globset: GlobSet,
}

impl RestoreFilter {
    /// Creates a new restore filter that always excludes system markers.
    pub fn new(additional_patterns: Vec<String>) -> anyhow::Result<Self> {
        let mut builder = GlobSetBuilder::new();

        // Add system marker patterns
        for pattern in NEVER_RESTORE {
            let glob = Glob::new(pattern).map_err(|e| {
                anyhow::anyhow!("Invalid restore filter pattern '{}': {}", pattern, e)
            })?;
            builder.add(glob);

            // Also match subdirectories
            let subdir_pattern = format!("{}/**", pattern);
            let glob = Glob::new(&subdir_pattern).map_err(|e| {
                anyhow::anyhow!("Invalid restore filter pattern '{}': {}", subdir_pattern, e)
            })?;
            builder.add(glob);
        }

        // Add user-provided patterns
        for pattern in additional_patterns {
            let glob = Glob::new(&pattern).map_err(|e| {
                anyhow::anyhow!("Invalid restore filter pattern '{}': {}", pattern, e)
            })?;
            builder.add(glob);
        }

        let globset = builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build restore filter globset: {}", e))?;

        Ok(Self { globset })
    }

    /// Checks if a path should be excluded from restore.
    pub fn should_exclude(&self, path: &Path) -> bool {
        self.globset.is_match(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_always_exclude_patterns() {
        let config = ExclusionConfig::new(vec![]).unwrap();

        // Should exclude caches
        assert!(config.should_exclude(Path::new(".cache/foo")));
        assert!(config.should_exclude(Path::new(".local/share/mise/installs/node")));

        // Should exclude common build artifacts
        assert!(config.should_exclude(Path::new("project/node_modules/foo")));
        assert!(config.should_exclude(Path::new("project/.venv/lib")));
        assert!(config.should_exclude(Path::new("project/__pycache__/foo.pyc")));
        assert!(config.should_exclude(Path::new("project/target/debug/binary")));

        // Should not exclude user data
        assert!(!config.should_exclude(Path::new("workspace/projects/myproject")));
        assert!(!config.should_exclude(Path::new(".gitconfig")));
    }

    #[test]
    fn test_custom_exclusions() {
        let config =
            ExclusionConfig::new(vec!["*.log".to_string(), "temp/**".to_string()]).unwrap();

        assert!(config.should_exclude(Path::new("app.log")));
        assert!(config.should_exclude(Path::new("temp/data.txt")));
        assert!(!config.should_exclude(Path::new("important.txt")));
    }

    #[test]
    fn test_system_marker_detection() {
        assert!(ExclusionConfig::is_system_marker(Path::new(".initialized")));
        assert!(ExclusionConfig::is_system_marker(Path::new(
            ".welcome_shown"
        )));
        assert!(ExclusionConfig::is_system_marker(Path::new(
            "workspace/.system/bootstrap.yaml"
        )));
        assert!(ExclusionConfig::is_system_marker(Path::new(
            "workspace/.system/installed"
        )));

        assert!(!ExclusionConfig::is_system_marker(Path::new(
            "workspace/projects/myproject"
        )));
        assert!(!ExclusionConfig::is_system_marker(Path::new(".gitconfig")));
    }

    #[test]
    fn test_restore_filter() {
        let filter = RestoreFilter::new(vec![]).unwrap();

        // Should exclude system markers
        assert!(filter.should_exclude(Path::new(".initialized")));
        assert!(filter.should_exclude(Path::new(".welcome_shown")));
        assert!(filter.should_exclude(Path::new("workspace/.system/bootstrap.yaml")));

        // Should not exclude user data
        assert!(!filter.should_exclude(Path::new("workspace/projects/myproject")));
        assert!(!filter.should_exclude(Path::new(".gitconfig")));
    }

    #[test]
    fn test_restore_filter_with_custom_patterns() {
        let filter = RestoreFilter::new(vec!["*.bak".to_string()]).unwrap();

        // Should exclude custom patterns
        assert!(filter.should_exclude(Path::new("file.bak")));

        // Should still exclude system markers
        assert!(filter.should_exclude(Path::new(".initialized")));
    }

    #[test]
    fn test_invalid_pattern() {
        let result = ExclusionConfig::new(vec!["[invalid".to_string()]);
        assert!(result.is_err());
    }
}
