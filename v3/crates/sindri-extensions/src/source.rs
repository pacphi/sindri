//! Extension source abstraction
//!
//! Provides an enum-based pattern for loading extensions from multiple sources:
//! - Bundled: Pre-packaged in Docker images at /opt/sindri/extensions
//! - Downloaded: Fetched from GitHub releases to ~/.sindri/extensions
//! - LocalDev: Development mode from v3/extensions/
//!
//! The `ExtensionSourceResolver` tries each source in priority order, enabling
//! both bundled and download modes transparently.

use anyhow::{anyhow, Context, Result};
use semver::Version;
use sindri_core::types::Extension;
use std::path::PathBuf;
use tracing::{debug, info};

// Re-export for convenience
pub use crate::distribution::ExtensionDistributor;

/// Extension source type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceType {
    Bundled,
    Downloaded,
    LocalDev,
}

impl std::fmt::Display for SourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceType::Bundled => write!(f, "bundled"),
            SourceType::Downloaded => write!(f, "downloaded"),
            SourceType::LocalDev => write!(f, "local-dev"),
        }
    }
}

/// Bundled extensions source
///
/// Loads extensions from a bundled directory (e.g., /opt/sindri/extensions).
/// Used in Docker builds where extensions are pre-packaged.
#[derive(Debug, Clone)]
pub struct BundledSource {
    pub base_path: PathBuf,
}

impl BundledSource {
    /// Create a new bundled source
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
        }
    }

    /// Create from SINDRI_EXT_HOME environment variable if it points to bundled path
    pub fn from_env() -> Option<Self> {
        std::env::var("SINDRI_EXT_HOME").ok().and_then(|path| {
            let path = PathBuf::from(&path);
            // Only use as bundled source if it's the /opt path (not user home)
            if path.starts_with("/opt/sindri") && path.exists() {
                Some(Self::new(path))
            } else {
                None
            }
        })
    }

    /// Check if extension is available
    pub fn is_available(&self, name: &str) -> bool {
        self.extension_path(name).is_some()
    }

    /// Get the extension.yaml path if it exists
    pub fn extension_path(&self, name: &str) -> Option<PathBuf> {
        let path = self.base_path.join(name).join("extension.yaml");
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    /// Load extension from bundled source
    pub fn get_extension(&self, name: &str) -> Result<Extension> {
        let ext_path = self
            .extension_path(name)
            .ok_or_else(|| anyhow!("Extension '{}' not found in bundled source", name))?;

        debug!(
            "Loading extension '{}' from bundled path: {:?}",
            name, ext_path
        );

        let content = std::fs::read_to_string(&ext_path)
            .context(format!("Failed to read extension file: {:?}", ext_path))?;

        let extension: Extension = serde_yaml::from_str(&content)
            .context(format!("Failed to parse extension.yaml for '{}'", name))?;

        Ok(extension)
    }
}

/// Downloaded extensions source
///
/// Loads extensions from the user's extensions directory (~/.sindri/extensions).
/// Extensions are downloaded from GitHub releases on demand.
#[derive(Debug, Clone)]
pub struct DownloadedSource {
    pub extensions_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub cli_version: Version,
}

impl DownloadedSource {
    /// Create a new downloaded source
    pub fn new(extensions_dir: PathBuf, cache_dir: PathBuf, cli_version: Version) -> Self {
        Self {
            extensions_dir,
            cache_dir,
            cli_version,
        }
    }

