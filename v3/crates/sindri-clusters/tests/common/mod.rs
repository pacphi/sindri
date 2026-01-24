//! Shared test utilities for sindri-clusters integration tests

use sindri_clusters::{ClusterConfig, ClusterProvider};

/// Generate a unique cluster name for tests
pub fn test_cluster_name(prefix: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("{}-{}", prefix, timestamp % 10000)
}

/// Skip test if provider is not installed
pub fn skip_if_not_installed<P: ClusterProvider>(provider: &P) -> bool {
    if !provider.check_installed() {
        eprintln!("Skipping test: {} not installed", provider.name());
        true
    } else {
        false
    }
}

/// Skip test if Docker is not running
pub fn skip_if_no_docker<P: ClusterProvider>(provider: &P) -> bool {
    if !provider.check_docker() {
        eprintln!("Skipping test: Docker not running");
        true
    } else {
        false
    }
}

/// Create a test cluster configuration
pub fn test_config(name: &str) -> ClusterConfig {
    ClusterConfig::new(name)
        .with_version("v1.31.0") // Use stable version for tests
        .with_nodes(1)
}

/// Cleanup helper - delete cluster if it exists
pub async fn cleanup_cluster<P: ClusterProvider>(provider: &P, name: &str) {
    if provider.exists(name).await {
        let _ = provider.destroy(name, true).await;
    }
}
