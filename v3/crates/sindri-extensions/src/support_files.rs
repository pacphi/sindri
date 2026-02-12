//! Support file management with version-aware GitHub fetching
//!
//! This module handles fetching and caching Sindri support files (common.sh,
//! compatibility-matrix.yaml, extension-source.yaml) with automatic version
//! matching and graceful fallback to bundled files.
//!
//! ## Version Handling
//!
//! Supports all semantic version formats:
//! - Stable: `3.0.0`
//! - Pre-release: `3.0.0-alpha.18`, `3.0.0-beta.3`, `3.0.0-rc.1`
//! - Development: `3.0.0-dev`
//! - Build metadata: `3.0.0+20240115`
//!
//! ## Fallback Strategy
//!
//! 1. **Volume cache** - Check if files exist and match CLI version
//! 2. **GitHub fetch** - Download from raw.githubusercontent.com
//! 3. **Bundled files** - Fall back to image-bundled files
//!
//! ## Usage
//!
//! ```rust,no_run
//! use sindri_extensions::SupportFileManager;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let manager = SupportFileManager::new()?;
//! manager.update_all(false).await?; // Update if needed
//! manager.update_all(true).await?;  // Force update
//! # Ok(())
//! # }
//! ```

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, info, warn};

/// Metadata about fetched support files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportFileMetadata {
    /// CLI version these files are for
    pub cli_version: String,

    /// When files were fetched
    pub fetched_at: DateTime<Utc>,

    /// Source of files (github, bundled)
    pub source: SupportFileSource,

    /// GitHub tag used (if fetched from GitHub)
    pub github_tag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SupportFileSource {
    /// Fetched from GitHub
    GitHub,

    /// Copied from bundled files in image
    Bundled,

    /// Unknown/legacy
    Unknown,
}

/// Support file entry
#[derive(Debug, Clone)]
pub struct SupportFile {
    /// Filename (e.g., "common.sh")
    pub name: &'static str,

    /// Path within GitHub repo (e.g., "v3/common.sh")
    pub repo_path: &'static str,

    /// Destination path relative to SINDRI_HOME
    pub dest_path: &'static str,

    /// Bundled fallback path (in Docker image)
    pub bundled_path: &'static str,
}

impl SupportFile {
    const COMMON_SH: Self = Self {
        name: "common.sh",
        repo_path: "v3/common.sh",
        dest_path: "extensions/common.sh",
        bundled_path: "/docker/config/sindri/common.sh",
    };

    const COMPATIBILITY_MATRIX: Self = Self {
        name: "compatibility-matrix.yaml",
        repo_path: "v3/compatibility-matrix.yaml",
        dest_path: "compatibility-matrix.yaml",
        bundled_path: "/docker/config/sindri/compatibility-matrix.yaml",
    };

    const EXTENSION_SOURCE: Self = Self {
        name: "extension-source.yaml",
        repo_path: "v3/extension-source.yaml",
        dest_path: "extension-source.yaml",
        bundled_path: "/docker/config/sindri/extension-source.yaml",
    };

    /// Get all support files
    pub fn all() -> &'static [Self] {
        &[
            Self::COMMON_SH,
            Self::COMPATIBILITY_MATRIX,
            Self::EXTENSION_SOURCE,
        ]
    }
}

/// Manager for support file operations
pub struct SupportFileManager {
    /// Current CLI version
    cli_version: Version,

    /// Sindri home directory (~/.sindri)
    sindri_home: PathBuf,

    /// Path to metadata file
    metadata_path: PathBuf,

    /// GitHub repository config
    repo_owner: String,
    repo_name: String,
}

