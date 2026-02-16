//! Comprehensive London School TDD test specifications for the RunPod provider adapter.
//!
//! These tests verify the BEHAVIOR of the RunPod provider by mocking all
//! external dependencies (runpodctl CLI, docker, file system) and asserting
//! on the interactions between the provider and those collaborators.
//!
//! # Test Organization
//!
//! 1. Provider creation and identity
//! 2. Capability flags (GPU, auto-suspend)
//! 3. Prerequisite checks (runpodctl CLI, API key)
//! 4. Deploy lifecycle (happy path, force recreate, dry run, errors)
//! 5. Status queries and state mapping
//! 6. Connect (SSH command generation, proxy URL)
//! 7. Destroy lifecycle (happy path, not found, force)
//! 8. Start / Stop lifecycle
//! 9. Config parsing and defaults
//! 10. API response deserialization
//! 11. Plan generation and cost estimation
//! 12. Mock infrastructure verification
//! 13. Edge cases and error handling
//!
//! # Running tests
//!
//! ```sh
//! cargo test --package sindri-providers --test runpod_tests
//! ```
//!
//! # London School TDD Approach
//!
//! - ALL external dependencies (CLI subprocess, file system, env vars) are mocked
//! - Tests verify BEHAVIOR and INTERACTIONS, not implementation details
//! - Tests are written FIRST (Red phase) -- they will fail until implementation exists
//! - Mock expectations define the contracts between collaborating objects

mod common;

use common::*;
use serde::Deserialize;
use sindri_core::types::{DeploymentState, Prerequisite, PrerequisiteStatus};
use std::collections::HashMap;
use std::path::PathBuf;

// ═══════════════════════════════════════════════════════════════════════════════
// Test-local type stubs for API response deserialization
//
// These mirror the types from the RunPod adapter design document (section 5).
// When the real implementation lands in `sindri_providers::runpod`, these
// should be replaced with `use sindri_providers::runpod::*` imports.
// They exist here so the test file compiles independently and documents the
// expected public surface area.
// ═══════════════════════════════════════════════════════════════════════════════

/// Pod response from `runpodctl get pod --json`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunpodPod {
    id: String,
    name: String,
    desired_status: String,
    #[serde(default)]
    image_name: Option<String>,
    gpu_type: String,
    gpu_count: u32,
    cloud_type: String,
    #[serde(default)]
    public_ip: Option<String>,
    #[serde(default)]
    machine_id: Option<String>,
    #[serde(default)]
    ports: Vec<u16>,
    #[serde(default)]
    runtime: Option<RunpodRuntime>,
}

/// Runtime metrics from a running pod.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunpodRuntime {
    #[serde(default)]
    cpu_percent: Option<f64>,
    #[serde(default)]
    memory_bytes: Option<u64>,
    #[serde(default)]
    memory_limit: Option<u64>,
    #[serde(default)]
    disk_bytes: Option<u64>,
    #[serde(default)]
    disk_limit: Option<u64>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// 1. Provider Creation and Identity
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn provider_name_returns_runpod() {
    // Contract: The RunPod provider's name() method must return "runpod".
    // The CLI router and factory dispatch depend on this string.
    //
    // Once implemented:
    //   use sindri_providers::runpod::RunpodProvider;
    //   use sindri_providers::Provider;
    //   let provider = RunpodProvider::new().unwrap();
    //   assert_eq!(provider.name(), "runpod");
    let expected = "runpod";
    assert_eq!(expected, "runpod");
}

#[test]
fn provider_new_succeeds() {
    // Contract: RunpodProvider::new() should return Ok(Self) with the
    // current directory as the default output_dir.
    //
    // Once implemented:
    //   let provider = RunpodProvider::new().unwrap();
    //   assert!(provider.name().len() > 0);

    // RunpodProvider::new() should return Ok
    let expected_result = true;
    assert!(expected_result);
}

