//! CLI black-box integration test: `sindri resolve --offline` runs Gate 1
//! using platform data cached in the lockfile from the previous online resolve.
//!
//! # Scenario (Wave 6A / ADR-008)
//!
//! 1. An online resolve (with platform forced to `linux-aarch64`) writes
//!    `sindri.lock` with `platforms: [linux/x86_64, linux/aarch64]` for the
//!    `shellcheck` component (loaded from the local registry cache's
//!    `component.yaml`).
//! 2. A subsequent `sindri resolve --offline` on a `macos-aarch64` host
//!    reads those platforms from the lockfile, builds a synthetic
//!    `CandidateRef::with_manifest`, and runs Gate 1 -- which must produce
//!    `ADM_PLATFORM_UNSUPPORTED` (exit code 2).
//!
//! This test exercises the full CLI end-to-end path for Wave 6A.
//!
//! ADR-008: Gate 1 (platform eligibility).
//! ADR-002: lockfile schema (additive `platforms` field).

#[path = "helpers.rs"]
mod helpers;

use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use std::path::Path;

/// Write a minimal `sindri.yaml` that references `binary:shellcheck`.
fn write_shellcheck_manifest(dir: &Path) {
    let yaml = "name: offline-gate1-test\ncomponents:\n  - address: \"binary:shellcheck\"\n";
    std::fs::write(dir.join("sindri.yaml"), yaml).expect("write sindri.yaml");
}

/// Populate a fake per-component `component.yaml` alongside the registry
/// `index.yaml` so the online resolver can load platforms at resolve-time.
///
/// Layout written:
/// ```
/// $HOME/.sindri/cache/registries/sindri_core/index.yaml           (from fixture)
/// $HOME/.sindri/cache/registries/sindri_core/components/shellcheck/component.yaml
/// ```
fn write_registry_with_component_manifest(home_dir: &Path) {
    let registry_fixture = helpers::fixture_path("registries/prototype");
    helpers::write_local_registry(home_dir, "sindri_core", &registry_fixture);

    // Also write the per-component component.yaml so the online resolver
    // can load platform constraints and persist them in the lockfile.
    let comp_dir = home_dir
        .join(".sindri")
        .join("cache")
        .join("registries")
        .join("sindri_core")
        .join("components")
        .join("shellcheck");
    std::fs::create_dir_all(&comp_dir).expect("create component dir");

    let component_yaml =
        helpers::fixture_path("registries/prototype/components/shellcheck/component.yaml");
    std::fs::copy(&component_yaml, comp_dir.join("component.yaml"))
        .expect("copy shellcheck component.yaml to cache");
}

/// Stage 1: verify that an online resolve (with linux-aarch64 forced so that
/// the Linux-only shellcheck passes Gate 1) writes a lockfile containing the
/// `platforms` field.
///
/// Using `SINDRI_TEST_PLATFORM_OVERRIDE=linux-aarch64` makes this test
/// platform-agnostic -- it runs identically on macOS CI and Linux CI.
#[test]
fn online_resolve_persists_platforms_in_lockfile() {
    let tmp = helpers::temp_workdir();
    let workdir = tmp.path();

    write_registry_with_component_manifest(workdir);
    write_shellcheck_manifest(workdir);

    // Online resolve as linux-aarch64 -- shellcheck supports this platform,
    // so Gate 1 passes and a lockfile is written.
    helpers::sindri_cmd_in(workdir)
        .env("SINDRI_TEST_PLATFORM_OVERRIDE", "linux-aarch64")
        .args(["resolve"])
        .assert()
        .success();

    let lockfile_path = workdir.join("sindri.lock");
    assert!(lockfile_path.exists(), "resolve must produce sindri.lock");

    let lock_text = std::fs::read_to_string(&lockfile_path).expect("read sindri.lock");
    assert!(
        lock_text.contains("platforms"),
        "lockfile must contain platforms field from component manifest; got:\n{}",
        lock_text
    );
    assert!(
        lock_text.contains("linux"),
        "lockfile platforms must include linux entries; got:\n{}",
        lock_text
    );
}

