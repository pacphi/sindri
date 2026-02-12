//! Binary download with progress tracking and verification
//!
//! This module provides functionality for downloading Sindri CLI binaries
//! from GitHub releases with:
//! - Progress tracking using indicatif
//! - SHA256 checksum verification
//! - Platform-specific binary selection
//! - Download retry logic (3 attempts by default)
//! - Resumable downloads
//! - Temporary file management
//!
//! # Example
//!
//! ```no_run
//! use sindri_update::{BinaryDownloader, ReleaseManager};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Get latest release
//!     let release_manager = ReleaseManager::new();
//!     let release = release_manager.get_latest().await?;
//!
//!     // Download binary for current platform
//!     let downloader = BinaryDownloader::new()?;
//!     let result = downloader.download_release(&release, None).await?;
//!
//!     println!("Downloaded to: {:?}", result.file_path);
//!     println!("Size: {} bytes", result.file_size);
//!     println!("Checksum: {}", result.checksum);
//!
//!     Ok(())
//! }
//! ```

use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::header::{CONTENT_LENGTH, RANGE};
use sha2::{Digest, Sha256};
use sindri_core::config::HierarchicalConfigLoader;
use sindri_core::retry::{RetryError, SimpleRetryExecutor, TracingObserver};
use sindri_core::types::{PlatformMatrix, RetryPolicy, RuntimeConfig};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::TempDir;
use tracing::{debug, info};

use crate::releases::{Release, ReleaseAsset};

/// Chunk size for downloading (1MB)
const DOWNLOAD_CHUNK_SIZE: usize = 1024 * 1024;

/// Download progress information
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    /// Total bytes to download
    pub total_bytes: u64,

    /// Bytes downloaded so far
    pub downloaded_bytes: u64,

    /// Download speed in bytes per second
    pub speed_bps: f64,

    /// Progress percentage (0-100)
    pub percentage: f64,
}

impl DownloadProgress {
    /// Create a new progress tracker
    pub fn new(total_bytes: u64) -> Self {
        Self {
            total_bytes,
            downloaded_bytes: 0,
            speed_bps: 0.0,
            percentage: 0.0,
        }
    }

    /// Update progress with new downloaded bytes
    pub fn update(&mut self, downloaded_bytes: u64) {
        self.downloaded_bytes = downloaded_bytes;
        self.percentage = if self.total_bytes > 0 {
            (self.downloaded_bytes as f64 / self.total_bytes as f64) * 100.0
        } else {
            0.0
        };
    }

    /// Check if download is complete
    pub fn is_complete(&self) -> bool {
        self.downloaded_bytes >= self.total_bytes
    }
}

/// Result of a download operation
#[derive(Debug)]
pub struct DownloadResult {
    /// Path to the downloaded file
    pub file_path: PathBuf,

    /// Size of the downloaded file in bytes
    pub file_size: u64,

    /// SHA256 checksum of the downloaded file
    pub checksum: String,

    /// Whether the download was resumed
    pub resumed: bool,
}

/// Binary downloader with retry and verification capabilities
pub struct BinaryDownloader {
    /// HTTP client
    client: reqwest::Client,

    /// Temporary directory for downloads
    temp_dir: TempDir,

    /// Number of retry attempts (legacy, kept for API compatibility)
    max_retries: u32,

    /// Enable progress bars
    show_progress: bool,

    /// Platform support matrix
    platform_matrix: PlatformMatrix,

    /// Runtime configuration (for future use - retry executor integration)
    #[allow(dead_code)]
    runtime_config: RuntimeConfig,

    /// Retry policy for download operations
    retry_policy: RetryPolicy,
}

impl BinaryDownloader {
    /// Create a new binary downloader
    pub fn new() -> Result<Self> {
        let temp_dir = TempDir::new().context("Failed to create temporary directory")?;

        // Load configuration
        let config_loader =
            HierarchicalConfigLoader::new().context("Failed to create config loader")?;
        let runtime_config = config_loader
            .load_runtime_config()
            .context("Failed to load runtime config")?;
        let platform_matrix = config_loader
            .load_platform_matrix()
            .context("Failed to load platform matrix")?;

        // Use configured timeout
        let timeout_secs = runtime_config.network.download_timeout_secs;
        let user_agent = &runtime_config.network.user_agent;

        // Get retry policy from config or use default
        let retry_policy = runtime_config
            .retry_policies
            .operations
            .get("download")
            .cloned()
            .unwrap_or_else(RetryPolicy::default);

        let max_retries = retry_policy.max_attempts;

        Ok(Self {
            client: reqwest::Client::builder()
                .user_agent(user_agent)
                .timeout(std::time::Duration::from_secs(timeout_secs))
                .build()
                .context("Failed to create HTTP client")?,
            temp_dir,
            max_retries,
            show_progress: true,
            platform_matrix,
            runtime_config,
            retry_policy,
        })
    }

    /// Set maximum retry attempts
    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Enable or disable progress bars
    pub fn with_progress(mut self, show: bool) -> Self {
        self.show_progress = show;
        self
    }

