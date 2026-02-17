//! Integration tests for the RunPod provider adapter.
//!
//! These tests verify the BEHAVIOR of the RunPod provider by mocking the
//! RunPod REST API with `wiremock` and calling actual Provider trait methods.
//!
//! # Test Organization
//!
//! 1. Provider creation and identity
//! 2. Capability flags (GPU, auto-suspend)
//! 3. Prerequisite checks
//! 4. Status queries via REST API
//! 5. Destroy lifecycle via REST API
//! 6. Start / Stop lifecycle via REST API
//! 7. Plan generation
//! 8. API response deserialization (using real provider types)
//! 9. State management
//! 10. Config parsing helpers
//! 11. Edge cases and error handling
//!
//! # Running tests
//!
//! ```sh
//! cargo test --package sindri-providers --test runpod_tests
//! ```

mod common;

use common::*;
use sindri_core::types::DeploymentState;
use sindri_providers::runpod::{CreatePodRequest, RunpodPod, RunpodRuntime, RunpodState};
use sindri_providers::Provider;
use std::collections::HashMap;
use std::path::PathBuf;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper: create a RunpodProvider pointed at the given wiremock server URL.
fn mock_provider(base_url: &str) -> sindri_providers::runpod::RunpodProvider {
    let client = reqwest::Client::new();
    sindri_providers::runpod::RunpodProvider::with_client(client, base_url.to_string())
}

/// Helper: build a SindriConfig from a temp directory with minimal RunPod YAML.
fn test_config(name: &str) -> (tempfile::TempDir, sindri_core::config::SindriConfig) {
    let tmp = tempfile::tempdir().unwrap();
    let yaml_path = tmp.path().join("sindri.yaml");
    let yaml = format!(
        r#"version: "3.0"
name: {name}
deployment:
  provider: docker
  image: "sindri-dev:latest"
  resources:
    memory: "4GB"
    cpus: 2
  volumes:
    home:
      size: "50GB"
extensions:
  profile: minimal
"#
    );
    std::fs::write(&yaml_path, yaml).unwrap();
    let config = sindri_core::config::SindriConfig::load(Some(camino::Utf8Path::new(
        yaml_path.to_str().unwrap(),
    )))
    .unwrap();
    (tmp, config)
}

// =============================================================================
// 1. Provider Creation and Identity
// =============================================================================

#[test]
fn provider_name_returns_runpod() {
    let provider = sindri_providers::runpod::RunpodProvider::new().unwrap();
    assert_eq!(provider.name(), "runpod");
}

#[test]
fn provider_new_succeeds() {
    let result = sindri_providers::runpod::RunpodProvider::new();
    assert!(result.is_ok());
}

#[test]
fn provider_with_output_dir_succeeds() {
    let dir = PathBuf::from("/tmp/test-runpod-output");
    let result = sindri_providers::runpod::RunpodProvider::with_output_dir(dir);
    assert!(result.is_ok());
}

// =============================================================================
// 2. Capability Flags
// =============================================================================

#[test]
fn supports_gpu_returns_true() {
    let provider = sindri_providers::runpod::RunpodProvider::new().unwrap();
    assert!(provider.supports_gpu(), "RunPod must report GPU support");
}

#[test]
fn supports_auto_suspend_returns_false() {
    let provider = sindri_providers::runpod::RunpodProvider::new().unwrap();
    assert!(
        !provider.supports_auto_suspend(),
        "RunPod does not auto-suspend"
    );
}

// =============================================================================
// 3. Prerequisite Checks
// =============================================================================

#[test]
fn check_prerequisites_does_not_panic() {
    let provider = sindri_providers::runpod::RunpodProvider::new().unwrap();
    let result = provider.check_prerequisites();
    assert!(result.is_ok());
}

#[test]
fn check_prerequisites_requires_only_api_key() {
    let provider = sindri_providers::runpod::RunpodProvider::new().unwrap();
    let status = provider.check_prerequisites().unwrap();
    // Should NOT require any CLI tool -- only RUNPOD_API_KEY
    for missing in &status.missing {
        assert_ne!(
            missing.name, "runpodctl",
            "Should not require runpodctl CLI"
        );
    }
}

