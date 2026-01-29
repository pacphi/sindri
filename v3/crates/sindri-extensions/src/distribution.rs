//! Extension distribution system for GitHub-based extension delivery
//!
//! This module handles:
//! - Extension distribution from GitHub releases
//! - Version compatibility checking against CLI version
//! - Downloading and extracting extension archives
//! - Local manifest management
//! - Extension installation, upgrade, and rollback

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use sindri_core::schema::SchemaValidator;
use sindri_core::types::Extension;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::fs;
use tracing::{debug, info};

/// GitHub repository for extensions
const EXTENSIONS_REPO: &str = "sindri/sindri-extensions";

/// Cache TTL for compatibility matrix and registry
const CACHE_TTL: Duration = Duration::from_secs(3600); // 1 hour

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

/// Extension distributor for managing GitHub-based extension distribution
pub struct ExtensionDistributor {
    /// Cache directory for downloaded files
    cache_dir: PathBuf,

    /// Extensions directory for extracted extensions
    extensions_dir: PathBuf,

    /// Current CLI version
    cli_version: Version,

    /// GitHub client
    github_client: Arc<octocrab::Octocrab>,
}

impl ExtensionDistributor {
    /// Create a new extension distributor
    ///
    /// # Arguments
    /// * `cache_dir` - Directory for caching downloads and metadata
    /// * `extensions_dir` - Directory for installed extensions
    /// * `cli_version` - Current CLI version
    pub fn new(cache_dir: PathBuf, extensions_dir: PathBuf, cli_version: Version) -> Result<Self> {
        let github_client = octocrab::instance();

        Ok(Self {
            cache_dir,
            extensions_dir,
            cli_version,
            github_client,
        })
    }

    /// Install an extension
    ///
    /// # Arguments
    /// * `name` - Extension name
    /// * `version` - Optional specific version (defaults to latest compatible)
    ///
    /// In bundled mode (SINDRI_EXT_HOME set), installs from bundled extensions.
    /// Otherwise, downloads from GitHub releases.
    pub async fn install(&self, name: &str, version: Option<&str>) -> Result<()> {
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
                        debug!("Using bundled extension {} version {}", name, bundled_version);
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
                    // Not bundled, find latest compatible from GitHub
                    self.find_latest_compatible(name, &version_req)
                        .await
                        .context("Failed to find latest compatible version")?
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
            return Ok(());
        }

