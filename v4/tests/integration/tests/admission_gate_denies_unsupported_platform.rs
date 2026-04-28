//! Scenario: a component that declares `platforms: [linux/x86_64, linux/aarch64]`
//! must be denied with `ADM_PLATFORM_UNSUPPORTED` when the active platform is
//! `macos-aarch64` (or any other non-Linux platform).
//!
//! # D12 — Gate 1 re-enablement (Wave 5G)
//!
//! The previous `#[ignore]` comment read:
//!   "FIXME(wave-4a-followup): admission Gate 1 needs per-component manifest
//!    fetch (Wave 3A.2)"
//!
//! The unblocking work (Wave 5F / PR #231) wired the per-component manifest
//! fetch path.  This test exercises the `sindri-resolver::admission` library
//! directly — an in-process integration that gives us full Gate 1 coverage
//! without depending on the CLI's offline resolver having a manifest-fetch
//! path end-to-end wired (that last mile ships with Wave 6A).
//!
//! The component fixture is intentionally implausible on any real CI host:
//! `platforms: [linux/x86_64, linux/aarch64]` while the forced target platform
//! is `macos-aarch64`.
//!
//! ADR-008: Gate 1 (platform eligibility).
//! ADR-003: OCI-only distribution (component manifests live in the registry).

use sindri_core::component::{
    ComponentCapabilities, ComponentManifest, ComponentMetadata, InstallConfig, Options,
};
use sindri_core::platform::{Arch, Capabilities, Os, Platform, TargetProfile};
use sindri_core::policy::{InstallPolicy, PolicyPreset};
use sindri_core::registry::{ComponentEntry, ComponentKind};
use sindri_resolver::admission::{AdmissionChecker, CandidateRef};
use sindri_resolver::ResolverError;
use std::collections::HashMap;

/// Construct a minimal [`ComponentEntry`] for use in admission tests.
fn shellcheck_entry() -> ComponentEntry {
    ComponentEntry {
        name: "shellcheck".into(),
        backend: "binary".into(),
        latest: "0.10.0".into(),
        versions: vec!["0.10.0".into()],
        description: "Shell script static analyser".into(),
        kind: ComponentKind::Component,
        oci_ref: "ghcr.io/sindri-dev/registry-core/shellcheck:0.10.0".into(),
        license: "GPL-3.0".into(),
        depends_on: vec![],
    }
}

/// Construct a [`ComponentManifest`] that restricts installation to Linux
/// (x86_64 and aarch64 variants).  This fixture is the in-memory equivalent
/// of `fixtures/registries/prototype/components/shellcheck/component.yaml`.
fn linux_only_manifest() -> ComponentManifest {
    ComponentManifest {
        metadata: ComponentMetadata {
            name: "shellcheck".into(),
            version: "0.10.0".into(),
            description: "Shell script static analyser".into(),
            license: "GPL-3.0".into(),
            tags: vec!["linter".into(), "shell".into()],
            homepage: Some("https://www.shellcheck.net".into()),
        },
        // Linux-only: Gate 1 must deny any non-Linux platform.
        platforms: vec![
            Platform {
                os: Os::Linux,
                arch: Arch::X86_64,
            },
            Platform {
                os: Os::Linux,
                arch: Arch::Aarch64,
            },
        ],
        install: InstallConfig::default(),
        depends_on: vec![],
        capabilities: ComponentCapabilities::default(),
        options: Options::default(),
        validate: None,
        configure: None,
        remove: None,
        overrides: HashMap::new(),
    }
}

/// Gate 1 must deny a Linux-only component when the host platform is macOS
/// (aarch64).  The error code must be `ADM_PLATFORM_UNSUPPORTED`.
#[test]
fn admission_gate_denies_unsupported_platform() {
    let entry = shellcheck_entry();
    let manifest = linux_only_manifest();

    // Simulate a macOS/aarch64 host — the same override the CLI test used via
    // `SINDRI_TEST_PLATFORM_OVERRIDE=macos-aarch64`.
    let target = TargetProfile {
        platform: Platform {
            os: Os::Macos,
            arch: Arch::Aarch64,
        },
        capabilities: Capabilities::default(),
    };

    let policy = InstallPolicy {
        preset: PolicyPreset::Default,
        allowed_licenses: vec![],
        denied_licenses: vec![],
        on_unknown_license: None,
        require_signed_registries: None,
        require_checksums: None,
        offline: Some(true),
        audit: None,
    };

    let checker = AdmissionChecker::new(&policy, &target);
    // Wire Gate 1 by supplying the manifest — exactly the path that the CLI
    // resolver will take once Wave 6A lands the per-component manifest fetch.
    let candidate = CandidateRef::with_manifest(&entry, &manifest, "sindri/core");

    let result = checker.admit_all(&[candidate]);

    match result {
        Err(ResolverError::AdmissionDenied { code, message }) => {
            assert_eq!(
                code, "ADM_PLATFORM_UNSUPPORTED",
                "expected ADM_PLATFORM_UNSUPPORTED, got `{code}`"
            );
            // The denial message must mention the rejected platform so operators
            // can act on it.
            assert!(
                message.contains("macos") || message.contains("aarch64"),
                "denial message should mention the unsupported platform, got: `{message}`"
            );
        }
        Err(other) => panic!("expected AdmissionDenied, got: {other:?}"),
        Ok(()) => {
            panic!("admission should have been denied for a Linux-only component on macos-aarch64")
        }
    }
}