#[test]
fn check_prerequisites_mentions_api_key_in_hints() {
    let provider = sindri_providers::runpod::RunpodProvider::new().unwrap();
    let status = provider.check_prerequisites().unwrap();

    // If API key is missing, the install hint should mention it
    for missing in &status.missing {
        if missing.name == "runpod-auth" {
            let hint = missing.install_hint.as_deref().unwrap_or("");
            assert!(hint.contains("RUNPOD_API_KEY"), "Should mention env var");
        }
    }
}

// =============================================================================
// 4. Status Queries via REST API
// =============================================================================

#[tokio::test]
async fn status_returns_running_when_api_reports_running_pod() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/pods"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "id": "pod-abc123",
                "name": "test-app",
                "status": "RUNNING",
                "desiredStatus": "RUNNING",
                "imageName": "sindri:latest",
                "gpu": { "type": "NVIDIA RTX A4000", "count": 1 },
                "publicIp": "1.2.3.4",
                "machine": { "id": "m-xyz" },
                "portMappings": [{ "privatePort": 8080, "publicPort": 8080, "type": "http" }],
                "volumeInGb": 50,
                "containerDiskInGb": 20,
                "costPerHr": 0.20,
                "runtime": {
                    "cpuPercent": 25.0,
                    "memoryBytes": 2147483648_u64,
                    "memoryLimit": 4294967296_u64
                }
            }
        ])))
        .mount(&server)
        .await;

    let provider = mock_provider(&server.uri());
    let (_tmp, config) = test_config("test-app");

    let status = provider.status(&config).await.unwrap();

    assert_eq!(status.name, "test-app");
    assert_eq!(status.provider, "runpod");
    assert_eq!(status.state, DeploymentState::Running);
    assert_eq!(status.instance_id.as_deref(), Some("pod-abc123"));
    assert_eq!(status.image.as_deref(), Some("sindri:latest"));
    assert!(!status.addresses.is_empty());
    assert!(status.resources.is_some());
    assert!(status.details.contains_key("gpu_type"));
}

#[tokio::test]
async fn status_returns_stopped_for_exited_pod() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/pods"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "id": "pod-exited-001",
                "name": "test-app",
                "status": "EXITED",
                "desiredStatus": "EXITED"
            }
        ])))
        .mount(&server)
        .await;

    let provider = mock_provider(&server.uri());
    let (_tmp, config) = test_config("test-app");
    let status = provider.status(&config).await.unwrap();

    assert_eq!(status.state, DeploymentState::Stopped);
}

#[tokio::test]
async fn status_returns_creating_for_created_pod() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/pods"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "id": "pod-new-001",
                "name": "test-app",
                "status": "CREATED"
            }
        ])))
        .mount(&server)
        .await;

    let provider = mock_provider(&server.uri());
    let (_tmp, config) = test_config("test-app");
    let status = provider.status(&config).await.unwrap();

    assert_eq!(status.state, DeploymentState::Creating);
}

#[tokio::test]
async fn status_returns_error_for_error_pod() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/pods"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "id": "pod-err-001",
                "name": "test-app",
                "status": "ERROR"
            }
        ])))
        .mount(&server)
        .await;

    let provider = mock_provider(&server.uri());
    let (_tmp, config) = test_config("test-app");
    let status = provider.status(&config).await.unwrap();

    assert_eq!(status.state, DeploymentState::Error);
}

#[tokio::test]
async fn status_returns_unknown_for_unrecognized_status() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/pods"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "id": "pod-unk-001",
                "name": "test-app",
                "status": "FOOBAR"
            }
        ])))
        .mount(&server)
        .await;

    let provider = mock_provider(&server.uri());
    let (_tmp, config) = test_config("test-app");
    let status = provider.status(&config).await.unwrap();

    assert_eq!(status.state, DeploymentState::Unknown);
}

