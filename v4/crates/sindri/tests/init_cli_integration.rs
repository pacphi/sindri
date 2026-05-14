//! Phase 4 (Docs/Impl reconciliation) — integration coverage for `sindri init`
//! and `sindri policy use`. Exercises the binary end-to-end via
//! `std::process::Command`.
//!
//! Covered behaviors (see `v4/docs/plan/2026-04-30-docs-impl-reconciliation.md`
//! Phase 4):
//!   * F-CLI-08: `--non-interactive` produces no prompts and accepts defaults.
//!   * F-CLI-09: `init --policy strict` writes `./sindri.policy.yaml`
//!     (project-scoped) by default. `--global` redirects to the global file.
//!   * F-CLI-10: `.gitignore` contains `.sindri/` but NOT `sindri.*.lock`.
//!   * F-CLI-11: unknown template exits non-zero with a "Available
//!     templates: …" message.
//!   * F-CLI-25: `policy use <preset>` writes `./sindri.policy.yaml` by
//!     default and `~/.sindri/policy.yaml` under `--global`.
//!   * F-XCUT-02: emitted `sindri.yaml` references the transitional
//!     `raw.githubusercontent.com` schema URL.

use sindri_core::well_known::{bom_schema_url, PROJECT_MANIFEST_FILENAME, PROJECT_POLICY_FILENAME};
use std::path::{Path, PathBuf};
use std::process::Command;

fn sindri_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sindri"))
}

/// Produce a Command rooted in `cwd` with `HOME` redirected to a sandbox so
/// we never touch the developer's real `~/.sindri`.
fn cmd_in(cwd: &Path, fake_home: &Path) -> Command {
    let mut cmd = Command::new(sindri_bin());
    cmd.current_dir(cwd)
        .env("HOME", fake_home)
        .env("SINDRI_HOME", fake_home) // Windows ignores HOME; SINDRI_HOME is the portable override
        .env_remove("SINDRI_BIN_PATH");
    cmd
}

#[test]
fn init_non_interactive_writes_manifest_and_gitignore() {
    let dir = tempfile::tempdir().expect("tempdir");
    let home = tempfile::tempdir().expect("home");
    let out = cmd_in(dir.path(), home.path())
        .args(["init", "--non-interactive"])
        .output()
        .expect("run sindri init");
    assert!(
        out.status.success(),
        "init failed: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let manifest =
        std::fs::read_to_string(dir.path().join(PROJECT_MANIFEST_FILENAME)).expect("manifest");
    // F-XCUT-02: schema pragmas reflect the build-time-configured URL.
    let expected_url = bom_schema_url();
    assert!(
        manifest.contains(&expected_url),
        "manifest missing expected schema URL '{}': {}",
        expected_url,
        manifest
    );
    assert!(manifest.contains("yaml-language-server"));
    assert!(manifest.contains("@schema"));

    // F-CLI-10: .gitignore lists .sindri/ only — no sindri.*.lock line.
    let gi = std::fs::read_to_string(dir.path().join(".gitignore")).expect("gitignore");
    assert!(
        gi.contains(".sindri/"),
        "gitignore missing .sindri/: {}",
        gi
    );
    assert!(
        !gi.contains("sindri.*.lock"),
        "gitignore must not list sindri.*.lock: {}",
        gi
    );
}

#[test]
fn init_unknown_template_errors_with_available_list() {
    let dir = tempfile::tempdir().expect("tempdir");
    let home = tempfile::tempdir().expect("home");
    let out = cmd_in(dir.path(), home.path())
        .args(["init", "--non-interactive", "--template", "bogus"])
        .output()
        .expect("run sindri init");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("bogus") && stderr.contains("minimal") && stderr.contains("anthropic-dev"),
        "unexpected stderr: {}",
        stderr
    );
    // Manifest was not written.
    assert!(!dir.path().join(PROJECT_MANIFEST_FILENAME).exists());
}

#[test]
fn init_policy_strict_writes_project_file_by_default() {
    let dir = tempfile::tempdir().expect("tempdir");
    let home = tempfile::tempdir().expect("home");
    let out = cmd_in(dir.path(), home.path())
        .args(["init", "--non-interactive", "--policy", "strict"])
        .output()
        .expect("run sindri init");
    assert!(out.status.success());
    // Project file present; global file not created.
    assert!(dir.path().join(PROJECT_POLICY_FILENAME).exists());
    assert!(!home.path().join(".sindri").join("policy.yaml").exists());
}

#[test]
fn init_policy_with_global_flag_writes_to_home() {
    let dir = tempfile::tempdir().expect("tempdir");
    let home = tempfile::tempdir().expect("home");
    let out = cmd_in(dir.path(), home.path())
        .args([
            "init",
            "--non-interactive",
            "--policy",
            "strict",
            "--global",
        ])
        .output()
        .expect("run sindri init");
    assert!(out.status.success());
    assert!(!dir.path().join(PROJECT_POLICY_FILENAME).exists());
    assert!(home.path().join(".sindri").join("policy.yaml").exists());
}

#[test]
fn init_policy_none_does_not_create_policy_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let home = tempfile::tempdir().expect("home");
    let out = cmd_in(dir.path(), home.path())
        .args(["init", "--non-interactive", "--policy", "none"])
        .output()
        .expect("run sindri init");
    assert!(out.status.success());
    assert!(!dir.path().join(PROJECT_POLICY_FILENAME).exists());
    assert!(!home.path().join(".sindri").join("policy.yaml").exists());
}

#[test]
fn init_existing_manifest_without_force_errors() {
    let dir = tempfile::tempdir().expect("tempdir");
    let home = tempfile::tempdir().expect("home");
    std::fs::write(
        dir.path().join(PROJECT_MANIFEST_FILENAME),
        "name: existing\n",
    )
    .unwrap();
    let out = cmd_in(dir.path(), home.path())
        .args(["init", "--non-interactive"])
        .output()
        .expect("run sindri init");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("already exists"), "stderr: {}", stderr);
}

#[test]
fn policy_use_writes_project_file_by_default() {
    let dir = tempfile::tempdir().expect("tempdir");
    let home = tempfile::tempdir().expect("home");
    let out = cmd_in(dir.path(), home.path())
        .args(["policy", "use", "strict"])
        .output()
        .expect("run sindri policy use");
    assert!(
        out.status.success(),
        "policy use failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(dir.path().join(PROJECT_POLICY_FILENAME).exists());
    assert!(!home.path().join(".sindri").join("policy.yaml").exists());
}

#[test]
fn policy_use_global_flag_writes_to_home() {
    let dir = tempfile::tempdir().expect("tempdir");
    let home = tempfile::tempdir().expect("home");
    let out = cmd_in(dir.path(), home.path())
        .args(["policy", "use", "offline", "--global"])
        .output()
        .expect("run sindri policy use");
    assert!(out.status.success());
    assert!(!dir.path().join(PROJECT_POLICY_FILENAME).exists());
    assert!(home.path().join(".sindri").join("policy.yaml").exists());
}
