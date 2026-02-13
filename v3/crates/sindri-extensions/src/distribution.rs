//! Extension distribution system for GitHub-based extension delivery
//!
//! This module handles:
//! - Extension distribution from raw.githubusercontent.com using CLI version tags
//! - Version compatibility checking against CLI version
//! - Downloading extension files (not archives) directly
//! - Extension installation, upgrade, and rollback
//! - Version tracking via StatusLedger (event-driven ledger)
//!
//! ## URL Derivation
//!
//! Extensions are fetched from raw.githubusercontent.com using the CLI version tag:
//! ```text
//! https://raw.githubusercontent.com/{owner}/{repo}/{tag}/{base_path}/{name}/extension.yaml
//! ```
//!
//! For example, with CLI version v3.0.0-alpha.5:
//! ```text
//! https://raw.githubusercontent.com/pacphi/sindri/v3.0.0-alpha.5/v3/extensions/nodejs/extension.yaml
//! ```

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sindri_core::config::HierarchicalConfigLoader;
use sindri_core::schema::SchemaValidator;
use sindri_core::types::Extension;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs;
use tracing::{debug, info, warn};

/// Cache TTL for compatibility matrix and registry
const CACHE_TTL: Duration = Duration::from_secs(3600); // 1 hour

/// Extension source configuration loaded from extension-source.yaml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionSourceConfig {
    /// GitHub repository configuration
    pub github: GitHubSourceConfig,
}

/// GitHub source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubSourceConfig {
    /// Repository owner (e.g., "pacphi")
    pub owner: String,

    /// Repository name (e.g., "sindri")
    pub repo: String,

    /// Base path within the repository (e.g., "v3/extensions")
    pub base_path: String,
}

impl ExtensionSourceConfig {
    /// Load configuration from file or use defaults
    ///
    /// Priority:
    /// 1. Bundled at /opt/sindri/extension-source.yaml (Docker dev)
    /// 2. User home at ~/.sindri/extension-source.yaml
    /// 3. Default configuration
    pub fn load() -> Result<Self> {
        // Priority 1: Check bundled location (Docker builds)
        if let Ok(ext_home) = std::env::var("SINDRI_EXT_HOME") {
            let bundled_path = std::path::PathBuf::from(&ext_home)
                .parent()
                .unwrap_or_else(|| std::path::Path::new("/opt/sindri"))
                .join("extension-source.yaml");

            if bundled_path.exists() {
                debug!(
                    "Loading extension source config from bundled path: {:?}",
                    bundled_path
                );
                let content = std::fs::read_to_string(&bundled_path).context(format!(
                    "Failed to read extension source config from {}",
                    bundled_path.display()
                ))?;
                return serde_yaml_ng::from_str(&content)
                    .context("Failed to parse extension source config");
            }
        }

        // Priority 2: Check user home directory
        if let Ok(home) = sindri_core::get_home_dir() {
            let user_path = home.join(".sindri/extension-source.yaml");
            if user_path.exists() {
                debug!(
                    "Loading extension source config from user path: {:?}",
                    user_path
                );
                let content = std::fs::read_to_string(&user_path).context(format!(
                    "Failed to read extension source config from {}",
                    user_path.display()
                ))?;
                return serde_yaml_ng::from_str(&content)
                    .context("Failed to parse extension source config");
            }
        }

        // Priority 3: Use defaults
        debug!("Using default extension source configuration");
        Ok(Self::default())
    }

    /// Build raw.githubusercontent.com URL for a file
    ///
    /// # Arguments
    /// * `tag` - Git tag (e.g., "v3.0.0-alpha.5")
    /// * `name` - Extension name (e.g., "nodejs")
    /// * `file` - File name (e.g., "extension.yaml")
    pub fn build_url(&self, tag: &str, name: &str, file: &str) -> String {
        format!(
            "https://raw.githubusercontent.com/{}/{}/{}/{}/{}/{}",
            self.github.owner, self.github.repo, tag, self.github.base_path, name, file
        )
    }

    /// Build URL for a file at a specific path (not under extensions)
    ///
    /// # Arguments
    /// * `tag` - Git tag (e.g., "v3.0.0-alpha.5")
    /// * `path` - Path within repo (e.g., "v3/compatibility-matrix.yaml")
    pub fn build_repo_url(&self, tag: &str, path: &str) -> String {
        format!(
            "https://raw.githubusercontent.com/{}/{}/{}/{}",
            self.github.owner, self.github.repo, tag, path
        )
    }

    /// Get the repository identifier (owner/repo)
    pub fn repo_identifier(&self) -> String {
        format!("{}/{}", self.github.owner, self.github.repo)
    }
}

impl Default for ExtensionSourceConfig {
    fn default() -> Self {
        Self {
            github: GitHubSourceConfig {
                owner: "pacphi".to_string(),
                repo: "sindri".to_string(),
                base_path: "v3/extensions".to_string(),
            },
        }
    }
}

/// Compatibility matrix defining CLI version to extension version mappings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityMatrix {
    /// Schema version
    pub schema_version: String,

    /// CLI version compatibility mappings
    pub cli_versions: HashMap<String, CliVersionCompat>,
}

/// CLI version compatibility information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliVersionCompat {
    /// Extension schema version required
    pub extension_schema: String,

    /// Compatible extension version requirements
    pub compatible_extensions: HashMap<String, String>,

    /// Breaking changes in this CLI version
    #[serde(default)]
    pub breaking_changes: Vec<String>,
}

