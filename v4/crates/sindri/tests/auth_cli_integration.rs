//! Integration smoke tests for the Phase 5 (ADR-027) auth CLI verbs.
//!
//! These exercise the binary end-to-end via `std::process::Command` to
//! cover both human-readable and `--json` output paths plus exit codes.
//! The fixtures use the integration scenario described in the auth-aware
//! plan: 3 components × 2 targets, mix of bound / deferred / failed.

use std::path::PathBuf;
use std::process::Command;

fn sindri_bin() -> PathBuf {
    // CARGO_BIN_EXE_<name> is set by Cargo for integration tests.
    PathBuf::from(env!("CARGO_BIN_EXE_sindri"))
}

fn write_fixture_lockfile(dir: &std::path::Path, name: &str, contents: &str) {
    std::fs::write(dir.join(name), contents).unwrap();
}

fn write_fixture_manifest(dir: &std::path::Path, contents: &str) {
    std::fs::write(dir.join("sindri.yaml"), contents).unwrap();
}

/// 3 components × 2 targets: mix of bound / deferred / failed.
fn scenario_lockfile_json() -> String {
    serde_json::json!({
        "version": 1,
        "bom_hash": "abc",
        "target": "local",
        "components": [],
        "auth_bindings": [
            {
                "id": "0000000000000001",
                "component": "npm:claude-code",
                "requirement": "anthropic_api_key",
                "audience": "urn:anthropic:api",
                "target": "local",
                "source": { "kind": "from-env", "var": "ANTHROPIC_API_KEY" },
                "priority": 100,
                "status": "bound"
            },
            {
                "id": "0000000000000002",
                "component": "npm:codex",
                "requirement": "openai_api_key",
                "audience": "urn:openai:api",
                "target": "local",
                "priority": 0,
                "status": "deferred",
                "reason": "no source matched (optional)"
            },
            {
                "id": "0000000000000003",
                "component": "brew:gh",
                "requirement": "github_token",
                "audience": "https://api.github.com",
                "target": "local",
                "priority": 0,
                "status": "failed",
                "reason": "no source matched (required)",
                "considered": [
                    { "capability-id": "wrong-aud", "source-kind": "from-env", "reason": "audience-mismatch" }
                ]
            }
        ]
    })
    .to_string()
}

fn scenario_manifest_yaml() -> &'static str {
    r#"
name: phase5-it
components: []
targets:
  local:
    kind: local
"#
}

