//! Phase 5 (F-POL-04) — CLI flag-line coverage for `sindri resolve --allow`.
//!
//! Exercises only the value-parsing surface; the in-memory policy
//! extension and ledger emission are unit-tested in
//! `sindri-resolver::license_override` and
//! `sindri-resolver::policy_ledger`. A full happy-path resolve test
//! requires a registry-cache fixture and is out of scope here.

use std::path::PathBuf;
use std::process::Command;

fn sindri_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sindri"))
}

#[test]
fn allow_without_equals_exits_with_clear_error() {
    let dir = tempfile::tempdir().unwrap();
    let out = Command::new(sindri_bin())
        .args(["resolve", "--allow", "MPL-2.0"])
        .current_dir(dir.path())
        .output()
        .expect("run sindri resolve");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("reason"),
        "expected reason-mandatory hint, got: {}",
        stderr
    );
}

#[test]
fn allow_with_empty_reason_exits_with_clear_error() {
    let dir = tempfile::tempdir().unwrap();
    let out = Command::new(sindri_bin())
        .args(["resolve", "--allow", "MPL-2.0="])
        .current_dir(dir.path())
        .output()
        .expect("run sindri resolve");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("reason"),
        "expected reason-empty hint, got: {}",
        stderr
    );
}

#[test]
fn allow_with_empty_license_exits_with_clear_error() {
    let dir = tempfile::tempdir().unwrap();
    let out = Command::new(sindri_bin())
        .args(["resolve", "--allow", "=ticket-123"])
        .current_dir(dir.path())
        .output()
        .expect("run sindri resolve");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("license"),
        "expected license-empty hint, got: {}",
        stderr
    );
}

#[test]
fn allow_parse_runs_before_manifest_check() {
    // No sindri.yaml exists in the temp dir. With a malformed --allow,
    // the parser fires first and the user sees the parse error rather
    // than the manifest-missing error.
    let dir = tempfile::tempdir().unwrap();
    let out = Command::new(sindri_bin())
        .args(["resolve", "--allow", "bad-value"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("invalid --allow"),
        "should surface --allow parse error first: {}",
        stderr
    );
    assert!(!stderr.contains("Manifest not found"));
}

#[test]
fn allow_valid_value_parses_and_proceeds_to_manifest_check() {
    // No manifest present, valid --allow value. Should reach the
    // manifest check and fail with that error, NOT the --allow parser.
    let dir = tempfile::tempdir().unwrap();
    let out = Command::new(sindri_bin())
        .args(["resolve", "--allow", "MPL-2.0=ticket-123"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("Manifest not found") || stderr.contains("sindri.yaml"),
        "expected manifest-missing path, got: {}",
        stderr
    );
}
