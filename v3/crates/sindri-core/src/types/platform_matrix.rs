//! Platform support matrix configuration types
//!
//! These types define platform detection, target mappings, and asset patterns
//! for multi-platform binary distribution.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete platform support matrix
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PlatformMatrix {
    /// Platform definitions
    pub platforms: HashMap<String, PlatformDefinition>,

    /// Default platform (fallback)
    #[serde(default)]
    pub default_platform: Option<String>,
}

impl Default for PlatformMatrix {
    fn default() -> Self {
        let mut platforms = HashMap::new();

        // Linux x86_64
        platforms.insert(
            "linux-x86_64".to_string(),
            PlatformDefinition {
                os: "linux".to_string(),
                arch: "x86_64".to_string(),
                target: "x86_64-unknown-linux-musl".to_string(),
                asset_pattern: Some(
                    "sindri-{version}-x86_64-unknown-linux-musl.tar.gz".to_string(),
                ),
                priority: 10,
                aliases: vec!["linux-amd64".to_string()],
                enabled: true,
            },
        );

        // Linux ARM64
        platforms.insert(
            "linux-aarch64".to_string(),
            PlatformDefinition {
                os: "linux".to_string(),
                arch: "aarch64".to_string(),
                target: "aarch64-unknown-linux-musl".to_string(),
                asset_pattern: Some(
                    "sindri-{version}-aarch64-unknown-linux-musl.tar.gz".to_string(),
                ),
                priority: 9,
                aliases: vec!["linux-arm64".to_string()],
                enabled: true,
            },
        );

        // macOS x86_64
        platforms.insert(
            "macos-x86_64".to_string(),
            PlatformDefinition {
                os: "macos".to_string(),
                arch: "x86_64".to_string(),
                target: "x86_64-apple-darwin".to_string(),
                asset_pattern: Some("sindri-{version}-x86_64-apple-darwin.tar.gz".to_string()),
                priority: 8,
                aliases: vec!["darwin-amd64".to_string(), "osx-x86_64".to_string()],
                enabled: true,
            },
        );

        // macOS ARM64 (Apple Silicon)
        platforms.insert(
            "macos-aarch64".to_string(),
            PlatformDefinition {
                os: "macos".to_string(),
                arch: "aarch64".to_string(),
                target: "aarch64-apple-darwin".to_string(),
                asset_pattern: Some("sindri-{version}-aarch64-apple-darwin.tar.gz".to_string()),
                priority: 9,
                aliases: vec!["darwin-arm64".to_string(), "osx-arm64".to_string()],
                enabled: true,
            },
        );

        // Windows x86_64
        platforms.insert(
            "windows-x86_64".to_string(),
            PlatformDefinition {
                os: "windows".to_string(),
                arch: "x86_64".to_string(),
                target: "x86_64-pc-windows-msvc".to_string(),
                asset_pattern: Some("sindri-{version}-x86_64-pc-windows-msvc.zip".to_string()),
                priority: 7,
                aliases: vec!["windows-amd64".to_string()],
                enabled: true,
            },
        );

        Self {
            platforms,
            default_platform: Some("linux-x86_64".to_string()),
        }
    }
}

/// Platform definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PlatformDefinition {
    /// Operating system (linux, macos, windows)
    pub os: String,

    /// CPU architecture (x86_64, aarch64)
    pub arch: String,

    /// Rust target triple
    pub target: String,

    /// Asset filename pattern (supports {version} placeholder)
    pub asset_pattern: Option<String>,

    /// Priority for platform selection (higher = preferred)
    #[serde(default = "default_priority")]
    pub priority: u32,

    /// Alternative names for this platform
    #[serde(default)]
    pub aliases: Vec<String>,

    /// Whether this platform is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_priority() -> u32 {
    5
}

fn default_enabled() -> bool {
    true
}

impl PlatformMatrix {
    /// Find platform definition by OS and architecture
    pub fn find_platform(&self, os: &str, arch: &str) -> Option<&PlatformDefinition> {
        // Normalize OS name
        let os_lower = os.to_lowercase();
        let normalized_os = match os_lower.as_str() {
            "darwin" | "osx" => "macos",
            other => other,
        };

        // Normalize architecture
        let arch_lower = arch.to_lowercase();
        let normalized_arch = match arch_lower.as_str() {
            "amd64" | "x64" => "x86_64",
            "arm64" => "aarch64",
            other => other,
        };

        // Try direct lookup
        let key = format!("{}-{}", normalized_os, normalized_arch);
        if let Some(platform) = self.platforms.get(&key) {
            if platform.enabled {
                return Some(platform);
            }
        }

        // Try finding by OS/arch match
        self.platforms.values().filter(|p| p.enabled).find(|p| {
            p.os.eq_ignore_ascii_case(normalized_os) && p.arch.eq_ignore_ascii_case(normalized_arch)
        })
    }

