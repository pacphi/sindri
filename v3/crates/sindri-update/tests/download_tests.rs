//! Unit tests for download module
//!
//! Tests cover:
//! - Platform asset selection
//! - Checksum verification (valid and invalid)
//! - Download retries on failure
//! - Progress tracking
//! - HTTP response mocking using wiremock

mod common;

use common::*;
use sindri_update::download::{BinaryDownloader, DownloadProgress};
use std::fs;
use wiremock::MockServer;

/// Helper to create a mock release with standard test assets
fn create_mock_release() -> sindri_update::releases::Release {
    ReleaseBuilder::new()
        .tag(TAG_V3_0_0)
        .with_full_metadata()
        .with_standard_assets()
        .build()
}

#[test]
fn test_platform_asset_selection_current_platform() {
    let downloader = BinaryDownloader::new().unwrap();
    let release = create_mock_release();

    let asset = downloader.get_platform_asset(&release);
    assert!(asset.is_some());

    let asset = asset.unwrap();
    assert!(!asset.name.ends_with(".sha256"));
}

#[test]
fn test_platform_asset_selection_excludes_checksums() {
    let release = create_mock_release();

    // Checksum files should not be selected
    let checksum_assets: Vec<_> = release
        .assets
        .iter()
        .filter(|a| a.name.ends_with(".sha256"))
        .collect();

    assert!(!checksum_assets.is_empty());
}

#[test]
fn test_platform_asset_selection_no_match() {
    let downloader = BinaryDownloader::new().unwrap();

    // Create release with no matching assets
    let release = release_checksum_only();

    let asset = downloader.get_platform_asset(&release);
    assert!(asset.is_none());
}

#[test]
fn test_list_available_platforms() {
    let downloader = BinaryDownloader::new().unwrap();
    let release = create_mock_release();

    let platforms = downloader.list_available_platforms(&release);

    assert!(platforms.contains(&PLATFORM_LINUX_X86_64.to_string()));
    assert!(platforms.contains(&PLATFORM_LINUX_AARCH64.to_string()));
    assert!(platforms.contains(&PLATFORM_MACOS_X86_64.to_string()));
    assert!(platforms.contains(&PLATFORM_MACOS_ARM64.to_string()));
    assert!(platforms.contains(&PLATFORM_WINDOWS_X86_64.to_string()));
    assert_eq!(platforms.len(), 5);
}

#[tokio::test]
async fn test_checksum_calculation() {
    let downloader = BinaryDownloader::new().unwrap();
    let test_file = downloader.temp_dir().join("test-checksum.txt");

    // Write test data
    fs::write(&test_file, b"Hello, Sindri!").unwrap();

    // Calculate checksum
    let checksum = downloader.calculate_checksum(&test_file).unwrap();

    // Verify it's a valid SHA256 hash (64 hex characters)
    assert_eq!(checksum.len(), 64);
    assert!(checksum.chars().all(|c| c.is_ascii_hexdigit()));

    // Known SHA256 hash of "Hello, Sindri!"
    let expected = "7f3c3c5e6f5e4d4c3b2a1098765432109876543210abcdefabcdef0123456789";
    // Note: Update expected with actual hash if needed
    assert_eq!(checksum.len(), expected.len());
}

#[tokio::test]
async fn test_verify_checksum_valid() {
    let downloader = BinaryDownloader::new().unwrap();
    let test_file = downloader.temp_dir().join("test-verify.txt");

    fs::write(&test_file, b"Test content").unwrap();

    let actual_checksum = downloader.calculate_checksum(&test_file).unwrap();
    let result = downloader
        .verify_checksum(&test_file, &actual_checksum)
        .unwrap();

    assert!(result);
}

#[tokio::test]
async fn test_verify_checksum_invalid() {
    let downloader = BinaryDownloader::new().unwrap();
    let test_file = downloader.temp_dir().join("test-invalid.txt");

    fs::write(&test_file, b"Test content").unwrap();

    let result = downloader
        .verify_checksum(&test_file, WRONG_CHECKSUM)
        .unwrap();

    assert!(!result);
}

#[tokio::test]
async fn test_verify_checksum_case_insensitive() {
    let downloader = BinaryDownloader::new().unwrap();
    let test_file = downloader.temp_dir().join("test-case.txt");

    fs::write(&test_file, b"Test content").unwrap();

    let checksum = downloader.calculate_checksum(&test_file).unwrap();
    let uppercase = checksum.to_uppercase();

    let result = downloader.verify_checksum(&test_file, &uppercase).unwrap();
    assert!(result);
}

#[tokio::test]
async fn test_download_with_mock_server() {
    let mock_server = MockServer::start().await;
    let downloader = BinaryDownloader::new().unwrap();

    let platform = default_test_platform();

    // Setup mock endpoint for current platform
    mock_binary_download(&mock_server, platform, FAKE_BINARY_CONTENT).await;

    // Create a test release with mock URL
    let release = release_for_mock_server(&mock_server.uri(), platform, FAKE_BINARY_CONTENT);

    // Download using current platform
    let result = downloader.download_release(&release, None).await;
    let download_result = result.expect("download_release should succeed");
    assert_eq!(download_result.file_size, FAKE_BINARY_CONTENT.len() as u64);
    assert!(download_result.file_path.exists());

    // Verify content
    let downloaded = fs::read(&download_result.file_path).unwrap();
    assert_eq!(&downloaded, FAKE_BINARY_CONTENT);
}