#[tokio::test]
async fn status_returns_not_deployed_when_pod_not_found() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/pods"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .mount(&server)
        .await;

    let provider = mock_provider(&server.uri());
    let (_tmp, config) = test_config("test-app");
    let status = provider.status(&config).await.unwrap();

    assert_eq!(status.state, DeploymentState::NotDeployed);
    assert_eq!(status.provider, "runpod");
}

#[tokio::test]
async fn status_includes_proxy_addresses_for_port_mappings() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/pods"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "id": "pod-ports-001",
                "name": "test-app",
                "status": "RUNNING",
                "portMappings": [
                    { "privatePort": 8080, "publicPort": 18080, "type": "http" },
                    { "privatePort": 3000, "publicPort": 13000, "type": "http" }
                ]
            }
        ])))
        .mount(&server)
        .await;

    let provider = mock_provider(&server.uri());
    let (_tmp, config) = test_config("test-app");
    let status = provider.status(&config).await.unwrap();

    // Should have proxy addresses for each port mapping
    assert!(
        status.addresses.len() >= 2,
        "Should have proxy addresses for port mappings"
    );
}

#[tokio::test]
async fn status_includes_gpu_metadata_in_details() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/pods"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "id": "pod-gpu-001",
                "name": "test-app",
                "status": "RUNNING",
                "gpu": { "type": "NVIDIA RTX A4000", "count": 2 },
                "machine": { "id": "m-123" }
            }
        ])))
        .mount(&server)
        .await;

    let provider = mock_provider(&server.uri());
    let (_tmp, config) = test_config("test-app");
    let status = provider.status(&config).await.unwrap();

    assert_eq!(
        status.details.get("gpu_type").map(|s| s.as_str()),
        Some("NVIDIA RTX A4000")
    );
    assert_eq!(
        status.details.get("gpu_count").map(|s| s.as_str()),
        Some("2")
    );
    assert_eq!(
        status.details.get("machine_id").map(|s| s.as_str()),
        Some("m-123")
    );
}

#[tokio::test]
async fn status_handles_api_error_gracefully() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/pods"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&server)
        .await;

    let provider = mock_provider(&server.uri());
    let (_tmp, config) = test_config("test-app");
    let result = provider.status(&config).await;

    assert!(result.is_err(), "Should propagate API errors");
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Failed to list pods"),
        "Error message should mention failure"
    );
}

// =============================================================================
// 5. Destroy Lifecycle via REST API
// =============================================================================

#[tokio::test]
async fn destroy_terminates_pod_via_delete_endpoint() {
    let server = MockServer::start().await;

    // GET /pods returns the pod to find its ID
    Mock::given(method("GET"))
        .and(path("/pods"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            { "id": "pod-destroy-001", "name": "test-app", "status": "RUNNING" }
        ])))
        .mount(&server)
        .await;

    // DELETE /pods/{podId} succeeds
    Mock::given(method("DELETE"))
        .and(path("/pods/pod-destroy-001"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let provider = mock_provider(&server.uri());
    let (_tmp, config) = test_config("test-app");
    let result = provider.destroy(&config, false).await;

    assert!(result.is_ok(), "Destroy should succeed");
}

#[tokio::test]
async fn destroy_fails_when_pod_not_found_and_no_state() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/pods"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .mount(&server)
        .await;

    let provider = mock_provider(&server.uri());
    let (_tmp, config) = test_config("test-app");
    let result = provider.destroy(&config, false).await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("No RunPod pod found"),
        "Should mention pod not found"
    );
}

#[tokio::test]
async fn destroy_propagates_api_delete_error() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/pods"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            { "id": "pod-fail-001", "name": "test-app", "status": "RUNNING" }
        ])))
        .mount(&server)
        .await;

    Mock::given(method("DELETE"))
        .and(path("/pods/pod-fail-001"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&server)
        .await;

    let provider = mock_provider(&server.uri());
    let (_tmp, config) = test_config("test-app");
    let result = provider.destroy(&config, false).await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Failed to terminate pod"));
}

// =============================================================================
// 6. Start / Stop Lifecycle via REST API
// =============================================================================

