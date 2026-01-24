//! Kind integration tests
//!
//! These tests require kind and Docker to be installed and running.
//! Run with: cargo test --test kind_integration -- --ignored

mod common;

use common::{
    cleanup_cluster, skip_if_no_docker, skip_if_not_installed, test_cluster_name, test_config,
};
use sindri_clusters::{ClusterProvider, ClusterState, KindProvider};

/// Test kind cluster creation and basic operations
#[tokio::test]
#[ignore] // Requires Docker and kind
async fn test_kind_cluster_lifecycle() {
    let provider = KindProvider::new();

    if skip_if_not_installed(&provider) || skip_if_no_docker(&provider) {
        return;
    }

    let cluster_name = test_cluster_name("kind-test");
    let config = test_config(&cluster_name);

    // Cleanup any existing cluster
    cleanup_cluster(&provider, &cluster_name).await;

    // Create cluster
    let info = provider
        .create(&config)
        .await
        .expect("Failed to create kind cluster");

    assert_eq!(info.name, cluster_name);
    assert_eq!(info.provider, "kind");
    assert_eq!(info.context, format!("kind-{}", cluster_name));

    // Verify cluster exists
    assert!(provider.exists(&cluster_name).await);

    // Check cluster is in list
    let clusters = provider.list().await.expect("Failed to list clusters");
    assert!(clusters.iter().any(|c| c.name == cluster_name));

    // Check status
    let status = provider
        .status(&cluster_name)
        .await
        .expect("Failed to get status");

    assert_eq!(status.name, cluster_name);
    assert!(status.ready);
    assert!(matches!(status.state, ClusterState::Running));

    // Get kubeconfig
    let kubeconfig = provider
        .get_kubeconfig(&cluster_name)
        .await
        .expect("Failed to get kubeconfig");

    assert!(kubeconfig.contains(&cluster_name));
    assert!(kubeconfig.contains("clusters:"));

    // Cleanup
    provider
        .destroy(&cluster_name, true)
        .await
        .expect("Failed to destroy cluster");

    // Verify cluster is gone
    assert!(!provider.exists(&cluster_name).await);
}

/// Test that creating a cluster that already exists returns the existing info
#[tokio::test]
#[ignore] // Requires Docker and kind
async fn test_kind_cluster_already_exists() {
    let provider = KindProvider::new();

    if skip_if_not_installed(&provider) || skip_if_no_docker(&provider) {
        return;
    }

    let cluster_name = test_cluster_name("kind-exists");
    let config = test_config(&cluster_name);

    // Cleanup any existing cluster
    cleanup_cluster(&provider, &cluster_name).await;

    // Create cluster first time
    let info1 = provider
        .create(&config)
        .await
        .expect("Failed to create cluster");
    assert_eq!(info1.name, cluster_name);

    // Create cluster second time (should succeed and return existing)
    let info2 = provider
        .create(&config)
        .await
        .expect("Second create should succeed");
    assert_eq!(info2.name, cluster_name);

    // Cleanup
    provider.destroy(&cluster_name, true).await.ok();
}

/// Test context name generation
#[test]
fn test_kind_context_name() {
    let provider = KindProvider::new();

    assert_eq!(provider.context_name("my-cluster"), "kind-my-cluster");
    assert_eq!(provider.context_name("test"), "kind-test");
    assert_eq!(provider.context_name("sindri-local"), "kind-sindri-local");
}

/// Test version detection
#[test]
fn test_kind_version() {
    let provider = KindProvider::new();

    if !provider.check_installed() {
        eprintln!("Skipping: kind not installed");
        return;
    }

    let version = provider.get_version();
    assert!(version.is_some(), "Should return version when installed");

    let v = version.unwrap();
    assert!(v.contains("kind"), "Version should contain 'kind'");
}