    /// Get platform definition by key
    pub fn get_platform(&self, key: &str) -> Option<&PlatformDefinition> {
        self.platforms.get(key).filter(|p| p.enabled)
    }

    /// Get all enabled platforms sorted by priority
    pub fn enabled_platforms(&self) -> Vec<(&String, &PlatformDefinition)> {
        let mut platforms: Vec<_> = self.platforms.iter().filter(|(_, p)| p.enabled).collect();
        platforms.sort_by(|a, b| b.1.priority.cmp(&a.1.priority));
        platforms
    }

    /// Get the default platform definition
    pub fn get_default(&self) -> Option<&PlatformDefinition> {
        self.default_platform
            .as_ref()
            .and_then(|key| self.get_platform(key))
    }
}

impl PlatformDefinition {
    /// Get asset filename for a specific version
    pub fn asset_filename(&self, version: &str) -> String {
        if let Some(pattern) = &self.asset_pattern {
            pattern.replace("{version}", version)
        } else {
            // Fallback pattern
            format!("sindri-{}-{}.tar.gz", version, self.target)
        }
    }

    /// Check if this platform matches the given OS and architecture
    pub fn matches(&self, os: &str, arch: &str) -> bool {
        self.os.eq_ignore_ascii_case(os) && self.arch.eq_ignore_ascii_case(arch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_matrix_defaults() {
        let matrix = PlatformMatrix::default();
        assert!(matrix.platforms.contains_key("linux-x86_64"));
        assert!(matrix.platforms.contains_key("linux-aarch64"));
        assert!(matrix.platforms.contains_key("macos-x86_64"));
        assert!(matrix.platforms.contains_key("macos-aarch64"));
        assert!(matrix.platforms.contains_key("windows-x86_64"));
        assert_eq!(matrix.platforms.len(), 5);
    }

    #[test]
    fn test_find_platform_exact_match() {
        let matrix = PlatformMatrix::default();
        let platform = matrix.find_platform("linux", "x86_64").unwrap();
        assert_eq!(platform.target, "x86_64-unknown-linux-musl");
    }

    #[test]
    fn test_find_platform_normalized() {
        let matrix = PlatformMatrix::default();

        // Test Darwin -> macOS normalization
        let platform = matrix.find_platform("darwin", "x86_64").unwrap();
        assert_eq!(platform.os, "macos");

        // Test amd64 -> x86_64 normalization
        let platform = matrix.find_platform("linux", "amd64").unwrap();
        assert_eq!(platform.arch, "x86_64");

        // Test arm64 -> aarch64 normalization
        let platform = matrix.find_platform("linux", "arm64").unwrap();
        assert_eq!(platform.arch, "aarch64");
    }

    #[test]
    fn test_platform_asset_filename() {
        let matrix = PlatformMatrix::default();
        let platform = matrix.platforms.get("linux-x86_64").unwrap();
        let filename = platform.asset_filename("3.0.0");
        assert_eq!(filename, "sindri-3.0.0-x86_64-unknown-linux-musl.tar.gz");
    }

    #[test]
    fn test_enabled_platforms_sorted() {
        let matrix = PlatformMatrix::default();
        let platforms = matrix.enabled_platforms();
        assert_eq!(platforms.len(), 5);

        // Verify sorted by priority (descending)
        for i in 1..platforms.len() {
            assert!(platforms[i - 1].1.priority >= platforms[i].1.priority);
        }
    }

    #[test]
    fn test_platform_matches() {
        let platform = PlatformDefinition {
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
            target: "x86_64-unknown-linux-musl".to_string(),
            asset_pattern: None,
            priority: 10,
            aliases: vec![],
            enabled: true,
        };

        assert!(platform.matches("linux", "x86_64"));
        assert!(platform.matches("Linux", "X86_64")); // Case insensitive
        assert!(!platform.matches("linux", "aarch64"));
        assert!(!platform.matches("macos", "x86_64"));
    }

    #[test]
    fn test_platform_serialization() {
        let matrix = PlatformMatrix::default();
        let yaml = serde_yaml_ng::to_string(&matrix).unwrap();
        let deserialized: PlatformMatrix = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(matrix.platforms.len(), deserialized.platforms.len());
    }

    #[test]
    fn test_default_platform() {
        let matrix = PlatformMatrix::default();
        let default = matrix.get_default().unwrap();
        assert_eq!(default.os, "linux");
        assert_eq!(default.arch, "x86_64");
    }
}
