//! Scenario: `sindri registry lint` flags a component missing
//! `metadata.license`.

#[path = "helpers.rs"]
mod helpers;

use predicates::str::contains;

#[test]
fn registry_lint_finds_missing_license() {
    let bad_component =
        helpers::fixture_path("registries/bad-no-license/components/oops/component.yaml");

    let assert = helpers::sindri_cmd()
        .args([
            "registry",
            "lint",
            bad_component.to_str().expect("utf-8 path"),
        ])
        .assert();

    assert
        .failure()
        .code(4)
        .stderr(contains("license"))
        .stderr(contains("oops"));
}