/// Verify the integrity of downloaded content against an expected SHA-256 hash.
///
/// If `expected_hash` is `Some`, the content is hashed and compared. A mismatch
/// causes an error. If `expected_hash` is `None`, a warning is logged but the
/// operation succeeds (graceful degradation for extensions without checksums yet).
///
/// # Arguments
/// * `content` - The downloaded content bytes
/// * `extension_name` - Name of the extension (for log messages)
/// * `expected_hash` - Optional expected SHA-256 hex digest
pub fn verify_content_integrity(
    content: &[u8],
    extension_name: &str,
    expected_hash: Option<&str>,
) -> Result<()> {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let actual_hash = format!("{:x}", hasher.finalize());

    match expected_hash {
        Some(expected) => {
            if actual_hash != expected.to_lowercase() {
                return Err(anyhow!(
                    "Integrity check failed for extension '{}': expected SHA-256 {}, got {}",
                    extension_name,
                    expected,
                    actual_hash
                ));
            }
            debug!(
                "Extension '{}' passed integrity check (SHA-256: {})",
                extension_name, actual_hash
            );
        }
        None => {
            warn!(
                "No checksum available for extension '{}'. \
                 Add a sha256 field to registry.yaml for supply-chain security.",
                extension_name
            );
        }
    }

    Ok(())
}

/// Extension distributor for managing GitHub-based extension distribution
pub struct ExtensionDistributor {
    /// Cache directory for downloaded files
    cache_dir: PathBuf,

    /// Extensions directory for extracted extensions
    extensions_dir: PathBuf,

    /// Current CLI version
    cli_version: Version,

    /// Extension source configuration
    source_config: ExtensionSourceConfig,
}

impl ExtensionDistributor {
    /// Create a new extension distributor
    ///
    /// # Arguments
    /// * `cache_dir` - Directory for caching downloads and metadata
    /// * `extensions_dir` - Directory for installed extensions
    /// * `cli_version` - Current CLI version
    pub fn new(cache_dir: PathBuf, extensions_dir: PathBuf, cli_version: Version) -> Result<Self> {
        let source_config = ExtensionSourceConfig::load()?;

        Ok(Self {
            cache_dir,
            extensions_dir,
            cli_version,
            source_config,
        })
    }

    /// Get the CLI version tag for fetching extensions
    ///
    /// Returns the tag format: "v{major}.{minor}.{patch}[-prerelease]"
    pub fn get_cli_tag(&self) -> String {
        format!("v{}", self.cli_version)
    }

    /// Install an extension
    ///
    /// # Arguments
    /// * `name` - Extension name
    /// * `version` - Optional specific version (defaults to latest compatible)
    ///
    /// In bundled mode (SINDRI_EXT_HOME set), installs from bundled extensions.
    /// Otherwise, downloads from raw.githubusercontent.com using CLI version tag.
    pub async fn install(
        &self,
        name: &str,
        version: Option<&str>,
    ) -> Result<(String, Option<String>)> {
        info!("Installing extension: {}", name);

        // 1. Fetch compatibility matrix
        let matrix = self
            .get_compatibility_matrix()
            .await
            .context("Failed to fetch compatibility matrix")?;

        // 2. Get compatible version range for this CLI
        let version_req = self
            .get_compatible_range(&matrix, name)
            .context("Failed to determine compatible version range")?;

        // 3. Determine target version
        let target_version = match version {
            Some(v) => {
                let ver = Version::parse(v).context(format!("Invalid version string: {}", v))?;
                if !version_req.matches(&ver) {
                    return Err(anyhow!(
                        "Version {} is not compatible with CLI {}. Compatible range: {}",
                        v,
                        self.cli_version,
                        version_req
                    ));
                }
                ver
            }
            None => {
                // In bundled mode, determine version from bundled extension
                if let Some(bundled_version) = self.get_bundled_extension_version(name).await? {
                    if version_req.matches(&bundled_version) {
                        debug!(
                            "Using bundled extension {} version {}",
                            name, bundled_version
                        );
                        bundled_version
                    } else {
                        return Err(anyhow!(
                            "Bundled extension {} version {} is not compatible with CLI {}. Compatible range: {}",
                            name,
                            bundled_version,
                            self.cli_version,
                            version_req
                        ));
                    }
                } else {
                    // Not bundled, find compatible version from GitHub
                    self.find_compatible_extension(name, &version_req)
                        .await
                        .context("Failed to find compatible extension version")?
                }
            }
        };

        info!(
            "Installing {} version {} (compatible with CLI {})",
            name, target_version, self.cli_version
        );

        // 4. Check if already installed
        if self.is_installed(name, &target_version)? {
            info!("{} {} is already installed", name, target_version);
            return Ok((target_version.to_string(), None));
        }

        // 5. Get extension directory (bundled or downloaded)
        let ext_dir = if let Some(bundled_dir) = self.get_bundled_extension_dir(name).await? {
            info!("Using bundled extension from {:?}", bundled_dir);
            bundled_dir
        } else {
            // Download extension files from raw.githubusercontent.com
            // Pass None for checksum - callers with registry access should use
            // install_with_registry() for checksum verification
            self.download_extension_files(name, &target_version, None)
                .await
                .context("Failed to download extension")?
        };

        // 6. Load and validate extension definition
        let extension = self
            .load_extension(&ext_dir)
            .context("Failed to load extension definition")?;
        self.validate_extension(&extension)
            .context("Extension validation failed")?;

        // 7. Resolve and install dependencies
        for dep in extension
            .metadata
            .dependencies
            .iter()
            .filter(|d| !d.is_empty())
        {
            if !self.is_any_version_installed(dep)? {
                info!("Installing dependency: {}", dep);
                Box::pin(self.install(dep, None)).await.map(|_| ())?;
            }
        }

        // 8. Execute installation using ExtensionExecutor
        info!("Executing installation for {} {}", name, target_version);
        // Prefer HOME env var for Docker compatibility (ALT_HOME volume mount)
        let home_dir = sindri_core::get_home_dir()?;
        let workspace_dir =
            std::env::current_dir().context("Could not determine current directory")?;

        // Load runtime config for mise timeout
        let config_loader =
            HierarchicalConfigLoader::new().context("Failed to create config loader")?;
        let runtime_config = config_loader
            .load_runtime_config()
            .context("Failed to load runtime config")?;

        let executor = crate::executor::ExtensionExecutor::new(&ext_dir, workspace_dir, home_dir)
            .with_timeout(runtime_config.network.mise_timeout_secs);

        let (install_output, install_result) = executor.install(&extension).await;

        // Write log file before checking result (ensures logs exist even on failure)
        let log_file = self.write_install_log(name, &install_output);

        install_result.context(format!("Failed to execute installation for {}", name))?;

        // 9. Validate installation
        let validation_result = executor
            .validate_extension(&extension)
            .await
            .context("Failed to validate installation")?;

        if !validation_result {
            return Err(anyhow!(
                "Extension {} failed post-installation validation",
                name
            ));
        }

        // 10. Event publishing is handled by the CLI layer
        info!("Successfully installed {} {}", name, target_version);
        Ok((target_version.to_string(), log_file))
    }

