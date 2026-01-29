//! Local installation manifest management
//!
//! The manifest tracks which extensions are installed locally,
//! their versions, and installation state. Located at:
//! ~/.sindri/manifest.yaml
//!
//! Structure follows section 6.3 "Manifest File" from the planning doc:
//! ```yaml
//! schema_version: "1.0"
//! cli_version: "3.0.0"
//! last_updated: "2026-01-21T10:00:00Z"
//! extensions:
//!   python:
//!     version: "1.2.0"
//!     installed_at: "2026-01-20T15:30:00Z"
//!     source: "github:pacphi/sindri"
//!     state: installed
//! ```

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use sindri_core::types::{ExtensionState, InstallManifest, InstalledExtension};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Manifest manager for tracking installed extensions
pub struct ManifestManager {
    /// Path to manifest file
    manifest_path: PathBuf,

    /// Current manifest data
    manifest: InstallManifest,
}

impl ManifestManager {
    /// Create a new manifest manager
    ///
    /// Creates the manifest file if it doesn't exist.
    pub fn new(manifest_path: PathBuf) -> Result<Self> {
        debug!("Loading manifest from: {:?}", manifest_path);

        let manifest = if manifest_path.exists() {
            Self::load_manifest(&manifest_path)?
        } else {
            info!("Creating new manifest at: {:?}", manifest_path);
            let manifest = InstallManifest::default();
            Self::ensure_parent_dir(&manifest_path)?;
            Self::save_manifest(&manifest_path, &manifest)?;
            manifest
        };

        Ok(Self {
            manifest_path,
            manifest,
        })
    }

    /// Load manifest from default location (~/.sindri/manifest.yaml)
    pub fn load_default() -> Result<Self> {
        let manifest_path = Self::default_manifest_path()?;
        Self::new(manifest_path)
    }

    /// Get the default manifest path
    pub fn default_manifest_path() -> Result<PathBuf> {
        let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
        Ok(home.join(".sindri").join("manifest.yaml"))
    }

    /// Ensure parent directory exists
    fn ensure_parent_dir(path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(())
    }

    /// Load manifest from file
    fn load_manifest(path: &Path) -> Result<InstallManifest> {
        let content = std::fs::read_to_string(path)?;
        let manifest: InstallManifest = serde_yaml::from_str(&content)?;
        debug!(
            "Loaded manifest with {} extensions",
            manifest.extensions.len()
        );
        Ok(manifest)
    }

    /// Save manifest to file
    fn save_manifest(path: &Path, manifest: &InstallManifest) -> Result<()> {
        let content = serde_yaml::to_string(manifest)?;
        std::fs::write(path, content)?;
        debug!(
            "Saved manifest with {} extensions",
            manifest.extensions.len()
        );
        Ok(())
    }

    /// Save current manifest to disk
    pub fn save(&mut self) -> Result<()> {
        self.manifest.last_updated = Utc::now();
        Self::save_manifest(&self.manifest_path, &self.manifest)
    }

    /// Check if an extension is installed
    pub fn is_installed(&self, name: &str) -> bool {
        self.manifest
            .extensions
            .get(name)
            .map(|ext| ext.state == ExtensionState::Installed)
            .unwrap_or(false)
    }

    /// Get installed extension info
    pub fn get_installed(&self, name: &str) -> Option<&InstalledExtension> {
        self.manifest.extensions.get(name)
    }

    /// Get installed version
    pub fn get_version(&self, name: &str) -> Option<&str> {
        self.manifest
            .extensions
            .get(name)
            .map(|ext| ext.version.as_str())
    }

    /// List all installed extensions
    pub fn list_installed(&self) -> Vec<(&str, &InstalledExtension)> {
        self.manifest
            .extensions
            .iter()
            .filter(|(_, ext)| ext.state == ExtensionState::Installed)
            .map(|(name, ext)| (name.as_str(), ext))
            .collect()
    }

    /// List all extensions (including failed/outdated)
    pub fn list_all(&self) -> Vec<(&str, &InstalledExtension)> {
        self.manifest
            .extensions
            .iter()
            .map(|(name, ext)| (name.as_str(), ext))
            .collect()
    }

    /// Mark extension as installed
    pub fn mark_installed(&mut self, name: &str, version: &str, source: &str) -> Result<()> {
        info!("Marking {} {} as installed", name, version);

        let extension = InstalledExtension {
            version: version.to_string(),
            installed_at: Utc::now(),
            source: source.to_string(),
            state: ExtensionState::Installed,
        };

        self.manifest.extensions.insert(name.to_string(), extension);
        self.save()
    }

