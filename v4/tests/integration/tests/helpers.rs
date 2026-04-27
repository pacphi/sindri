//! Shared utilities for the Wave 4A integration-test harness.
//!
//! Each scenario file in `tests/` pulls this module in via
//! `#[path = "helpers.rs"] mod helpers;`. Cargo's libtest layout compiles
//! each integration-test file as its own crate, so a top-level `helpers.rs`
//! sibling (without a `mod.rs`) is the lightest-weight way to share code
//! without making cargo treat it as an extra test target.
//!
//! All helpers panic on infrastructure failures (missing fixtures, failed
//! tempdir creation, …) per the testing convention — production paths
//! avoid panics entirely, but tests should fail loudly when their
//! environment is broken.

#![allow(dead_code)] // each test file uses a different subset

use assert_cmd::Command;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Build an [`assert_cmd::Command`] invoking the workspace `sindri` binary.
///
/// Sets `RUST_LOG=warn` to keep test output focussed on assertions and
/// strips inherited `SINDRI_TEST_PLATFORM_OVERRIDE` so individual tests
/// must opt in explicitly.
pub fn sindri_cmd() -> Command {
    let mut cmd = Command::cargo_bin("sindri").expect("sindri binary built by cargo test");
    cmd.env("RUST_LOG", "warn");
    cmd.env_remove("SINDRI_TEST_PLATFORM_OVERRIDE");
    cmd
}

/// Create a fresh tempdir for a scenario.
///
/// Returning the [`TempDir`] handle (not just the path) keeps the directory
/// alive for the lifetime of the test — when the handle drops, the
/// directory is removed.
pub fn temp_workdir() -> TempDir {
    tempfile::Builder::new()
        .prefix("sindri-it-")
        .tempdir()
        .expect("create tempdir")
}

/// Resolve a fixture path relative to this crate's `Cargo.toml`.
///
/// Fixtures live under `v4/tests/integration/fixtures/`; pass a path
/// relative to that directory (e.g. `"manifests/minimal.sindri.yaml"`).
pub fn fixture_path(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures");
    let p = root.join(name);
    assert!(
        p.exists(),
        "fixture not found: {} (looked in {})",
        name,
        p.display(),
    );
    p
}

/// Populate a fake `$HOME/.sindri/cache/registries/<registry_name>/index.yaml`
/// pointing at the given local registry fixture.
///
/// `home_dir` is the directory you've set as `HOME` for this test (typically
/// the same tempdir used as the workdir). `registry_fixture` is a path to a
/// fixture directory containing `index.yaml` (e.g.
/// `fixtures/registries/prototype`). The resolver loads its registry index
/// from the cache — this helper short-circuits the `sindri registry refresh`
/// step so scenarios stay focussed on the verb under test.
pub fn write_local_registry(home_dir: &Path, registry_name: &str, registry_fixture: &Path) {
    let index_src = registry_fixture.join("index.yaml");
    assert!(
        index_src.exists(),
        "registry fixture missing index.yaml: {}",
        index_src.display(),
    );
    let cache_dir = home_dir
        .join(".sindri")
        .join("cache")
        .join("registries")
        .join(registry_name);
    std::fs::create_dir_all(&cache_dir).expect("create cache dir");
    std::fs::copy(&index_src, cache_dir.join("index.yaml")).expect("copy registry index");
}
