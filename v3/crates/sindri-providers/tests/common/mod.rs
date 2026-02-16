//! Common test helpers for sindri-providers integration tests
//!
//! Provides mock infrastructure following the London School TDD approach:
//! - Mock Provider trait implementations for verifying interactions
//! - Process mock helpers for simulating CLI tool execution
//! - SindriConfig fixture builders for test scenarios

use anyhow::Result;
use sindri_core::config::SindriConfig;
use sindri_core::types::{
    ConnectionInfo, DeployResult, DeploymentState, DeploymentStatus, DeploymentTimestamps,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

// ─── Process Mock Infrastructure ─────────────────────────────────────────────

/// Records of CLI command invocations for verification.
/// Each entry is (program, args).
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct CommandLog {
    #[allow(clippy::type_complexity)]
    pub calls: Arc<Mutex<Vec<(String, Vec<String>)>>>,
}

impl CommandLog {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Record a command invocation.
    #[allow(dead_code)]
    pub fn record(&self, program: &str, args: &[&str]) {
        let mut calls = self.calls.lock().unwrap();
        calls.push((
            program.to_string(),
            args.iter().map(|s| s.to_string()).collect(),
        ));
    }

    /// Assert the command was called at least once.
    #[allow(dead_code)]
    pub fn assert_called(&self, program: &str) {
        let calls = self.calls.lock().unwrap();
        assert!(
            calls.iter().any(|(p, _)| p == program),
            "'{}' was never called. Actual calls: {:?}",
            program,
            *calls
        );
    }

    /// Assert the command was called with specific argument substrings.
    #[allow(dead_code)]
    pub fn assert_called_with(&self, program: &str, expected_args: &[&str]) {
        let calls = self.calls.lock().unwrap();
        let matching = calls.iter().any(|(p, args)| {
            p == program
                && expected_args
                    .iter()
                    .all(|expected| args.iter().any(|actual| actual.contains(expected)))
        });
        assert!(
            matching,
            "'{}' was not called with args containing {:?}. Actual calls: {:?}",
            program, expected_args, *calls
        );
    }

    /// Assert the command was NOT called.
    #[allow(dead_code)]
    pub fn assert_not_called(&self, program: &str) {
        let calls = self.calls.lock().unwrap();
        assert!(
            !calls.iter().any(|(p, _)| p == program),
            "'{}' was called but should not have been. Actual calls: {:?}",
            program,
            *calls
        );
    }

    /// Get the total number of calls for a specific program.
    #[allow(dead_code)]
    pub fn call_count(&self, program: &str) -> usize {
        let calls = self.calls.lock().unwrap();
        calls.iter().filter(|(p, _)| p == program).count()
    }

    /// Get all calls for a specific program.
    #[allow(dead_code)]
    pub fn calls_for(&self, program: &str) -> Vec<Vec<String>> {
        let calls = self.calls.lock().unwrap();
        calls
            .iter()
            .filter(|(p, _)| p == program)
            .map(|(_, args)| args.clone())
            .collect()
    }
}

// ─── Process Mock Scripts ────────────────────────────────────────────────────

/// Creates a mock executable script in a temp directory.
/// Returns the directory path that should be prepended to PATH.
///
/// The mock script:
/// 1. Logs the invocation to a file for later verification
/// 2. Prints the configured stdout
/// 3. Exits with the configured code
pub fn create_mock_executable(
    dir: &std::path::Path,
    name: &str,
    stdout: &str,
    exit_code: i32,
) -> Result<()> {
    let script_path = dir.join(name);
    let log_path = dir.join(format!("{}.log", name));

    let script = format!(
        r#"#!/bin/sh
echo "$0 $*" >> "{log}"
cat <<'MOCK_OUTPUT'
{stdout}
MOCK_OUTPUT
exit {exit_code}
"#,
        log = log_path.display(),
        stdout = stdout,
        exit_code = exit_code,
    );

    std::fs::write(&script_path, script)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))?;
    }

    Ok(())
}