    /// Mark extension as installing
    pub fn mark_installing(&mut self, name: &str, version: &str, source: &str) -> Result<()> {
        debug!("Marking {} {} as installing", name, version);

        let extension = InstalledExtension {
            version: version.to_string(),
            installed_at: Utc::now(),
            source: source.to_string(),
            state: ExtensionState::Installing,
        };

        self.manifest.extensions.insert(name.to_string(), extension);
        self.save()
    }

    /// Mark extension as failed
    pub fn mark_failed(&mut self, name: &str) -> Result<()> {
        warn!("Marking {} as failed", name);

        if let Some(ext) = self.manifest.extensions.get_mut(name) {
            ext.state = ExtensionState::Failed;
            self.save()
        } else {
            Err(anyhow!("Extension {} not found in manifest", name))
        }
    }

    /// Mark extension as outdated
    pub fn mark_outdated(&mut self, name: &str) -> Result<()> {
        debug!("Marking {} as outdated", name);

        if let Some(ext) = self.manifest.extensions.get_mut(name) {
            ext.state = ExtensionState::Outdated;
            self.save()
        } else {
            Err(anyhow!("Extension {} not found in manifest", name))
        }
    }

    /// Mark extension as removing
    pub fn mark_removing(&mut self, name: &str) -> Result<()> {
        debug!("Marking {} as removing", name);

        if let Some(ext) = self.manifest.extensions.get_mut(name) {
            ext.state = ExtensionState::Removing;
            self.save()
        } else {
            Err(anyhow!("Extension {} not found in manifest", name))
        }
    }

    /// Mark extension as uninstalled (removes from manifest)
    ///
    /// This is an alias for `remove()` to match the bash implementation's terminology
    pub fn mark_uninstalled(&mut self, name: &str) -> Result<()> {
        self.remove(name)
    }

    /// Remove extension from manifest
    pub fn remove(&mut self, name: &str) -> Result<()> {
        info!("Removing {} from manifest", name);

        if self.manifest.extensions.remove(name).is_some() {
            self.save()
        } else {
            Err(anyhow!("Extension {} not found in manifest", name))
        }
    }

    /// Update extension version
    pub fn update_version(&mut self, name: &str, new_version: &str) -> Result<()> {
        info!("Updating {} to version {}", name, new_version);

        if let Some(ext) = self.manifest.extensions.get_mut(name) {
            ext.version = new_version.to_string();
            ext.installed_at = Utc::now();
            ext.state = ExtensionState::Installed;
            self.save()
        } else {
            Err(anyhow!("Extension {} not found in manifest", name))
        }
    }

    /// Get extensions by state
    pub fn get_by_state(&self, state: ExtensionState) -> Vec<(&str, &InstalledExtension)> {
        self.manifest
            .extensions
            .iter()
            .filter(|(_, ext)| ext.state == state)
            .map(|(name, ext)| (name.as_str(), ext))
            .collect()
    }

    /// Get failed extensions
    pub fn get_failed(&self) -> Vec<(&str, &InstalledExtension)> {
        self.get_by_state(ExtensionState::Failed)
    }

    /// Get outdated extensions
    pub fn get_outdated(&self) -> Vec<(&str, &InstalledExtension)> {
        self.get_by_state(ExtensionState::Outdated)
    }

    /// Get CLI version from manifest
    pub fn cli_version(&self) -> &str {
        &self.manifest.cli_version
    }

    /// Get last update time
    pub fn last_updated(&self) -> DateTime<Utc> {
        self.manifest.last_updated
    }

    /// Update CLI version in manifest
    pub fn update_cli_version(&mut self, version: &str) -> Result<()> {
        info!("Updating CLI version in manifest to {}", version);
        self.manifest.cli_version = version.to_string();
        self.save()
    }

    /// Check if manifest needs migration
    pub fn needs_migration(&self) -> bool {
        // Compare schema version
        self.manifest.schema_version != "1.0"
    }

    /// Get all extensions as HashMap
    pub fn extensions(&self) -> &HashMap<String, InstalledExtension> {
        &self.manifest.extensions
    }

    /// Count installed extensions
    pub fn count_installed(&self) -> usize {
        self.manifest
            .extensions
            .values()
            .filter(|ext| ext.state == ExtensionState::Installed)
            .count()
    }

    /// Clear all extensions (for reset/cleanup)
    pub fn clear_all(&mut self) -> Result<()> {
        warn!("Clearing all extensions from manifest");
        self.manifest.extensions.clear();
        self.save()
    }