    /// Create from environment/defaults
    pub fn from_env() -> Result<Self> {
        let home = sindri_core::get_home_dir()?;

        let extensions_dir = std::env::var("SINDRI_EXT_HOME")
            .ok()
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".sindri/extensions"));

        let cache_dir = home.join(".sindri/cache");

        let cli_version =
            Version::parse(env!("CARGO_PKG_VERSION")).unwrap_or_else(|_| Version::new(3, 0, 0));

        Ok(Self::new(extensions_dir, cache_dir, cli_version))
    }

    /// Check if extension is already downloaded
    pub fn is_available(&self, name: &str) -> bool {
        self.find_version_dir(name).is_some()
    }

    /// Get the extension.yaml path if downloaded
    pub fn extension_path(&self, name: &str) -> Option<PathBuf> {
        self.find_version_dir(name)
            .map(|d| d.join("extension.yaml"))
    }

    /// Find the latest version directory for an extension
    pub fn find_version_dir(&self, name: &str) -> Option<PathBuf> {
        let ext_base = self.extensions_dir.join(name);
        if !ext_base.exists() {
            return None;
        }

        let versions: Vec<_> = std::fs::read_dir(&ext_base)
            .ok()?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().is_dir())
            .filter(|entry| entry.path().join("extension.yaml").exists())
            .collect();

        // Return newest version (reversed directory order)
        versions.into_iter().next_back().map(|e| e.path())
    }

    /// Load extension from downloaded source (already downloaded)
    pub fn get_extension(&self, name: &str) -> Result<Extension> {
        let version_dir = self
            .find_version_dir(name)
            .ok_or_else(|| anyhow!("Extension '{}' not downloaded", name))?;

        let ext_path = version_dir.join("extension.yaml");
        debug!(
            "Loading extension '{}' from downloaded path: {:?}",
            name, ext_path
        );

        let content = std::fs::read_to_string(&ext_path)
            .context(format!("Failed to read extension file: {:?}", ext_path))?;

        let extension: Extension = serde_yaml::from_str(&content)
            .context(format!("Failed to parse extension.yaml for '{}'", name))?;

        Ok(extension)
    }

    /// Download extension from GitHub and then load it
    ///
    /// This downloads extension metadata without executing installation.
    /// Used by listing and info commands that need extension data without
    /// modifying system state.
    pub async fn download_and_get(&self, name: &str) -> Result<Extension> {
        info!(
            "Extension '{}' not found locally, downloading from GitHub...",
            name
        );

        let distributor = ExtensionDistributor::new(
            self.cache_dir.clone(),
            self.extensions_dir.clone(),
            self.cli_version.clone(),
        )?;

        // Download metadata only - no installation execution
        distributor
            .download_metadata(name, None)
            .await
            .context(format!(
                "Failed to download extension metadata for '{}'",
                name
            ))
    }
}

/// Local development source
///
/// Loads extensions from the v3/extensions/ directory during development.
/// Only available when running from the source tree.
#[derive(Debug, Clone)]
pub struct LocalDevSource {
    pub extensions_path: PathBuf,
}

impl LocalDevSource {
    /// Create a new local dev source
    pub fn new(extensions_path: impl Into<PathBuf>) -> Self {
        Self {
            extensions_path: extensions_path.into(),
        }
    }

    /// Detect local dev source from CARGO_MANIFEST_DIR
    pub fn detect() -> Option<Self> {
        // Only works in development (cargo run)
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        let extensions_path = manifest_dir
            .parent()? // sindri-extensions -> crates
            .parent()? // crates -> v3
            .join("extensions");

        if extensions_path.exists() {
            Some(Self::new(extensions_path))
        } else {
            None
        }
    }

    /// Check if extension is available
    pub fn is_available(&self, name: &str) -> bool {
        self.extension_path(name).is_some()
    }