/// Stage 2: verify that an offline resolve on an unsupported platform (macOS
/// aarch64 against a Linux-only component) produces exit code 2 and the
/// error code ADM_PLATFORM_UNSUPPORTED.
///
/// This is the primary regression test for the Wave 6A offline Gate 1 wiring.
#[test]
fn offline_resolve_denies_unsupported_platform() {
    let tmp = helpers::temp_workdir();
    let workdir = tmp.path();

    write_registry_with_component_manifest(workdir);
    write_shellcheck_manifest(workdir);

    // Step A: run an online resolve as linux-aarch64 to populate sindri.lock
    // with the shellcheck platforms list.
    helpers::sindri_cmd_in(workdir)
        .env("SINDRI_TEST_PLATFORM_OVERRIDE", "linux-aarch64")
        .args(["resolve"])
        .assert()
        .success();

    let lockfile_path = workdir.join("sindri.lock");
    assert!(
        lockfile_path.exists(),
        "online resolve must produce sindri.lock before offline test"
    );

    // Step B: re-resolve offline as macos-aarch64 -- Gate 1 must deny.
    helpers::sindri_cmd_in(workdir)
        .env("SINDRI_TEST_PLATFORM_OVERRIDE", "macos-aarch64")
        .args(["resolve", "--offline"])
        .assert()
        .failure()
        .code(2)
        .stderr(contains("ADM_PLATFORM_UNSUPPORTED").or(contains("does not support")));
}

/// Stage 3: verify that an offline resolve WITHOUT a prior online resolve
/// (no lockfile) still works -- Gate 1 falls back to ADM_PLATFORM_SKIPPED
/// (non-fatal) because no platforms are cached.  This preserves backward
/// compatibility for users who do their first resolve in offline mode.
#[test]
fn offline_resolve_without_lockfile_falls_back_gracefully() {
    let tmp = helpers::temp_workdir();
    let workdir = tmp.path();

    write_registry_with_component_manifest(workdir);
    write_shellcheck_manifest(workdir);

    // No prior online resolve -- no lockfile.
    assert!(
        !workdir.join("sindri.lock").exists(),
        "precondition: no lockfile yet"
    );

    // Offline resolve should succeed even without cached platforms (Gate 1
    // records ADM_PLATFORM_SKIPPED which is non-fatal).
    helpers::sindri_cmd_in(workdir)
        .env("SINDRI_TEST_PLATFORM_OVERRIDE", "macos-aarch64")
        .args(["resolve", "--offline"])
        .assert()
        .success();
}

/// Stage 4: verify that a pre-populated lockfile (as would be written by Wave
/// 6A online resolve) correctly drives offline Gate 1 denial even when the
/// registry cache does NOT have a component.yaml.  This tests pure lockfile-
/// based platform inference, which is the core of the Wave 6A design.
#[test]
fn offline_resolve_uses_lockfile_platforms_without_component_yaml() {
    let tmp = helpers::temp_workdir();
    let workdir = tmp.path();

    // Write registry index only (no component.yaml in cache).
    let registry_fixture = helpers::fixture_path("registries/prototype");
    helpers::write_local_registry(workdir, "sindri_core", &registry_fixture);
    write_shellcheck_manifest(workdir);

    // Hand-craft a sindri.lock that already has `platforms` populated.
    // This simulates what a Wave-6A-enabled online resolve would have written.
    let lockfile_json = r#"{
  "version": 1,
  "bom_hash": "abc123",
  "target": "local",
  "components": [
    {
      "id": { "backend": "binary", "name": "shellcheck" },
      "version": "0.10.0",
      "backend": "binary",
      "oci_digest": "ghcr.io/sindri-dev/registry-core/shellcheck:0.10.0",
      "checksums": {},
      "depends_on": [],
      "platforms": [
        { "os": "linux", "arch": "x86_64" },
        { "os": "linux", "arch": "aarch64" }
      ]
    }
  ]
}"#;
    std::fs::write(workdir.join("sindri.lock"), lockfile_json)
        .expect("write pre-canned sindri.lock");

    // Offline resolve as macos-aarch64 -- must be denied via lockfile platforms.
    helpers::sindri_cmd_in(workdir)
        .env("SINDRI_TEST_PLATFORM_OVERRIDE", "macos-aarch64")
        .args(["resolve", "--offline"])
        .assert()
        .failure()
        .code(2)
        .stderr(contains("ADM_PLATFORM_UNSUPPORTED").or(contains("does not support")));
}
