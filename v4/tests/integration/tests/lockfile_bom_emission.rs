//! Scenario: `sindri bom --format spdx` emits a parseable SPDX 2.3 document.
//!
//! Pins behaviour for Wave 4B's SBOM rewrite: when the SBOM emitter is
//! upgraded, this test will keep proving the contract (top-level
//! `spdxVersion`, `dataLicense`, `documentNamespace`, non-empty `packages`).

#[path = "helpers.rs"]
mod helpers;

use serde_json::Value;

#[test]
fn bom_spdx_emission_is_well_formed() {
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
            "bom-fixture",
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

    let bom_path = workdir.join("bom.spdx.json");
    helpers::sindri_cmd()
        .current_dir(workdir)
        .env("HOME", workdir)
        .args([
            "bom",
            "--format",
            "spdx",
            "--output",
            bom_path.to_str().expect("utf-8 path"),
        ])
        .assert()
        .success();

    assert!(
        bom_path.exists(),
        "SBOM file must exist at {}",
        bom_path.display()
    );

    let raw = std::fs::read_to_string(&bom_path).expect("read SBOM");
    let parsed: Value = serde_json::from_str(&raw).expect("SBOM must be valid JSON");

    assert_eq!(
        parsed.get("spdxVersion").and_then(Value::as_str),
        Some("SPDX-2.3"),
        "SBOM must declare SPDX-2.3; got: {:?}",
        parsed.get("spdxVersion"),
    );
    assert!(
        parsed.get("dataLicense").is_some(),
        "SBOM must include `dataLicense`",
    );
    assert!(
        parsed.get("documentNamespace").is_some(),
        "SBOM must include `documentNamespace`",
    );
    let packages = parsed
        .get("packages")
        .and_then(Value::as_array)
        .expect("SBOM must include a `packages` array");
    assert!(!packages.is_empty(), "SBOM `packages` must be non-empty");
}
