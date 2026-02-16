//! Northflank provider test specifications -- London School TDD (Red phase)
//!
//! These tests define the expected behavior of the NorthflankProvider before
//! implementation exists. They follow the London School (mockist) approach:
//!
//! - Mock ALL external dependencies (northflank CLI, filesystem, network)
//! - Test behavior and interactions, not implementation details
//! - Focus on object collaborations and message passing
//! - Define contracts through mock expectations
//!
//! Run with: cargo test --package sindri-providers --test northflank_tests
//!
//! All tests are expected to FAIL initially (Red phase).
//! Implementation will make them pass (Green phase).
//!
//! Test categories (42 tests total):
//!   1.  Provider creation (3 tests)
//!   2.  Capability flags (2 tests)
//!   3.  Prerequisite checks (4 tests)
//!   4.  API response deserialization (6 tests)
//!   5.  Status mapping (1 test)
//!   6.  Compute plan mapping (6 tests)
//!   7.  GPU tier mapping (5 tests)
//!   8.  Deploy lifecycle -- async with mocked CLI (4 tests)
//!   9.  Status queries -- async with mocked CLI (3 tests)
//!   10. Connect -- async with mocked CLI (3 tests)
//!   11. Destroy -- async with mocked CLI (3 tests)
//!   12. Start/Stop -- async with mocked CLI (3 tests)
//!   13. Plan (dry-run) -- async (2 tests)
//!   14. Service definition builder (3 tests)
//!   15. Secret groups -- async (1 test)
//!   16. Config/state fixtures (2 tests)
//!   17. Factory integration (1 test)

mod common;

use common::{
    create_conditional_mock, create_mock_executable, create_northflank_config_fixture,
    create_northflank_state, read_mock_log,
};
use serde_json::json;
use serial_test::serial;
use sindri_core::types::{DeployOptions, DeploymentState};
use std::collections::HashMap;
use std::path::PathBuf;

// ═══════════════════════════════════════════════════════════════════════════════
// 1. Provider Creation
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn provider_new_succeeds() {
    let provider = sindri_providers::northflank::NorthflankProvider::new();
    assert!(provider.is_ok(), "NorthflankProvider::new() should succeed");
}

#[test]
fn provider_with_output_dir_succeeds() {
    let dir = PathBuf::from("/tmp/test-northflank-output");
    let provider = sindri_providers::northflank::NorthflankProvider::with_output_dir(dir);
    assert!(provider.is_ok(), "with_output_dir() should succeed");
}