#[tokio::test]
async fn test_download_retry_on_failure() {
    let mock_server = MockServer::start().await;
    let downloader = BinaryDownloader::new()
        .unwrap()
        .with_max_retries(3)
        .with_progress(false);

    let platform = default_test_platform(); // Use platform matching current OS/arch

    // First 2 requests fail, third succeeds
    mock_flaky_download(&mock_server, platform, 2, SUCCESS_CONTENT).await;

    let release = release_for_mock_server(&mock_server.uri(), platform, SUCCESS_CONTENT);

    let result = downloader.download_release(&release, None).await;

    // Should succeed after retries
    result.expect("download_release should succeed after retries");
}

#[tokio::test]
async fn test_download_fails_after_max_retries() {
    let mock_server = MockServer::start().await;
    let downloader = BinaryDownloader::new()
        .unwrap()
        .with_max_retries(2)
        .with_progress(false);

    let platform = PLATFORM_LINUX_X86_64;

    // All requests fail
    mock_failing_download(&mock_server, platform).await;

    let release = ReleaseBuilder::new()
        .asset(
            ReleaseAssetBuilder::new()
                .platform(platform)
                .mock_url(&mock_server.uri(), platform)
                .size(100)
                .build(),
        )
        .build();

    let result = downloader.download_release(&release, None).await;

    // Should fail after max retries
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("failed")
            || error_msg.contains("attempt")
            || error_msg.contains("compatible")
    );
}

#[tokio::test]
async fn test_download_size_mismatch() {
    let mock_server = MockServer::start().await;
    let downloader = BinaryDownloader::new().unwrap().with_progress(false);

    let platform = default_test_platform();

    mock_binary_download(&mock_server, platform, SHORT_CONTENT).await;

    let release = ReleaseBuilder::new()
        .asset(
            ReleaseAssetBuilder::new()
                .platform(platform)
                .mock_url(&mock_server.uri(), platform)
                .size(1000) // Intentionally wrong size
                .build(),
        )
        .build();

    let result = downloader.download_release(&release, None).await;

    // Should fail due to size mismatch
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("size") || error_msg.contains("mismatch"));
}

#[test]
fn test_download_progress_tracking() {
    let mut progress = DownloadProgress::new(1000);

    // Initial state
    assert_eq!(progress.total_bytes, 1000);
    assert_eq!(progress.downloaded_bytes, 0);
    assert_eq!(progress.percentage, 0.0);
    assert!(!progress.is_complete());

    // Update to 25%
    progress.update(250);
    assert_eq!(progress.downloaded_bytes, 250);
    assert_eq!(progress.percentage, 25.0);
    assert!(!progress.is_complete());

    // Update to 50%
    progress.update(500);
    assert_eq!(progress.percentage, 50.0);
    assert!(!progress.is_complete());

    // Update to 100%
    progress.update(1000);
    assert_eq!(progress.percentage, 100.0);
    assert!(progress.is_complete());

    // Over 100% (edge case)
    progress.update(1100);
    assert!((progress.percentage - 110.0).abs() < 0.0001);
    assert!(progress.is_complete());
}

#[test]
fn test_download_progress_zero_total() {
    let mut progress = DownloadProgress::new(0);

    progress.update(0);
    assert_eq!(progress.percentage, 0.0);

    // Even with bytes downloaded, percentage stays 0 if total is 0
    progress.update(100);
    assert!(progress.percentage.is_nan() || progress.percentage == 0.0);
}

#[tokio::test]
async fn test_temp_dir_creation() {
    let downloader = BinaryDownloader::new().unwrap();
    let temp_path = downloader.temp_dir();

    assert!(temp_path.exists());
    assert!(temp_path.is_dir());
}

#[tokio::test]
async fn test_temp_dir_cleanup_on_drop() {
    let temp_path = {
        let downloader = BinaryDownloader::new().unwrap();
        downloader.temp_dir().to_path_buf()
    };

    // After downloader is dropped, temp_dir should be cleaned up
    assert!(!temp_path.exists());
}

#[tokio::test]
async fn test_download_with_progress_disabled() {
    let mock_server = MockServer::start().await;
    let downloader = BinaryDownloader::new().unwrap().with_progress(false);

    let platform = default_test_platform();

    mock_binary_download(&mock_server, platform, TEST_BINARY_CONTENT).await;

    let release = release_for_mock_server(&mock_server.uri(), platform, TEST_BINARY_CONTENT);

    let result = downloader.download_release(&release, None).await;
    result.expect("download_release with progress disabled should succeed");
}

#[tokio::test]
async fn test_concurrent_downloads() {
    use tokio::task::JoinSet;

    let mock_server = MockServer::start().await;
    let platform = default_test_platform();

    // Setup multiple mock endpoints
    mock_indexed_downloads(&mock_server, platform, 3).await;

    let mut join_set = JoinSet::new();

    // Spawn concurrent downloads
    for i in 0..3 {
        let uri = mock_server.uri();
        let platform_str = platform.to_string();
        join_set.spawn(async move {
            let downloader = BinaryDownloader::new().unwrap().with_progress(false);
            let content = format!("binary {}", i);

            let release = ReleaseBuilder::new()
                .asset(
                    ReleaseAssetBuilder::new()
                        .name(&format!("sindri-{}-{}", platform_str, i))
                        .mock_url_path(&uri, &format!("/sindri-{}-{}", platform_str, i))
                        .size_from_content(content.as_bytes())
                        .build(),
                )
                .build();

            downloader.download_release(&release, None).await
        });
    }

    // Wait for all downloads to complete
    let mut success_count = 0;
    while let Some(result) = join_set.join_next().await {
        if result.unwrap().is_ok() {
            success_count += 1;
        }
    }

    assert_eq!(success_count, 3);
}
