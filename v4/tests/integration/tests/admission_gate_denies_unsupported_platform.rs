//! Scenario: a Linux-only component should be denied (ADM_PLATFORM_UNSUPPORTED)
//! when the active platform is `macos-aarch64`.
//!
//! # FIXME(wave-4a-followup)
//!
//! This test is `#[ignore]`-d for the initial harness landing.
//!
//! ADR-008 Gate 1 (platform admission) only **denies** when the candidate
//! component's `ComponentManifest` is available — without one the gate
//! short-circuits to `ADM_PLATFORM_SKIPPED`. The current resolver pipeline
//! (Wave 2A) walks the registry **index** alone; per-component manifest
//! fetch arrives with OCI live-fetch in Wave 3A.2.
//!
//! Once the resolver fetches manifests for the closure, dropping the
//! `#[ignore]` should be enough — the `SINDRI_TEST_PLATFORM_OVERRIDE`
//! hook in `sindri-core::platform` already makes the override drive
//! `Platform::current()`, and the `shellcheck` fixture is intentionally
//! Linux-only.

#[path = "helpers.rs"]
mod helpers;

use predicates::str::contains;

#[test]
#[ignore = "FIXME(wave-4a-followup): admission Gate 1 needs per-component manifest fetch (Wave 3A.2)"]
fn admission_gate_denies_unsupported_platform() {
    let tmp = helpers::temp_workdir();
    let workdir = tmp.path();

    let registry_fixture = helpers::fixture_path("registries/prototype");
    helpers::write_local_registry(workdir, "core", &registry_fixture);

    // Manifest pinning a Linux-only fixture.
    std::fs::write(
        workdir.join("sindri.yaml"),
        "name: admission-fixture\ncomponents:\n  - address: \"binary:shellcheck\"\n",
    )
    .expect("write manifest");

    let assert = helpers::sindri_cmd()
        .current_dir(workdir)
        .env("HOME", workdir)
        .env("SINDRI_TEST_PLATFORM_OVERRIDE", "macos-aarch64")
        .args(["resolve", "--offline"])
        .assert();

    assert
        .failure()
        .code(2)
        .stderr(contains("ADM_PLATFORM_UNSUPPORTED"));
}