/// Creates a conditional mock that returns different output based on arguments.
/// Patterns are matched as substring checks on the full argument string.
pub fn create_conditional_mock(
    dir: &std::path::Path,
    name: &str,
    conditions: &[(&str, &str, i32)], // (arg_pattern, stdout, exit_code)
    default_stdout: &str,
    default_exit: i32,
) -> Result<()> {
    let script_path = dir.join(name);
    let log_path = dir.join(format!("{}.log", name));

    let mut script = format!(
        r#"#!/bin/sh
echo "$0 $*" >> "{log}"
ALL_ARGS="$*"
"#,
        log = log_path.display(),
    );

    for (i, (pattern, stdout, exit_code)) in conditions.iter().enumerate() {
        let keyword = if i == 0 { "if" } else { "elif" };
        script.push_str(&format!(
            r#"{keyword} echo "$ALL_ARGS" | grep -qF -- '{pattern}'; then
  cat <<'MOCK_OUT_{i}'
{stdout}
MOCK_OUT_{i}
  exit {exit_code}
"#,
            keyword = keyword,
            pattern = pattern,
            stdout = stdout,
            exit_code = exit_code,
            i = i,
        ));
    }

    script.push_str(&format!(
        r#"else
  cat <<'MOCK_DEFAULT'
{default_stdout}
MOCK_DEFAULT
  exit {default_exit}
fi
"#,
        default_stdout = default_stdout,
        default_exit = default_exit,
    ));

    std::fs::write(&script_path, script)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))?;
    }

    Ok(())
}

/// Read the invocation log for a mock executable.
pub fn read_mock_log(dir: &std::path::Path, name: &str) -> Vec<String> {
    let log_path = dir.join(format!("{}.log", name));
    if log_path.exists() {
        std::fs::read_to_string(&log_path)
            .unwrap_or_default()
            .lines()
            .map(|s| s.to_string())
            .collect()
    } else {
        vec![]
    }
}

// ─── SindriConfig Fixtures ───────────────────────────────────────────────────

/// Creates a temporary directory with a minimal sindri.yaml for the Docker provider.
/// Returns (TempDir, SindriConfig) -- keep TempDir alive for the test duration.
#[allow(dead_code)]
pub fn create_docker_config(name: &str) -> Result<(TempDir, SindriConfig)> {
    let tmp = tempfile::tempdir()?;
    let yaml_path = tmp.path().join("sindri.yaml");
    let yaml_content = format!(
        r#"version: "3.0"
name: {name}
deployment:
  provider: docker
  image: "ghcr.io/pacphi/sindri:latest"
  resources:
    memory: "4GB"
    cpus: 2
  volumes:
    home:
      size: "10GB"
extensions:
  profile: minimal
"#,
        name = name,
    );
    std::fs::write(&yaml_path, yaml_content)?;

    let config = SindriConfig::load(Some(camino::Utf8Path::new(yaml_path.to_str().unwrap())))?;

    Ok((tmp, config))
}

/// Creates a temporary directory with a RunPod provider sindri.yaml.
/// The RunPod provider doesn't exist yet, so we use Docker as the
/// base provider type and include RunPod-specific config in the providers block.
#[allow(dead_code)]
pub fn create_runpod_config_fixture(
    name: &str,
    gpu_type: &str,
    gpu_count: u32,
    container_disk_gb: u32,
    volume_size_gb: u32,
) -> Result<(TempDir, PathBuf)> {
    let tmp = tempfile::tempdir()?;
    let yaml_path = tmp.path().join("sindri.yaml");

    // Write a YAML that will be used by the RunPod provider
    // The provider reads these fields directly from the config
    let yaml_content = format!(
        r#"version: "3.0"
name: {name}
deployment:
  provider: docker
  image: "sindri-dev:latest"
  resources:
    memory: "8GB"
    cpus: 4
  volumes:
    home:
      size: "{volume_size_gb}GB"
extensions:
  profile: minimal
providers:
  runpod:
    gpu_type: "{gpu_type}"
    gpu_count: {gpu_count}
    container_disk_gb: {container_disk_gb}
    volume_size_gb: {volume_size_gb}
    cloud_type: "COMMUNITY"
    volume_mount_path: "/workspace"
"#,
        name = name,
        gpu_type = gpu_type,
        gpu_count = gpu_count,
        container_disk_gb = container_disk_gb,
        volume_size_gb = volume_size_gb,
    );

    std::fs::write(&yaml_path, &yaml_content)?;
    let path = yaml_path.clone();
    Ok((tmp, path))
}