#[test]
fn provider_name_returns_northflank() {
    use sindri_providers::traits::Provider;
    let provider = sindri_providers::northflank::NorthflankProvider::new().unwrap();
    assert_eq!(provider.name(), "northflank");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 2. Capability Flags
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn supports_gpu_returns_true() {
    use sindri_providers::traits::Provider;
    let provider = sindri_providers::northflank::NorthflankProvider::new().unwrap();
    assert!(provider.supports_gpu(), "Northflank supports GPU workloads");
}

#[test]
fn supports_auto_suspend_returns_true() {
    use sindri_providers::traits::Provider;
    let provider = sindri_providers::northflank::NorthflankProvider::new().unwrap();
    assert!(
        provider.supports_auto_suspend(),
        "Northflank supports pause/resume"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// 3. Prerequisite Checks
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn check_prerequisites_does_not_panic() {
    use sindri_providers::traits::Provider;
    let provider = sindri_providers::northflank::NorthflankProvider::new().unwrap();
    let result = provider.check_prerequisites();
    assert!(
        result.is_ok(),
        "check_prerequisites() must not panic: {:?}",
        result.err()
    );
}

#[test]
#[serial]
fn check_prerequisites_satisfied_with_cli_and_auth() {
    use sindri_providers::traits::Provider;

    let tmp = tempfile::tempdir().unwrap();
    create_mock_executable(tmp.path(), "northflank", "1.2.3", 0).unwrap();

    let original_path = std::env::var("PATH").unwrap_or_default();
    std::env::set_var(
        "PATH",
        format!("{}:{}", tmp.path().display(), original_path),
    );
    std::env::set_var("NORTHFLANK_API_TOKEN", "test-token-123");

    let provider = sindri_providers::northflank::NorthflankProvider::new().unwrap();
    let result = provider.check_prerequisites().unwrap();

    std::env::set_var("PATH", &original_path);
    std::env::remove_var("NORTHFLANK_API_TOKEN");

    assert!(
        result.satisfied,
        "With CLI + auth token, prerequisites should be satisfied. Missing: {:?}",
        result.missing
    );
}

#[test]
#[serial]
fn check_prerequisites_cli_missing_shows_npm_hint() {
    use sindri_providers::traits::Provider;

    let original_path = std::env::var("PATH").unwrap_or_default();
    std::env::set_var("PATH", "/nonexistent");

    let provider = sindri_providers::northflank::NorthflankProvider::new().unwrap();
    let result = provider.check_prerequisites().unwrap();

    std::env::set_var("PATH", &original_path);

    assert!(
        !result.satisfied,
        "Without CLI, prerequisites should not be satisfied"
    );
    let cli_missing = result.missing.iter().find(|p| p.name == "northflank");
    assert!(
        cli_missing.is_some(),
        "Missing list should contain 'northflank'"
    );
    let hint = cli_missing.unwrap().install_hint.as_deref().unwrap_or("");
    assert!(
        hint.contains("npm"),
        "Install hint should mention npm: got '{}'",
        hint
    );
}

#[test]
#[serial]
fn check_prerequisites_no_auth_shows_login_hint() {
    use sindri_providers::traits::Provider;

    let tmp = tempfile::tempdir().unwrap();
    create_conditional_mock(
        tmp.path(),
        "northflank",
        &[
            ("--version", "1.2.3", 0),
            ("list projects", "Unauthorized", 1),
        ],
        "Unauthorized",
        1,
    )
    .unwrap();

    let original_path = std::env::var("PATH").unwrap_or_default();
    std::env::set_var(
        "PATH",
        format!("{}:{}", tmp.path().display(), original_path),
    );
    let original_token = std::env::var("NORTHFLANK_API_TOKEN").ok();
    std::env::remove_var("NORTHFLANK_API_TOKEN");

    let provider = sindri_providers::northflank::NorthflankProvider::new().unwrap();
    let result = provider.check_prerequisites().unwrap();

    std::env::set_var("PATH", &original_path);
    if let Some(t) = original_token {
        std::env::set_var("NORTHFLANK_API_TOKEN", t);
    }

    let auth_missing = result.missing.iter().find(|p| p.name.contains("auth"));
    assert!(
        auth_missing.is_some(),
        "Missing list should contain auth prerequisite. Missing: {:?}",
        result.missing
    );
    let hint = auth_missing.unwrap().install_hint.as_deref().unwrap_or("");
    assert!(
        hint.contains("northflank login") || hint.contains("NORTHFLANK_API_TOKEN"),
        "Auth hint should mention login or token: got '{}'",
        hint
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// 4. API Response Deserialization
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn deserialize_service_response_full() {
    let json = r#"{
        "id": "svc-abc123", "name": "my-service", "status": "running",
        "image": "ghcr.io/org/sindri:latest", "computePlan": "nf-compute-50", "instances": 1,
        "ports": [
            {"name": "ssh", "internalPort": 22, "public": false, "dns": null},
            {"name": "http", "internalPort": 8080, "public": true, "dns": "my-service.example.northflank.app"}
        ],
        "metrics": {"cpuPercent": 15.5, "memoryBytes": 1073741824, "memoryLimit": 4294967296, "diskBytes": null, "diskLimit": null}
    }"#;

    let svc: sindri_providers::northflank::NorthflankService = serde_json::from_str(json).unwrap();
    assert_eq!(svc.id, "svc-abc123");
    assert_eq!(svc.name, "my-service");
    assert_eq!(svc.status, "running");
    assert_eq!(svc.image, Some("ghcr.io/org/sindri:latest".to_string()));
    assert_eq!(svc.compute_plan, "nf-compute-50");
    assert_eq!(svc.instances, 1);
    assert_eq!(svc.ports.len(), 2);
    assert!(!svc.ports[0].public);
    assert!(svc.ports[1].public);
    assert_eq!(
        svc.ports[1].dns.as_deref(),
        Some("my-service.example.northflank.app")
    );
    assert!(svc.metrics.is_some());
}

#[test]
fn deserialize_service_response_minimal() {
    let json = r#"{"id": "svc-min", "name": "min", "status": "creating", "image": null, "computePlan": "nf-compute-10", "instances": 1, "ports": [], "metrics": null}"#;
    let svc: sindri_providers::northflank::NorthflankService = serde_json::from_str(json).unwrap();
    assert_eq!(svc.status, "creating");
    assert!(svc.image.is_none());
    assert!(svc.metrics.is_none());
}

#[test]
fn deserialize_service_list() {
    let json = r#"[
        {"id": "svc-1", "name": "web", "status": "running", "image": null, "computePlan": "nf-compute-20", "instances": 1, "ports": [], "metrics": null},
        {"id": "svc-2", "name": "worker", "status": "paused", "image": null, "computePlan": "nf-compute-50", "instances": 2, "ports": [], "metrics": null}
    ]"#;
    let svcs: Vec<sindri_providers::northflank::NorthflankService> =
        serde_json::from_str(json).unwrap();
    assert_eq!(svcs.len(), 2);
    assert_eq!(svcs[0].status, "running");
    assert_eq!(svcs[1].status, "paused");
}

#[test]
fn deserialize_port_with_dns() {
    let json = r#"{"name": "web", "internalPort": 3000, "public": true, "dns": "web.nf.app"}"#;
    let port: sindri_providers::northflank::NorthflankServicePort =
        serde_json::from_str(json).unwrap();
    assert_eq!(port.internal_port, 3000);
    assert!(port.public);
    assert_eq!(port.dns.as_deref(), Some("web.nf.app"));
}

#[test]
fn deserialize_metrics_partial() {
    let json = r#"{"cpuPercent": 42.7, "memoryBytes": 2147483648, "memoryLimit": 4294967296, "diskBytes": null, "diskLimit": null}"#;
    let m: sindri_providers::northflank::NorthflankMetrics = serde_json::from_str(json).unwrap();
    assert_eq!(m.cpu_percent, Some(42.7));
    assert!(m.disk_bytes.is_none());
}

#[test]
fn deserialize_project_response() {
    let json = r#"{"id": "proj-xyz", "name": "sindri-myenv"}"#;
    let proj: sindri_providers::northflank::NorthflankProject = serde_json::from_str(json).unwrap();
    assert_eq!(proj.id, "proj-xyz");
    assert_eq!(proj.name, "sindri-myenv");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 5. Status Mapping
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn service_status_to_deployment_state() {
    let cases = vec![
        ("running", DeploymentState::Running),
        ("paused", DeploymentState::Paused),
        ("creating", DeploymentState::Creating),
        ("pending", DeploymentState::Creating),
        ("error", DeploymentState::Error),
        ("failed", DeploymentState::Error),
        ("stopped", DeploymentState::Stopped),
        ("anything-else", DeploymentState::Unknown),
    ];
    for (s, expected) in cases {
        assert_eq!(
            sindri_providers::northflank::map_service_status(s),
            expected,
            "for status '{}'",
            s
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// 6. Compute Plan Mapping
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn compute_plan_small() {
    assert_eq!(
        sindri_providers::northflank::compute_plan_from_resources(1, 512),
        "nf-compute-10"
    );
}
#[test]
fn compute_plan_medium() {
    assert_eq!(
        sindri_providers::northflank::compute_plan_from_resources(2, 2048),
        "nf-compute-20"
    );
}
#[test]
fn compute_plan_standard() {
    assert_eq!(
        sindri_providers::northflank::compute_plan_from_resources(4, 4096),
        "nf-compute-50"
    );
}
#[test]
fn compute_plan_large() {
    assert_eq!(
        sindri_providers::northflank::compute_plan_from_resources(8, 8192),
        "nf-compute-100"
    );
}
#[test]
fn compute_plan_xlarge() {
    assert_eq!(
        sindri_providers::northflank::compute_plan_from_resources(16, 16384),
        "nf-compute-200"
    );
}
#[test]
fn compute_plan_zero() {
    assert_eq!(
        sindri_providers::northflank::compute_plan_from_resources(0, 0),
        "nf-compute-10"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// 7. GPU Tier Mapping
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn gpu_small() {
    assert_eq!(
        sindri_providers::northflank::northflank_gpu_from_tier(Some("gpu-small")),
        "nvidia-a10g"
    );
}
#[test]
fn gpu_medium() {
    assert_eq!(
        sindri_providers::northflank::northflank_gpu_from_tier(Some("gpu-medium")),
        "nvidia-a10g"
    );
}
#[test]
fn gpu_large() {
    assert_eq!(
        sindri_providers::northflank::northflank_gpu_from_tier(Some("gpu-large")),
        "nvidia-a100"
    );
}
#[test]
fn gpu_xlarge() {
    assert_eq!(
        sindri_providers::northflank::northflank_gpu_from_tier(Some("gpu-xlarge")),
        "nvidia-a100"
    );
}
#[test]
fn gpu_none() {
    assert_eq!(
        sindri_providers::northflank::northflank_gpu_from_tier(None),
        "nvidia-a10g"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// 8. Deploy Lifecycle (async, mocked CLI)
// ═══════════════════════════════════════════════════════════════════════════════

/// Helper: set up mocked northflank CLI on PATH and return cleanup closure.
fn setup_mock_env(tmp: &std::path::Path) -> String {
    let original = std::env::var("PATH").unwrap_or_default();
    std::env::set_var("PATH", format!("{}:{}", tmp.display(), original));
    std::env::set_var("NORTHFLANK_API_TOKEN", "test-token");
    original
}

fn restore_env(original_path: &str) {
    std::env::set_var("PATH", original_path);
    std::env::remove_var("NORTHFLANK_API_TOKEN");
}

fn load_config(path: &std::path::Path) -> sindri_core::config::SindriConfig {
    sindri_core::config::SindriConfig::load(Some(camino::Utf8Path::new(path.to_str().unwrap())))
        .unwrap()
}

#[tokio::test]
#[serial]
async fn deploy_dry_run_returns_success_without_cli_calls() {
    use sindri_providers::traits::Provider;

    let (_tmp, cfg_path) =
        create_northflank_config_fixture("dry", "sindri-dry", "dry", "nf-compute-20").unwrap();
    let provider = sindri_providers::northflank::NorthflankProvider::new().unwrap();
    let config = load_config(&cfg_path);

    let result = provider
        .deploy(
            &config,
            DeployOptions {
                dry_run: true,
                ..Default::default()
            },
        )
        .await;
    assert!(result.is_ok(), "Dry-run should succeed");
    let r = result.unwrap();
    assert!(r.success);
    assert_eq!(r.provider, "northflank");
    assert!(
        r.instance_id.is_none(),
        "Dry run should have no instance_id"
    );
}

#[tokio::test]
#[serial]
async fn deploy_existing_service_without_force_errors() {
    use sindri_providers::traits::Provider;

    let tmp = tempfile::tempdir().unwrap();
    let svc_json = json!([{"id":"svc-ex","name":"dup","status":"running","image":null,"computePlan":"nf-compute-20","instances":1,"ports":[],"metrics":null}]);
    create_conditional_mock(
        tmp.path(),
        "northflank",
        &[
            ("list services", &svc_json.to_string(), 0),
            ("--version", "1.2.3", 0),
            ("list projects", "{}", 0),
        ],
        "{}",
        0,
    )
    .unwrap();

    let orig = setup_mock_env(tmp.path());
    let (_ct, cp) =
        create_northflank_config_fixture("dup", "sindri-dup", "dup", "nf-compute-20").unwrap();
    let provider = sindri_providers::northflank::NorthflankProvider::new().unwrap();
    let result = provider
        .deploy(&load_config(&cp), DeployOptions::default())
        .await;
    restore_env(&orig);

    assert!(result.is_err(), "Existing service + no --force should fail");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("already exists") || msg.contains("--force"),
        "got '{}'",
        msg
    );
}

#[tokio::test]
#[serial]
async fn deploy_force_destroys_then_creates() {
    use sindri_providers::traits::Provider;

    let tmp = tempfile::tempdir().unwrap();
    let svc = json!([{"id":"svc-old","name":"fc","status":"running","image":null,"computePlan":"nf-compute-20","instances":1,"ports":[],"metrics":null}]);
    let new_svc = json!({"id":"svc-new","name":"fc","status":"creating","image":"img","computePlan":"nf-compute-20","instances":1,"ports":[{"name":"ssh","internalPort":22,"public":false,"dns":null}],"metrics":null});

    create_conditional_mock(
        tmp.path(),
        "northflank",
        &[
            ("list services", &svc.to_string(), 0),
            ("delete service", "ok", 0),
            ("get project", "{}", 0),
            ("create service", &new_svc.to_string(), 0),
            ("create volume", "{}", 0),
            ("--version", "1.2.3", 0),
            ("list projects", "{}", 0),
        ],
        "{}",
        0,
    )
    .unwrap();

    let orig = setup_mock_env(tmp.path());
    let (_ct, cp) =
        create_northflank_config_fixture("fc", "sindri-fc", "fc", "nf-compute-20").unwrap();
    let provider = sindri_providers::northflank::NorthflankProvider::new().unwrap();
    let result = provider
        .deploy(
            &load_config(&cp),
            DeployOptions {
                force: true,
                ..Default::default()
            },
        )
        .await;
    restore_env(&orig);

    let log = read_mock_log(tmp.path(), "northflank");
    let del = log.iter().position(|l| l.contains("delete"));
    let cre = log.iter().position(|l| l.contains("create service"));
    if let (Some(d), Some(c)) = (del, cre) {
        assert!(d < c, "delete before create. Log: {:?}", log);
    }
    assert!(
        result.is_ok(),
        "Deploy --force should succeed: {:?}",
        result.err()
    );
}

#[tokio::test]
#[serial]
async fn deploy_creates_project_when_missing() {
    use sindri_providers::traits::Provider;
    let tmp = tempfile::tempdir().unwrap();
    let new_svc = json!({"id":"svc-x","name":"np","status":"running","image":"img","computePlan":"nf-compute-20","instances":1,"ports":[],"metrics":null});
    create_conditional_mock(
        tmp.path(),
        "northflank",
        &[
            ("list services", "[]", 0),
            ("get project", "not found", 1),
            (
                "create project",
                r#"{"id":"proj-new","name":"sindri-np"}"#,
                0,
            ),
            ("create service", &new_svc.to_string(), 0),
            ("create volume", "{}", 0),
            ("--version", "1.2.3", 0),
            ("list projects", "{}", 0),
        ],
        "{}",
        0,
    )
    .unwrap();

    let orig = setup_mock_env(tmp.path());
    let (_ct, cp) =
        create_northflank_config_fixture("np", "sindri-np", "np", "nf-compute-20").unwrap();
    let provider = sindri_providers::northflank::NorthflankProvider::new().unwrap();
    let _ = provider
        .deploy(&load_config(&cp), DeployOptions::default())
        .await;
    restore_env(&orig);

    let log = read_mock_log(tmp.path(), "northflank");
    assert!(
        log.iter().any(|l| l.contains("create project")),
        "Should create project. Log: {:?}",
        log
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// 9. Status Queries (async, mocked CLI)
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[serial]
async fn status_running() {
    use sindri_providers::traits::Provider;
    let tmp = tempfile::tempdir().unwrap();
    let svc = json!([{"id":"svc-r","name":"sr","status":"running","image":"img","computePlan":"nf-compute-50","instances":1,"ports":[{"name":"http","internalPort":8080,"public":true,"dns":"sr.nf.app"}],"metrics":{"cpuPercent":15.0,"memoryBytes":1073741824_i64,"memoryLimit":4294967296_i64,"diskBytes":null,"diskLimit":null}}]);
    create_conditional_mock(
        tmp.path(),
        "northflank",
        &[
            ("list services", &svc.to_string(), 0),
            ("--version", "1.2.3", 0),
            ("list projects", "{}", 0),
        ],
        "[]",
        0,
    )
    .unwrap();

    let orig = setup_mock_env(tmp.path());
    let (_ct, cp) =
        create_northflank_config_fixture("sr", "sindri-sr", "sr", "nf-compute-50").unwrap();
    let result = sindri_providers::northflank::NorthflankProvider::new()
        .unwrap()
        .status(&load_config(&cp))
        .await;
    restore_env(&orig);

    assert!(result.is_ok());
    let s = result.unwrap();
    assert_eq!(s.state, DeploymentState::Running);
    assert_eq!(s.provider, "northflank");
    assert!(s.instance_id.is_some());
}

#[tokio::test]
#[serial]
async fn status_paused() {
    use sindri_providers::traits::Provider;
    let tmp = tempfile::tempdir().unwrap();
    let svc = json!([{"id":"svc-p","name":"sp","status":"paused","image":null,"computePlan":"nf-compute-20","instances":1,"ports":[],"metrics":null}]);
    create_conditional_mock(
        tmp.path(),
        "northflank",
        &[
            ("list services", &svc.to_string(), 0),
            ("--version", "1.2.3", 0),
            ("list projects", "{}", 0),
        ],
        "[]",
        0,
    )
    .unwrap();

    let orig = setup_mock_env(tmp.path());
    let (_ct, cp) =
        create_northflank_config_fixture("sp", "sindri-sp", "sp", "nf-compute-20").unwrap();
    let result = sindri_providers::northflank::NorthflankProvider::new()
        .unwrap()
        .status(&load_config(&cp))
        .await;
    restore_env(&orig);

    assert_eq!(result.unwrap().state, DeploymentState::Paused);
}

#[tokio::test]
#[serial]
async fn status_not_deployed() {
    use sindri_providers::traits::Provider;
    let tmp = tempfile::tempdir().unwrap();
    create_conditional_mock(
        tmp.path(),
        "northflank",
        &[
            ("list services", "[]", 0),
            ("--version", "1.2.3", 0),
            ("list projects", "{}", 0),
        ],
        "[]",
        0,
    )
    .unwrap();

    let orig = setup_mock_env(tmp.path());
    let (_ct, cp) =
        create_northflank_config_fixture("sn", "sindri-sn", "sn", "nf-compute-20").unwrap();
    let result = sindri_providers::northflank::NorthflankProvider::new()
        .unwrap()
        .status(&load_config(&cp))
        .await;
    restore_env(&orig);

    assert_eq!(result.unwrap().state, DeploymentState::NotDeployed);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 10. Connect (async, mocked CLI)
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[serial]
async fn connect_running_calls_exec() {
    use sindri_providers::traits::Provider;
    let tmp = tempfile::tempdir().unwrap();
    let svc = json!([{"id":"svc-c","name":"cn","status":"running","image":null,"computePlan":"nf-compute-20","instances":1,"ports":[],"metrics":null}]);
    create_conditional_mock(
        tmp.path(),
        "northflank",
        &[
            ("list services", &svc.to_string(), 0),
            ("exec", "", 0),
            ("--version", "1.2.3", 0),
            ("list projects", "{}", 0),
        ],
        "{}",
        0,
    )
    .unwrap();

    let orig = setup_mock_env(tmp.path());
    let (_ct, cp) =
        create_northflank_config_fixture("cn", "sindri-cn", "cn", "nf-compute-20").unwrap();
    let _ = sindri_providers::northflank::NorthflankProvider::new()
        .unwrap()
        .connect(&load_config(&cp))
        .await;
    restore_env(&orig);

    let log = read_mock_log(tmp.path(), "northflank");
    assert!(
        log.iter().any(|l| l.contains("exec")),
        "Should call exec. Log: {:?}",
        log
    );
}

#[tokio::test]
#[serial]
async fn connect_paused_resumes_then_connects() {
    use sindri_providers::traits::Provider;
    let tmp = tempfile::tempdir().unwrap();
    let svc = json!([{"id":"svc-cp","name":"cr","status":"paused","image":null,"computePlan":"nf-compute-20","instances":1,"ports":[],"metrics":null}]);
    create_conditional_mock(
        tmp.path(),
        "northflank",
        &[
            ("list services", &svc.to_string(), 0),
            ("resume", "", 0),
            ("exec", "", 0),
            ("--version", "1.2.3", 0),
            ("list projects", "{}", 0),
        ],
        "{}",
        0,
    )
    .unwrap();

    let orig = setup_mock_env(tmp.path());
    let (_ct, cp) =
        create_northflank_config_fixture("cr", "sindri-cr", "cr", "nf-compute-20").unwrap();
    let _ = sindri_providers::northflank::NorthflankProvider::new()
        .unwrap()
        .connect(&load_config(&cp))
        .await;
    restore_env(&orig);

    let log = read_mock_log(tmp.path(), "northflank");
    let ri = log.iter().position(|l| l.contains("resume"));
    let ei = log.iter().position(|l| l.contains("exec"));
    assert!(ri.is_some(), "Should call resume. Log: {:?}", log);
    if let (Some(r), Some(e)) = (ri, ei) {
        assert!(r < e, "resume before exec");
    }
}

#[tokio::test]
#[serial]
async fn connect_no_service_errors() {
    use sindri_providers::traits::Provider;
    let tmp = tempfile::tempdir().unwrap();
    create_conditional_mock(
        tmp.path(),
        "northflank",
        &[
            ("list services", "[]", 0),
            ("--version", "1.2.3", 0),
            ("list projects", "{}", 0),
        ],
        "[]",
        0,
    )
    .unwrap();

    let orig = setup_mock_env(tmp.path());
    let (_ct, cp) =
        create_northflank_config_fixture("nc", "sindri-nc", "nc", "nf-compute-20").unwrap();
    let result = sindri_providers::northflank::NorthflankProvider::new()
        .unwrap()
        .connect(&load_config(&cp))
        .await;
    restore_env(&orig);

    assert!(result.is_err(), "Connect without service should fail");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 11. Destroy (async, mocked CLI)
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[serial]
async fn destroy_deletes_service() {
    use sindri_providers::traits::Provider;
    let tmp = tempfile::tempdir().unwrap();
    let svc = json!([{"id":"svc-d","name":"ds","status":"running","image":null,"computePlan":"nf-compute-20","instances":1,"ports":[],"metrics":null}]);
    create_conditional_mock(
        tmp.path(),
        "northflank",
        &[
            ("list services", &svc.to_string(), 0),
            ("delete service", "deleted", 0),
            ("--version", "1.2.3", 0),
            ("list projects", "{}", 0),
        ],
        "{}",
        0,
    )
    .unwrap();

    let orig = setup_mock_env(tmp.path());
    let (_ct, cp) =
        create_northflank_config_fixture("ds", "sindri-ds", "ds", "nf-compute-20").unwrap();
    let result = sindri_providers::northflank::NorthflankProvider::new()
        .unwrap()
        .destroy(&load_config(&cp), true)
        .await;
    restore_env(&orig);

    assert!(result.is_ok(), "Destroy should succeed: {:?}", result.err());
    let log = read_mock_log(tmp.path(), "northflank");
    assert!(
        log.iter().any(|l| l.contains("delete")),
        "Should call delete. Log: {:?}",
        log
    );
}

#[tokio::test]
#[serial]
async fn destroy_preserves_project() {
    use sindri_providers::traits::Provider;
    let tmp = tempfile::tempdir().unwrap();
    let svc = json!([{"id":"svc-dp","name":"dp","status":"running","image":null,"computePlan":"nf-compute-20","instances":1,"ports":[],"metrics":null}]);
    create_conditional_mock(
        tmp.path(),
        "northflank",
        &[
            ("list services", &svc.to_string(), 0),
            ("delete service", "deleted", 0),
            ("--version", "1.2.3", 0),
            ("list projects", "{}", 0),
        ],
        "{}",
        0,
    )
    .unwrap();

    let orig = setup_mock_env(tmp.path());
    let (_ct, cp) =
        create_northflank_config_fixture("dp", "sindri-dp", "dp", "nf-compute-20").unwrap();
    let _ = sindri_providers::northflank::NorthflankProvider::new()
        .unwrap()
        .destroy(&load_config(&cp), true)
        .await;
    restore_env(&orig);

    let log = read_mock_log(tmp.path(), "northflank");
    assert!(
        !log.iter().any(|l| l.contains("delete project")),
        "Should NOT delete project. Log: {:?}",
        log
    );
}

#[tokio::test]
#[serial]
async fn destroy_no_service_errors() {
    use sindri_providers::traits::Provider;
    let tmp = tempfile::tempdir().unwrap();
    create_conditional_mock(
        tmp.path(),
        "northflank",
        &[
            ("list services", "[]", 0),
            ("--version", "1.2.3", 0),
            ("list projects", "{}", 0),
        ],
        "[]",
        0,
    )
    .unwrap();

    let orig = setup_mock_env(tmp.path());
    let (_ct, cp) =
        create_northflank_config_fixture("nd", "sindri-nd", "nd", "nf-compute-20").unwrap();
    let result = sindri_providers::northflank::NorthflankProvider::new()
        .unwrap()
        .destroy(&load_config(&cp), false)
        .await;
    restore_env(&orig);

    assert!(result.is_err(), "Destroy with no service should fail");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 12. Start/Stop (Resume/Pause)
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[serial]
async fn start_calls_resume() {
    use sindri_providers::traits::Provider;
    let tmp = tempfile::tempdir().unwrap();
    let svc = json!([{"id":"svc-st","name":"st","status":"paused","image":null,"computePlan":"nf-compute-20","instances":1,"ports":[],"metrics":null}]);
    create_conditional_mock(
        tmp.path(),
        "northflank",
        &[
            ("list services", &svc.to_string(), 0),
            ("resume", "", 0),
            ("--version", "1.2.3", 0),
            ("list projects", "{}", 0),
        ],
        "{}",
        0,
    )
    .unwrap();

    let orig = setup_mock_env(tmp.path());
    let (_ct, cp) =
        create_northflank_config_fixture("st", "sindri-st", "st", "nf-compute-20").unwrap();
    let result = sindri_providers::northflank::NorthflankProvider::new()
        .unwrap()
        .start(&load_config(&cp))
        .await;
    restore_env(&orig);

    assert!(result.is_ok(), "start() should succeed: {:?}", result.err());
    assert!(read_mock_log(tmp.path(), "northflank")
        .iter()
        .any(|l| l.contains("resume")));
}

#[tokio::test]
#[serial]
async fn stop_calls_pause() {
    use sindri_providers::traits::Provider;
    let tmp = tempfile::tempdir().unwrap();
    let svc = json!([{"id":"svc-sp2","name":"sp2","status":"running","image":null,"computePlan":"nf-compute-20","instances":1,"ports":[],"metrics":null}]);
    create_conditional_mock(
        tmp.path(),
        "northflank",
        &[
            ("list services", &svc.to_string(), 0),
            ("pause", "", 0),
            ("--version", "1.2.3", 0),
            ("list projects", "{}", 0),
        ],
        "{}",
        0,
    )
    .unwrap();

    let orig = setup_mock_env(tmp.path());
    let (_ct, cp) =
        create_northflank_config_fixture("sp2", "sindri-sp2", "sp2", "nf-compute-20").unwrap();
    let result = sindri_providers::northflank::NorthflankProvider::new()
        .unwrap()
        .stop(&load_config(&cp))
        .await;
    restore_env(&orig);

    assert!(result.is_ok(), "stop() should succeed: {:?}", result.err());
    assert!(read_mock_log(tmp.path(), "northflank")
        .iter()
        .any(|l| l.contains("pause")));
}

#[tokio::test]
#[serial]
async fn start_no_service_errors() {
    use sindri_providers::traits::Provider;
    let tmp = tempfile::tempdir().unwrap();
    create_conditional_mock(
        tmp.path(),
        "northflank",
        &[
            ("list services", "[]", 0),
            ("--version", "1.2.3", 0),
            ("list projects", "{}", 0),
        ],
        "[]",
        0,
    )
    .unwrap();

    let orig = setup_mock_env(tmp.path());
    let (_ct, cp) =
        create_northflank_config_fixture("ns", "sindri-ns", "ns", "nf-compute-20").unwrap();
    let result = sindri_providers::northflank::NorthflankProvider::new()
        .unwrap()
        .start(&load_config(&cp))
        .await;
    restore_env(&orig);

    assert!(result.is_err(), "start() without service should fail");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 13. Plan (dry-run)
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn plan_returns_project_service_actions() {
    use sindri_providers::traits::Provider;
    let (_ct, cp) =
        create_northflank_config_fixture("pl", "sindri-pl", "pl", "nf-compute-50").unwrap();
    let result = sindri_providers::northflank::NorthflankProvider::new()
        .unwrap()
        .plan(&load_config(&cp))
        .await;

    assert!(result.is_ok());
    let plan = result.unwrap();
    assert_eq!(plan.provider, "northflank");
    assert!(plan.actions.len() >= 2);
    assert!(plan.actions.iter().any(|a| a.resource.contains("project")));
    assert!(plan.actions.iter().any(|a| a.resource.contains("service")));
}

#[tokio::test]
async fn plan_includes_volume_action() {
    use sindri_providers::traits::Provider;
    let (_ct, cp) =
        create_northflank_config_fixture("pv", "sindri-pv", "pv", "nf-compute-20").unwrap();
    let plan = sindri_providers::northflank::NorthflankProvider::new()
        .unwrap()
        .plan(&load_config(&cp))
        .await
        .unwrap();
    assert!(
        plan.actions.iter().any(|a| a.resource.contains("volume")),
        "Actions: {:?}",
        plan.actions
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// 14. Service Definition Builder
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn build_service_definition_basic() {
    let cfg = sindri_providers::northflank::NorthflankDeployConfig {
        name: "svc",
        project_name: "proj".into(),
        service_name: "svc".into(),
        compute_plan: "nf-compute-50".into(),
        instances: 1,
        gpu_type: None,
        gpu_count: 0,
        volume_size_gb: 10,
        volume_mount_path: "/workspace".into(),
        region: None,
        ports: vec![sindri_providers::northflank::NorthflankPort {
            name: "ssh".into(),
            internal_port: 22,
            public: false,
            protocol: "TCP".into(),
        }],
        health_check: None,
        auto_scaling: None,
        cpus: 4,
        memory_mb: 4096,
        image: "img:latest".into(),
    };
    let provider = sindri_providers::northflank::NorthflankProvider::new().unwrap();
    let json_str = provider.build_service_definition(&cfg).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(v["name"], "svc");
    assert_eq!(v["billing"]["deploymentPlan"], "nf-compute-50");
    assert!(v["ports"].is_array());
}

#[test]
fn build_service_definition_with_health_check() {
    let cfg = sindri_providers::northflank::NorthflankDeployConfig {
        name: "hc",
        project_name: "p".into(),
        service_name: "hc".into(),
        compute_plan: "nf-compute-20".into(),
        instances: 1,
        gpu_type: None,
        gpu_count: 0,
        volume_size_gb: 10,
        volume_mount_path: "/ws".into(),
        region: None,
        ports: vec![],
        health_check: Some(sindri_providers::northflank::NorthflankHealthCheck {
            path: "/health".into(),
            port: 8080,
            interval_secs: 30,
            timeout_secs: 5,
        }),
        auto_scaling: None,
        cpus: 2,
        memory_mb: 2048,
        image: "img".into(),
    };
    let v: serde_json::Value = serde_json::from_str(
        &sindri_providers::northflank::NorthflankProvider::new()
            .unwrap()
            .build_service_definition(&cfg)
            .unwrap(),
    )
    .unwrap();
    assert!(v["healthChecks"].is_array());
    assert_eq!(v["healthChecks"][0]["path"], "/health");
    assert_eq!(v["healthChecks"][0]["port"], 8080);
}

#[test]
fn build_service_definition_with_auto_scaling() {
    let cfg = sindri_providers::northflank::NorthflankDeployConfig {
        name: "as",
        project_name: "p".into(),
        service_name: "as".into(),
        compute_plan: "nf-compute-50".into(),
        instances: 1,
        gpu_type: None,
        gpu_count: 0,
        volume_size_gb: 10,
        volume_mount_path: "/ws".into(),
        region: None,
        ports: vec![],
        health_check: None,
        auto_scaling: Some(sindri_providers::northflank::NorthflankAutoScaling {
            min_instances: 1,
            max_instances: 5,
            cpu_target_percent: 75,
        }),
        cpus: 4,
        memory_mb: 4096,
        image: "img".into(),
    };
    let v: serde_json::Value = serde_json::from_str(
        &sindri_providers::northflank::NorthflankProvider::new()
            .unwrap()
            .build_service_definition(&cfg)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(v["deployment"]["scaling"]["minInstances"], 1);
    assert_eq!(v["deployment"]["scaling"]["maxInstances"], 5);
    assert_eq!(v["deployment"]["scaling"]["targetCpu"], 75);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 15. Secret Groups
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
#[serial]
async fn create_secret_group_calls_cli() {
    let tmp = tempfile::tempdir().unwrap();
    create_conditional_mock(
        tmp.path(),
        "northflank",
        &[("create secret", r#"{"id":"sg-1"}"#, 0)],
        "{}",
        0,
    )
    .unwrap();

    let orig = setup_mock_env(tmp.path());
    let mut secrets = HashMap::new();
    secrets.insert("DB_PASS".into(), "s3cret".into());
    let result = sindri_providers::northflank::NorthflankProvider::new()
        .unwrap()
        .create_secret_group("proj", "svc", &secrets)
        .await;
    restore_env(&orig);

    assert!(result.is_ok());
    assert!(read_mock_log(tmp.path(), "northflank")
        .iter()
        .any(|l| l.contains("create secret")));
}

// ═══════════════════════════════════════════════════════════════════════════════
// 16. Config/State Fixture Verification
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn config_fixture_creates_valid_yaml() {
    let (_t, p) =
        create_northflank_config_fixture("fx", "sindri-fx", "fx-svc", "nf-compute-20").unwrap();
    let yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
    assert_eq!(yaml["name"], "fx");
    assert_eq!(yaml["providers"]["northflank"]["project_name"], "sindri-fx");
    assert_eq!(
        yaml["providers"]["northflank"]["compute_plan"],
        "nf-compute-20"
    );
}

#[test]
fn state_fixture_creates_valid_json() {
    let tmp = tempfile::tempdir().unwrap();
    let sp = create_northflank_state(tmp.path(), "proj", "svc").unwrap();
    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&sp).unwrap()).unwrap();
    assert_eq!(v["project_name"], "proj");
    assert_eq!(v["service_name"], "svc");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 17. Factory Integration
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn create_provider_returns_northflank() {
    use sindri_core::types::Provider as ProviderType;
    let result = sindri_providers::create_provider(ProviderType::Northflank);
    assert!(
        result.is_ok(),
        "create_provider(Northflank) should succeed: {:?}",
        result.err()
    );
    assert_eq!(result.unwrap().name(), "northflank");
}
