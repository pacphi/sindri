//! Gate 4 — capability-trust admission, integration coverage (F-REG-06).
//!
//! Phase 2 of the 2026-04-30 reconciliation plan centralised the
//! collision-prefix rule into `sindri-policy::check_collision_prefix` so
//! that the publish-time `registry lint` and the resolve-time admission
//! gate can never disagree.
//!
//! These tests pin the contract end-to-end:
//!
//! 1. A manifest with a violating prefix that somehow reaches the resolver
//!    (e.g. a registry maintainer skipped lint, a manifest was tampered
//!    with after publish, or a fork ships a bad fixture) MUST be rejected
//!    at admission with `ADM_CAPABILITY_TRUST_VIOLATION`.
//! 2. The `:shared` escape hatch is rejected for non-core registries even
//!    if the prefix string itself is otherwise harmless.
//! 3. A well-formed prefix is admitted.
//!
//! The unit tests under `sindri-policy::capability_trust` and
//! `sindri-resolver::admission::tests::gate4_*` cover the rule logic at
//! the unit level. This file covers the wiring at the integration level
//! so a future refactor that disconnects admission from the rule (or
//! re-implements it inline) fails CI loudly.

use sindri_core::component::{
    CollisionHandlingConfig, ComponentCapabilities, ComponentManifest, ComponentMetadata,
    InstallConfig, Options,
};
use sindri_core::platform::{Arch, Capabilities, Os, Platform, TargetProfile};
use sindri_core::policy::{InstallPolicy, PolicyPreset};
use sindri_core::registry::{ComponentEntry, ComponentKind, CORE_REGISTRY_NAME};
use sindri_resolver::admission::{AdmissionChecker, CandidateRef};
use sindri_resolver::ResolverError;
use std::collections::HashMap;

fn entry(name: &str) -> ComponentEntry {
    ComponentEntry {
        name: name.into(),
        backend: "binary".into(),
        latest: "1.0.0".into(),
        versions: vec!["1.0.0".into()],
        description: format!("integration test fixture for {name}"),
        kind: ComponentKind::Component,
        oci_ref: format!("ghcr.io/test/{name}:1.0.0"),
        license: "MIT".into(),
        depends_on: vec![],
    }
}

fn manifest(name: &str, prefix: Option<&str>) -> ComponentManifest {
    let capabilities = ComponentCapabilities {
        collision_handling: prefix.map(|p| CollisionHandlingConfig {
            path_prefix: p.into(),
        }),
        ..Default::default()
    };
    ComponentManifest {
        metadata: ComponentMetadata {
            name: name.into(),
            version: "1.0.0".into(),
            description: "fixture".into(),
            license: "MIT".into(),
            tags: vec![],
            homepage: None,
        },
        platforms: vec![Platform {
            os: Os::Linux,
            arch: Arch::X86_64,
        }],
        install: InstallConfig::default(),
        depends_on: vec![],
        capabilities,
        options: Options::default(),
        validate: None,
        configure: None,
        remove: None,
        overrides: HashMap::new(),
        auth: Default::default(),
    }
}

fn target() -> TargetProfile {
    TargetProfile {
        platform: Platform {
            os: Os::Linux,
            arch: Arch::X86_64,
        },
        capabilities: Capabilities::default(),
    }
}

fn permissive_policy() -> InstallPolicy {
    InstallPolicy {
        preset: PolicyPreset::Default,
        ..Default::default()
    }
}

/// A manifest whose prefix points at `etc/passwd` is rejected at admission
/// even when the registry that supplied it would have linted clean (or no
/// lint was ever run). This is the F-REG-06 "tampered after lint" guard.
#[test]
fn gate4_rejects_mismatched_prefix_at_admission() {
    let e = entry("nodejs");
    let m = manifest("nodejs", Some("etc/passwd"));
    let policy = permissive_policy();
    let target = target();

    let checker = AdmissionChecker::new(&policy, &target);
    let cand = CandidateRef::with_manifest(&e, &m, "any-registry");

    let result = checker.admit_all(&[cand]);

    match result {
        Err(ResolverError::AdmissionDenied { code, message }) => {
            assert_eq!(code, "ADM_CAPABILITY_TRUST_VIOLATION");
            assert!(
                message.contains("first segment") || message.contains("nodejs"),
                "denial message should explain the violation, got: {message}"
            );
        }
        Err(other) => panic!("expected AdmissionDenied, got: {other:?}"),
        Ok(()) => panic!("violating prefix must not pass Gate 4"),
    }
}

/// A non-core registry that declares the `:shared` escape hatch is rejected
/// at admission. The prefix string is otherwise valid; the `:shared`
/// reservation is per-registry policy.
#[test]
fn gate4_rejects_shared_prefix_from_third_party_registry() {
    let e = entry("custom");
    let m = manifest("custom", Some(":shared"));
    let policy = permissive_policy();
    let target = target();

    let checker = AdmissionChecker::new(&policy, &target);
    let cand = CandidateRef::with_manifest(&e, &m, "acme/internal");

    let result = checker.admit_all(&[cand]);

    match result {
        Err(ResolverError::AdmissionDenied { code, message }) => {
            assert_eq!(code, "ADM_CAPABILITY_TRUST_VIOLATION");
            assert!(
                message.contains("`:shared`") || message.contains("reserved"),
                "denial message should mention the :shared reservation, got: {message}"
            );
        }
        Err(other) => panic!("expected AdmissionDenied, got: {other:?}"),
        Ok(()) => panic!("`:shared` from non-core must not pass Gate 4"),
    }
}

/// A well-formed manifest from any registry passes Gate 4.
#[test]
fn gate4_admits_well_formed_prefix() {
    let e = entry("nodejs");
    let m = manifest("nodejs", Some("nodejs/bin"));
    let policy = permissive_policy();
    let target = target();

    let checker = AdmissionChecker::new(&policy, &target);
    let cand = CandidateRef::with_manifest(&e, &m, "any-registry");

    let result = checker.admit_all(&[cand]);
    assert!(
        result.is_ok(),
        "well-formed prefix must pass admission: {result:?}"
    );
}

/// `:shared` from `sindri/core` is the documented escape hatch — admit.
#[test]
fn gate4_admits_shared_prefix_from_core_registry() {
    let e = entry("nodejs");
    let m = manifest("nodejs", Some(":shared"));
    let policy = permissive_policy();
    let target = target();

    let checker = AdmissionChecker::new(&policy, &target);
    let cand = CandidateRef::with_manifest(&e, &m, CORE_REGISTRY_NAME);

    let result = checker.admit_all(&[cand]);
    assert!(
        result.is_ok(),
        "`:shared` from sindri/core must pass: {result:?}"
    );
}