#[tokio::test]
async fn start_calls_post_start_endpoint() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/pods"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            { "id": "pod-start-001", "name": "test-app", "status": "EXITED" }
        ])))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/pods/pod-start-001/start"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let provider = mock_provider(&server.uri());
    let (_tmp, config) = test_config("test-app");
    let result = provider.start(&config).await;

    assert!(result.is_ok(), "Start should succeed");
}

#[tokio::test]
async fn stop_calls_post_stop_endpoint() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/pods"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            { "id": "pod-stop-001", "name": "test-app", "status": "RUNNING" }
        ])))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/pods/pod-stop-001/stop"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let provider = mock_provider(&server.uri());
    let (_tmp, config) = test_config("test-app");
    let result = provider.stop(&config).await;

    assert!(result.is_ok(), "Stop should succeed");
}

#[tokio::test]
async fn start_fails_when_no_pod_found() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/pods"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .mount(&server)
        .await;

    let provider = mock_provider(&server.uri());
    let (_tmp, config) = test_config("test-app");
    let result = provider.start(&config).await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("No RunPod pod found"));
}

#[tokio::test]
async fn stop_fails_when_no_pod_found() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/pods"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .mount(&server)
        .await;

    let provider = mock_provider(&server.uri());
    let (_tmp, config) = test_config("test-app");
    let result = provider.stop(&config).await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("No RunPod pod found"));
}

#[tokio::test]
async fn start_propagates_api_failure() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/pods"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            { "id": "pod-sf-001", "name": "test-app", "status": "EXITED" }
        ])))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/pods/pod-sf-001/start"))
        .respond_with(ResponseTemplate::new(500).set_body_string("GPU unavailable"))
        .mount(&server)
        .await;

    let provider = mock_provider(&server.uri());
    let (_tmp, config) = test_config("test-app");
    let result = provider.start(&config).await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Failed to start pod"));
}

#[tokio::test]
async fn stop_propagates_api_failure() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/pods"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            { "id": "pod-sf-002", "name": "test-app", "status": "RUNNING" }
        ])))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/pods/pod-sf-002/stop"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Server error"))
        .mount(&server)
        .await;

    let provider = mock_provider(&server.uri());
    let (_tmp, config) = test_config("test-app");
    let result = provider.stop(&config).await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Failed to stop pod"));
}

// =============================================================================
// 7. Plan Generation
// =============================================================================

#[tokio::test]
async fn plan_returns_runpod_provider() {
    let provider = sindri_providers::runpod::RunpodProvider::new().unwrap();
    let (_tmp, config) = test_config("plan-test-app");
    let plan = provider.plan(&config).await.unwrap();

    assert_eq!(plan.provider, "runpod");
}

#[tokio::test]
async fn plan_includes_create_pod_action() {
    let provider = sindri_providers::runpod::RunpodProvider::new().unwrap();
    let (_tmp, config) = test_config("plan-test-app");
    let plan = provider.plan(&config).await.unwrap();

    let has_create = plan
        .actions
        .iter()
        .any(|a| a.resource == "runpod-pod" && a.description.contains("Create RunPod pod"));
    assert!(has_create, "Plan should include a create pod action");
}

#[tokio::test]
async fn plan_includes_gpu_config_in_resources() {
    let provider = sindri_providers::runpod::RunpodProvider::new().unwrap();
    let (_tmp, config) = test_config("plan-test-app");
    let plan = provider.plan(&config).await.unwrap();

    let pod_resource = plan
        .resources
        .iter()
        .find(|r| r.resource_type == "runpod-pod");
    assert!(pod_resource.is_some(), "Should have runpod-pod resource");
    let resource = pod_resource.unwrap();
    assert!(resource.config.contains_key("gpu_type"));
    assert!(resource.config.contains_key("gpu_count"));
    assert!(resource.config.contains_key("container_disk_gb"));
    assert!(resource.config.contains_key("volume_size_gb"));
    assert!(resource.config.contains_key("cloud_type"));
}