    /// Download extension metadata without executing installation
    ///
    /// This is used by listing operations that need extension metadata
    /// but should not execute installation scripts or modify system state.
    ///
    /// # Arguments
    /// * `name` - Extension name
    /// * `version` - Optional specific version (defaults to latest compatible)
    ///
    /// # Returns
    /// The downloaded extension definition
    pub async fn download_metadata(&self, name: &str, version: Option<&str>) -> Result<Extension> {
        debug!("Downloading metadata for extension: {}", name);

        // 1. Fetch compatibility matrix
        let matrix = self
            .get_compatibility_matrix()
            .await
            .context("Failed to fetch compatibility matrix")?;

        // 2. Get compatible version range for this CLI
        let version_req = self
            .get_compatible_range(&matrix, name)
            .context("Failed to determine compatible version range")?;

        // 3. Determine target version
        let target_version = match version {
            Some(v) => {
                let ver = Version::parse(v).context(format!("Invalid version string: {}", v))?;
                if !version_req.matches(&ver) {
                    return Err(anyhow!(
                        "Version {} is not compatible with CLI {}. Compatible range: {}",
                        v,
                        self.cli_version,
                        version_req
                    ));
                }
                ver
            }
            None => {
                // In bundled mode, determine version from bundled extension
                if let Some(bundled_version) = self.get_bundled_extension_version(name).await? {
                    if version_req.matches(&bundled_version) {
                        debug!(
                            "Using bundled extension {} version {}",
                            name, bundled_version
                        );
                        bundled_version
                    } else {
                        return Err(anyhow!(
                            "Bundled extension {} version {} is not compatible with CLI {}. Compatible range: {}",
                            name,
                            bundled_version,
                            self.cli_version,
                            version_req
                        ));
                    }
                } else {
                    // Not bundled, find compatible version from GitHub
                    self.find_compatible_extension(name, &version_req)
                        .await
                        .context("Failed to find compatible extension version")?
                }
            }
        };

        debug!(
            "Fetching {} version {} metadata (compatible with CLI {})",
            name, target_version, self.cli_version
        );

        // 4. Get extension directory (bundled or downloaded)
        let ext_dir = if let Some(bundled_dir) = self.get_bundled_extension_dir(name).await? {
            debug!("Using bundled extension from {:?}", bundled_dir);
            bundled_dir
        } else {
            // Download extension files from raw.githubusercontent.com
            self.download_extension_files(name, &target_version, None)
                .await
                .context("Failed to download extension metadata")?
        };

        // 5. Load extension definition (no validation, no execution)
        let extension = self
            .load_extension(&ext_dir)
            .context("Failed to load extension definition")?;

        info!(
            "Downloaded extension {} v{} to {:?}",
            name, target_version, ext_dir
        );

        Ok(extension)
    }

    /// Write installation output to a per-extension log file
    ///
    /// Returns the log file path as a String, or None if writing failed.
    fn write_install_log(
        &self,
        name: &str,
        output: &crate::executor::InstallOutput,
    ) -> Option<String> {
        match crate::log_files::ExtensionLogWriter::new_default() {
            Ok(writer) => match writer.write_log(name, chrono::Utc::now(), output) {
                Ok(path) => Some(path.to_string_lossy().to_string()),
                Err(e) => {
                    warn!("Failed to write install log for {}: {}", name, e);
                    None
                }
            },
            Err(e) => {
                warn!("Failed to create log writer: {}", e);
                None
            }
        }
    }

    /// Upgrade an extension to the latest compatible version
    ///
    /// # Arguments
    /// * `name` - Extension name
    pub async fn upgrade(&self, name: &str) -> Result<()> {
        info!("Upgrading extension: {}", name);

        // 1. Get current installed version
        let current = self
            .get_installed_version(name)?
            .ok_or_else(|| anyhow!("{} is not installed", name))?;

        // 2. Get compatibility matrix
        let matrix = self
            .get_compatibility_matrix()
            .await
            .context("Failed to fetch compatibility matrix")?;

        // 3. Get compatible version range
        let version_req = self
            .get_compatible_range(&matrix, name)
            .context("Failed to determine compatible version range")?;

        // 4. Find compatible version from extension.yaml
        let latest = self
            .find_compatible_extension(name, &version_req)
            .await
            .context("Failed to find compatible extension version")?;

        if latest <= current {
            info!(
                "{} {} is already the latest compatible version",
                name, current
            );
            return Ok(());
        }

        info!("Upgrading {} {} -> {}", name, current, latest);

        // 5. Install new version (keeps old version for rollback)
        self.install(name, Some(&latest.to_string()))
            .await
            .map(|_| ())?;

        info!("Successfully upgraded {} to {}", name, latest);
        Ok(())
    }