        // 5. Get extension directory (bundled or downloaded)
        let ext_dir = if let Some(bundled_dir) = self.get_bundled_extension_dir(name).await? {
            info!("Using bundled extension from {:?}", bundled_dir);
            bundled_dir
        } else {
            // Download extension archive
            let archive_path = self
                .download_extension(name, &target_version)
                .await
                .context("Failed to download extension")?;

            // Extract to extensions directory
            self.extract_extension(&archive_path, name, &target_version)
                .context("Failed to extract extension")?
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
                Box::pin(self.install(dep, None)).await?;
            }
        }

        // 8. Update manifest
        self.update_manifest(name, &target_version)
            .await
            .context("Failed to update manifest")?;

        info!("Successfully installed {} {}", name, target_version);
        Ok(())
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

        // 4. Find latest compatible version
        let latest = self
            .find_latest_compatible(name, &version_req)
            .await
            .context("Failed to find latest compatible version")?;

        if latest <= current {
            info!(
                "{} {} is already the latest compatible version",
                name, current
            );
            return Ok(());
        }

        info!("Upgrading {} {} -> {}", name, current, latest);

        // 5. Install new version (keeps old version for rollback)
        self.install(name, Some(&latest.to_string())).await?;

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

        // Update manifest to point to previous version
        self.update_manifest(name, previous)
            .await
            .context("Failed to update manifest")?;

        info!("Successfully rolled back {} to {}", name, previous);
        Ok(())
    }

    /// Get bundled extension directory if available
    ///
    /// Returns Some(PathBuf) if the extension exists in SINDRI_EXT_HOME, None otherwise.
    async fn get_bundled_extension_dir(&self, name: &str) -> Result<Option<PathBuf>> {
        if let Ok(ext_home) = std::env::var("SINDRI_EXT_HOME") {
            let bundled_ext_dir = std::path::PathBuf::from(&ext_home).join(name);

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
            let extension = self.load_extension(&bundled_dir)
                .context("Failed to load bundled extension")?;
            let version = Version::parse(&extension.metadata.version)
                .context(format!("Invalid version in bundled extension: {}", extension.metadata.version))?;
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
                let content = fs::read_to_string(&bundled_path)
                    .await
                    .context(format!(
                        "Failed to read bundled compatibility matrix at {}",
                        bundled_path.display()
                    ))?;
                return serde_yaml::from_str(&content)
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
                    return serde_yaml::from_str(&content)
                        .context("Failed to parse cached compatibility matrix");
                }
            }
        }

        debug!("Fetching compatibility matrix from GitHub");

        // Fetch from GitHub
        let url = format!(
            "https://raw.githubusercontent.com/{}/main/compatibility-matrix.yaml",
            EXTENSIONS_REPO
        );
        let client = reqwest::Client::new();
        let content = client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch compatibility matrix from GitHub")?
            .text()
            .await
            .context("Failed to read compatibility matrix response")?;

        // Cache it
        fs::create_dir_all(&self.cache_dir)
            .await
            .context("Failed to create cache directory")?;
        fs::write(&cache_path, &content)
            .await
            .context("Failed to write compatibility matrix to cache")?;

        serde_yaml::from_str(&content).context("Failed to parse compatibility matrix")
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
    pub async fn find_latest_compatible(&self, name: &str, req: &VersionReq) -> Result<Version> {
        debug!("Finding latest compatible version for {}", name);

        let (owner, repo) = self.parse_repo()?;
        let releases = self
            .github_client
            .repos(owner, repo)
            .releases()
            .list()
            .per_page(100)
            .send()
            .await
            .context("Failed to fetch releases from GitHub")?;

        let prefix = format!("{}@", name);

        let compatible: Vec<Version> = releases
            .items
            .iter()
            .filter(|r| r.tag_name.starts_with(&prefix))
            .filter_map(|r| {
                let version_str = r.tag_name.trim_start_matches(&prefix);
                Version::parse(version_str).ok()
            })
            .filter(|v| req.matches(v))
            .collect();

        compatible.into_iter().max().ok_or_else(|| {
            anyhow!(
                "No compatible version found for {} (requires {})",
                name,
                req
            )
        })
    }

    /// List all available versions for an extension from GitHub releases
    ///
    /// Returns a list of (Version, DateTime<Utc>, bool) tuples where:
    /// - Version: The semantic version
    /// - DateTime<Utc>: The release date
    /// - bool: Whether this version is compatible with the current CLI
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

        let (owner, repo) = self.parse_repo()?;
        let releases = self
            .github_client
            .repos(owner, repo)
            .releases()
            .list()
            .per_page(100)
            .send()
            .await
            .context("Failed to fetch releases from GitHub")?;

        let prefix = format!("{}@", name);

        let mut versions: Vec<(Version, DateTime<Utc>, bool)> = releases
            .items
            .iter()
            .filter(|r| r.tag_name.starts_with(&prefix))
            .filter_map(|r| {
                let version_str = r.tag_name.trim_start_matches(&prefix);
                let version = Version::parse(version_str).ok()?;
                let published_at = r.published_at.unwrap_or_else(Utc::now);
                let is_compatible = compatible_range
                    .map(|req| req.matches(&version))
                    .unwrap_or(true);
                Some((version, published_at, is_compatible))
            })
            .collect();

        // Sort by version descending (newest first)
        versions.sort_by(|a, b| b.0.cmp(&a.0));

        Ok(versions)
    }

    /// Get the previous version of an extension from the manifest
    ///
    /// Returns the most recent previous version if available
    ///
    /// # Arguments
    /// * `name` - Extension name
    pub fn get_previous_version(&self, name: &str) -> Result<Option<Version>> {
        let manifest = self.load_manifest_sync()?;

        if let Some(ext_entry) = manifest.extensions.get(name) {
            if let Some(prev_version_str) = ext_entry.previous_versions.first() {
                let version = Version::parse(prev_version_str).context(format!(
                    "Invalid previous version in manifest: {}",
                    prev_version_str
                ))?;
                return Ok(Some(version));
            }
        }

        Ok(None)
    }

    /// Download an extension archive from GitHub releases
    async fn download_extension(&self, name: &str, version: &Version) -> Result<PathBuf> {
        debug!("Downloading {} version {}", name, version);

        let tag = format!("{}@{}", name, version);
        let download_url = format!(
            "https://github.com/{}/releases/download/{}/{}-{}.tar.gz",
            EXTENSIONS_REPO, tag, name, version
        );

        let download_dir = self.cache_dir.join("downloads");
        fs::create_dir_all(&download_dir)
            .await
            .context("Failed to create downloads directory")?;

        let archive_path = download_dir.join(format!("{}-{}.tar.gz", name, version));

        // Skip download if already exists
        if archive_path.exists() {
            debug!("Using cached archive: {}", archive_path.display());
            return Ok(archive_path);
        }

        // Download with progress
        let client = reqwest::Client::new();
        let response = client
            .get(&download_url)
            .send()
            .await
            .context(format!("Failed to download from {}", download_url))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to download extension: HTTP {}",
                response.status()
            ));
        }

        let bytes = response
            .bytes()
            .await
            .context("Failed to read download response")?;
        fs::write(&archive_path, &bytes)
            .await
            .context("Failed to write archive to disk")?;

        info!("Downloaded {} to {}", name, archive_path.display());
        Ok(archive_path)
    }

    /// Extract an extension archive to the extensions directory
    fn extract_extension(&self, archive: &Path, name: &str, version: &Version) -> Result<PathBuf> {
        debug!("Extracting {} version {}", name, version);

        let dest = self.extensions_dir.join(name).join(version.to_string());
        std::fs::create_dir_all(&dest).context("Failed to create extraction directory")?;

        let file = std::fs::File::open(archive).context("Failed to open archive")?;
        let decoder = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);
        archive.unpack(&dest).context("Failed to extract archive")?;

        info!("Extracted {} to {}", name, dest.display());
        Ok(dest)
    }

    /// Load an extension definition from a directory
    fn load_extension(&self, ext_dir: &Path) -> Result<Extension> {
        let extension_yaml = ext_dir.join("extension.yaml");
        let content = std::fs::read_to_string(&extension_yaml)
            .context(format!("Failed to read {}", extension_yaml.display()))?;

        serde_yaml::from_str(&content).context("Failed to parse extension.yaml")
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
        use sha2::{Digest, Sha256};

        // First perform basic validation
        self.validate_extension(extension)?;

        // Compute SHA256 checksum of the YAML content
        let mut hasher = Sha256::new();
        hasher.update(yaml_content.as_bytes());
        let computed = hasher.finalize();
        let computed_hex = format!("{:x}", computed);

        if computed_hex != expected_checksum.to_lowercase() {
            return Err(anyhow!(
                "Checksum mismatch for extension {}: expected {}, got {}",
                extension.metadata.name,
                expected_checksum,
                computed_hex
            ));
        }

        debug!(
            "Extension {} passed checksum verification",
            extension.metadata.name
        );

        Ok(())
    }

    /// Check if a specific version of an extension is installed
    fn is_installed(&self, name: &str, version: &Version) -> Result<bool> {
        let ext_dir = self.extensions_dir.join(name).join(version.to_string());
        Ok(ext_dir.exists())
    }

    /// Check if any version of an extension is installed
    fn is_any_version_installed(&self, name: &str) -> Result<bool> {
        let ext_dir = self.extensions_dir.join(name);
        if !ext_dir.exists() {
            return Ok(false);
        }

        // Check if there's at least one version directory
        let entries = std::fs::read_dir(&ext_dir)
            .context(format!("Failed to read directory: {}", ext_dir.display()))?;

        Ok(entries.count() > 0)
    }

    /// Get the currently installed version of an extension
    fn get_installed_version(&self, name: &str) -> Result<Option<Version>> {
        let manifest = self.load_manifest_sync()?;

        if let Some(ext_entry) = manifest.extensions.get(name) {
            let version = Version::parse(&ext_entry.version).context(format!(
                "Invalid version in manifest: {}",
                ext_entry.version
            ))?;
            Ok(Some(version))
        } else {
            Ok(None)
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

    /// Update the manifest with the installed extension version
    async fn update_manifest(&self, name: &str, version: &Version) -> Result<()> {
        let manifest_path = self.extensions_dir.parent().unwrap().join("manifest.yaml");

        let mut manifest = if manifest_path.exists() {
            let content = fs::read_to_string(&manifest_path)
                .await
                .context("Failed to read manifest")?;
            serde_yaml::from_str(&content).context("Failed to parse manifest")?
        } else {
            ExtensionManifest::new(self.cli_version.to_string())
        };

        // Get previous versions
        let previous = if let Some(existing) = manifest.extensions.get(name) {
            let mut prev = existing.previous_versions.clone();
            prev.insert(0, existing.version.clone());
            prev
        } else {
            vec![]
        };

        // Update entry
        manifest.extensions.insert(
            name.to_string(),
            ManifestEntry {
                version: version.to_string(),
                installed_at: Utc::now(),
                source: format!("github:{}", EXTENSIONS_REPO),
                previous_versions: previous,
                protected: false,
            },
        );

        manifest.last_updated = Utc::now();

        // Write manifest
        let content = serde_yaml::to_string(&manifest).context("Failed to serialize manifest")?;
        fs::write(&manifest_path, content)
            .await
            .context("Failed to write manifest")?;

        Ok(())
    }

    /// Load manifest synchronously
    fn load_manifest_sync(&self) -> Result<ExtensionManifest> {
        let manifest_path = self.extensions_dir.parent().unwrap().join("manifest.yaml");

        if !manifest_path.exists() {
            return Ok(ExtensionManifest::new(self.cli_version.to_string()));
        }

        let content = std::fs::read_to_string(&manifest_path).context("Failed to read manifest")?;
        serde_yaml::from_str(&content).context("Failed to parse manifest")
    }

    /// Parse the repository owner and name
    fn parse_repo(&self) -> Result<(&str, &str)> {
        let parts: Vec<&str> = EXTENSIONS_REPO.split('/').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid repository format: {}", EXTENSIONS_REPO));
        }
        Ok((parts[0], parts[1]))
    }
}

/// Extension manifest tracking installed extensions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    /// Schema version
    pub schema_version: String,

    /// CLI version
    pub cli_version: String,

    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,

    /// Installed extensions
    pub extensions: HashMap<String, ManifestEntry>,
}

impl ExtensionManifest {
    /// Create a new manifest
    pub fn new(cli_version: String) -> Self {
        Self {
            schema_version: "1.0".to_string(),
            cli_version,
            last_updated: Utc::now(),
            extensions: HashMap::new(),
        }
    }
}

/// Manifest entry for an installed extension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    /// Installed version
    pub version: String,

    /// Installation timestamp
    pub installed_at: DateTime<Utc>,

    /// Source (e.g., "github:sindri/sindri-extensions")
    pub source: String,

    /// Previous versions (for rollback)
    #[serde(default)]
    pub previous_versions: Vec<String>,

    /// Protected (cannot be removed)
    #[serde(default)]
    pub protected: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_creation() {
        let manifest = ExtensionManifest::new("3.0.0".to_string());
        assert_eq!(manifest.schema_version, "1.0");
        assert_eq!(manifest.cli_version, "3.0.0");
        assert!(manifest.extensions.is_empty());
    }

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
}