    /// Get the extension.yaml path if it exists
    pub fn extension_path(&self, name: &str) -> Option<PathBuf> {
        let path = self.extensions_path.join(name).join("extension.yaml");
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    /// Load extension from local dev source
    pub fn get_extension(&self, name: &str) -> Result<Extension> {
        let ext_path = self
            .extension_path(name)
            .ok_or_else(|| anyhow!("Extension '{}' not found in local dev source", name))?;

        debug!(
            "Loading extension '{}' from local dev path: {:?}",
            name, ext_path
        );

        let content = std::fs::read_to_string(&ext_path)
            .context(format!("Failed to read extension file: {:?}", ext_path))?;

        let extension: Extension = serde_yaml::from_str(&content)
            .context(format!("Failed to parse extension.yaml for '{}'", name))?;

        Ok(extension)
    }
}

/// Extension source resolver
///
/// Tries multiple sources in priority order:
/// 1. Bundled (if available) - fastest, pre-packaged
/// 2. Local dev (if in dev environment) - for development
/// 3. Downloaded - fallback, fetches from GitHub
pub struct ExtensionSourceResolver {
    bundled: Option<BundledSource>,
    local_dev: Option<LocalDevSource>,
    downloaded: DownloadedSource,
}

impl ExtensionSourceResolver {
    /// Create a new resolver with explicit sources
    pub fn new(
        bundled: Option<BundledSource>,
        local_dev: Option<LocalDevSource>,
        downloaded: DownloadedSource,
    ) -> Self {
        Self {
            bundled,
            local_dev,
            downloaded,
        }
    }

    /// Create resolver from environment
    ///
    /// Priority order:
    /// 1. Bundled source (if SINDRI_EXT_HOME points to /opt/sindri)
    /// 2. Local dev source (if running from source tree)
    /// 3. Downloaded source (always available as fallback)
    pub fn from_env() -> Result<Self> {
        let bundled = BundledSource::from_env();
        if bundled.is_some() {
            debug!("Bundled source available");
        }

        let local_dev = LocalDevSource::detect();
        if local_dev.is_some() {
            debug!("Local dev source available");
        }

        let downloaded = DownloadedSource::from_env()?;
        debug!(
            "Downloaded source configured: {:?}",
            downloaded.extensions_dir
        );

        Ok(Self::new(bundled, local_dev, downloaded))
    }

    /// Check if extension is available in any local source (without downloading)
    pub fn is_available_locally(&self, name: &str) -> bool {
        if let Some(ref bundled) = self.bundled {
            if bundled.is_available(name) {
                return true;
            }
        }

        if let Some(ref local_dev) = self.local_dev {
            if local_dev.is_available(name) {
                return true;
            }
        }

        self.downloaded.is_available(name)
    }

    /// Find which source has the extension
    pub fn find_source(&self, name: &str) -> Option<SourceType> {
        if let Some(ref bundled) = self.bundled {
            if bundled.is_available(name) {
                return Some(SourceType::Bundled);
            }
        }

        if let Some(ref local_dev) = self.local_dev {
            if local_dev.is_available(name) {
                return Some(SourceType::LocalDev);
            }
        }

        if self.downloaded.is_available(name) {
            return Some(SourceType::Downloaded);
        }

        None
    }

    /// Get the extension.yaml path from any available source
    pub fn extension_path(&self, name: &str) -> Option<PathBuf> {
        if let Some(ref bundled) = self.bundled {
            if let Some(path) = bundled.extension_path(name) {
                return Some(path);
            }
        }

        if let Some(ref local_dev) = self.local_dev {
            if let Some(path) = local_dev.extension_path(name) {
                return Some(path);
            }
        }

        self.downloaded.extension_path(name)
    }

    /// Get extension from any available source
    ///
    /// Tries sources in priority order. If not available locally,
    /// downloads from GitHub as a fallback.
    pub async fn get_extension(&self, name: &str) -> Result<Extension> {
        // Priority 1: Bundled source
        if let Some(ref bundled) = self.bundled {
            if bundled.is_available(name) {
                debug!("Loading extension '{}' from bundled source", name);
                return bundled.get_extension(name);
            }
        }

        // Priority 2: Local dev source
        if let Some(ref local_dev) = self.local_dev {
            if local_dev.is_available(name) {
                debug!("Loading extension '{}' from local-dev source", name);
                return local_dev.get_extension(name);
            }
        }

        // Priority 3: Downloaded source (check if already downloaded)
        if self.downloaded.is_available(name) {
            debug!("Loading extension '{}' from downloaded source", name);
            return self.downloaded.get_extension(name);
        }

        // Fallback: Download from GitHub
        debug!("Extension '{}' not available locally, downloading", name);
        self.downloaded.download_and_get(name).await
    }