    /// Rollback an extension to the previous version
    ///
    /// # Arguments
    /// * `name` - Extension name
    pub async fn rollback(&self, name: &str) -> Result<()> {
        info!("Rolling back extension: {}", name);

        let versions = self.get_installed_versions(name)?;
        if versions.len() < 2 {
            return Err(anyhow!("No previous version available for rollback"));
        }

        let current = &versions[0];
        let previous = &versions[1];

        info!("Rolling back {} {} -> {}", name, current, previous);

        // Event publishing is handled by the CLI layer
        info!("Successfully rolled back {} to {}", name, previous);
        Ok(())
    }

    /// Get bundled extension directory if available
    ///
    /// Returns Some(PathBuf) if the extension exists in SINDRI_EXT_HOME AND
    /// SINDRI_EXT_HOME points to a bundled location (/opt/sindri), None otherwise.
    ///
    /// This prevents treating user's download directory (~/.sindri/extensions)
    /// as a bundled source, which would cause path resolution issues.
    async fn get_bundled_extension_dir(&self, name: &str) -> Result<Option<PathBuf>> {
        if let Ok(ext_home) = std::env::var("SINDRI_EXT_HOME") {
            let ext_home_path = std::path::PathBuf::from(&ext_home);

            // Only treat as bundled if it's under /opt/sindri (not user's home directory)
            // This matches the check in BundledSource::from_env() in source.rs
            if !ext_home_path.starts_with("/opt/sindri") {
                debug!(
                    "SINDRI_EXT_HOME={:?} is not a bundled path, skipping bundled check",
                    ext_home
                );
                return Ok(None);
            }

            let bundled_ext_dir = ext_home_path.join(name);

            if bundled_ext_dir.exists() && bundled_ext_dir.join("extension.yaml").exists() {
                debug!("Found bundled extension at {:?}", bundled_ext_dir);
                return Ok(Some(bundled_ext_dir));
            }
        }
        Ok(None)
    }

    /// Get bundled extension version if available
    ///
    /// Returns Some(Version) if the extension exists in SINDRI_EXT_HOME, None otherwise.
    async fn get_bundled_extension_version(&self, name: &str) -> Result<Option<Version>> {
        if let Some(bundled_dir) = self.get_bundled_extension_dir(name).await? {
            let extension = self
                .load_extension(&bundled_dir)
                .context("Failed to load bundled extension")?;
            let version = Version::parse(&extension.metadata.version).context(format!(
                "Invalid version in bundled extension: {}",
                extension.metadata.version
            ))?;
            return Ok(Some(version));
        }
        Ok(None)
    }

    /// Get the compatibility matrix from cache or GitHub
    ///
    /// In bundled mode (SINDRI_EXT_HOME set), loads from /opt/sindri/compatibility-matrix.yaml.
    /// Otherwise, fetches from GitHub with local caching.
    pub async fn get_compatibility_matrix(&self) -> Result<CompatibilityMatrix> {
        // Check for bundled mode (build-from-source with extensions at /opt/sindri)
        if let Ok(ext_home) = std::env::var("SINDRI_EXT_HOME") {
            let bundled_path = std::path::PathBuf::from(&ext_home)
                .parent()
                .unwrap_or_else(|| std::path::Path::new("/opt/sindri"))
                .join("compatibility-matrix.yaml");

            if bundled_path.exists() {
                debug!("Using bundled compatibility matrix from {:?}", bundled_path);
                let content = fs::read_to_string(&bundled_path).await.context(format!(
                    "Failed to read bundled compatibility matrix at {}",
                    bundled_path.display()
                ))?;
                return serde_yaml_ng::from_str(&content)
                    .context("Failed to parse bundled compatibility matrix");
            } else {
                debug!(
                    "Bundled compatibility matrix not found at {:?}, falling back to cache/GitHub",
                    bundled_path
                );
            }
        }

        let cache_path = self.cache_dir.join("compatibility-matrix.yaml");

        // Check cache
        if let Ok(metadata) = fs::metadata(&cache_path).await {
            if let Ok(modified) = metadata.modified() {
                if modified.elapsed().unwrap_or(Duration::MAX) < CACHE_TTL {
                    debug!("Using cached compatibility matrix");
                    let content = fs::read_to_string(&cache_path)
                        .await
                        .context("Failed to read cached compatibility matrix")?;
                    return serde_yaml_ng::from_str(&content)
                        .context("Failed to parse cached compatibility matrix");
                }
            }
        }

        debug!("Fetching compatibility matrix from GitHub");

        // Fetch from GitHub using CLI version tag
        let tag = self.get_cli_tag();
        let url = self
            .source_config
            .build_repo_url(&tag, "v3/compatibility-matrix.yaml");

        let client = reqwest::Client::new();
        let response = client.get(&url).send().await;

        let content = match response {
            Ok(resp) if resp.status().is_success() => resp
                .text()
                .await
                .context("Failed to read compatibility matrix response")?,
            Ok(resp) => {
                // If the tag doesn't exist, fall back to main branch
                warn!(
                    "CLI tag {} not found (HTTP {}), falling back to main branch",
                    tag,
                    resp.status()
                );
                let fallback_url = self
                    .source_config
                    .build_repo_url("main", "v3/compatibility-matrix.yaml");
                client
                    .get(&fallback_url)
                    .send()
                    .await
                    .context("Failed to fetch compatibility matrix from GitHub (main branch)")?
                    .text()
                    .await
                    .context("Failed to read compatibility matrix response")?
            }
            Err(e) => {
                return Err(anyhow!(
                    "Failed to fetch compatibility matrix from GitHub: {}",
                    e
                ));
            }
        };

        // Cache it
        fs::create_dir_all(&self.cache_dir)
            .await
            .context("Failed to create cache directory")?;
        fs::write(&cache_path, &content)
            .await
            .context("Failed to write compatibility matrix to cache")?;

        serde_yaml_ng::from_str(&content).context("Failed to parse compatibility matrix")
    }

