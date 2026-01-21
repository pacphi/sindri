//! Registry and profile types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Extension registry (registry.yaml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionRegistry {
    /// Registry version
    pub version: String,

    /// Extension entries
    pub extensions: HashMap<String, RegistryEntry>,
}

/// Registry entry for an extension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    /// Extension category
    pub category: String,

    /// Short description
    pub description: String,

    /// Whether this extension is protected (core system)
    #[serde(default)]
    pub protected: bool,

    /// Dependencies (other extension names)
    #[serde(default)]
    pub dependencies: Vec<String>,

    /// Conflicts with these extensions
    #[serde(default)]
    pub conflicts: Vec<String>,
}

/// Profiles file (profiles.yaml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilesFile {
    /// Profiles version
    pub version: String,

    /// Profile definitions
    pub profiles: HashMap<String, Profile>,
}

/// Profile definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// Profile description
    pub description: String,

    /// Extensions included in this profile
    pub extensions: Vec<String>,
}

/// Compatibility matrix for CLI ↔ extension versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityMatrix {
    /// Schema version
    pub schema_version: String,

    /// CLI version compatibility
    pub cli_versions: HashMap<String, CliVersionCompat>,
}

/// CLI version compatibility entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliVersionCompat {
    /// Extension schema version
    pub extension_schema: String,

    /// Compatible extension versions
    pub compatible_extensions: HashMap<String, String>,

    /// Breaking changes in this version
    #[serde(default)]
    pub breaking_changes: Vec<String>,
}

/// Local installation manifest (~/.sindri/manifest.yaml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallManifest {
    /// Schema version
    pub schema_version: String,

    /// CLI version
    pub cli_version: String,

    /// Last update time
    pub last_updated: chrono::DateTime<chrono::Utc>,

    /// Installed extensions
    pub extensions: HashMap<String, InstalledExtension>,
}

impl Default for InstallManifest {
    fn default() -> Self {
        Self {
            schema_version: "1.0".to_string(),
            cli_version: env!("CARGO_PKG_VERSION").to_string(),
            last_updated: chrono::Utc::now(),
            extensions: HashMap::new(),
        }
    }
}

/// Installed extension entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledExtension {
    /// Installed version
    pub version: String,

    /// Installation timestamp
    pub installed_at: chrono::DateTime<chrono::Utc>,

    /// Installation source
    pub source: String,

    /// Extension state
    #[serde(default)]
    pub state: ExtensionState,
}

/// Extension installation state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExtensionState {
    /// Installed and working
    #[default]
    Installed,
    /// Installation failed
    Failed,
    /// Needs upgrade
    Outdated,
    /// Being installed
    Installing,
    /// Being removed
    Removing,
}

/// Resolved extension list with dependency order
#[derive(Debug, Clone)]
pub struct ResolvedExtensions {
    /// Extensions in installation order (dependencies first)
    pub ordered: Vec<String>,

    /// Dependency graph
    pub dependencies: HashMap<String, Vec<String>>,

    /// Any conflicts detected
    pub conflicts: Vec<ExtensionConflict>,

    /// Missing extensions
    pub missing: Vec<String>,
}

/// Extension conflict
#[derive(Debug, Clone)]
pub struct ExtensionConflict {
    /// First extension
    pub extension1: String,

    /// Second extension (conflicts with first)
    pub extension2: String,

    /// Conflict reason
    pub reason: String,
}