    /// Get the temporary directory path
    pub fn temp_dir(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Download a release binary with retry logic
    ///
    /// Uses the policy-based retry executor from sindri-core for consistent
    /// retry behavior across the codebase.
    pub async fn download_release(
        &self,
        release: &Release,
        platform_override: Option<&str>,
    ) -> Result<DownloadResult> {
        // Get the appropriate asset for the platform
        let asset = if let Some(platform) = platform_override {
            self.get_platform_asset_by_name(release, platform)
        } else {
            self.get_platform_asset(release)
        }
        .ok_or_else(|| anyhow!("No compatible binary found for this platform"))?;

        info!(
            "Downloading {} ({})",
            asset.name,
            human_readable_size(asset.size)
        );

        // Create retry executor with tracing observer
        let observer = Arc::new(TracingObserver::new("download"));
        let executor = SimpleRetryExecutor::<anyhow::Error, _, _>::new(self.retry_policy.clone())
            .with_observer(observer);

        // Track attempt number for progress display
        let attempt_counter = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempt_counter_clone = attempt_counter.clone();

        // Execute download with retry logic
        let result = executor
            .execute(|| {
                let attempt =
                    attempt_counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                self.download_asset(asset, attempt)
            })
            .await;

        match result {
            Ok(download_result) => {
                info!("Download completed successfully");
                Ok(download_result)
            }
            Err(retry_err) => {
                // Convert RetryError to anyhow::Error with context
                match retry_err {
                    RetryError::Exhausted {
                        attempts,
                        source,
                        total_duration,
                    } => Err(anyhow!(
                        "Download failed after {} attempts over {:.1}s: {}",
                        attempts,
                        total_duration.as_secs_f64(),
                        source
                    )),
                    RetryError::NonRetryable(source) => {
                        Err(anyhow!("Download failed (non-retryable): {}", source))
                    }
                    RetryError::Cancelled {
                        attempts,
                        last_error,
                    } => {
                        if let Some(err) = last_error {
                            Err(anyhow!(
                                "Download cancelled after {} attempts: {}",
                                attempts,
                                err
                            ))
                        } else {
                            Err(anyhow!("Download cancelled after {} attempts", attempts))
                        }
                    }
                    RetryError::AttemptTimeout { attempt, timeout } => Err(anyhow!(
                        "Download attempt {} timed out after {}ms",
                        attempt,
                        timeout.as_millis()
                    )),
                }
            }
        }
    }

    /// Download a single asset
    async fn download_asset(&self, asset: &ReleaseAsset, attempt: u32) -> Result<DownloadResult> {
        let file_path = self.temp_dir.path().join(&asset.name);
        let temp_file_path = self.temp_dir.path().join(format!("{}.tmp", asset.name));

        // Check if partial download exists
        let (start_pos, resumed) = if temp_file_path.exists() {
            let metadata = fs::metadata(&temp_file_path)?;
            let size = metadata.len();
            if size < asset.size {
                debug!("Resuming download from byte {}", size);
                (size, true)
            } else {
                // File is complete or corrupted, start over
                fs::remove_file(&temp_file_path)?;
                (0, false)
            }
        } else {
            (0, false)
        };

        // Create request with range header for resumable download
        let mut request = self.client.get(&asset.browser_download_url);
        if start_pos > 0 {
            request = request.header(RANGE, format!("bytes={}-", start_pos));
        }

        let response = request
            .send()
            .await
            .context("Failed to send download request")?;

        if !response.status().is_success() && response.status().as_u16() != 206 {
            return Err(anyhow!(
                "Download failed with status: {}",
                response.status()
            ));
        }

        // Get total size
        let total_size = if start_pos > 0 {
            asset.size
        } else {
            response
                .headers()
                .get(CONTENT_LENGTH)
                .and_then(|ct| ct.to_str().ok())
                .and_then(|ct| ct.parse::<u64>().ok())
                .unwrap_or(asset.size)
        };

        // Create progress bar
        let progress = if self.show_progress {
            let pb = ProgressBar::new(total_size);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
                    .expect("Invalid progress bar template")
                    .progress_chars("#>-"),
            );
            pb.set_message(format!("Downloading {} (attempt {})", asset.name, attempt));
            if start_pos > 0 {
                pb.set_position(start_pos);
            }
            Some(pb)
        } else {
            None
        };

        // Download to temporary file
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&temp_file_path)
            .context("Failed to create temporary file")?;

        let mut downloaded = start_pos;
        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk: bytes::Bytes = chunk_result.context("Failed to read download chunk")?;
            file.write_all(&chunk)
                .context("Failed to write to temporary file")?;

            downloaded += chunk.len() as u64;

