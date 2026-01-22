//! GitHub releases management

use anyhow::{anyhow, Result};
use semver::Version;
use serde::Deserialize;
use sindri_core::config::HierarchicalConfigLoader;
use sindri_core::types::{GitHubConfig, PlatformMatrix};
use tracing::{debug, info};

/// Release information
#[derive(Debug, Clone, Deserialize)]
pub struct Release {
    /// Release tag (e.g., "v3.0.0")
    pub tag_name: String,

    /// Release name
    pub name: Option<String>,

    /// Release body (changelog)
    pub body: Option<String>,

    /// Whether this is a prerelease
    pub prerelease: bool,

    /// Whether this is a draft
    pub draft: bool,

    /// Release assets
    pub assets: Vec<ReleaseAsset>,

    /// Published date
    pub published_at: Option<String>,
}

/// Release asset
#[derive(Debug, Clone, Deserialize)]
pub struct ReleaseAsset {
    /// Asset name
    pub name: String,

    /// Download URL
    pub browser_download_url: String,

    /// Asset size in bytes
    pub size: u64,
}

/// Release manager for checking and downloading updates
pub struct ReleaseManager {
    /// GitHub API client
    client: reqwest::Client,

    /// Include prereleases
    include_prerelease: bool,

    /// GitHub configuration
    github_config: GitHubConfig,

    /// Platform support matrix
    platform_matrix: PlatformMatrix,
}

impl ReleaseManager {
    /// Create a new release manager
    pub fn new() -> Self {
        // Load configuration
        let config_loader =
            HierarchicalConfigLoader::new().expect("Failed to create config loader");
        let runtime_config = config_loader
            .load_runtime_config()
            .expect("Failed to load runtime config");
        let platform_matrix = config_loader
            .load_platform_matrix()
            .expect("Failed to load platform matrix");

        Self {
            client: reqwest::Client::builder()
                .user_agent(&runtime_config.network.user_agent)
                .build()
                .expect("Failed to create HTTP client"),
            include_prerelease: false,
            github_config: runtime_config.github,
            platform_matrix,
        }
    }

    /// Include prerelease versions
    pub fn with_prerelease(mut self) -> Self {
        self.include_prerelease = true;
        self
    }

    /// Get latest release
    pub async fn get_latest(&self) -> Result<Release> {
        let url = format!(
            "{}/repos/{}/{}/releases/latest",
            self.github_config.api_url, self.github_config.repo_owner, self.github_config.repo_name
        );

        debug!("Fetching latest release from: {}", url);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to fetch release: {}", response.status()));
        }

        let release: Release = response.json().await?;
        Ok(release)
    }

    /// List all releases
    pub async fn list_releases(&self, limit: usize) -> Result<Vec<Release>> {
        let url = format!(
            "{}/repos/{}/{}/releases?per_page={}",
            self.github_config.api_url,
            self.github_config.repo_owner,
            self.github_config.repo_name,
            limit
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to list releases: {}", response.status()));
        }

        let mut releases: Vec<Release> = response.json().await?;

        // Filter out drafts and optionally prereleases
        releases.retain(|r| !r.draft && (self.include_prerelease || !r.prerelease));

        Ok(releases)
    }

    /// Get release by tag
    pub async fn get_release(&self, tag: &str) -> Result<Release> {
        let url = format!(
            "{}/repos/{}/{}/releases/tags/{}",
            self.github_config.api_url,
            self.github_config.repo_owner,
            self.github_config.repo_name,
            tag
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("Release {} not found", tag));
        }

        let release: Release = response.json().await?;
        Ok(release)
    }

    /// Check if update is available
    pub async fn check_update(&self, current_version: &str) -> Result<Option<Release>> {
        let current = Version::parse(current_version)?;
        let latest = self.get_latest().await?;

        let latest_version = latest
            .tag_name
            .strip_prefix('v')
            .unwrap_or(&latest.tag_name);

        let latest_ver = Version::parse(latest_version)?;

        if latest_ver > current {
            info!("Update available: {} -> {}", current, latest_ver);
            Ok(Some(latest))
        } else {
            debug!("Already on latest version: {}", current);
            Ok(None)
        }
    }

    /// Get appropriate asset for current platform
    pub fn get_platform_asset<'a>(&self, release: &'a Release) -> Option<&'a ReleaseAsset> {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;

        // Find platform definition from matrix
        let platform = self.platform_matrix.find_platform(os, arch)?;

        debug!(
            "Detected platform: {}-{} -> target: {}",
            os, arch, platform.target
        );

        release
            .assets
            .iter()
            .find(|a| a.name.contains(&platform.target))
    }
}

impl Default for ReleaseManager {
    fn default() -> Self {
        Self::new()
    }
}
