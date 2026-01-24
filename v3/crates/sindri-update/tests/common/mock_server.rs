//! Mock server helpers for download testing
//!
//! Provides utilities for setting up wiremock mock servers with
//! common response patterns for binary downloads.

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::constants::*;

/// Set up a mock binary download endpoint
///
/// Creates a GET endpoint at `/sindri-{platform}` that returns the provided content.
pub async fn mock_binary_download(server: &MockServer, platform: &str, content: &[u8]) {
    Mock::given(method("GET"))
        .and(path(format!("/sindri-{}", platform)))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(content))
        .mount(server)
        .await;
}

/// Set up a mock download that fails N times before succeeding
///
/// First `fail_count` requests return 500, subsequent requests return the content.
pub async fn mock_flaky_download(
    server: &MockServer,
    platform: &str,
    fail_count: u64,
    content: &[u8],
) {
    // First N requests fail
    Mock::given(method("GET"))
        .and(path(format!("/sindri-{}", platform)))
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(fail_count)
        .mount(server)
        .await;

    // Subsequent requests succeed
    Mock::given(method("GET"))
        .and(path(format!("/sindri-{}", platform)))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(content))
        .mount(server)
        .await;
}

/// Set up a mock download that always fails with 500
pub async fn mock_failing_download(server: &MockServer, platform: &str) {
    Mock::given(method("GET"))
        .and(path(format!("/sindri-{}", platform)))
        .respond_with(ResponseTemplate::new(500))
        .mount(server)
        .await;
}

/// Set up multiple mock download endpoints with indexed paths
///
/// Creates endpoints at `/sindri-{platform}-{index}` for concurrent download testing.
pub async fn mock_indexed_downloads(server: &MockServer, platform: &str, count: usize) {
    for i in 0..count {
        let content = format!("binary {}", i);
        Mock::given(method("GET"))
            .and(path(format!("/sindri-{}-{}", platform, i)))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(content))
            .mount(server)
            .await;
    }
}

/// Set up a mock binary download with standard fake content
pub async fn mock_standard_download(server: &MockServer, platform: &str) {
    mock_binary_download(server, platform, FAKE_BINARY_CONTENT).await;
}

/// Set up a mock binary download for the default test platform
pub async fn mock_default_platform_download(server: &MockServer, content: &[u8]) {
    mock_binary_download(server, default_test_platform(), content).await;
}
