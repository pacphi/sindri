//! Hierarchical configuration loader with precedence
//!
//! Loads configuration from multiple sources with the following precedence (low to high):
//! 1. Embedded defaults (built into binary)
//! 2. Global config (~/.sindri/config.yaml)
//! 3. Runtime config (~/.sindri/sindri-runtime.yaml)
//! 4. Environment variables (SINDRI_* prefix)
//! 5. CLI flags (handled by caller)

use crate::error::{Error, Result};
use crate::types::{PlatformMatrix, RuntimeConfig};
use camino::{Utf8Path, Utf8PathBuf};
use rust_embed::RustEmbed;
use serde::de::DeserializeOwned;
use std::env;
use std::fs;

/// Embedded configuration files
#[derive(RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/../../embedded/config/"]
#[prefix = ""]
struct EmbeddedConfigs;

/// Configuration hierarchy loader
pub struct HierarchicalConfigLoader {
    /// Base directory for configuration files
    config_dir: Utf8PathBuf,
}

impl HierarchicalConfigLoader {
    /// Create a new hierarchical config loader
    pub fn new() -> Result<Self> {
        let config_dir = Self::get_config_dir()?;
        Ok(Self { config_dir })
    }

    /// Create a loader with a custom config directory
    pub fn with_dir(config_dir: Utf8PathBuf) -> Self {
        Self { config_dir }
    }

    /// Get the standard config directory (~/.sindri)
    fn get_config_dir() -> Result<Utf8PathBuf> {
        let home = env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .map_err(|_| Error::invalid_config("Could not determine home directory"))?;

        let config_dir = Utf8PathBuf::from(home).join(".sindri");

        // Create directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;
        }