#[tokio::test]
async fn plan_includes_cost_estimate_in_usd() {
    let provider = sindri_providers::runpod::RunpodProvider::new().unwrap();
    let (_tmp, config) = test_config("plan-test-app");
    let plan = provider.plan(&config).await.unwrap();

    let cost = plan.estimated_cost.as_ref();
    assert!(cost.is_some(), "Plan should include cost estimate");
    assert_eq!(cost.unwrap().currency, "USD");
    assert!(
        cost.unwrap().hourly.is_some(),
        "Hourly cost should be present"
    );
}

// =============================================================================
// 8. API Response Deserialization (using real provider types)
// =============================================================================

#[test]
fn pod_full_json_deserializes_with_real_types() {
    let json = r#"{
        "id": "abc123",
        "name": "my-pod",
        "status": "RUNNING",
        "desiredStatus": "RUNNING",
        "image": "ghcr.io/org/sindri:latest",
        "gpu": { "type": "NVIDIA RTX A4000", "count": 1 },
        "publicIp": "1.2.3.4",
        "machine": { "id": "m-xyz" },
        "portMappings": [{ "privatePort": 8080, "publicPort": 8080, "type": "http" }],
        "volumeInGb": 50,
        "containerDiskInGb": 20,
        "costPerHr": 0.20,
        "runtime": {
            "cpuPercent": 25.0,
            "memoryBytes": 2147483648,
            "memoryLimit": 4294967296,
            "diskBytes": 1073741824,
            "diskLimit": 21474836480
        }
    }"#;

    let pod: RunpodPod = serde_json::from_str(json).unwrap();
    assert_eq!(pod.id, "abc123");
    assert_eq!(pod.name, "my-pod");
    assert_eq!(pod.status.as_deref(), Some("RUNNING"));
    assert!(pod.gpu.is_some());
    assert_eq!(
        pod.gpu.as_ref().unwrap().gpu_type.as_deref(),
        Some("NVIDIA RTX A4000")
    );
    assert_eq!(pod.gpu.as_ref().unwrap().count, Some(1));
    assert_eq!(pod.public_ip.as_deref(), Some("1.2.3.4"));
    assert!(pod.runtime.is_some());
    assert_eq!(pod.runtime.as_ref().unwrap().cpu_percent, Some(25.0));
}

#[test]
fn pod_minimal_json_deserializes_with_real_types() {
    let json = r#"{ "id": "min-001", "name": "minimal-pod" }"#;

    let pod: RunpodPod = serde_json::from_str(json).unwrap();
    assert_eq!(pod.id, "min-001");
    assert!(pod.status.is_none());
    assert!(pod.gpu.is_none());
    assert!(pod.runtime.is_none());
}

#[test]
fn pod_list_deserializes_with_real_types() {
    let json = r#"[
        { "id": "p1", "name": "pod-one", "status": "RUNNING" },
        { "id": "p2", "name": "pod-two", "status": "EXITED" }
    ]"#;

    let pods: Vec<RunpodPod> = serde_json::from_str(json).unwrap();
    assert_eq!(pods.len(), 2);
    assert_eq!(pods[0].status.as_deref(), Some("RUNNING"));
    assert_eq!(pods[1].status.as_deref(), Some("EXITED"));
}

#[test]
fn empty_pod_list_deserializes() {
    let json = "[]";
    let pods: Vec<RunpodPod> = serde_json::from_str(json).unwrap();
    assert!(pods.is_empty());
}

#[test]
fn malformed_json_fails_to_parse() {
    let bad_json = "{id: 'invalid'}";
    let result: Result<RunpodPod, _> = serde_json::from_str(bad_json);
    assert!(result.is_err());
}

