//! Version compatibility checks

use anyhow::{Context, Result};
use semver::Version;

#[derive(Debug, Clone)]
pub struct VersionCompatibility {
    pub backup_version: Version,
    pub current_version: Version,
    pub compatible: bool,
    pub issues: Vec<CompatibilityIssue>,
}

#[derive(Debug, Clone)]
pub enum CompatibilityIssue {
    MajorVersionMismatch {
        backup: u64,
        current: u64,
    },
    ExtensionFormatChanged {
        old_format: String,
        new_format: String,
    },
    MissingExtension {
        name: String,
        required_version: String,
    },
    IncompatibleProvider {
        backup_provider: String,
    },
}

impl VersionCompatibility {
    pub fn check(backup_version_str: &str) -> Result<Self> {
        let backup_version = Version::parse(backup_version_str)?;
        let current_version = Self::get_current_version()?;

        let mut issues = Vec::new();
        let mut compatible = true;

        if backup_version.major != current_version.major {
            issues.push(CompatibilityIssue::MajorVersionMismatch {
                backup: backup_version.major,
                current: current_version.major,
            });
            compatible = false;
        }

        Ok(Self {
            backup_version,
            current_version,
            compatible,
            issues,
        })
    }

    pub fn can_auto_upgrade(&self) -> bool {
        self.backup_version.major == self.current_version.major
    }

    fn get_current_version() -> Result<Version> {
        const VERSION: &str = env!("CARGO_PKG_VERSION");
        Version::parse(VERSION).context("Failed to parse current version")
    }

    pub fn message(&self) -> String {
        if self.compatible {
            format!(
                "Backup version {} is compatible with current version {}",
                self.backup_version, self.current_version
            )
        } else {
            format!(
                "Backup version {} is NOT compatible with current version {}",
                self.backup_version, self.current_version
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_invalid_semver() {
        let result = VersionCompatibility::check("not-a-version");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("unexpected character"),
            "Expected semver parse error, got: {}",
            err
        );
    }

    #[test]
    fn test_check_partial_semver_missing_patch() {
        let result = VersionCompatibility::check("3.0");
        assert!(result.is_err(), "Semver requires major.minor.patch");
    }

    #[test]
    fn test_check_compatible_version() {
        // Current version is 3.x.x from CARGO_PKG_VERSION
        let result = VersionCompatibility::check("3.0.0").unwrap();
        assert!(result.compatible, "Same major version should be compatible");
        assert!(result.issues.is_empty());
        assert!(result.can_auto_upgrade());
        assert!(result.message().contains("compatible"));
    }

    #[test]
    fn test_check_incompatible_major_version() {
        // Major version 99 should never match the current 3.x
        let result = VersionCompatibility::check("99.0.0").unwrap();
        assert!(
            !result.compatible,
            "Different major version should be incompatible"
        );
        assert_eq!(result.issues.len(), 1);
        assert!(matches!(
            &result.issues[0],
            CompatibilityIssue::MajorVersionMismatch { backup: 99, .. }
        ));
        assert!(!result.can_auto_upgrade());
        assert!(result.message().contains("NOT compatible"));
    }

    #[test]
    fn test_check_empty_string() {
        let result = VersionCompatibility::check("");
        assert!(result.is_err());
    }
}
