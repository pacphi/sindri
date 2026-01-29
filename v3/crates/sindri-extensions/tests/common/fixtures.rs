//! Test fixture loading utilities
//!
//! Provides helpers for loading and managing test fixtures including
//! extension YAML files, manifest files, and temporary directories.

#![allow(dead_code)]

use anyhow::{Context, Result};
use sindri_core::types::Extension;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Fixture manager for test extensions
pub struct FixtureManager {
    /// Temporary directory for test operations
    temp_dir: TempDir,
    /// Base fixtures directory
    fixtures_base: PathBuf,
}

impl FixtureManager {
    /// Create a new fixture manager
    pub fn new() -> Result<Self> {
        let temp_dir = TempDir::new().context("Failed to create temp directory")?;

        // Determine fixtures base relative to the test file
        let fixtures_base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures");

        Ok(Self {
            temp_dir,
            fixtures_base,
        })
    }

    /// Get the temporary directory path
    pub fn temp_path(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Get the extensions fixture directory
    pub fn extensions_dir(&self) -> PathBuf {
        self.fixtures_base.join("extensions")
    }

    /// Get the manifests fixture directory
    pub fn manifests_dir(&self) -> PathBuf {
        self.fixtures_base.join("manifests")
    }

    /// Create a temporary extension directory with a test script
    pub fn create_extension_dir(&self, name: &str) -> Result<PathBuf> {
        let ext_dir = self.temp_dir.path().join(name);
        std::fs::create_dir_all(&ext_dir).context("Failed to create extension directory")?;

        // Create scripts directory
        let scripts_dir = ext_dir.join("scripts");
        std::fs::create_dir_all(&scripts_dir).context("Failed to create scripts directory")?;

        // Create a basic install script
        let install_script = scripts_dir.join("install.sh");
        std::fs::write(
            &install_script,
            r#"#!/bin/bash
echo "Installing test extension..."
exit 0
"#,
        )
        .context("Failed to write install script")?;

        // Make script executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o755);
            std::fs::set_permissions(&install_script, perms)?;
        }

        Ok(ext_dir)
    }

    /// Create a temporary extension directory with mise config
    pub fn create_mise_extension_dir(&self, name: &str, tools: &[&str]) -> Result<PathBuf> {
        let ext_dir = self.create_extension_dir(name)?;

        // Create mise.toml
        let mise_content = tools
            .iter()
            .map(|t| format!("{} = \"latest\"", t))
            .collect::<Vec<_>>()
            .join("\n");

        let mise_config = format!("[tools]\n{}\n", mise_content);
        std::fs::write(ext_dir.join("mise.toml"), mise_config)
            .context("Failed to write mise.toml")?;

        Ok(ext_dir)
    }

    /// Create a workspace directory
    pub fn create_workspace(&self) -> Result<PathBuf> {
        let workspace = self.temp_dir.path().join("workspace");
        std::fs::create_dir_all(&workspace).context("Failed to create workspace")?;
        Ok(workspace)
    }

    /// Create a home directory with standard structure
    pub fn create_home(&self) -> Result<PathBuf> {
        let home = self.temp_dir.path().join("home");
        std::fs::create_dir_all(&home).context("Failed to create home directory")?;

        // Create standard directories
        std::fs::create_dir_all(home.join(".sindri/extensions"))?;
        std::fs::create_dir_all(home.join(".sindri/state"))?;
        std::fs::create_dir_all(home.join(".sindri/cache"))?;
        std::fs::create_dir_all(home.join(".config/mise/conf.d"))?;
        std::fs::create_dir_all(home.join(".local/share/mise/shims"))?;

        Ok(home)
    }

    /// Load a fixture extension YAML file
    pub fn load_extension_yaml(&self, name: &str) -> Result<String> {
        let path = self.extensions_dir().join(format!("{}.yaml", name));
        std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read fixture: {:?}", path))
    }