    /// Export manifest to another location
    pub fn export_to(&self, path: &Path) -> Result<()> {
        Self::ensure_parent_dir(path)?;
        Self::save_manifest(path, &self.manifest)
    }

    /// Import manifest from another location
    pub fn import_from(path: &Path, destination: PathBuf) -> Result<Self> {
        if !path.exists() {
            return Err(anyhow!("Import file does not exist: {:?}", path));
        }

        let manifest = Self::load_manifest(path)?;
        Self::ensure_parent_dir(&destination)?;
        Self::save_manifest(&destination, &manifest)?;

        Ok(Self {
            manifest_path: destination,
            manifest,
        })
    }

    /// Validate manifest integrity
    pub fn validate(&self) -> Result<Vec<String>> {
        let mut warnings = Vec::new();

        // Check CLI version format
        if self.manifest.cli_version.is_empty() {
            warnings.push("CLI version is empty".to_string());
        }

        // Check for invalid states
        for (name, ext) in &self.manifest.extensions {
            if ext.version.is_empty() {
                warnings.push(format!("Extension {} has empty version", name));
            }
            if ext.source.is_empty() {
                warnings.push(format!("Extension {} has empty source", name));
            }
        }

        Ok(warnings)
    }

    /// Get reference to the underlying manifest
    pub fn manifest(&self) -> &InstallManifest {
        &self.manifest
    }

    /// Get mutable reference to the underlying manifest
    pub fn manifest_mut(&mut self) -> &mut InstallManifest {
        &mut self.manifest
    }
}

impl Default for ManifestManager {
    fn default() -> Self {
        Self::load_default().expect("Failed to load default manifest")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_new_manifest() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("manifest.yaml");

        let manager = ManifestManager::new(manifest_path.clone()).unwrap();
        assert!(manifest_path.exists());
        assert_eq!(manager.count_installed(), 0);
    }

    #[test]
    fn test_mark_installed() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("manifest.yaml");

        let mut manager = ManifestManager::new(manifest_path).unwrap();
        manager
            .mark_installed("python", "3.13.0", "github:pacphi/sindri")
            .unwrap();

        assert!(manager.is_installed("python"));
        assert_eq!(manager.get_version("python"), Some("3.13.0"));
        assert_eq!(manager.count_installed(), 1);
    }

    #[test]
    fn test_mark_failed() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("manifest.yaml");

        let mut manager = ManifestManager::new(manifest_path).unwrap();
        manager
            .mark_installed("nodejs", "20.0.0", "github:pacphi/sindri")
            .unwrap();
        manager.mark_failed("nodejs").unwrap();

        assert!(!manager.is_installed("nodejs"));
        let ext = manager.get_installed("nodejs").unwrap();
        assert_eq!(ext.state, ExtensionState::Failed);
    }

    #[test]
    fn test_remove() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("manifest.yaml");

        let mut manager = ManifestManager::new(manifest_path).unwrap();
        manager
            .mark_installed("golang", "1.25.0", "github:pacphi/sindri")
            .unwrap();
        assert!(manager.is_installed("golang"));

        manager.remove("golang").unwrap();
        assert!(!manager.is_installed("golang"));
    }

    #[test]
    fn test_update_version() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("manifest.yaml");

        let mut manager = ManifestManager::new(manifest_path).unwrap();
        manager
            .mark_installed("rust", "1.70.0", "github:pacphi/sindri")
            .unwrap();

        manager.update_version("rust", "1.92.0").unwrap();
        assert_eq!(manager.get_version("rust"), Some("1.92.0"));
    }

    #[test]
    fn test_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("manifest.yaml");

        {
            let mut manager = ManifestManager::new(manifest_path.clone()).unwrap();
            manager
                .mark_installed("python", "3.13.0", "github:pacphi/sindri")
                .unwrap();
        }

        // Reload from disk
        let manager = ManifestManager::new(manifest_path).unwrap();
        assert!(manager.is_installed("python"));
        assert_eq!(manager.get_version("python"), Some("3.13.0"));
    }

    #[test]
    fn test_get_by_state() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("manifest.yaml");

        let mut manager = ManifestManager::new(manifest_path).unwrap();
        manager
            .mark_installed("python", "3.13.0", "github:pacphi/sindri")
            .unwrap();
        manager
            .mark_installed("nodejs", "20.0.0", "github:pacphi/sindri")
            .unwrap();
        manager.mark_failed("nodejs").unwrap();
        manager.mark_outdated("python").unwrap();

        let failed = manager.get_failed();
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].0, "nodejs");

        let outdated = manager.get_outdated();
        assert_eq!(outdated.len(), 1);
        assert_eq!(outdated[0].0, "python");
    }
}