    /// Get the compatible version range for an extension
    pub fn get_compatible_range(
        &self,
        matrix: &CompatibilityMatrix,
        name: &str,
    ) -> Result<VersionReq> {
        // Find matching CLI version pattern (3.0.x, 3.1.x, etc.)
        let cli_pattern = format!("{}.{}.x", self.cli_version.major, self.cli_version.minor);

        let compat = matrix.cli_versions.get(&cli_pattern).ok_or_else(|| {
            anyhow!(
                "CLI version {} not found in compatibility matrix",
                self.cli_version
            )
        })?;

        let range_str = compat.compatible_extensions.get(name).ok_or_else(|| {
            anyhow!(
                "Extension {} not found in compatibility matrix for CLI {}",
                name,
                cli_pattern
            )
        })?;

        VersionReq::parse(range_str).context(format!("Invalid version requirement: {}", range_str))
    }

    /// Find the latest compatible version for an extension
    ///
    /// This is an alias for `find_compatible_extension` for backwards compatibility.
    ///
    /// # Arguments
    /// * `name` - Extension name
    /// * `req` - Version requirement to check compatibility
    pub async fn find_latest_compatible(&self, name: &str, req: &VersionReq) -> Result<Version> {
        self.find_compatible_extension(name, req).await
    }

    /// Find a compatible extension version by fetching extension.yaml from GitHub
    ///
    /// Unlike the old releases-based approach, this method fetches the extension.yaml
    /// directly using the CLI version tag and reads the version from the extension metadata.
    ///
    /// # Arguments
    /// * `name` - Extension name
    /// * `req` - Version requirement to check compatibility
    pub async fn find_compatible_extension(&self, name: &str, req: &VersionReq) -> Result<Version> {
        debug!(
            "Finding compatible extension version for {} (CLI tag: {})",
            name,
            self.get_cli_tag()
        );

        // Fetch extension.yaml using CLI version tag
        let tag = self.get_cli_tag();
        let url = self.source_config.build_url(&tag, name, "extension.yaml");

        let client = reqwest::Client::new();
        let response = client.get(&url).send().await;

        let content = match response {
            Ok(resp) if resp.status().is_success() => resp
                .text()
                .await
                .context("Failed to read extension.yaml response")?,
            Ok(resp) if resp.status() == reqwest::StatusCode::NOT_FOUND => {
                // If the tag doesn't exist, try the main branch
                warn!(
                    "Extension {} not found at tag {}, trying main branch",
                    name, tag
                );
                let fallback_url = self.source_config.build_url("main", name, "extension.yaml");
                let fallback_resp = client
                    .get(&fallback_url)
                    .send()
                    .await
                    .context("Failed to fetch extension from main branch")?;

                if !fallback_resp.status().is_success() {
                    return Err(anyhow!(
                        "Extension '{}' not found in repository (tried tag {} and main branch)",
                        name,
                        tag
                    ));
                }

                fallback_resp
                    .text()
                    .await
                    .context("Failed to read extension.yaml response")?
            }
            Ok(resp) => {
                return Err(anyhow!(
                    "Failed to fetch extension '{}': HTTP {}",
                    name,
                    resp.status()
                ));
            }
            Err(e) => {
                return Err(anyhow!(
                    "Failed to fetch extension '{}' from GitHub: {}",
                    name,
                    e
                ));
            }
        };

        // Parse the extension.yaml to get the version
        let extension: Extension =
            serde_yaml_ng::from_str(&content).context("Failed to parse extension.yaml")?;

        let version = Version::parse(&extension.metadata.version).context(format!(
            "Invalid version in extension: {}",
            extension.metadata.version
        ))?;

        // Verify the version is compatible
        if !req.matches(&version) {
            return Err(anyhow!(
                "Extension {} version {} is not compatible with CLI {} (requires {})",
                name,
                version,
                self.cli_version,
                req
            ));
        }

        debug!("Found compatible extension {} version {}", name, version);
        Ok(version)
    }

    /// List available extension versions
    ///
    /// Since we no longer use GitHub releases for per-extension versioning,
    /// this returns the version from the extension.yaml at the current CLI tag.
    ///
    /// # Arguments
    /// * `name` - Extension name
    /// * `compatible_range` - Optional version requirement to check compatibility
    pub async fn list_available_versions(
        &self,
        name: &str,
        compatible_range: Option<&VersionReq>,
    ) -> Result<Vec<(Version, DateTime<Utc>, bool)>> {
        debug!("Listing available versions for {}", name);

        // With the new model, each CLI release has one version of each extension
        // We fetch the extension.yaml to get the version

        let tag = self.get_cli_tag();
        let url = self.source_config.build_url(&tag, name, "extension.yaml");

        let client = reqwest::Client::new();
        let response = client.get(&url).send().await;

        let content = match response {
            Ok(resp) if resp.status().is_success() => resp.text().await?,
            Ok(_) | Err(_) => {
                // Try main branch as fallback
                let fallback_url = self.source_config.build_url("main", name, "extension.yaml");
                let fallback_resp = client.get(&fallback_url).send().await?;
                if !fallback_resp.status().is_success() {
                    return Ok(vec![]); // Extension not found
                }
                fallback_resp.text().await?
            }
        };

        let extension: Extension = serde_yaml_ng::from_str(&content)?;
        let version = Version::parse(&extension.metadata.version)?;

        let is_compatible = compatible_range
            .map(|req| req.matches(&version))
            .unwrap_or(true);

        // Use current time as "published_at" since we don't track release dates
        Ok(vec![(version, Utc::now(), is_compatible)])
    }