    /// Get extension from local sources only (no download)
    pub fn get_extension_local(&self, name: &str) -> Result<Extension> {
        // Priority 1: Bundled source
        if let Some(ref bundled) = self.bundled {
            if bundled.is_available(name) {
                return bundled.get_extension(name);
            }
        }

        // Priority 2: Local dev source
        if let Some(ref local_dev) = self.local_dev {
            if local_dev.is_available(name) {
                return local_dev.get_extension(name);
            }
        }

        // Priority 3: Downloaded source (already downloaded only)
        if self.downloaded.is_available(name) {
            return self.downloaded.get_extension(name);
        }

        Err(anyhow!(
            "Extension '{}' not found in any local source. \
             Use get_extension() to allow downloading.",
            name
        ))
    }

    /// Check if running in bundled mode
    pub fn is_bundled_mode(&self) -> bool {
        self.bundled.is_some()
    }

    /// Check if running in development mode
    pub fn is_dev_mode(&self) -> bool {
        self.local_dev.is_some()
    }

    /// Get the downloaded source for direct access
    pub fn downloaded(&self) -> &DownloadedSource {
        &self.downloaded
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Create a minimal extension.yaml for testing
    fn create_test_extension(dir: &std::path::Path, name: &str, version: &str) {
        let ext_dir = dir.join(name);
        fs::create_dir_all(&ext_dir).unwrap();

        let extension_yaml = format!(
            r#"
metadata:
  name: {name}
  version: {version}
  description: Test extension
  category: testing
  dependencies: []
  conflicts: []
install:
  method: mise
  mise:
    tools: []
validate:
  commands: []
"#,
            name = name,
            version = version
        );

        fs::write(ext_dir.join("extension.yaml"), extension_yaml).unwrap();
    }

    /// Create a versioned extension structure (downloaded format)
    fn create_versioned_extension(dir: &std::path::Path, name: &str, version: &str) {
        let ext_dir = dir.join(name).join(version);
        fs::create_dir_all(&ext_dir).unwrap();

        let extension_yaml = format!(
            r#"
metadata:
  name: {name}
  version: {version}
  description: Test extension (versioned)
  category: testing
  dependencies: []
  conflicts: []
install:
  method: mise
  mise:
    tools: []
validate:
  commands: []
"#,
            name = name,
            version = version
        );

        fs::write(ext_dir.join("extension.yaml"), extension_yaml).unwrap();
    }

    #[test]
    fn test_bundled_source_availability() {
        let temp_dir = TempDir::new().unwrap();
        create_test_extension(temp_dir.path(), "nodejs", "1.0.0");

        let source = BundledSource::new(temp_dir.path());

        assert!(source.is_available("nodejs"));
        assert!(!source.is_available("nonexistent"));
    }

    #[test]
    fn test_bundled_source_extension_path() {
        let temp_dir = TempDir::new().unwrap();
        create_test_extension(temp_dir.path(), "python", "2.0.0");

        let source = BundledSource::new(temp_dir.path());

        let path = source.extension_path("python");
        assert!(path.is_some());
        assert!(path.unwrap().ends_with("python/extension.yaml"));

        assert!(source.extension_path("nonexistent").is_none());
    }

    #[test]
    fn test_bundled_source_get_extension() {
        let temp_dir = TempDir::new().unwrap();
        create_test_extension(temp_dir.path(), "rust", "1.5.0");

        let source = BundledSource::new(temp_dir.path());

        let extension = source.get_extension("rust").unwrap();
        assert_eq!(extension.metadata.name, "rust");
        assert_eq!(extension.metadata.version, "1.5.0");
    }

    #[test]
    fn test_bundled_source_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let source = BundledSource::new(temp_dir.path());

        let result = source.get_extension("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_downloaded_source_versioned_structure() {
        let temp_dir = TempDir::new().unwrap();
        let ext_dir = temp_dir.path().join("extensions");
        fs::create_dir_all(&ext_dir).unwrap();

        create_versioned_extension(&ext_dir, "golang", "1.2.0");
        create_versioned_extension(&ext_dir, "golang", "1.3.0");

        let source = DownloadedSource::new(
            ext_dir,
            temp_dir.path().join("cache"),
            Version::new(3, 0, 0),
        );

        assert!(source.is_available("golang"));

        // Should find a version
        let version_dir = source.find_version_dir("golang").unwrap();
        assert!(version_dir.to_string_lossy().contains("golang"));
    }

    #[test]
    fn test_downloaded_source_get_extension() {
        let temp_dir = TempDir::new().unwrap();
        let ext_dir = temp_dir.path().join("extensions");
        fs::create_dir_all(&ext_dir).unwrap();

        create_versioned_extension(&ext_dir, "docker", "2.0.0");

        let source = DownloadedSource::new(
            ext_dir,
            temp_dir.path().join("cache"),
            Version::new(3, 0, 0),
        );

        let extension = source.get_extension("docker").unwrap();
        assert_eq!(extension.metadata.name, "docker");
        assert_eq!(extension.metadata.version, "2.0.0");
    }

    #[test]
    fn test_local_dev_source_availability() {
        let temp_dir = TempDir::new().unwrap();
        create_test_extension(temp_dir.path(), "devtool", "0.1.0");

        let source = LocalDevSource::new(temp_dir.path());

        assert!(source.is_available("devtool"));
        assert!(!source.is_available("missing"));
    }

    #[test]
    fn test_local_dev_source_get_extension() {
        let temp_dir = TempDir::new().unwrap();
        create_test_extension(temp_dir.path(), "myext", "3.0.0");

        let source = LocalDevSource::new(temp_dir.path());

        let extension = source.get_extension("myext").unwrap();
        assert_eq!(extension.metadata.name, "myext");
        assert_eq!(extension.metadata.version, "3.0.0");
    }

    #[test]
    fn test_resolver_priority_bundled_first() {
        let bundled_dir = TempDir::new().unwrap();
        let downloaded_dir = TempDir::new().unwrap();

        // Create same extension in both with different versions
        create_test_extension(bundled_dir.path(), "shared", "1.0.0");

        let ext_dir = downloaded_dir.path().join("extensions");
        fs::create_dir_all(&ext_dir).unwrap();
        create_versioned_extension(&ext_dir, "shared", "2.0.0");

        let resolver = ExtensionSourceResolver::new(
            Some(BundledSource::new(bundled_dir.path())),
            None,
            DownloadedSource::new(
                ext_dir,
                downloaded_dir.path().join("cache"),
                Version::new(3, 0, 0),
            ),
        );

        // Should find from bundled source first
        let source_type = resolver.find_source("shared");
        assert_eq!(source_type, Some(SourceType::Bundled));

        // Should load bundled version
        let extension = resolver.get_extension_local("shared").unwrap();
        assert_eq!(extension.metadata.version, "1.0.0");
    }

    #[test]
    fn test_resolver_fallback_to_downloaded() {
        let bundled_dir = TempDir::new().unwrap();
        let downloaded_dir = TempDir::new().unwrap();

        // Only create in downloaded source
        let ext_dir = downloaded_dir.path().join("extensions");
        fs::create_dir_all(&ext_dir).unwrap();
        create_versioned_extension(&ext_dir, "onlydownloaded", "5.0.0");

        let resolver = ExtensionSourceResolver::new(
            Some(BundledSource::new(bundled_dir.path())),
            None,
            DownloadedSource::new(
                ext_dir,
                downloaded_dir.path().join("cache"),
                Version::new(3, 0, 0),
            ),
        );

        // Should find from downloaded source
        let source_type = resolver.find_source("onlydownloaded");
        assert_eq!(source_type, Some(SourceType::Downloaded));

        // Should load downloaded version
        let extension = resolver.get_extension_local("onlydownloaded").unwrap();
        assert_eq!(extension.metadata.version, "5.0.0");
    }

    #[test]
    fn test_resolver_local_dev_priority() {
        let local_dev_dir = TempDir::new().unwrap();
        let downloaded_dir = TempDir::new().unwrap();

        // Create in local dev with older version
        create_test_extension(local_dev_dir.path(), "devext", "0.1.0");

        // Create in downloaded with newer version
        let ext_dir = downloaded_dir.path().join("extensions");
        fs::create_dir_all(&ext_dir).unwrap();
        create_versioned_extension(&ext_dir, "devext", "1.0.0");

        let resolver = ExtensionSourceResolver::new(
            None,
            Some(LocalDevSource::new(local_dev_dir.path())),
            DownloadedSource::new(
                ext_dir,
                downloaded_dir.path().join("cache"),
                Version::new(3, 0, 0),
            ),
        );

        // Should prefer local dev
        let source_type = resolver.find_source("devext");
        assert_eq!(source_type, Some(SourceType::LocalDev));

        // Should load local dev version
        let extension = resolver.get_extension_local("devext").unwrap();
        assert_eq!(extension.metadata.version, "0.1.0");
    }

    #[test]
    fn test_resolver_mode_detection() {
        let bundled_dir = TempDir::new().unwrap();
        let local_dev_dir = TempDir::new().unwrap();
        let downloaded_dir = TempDir::new().unwrap();

        let ext_dir = downloaded_dir.path().join("extensions");
        fs::create_dir_all(&ext_dir).unwrap();

        // With bundled
        let resolver_bundled = ExtensionSourceResolver::new(
            Some(BundledSource::new(bundled_dir.path())),
            None,
            DownloadedSource::new(
                ext_dir.clone(),
                downloaded_dir.path().join("cache"),
                Version::new(3, 0, 0),
            ),
        );
        assert!(resolver_bundled.is_bundled_mode());
        assert!(!resolver_bundled.is_dev_mode());

        // With local dev
        let resolver_dev = ExtensionSourceResolver::new(
            None,
            Some(LocalDevSource::new(local_dev_dir.path())),
            DownloadedSource::new(
                ext_dir.clone(),
                downloaded_dir.path().join("cache"),
                Version::new(3, 0, 0),
            ),
        );
        assert!(!resolver_dev.is_bundled_mode());
        assert!(resolver_dev.is_dev_mode());

        // Neither (production download mode)
        let resolver_prod = ExtensionSourceResolver::new(
            None,
            None,
            DownloadedSource::new(
                ext_dir,
                downloaded_dir.path().join("cache"),
                Version::new(3, 0, 0),
            ),
        );
        assert!(!resolver_prod.is_bundled_mode());
        assert!(!resolver_prod.is_dev_mode());
    }

    #[test]
    fn test_source_type_display() {
        assert_eq!(format!("{}", SourceType::Bundled), "bundled");
        assert_eq!(format!("{}", SourceType::Downloaded), "downloaded");
        assert_eq!(format!("{}", SourceType::LocalDev), "local-dev");
    }

    #[test]
    fn test_resolver_not_found() {
        let downloaded_dir = TempDir::new().unwrap();
        let ext_dir = downloaded_dir.path().join("extensions");
        fs::create_dir_all(&ext_dir).unwrap();

        let resolver = ExtensionSourceResolver::new(
            None,
            None,
            DownloadedSource::new(
                ext_dir,
                downloaded_dir.path().join("cache"),
                Version::new(3, 0, 0),
            ),
        );

        assert!(!resolver.is_available_locally("nonexistent"));
        assert!(resolver.find_source("nonexistent").is_none());

        let result = resolver.get_extension_local("nonexistent");
        assert!(result.is_err());
    }
}
