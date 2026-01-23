//! K3d integration tests
//!
//! These tests require k3d and Docker to be installed and running.
//! Run with: cargo test --test k3d_integration -- --ignored

mod common;

use common::{
    cleanup_cluster, skip_if_no_docker, skip_if_not_installed, test_cluster_name, test_config,
};
use sindri_clusters::{
    ClusterConfig, ClusterProvider, ClusterState, K3dConfig, K3dProvider, K3dRegistryConfig,
};

/// Test k3d cluster creation and basic operations
#[tokio::test]
#[ignore] // Requires Docker and k3d
async fn test_k3d_cluster_lifecycle() {
    let provider = K3dProvider::new();

    if skip_if_not_installed(&provider) || skip_if_no_docker(&provider) {
        return;
    }

    let cluster_name = test_cluster_name("k3d-test");
    let config = test_config(&cluster_name);

    // Cleanup any existing cluster
    cleanup_cluster(&provider, &cluster_name).await;

    // Create cluster
    let info = provider
        .create(&config)
        .await
        .expect("Failed to create k3d cluster");

    assert_eq!(info.name, cluster_name);
    assert_eq!(info.provider, "k3d");
    assert_eq!(info.context, format!("k3d-{}", cluster_name));

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

/// Test k3d cluster with registry
#[tokio::test]
#[ignore] // Requires Docker and k3d
async fn test_k3d_cluster_with_registry() {
    let provider = K3dProvider::new();

    if skip_if_not_installed(&provider) || skip_if_no_docker(&provider) {
        return;
    }

    let cluster_name = test_cluster_name("k3d-reg");
    let config = ClusterConfig::new(&cluster_name)
        .with_version("v1.31.0")
        .with_nodes(1)
        .with_k3d_config(K3dConfig {
            image: None,
            registry: K3dRegistryConfig {
                enabled: true,
                name: format!("{}-registry", cluster_name),
                port: 5111, // Use non-standard port to avoid conflicts
            },
        });

    // Cleanup any existing cluster
    cleanup_cluster(&provider, &cluster_name).await;

    // Create cluster with registry
    let info = provider
        .create(&config)
        .await
        .expect("Failed to create k3d cluster with registry");

    assert_eq!(info.name, cluster_name);
    assert!(info.registry_url.is_some(), "Should have registry URL");
    assert!(info.registry_url.as_ref().unwrap().contains("5111"));

    // Cleanup
    provider.destroy(&cluster_name, true).await.ok();
}

/// Test context name generation
#[test]
fn test_k3d_context_name() {
    let provider = K3dProvider::new();

    assert_eq!(provider.context_name("my-cluster"), "k3d-my-cluster");
    assert_eq!(provider.context_name("test"), "k3d-test");
    assert_eq!(provider.context_name("sindri-local"), "k3d-sindri-local");
}

/// Test version detection
#[test]
fn test_k3d_version() {
    let provider = K3dProvider::new();

    if !provider.check_installed() {
        eprintln!("Skipping: k3d not installed");
        return;
    }

    let version = provider.get_version();
    assert!(version.is_some(), "Should return version when installed");

    let v = version.unwrap();
    assert!(v.contains("k3d"), "Version should contain 'k3d'");
}

/// Test JSON output parsing
#[test]
fn test_k3d_json_parsing() {
    // Test that our JSON parsing handles various formats correctly
    let json = r#"[
        {"name": "test-cluster", "serversCount": 1, "agentsCount": 2},
        {"name": "another-cluster", "serversCount": 1, "agentsCount": 0}
    ]"#;

    #[derive(serde::Deserialize)]
    struct K3dClusterInfo {
        name: String,
        #[serde(rename = "serversCount", default)]
        servers_count: u32,
        #[serde(rename = "agentsCount", default)]
        agents_count: u32,
    }

    let clusters: Vec<K3dClusterInfo> = serde_json::from_str(json).unwrap();

    assert_eq!(clusters.len(), 2);
    assert_eq!(clusters[0].name, "test-cluster");
    assert_eq!(clusters[0].servers_count, 1);
    assert_eq!(clusters[0].agents_count, 2);
}