impl SupportFileManager {
    /// Create new manager with default paths
    pub fn new() -> Result<Self> {
        let cli_version = Self::get_cli_version()?;

        let home = sindri_core::get_home_dir()?;

        let sindri_home = std::env::var("SINDRI_HOME")
            .ok()
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".sindri"));

        let metadata_path = sindri_home.join(".support-files-metadata.yaml");

        Ok(Self {
            cli_version,
            sindri_home,
            metadata_path,
            repo_owner: "pacphi".to_string(),
            repo_name: "sindri".to_string(),
        })
    }

    /// Get current CLI version
    fn get_cli_version() -> Result<Version> {
        Version::parse(env!("CARGO_PKG_VERSION")).context("Failed to parse CLI version")
    }

    /// Build GitHub tag from version
    ///
    /// Handles all version formats:
    /// - `3.0.0` → `v3.0.0`
    /// - `3.0.0-alpha.18` → `v3.0.0-alpha.18`
    /// - `3.0.0-beta.3` → `v3.0.0-beta.3`
    /// - `3.0.0-rc.1` → `v3.0.0-rc.1`
    /// - `3.0.0-dev` → `v3.0.0-dev`
    /// - `3.0.0+20240115` → `v3.0.0` (build metadata stripped)
    pub fn build_tag(&self) -> String {
        // Build metadata must be explicitly stripped (Display includes it)
        format!(
            "v{}.{}.{}{}",
            self.cli_version.major,
            self.cli_version.minor,
            self.cli_version.patch,
            if self.cli_version.pre.is_empty() {
                String::new()
            } else {
                format!("-{}", self.cli_version.pre)
            }
        )
    }

    /// Build GitHub raw URL for a file
    fn build_github_url(&self, tag: &str, repo_path: &str) -> String {
        format!(
            "https://raw.githubusercontent.com/{}/{}/{}/{}",
            self.repo_owner, self.repo_name, tag, repo_path
        )
    }

    /// Check if support files need updating
    pub async fn needs_update(&self) -> Result<bool> {
        // Check if metadata exists
        if !self.metadata_path.exists() {
            debug!("No metadata file found, update needed");
            return Ok(true);
        }

        // Load and check metadata
        let metadata = self.load_metadata().await?;

        // Parse stored version
        let stored_version =
            Version::parse(&metadata.cli_version).context("Failed to parse stored CLI version")?;

        // Update needed if versions don't match
        let needs_update = stored_version != self.cli_version;

        if needs_update {
            info!(
                "Version mismatch: stored={}, current={}",
                stored_version, self.cli_version
            );
        }

        Ok(needs_update)
    }

    /// Load metadata file
    async fn load_metadata(&self) -> Result<SupportFileMetadata> {
        let content = fs::read_to_string(&self.metadata_path)
            .await
            .context("Failed to read metadata file")?;

        serde_yaml::from_str(&content).context("Failed to parse metadata file")
    }

    /// Save metadata file
    async fn save_metadata(&self, metadata: &SupportFileMetadata) -> Result<()> {
        let content = serde_yaml::to_string(metadata).context("Failed to serialize metadata")?;

        fs::write(&self.metadata_path, content)
            .await
            .context("Failed to write metadata file")?;

        Ok(())
    }

    /// Update all support files
    ///
    /// # Arguments
    /// * `force` - If true, update even if versions match
    ///
    /// # Returns
    /// * `Ok(true)` - Files were updated
    /// * `Ok(false)` - Files were already up-to-date
    pub async fn update_all(&self, force: bool) -> Result<bool> {
        // Check if update needed
        if !force && !self.needs_update().await? {
            debug!("Support files already up-to-date");
            return Ok(false);
        }

        info!("Updating support files for CLI v{}", self.cli_version);

        // Try GitHub first, then fall back to bundled
        let tag = self.build_tag();
        let mut source = SupportFileSource::Unknown;
        let mut success_count = 0;

        for file in SupportFile::all() {
            let dest_path = self.sindri_home.join(file.dest_path);

            // Ensure parent directory exists
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent).await?;
            }

            // Try GitHub first
            match self.fetch_from_github(&tag, file).await {
                Ok(content) => {
                    fs::write(&dest_path, content).await?;
                    info!("✓ Fetched {} from GitHub", file.name);
                    source = SupportFileSource::GitHub;
                    success_count += 1;
                }
                Err(e) => {
                    warn!("Failed to fetch {} from GitHub: {}", file.name, e);

                    // Fall back to bundled
                    match self.copy_bundled(file, &dest_path).await {
                        Ok(()) => {
                            info!("✓ Copied {} from bundled", file.name);
                            source = SupportFileSource::Bundled;
                            success_count += 1;
                        }
                        Err(e2) => {
                            warn!("Failed to copy bundled {}: {}", file.name, e2);
                        }
                    }
                }
            }
        }

        if success_count == 0 {
            return Err(anyhow!("Failed to update any support files"));
        }

        // Save metadata
        let metadata = SupportFileMetadata {
            cli_version: self.cli_version.to_string(),
            fetched_at: Utc::now(),
            source,
            github_tag: Some(tag),
        };

        self.save_metadata(&metadata).await?;

        info!(
            "Support files updated successfully ({} files from {:?})",
            success_count, metadata.source
        );

        Ok(true)
    }

    /// Fetch file from GitHub
    async fn fetch_from_github(&self, tag: &str, file: &SupportFile) -> Result<Vec<u8>> {
        let url = self.build_github_url(tag, file.repo_path);
        debug!("Fetching {} from {}", file.name, url);

        let response = reqwest::get(&url)
            .await
            .context(format!("Failed to fetch {} from GitHub", file.name))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "GitHub returned status {} for {}",
                response.status(),
                file.name
            ));
        }

        let content = response
            .bytes()
            .await
            .context("Failed to read response body")?
            .to_vec();

        Ok(content)
    }

    /// Copy file from bundled location
    async fn copy_bundled(&self, file: &SupportFile, dest: &PathBuf) -> Result<()> {
        let bundled_path = PathBuf::from(file.bundled_path);

        if !bundled_path.exists() {
            return Err(anyhow!("Bundled file not found: {:?}", bundled_path));
        }

        fs::copy(&bundled_path, dest)
            .await
            .context(format!("Failed to copy bundled {}", file.name))?;

        Ok(())
    }

    /// Get current metadata (if available)
    pub async fn get_metadata(&self) -> Result<Option<SupportFileMetadata>> {
        if !self.metadata_path.exists() {
            return Ok(None);
        }

        self.load_metadata().await.map(Some)
    }

    /// Force update from bundled files (offline fallback)
    pub async fn update_from_bundled(&self) -> Result<()> {
        info!("Updating support files from bundled sources (offline mode)");

        for file in SupportFile::all() {
            let dest_path = self.sindri_home.join(file.dest_path);

            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent).await?;
            }

            self.copy_bundled(file, &dest_path).await?;
            info!("✓ Copied {} from bundled", file.name);
        }

        // Save metadata
        let metadata = SupportFileMetadata {
            cli_version: self.cli_version.to_string(),
            fetched_at: Utc::now(),
            source: SupportFileSource::Bundled,
            github_tag: None,
        };

        self.save_metadata(&metadata).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_tag() {
        let manager = SupportFileManager {
            cli_version: Version::parse("3.0.0").unwrap(),
            sindri_home: PathBuf::from("/tmp"),
            metadata_path: PathBuf::from("/tmp/.metadata"),
            repo_owner: "pacphi".to_string(),
            repo_name: "sindri".to_string(),
        };

        assert_eq!(manager.build_tag(), "v3.0.0");
    }

    #[test]
    fn test_build_tag_prerelease() {
        let manager = SupportFileManager {
            cli_version: Version::parse("3.0.0-alpha.18").unwrap(),
            sindri_home: PathBuf::from("/tmp"),
            metadata_path: PathBuf::from("/tmp/.metadata"),
            repo_owner: "pacphi".to_string(),
            repo_name: "sindri".to_string(),
        };

        assert_eq!(manager.build_tag(), "v3.0.0-alpha.18");
    }

    #[test]
    fn test_build_tag_build_metadata() {
        // Build metadata is stripped by semver::Version
        let manager = SupportFileManager {
            cli_version: Version::parse("3.0.0+20240115").unwrap(),
            sindri_home: PathBuf::from("/tmp"),
            metadata_path: PathBuf::from("/tmp/.metadata"),
            repo_owner: "pacphi".to_string(),
            repo_name: "sindri".to_string(),
        };

        // Build metadata is automatically stripped
        assert_eq!(manager.build_tag(), "v3.0.0");
    }

    #[test]
    fn test_build_github_url() {
        let manager = SupportFileManager {
            cli_version: Version::parse("3.0.0-alpha.18").unwrap(),
            sindri_home: PathBuf::from("/tmp"),
            metadata_path: PathBuf::from("/tmp/.metadata"),
            repo_owner: "pacphi".to_string(),
            repo_name: "sindri".to_string(),
        };

        let url = manager.build_github_url("v3.0.0-alpha.18", "v3/common.sh");
        assert_eq!(
            url,
            "https://raw.githubusercontent.com/pacphi/sindri/v3.0.0-alpha.18/v3/common.sh"
        );
    }
}
