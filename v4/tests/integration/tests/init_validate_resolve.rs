//! Scenario: `init` → `validate` → `resolve --offline` round-trip.

#[path = "helpers.rs"]
mod helpers;

use predicates::str::contains;

#[test]
fn init_validate_resolve_round_trip() {
    let tmp = helpers::temp_workdir();
    let workdir = tmp.path();

    // Pre-populate the registry cache rooted at $HOME so `sindri resolve`
    // sees the prototype components without a network round-trip.
    let registry_fixture = helpers::fixture_path("registries/prototype");
    helpers::write_local_registry(workdir, "core", &registry_fixture);

    // 1. `sindri init`
    helpers::sindri_cmd()
        .current_dir(workdir)
        .env("HOME", workdir)
        .args([
            "init",
            "--non-interactive",
            "--template",
            "minimal",
            "--name",
            "test-project",
            "--policy",
            "default",
        ])
        .assert()
        .success();

    let manifest = workdir.join("sindri.yaml");
    assert!(manifest.exists(), "init must create sindri.yaml");
    let manifest_text = std::fs::read_to_string(&manifest).expect("read sindri.yaml");
    assert!(
        manifest_text.contains("name: test-project"),
        "manifest should record the project name; got:\n{}",
        manifest_text
    );

    // 2. `sindri validate sindri.yaml`
    helpers::sindri_cmd()
        .current_dir(workdir)
        .env("HOME", workdir)
        .args(["validate", "sindri.yaml"])
        .assert()
        .success()
        .stdout(contains("is valid"));

    // 3. The minimal template only declares `mise:nodejs`. Rewrite the
    //    manifest in place to also include `binary:gh` so the resolve
    //    assertion can prove the closure walks more than one component.
    std::fs::write(
        &manifest,
        "name: test-project\ncomponents:\n  - address: \"mise:nodejs\"\n  - address: \"binary:gh\"\n",
    )
    .expect("rewrite manifest with extra component");

    // 4. `sindri resolve --offline`
    helpers::sindri_cmd()
        .current_dir(workdir)
        .env("HOME", workdir)
        .args(["resolve", "--offline"])
        .assert()
        .success();

    let lockfile = workdir.join("sindri.lock");
    assert!(lockfile.exists(), "resolve must produce sindri.lock");
    let lock_text = std::fs::read_to_string(&lockfile).expect("read sindri.lock");
    assert!(
        lock_text.contains("mise:nodejs") || lock_text.contains("\"name\": \"nodejs\""),
        "lockfile must record nodejs; got:\n{}",
        lock_text
    );
    assert!(
        lock_text.contains("binary:gh") || lock_text.contains("\"name\": \"gh\""),
        "lockfile must record gh; got:\n{}",
        lock_text
    );
}