/// Creates a temporary directory with a Northflank provider sindri.yaml.
#[allow(dead_code)]
pub fn create_northflank_config_fixture(
    name: &str,
    project_name: &str,
    service_name: &str,
    compute_plan: &str,
) -> Result<(TempDir, PathBuf)> {
    let tmp = tempfile::tempdir()?;
    let yaml_path = tmp.path().join("sindri.yaml");

    let yaml_content = format!(
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
      size: "10GB"
extensions:
  profile: minimal
providers:
  northflank:
    project_name: "{project_name}"
    service_name: "{service_name}"
    compute_plan: "{compute_plan}"
    instances: 1
    volume_size_gb: 10
    volume_mount_path: "/workspace"
"#,
        name = name,
        project_name = project_name,
        service_name = service_name,
        compute_plan = compute_plan,
    );

    std::fs::write(&yaml_path, &yaml_content)?;
    let path = yaml_path.clone();
    Ok((tmp, path))
}

// ─── State File Helpers ──────────────────────────────────────────────────────

/// Creates a RunPod state file in the given directory.
#[allow(dead_code)]
pub fn create_runpod_state(dir: &std::path::Path, pod_id: &str, app_name: &str) -> Result<PathBuf> {
    let state_dir = dir.join(".sindri").join("state");
    std::fs::create_dir_all(&state_dir)?;
    let state_file = state_dir.join("runpod.json");

    let state = serde_json::json!({
        "pod_id": pod_id,
        "app_name": app_name,
        "gpu_type": "NVIDIA RTX A4000",
        "created_at": "2026-01-15T10:00:00Z"
    });

    std::fs::write(&state_file, serde_json::to_string_pretty(&state)?)?;
    Ok(state_file)
}

/// Creates a Northflank state file in the given directory.
#[allow(dead_code)]
pub fn create_northflank_state(
    dir: &std::path::Path,
    project_name: &str,
    service_name: &str,
) -> Result<PathBuf> {
    let state_dir = dir.join(".sindri").join("state");
    std::fs::create_dir_all(&state_dir)?;
    let state_file = state_dir.join("northflank.json");

    let state = serde_json::json!({
        "project_name": project_name,
        "service_name": service_name,
        "service_id": service_name.to_lowercase().replace(' ', "-"),
        "compute_plan": "nf-compute-50",
        "created_at": "2026-01-15T10:00:00Z"
    });

    std::fs::write(&state_file, serde_json::to_string_pretty(&state)?)?;
    Ok(state_file)
}

// ─── Result Builders ─────────────────────────────────────────────────────────

/// Build a successful DeployResult for test assertions.
#[allow(dead_code)]
pub fn deploy_result_ok(name: &str, provider: &str, instance_id: &str) -> DeployResult {
    DeployResult {
        success: true,
        name: name.to_string(),
        provider: provider.to_string(),
        instance_id: Some(instance_id.to_string()),
        connection: Some(ConnectionInfo {
            ssh_command: Some(format!("ssh root@{}", instance_id)),
            http_url: None,
            https_url: None,
            instructions: None,
        }),
        messages: vec![format!("Deployed {} on {}", name, provider)],
        warnings: vec![],
    }
}

/// Build a DeploymentStatus for test assertions.
#[allow(dead_code)]
pub fn deployment_status(name: &str, provider: &str, state: DeploymentState) -> DeploymentStatus {
    DeploymentStatus {
        name: name.to_string(),
        provider: provider.to_string(),
        state,
        instance_id: Some("test-instance-001".to_string()),
        image: Some("sindri-dev:latest".to_string()),
        addresses: vec![],
        resources: None,
        timestamps: DeploymentTimestamps::default(),
        details: HashMap::new(),
    }
}