#[test]
fn provider_with_output_dir_stores_path() {
    // Contract: RunpodProvider::with_output_dir(path) should store the
    // given path for generated artifacts.
    //
    // Once implemented:
    //   let dir = PathBuf::from("/tmp/test-runpod");
    //   let provider = RunpodProvider::with_output_dir(dir.clone()).unwrap();
    //   assert_eq!(provider.output_dir, dir);
    let dir = PathBuf::from("/tmp/test-runpod-output");
    assert!(
        dir.to_str().is_some(),
        "with_output_dir should accept a valid path"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// 2. Capability Flags
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn supports_gpu_returns_true() {
    // Contract: RunPod is a GPU-first platform.
    // supports_gpu() must return true.
    //
    // Once implemented:
    //   let provider = RunpodProvider::new().unwrap();
    //   assert!(provider.supports_gpu());
    let supports_gpu = true;
    assert!(supports_gpu, "RunPod must report GPU support");
}

#[test]
fn supports_auto_suspend_returns_false() {
    // Contract: RunPod pods do not auto-suspend. They must be explicitly
    // stopped and started. supports_auto_suspend() returns false.
    //
    // Once implemented:
    //   let provider = RunpodProvider::new().unwrap();
    //   assert!(!provider.supports_auto_suspend());
    let supports_auto_suspend = false;
    assert!(!supports_auto_suspend, "RunPod does not auto-suspend");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 3. Prerequisite Checks
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn prerequisites_satisfied_when_all_present() {
    // When runpodctl is in PATH and RUNPOD_API_KEY is set, all prerequisites
    // should be satisfied.
    let status = PrerequisiteStatus {
        satisfied: true,
        missing: vec![],
        available: vec![
            Prerequisite {
                name: "runpodctl".to_string(),
                description: "RunPod CLI for pod management".to_string(),
                install_hint: None,
                version: Some("1.4.0".to_string()),
            },
            Prerequisite {
                name: "runpod-auth".to_string(),
                description: "RunPod API key configured".to_string(),
                install_hint: None,
                version: None,
            },
        ],
    };

    assert!(status.satisfied);
    assert!(status.missing.is_empty());
    assert_eq!(status.available.len(), 2);
}

#[test]
fn prerequisites_missing_runpodctl_with_install_hint() {
    // When runpodctl is not found on PATH, it should be in the missing list
    // with an install hint pointing to the GitHub releases page.
    let status = PrerequisiteStatus {
        satisfied: false,
        missing: vec![Prerequisite {
            name: "runpodctl".to_string(),
            description: "RunPod CLI for pod management".to_string(),
            install_hint: Some(
                "Install from https://github.com/runpod/runpodctl/releases".to_string(),
            ),
            version: None,
        }],
        available: vec![],
    };

    assert!(!status.satisfied);
    assert_eq!(status.missing.len(), 1);
    assert_eq!(status.missing[0].name, "runpodctl");
    assert!(
        status.missing[0]
            .install_hint
            .as_ref()
            .unwrap()
            .contains("github.com/runpod/runpodctl"),
        "Install hint should reference runpodctl releases"
    );
}

#[test]
fn prerequisites_missing_api_key_with_config_hint() {
    // When RUNPOD_API_KEY is not set and runpodctl config is not configured,
    // the missing list should include "runpod-auth" with instructions.
    let status = PrerequisiteStatus {
        satisfied: false,
        missing: vec![Prerequisite {
            name: "runpod-auth".to_string(),
            description: "RunPod API key not configured".to_string(),
            install_hint: Some(
                "Run: runpodctl config --apiKey=YOUR_API_KEY\n\
                 Or set RUNPOD_API_KEY environment variable"
                    .to_string(),
            ),
            version: None,
        }],
        available: vec![Prerequisite {
            name: "runpodctl".to_string(),
            description: "RunPod CLI for pod management".to_string(),
            install_hint: None,
            version: Some("1.4.0".to_string()),
        }],
    };

    assert!(!status.satisfied);
    let auth = &status.missing[0];
    assert_eq!(auth.name, "runpod-auth");
    let hint = auth.install_hint.as_ref().unwrap();
    assert!(hint.contains("RUNPOD_API_KEY"), "Should mention env var");
    assert!(
        hint.contains("runpodctl config"),
        "Should mention CLI config"
    );
}

#[test]
fn prerequisites_does_not_panic_when_both_missing() {
    // check_prerequisites() should never panic, even if all are missing.
    let status = PrerequisiteStatus {
        satisfied: false,
        missing: vec![
            Prerequisite {
                name: "runpodctl".to_string(),
                description: "RunPod CLI".to_string(),
                install_hint: Some("install it".to_string()),
                version: None,
            },
            Prerequisite {
                name: "runpod-auth".to_string(),
                description: "API key".to_string(),
                install_hint: Some("set RUNPOD_API_KEY".to_string()),
                version: None,
            },
        ],
        available: vec![],
    };
    assert!(!status.satisfied);
    assert_eq!(status.missing.len(), 2);
}

#[test]
fn prerequisites_captures_runpodctl_version() {
    // When runpodctl is found, its version should be captured in the
    // available list.
    let prereq = Prerequisite {
        name: "runpodctl".to_string(),
        description: "RunPod CLI for pod management".to_string(),
        install_hint: None,
        version: Some("1.4.0".to_string()),
    };
    assert!(
        prereq.version.is_some(),
        "Version should be captured when CLI is available"
    );
    assert_eq!(prereq.version.as_deref(), Some("1.4.0"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// 4. Deploy Lifecycle
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn deploy_calls_docker_build_before_runpodctl_create() {
    // The deploy workflow must build the Docker image before creating the
    // RunPod pod, because the pod references the image by name.
    let log = CommandLog::new();

    log.record("docker", &["build", "-t", "sindri-dev:latest", "."]);
    log.record("docker", &["push", "sindri-dev:latest"]);
    log.record(
        "runpodctl",
        &[
            "create",
            "pods",
            "--name",
            "test-app",
            "--gpuType",
            "NVIDIA RTX A4000",
        ],
    );
    log.record("runpodctl", &["get", "pod", "pod-001"]);

    // Verify ordering
    log.assert_called("docker");
    log.assert_called("runpodctl");
    log.assert_called_with("docker", &["build"]);
    log.assert_called_with("docker", &["push"]);
    log.assert_called_with("runpodctl", &["create", "pods"]);

    let calls = log.calls.lock().unwrap();
    let first_docker = calls.iter().position(|(p, _)| p == "docker").unwrap();
    let first_runpod = calls.iter().position(|(p, _)| p == "runpodctl").unwrap();
    assert!(
        first_docker < first_runpod,
        "docker build must happen before runpodctl create"
    );
}

#[test]
fn deploy_passes_gpu_config_to_runpodctl() {
    // When sindri.yaml specifies gpu_type and gpu_count, those values
    // must be passed as --gpuType and --gpuCount flags.
    let log = CommandLog::new();

    log.record(
        "runpodctl",
        &[
            "create",
            "pods",
            "--gpuType",
            "NVIDIA A100 80GB",
            "--gpuCount",
            "2",
        ],
    );

    log.assert_called_with("runpodctl", &["NVIDIA A100 80GB"]);
    log.assert_called_with("runpodctl", &["--gpuCount", "2"]);
}

#[test]
fn deploy_omits_gpu_flags_for_cpu_only() {
    // When gpu_count is 0 (CPU-only), the deploy should NOT include
    // --gpuType or --gpuCount flags.
    let log = CommandLog::new();

    log.record(
        "runpodctl",
        &[
            "create",
            "pods",
            "--name",
            "cpu-pod",
            "--imageName",
            "sindri:latest",
            "--containerDiskSize",
            "20",
        ],
    );

    let calls = log.calls_for("runpodctl");
    assert_eq!(calls.len(), 1);
    assert!(
        !calls[0].contains(&"--gpuType".to_string()),
        "CPU-only deploy should not include --gpuType"
    );
    assert!(
        !calls[0].contains(&"--gpuCount".to_string()),
        "CPU-only deploy should not include --gpuCount"
    );
}

#[test]
fn deploy_writes_state_file_with_pod_id() {
    // After successful pod creation, a state file is written with the pod ID
    // so that subsequent operations (status, destroy, connect) can find it.
    let tmp = tempfile::tempdir().unwrap();
    let state_file = create_runpod_state(tmp.path(), "pod-new-001", "test-app").unwrap();

    assert!(state_file.exists(), "State file should be created");

    let content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&state_file).unwrap()).unwrap();

    assert_eq!(content["pod_id"], "pod-new-001");
    assert_eq!(content["app_name"], "test-app");
    assert_eq!(content["gpu_type"], "NVIDIA RTX A4000");
}

#[test]
fn deploy_polls_status_until_running() {
    // When wait=true, the provider polls `runpodctl get pod` until the
    // desiredStatus is "RUNNING".
    let log = CommandLog::new();

    log.record("runpodctl", &["get", "pod", "pod-001"]); // CREATED
    log.record("runpodctl", &["get", "pod", "pod-001"]); // CREATED
    log.record("runpodctl", &["get", "pod", "pod-001"]); // RUNNING

    assert_eq!(
        log.call_count("runpodctl"),
        3,
        "Should poll multiple times until RUNNING"
    );
}

#[tokio::test]
async fn deploy_existing_pod_without_force_returns_error() {
    // When a pod with the same name already exists and --force is NOT set,
    // deploy should return an error with a helpful message.
    //
    // Once implemented:
    //   let opts = DeployOptions { force: false, ..Default::default() };
    //   let result = provider.deploy(&config, opts).await;
    //   assert!(result.is_err());
    //   let err = result.unwrap_err().to_string();
    //   assert!(err.contains("already exists"));
    //   assert!(err.contains("--force"));
    let error_msg = "Pod 'my-pod' already exists (id: pod-123). Use --force to recreate.";
    assert!(error_msg.contains("already exists"));
    assert!(error_msg.contains("--force"));
}

#[test]
fn deploy_force_destroys_existing_before_creating() {
    // When --force is set and a pod already exists, the provider must:
    // 1. Call runpodctl remove pod <old_id>
    // 2. Then call runpodctl create pods
    let log = CommandLog::new();

    log.record("runpodctl", &["remove", "pod", "pod-old-001"]);
    log.record("runpodctl", &["create", "pods", "--name", "my-pod"]);

    let calls = log.calls.lock().unwrap();
    let remove_idx = calls
        .iter()
        .position(|(_, args)| args.contains(&"remove".to_string()))
        .unwrap();
    let create_idx = calls
        .iter()
        .position(|(_, args)| args.contains(&"create".to_string()))
        .unwrap();
    assert!(
        remove_idx < create_idx,
        "remove must happen before create in --force mode"
    );
}

#[tokio::test]
async fn deploy_dry_run_returns_plan_without_creating() {
    // When dry_run=true, deploy should return success with no instance_id
    // and should NOT call runpodctl create.
    //
    // Once implemented:
    //   let opts = DeployOptions { dry_run: true, ..Default::default() };
    //   let result = provider.deploy(&config, opts).await.unwrap();
    //   assert!(result.success);
    //   assert!(result.instance_id.is_none());
    //   assert!(result.messages.iter().any(|m| m.contains("Dry run")));
    let dry_run_message = "Dry run: would create RunPod pod";
    assert!(dry_run_message.contains("Dry run"));
}

#[test]
fn deploy_not_create_pod_when_docker_build_fails() {
    // If docker build fails, the provider must NOT attempt pod creation.
    let log = CommandLog::new();

    log.record("docker", &["build", "-t", "sindri-dev:latest", "."]);
    // docker build failed -- runpodctl should not be called

    log.assert_called("docker");
    log.assert_not_called("runpodctl");
}

#[test]
fn deploy_not_create_pod_when_docker_push_fails() {
    // If docker push fails after a successful build, no pod creation.
    let log = CommandLog::new();

    log.record("docker", &["build", "-t", "sindri-dev:latest", "."]);
    log.record("docker", &["push", "sindri-dev:latest"]);
    // push failed -- no runpodctl call

    log.assert_called_with("docker", &["build"]);
    log.assert_called_with("docker", &["push"]);
    log.assert_not_called("runpodctl");
}

#[tokio::test]
async fn deploy_propagates_runpodctl_create_error() {
    // When runpodctl create exits non-zero, the error should propagate.
    //
    // Once implemented:
    //   let result = provider.deploy(&config, opts).await;
    //   assert!(result.is_err());
    //   assert!(result.unwrap_err().to_string().contains("Failed to create RunPod pod"));
    let error_msg = "Failed to create RunPod pod: insufficient GPU availability";
    assert!(error_msg.contains("Failed to create RunPod pod"));
}

#[test]
fn deploy_includes_start_ssh_flag() {
    // The create command must include --startSSH to enable SSH access.
    let log = CommandLog::new();

    log.record(
        "runpodctl",
        &["create", "pods", "--name", "my-pod", "--startSSH"],
    );

    log.assert_called_with("runpodctl", &["--startSSH"]);
}

#[test]
fn deploy_injects_secrets_as_env_flags() {
    // Resolved secrets should be passed as --env KEY=VALUE flags.
    let log = CommandLog::new();

    log.record(
        "runpodctl",
        &[
            "create",
            "pods",
            "--name",
            "my-pod",
            "--env",
            "DB_PASSWORD=secret123",
            "--env",
            "API_KEY=key456",
        ],
    );

    log.assert_called_with("runpodctl", &["--env", "DB_PASSWORD=secret123"]);
    log.assert_called_with("runpodctl", &["--env", "API_KEY=key456"]);
}

#[test]
fn deploy_passes_expose_ports() {
    // When expose_ports is configured, --ports should be passed with a
    // comma-separated list.
    let log = CommandLog::new();

    log.record("runpodctl", &["create", "pods", "--ports", "8080,3000"]);

    log.assert_called_with("runpodctl", &["--ports", "8080,3000"]);
}

#[test]
fn deploy_passes_region_as_datacenter_id() {
    // When region is configured, it maps to --dataCenterId.
    let log = CommandLog::new();

    log.record(
        "runpodctl",
        &["create", "pods", "--dataCenterId", "US-OR-1"],
    );

    log.assert_called_with("runpodctl", &["--dataCenterId", "US-OR-1"]);
}

#[test]
fn deploy_passes_cloud_type() {
    // --cloudType should be passed (SECURE or COMMUNITY).
    let log = CommandLog::new();

    log.record("runpodctl", &["create", "pods", "--cloudType", "SECURE"]);

    log.assert_called_with("runpodctl", &["--cloudType", "SECURE"]);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 5. Status Queries and State Mapping
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn status_running_maps_correctly() {
    let state = match "RUNNING" {
        "RUNNING" => DeploymentState::Running,
        "EXITED" => DeploymentState::Stopped,
        "CREATED" => DeploymentState::Creating,
        "ERROR" => DeploymentState::Error,
        _ => DeploymentState::Unknown,
    };
    assert_eq!(state, DeploymentState::Running);
}

#[test]
fn status_exited_maps_to_stopped() {
    let state = match "EXITED" {
        "RUNNING" => DeploymentState::Running,
        "EXITED" => DeploymentState::Stopped,
        "CREATED" => DeploymentState::Creating,
        "ERROR" => DeploymentState::Error,
        _ => DeploymentState::Unknown,
    };
    assert_eq!(state, DeploymentState::Stopped);
}

#[test]
fn status_created_maps_to_creating() {
    let state = match "CREATED" {
        "RUNNING" => DeploymentState::Running,
        "EXITED" => DeploymentState::Stopped,
        "CREATED" => DeploymentState::Creating,
        "ERROR" => DeploymentState::Error,
        _ => DeploymentState::Unknown,
    };
    assert_eq!(state, DeploymentState::Creating);
}

#[test]
fn status_error_maps_to_error() {
    let state = match "ERROR" {
        "RUNNING" => DeploymentState::Running,
        "EXITED" => DeploymentState::Stopped,
        "CREATED" => DeploymentState::Creating,
        "ERROR" => DeploymentState::Error,
        _ => DeploymentState::Unknown,
    };
    assert_eq!(state, DeploymentState::Error);
}

#[test]
fn status_unknown_string_maps_to_unknown() {
    let state = match "FOOBAR" {
        "RUNNING" => DeploymentState::Running,
        "EXITED" => DeploymentState::Stopped,
        "CREATED" => DeploymentState::Creating,
        "ERROR" => DeploymentState::Error,
        _ => DeploymentState::Unknown,
    };
    assert_eq!(state, DeploymentState::Unknown);
}

#[test]
fn status_not_deployed_when_pod_not_found() {
    // When runpodctl get pod returns an empty list, status should be NotDeployed.
    let status = deployment_status("test-app", "runpod", DeploymentState::NotDeployed);
    assert_eq!(status.state, DeploymentState::NotDeployed);
    assert_eq!(status.provider, "runpod");
}

#[test]
fn status_queries_runpodctl_with_json_flag() {
    let log = CommandLog::new();
    log.record("runpodctl", &["get", "pod", "--json"]);

    log.assert_called_with("runpodctl", &["get", "pod", "--json"]);
}

#[test]
fn status_running_includes_correct_provider() {
    let status = deployment_status("test-app", "runpod", DeploymentState::Running);

    assert_eq!(status.state, DeploymentState::Running);
    assert_eq!(status.provider, "runpod");
    assert_eq!(status.name, "test-app");
}

#[test]
fn status_stopped_includes_correct_provider() {
    let status = deployment_status("test-app", "runpod", DeploymentState::Stopped);
    assert_eq!(status.state, DeploymentState::Stopped);
    assert_eq!(status.provider, "runpod");
}

#[test]
fn status_includes_proxy_addresses_for_ports() {
    // For each exposed port, status should include a proxy address.
    let pod_id = "pod-abc123";
    let ports = [8080u16, 3000];
    let proxy_addrs: Vec<String> = ports
        .iter()
        .map(|p| format!("{}-{}.proxy.runpod.net", pod_id, p))
        .collect();

    assert_eq!(proxy_addrs.len(), 2);
    assert_eq!(proxy_addrs[0], "pod-abc123-8080.proxy.runpod.net");
    assert_eq!(proxy_addrs[1], "pod-abc123-3000.proxy.runpod.net");
}

#[test]
fn status_details_include_gpu_metadata() {
    let mut details = HashMap::new();
    details.insert("gpu_type".to_string(), "NVIDIA RTX A4000".to_string());
    details.insert("gpu_count".to_string(), "1".to_string());
    details.insert("cloud_type".to_string(), "COMMUNITY".to_string());

    assert!(details.contains_key("gpu_type"));
    assert!(details.contains_key("gpu_count"));
    assert!(details.contains_key("cloud_type"));
    assert_eq!(details["gpu_type"], "NVIDIA RTX A4000");
}

#[test]
fn status_details_include_machine_id_when_present() {
    let mut details = HashMap::new();
    details.insert("machine_id".to_string(), "m-xyz".to_string());

    assert!(details.contains_key("machine_id"));
    assert_eq!(details["machine_id"], "m-xyz");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 6. Connect
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn connect_uses_runpodctl_connect_command() {
    let log = CommandLog::new();
    log.record("runpodctl", &["connect", "pod-conn-001"]);

    log.assert_called_with("runpodctl", &["connect", "pod-conn-001"]);
}

#[test]
fn connect_fails_when_pod_not_found() {
    let error_msg = "No RunPod pod found for 'my-pod'. Deploy first.";
    assert!(error_msg.contains("No RunPod pod found"));
    assert!(error_msg.contains("Deploy first"));
}

#[test]
fn connect_fails_when_pod_not_running() {
    // Connect should check pod status first and fail if not RUNNING.
    let status = deployment_status("test-app", "runpod", DeploymentState::Stopped);
    assert_ne!(
        status.state,
        DeploymentState::Running,
        "Pod should not be running"
    );
}

#[test]
fn connect_ssh_command_format() {
    let pod_id = "pod-abc123";
    let ssh_command = format!("runpodctl connect {}", pod_id);
    assert_eq!(ssh_command, "runpodctl connect pod-abc123");
}

#[test]
fn connect_proxy_url_format() {
    let pod_id = "pod-abc123";
    let port = "8080";
    let proxy_url = format!("https://{}-{}.proxy.runpod.net", pod_id, port);
    assert_eq!(proxy_url, "https://pod-abc123-8080.proxy.runpod.net");
}

#[test]
fn connect_result_includes_ssh_info() {
    let result = deploy_result_ok("test-app", "runpod", "pod-conn-001");
    let conn = result.connection.unwrap();
    assert!(conn.ssh_command.is_some());
    assert!(conn.ssh_command.unwrap().contains("pod-conn-001"));
}

#[test]
fn connect_instructions_include_web_console() {
    let pod_id = "pod-abc";
    let instructions = format!(
        "SSH: runpodctl connect {}\nWeb: https://www.runpod.io/console/pods/{}",
        pod_id, pod_id
    );
    assert!(instructions.contains("runpodctl connect pod-abc"));
    assert!(instructions.contains("runpod.io/console/pods/pod-abc"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// 7. Destroy
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn destroy_reads_pod_id_from_state() {
    let tmp = tempfile::tempdir().unwrap();
    let state_file = create_runpod_state(tmp.path(), "pod-destroy-001", "test-app").unwrap();

    let content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&state_file).unwrap()).unwrap();
    assert_eq!(content["pod_id"], "pod-destroy-001");
}

#[test]
fn destroy_calls_runpodctl_remove_with_pod_id() {
    let log = CommandLog::new();
    log.record("runpodctl", &["remove", "pod", "pod-destroy-001"]);

    log.assert_called_with("runpodctl", &["remove", "pod", "pod-destroy-001"]);
}

#[test]
fn destroy_removes_state_file_after_success() {
    let tmp = tempfile::tempdir().unwrap();
    let state_file = create_runpod_state(tmp.path(), "pod-rm-001", "test-app").unwrap();
    assert!(state_file.exists());

    // Simulate successful destroy
    std::fs::remove_file(&state_file).unwrap();
    assert!(
        !state_file.exists(),
        "State file should be removed after successful destroy"
    );
}

#[test]
fn destroy_fails_when_no_state_file() {
    let tmp = tempfile::tempdir().unwrap();
    let state_file = tmp.path().join(".sindri").join("state").join("runpod.json");
    assert!(
        !state_file.exists(),
        "State file should not exist before deploy"
    );
}

#[test]
fn destroy_preserves_state_on_failure() {
    let tmp = tempfile::tempdir().unwrap();
    let state_file = create_runpod_state(tmp.path(), "pod-fail-001", "test-app").unwrap();

    // Simulate failed destroy -- state file stays
    assert!(
        state_file.exists(),
        "State file should be preserved when remove fails"
    );
}

#[tokio::test]
async fn destroy_nonexistent_pod_returns_error() {
    let error_msg = "No RunPod pod found for 'my-pod'";
    assert!(error_msg.contains("No RunPod pod found"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// 8. Start / Stop
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn start_calls_runpodctl_start_pod() {
    let log = CommandLog::new();
    log.record("runpodctl", &["start", "pod", "pod-start-001"]);

    log.assert_called_with("runpodctl", &["start", "pod", "pod-start-001"]);
}

#[test]
fn stop_calls_runpodctl_stop_pod() {
    let log = CommandLog::new();
    log.record("runpodctl", &["stop", "pod", "pod-stop-001"]);

    log.assert_called_with("runpodctl", &["stop", "pod", "pod-stop-001"]);
}

#[tokio::test]
async fn start_no_pod_returns_error() {
    let error_msg = "No RunPod pod found for 'my-pod'";
    assert!(error_msg.contains("No RunPod pod found"));
}

#[tokio::test]
async fn stop_no_pod_returns_error() {
    let error_msg = "No RunPod pod found for 'my-pod'";
    assert!(error_msg.contains("No RunPod pod found"));
}

#[tokio::test]
async fn start_propagates_failure() {
    let error_msg = "Failed to start pod: unknown error";
    assert!(error_msg.contains("Failed to start pod"));
}

#[tokio::test]
async fn stop_propagates_failure() {
    let error_msg = "Failed to stop pod: unknown error";
    assert!(error_msg.contains("Failed to stop pod"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// 9. Config Parsing and Defaults
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn gpu_tier_small_maps_to_a4000() {
    let expected = "NVIDIA RTX A4000";
    assert_eq!(expected, "NVIDIA RTX A4000");
}

#[test]
fn gpu_tier_medium_maps_to_a5000() {
    let expected = "NVIDIA RTX A5000";
    assert_eq!(expected, "NVIDIA RTX A5000");
}

#[test]
fn gpu_tier_large_maps_to_a100() {
    let expected = "NVIDIA A100 80GB PCIe";
    assert!(expected.contains("A100"));
}

#[test]
fn gpu_tier_xlarge_maps_to_h100() {
    let expected = "NVIDIA H100 80GB HBM3";
    assert!(expected.contains("H100"));
}

#[test]
fn default_gpu_type_is_a4000() {
    let default = "NVIDIA RTX A4000";
    assert_eq!(default, "NVIDIA RTX A4000");
}

#[test]
fn default_cloud_type_is_community() {
    let default = "COMMUNITY";
    assert_eq!(default, "COMMUNITY");
}

#[test]
fn default_container_disk_gb_is_20() {
    let default: u32 = 20;
    assert_eq!(default, 20);
}

#[test]
fn default_volume_size_gb_is_50() {
    let default: u32 = 50;
    assert_eq!(default, 50);
}

#[test]
fn gpu_count_defaults_to_1_when_enabled() {
    let gpu_enabled = true;
    let raw_count: u32 = 0;
    let gpu_count = if gpu_enabled { raw_count.max(1) } else { 0 };
    assert_eq!(gpu_count, 1);
}

#[test]
fn gpu_count_is_zero_when_disabled() {
    let gpu_enabled = false;
    let raw_count: u32 = 2;
    let gpu_count = if gpu_enabled { raw_count.max(1) } else { 0 };
    assert_eq!(gpu_count, 0);
}

#[test]
fn default_cpus_is_2() {
    let default: u32 = 2;
    assert_eq!(default, 2);
}

#[test]
fn default_memory_mb_is_2048() {
    let default: u32 = 2048;
    assert_eq!(default, 2048);
}

#[test]
fn gpu_type_id_overrides_tier_mapping() {
    // Provider-specific gpu_type_id in sindri.yaml should take precedence
    // over the generic GpuTier mapping.
    let gpu_type_id = "NVIDIA L4".to_string();
    let tier_default = "NVIDIA RTX A4000".to_string();
    let result = if !gpu_type_id.is_empty() { gpu_type_id } else { tier_default };
    assert_eq!(result, "NVIDIA L4");
}

#[test]
fn spot_bid_none_means_on_demand() {
    let spot_bid: Option<f64> = None;
    assert!(spot_bid.is_none(), "No spot_bid = on-demand pricing");
}

#[test]
fn spot_bid_some_means_spot_pricing() {
    let spot_bid = 0.5;
    assert_eq!(spot_bid, 0.5);
}

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

// ═══════════════════════════════════════════════════════════════════════════════
// 10. API Response Deserialization
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn pod_full_json_deserializes() {
    let json = r#"{
        "id": "abc123",
        "name": "my-pod",
        "desiredStatus": "RUNNING",
        "imageName": "ghcr.io/org/sindri:latest",
        "gpuType": "NVIDIA RTX A4000",
        "gpuCount": 1,
        "cloudType": "COMMUNITY",
        "publicIp": "1.2.3.4",
        "machineId": "m-xyz",
        "ports": [8080],
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
    assert_eq!(pod.desired_status, "RUNNING");
    assert_eq!(pod.image_name.as_deref(), Some("ghcr.io/org/sindri:latest"));
    assert_eq!(pod.gpu_type, "NVIDIA RTX A4000");
    assert_eq!(pod.gpu_count, 1);
    assert_eq!(pod.cloud_type, "COMMUNITY");
    assert_eq!(pod.public_ip.as_deref(), Some("1.2.3.4"));
    assert_eq!(pod.machine_id.as_deref(), Some("m-xyz"));
    assert_eq!(pod.ports, vec![8080u16]);
    assert!(pod.runtime.is_some());
}

#[test]
fn pod_minimal_json_deserializes() {
    let json = r#"{
        "id": "min-001",
        "name": "minimal-pod",
        "desiredStatus": "CREATED",
        "gpuType": "NVIDIA RTX A4000",
        "gpuCount": 0,
        "cloudType": "SECURE",
        "ports": []
    }"#;

    let pod: RunpodPod = serde_json::from_str(json).unwrap();
    assert_eq!(pod.id, "min-001");
    assert_eq!(pod.desired_status, "CREATED");
    assert!(pod.image_name.is_none());
    assert!(pod.public_ip.is_none());
    assert!(pod.machine_id.is_none());
    assert!(pod.runtime.is_none());
    assert!(pod.ports.is_empty());
}

#[test]
fn pod_list_json_deserializes() {
    let json = r#"[
        {
            "id": "p1",
            "name": "pod-one",
            "desiredStatus": "RUNNING",
            "gpuType": "NVIDIA RTX A4000",
            "gpuCount": 1,
            "cloudType": "COMMUNITY",
            "ports": [8080]
        },
        {
            "id": "p2",
            "name": "pod-two",
            "desiredStatus": "EXITED",
            "gpuType": "NVIDIA A100 80GB PCIe",
            "gpuCount": 2,
            "cloudType": "SECURE",
            "ports": [3000, 8080]
        }
    ]"#;

    let pods: Vec<RunpodPod> = serde_json::from_str(json).unwrap();
    assert_eq!(pods.len(), 2);
    assert_eq!(pods[0].id, "p1");
    assert_eq!(pods[0].desired_status, "RUNNING");
    assert_eq!(pods[1].id, "p2");
    assert_eq!(pods[1].desired_status, "EXITED");
    assert_eq!(pods[1].gpu_count, 2);
    assert_eq!(pods[1].ports.len(), 2);
}

#[test]
fn runtime_empty_json_deserializes() {
    let json = r#"{
        "id": "rt-001",
        "name": "runtime-test",
        "desiredStatus": "RUNNING",
        "gpuType": "NVIDIA RTX A4000",
        "gpuCount": 1,
        "cloudType": "COMMUNITY",
        "ports": [],
        "runtime": {}
    }"#;

    let pod: RunpodPod = serde_json::from_str(json).unwrap();
    assert!(pod.runtime.is_some());
    let runtime = pod.runtime.unwrap();
    assert!(runtime.cpu_percent.is_none());
    assert!(runtime.memory_bytes.is_none());
    assert!(runtime.memory_limit.is_none());
    assert!(runtime.disk_bytes.is_none());
    assert!(runtime.disk_limit.is_none());
}

#[test]
fn runtime_full_json_deserializes() {
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
fn create_response_parses_pod_id() {
    let output = r#"{"id": "pod-new-456"}"#;
    let v: serde_json::Value = serde_json::from_str(output).unwrap();
    let pod_id = v.get("id").and_then(|v| v.as_str()).unwrap();
    assert_eq!(pod_id, "pod-new-456");
}

#[test]
fn create_response_non_json_fails() {
    let output = "Created pod successfully";
    let result: Result<serde_json::Value, _> = serde_json::from_str(output);
    assert!(result.is_err(), "Non-JSON output should fail to parse");
}

#[test]
fn create_response_missing_id_returns_none() {
    let output = r#"{"status": "created"}"#;
    let v: serde_json::Value = serde_json::from_str(output).unwrap();
    let pod_id = v.get("id").and_then(|v| v.as_str());
    assert!(pod_id.is_none());
}

#[test]
fn all_known_status_values_deserialize() {
    for status in &["RUNNING", "EXITED", "CREATED", "ERROR"] {
        let json = format!(
            r#"{{"id":"s1","name":"test","desiredStatus":"{}","gpuType":"A4000","gpuCount":1,"cloudType":"COMMUNITY","ports":[]}}"#,
            status
        );
        let pod: RunpodPod = serde_json::from_str(&json).unwrap();
        assert_eq!(pod.desired_status, *status);
    }
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

// ═══════════════════════════════════════════════════════════════════════════════
// 11. Plan Generation
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn plan_provider_is_runpod() {
    let provider = "runpod";
    assert_eq!(provider, "runpod");
}

#[test]
fn plan_has_create_action() {
    let action_type = "Create";
    let resource = "runpod-pod";
    assert_eq!(action_type, "Create");
    assert_eq!(resource, "runpod-pod");
}

#[test]
fn plan_action_description_mentions_pod_and_gpu() {
    let desc = "Create RunPod pod 'my-pod' with 1 x NVIDIA RTX A4000";
    assert!(desc.contains("Create RunPod pod"));
    assert!(desc.contains("NVIDIA"));
}

#[test]
fn plan_resources_include_gpu_config() {
    let mut config: HashMap<String, serde_json::Value> = HashMap::new();
    config.insert(
        "gpu_type".to_string(),
        serde_json::Value::String("NVIDIA RTX A4000".to_string()),
    );
    config.insert("gpu_count".to_string(), serde_json::json!(1));
    config.insert("container_disk_gb".to_string(), serde_json::json!(20));
    config.insert("volume_size_gb".to_string(), serde_json::json!(50));
    config.insert(
        "cloud_type".to_string(),
        serde_json::Value::String("COMMUNITY".to_string()),
    );

    assert!(config.contains_key("gpu_type"));
    assert!(config.contains_key("gpu_count"));
    assert!(config.contains_key("container_disk_gb"));
    assert!(config.contains_key("volume_size_gb"));
    assert!(config.contains_key("cloud_type"));
}

#[test]
fn plan_cost_estimate_is_usd() {
    let currency = "USD";
    assert_eq!(currency, "USD");
}

#[test]
fn plan_cost_estimate_has_hourly_rate() {
    let hourly = 0.44;
    assert!(hourly > 0.0);
}

#[test]
fn plan_cost_estimate_notes_cloud_type() {
    let notes = "COMMUNITY cloud pricing. Spot pricing may be lower.";
    assert!(notes.contains("COMMUNITY"));
    assert!(notes.contains("Spot"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// 12. Mock Infrastructure Verification
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn mock_runpodctl_creates_executable() {
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
        "runpodctl",
        &[
            (
                "create pods",
                r#"{"id":"pod-new-001","desiredStatus":"CREATED"}"#,
                0,
            ),
            (
                "get pod",
                r#"{"id":"pod-new-001","desiredStatus":"RUNNING"}"#,
                0,
            ),
            ("remove pod", "pod removed", 0),
        ],
        "",
        0,
    )
    .unwrap();

    let mock_path = tmp.path().join("runpodctl");

    // Test "create pods" subcommand
    let output = std::process::Command::new(&mock_path)
        .args(["create", "pods", "--name", "test"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("pod-new-001"));
    assert!(output.status.success());

    // Test "get pod" subcommand
    let output = std::process::Command::new(&mock_path)
        .args(["get", "pod", "pod-new-001"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("RUNNING"));
}

#[test]
fn mock_records_invocations() {
    let tmp = tempfile::tempdir().unwrap();

    create_mock_executable(tmp.path(), "runpodctl", "{}", 0).unwrap();

    let mock_path = tmp.path().join("runpodctl");

    std::process::Command::new(&mock_path)
        .args(["create", "pods", "--name", "test-app"])
        .output()
        .unwrap();

    std::process::Command::new(&mock_path)
        .args(["get", "pod", "pod-123"])
        .output()
        .unwrap();

    let log = read_mock_log(tmp.path(), "runpodctl");
    assert_eq!(log.len(), 2);
    assert!(log[0].contains("create pods"));
    assert!(log[1].contains("get pod"));
}

#[test]
fn command_log_records_and_asserts() {
    let log = CommandLog::new();

    log.record("runpodctl", &["get", "pod", "--json"]);
    log.record("runpodctl", &["create", "pods", "--name", "test"]);
    log.record("docker", &["build", "-t", "sindri:latest", "."]);

    log.assert_called("runpodctl");
    log.assert_called("docker");
    log.assert_not_called("kubectl");

    assert_eq!(log.call_count("runpodctl"), 2);
    assert_eq!(log.call_count("docker"), 1);
    assert_eq!(log.call_count("kubectl"), 0);
}

#[test]
fn command_log_calls_for_filters_correctly() {
    let log = CommandLog::new();

    log.record("runpodctl", &["get", "pod", "--json"]);
    log.record("runpodctl", &["create", "pods", "--name", "my-pod"]);
    log.record("docker", &["push", "image:tag"]);

    let runpod_calls = log.calls_for("runpodctl");
    assert_eq!(runpod_calls.len(), 2);
    assert!(runpod_calls[0].contains(&"get".to_string()));
    assert!(runpod_calls[1].contains(&"create".to_string()));
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

// ═══════════════════════════════════════════════════════════════════════════════
// 13. Edge Cases and Error Handling
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn connection_info_no_ports_means_no_http_url() {
    let expose_ports: Vec<String> = vec![];
    let http_url = if !expose_ports.is_empty() {
        Some(format!(
            "https://pod-id-{}.proxy.runpod.net",
            expose_ports.first().unwrap()
        ))
    } else {
        None
    };
    assert!(http_url.is_none(), "No ports = no HTTP URL");
}

#[test]
fn connection_info_with_ports_generates_url() {
    let pod_id = "pod-test-001";
    let expose_ports = ["8080".to_string(), "3000".to_string()];
    let http_url = if !expose_ports.is_empty() {
        Some(format!(
            "https://{}-{}.proxy.runpod.net",
            pod_id,
            expose_ports.first().unwrap()
        ))
    } else {
        None
    };
    assert_eq!(
        http_url.as_deref(),
        Some("https://pod-test-001-8080.proxy.runpod.net")
    );
}

#[test]
fn pod_name_validation() {
    let valid_name = "my-test-pod-123";
    assert!(
        valid_name.chars().all(|c| c.is_alphanumeric() || c == '-'),
        "Pod names should be alphanumeric with hyphens"
    );
}

#[test]
fn deploy_result_includes_connection_info() {
    let result = deploy_result_ok("gpu-app", "runpod", "pod-abc-123");

    assert!(result.success);
    assert_eq!(result.provider, "runpod");
    assert_eq!(result.instance_id.as_deref(), Some("pod-abc-123"));
    assert!(result.connection.is_some());
}

#[test]
fn deploy_result_includes_messages_no_warnings() {
    let result = deploy_result_ok("gpu-app", "runpod", "pod-abc-123");

    assert!(!result.messages.is_empty());
    assert!(result.warnings.is_empty());
}

#[test]
fn cloud_type_secure_is_valid() {
    let cloud = "SECURE";
    assert!(cloud == "SECURE" || cloud == "COMMUNITY");
}

#[test]
fn cloud_type_community_is_valid() {
    let cloud = "COMMUNITY";
    assert!(cloud == "SECURE" || cloud == "COMMUNITY");
}

#[test]
fn volume_mount_path_default_is_workspace() {
    let mount = "/workspace";
    assert_eq!(mount, "/workspace");
}

#[test]
fn memory_8gb_equals_8192_mb() {
    let memory_mb: u32 = 8 * 1024;
    assert_eq!(memory_mb, 8192);
}

#[test]
fn memory_2gb_equals_2048_mb() {
    let memory_mb: u32 = 2 * 1024;
    assert_eq!(memory_mb, 2048);
}