    /// Parse an extension from YAML string
    pub fn parse_extension(&self, yaml: &str) -> Result<Extension> {
        serde_yaml::from_str(yaml).context("Failed to parse extension YAML")
    }

    /// Load and parse a fixture extension
    pub fn load_extension(&self, name: &str) -> Result<Extension> {
        let yaml = self.load_extension_yaml(name)?;
        self.parse_extension(&yaml)
    }

    /// Write an extension YAML to the temp directory
    pub fn write_extension(&self, name: &str, yaml: &str) -> Result<PathBuf> {
        let ext_dir = self.create_extension_dir(name)?;
        let yaml_path = ext_dir.join("extension.yaml");
        std::fs::write(&yaml_path, yaml).context("Failed to write extension YAML")?;
        Ok(yaml_path)
    }

    /// Create a manifest file
    pub fn create_manifest(&self, content: &str) -> Result<PathBuf> {
        let manifest_path = self.temp_dir.path().join("manifest.json");
        std::fs::write(&manifest_path, content).context("Failed to write manifest")?;
        Ok(manifest_path)
    }
}

impl Default for FixtureManager {
    fn default() -> Self {
        Self::new().expect("Failed to create FixtureManager")
    }
}

/// Create a minimal extension YAML from parts
pub fn create_extension_yaml(name: &str, version: &str, method: &str, category: &str) -> String {
    format!(
        r#"metadata:
  name: {name}
  version: "{version}"
  description: Test extension {name}
  category: {category}

install:
  method: {method}
  script:
    path: scripts/install.sh
    timeout: 60

validate:
  commands:
    - name: echo
      versionFlag: "test"
"#
    )
}

/// Create a mise-based extension YAML
pub fn create_mise_extension_yaml(name: &str, version: &str, tools: &[&str]) -> String {
    let tools_yaml = tools
        .iter()
        .map(|t| format!("      - {}@latest", t))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"metadata:
  name: {name}
  version: "{version}"
  description: Mise-based extension {name}
  category: languages

install:
  method: mise
  mise:
    configFile: mise.toml
    reshim_after_install: true

validate:
  mise:
    tools:
{tools_yaml}
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixture_manager_creation() {
        let manager = FixtureManager::new().unwrap();
        assert!(manager.temp_path().exists());
    }

    #[test]
    fn test_create_extension_dir() {
        let manager = FixtureManager::new().unwrap();
        let ext_dir = manager.create_extension_dir("test-ext").unwrap();
        assert!(ext_dir.exists());
        assert!(ext_dir.join("scripts/install.sh").exists());
    }

    #[test]
    fn test_create_mise_extension_dir() {
        let manager = FixtureManager::new().unwrap();
        let ext_dir = manager
            .create_mise_extension_dir("mise-ext", &["python", "node"])
            .unwrap();
        assert!(ext_dir.join("mise.toml").exists());
        let content = std::fs::read_to_string(ext_dir.join("mise.toml")).unwrap();
        assert!(content.contains("python"));
        assert!(content.contains("node"));
    }

    #[test]
    fn test_create_workspace_and_home() {
        let manager = FixtureManager::new().unwrap();
        let workspace = manager.create_workspace().unwrap();
        let home = manager.create_home().unwrap();
        assert!(workspace.exists());
        assert!(home.join(".sindri/extensions").exists());
        assert!(home.join(".config/mise/conf.d").exists());
    }

    #[test]
    fn test_create_extension_yaml() {
        let yaml = create_extension_yaml("test", "1.0.0", "script", "testing");
        assert!(yaml.contains("name: test"));
        assert!(yaml.contains("version: \"1.0.0\""));
        assert!(yaml.contains("method: script"));
    }

    #[test]
    fn test_create_mise_extension_yaml() {
        let yaml = create_mise_extension_yaml("mise-test", "1.0.0", &["python", "node"]);
        assert!(yaml.contains("method: mise"));
        assert!(yaml.contains("python@latest"));
        assert!(yaml.contains("node@latest"));
    }
}