            if let Some(pb) = &progress {
                pb.set_position(downloaded);
            }
        }

        if let Some(pb) = progress {
            pb.finish_with_message(format!("Downloaded {}", asset.name));
        }

        // Verify file size
        let final_size = fs::metadata(&temp_file_path)?.len();
        if final_size != asset.size {
            return Err(anyhow!(
                "File size mismatch: expected {}, got {}",
                asset.size,
                final_size
            ));
        }

        // Calculate checksum
        debug!("Calculating SHA256 checksum...");
        let checksum = self.calculate_checksum(&temp_file_path)?;

        // Move temporary file to final location
        fs::rename(&temp_file_path, &file_path)
            .context("Failed to move downloaded file to final location")?;

        Ok(DownloadResult {
            file_path,
            file_size: final_size,
            checksum,
            resumed,
        })
    }

    /// Calculate SHA256 checksum of a file
    pub fn calculate_checksum(&self, path: &Path) -> Result<String> {
        let mut file = File::open(path).context("Failed to open file for checksum calculation")?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; DOWNLOAD_CHUNK_SIZE];

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    }

    /// Verify checksum of a downloaded file
    pub fn verify_checksum(&self, file_path: &Path, expected: &str) -> Result<bool> {
        let actual = self.calculate_checksum(file_path)?;
        Ok(actual.eq_ignore_ascii_case(expected))
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

        self.get_platform_asset_by_name(release, &platform.target)
    }

    /// Get asset by platform name pattern
    fn get_platform_asset_by_name<'a>(
        &self,
        release: &'a Release,
        platform: &str,
    ) -> Option<&'a ReleaseAsset> {
        release
            .assets
            .iter()
            .find(|a| a.name.contains(platform) && !a.name.ends_with(".sha256"))
    }

    /// Get list of all available platforms in a release
    pub fn list_available_platforms(&self, release: &Release) -> Vec<String> {
        let mut platforms = Vec::new();

        // Check all enabled platforms from the matrix
        for (_key, platform) in self.platform_matrix.enabled_platforms() {
            if release
                .assets
                .iter()
                .any(|a| a.name.contains(&platform.target))
            {
                platforms.push(platform.target.clone());
            }
        }

        platforms
    }
}

impl Default for BinaryDownloader {
    fn default() -> Self {
        Self::new().expect("Failed to create binary downloader")
    }
}

/// Convert bytes to human-readable size
fn human_readable_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_index])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_download_progress() {
        let mut progress = DownloadProgress::new(1000);
        assert_eq!(progress.percentage, 0.0);
        assert!(!progress.is_complete());

        progress.update(500);
        assert_eq!(progress.percentage, 50.0);
        assert!(!progress.is_complete());

        progress.update(1000);
        assert_eq!(progress.percentage, 100.0);
        assert!(progress.is_complete());
    }

    #[test]
    fn test_human_readable_size() {
        assert_eq!(human_readable_size(0), "0.00 B");
        assert_eq!(human_readable_size(1023), "1023.00 B");
        assert_eq!(human_readable_size(1024), "1.00 KB");
        assert_eq!(human_readable_size(1024 * 1024), "1.00 MB");
        assert_eq!(human_readable_size(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_platform_detection() {
        let downloader = BinaryDownloader::new().unwrap();

        // Create a mock release
        let release = Release {
            tag_name: "v3.0.0".to_string(),
            name: Some("Release 3.0.0".to_string()),
            body: None,
            prerelease: false,
            draft: false,
            assets: vec![
                ReleaseAsset {
                    name: "sindri-x86_64-unknown-linux-musl".to_string(),
                    browser_download_url: "https://example.com/linux".to_string(),
                    size: 1000,
                },
                ReleaseAsset {
                    name: "sindri-aarch64-unknown-linux-musl".to_string(),
                    browser_download_url: "https://example.com/linux-arm".to_string(),
                    size: 1000,
                },
                ReleaseAsset {
                    name: "sindri-aarch64-apple-darwin".to_string(),
                    browser_download_url: "https://example.com/macos-arm".to_string(),
                    size: 1000,
                },
            ],
            published_at: None,
        };

        let platforms = downloader.list_available_platforms(&release);
        assert!(platforms.contains(&"x86_64-unknown-linux-musl".to_string()));
        assert!(platforms.contains(&"aarch64-unknown-linux-musl".to_string()));
        assert!(platforms.contains(&"aarch64-apple-darwin".to_string()));
        assert_eq!(platforms.len(), 3);
    }

    #[tokio::test]
    async fn test_checksum_calculation() {
        let downloader = BinaryDownloader::new().unwrap();
        let test_file = downloader.temp_dir().join("test.txt");

        // Write test data
        fs::write(&test_file, b"Hello, World!").unwrap();

        // Calculate checksum
        let checksum = downloader.calculate_checksum(&test_file).unwrap();

        // Known SHA256 hash of "Hello, World!"
        let expected = "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f";
        assert_eq!(checksum, expected);
    }

    #[tokio::test]
    async fn test_verify_checksum() {
        let downloader = BinaryDownloader::new().unwrap();
        let test_file = downloader.temp_dir().join("test.txt");

        fs::write(&test_file, b"Hello, World!").unwrap();

        let expected = "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f";
        assert!(downloader.verify_checksum(&test_file, expected).unwrap());

        let wrong = "0000000000000000000000000000000000000000000000000000000000000000";
        assert!(!downloader.verify_checksum(&test_file, wrong).unwrap());
    }
}