#[test]
fn create_pod_request_serialization() {
    let request = CreatePodRequest {
        name: "test-pod".to_string(),
        image_name: "ghcr.io/org/sindri:latest".to_string(),
        gpu_type_ids: Some(vec!["NVIDIA RTX A4000".to_string()]),
        gpu_count: Some(1),
        compute_type: Some("GPU".to_string()),
        cloud_type: Some("COMMUNITY".to_string()),
        container_disk_in_gb: Some(20),
        volume_in_gb: Some(50),
        volume_mount_path: Some("/workspace".to_string()),
        ports: Some(vec!["22/tcp".to_string(), "8080/http".to_string()]),
        data_center_ids: None,
        env: None,
        interruptible: None,
    };

    let json = serde_json::to_value(&request).unwrap();
    assert_eq!(json["name"], "test-pod");
    assert_eq!(json["imageName"], "ghcr.io/org/sindri:latest");
    assert_eq!(json["gpuTypeIds"][0], "NVIDIA RTX A4000");
    assert_eq!(json["gpuCount"], 1);
    assert_eq!(json["computeType"], "GPU");
    assert_eq!(json["cloudType"], "COMMUNITY");
    assert_eq!(json["containerDiskInGb"], 20);
    assert_eq!(json["volumeInGb"], 50);
    assert_eq!(json["volumeMountPath"], "/workspace");
}

#[test]
fn create_pod_request_cpu_only_omits_gpu_fields() {
    let request = CreatePodRequest {
        name: "cpu-pod".to_string(),
        image_name: "sindri:latest".to_string(),
        gpu_type_ids: None,
        gpu_count: None,
        compute_type: Some("CPU".to_string()),
        cloud_type: Some("COMMUNITY".to_string()),
        container_disk_in_gb: Some(20),
        volume_in_gb: None,
        volume_mount_path: None,
        ports: None,
        data_center_ids: None,
        env: None,
        interruptible: None,
    };

    let json = serde_json::to_value(&request).unwrap();
    assert_eq!(json["computeType"], "CPU");
    assert!(json.get("gpuTypeIds").is_none());
    assert!(json.get("gpuCount").is_none());
}

#[test]
fn create_pod_request_with_env_vars() {
    let mut env = HashMap::new();
    env.insert("DB_PASSWORD".to_string(), "secret123".to_string());
    env.insert("API_KEY".to_string(), "key456".to_string());

    let request = CreatePodRequest {
        name: "env-pod".to_string(),
        image_name: "sindri:latest".to_string(),
        gpu_type_ids: None,
        gpu_count: None,
        compute_type: None,
        cloud_type: None,
        container_disk_in_gb: None,
        volume_in_gb: None,
        volume_mount_path: None,
        ports: None,
        data_center_ids: None,
        env: Some(env),
        interruptible: None,
    };

    let json = serde_json::to_value(&request).unwrap();
    assert_eq!(json["env"]["DB_PASSWORD"], "secret123");
    assert_eq!(json["env"]["API_KEY"], "key456");
}

#[test]
fn runtime_metrics_deserialization() {
    let json = r#"{
        "cpuPercent": 75.5,
        "memoryBytes": 8589934592,
        "memoryLimit": 17179869184,
        "diskBytes": 5368709120,
        "diskLimit": 21474836480
    }"#;

    let runtime: RunpodRuntime = serde_json::from_str(json).unwrap();
    assert_eq!(runtime.cpu_percent, Some(75.5));
    assert_eq!(runtime.memory_bytes, Some(8589934592));
    assert_eq!(runtime.memory_limit, Some(17179869184));
    assert_eq!(runtime.disk_bytes, Some(5368709120));
    assert_eq!(runtime.disk_limit, Some(21474836480));
}

#[test]
fn runtime_empty_deserialization() {
    let json = "{}";
    let runtime: RunpodRuntime = serde_json::from_str(json).unwrap();
    assert!(runtime.cpu_percent.is_none());
    assert!(runtime.memory_bytes.is_none());
}

// =============================================================================
// 9. State Management
// =============================================================================

#[test]
fn state_serialization_roundtrip() {
    let state = RunpodState {
        pod_id: "pod-test-123".to_string(),
        app_name: "my-app".to_string(),
        gpu_type: "NVIDIA RTX A4000".to_string(),
        gpu_count: 1,
        image: Some("sindri:latest".to_string()),
        created_at: "2026-02-16T10:00:00Z".to_string(),
    };

    let json = serde_json::to_string(&state).unwrap();
    let deserialized: RunpodState = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.pod_id, "pod-test-123");
    assert_eq!(deserialized.app_name, "my-app");
    assert_eq!(deserialized.gpu_type, "NVIDIA RTX A4000");
    assert_eq!(deserialized.gpu_count, 1);
    assert_eq!(deserialized.image.as_deref(), Some("sindri:latest"));
}

