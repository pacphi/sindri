//! Backup profile definitions and management.
//!
//! This module defines three backup profiles:
//! - UserData: Smallest backup for migration (projects, Claude data, git config)
//! - Standard: Default balanced backup (user-data + configs)
//! - Full: Complete disaster recovery (everything except caches)

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

/// Backup profile determines what data is included in the backup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackupProfile {
    /// User data only: projects, Claude data, git config
    /// Size: 100MB-1GB
    /// Use case: Migration to new provider/version
    UserData,

    /// Standard backup: user-data + shell/app configs
    /// Size: 1-5GB
    /// Use case: Regular backups, disaster recovery
    Standard,

    /// Full backup: everything except caches and mise installs
    /// Size: 5-20GB
    /// Use case: Complete disaster recovery, forensic analysis
    Full,
}

impl BackupProfile {
    /// Returns the list of paths to include for this profile.
    /// Returns None for Full profile (includes everything).
    pub fn includes(&self) -> Option<Vec<PathBuf>> {
        match self {
            BackupProfile::UserData => Some(vec![
                PathBuf::from("workspace/projects"),
                PathBuf::from("workspace/config"),
                PathBuf::from("workspace/scripts"),
                PathBuf::from("workspace/bin"),
                PathBuf::from(".claude"),
                PathBuf::from(".ssh/host_keys"),
                PathBuf::from(".gitconfig"),
            ]),
            BackupProfile::Standard => Some(vec![
                PathBuf::from("workspace/projects"),
                PathBuf::from("workspace/config"),
                PathBuf::from("workspace/scripts"),
                PathBuf::from("workspace/bin"),
                PathBuf::from(".claude"),
                PathBuf::from(".ssh/host_keys"),
                PathBuf::from(".gitconfig"),
                PathBuf::from(".bashrc"),
                PathBuf::from(".profile"),
                PathBuf::from(".config"),
                PathBuf::from(".local/bin"),
            ]),
            BackupProfile::Full => None, // Include everything
        }
    }

    /// Returns profile-specific exclusion patterns.
    pub fn excludes(&self) -> Vec<String> {
        match self {
            BackupProfile::UserData => vec![
                // Exclude configs that aren't essential for migration
                ".config".to_string(),
                ".local".to_string(),
                ".bashrc".to_string(),
                ".profile".to_string(),
            ],
            BackupProfile::Standard => vec![
                // Exclude mise shims and tool installations
                ".config/mise/shims".to_string(),
                ".local/share/mise".to_string(),
                ".local/state".to_string(),
            ],
            BackupProfile::Full => vec![
                // Full mode has minimal exclusions (caches only)
            ],
        }
    }

    /// Returns a human-readable description of this profile.
    pub fn description(&self) -> &str {
        match self {
            BackupProfile::UserData => {
                "Projects, Claude data, git config (smallest, migration-focused)"
            }
            BackupProfile::Standard => "User data + shell/app configs (default, balanced)",
            BackupProfile::Full => "Everything except caches (largest, complete recovery)",
        }
    }

    /// Returns the typical size range for this profile.
    pub fn typical_size(&self) -> &str {
        match self {
            BackupProfile::UserData => "100MB-1GB",
            BackupProfile::Standard => "1-5GB",
            BackupProfile::Full => "5-20GB",
        }
    }

    /// Returns the recommended use case for this profile.
    pub fn use_case(&self) -> &str {
        match self {
            BackupProfile::UserData => "Migration to new provider/version",
            BackupProfile::Standard => "Regular backups, disaster recovery",
            BackupProfile::Full => "Complete disaster recovery, forensic analysis",
        }
    }

    /// Returns all available profiles.
    pub fn all() -> Vec<BackupProfile> {
        vec![
            BackupProfile::UserData,
            BackupProfile::Standard,
            BackupProfile::Full,
        ]
    }

    /// Parses a profile from a string.
    pub fn from_str(s: &str) -> Option<BackupProfile> {
        match s.to_lowercase().as_str() {
            "user-data" | "userdata" => Some(BackupProfile::UserData),
            "standard" => Some(BackupProfile::Standard),
            "full" => Some(BackupProfile::Full),
            _ => None,
        }
    }
}

impl Default for BackupProfile {
    fn default() -> Self {
        BackupProfile::Standard
    }
}

impl fmt::Display for BackupProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackupProfile::UserData => write!(f, "user-data"),
            BackupProfile::Standard => write!(f, "standard"),
            BackupProfile::Full => write!(f, "full"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_includes() {
        let user_data = BackupProfile::UserData;
        let includes = user_data.includes().unwrap();
        assert!(includes.contains(&PathBuf::from("workspace/projects")));
        assert!(includes.contains(&PathBuf::from(".claude")));
        assert!(includes.contains(&PathBuf::from(".gitconfig")));

        let full = BackupProfile::Full;
        assert!(
            full.includes().is_none(),
            "Full profile includes everything"
        );
    }

    #[test]
    fn test_profile_excludes() {
        let user_data = BackupProfile::UserData;
        let excludes = user_data.excludes();
        assert!(excludes.contains(&".config".to_string()));
        assert!(excludes.contains(&".local".to_string()));

        let standard = BackupProfile::Standard;
        let excludes = standard.excludes();
        assert!(excludes.contains(&".config/mise/shims".to_string()));
    }

    #[test]
    fn test_profile_default() {
        assert_eq!(BackupProfile::default(), BackupProfile::Standard);
    }

    #[test]
    fn test_profile_from_str() {
        assert_eq!(
            BackupProfile::from_str("user-data"),
            Some(BackupProfile::UserData)
        );
        assert_eq!(
            BackupProfile::from_str("userdata"),
            Some(BackupProfile::UserData)
        );
        assert_eq!(
            BackupProfile::from_str("standard"),
            Some(BackupProfile::Standard)
        );
        assert_eq!(BackupProfile::from_str("full"), Some(BackupProfile::Full));
        assert_eq!(BackupProfile::from_str("unknown"), None);
    }

    #[test]
    fn test_profile_display() {
        assert_eq!(BackupProfile::UserData.to_string(), "user-data");
        assert_eq!(BackupProfile::Standard.to_string(), "standard");
        assert_eq!(BackupProfile::Full.to_string(), "full");
    }

    #[test]
    fn test_profile_all() {
        let profiles = BackupProfile::all();
        assert_eq!(profiles.len(), 3);
        assert!(profiles.contains(&BackupProfile::UserData));
        assert!(profiles.contains(&BackupProfile::Standard));
        assert!(profiles.contains(&BackupProfile::Full));
    }

    #[test]
    fn test_profile_descriptions() {
        assert!(!BackupProfile::UserData.description().is_empty());
        assert!(!BackupProfile::Standard.description().is_empty());
        assert!(!BackupProfile::Full.description().is_empty());
    }
}
