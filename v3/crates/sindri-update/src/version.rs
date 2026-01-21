//! Version information and comparison

use semver::Version;
use serde::{Deserialize, Serialize};

/// Version information for the CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    /// Semantic version
    pub version: String,

    /// Git commit SHA (short)
    pub commit: Option<String>,

    /// Build date
    pub build_date: Option<String>,

    /// Target triple
    pub target: Option<String>,
}

impl VersionInfo {
    /// Create version info for current build
    pub fn current() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            commit: option_env!("GIT_SHA").map(String::from),
            build_date: option_env!("BUILD_DATE").map(String::from),
            target: option_env!("TARGET").map(String::from),
        }
    }

    /// Parse semantic version
    pub fn semver(&self) -> Option<Version> {
        Version::parse(&self.version).ok()
    }

    /// Check if this version is newer than another
    pub fn is_newer_than(&self, other: &VersionInfo) -> bool {
        match (self.semver(), other.semver()) {
            (Some(a), Some(b)) => a > b,
            _ => false,
        }
    }

    /// Format as display string
    pub fn display(&self) -> String {
        let mut parts = vec![format!("sindri {}", self.version)];

        if let Some(commit) = &self.commit {
            parts.push(format!("({})", commit));
        }

        if let Some(target) = &self.target {
            parts.push(target.clone());
        }

        parts.join(" ")
    }
}

impl Default for VersionInfo {
    fn default() -> Self {
        Self::current()
    }
}

impl std::fmt::Display for VersionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        let v1 = VersionInfo {
            version: "3.0.0".to_string(),
            commit: None,
            build_date: None,
            target: None,
        };

        let v2 = VersionInfo {
            version: "3.1.0".to_string(),
            commit: None,
            build_date: None,
            target: None,
        };

        assert!(v2.is_newer_than(&v1));
        assert!(!v1.is_newer_than(&v2));
    }
}