    /// Get the previous version of an extension from the ledger history
    ///
    /// Scans ledger events for the most recent version that differs from the current version.
    ///
    /// # Arguments
    /// * `name` - Extension name
    pub fn get_previous_version(&self, name: &str) -> Result<Option<Version>> {
        let ledger = crate::ledger::StatusLedger::load_default()?;

        // Get the current version from ledger
        let current_version = ledger
            .get_all_latest_status()?
            .get(name)
            .and_then(|s| s.version.clone());

        // Scan event history for previous versions
        let history = ledger.get_extension_history(name, None)?;

        // Walk events in reverse (most recent first) to find a version different from current
        for envelope in history.iter().rev() {
            let event_version = match &envelope.event {
                crate::events::ExtensionEvent::InstallCompleted { version, .. } => {
                    Some(version.clone())
                }
                crate::events::ExtensionEvent::UpgradeCompleted { to_version, .. } => {
                    Some(to_version.clone())
                }
                _ => None,
            };

            if let Some(v) = event_version {
                // Skip if this matches the current version
                if current_version.as_deref() == Some(v.as_str()) {
                    continue;
                }
                let version = Version::parse(&v)
                    .context(format!("Invalid previous version in ledger: {}", v))?;
                return Ok(Some(version));
            }
        }

        Ok(None)
    }

    /// Download extension files from raw.githubusercontent.com
    ///
    /// Downloads the extension.yaml and any additional files referenced in it.
    /// Files are saved to ~/.sindri/extensions/{name}/{version}/
    ///
    /// When `expected_sha256` is provided, the downloaded extension.yaml content
    /// is verified against the checksum before saving. If absent, a warning is
    /// logged but the download proceeds.
    ///
    /// # Arguments
    /// * `name` - Extension name
    /// * `version` - Extension version to download
    /// * `expected_sha256` - Optional SHA-256 checksum from the registry
    async fn download_extension_files(
        &self,
        name: &str,
        version: &Version,
        expected_sha256: Option<&str>,
    ) -> Result<PathBuf> {
        debug!(
            "Downloading extension {} version {} using CLI tag {}",
            name,
            version,
            self.get_cli_tag()
        );

        let dest = self.extensions_dir.join(name).join(version.to_string());

        // Skip if already exists
        if dest.join("extension.yaml").exists() {
            debug!("Extension already downloaded at {}", dest.display());
            return Ok(dest);
        }

        fs::create_dir_all(&dest)
            .await
            .context("Failed to create extension directory")?;

        let tag = self.get_cli_tag();
        let client = reqwest::Client::new();

        // Download extension.yaml
        let ext_yaml_url = self.source_config.build_url(&tag, name, "extension.yaml");
        let content = self
            .fetch_file_with_fallback(&client, &ext_yaml_url, &tag, name, "extension.yaml")
            .await?;

        // Verify content integrity against registry checksum
        verify_content_integrity(content.as_bytes(), name, expected_sha256)?;

        // Parse to discover additional files
        let extension: Extension =
            serde_yaml_ng::from_str(&content).context("Failed to parse extension.yaml")?;

        // Save extension.yaml
        fs::write(dest.join("extension.yaml"), &content)
            .await
            .context("Failed to write extension.yaml")?;

        // Download additional files if referenced in the extension
        // Check for scripts and other files that might be needed
        self.download_additional_files(&client, &tag, name, &dest, &extension)
            .await?;

        info!(
            "Downloaded extension {} {} to {}",
            name,
            version,
            dest.display()
        );
        Ok(dest)
    }