        Ok(config_dir)
    }

    /// Load runtime configuration with hierarchical precedence
    pub fn load_runtime_config(&self) -> Result<RuntimeConfig> {
        // Start with embedded defaults
        let mut config = Self::load_embedded_config::<RuntimeConfig>("runtime-defaults.yaml")?;

        // Load from global runtime config if it exists
        let runtime_config_path = self.config_dir.join("sindri-runtime.yaml");
        if runtime_config_path.exists() {
            let file_config = self.load_yaml_file::<RuntimeConfig>(&runtime_config_path)?;
            config = Self::merge_runtime_config(config, file_config);
        }

        // Apply environment variable overrides
        config = self.apply_env_overrides(config)?;

        Ok(config)
    }

    /// Load platform matrix configuration
    pub fn load_platform_matrix(&self) -> Result<PlatformMatrix> {
        // Start with embedded defaults
        let mut matrix = Self::load_embedded_config::<PlatformMatrix>("platform-rules.yaml")?;

        // Load from global platform rules if it exists
        let platform_rules_path = self.config_dir.join("platform-rules.yaml");
        if platform_rules_path.exists() {
            let file_matrix = self.load_yaml_file::<PlatformMatrix>(&platform_rules_path)?;
            matrix = Self::merge_platform_matrix(matrix, file_matrix);
        }

        Ok(matrix)
    }

    /// Load an embedded configuration file
    fn load_embedded_config<T: DeserializeOwned>(filename: &str) -> Result<T> {
        let embedded_file = EmbeddedConfigs::get(filename).ok_or_else(|| {
            Error::config_not_found(format!("Embedded config not found: {}", filename))
        })?;

        let content = std::str::from_utf8(&embedded_file.data).map_err(|_| {
            Error::invalid_config(format!("Invalid UTF-8 in embedded config: {}", filename))
        })?;

        let config: T = serde_yaml_ng::from_str(content).map_err(|e| {
            Error::invalid_config(format!(
                "Failed to parse embedded config {}: {}",
                filename, e
            ))
        })?;

        Ok(config)
    }

    /// Load a YAML file and parse it
    fn load_yaml_file<T: DeserializeOwned>(&self, path: &Utf8Path) -> Result<T> {
        let content = fs::read_to_string(path)?;
        let config: T = serde_yaml_ng::from_str(&content)
            .map_err(|e| Error::invalid_config(format!("Failed to parse {}: {}", path, e)))?;
        Ok(config)
    }

    /// Merge two runtime configs (base is overridden by overlay)
    fn merge_runtime_config(base: RuntimeConfig, overlay: RuntimeConfig) -> RuntimeConfig {
        RuntimeConfig {
            network: overlay.network,
            retry_policies: Self::merge_retry_policies(base.retry_policies, overlay.retry_policies),
            github: overlay.github,
            backup: overlay.backup,
            git_workflow: overlay.git_workflow,
            display: overlay.display,
        }
    }

    /// Merge retry policies
    fn merge_retry_policies(
        mut base: crate::types::RetryPoliciesConfig,
        overlay: crate::types::RetryPoliciesConfig,
    ) -> crate::types::RetryPoliciesConfig {
        // Merge operation-specific policies
        for (key, policy) in overlay.operations {
            base.operations.insert(key, policy);
        }
        base.default = overlay.default;
        base
    }

    /// Merge platform matrices
    fn merge_platform_matrix(mut base: PlatformMatrix, overlay: PlatformMatrix) -> PlatformMatrix {
        // Overlay platforms take precedence
        for (key, platform) in overlay.platforms {
            base.platforms.insert(key, platform);
        }

        // Use overlay default if specified
        if overlay.default_platform.is_some() {
            base.default_platform = overlay.default_platform;
        }

        base
    }

    /// Apply environment variable overrides to runtime config
    fn apply_env_overrides(&self, mut config: RuntimeConfig) -> Result<RuntimeConfig> {
        // Network timeouts
        if let Ok(val) = env::var("SINDRI_HTTP_TIMEOUT_SECS") {
            config.network.http_timeout_secs = val.parse().map_err(|_| {
                Error::invalid_config("SINDRI_HTTP_TIMEOUT_SECS must be a valid number")
            })?;
        }

        if let Ok(val) = env::var("SINDRI_DOWNLOAD_TIMEOUT_SECS") {
            config.network.download_timeout_secs = val.parse().map_err(|_| {
                Error::invalid_config("SINDRI_DOWNLOAD_TIMEOUT_SECS must be a valid number")
            })?;
        }

        if let Ok(val) = env::var("SINDRI_DEPLOY_TIMEOUT_SECS") {
            config.network.deploy_timeout_secs = val.parse().map_err(|_| {
                Error::invalid_config("SINDRI_DEPLOY_TIMEOUT_SECS must be a valid number")
            })?;
        }

        if let Ok(val) = env::var("SINDRI_DOWNLOAD_CHUNK_SIZE") {
            config.network.download_chunk_size = val.parse().map_err(|_| {
                Error::invalid_config("SINDRI_DOWNLOAD_CHUNK_SIZE must be a valid number")
            })?;
        }

        if let Ok(val) = env::var("SINDRI_MISE_TIMEOUT_SECS") {
            config.network.mise_timeout_secs = val.parse().map_err(|_| {
                Error::invalid_config("SINDRI_MISE_TIMEOUT_SECS must be a valid number")
            })?;
        }

        // GitHub configuration
        if let Ok(val) = env::var("SINDRI_GITHUB_REPO_OWNER") {
            config.github.repo_owner = val;
        }

        if let Ok(val) = env::var("SINDRI_GITHUB_REPO_NAME") {
            config.github.repo_name = val;
        }

        // Backup configuration
        if let Ok(val) = env::var("SINDRI_MAX_BACKUPS") {
            config.backup.max_backups = val
                .parse()
                .map_err(|_| Error::invalid_config("SINDRI_MAX_BACKUPS must be a valid number"))?;
        }

        // Display configuration
        if let Ok(val) = env::var("SINDRI_VERBOSE") {
            config.display.verbose = val.parse().unwrap_or(false);
        }

        if let Ok(val) = env::var("SINDRI_NO_COLOR") {
            config.display.color_enabled = !val.parse().unwrap_or(false);
        }

        Ok(config)
    }

    /// Get the config directory path
    pub fn config_dir(&self) -> &Utf8Path {
        &self.config_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tempfile::TempDir;

    fn create_temp_loader() -> (HierarchicalConfigLoader, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config_dir =
            Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf()).expect("Invalid UTF-8 path");
        let loader = HierarchicalConfigLoader::with_dir(config_dir);
        (loader, temp_dir)
    }

    #[test]
    #[serial]
    fn test_load_runtime_config_defaults() {
        let (loader, _temp) = create_temp_loader();
        let config = loader.load_runtime_config().unwrap();
        assert_eq!(config.network.http_timeout_secs, 300);
        assert_eq!(config.github.repo_owner, "pacphi");
    }

    #[test]
    #[serial]
    fn test_load_runtime_config_from_file() {
        let (loader, _temp) = create_temp_loader();

        // Write a custom config
        let config_content = r#"
network:
  http-timeout-secs: 600
  download-timeout-secs: 900
github:
  repo-owner: "custom-owner"
  repo-name: "custom-repo"
"#;
        let config_path = loader.config_dir().join("sindri-runtime.yaml");
        fs::write(&config_path, config_content).unwrap();

        let config = loader.load_runtime_config().unwrap();
        assert_eq!(config.network.http_timeout_secs, 600);
        assert_eq!(config.network.download_timeout_secs, 900);
        assert_eq!(config.github.repo_owner, "custom-owner");
        assert_eq!(config.github.repo_name, "custom-repo");
    }

    #[test]
    fn test_load_platform_matrix_defaults() {
        let (loader, _temp) = create_temp_loader();
        let matrix = loader.load_platform_matrix().unwrap();
        assert!(matrix.platforms.contains_key("linux-x86_64"));
        assert_eq!(matrix.platforms.len(), 5);
    }

    #[test]
    fn test_load_platform_matrix_from_file() {
        let (loader, _temp) = create_temp_loader();

        // Write custom platform rules
        let platform_content = r#"
platforms:
  custom-platform:
    os: "custom-os"
    arch: "custom-arch"
    target: "custom-target"
    priority: 100
    enabled: true
default-platform: "custom-platform"
"#;
        let platform_path = loader.config_dir().join("platform-rules.yaml");
        fs::write(&platform_path, platform_content).unwrap();

        let matrix = loader.load_platform_matrix().unwrap();
        assert!(matrix.platforms.contains_key("custom-platform"));
        assert_eq!(matrix.default_platform, Some("custom-platform".to_string()));
    }

    #[test]
    #[serial]
    fn test_env_overrides() {
        let (loader, _temp) = create_temp_loader();

        // Set environment variables
        env::set_var("SINDRI_HTTP_TIMEOUT_SECS", "1200");
        env::set_var("SINDRI_MISE_TIMEOUT_SECS", "600");
        env::set_var("SINDRI_GITHUB_REPO_OWNER", "env-owner");
        env::set_var("SINDRI_MAX_BACKUPS", "5");

        let config = loader.load_runtime_config().unwrap();
        assert_eq!(config.network.http_timeout_secs, 1200);
        assert_eq!(config.network.mise_timeout_secs, 600);
        assert_eq!(config.github.repo_owner, "env-owner");
        assert_eq!(config.backup.max_backups, 5);

        // Clean up
        env::remove_var("SINDRI_HTTP_TIMEOUT_SECS");
        env::remove_var("SINDRI_MISE_TIMEOUT_SECS");
        env::remove_var("SINDRI_GITHUB_REPO_OWNER");
        env::remove_var("SINDRI_MAX_BACKUPS");
    }

    #[test]
    fn test_merge_runtime_config() {
        let base = RuntimeConfig::default();
        let mut overlay = RuntimeConfig::default();
        overlay.network.http_timeout_secs = 999;
        overlay.github.repo_owner = "merged-owner".to_string();

        let merged = HierarchicalConfigLoader::merge_runtime_config(base, overlay);
        assert_eq!(merged.network.http_timeout_secs, 999);
        assert_eq!(merged.github.repo_owner, "merged-owner");
    }

    #[test]
    fn test_config_dir_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = Utf8PathBuf::from_path_buf(temp_dir.path().join("new_config"))
            .expect("Invalid UTF-8 path");

        // Directory doesn't exist yet
        assert!(!config_dir.exists());

        let loader = HierarchicalConfigLoader::with_dir(config_dir.clone());
        assert_eq!(loader.config_dir(), config_dir);
    }
}