#[test]
fn auth_show_human_lists_all_three_bindings() {
    let dir = tempfile::tempdir().unwrap();
    write_fixture_manifest(dir.path(), scenario_manifest_yaml());
    write_fixture_lockfile(dir.path(), "sindri.lock", &scenario_lockfile_json());

    let out = Command::new(sindri_bin())
        .args(["auth", "show"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("npm:claude-code"));
    assert!(stdout.contains("npm:codex"));
    assert!(stdout.contains("brew:gh"));
    assert!(stdout.contains("bound"));
    assert!(stdout.contains("deferred"));
    assert!(stdout.contains("failed"));
    assert!(stdout.contains("audience-mismatch"));
}

#[test]
fn auth_show_filters_by_component() {
    let dir = tempfile::tempdir().unwrap();
    write_fixture_manifest(dir.path(), scenario_manifest_yaml());
    write_fixture_lockfile(dir.path(), "sindri.lock", &scenario_lockfile_json());

    let out = Command::new(sindri_bin())
        .args(["auth", "show", "npm:claude-code"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("npm:claude-code"));
    assert!(!stdout.contains("brew:gh"));
}

#[test]
fn auth_show_json_emits_stable_shape() {
    let dir = tempfile::tempdir().unwrap();
    write_fixture_manifest(dir.path(), scenario_manifest_yaml());
    write_fixture_lockfile(dir.path(), "sindri.lock", &scenario_lockfile_json());

    let out = Command::new(sindri_bin())
        .args(["auth", "show", "--json"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Stable JSON shape: { "target": "...", "bindings": [...] }
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(parsed["target"], "local");
    assert_eq!(parsed["bindings"].as_array().unwrap().len(), 3);
    let first = &parsed["bindings"][0];
    // Required fields per CLI.md schema.
    for field in [
        "id",
        "component",
        "requirement",
        "audience",
        "target",
        "status",
    ] {
        assert!(first.get(field).is_some(), "missing field {}", field);
    }
}

#[test]
fn auth_show_missing_lockfile_exits_nonzero() {
    let dir = tempfile::tempdir().unwrap();
    write_fixture_manifest(dir.path(), scenario_manifest_yaml());

    let out = Command::new(sindri_bin())
        .args(["auth", "show", "--json"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("LOCKFILE_NOT_FOUND") || stdout.contains("error"));
}

#[test]
fn auth_refresh_writes_lockfile_and_reports_counts() {
    // Refresh without component manifests is a no-op on the binding
    // pass (resolver needs ComponentManifests to bind), but it must
    // round-trip the lockfile and exit zero.
    let dir = tempfile::tempdir().unwrap();
    write_fixture_manifest(dir.path(), scenario_manifest_yaml());
    write_fixture_lockfile(dir.path(), "sindri.lock", &scenario_lockfile_json());

    let out = Command::new(sindri_bin())
        .args(["auth", "refresh", "--json"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(parsed["refreshed"], true);
    assert!(parsed["auth_bindings"].is_object());
}

#[test]
fn doctor_auth_clean_lockfile_exits_zero() {
    // Build a lockfile with only Bound bindings.
    let dir = tempfile::tempdir().unwrap();
    write_fixture_manifest(dir.path(), scenario_manifest_yaml());
    let lf = serde_json::json!({
        "version": 1,
        "bom_hash": "abc",
        "target": "local",
        "components": [],
        "auth_bindings": [
            {
                "id": "1", "component": "npm:c", "requirement": "t",
                "audience": "u", "target": "local",
                "source": { "kind": "from-env", "var": "X" },
                "priority": 0, "status": "bound"
            }
        ]
    })
    .to_string();
    write_fixture_lockfile(dir.path(), "sindri.lock", &lf);

    let out = Command::new(sindri_bin())
        .args(["doctor", "--auth", "--json"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["gate5"]["allowed"], true);
}

#[test]
fn doctor_auth_failed_binding_in_ci_exits_policy_denied() {
    // Lockfile with a Failed required binding and CI=1 → Gate 5 denies.
    let dir = tempfile::tempdir().unwrap();
    write_fixture_manifest(dir.path(), scenario_manifest_yaml());
    write_fixture_lockfile(dir.path(), "sindri.lock", &scenario_lockfile_json());

    let out = Command::new(sindri_bin())
        .args(["doctor", "--auth", "--json"])
        .env("CI", "1")
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Non-zero exit (EXIT_POLICY_DENIED = 2)
    assert_eq!(out.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert_eq!(parsed["ok"], false);
    assert_eq!(parsed["gate5"]["allowed"], false);
    // Remediation hint must point at one of the new verbs.
    let msg = parsed["gate5"]["message"].as_str().unwrap_or("");
    assert!(msg.contains("Gate 5") || msg.contains("auth"));
}

#[test]
fn target_auth_bind_writes_provides_entry() {
    // Use a manifest that already has a target, plus a lockfile with a
    // Failed binding that has one considered candidate.
    let dir = tempfile::tempdir().unwrap();
    write_fixture_manifest(dir.path(), scenario_manifest_yaml());
    let lf = serde_json::json!({
        "version": 1,
        "bom_hash": "abc",
        "target": "local",
        "components": [],
        "auth_bindings": [
            {
                "id": "deadbeefdeadbeef",
                "component": "brew:gh",
                "requirement": "github_token",
                "audience": "https://api.github.com",
                "target": "local",
                "priority": 0,
                "status": "failed",
                "reason": "no source matched (required)",
                "considered": [
                    {
                        "capability-id": "github_token",
                        "source-kind": "from-env",
                        "reason": "audience-mismatch"
                    }
                ]
            }
        ]
    })
    .to_string();
    write_fixture_lockfile(dir.path(), "sindri.lock", &lf);

    let out = Command::new(sindri_bin())
        .args([
            "target",
            "auth",
            "local",
            "--bind",
            "deadbeefdeadbeef",
            "--json",
        ])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Re-read the manifest and confirm `provides:` was written.
    let yaml = std::fs::read_to_string(dir.path().join("sindri.yaml")).unwrap();
    assert!(yaml.contains("provides"), "manifest after bind:\n{}", yaml);
    assert!(yaml.contains("github_token"));

    // Round-trip parse to confirm it's a valid TargetConfig.
    use sindri_core::manifest::BomManifest;
    let bom: BomManifest = serde_yaml::from_str(&yaml).expect("valid manifest");
    let local = bom.targets.get("local").unwrap();
    assert_eq!(local.provides.len(), 1);
    assert_eq!(local.provides[0].id, "github_token");
}

#[test]
fn completions_bash_emits_completion_script() {
    let out = Command::new(sindri_bin())
        .args(["completions", "bash"])
        .output()
        .unwrap();

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    // clap_complete bash output references the bin name and `_sindri`.
    assert!(stdout.contains("sindri"));
    assert!(stdout.contains("complete"));
}

#[test]
fn completions_unknown_shell_exits_nonzero() {
    let out = Command::new(sindri_bin())
        .args(["completions", "tcsh"])
        .output()
        .unwrap();
    assert!(!out.status.success());
}
