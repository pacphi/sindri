//! Version information for the sindri CLI

use serde::{Deserialize, Serialize};

/// Version information
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

impl std::fmt::Display for VersionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display())
    }
}
