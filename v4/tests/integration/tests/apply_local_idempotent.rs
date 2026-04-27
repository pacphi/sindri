//! Scenario: `apply --dry-run --yes` is idempotent.
//!
//! The dry-run path exercises the entire apply lifecycle — lockfile load,
//! collision validation, plan rendering — without invoking any real
//! backend (`mise`, `binary`, …) which would not exist on a CI runner.

#[path = "helpers.rs"]
mod helpers;

#[test]
#[cfg_attr(windows, ignore)] // FIXME(wave-4a-followup): tempdir + path quoting on Windows runners
fn apply_dry_run_is_idempotent() {
    let tmp = helpers::temp_workdir();
    let workdir = tmp.path();

    let registry_fixture = helpers::fixture_path("registries/prototype");
    helpers::write_local_registry(workdir, "core", &registry_fixture);

    helpers::sindri_cmd()
        .current_dir(workdir)
        .env("HOME", workdir)
        .args([
            "init",
            "--non-interactive",
            "--template",
            "minimal",
            "--name",
            "apply-fixture",
            "--policy",
            "default",
        ])
        .assert()
        .success();

    helpers::sindri_cmd()
        .current_dir(workdir)
        .env("HOME", workdir)
        .args(["resolve", "--offline"])
        .assert()
        .success();

    // First dry-run.
    let first = helpers::sindri_cmd()
        .current_dir(workdir)
        .env("HOME", workdir)
        .args(["apply", "--dry-run", "--yes", "--target", "local"])
        .assert()
        .success();
    let first_out = String::from_utf8_lossy(&first.get_output().stdout).to_string();

    // Second dry-run — should produce the same plan headline.
    let second = helpers::sindri_cmd()
        .current_dir(workdir)
        .env("HOME", workdir)
        .args(["apply", "--dry-run", "--yes", "--target", "local"])
        .assert()
        .success();
    let second_out = String::from_utf8_lossy(&second.get_output().stdout).to_string();

    // Plan headers ("Plan: N component(s) to apply on local:") must match
    // verbatim across runs — the strongest no-op signal we can assert
    // without depending on backend-side state.
    let extract_plan_header = |s: &str| -> String {
        s.lines()
            .find(|line| line.starts_with("Plan: "))
            .unwrap_or_default()
            .to_string()
    };
    assert_eq!(
        extract_plan_header(&first_out),
        extract_plan_header(&second_out),
        "apply --dry-run plan header diverged between runs:\nfirst:\n{}\nsecond:\n{}",
        first_out,
        second_out,
    );
    assert!(
        first_out.contains("Dry run") && second_out.contains("Dry run"),
        "dry-run banner must appear in both runs"
    );
}