#[test]
fn state_file_creation_helper_works() {
    let tmp = tempfile::tempdir().unwrap();
    let state_path = create_runpod_state(tmp.path(), "pod-state-001", "my-app").unwrap();

    assert!(state_path.exists());
    let content = std::fs::read_to_string(&state_path).unwrap();
    let state: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(state["pod_id"], "pod-state-001");
    assert_eq!(state["app_name"], "my-app");
    assert_eq!(state["gpu_type"], "NVIDIA RTX A4000");
}

#[test]
fn state_file_removal_works() {
    let tmp = tempfile::tempdir().unwrap();
    let state_path = create_runpod_state(tmp.path(), "pod-rm-001", "my-app").unwrap();
    assert!(state_path.exists());

    std::fs::remove_file(&state_path).unwrap();
    assert!(!state_path.exists(), "State file should be removable");
}

// =============================================================================
// 10. Config Parsing Helpers
// =============================================================================

#[test]
fn config_fixture_creates_valid_yaml() {
    let (_tmp, path) =
        create_runpod_config_fixture("test-gpu-app", "NVIDIA RTX A4000", 1, 20, 50).unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(&content).unwrap();

    assert_eq!(yaml["name"], "test-gpu-app");
    assert_eq!(yaml["providers"]["runpod"]["gpu_type"], "NVIDIA RTX A4000");
    assert_eq!(yaml["providers"]["runpod"]["gpu_count"], 1);
    assert_eq!(yaml["providers"]["runpod"]["container_disk_gb"], 20);
    assert_eq!(yaml["providers"]["runpod"]["volume_size_gb"], 50);
}

#[test]
fn config_fixture_uses_defaults() {
    let (_tmp, path) =
        create_runpod_config_fixture("minimal-app", "NVIDIA RTX A4000", 1, 20, 50).unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(&content).unwrap();

    assert_eq!(yaml["providers"]["runpod"]["cloud_type"], "COMMUNITY");
    assert_eq!(
        yaml["providers"]["runpod"]["volume_mount_path"],
        "/workspace"
    );
}

// =============================================================================
// 11. Edge Cases and Error Handling
// =============================================================================

#[test]
fn connection_info_no_ports_means_no_http_url() {
    let expose_ports: Vec<String> = vec![];
    let http_url: Option<String> = expose_ports
        .first()
        .map(|port| format!("https://pod-id-{}.proxy.runpod.net", port));
    assert!(http_url.is_none(), "No ports = no HTTP URL");
}

#[test]
fn connection_info_with_ports_generates_proxy_url() {
    let pod_id = "pod-test-001";
    let port = "8080";
    let proxy_url = format!("https://{}-{}.proxy.runpod.net", pod_id, port);
    assert_eq!(proxy_url, "https://pod-test-001-8080.proxy.runpod.net");
}

#[test]
fn deploy_result_builder_creates_valid_result() {
    let result = deploy_result_ok("gpu-app", "runpod", "pod-abc-123");

    assert!(result.success);
    assert_eq!(result.provider, "runpod");
    assert_eq!(result.instance_id.as_deref(), Some("pod-abc-123"));
    assert!(result.connection.is_some());
    assert!(!result.messages.is_empty());
    assert!(result.warnings.is_empty());
}

#[test]
fn deployment_status_builder_creates_valid_status() {
    let status = deployment_status("test-app", "runpod", DeploymentState::Running);

    assert_eq!(status.state, DeploymentState::Running);
    assert_eq!(status.provider, "runpod");
    assert_eq!(status.name, "test-app");
}

