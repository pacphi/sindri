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
    MajorVersionMismatch { backup: u64, current: u64 },
    ExtensionFormatChanged { old_format: String, new_format: String },
    MissingExtension { name: String, required_version: String },
    IncompatibleProvider { backup_provider: String },
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