    /// Fetch a file with fallback to main branch
    async fn fetch_file_with_fallback(
        &self,
        client: &reqwest::Client,
        url: &str,
        tag: &str,
        name: &str,
        file: &str,
    ) -> Result<String> {
        let response = client.get(url).send().await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                resp.text().await.context("Failed to read response body")
            }
            Ok(resp) if resp.status() == reqwest::StatusCode::NOT_FOUND => {
                // Try main branch as fallback
                warn!("File {} not found at tag {}, trying main branch", file, tag);
                let fallback_url = self.source_config.build_url("main", name, file);
                let fallback_resp = client
                    .get(&fallback_url)
                    .send()
                    .await
                    .context("Failed to fetch from main branch")?;

                if !fallback_resp.status().is_success() {
                    return Err(anyhow!(
                        "File '{}' not found for extension '{}' (tried tag {} and main)",
                        file,
                        name,
                        tag
                    ));
                }

                fallback_resp
                    .text()
                    .await
                    .context("Failed to read fallback response")
            }
            Ok(resp) => Err(anyhow!("Failed to fetch '{}': HTTP {}", url, resp.status())),
            Err(e) => Err(anyhow!("Failed to fetch '{}': {}", url, e)),
        }
    }

    /// Download additional files referenced in the extension
    async fn download_additional_files(
        &self,
        client: &reqwest::Client,
        tag: &str,
        name: &str,
        dest: &Path,
        extension: &Extension,
    ) -> Result<()> {
        // Check for mise config files
        if let Some(ref mise) = extension.install.mise {
            let config_file = mise.config_file.as_deref().unwrap_or("mise.toml");
            let file_name = config_file.trim_start_matches("./");
            self.download_optional_file(client, tag, name, dest, file_name)
                .await?;
        }

        // Check for install scripts
        if let Some(ref script) = extension.install.script {
            let file_name = script.path.trim_start_matches("./");
            self.download_optional_file(client, tag, name, dest, file_name)
                .await?;
        }

        // Check for configure templates
        if let Some(ref configure) = extension.configure {
            for template in &configure.templates {
                let source_name = template.source.trim_start_matches("./");
                self.download_optional_file(client, tag, name, dest, source_name)
                    .await?;
            }
        }

        // Check for remove scripts
        if let Some(ref remove) = extension.remove {
            if let Some(ref script) = remove.script {
                if let Some(ref path) = script.path {
                    let file_name = path.trim_start_matches("./");
                    self.download_optional_file(client, tag, name, dest, file_name)
                        .await?;
                }
            }
        }

        // Check for upgrade scripts
        if let Some(ref upgrade) = extension.upgrade {
            if let Some(ref script) = upgrade.script {
                let file_name = script.path.trim_start_matches("./");
                self.download_optional_file(client, tag, name, dest, file_name)
                    .await?;
            }
        }

        Ok(())
    }

    /// Download an optional file (doesn't fail if not found)
    async fn download_optional_file(
        &self,
        client: &reqwest::Client,
        tag: &str,
        name: &str,
        dest: &Path,
        file: &str,
    ) -> Result<()> {
        let url = self.source_config.build_url(tag, name, file);
        debug!("Downloading optional file: {}", url);

        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let content = resp.bytes().await?;

                // Create parent directories if needed
                let file_path = dest.join(file);
                if let Some(parent) = file_path.parent() {
                    fs::create_dir_all(parent).await?;
                }

                fs::write(&file_path, &content).await?;

                // Make scripts executable
                #[cfg(unix)]
                if file.ends_with(".sh") {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = std::fs::metadata(&file_path)?.permissions();
                    perms.set_mode(0o755);
                    std::fs::set_permissions(&file_path, perms)?;
                }

                debug!("Downloaded {}", file);
            }
            Ok(resp) => {
                debug!(
                    "Optional file {} not found (HTTP {}), skipping",
                    file,
                    resp.status()
                );
            }
            Err(e) => {
                debug!("Failed to download optional file {}: {}, skipping", file, e);
            }
        }

        Ok(())
    }

    /// Load an extension definition from a directory
    fn load_extension(&self, ext_dir: &Path) -> Result<Extension> {
        let extension_yaml = ext_dir.join("extension.yaml");
        let content = std::fs::read_to_string(&extension_yaml)
            .context(format!("Failed to read {}", extension_yaml.display()))?;

        serde_yaml_ng::from_str(&content).context("Failed to parse extension.yaml")
    }

    /// Validate an extension definition
    ///
    /// This performs comprehensive validation including:
    /// 1. Structural and semantic validation via ExtensionValidator
    /// 2. Dependency cycle detection
    /// 3. Signature verification warnings (placeholder for future)
    ///
    /// For full dependency graph validation (checking dependencies exist in registry),
    /// use `validate_extension_with_registry` instead.
    fn validate_extension(&self, extension: &Extension) -> Result<()> {
        // 1. Structural and semantic validation using ExtensionValidator
        let schema_validator =
            SchemaValidator::new().context("Failed to create schema validator")?;
        let ext_validator = crate::validator::ExtensionValidator::new(&schema_validator);
        ext_validator
            .validate_extension_struct(extension)
            .context("Extension structural/semantic validation failed")?;

        debug!(
            "Extension {} v{} passed structural validation",
            extension.metadata.name, extension.metadata.version
        );

        // 2. Check for self-dependency (already done by ExtensionValidator, but explicit here)
        if extension
            .metadata
            .dependencies
            .contains(&extension.metadata.name)
        {
            return Err(anyhow!(
                "Extension {} cannot depend on itself",
                extension.metadata.name
            ));
        }

        // 3. Signature verification (placeholder for future implementation)
        // Currently, extensions don't have a signature field in the schema
        // Log a debug message for future reference
        debug!(
            "Signature verification not yet implemented for extension {}",
            extension.metadata.name
        );

        info!(
            "Extension {} v{} passed all validation checks",
            extension.metadata.name, extension.metadata.version
        );

        Ok(())
    }

    /// Validate an extension with full registry context
    ///
    /// This performs all validation from `validate_extension` plus:
    /// - Verifies all dependencies exist in the registry
    /// - Validates there are no circular dependencies in the dependency graph
    ///
    /// # Arguments
    /// * `extension` - The extension to validate
    /// * `registry` - The extension registry for dependency verification
    pub fn validate_extension_with_registry(
        &self,
        extension: &Extension,
        registry: &crate::registry::ExtensionRegistry,
    ) -> Result<()> {
        // First perform basic validation
        self.validate_extension(extension)?;

        // Verify all dependencies exist in the registry
        for dep in &extension.metadata.dependencies {
            if dep.is_empty() {
                continue;
            }
            if !registry.has_extension(dep) {
                return Err(anyhow!(
                    "Extension {} depends on '{}' which is not found in the registry",
                    extension.metadata.name,
                    dep
                ));
            }
        }

        // Use DependencyResolver to check for circular dependencies
        let resolver = crate::dependency::DependencyResolver::new(registry);
        resolver.resolve(&extension.metadata.name).context(format!(
            "Dependency resolution failed for extension {}",
            extension.metadata.name
        ))?;

        debug!(
            "Extension {} passed dependency graph validation",
            extension.metadata.name
        );

        Ok(())
    }

    /// Validate an extension with checksum verification
    ///
    /// This performs all validation from `validate_extension` plus
    /// checksum verification of the extension YAML content.
    ///
    /// # Arguments
    /// * `extension` - The extension to validate
    /// * `yaml_content` - The original YAML content for checksum verification
    /// * `expected_checksum` - The expected SHA256 checksum (hex-encoded)
    pub fn validate_extension_with_checksum(
        &self,
        extension: &Extension,
        yaml_content: &str,
        expected_checksum: &str,
    ) -> Result<()> {
        // First perform basic validation
        self.validate_extension(extension)?;

        // Verify content integrity using the shared helper
        verify_content_integrity(
            yaml_content.as_bytes(),
            &extension.metadata.name,
            Some(expected_checksum),
        )?;

        Ok(())
    }

    /// Check if a specific version of an extension is installed
    fn is_installed(&self, name: &str, version: &Version) -> Result<bool> {
        // Check ledger state first (authoritative source)
        if let Ok(ledger) = crate::ledger::StatusLedger::load_default() {
            if let Ok(status_map) = ledger.get_all_latest_status() {
                if let Some(status) = status_map.get(name) {
                    if status.current_state == sindri_core::types::ExtensionState::Installed {
                        if let Some(v) = &status.version {
                            return Ok(v == &version.to_string());
                        }
                    }
                    return Ok(false);
                }
            }
        }

        // Fallback to filesystem check
        let ext_dir = self.extensions_dir.join(name).join(version.to_string());
        Ok(ext_dir.exists())
    }

    /// Check if any version of an extension is installed
    fn is_any_version_installed(&self, name: &str) -> Result<bool> {
        // Check ledger state first (authoritative source)
        if let Ok(ledger) = crate::ledger::StatusLedger::load_default() {
            if let Ok(status_map) = ledger.get_all_latest_status() {
                if let Some(status) = status_map.get(name) {
                    return Ok(
                        status.current_state == sindri_core::types::ExtensionState::Installed
                    );
                }
            }
        }

        // Fallback to filesystem check
        let ext_dir = self.extensions_dir.join(name);
        if !ext_dir.exists() {
            return Ok(false);
        }

        // Check if there's at least one version directory
        let entries = std::fs::read_dir(&ext_dir)
            .context(format!("Failed to read directory: {}", ext_dir.display()))?;

        Ok(entries.count() > 0)
    }

    /// Get the currently installed version of an extension from the ledger
    fn get_installed_version(&self, name: &str) -> Result<Option<Version>> {
        let ledger = crate::ledger::StatusLedger::load_default()?;
        let version_str = ledger
            .get_all_latest_status()?
            .get(name)
            .and_then(|s| s.version.clone());

        match version_str {
            Some(v) => {
                let version =
                    Version::parse(&v).context(format!("Invalid version in ledger: {}", v))?;
                Ok(Some(version))
            }
            None => Ok(None),
        }
    }

    /// Get all installed versions of an extension (sorted newest first)
    fn get_installed_versions(&self, name: &str) -> Result<Vec<Version>> {
        let ext_dir = self.extensions_dir.join(name);
        if !ext_dir.exists() {
            return Ok(vec![]);
        }

        let entries = std::fs::read_dir(&ext_dir)
            .context(format!("Failed to read directory: {}", ext_dir.display()))?;

        let mut versions: Vec<Version> = entries
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().is_dir())
            .filter_map(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .and_then(|s| Version::parse(s).ok())
            })
            .collect();

        versions.sort_by(|a, b| b.cmp(a)); // Newest first
        Ok(versions)
    }

    /// Get the source configuration
    pub fn source_config(&self) -> &ExtensionSourceConfig {
        &self.source_config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        let version = Version::parse("1.2.3").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
    }

    #[test]
    fn test_version_requirement() {
        let req = VersionReq::parse(">=1.0.0,<2.0.0").unwrap();
        assert!(req.matches(&Version::parse("1.5.0").unwrap()));
        assert!(!req.matches(&Version::parse("2.0.0").unwrap()));
        assert!(!req.matches(&Version::parse("0.9.0").unwrap()));
    }

    #[test]
    fn test_extension_source_config_default() {
        let config = ExtensionSourceConfig::default();
        assert_eq!(config.github.owner, "pacphi");
        assert_eq!(config.github.repo, "sindri");
        assert_eq!(config.github.base_path, "v3/extensions");
    }

    #[test]
    fn test_extension_source_config_build_url() {
        let config = ExtensionSourceConfig::default();
        let url = config.build_url("v3.0.0-alpha.5", "nodejs", "extension.yaml");
        assert_eq!(
            url,
            "https://raw.githubusercontent.com/pacphi/sindri/v3.0.0-alpha.5/v3/extensions/nodejs/extension.yaml"
        );
    }

    #[test]
    fn test_extension_source_config_build_repo_url() {
        let config = ExtensionSourceConfig::default();
        let url = config.build_repo_url("v3.0.0-alpha.5", "v3/compatibility-matrix.yaml");
        assert_eq!(
            url,
            "https://raw.githubusercontent.com/pacphi/sindri/v3.0.0-alpha.5/v3/compatibility-matrix.yaml"
        );
    }

    #[test]
    fn test_extension_source_config_repo_identifier() {
        let config = ExtensionSourceConfig::default();
        assert_eq!(config.repo_identifier(), "pacphi/sindri");
    }

    #[test]
    fn test_cli_tag_format() {
        // Test that CLI version formats correctly as a tag
        let version = Version::parse("3.0.0-alpha.5").unwrap();
        let tag = format!("v{}", version);
        assert_eq!(tag, "v3.0.0-alpha.5");
    }

    #[test]
    fn test_cli_tag_format_stable() {
        let version = Version::parse("3.0.0").unwrap();
        let tag = format!("v{}", version);
        assert_eq!(tag, "v3.0.0");
    }

    #[test]
    fn test_verify_content_integrity_matching_hash() {
        let content = b"hello world";
        // SHA-256 of "hello world"
        let expected = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
        let result = verify_content_integrity(content, "test-ext", Some(expected));
        result.expect("integrity check with matching hash should succeed");
    }

    #[test]
    fn test_verify_content_integrity_mismatched_hash() {
        let content = b"hello world";
        let wrong_hash = "0000000000000000000000000000000000000000000000000000000000000000";
        let result = verify_content_integrity(content, "test-ext", Some(wrong_hash));
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Integrity check failed"));
        assert!(err_msg.contains("test-ext"));
    }

    #[test]
    fn test_verify_content_integrity_no_hash_succeeds() {
        // When no hash is provided, verification should succeed (with a warning)
        let content = b"any content";
        let result = verify_content_integrity(content, "test-ext", None);
        result.expect("integrity check with no expected hash should succeed");
    }

    #[test]
    fn test_verify_content_integrity_case_insensitive() {
        let content = b"hello world";
        // Use uppercase hex
        let expected = "B94D27B9934D3E08A52E52D7DA7DABFAC484EFE37A5380EE9088F7ACE2EFCDE9";
        let result = verify_content_integrity(content, "test-ext", Some(expected));
        result.expect("integrity check with uppercase hex hash should succeed");
    }
}