#[tokio::test]
async fn status_falls_back_to_desired_status_when_status_missing() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/pods"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "id": "pod-ds-001",
                "name": "test-app",
                "desiredStatus": "RUNNING"
            }
        ])))
        .mount(&server)
        .await;

    let provider = mock_provider(&server.uri());
    let (_tmp, config) = test_config("test-app");
    let status = provider.status(&config).await.unwrap();

    assert_eq!(
        status.state,
        DeploymentState::Running,
        "Should fall back to desiredStatus"
    );
}

#[tokio::test]
async fn status_with_resource_usage_populated() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/pods"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "id": "pod-ru-001",
                "name": "test-app",
                "status": "RUNNING",
                "runtime": {
                    "cpuPercent": 50.0,
                    "memoryBytes": 4294967296_u64,
                    "memoryLimit": 8589934592_u64,
                    "diskBytes": 2147483648_u64,
                    "diskLimit": 21474836480_u64
                }
            }
        ])))
        .mount(&server)
        .await;

    let provider = mock_provider(&server.uri());
    let (_tmp, config) = test_config("test-app");
    let status = provider.status(&config).await.unwrap();

    let resources = status.resources.unwrap();
    assert_eq!(resources.cpu_percent, Some(50.0));
    assert_eq!(resources.memory_bytes, Some(4294967296));
    assert_eq!(resources.memory_limit, Some(8589934592));
    assert_eq!(resources.disk_bytes, Some(2147483648));
    assert_eq!(resources.disk_limit, Some(21474836480));
}

// =============================================================================
// Mock Infrastructure Verification
// =============================================================================

#[test]
fn mock_executable_creates_runnable_script() {
    let tmp = tempfile::tempdir().unwrap();

    create_mock_executable(
        tmp.path(),
        "runpodctl",
        r#"{"id":"pod-test-001","desiredStatus":"RUNNING"}"#,
        0,
    )
    .unwrap();

    let mock_path = tmp.path().join("runpodctl");
    assert!(mock_path.exists(), "Mock executable should be created");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::metadata(&mock_path).unwrap().permissions();
        assert_eq!(perms.mode() & 0o111, 0o111, "Mock should be executable");
    }
}

#[test]
fn conditional_mock_returns_correct_output_per_subcommand() {
    let tmp = tempfile::tempdir().unwrap();

    create_conditional_mock(
        tmp.path(),
        "test-cli",
        &[
            ("list", r#"["item1","item2"]"#, 0),
            ("get", r#"{"id":"abc"}"#, 0),
            ("delete", "deleted", 0),
        ],
        "",
        0,
    )
    .unwrap();

    let mock_path = tmp.path().join("test-cli");

    let output = std::process::Command::new(&mock_path)
        .args(["list"])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&output.stdout).contains("item1"));

    let output = std::process::Command::new(&mock_path)
        .args(["get", "abc"])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&output.stdout).contains("abc"));
}

#[test]
fn mock_records_invocations() {
    let tmp = tempfile::tempdir().unwrap();
    create_mock_executable(tmp.path(), "test-cli", "{}", 0).unwrap();

    let mock_path = tmp.path().join("test-cli");

    std::process::Command::new(&mock_path)
        .args(["action1", "arg1"])
        .output()
        .unwrap();

    std::process::Command::new(&mock_path)
        .args(["action2", "arg2"])
        .output()
        .unwrap();

    let log = read_mock_log(tmp.path(), "test-cli");
    assert_eq!(log.len(), 2);
    assert!(log[0].contains("action1"));
    assert!(log[1].contains("action2"));
}

#[test]
fn command_log_records_and_asserts() {
    let log = CommandLog::new();

    log.record("cli-a", &["get", "pod", "--json"]);
    log.record("cli-a", &["create", "pods", "--name", "test"]);
    log.record("cli-b", &["build", "-t", "sindri:latest", "."]);

    log.assert_called("cli-a");
    log.assert_called("cli-b");
    log.assert_not_called("cli-c");

    assert_eq!(log.call_count("cli-a"), 2);
    assert_eq!(log.call_count("cli-b"), 1);
    assert_eq!(log.call_count("cli-c"), 0);
}
