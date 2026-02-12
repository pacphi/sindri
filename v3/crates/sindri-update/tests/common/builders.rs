//! Builder patterns for test data construction
//!
//! Provides fluent APIs for constructing Release and ReleaseAsset objects
//! with sensible defaults for testing.

use sindri_update::releases::{Release, ReleaseAsset};

use super::constants::*;

/// Builder for constructing Release objects with sensible test defaults
#[derive(Debug, Clone)]
pub struct ReleaseBuilder {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    prerelease: bool,
    draft: bool,
    assets: Vec<ReleaseAsset>,
    published_at: Option<String>,
}

impl ReleaseBuilder {
    /// Create a new ReleaseBuilder with minimal defaults
    pub fn new() -> Self {
        Self {
            tag_name: TAG_V3_0_0.to_string(),
            name: None,
            body: None,
            prerelease: false,
            draft: false,
            assets: Vec::new(),
            published_at: None,
        }
    }

    /// Set the tag name
    pub fn tag(mut self, tag: &str) -> Self {
        self.tag_name = tag.to_string();
        self
    }

    /// Set the version (automatically adds 'v' prefix for tag)
    pub fn version(mut self, version: &str) -> Self {
        self.tag_name = format!("v{}", version);
        self
    }

    /// Set the release name
    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    /// Set the release body/changelog
    pub fn body(mut self, body: &str) -> Self {
        self.body = Some(body.to_string());
        self
    }

    /// Mark as prerelease
    pub fn prerelease(mut self) -> Self {
        self.prerelease = true;
        self
    }

    /// Mark as draft
    pub fn draft(mut self) -> Self {
        self.draft = true;
        self
    }

    /// Set the published date
    pub fn published_at(mut self, date: &str) -> Self {
        self.published_at = Some(date.to_string());
        self
    }

    /// Add a single asset
    pub fn asset(mut self, asset: ReleaseAsset) -> Self {
        self.assets.push(asset);
        self
    }

    /// Add multiple assets
    pub fn assets(mut self, assets: Vec<ReleaseAsset>) -> Self {
        self.assets.extend(assets);
        self
    }

    /// Add standard platform assets for common test scenarios
    pub fn with_standard_assets(mut self) -> Self {
        self.assets = vec![
            ReleaseAssetBuilder::new()
                .platform(PLATFORM_LINUX_X86_64)
                .url("https://example.com/sindri-linux")
                .size(1024)
                .build(),
            ReleaseAssetBuilder::new()
                .platform(PLATFORM_LINUX_AARCH64)
                .url("https://example.com/sindri-linux-arm")
                .size(1024)
                .build(),
            ReleaseAssetBuilder::new()
                .platform(PLATFORM_MACOS_X86_64)
                .url("https://example.com/sindri-macos")
                .size(1024)
                .build(),
            ReleaseAssetBuilder::new()
                .platform(PLATFORM_MACOS_ARM64)
                .url("https://example.com/sindri-macos-arm")
                .size(1024)
                .build(),
            ReleaseAssetBuilder::new()
                .platform(PLATFORM_WINDOWS_X86_64)
                .name_suffix(".exe")
                .url("https://example.com/sindri-windows")
                .size(1024)
                .build(),
            ReleaseAssetBuilder::checksum()
                .url("https://example.com/checksums")
                .build(),
        ];
        self
    }

    /// Build a complete test release with name and notes
    pub fn with_full_metadata(self) -> Self {
        self.name("Test Release 3.0.0")
            .body("Test release notes")
            .published_at("2024-01-01T00:00:00Z")
    }

    /// Build the Release
    pub fn build(self) -> Release {
        Release {
            tag_name: self.tag_name,
            name: self.name,
            body: self.body,
            prerelease: self.prerelease,
            draft: self.draft,
            assets: self.assets,
            published_at: self.published_at,
        }
    }
}

impl Default for ReleaseBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing ReleaseAsset objects
#[derive(Debug, Clone)]
pub struct ReleaseAssetBuilder {
    name: String,
    browser_download_url: String,
    size: u64,
}

impl ReleaseAssetBuilder {
    /// Create a new asset builder with minimal defaults
    pub fn new() -> Self {
        Self {
            name: String::new(),
            browser_download_url: String::new(),
            size: 1024,
        }
    }

    /// Create a checksum asset builder
    pub fn checksum() -> Self {
        Self {
            name: "checksums.sha256".to_string(),
            browser_download_url: String::new(),
            size: 256,
        }
    }

    /// Set the platform (generates standard asset name)
    pub fn platform(mut self, platform: &str) -> Self {
        self.name = format!("sindri-{}", platform);
        self
    }

    /// Add a suffix to the name (e.g., ".exe" for Windows)
    pub fn name_suffix(mut self, suffix: &str) -> Self {
        self.name.push_str(suffix);
        self
    }

    /// Set a custom name
    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    /// Set the download URL
    pub fn url(mut self, url: &str) -> Self {
        self.browser_download_url = url.to_string();
        self
    }

    /// Set the URL using a mock server base URL and platform
    pub fn mock_url(mut self, server_uri: &str, platform: &str) -> Self {
        self.browser_download_url = format!("{}/sindri-{}", server_uri, platform);
        self
    }

    /// Set the URL using a mock server base URL with custom path
    pub fn mock_url_path(mut self, server_uri: &str, path: &str) -> Self {
        self.browser_download_url = format!("{}{}", server_uri, path);
        self
    }

    /// Set the asset size
    pub fn size(mut self, size: u64) -> Self {
        self.size = size;
        self
    }

    /// Set size from content bytes
    pub fn size_from_content(mut self, content: &[u8]) -> Self {
        self.size = content.len() as u64;
        self
    }

    /// Build the ReleaseAsset
    pub fn build(self) -> ReleaseAsset {
        ReleaseAsset {
            name: self.name,
            browser_download_url: self.browser_download_url,
            size: self.size,
        }
    }
}

impl Default for ReleaseAssetBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a release for a specific platform with mock server URL
pub fn release_for_mock_server(server_uri: &str, platform: &str, content: &[u8]) -> Release {
    ReleaseBuilder::new()
        .asset(
            ReleaseAssetBuilder::new()
                .platform(platform)
                .mock_url(server_uri, platform)
                .size_from_content(content)
                .build(),
        )
        .build()
}

/// Create a release with only a checksum asset (for no-match tests)
pub fn release_checksum_only() -> Release {
    ReleaseBuilder::new()
        .asset(
            ReleaseAssetBuilder::checksum()
                .url("https://example.com/checksums")
                .build(),
        )
        .build()
}
