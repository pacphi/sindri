//! Phase-2 acceptance tests for the `--strict-oci` admission gate
//! (DDD-08, ADR-028). Drives [`sindri_resolver::resolve_with_sources`]
//! end-to-end via small fixture manifests.
//!
//! Plan §2 acceptance:
//!   1. `--strict-oci` rejects a lockfile containing a `LocalPath` source.
//!   2. `--strict-oci` accepts a lockfile that contains only verified
//!      `Oci` and `LocalOci` sources.

use sindri_core::component::BomEntry;
use sindri_core::manifest::BomManifest;
use sindri_core::platform::{Arch, Os, Platform};
use sindri_core::policy::{InstallPolicy, PolicyPreset};
use sindri_core::registry::{ComponentEntry, ComponentKind, CORE_REGISTRY_NAME};
use sindri_registry::source::{ComponentName, LocalPathSource, OciSourceConfig, RegistrySource};
use sindri_resolver::{ResolveOptions, ResolverError};
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

fn permissive_policy() -> InstallPolicy {
    InstallPolicy {
        preset: PolicyPreset::Default,
        allowed_licenses: vec![],
        denied_licenses: vec![],
        on_unknown_license: None,
        require_signed_registries: None,
        require_checksums: None,
        offline: Some(true),
        audit: None,
        auth: sindri_core::policy::AuthPolicy::default(),
    }
}

fn entry(name: &str) -> ComponentEntry {
    ComponentEntry {
        name: name.into(),
        backend: "mise".into(),
        latest: "1.0.0".into(),
        versions: vec!["1.0.0".into()],
        description: "test".into(),
        kind: ComponentKind::Component,
        oci_ref: format!(
            "ghcr.io/sindri-dev/registry-core/{}@sha256:{}",
            name,
            "a".repeat(64)
        ),
        license: "MIT".into(),
        depends_on: vec![],
    }
}

fn write_bom(tmp: &TempDir, components: &[&str]) -> std::path::PathBuf {
    let mut bom = BomManifest {
        schema: None,
        name: Some("strict-oci-test".into()),
        components: vec![],
        registry: sindri_core::manifest::RegistrySection::default(),
        targets: HashMap::new(),
        preferences: None,
        r#override: None,
        secrets: HashMap::new(),
    };
    for c in components {
        bom.components.push(BomEntry {
            address: format!("mise:{}", c),
            version: None,
            options: Default::default(),
        });
    }
    let yaml = serde_yaml::to_string(&bom).unwrap();
    let p = tmp.path().join("sindri.yaml");
    fs::write(&p, yaml).unwrap();
    p
}

fn registry_with(names: &[&str]) -> HashMap<String, ComponentEntry> {
    let mut m = HashMap::new();
    for n in names {
        m.insert(format!("mise:{}", n), entry(n));
    }
    m
}

fn options_for(tmp: &TempDir, manifest: std::path::PathBuf, strict_oci: bool) -> ResolveOptions {
    ResolveOptions {
        manifest_path: manifest,
        lockfile_path: tmp.path().join("sindri.lock"),
        target_name: "local".into(),
        offline: true,
        strict: false,
        explain: None,
        registry_manifest_digest: None,
        target_kind: Some("local".into()),
        component_digests: HashMap::new(),
        registry_cache_root: None,
        strict_oci,
    }
}

#[test]
fn strict_oci_rejects_lockfile_with_local_path_source() {
    let tmp = TempDir::new().unwrap();
    let manifest = write_bom(&tmp, &["nodejs"]);
    let registry = registry_with(&["nodejs"]);

    // A LocalPath source with a scope match for `nodejs` — the resolver
    // will pick this descriptor and the strict gate must reject it.
    let local_path = tmp.path().join("local-reg");
    fs::create_dir_all(&local_path).unwrap();
    let sources = vec![RegistrySource::LocalPath(
        LocalPathSource::new(&local_path).with_scope(vec![ComponentName::from("nodejs")]),
    )];

    let policy = permissive_policy();
    let platform = Platform {
        os: Os::Linux,
        arch: Arch::X86_64,
    };
    let opts = options_for(&tmp, manifest, true);

    let err = sindri_resolver::resolve_with_sources(&opts, &registry, &sources, &policy, &platform)
        .expect_err("strict-oci must reject a LocalPath descriptor");
    match err {
        ResolverError::SourceNotProductionGrade { offenders } => {
            assert_eq!(offenders.len(), 1);
            assert_eq!(offenders[0].0, "mise:nodejs");
            assert_eq!(offenders[0].1, "local-path");
        }
        other => panic!("expected SourceNotProductionGrade, got {:?}", other),
    }
}

#[test]
fn strict_oci_accepts_lockfile_with_only_oci_sources() {
    let tmp = TempDir::new().unwrap();
    let manifest = write_bom(&tmp, &["nodejs"]);
    let registry = registry_with(&["nodejs"]);

    // A canonical OCI source — the resolver records `SourceDescriptor::Oci`
    // for `nodejs`, which the strict gate accepts.
    let sources = vec![RegistrySource::Oci(OciSourceConfig {
        url: "oci://ghcr.io/sindri-dev/registry-core".into(),
        tag: "1.0.0".into(),
        scope: None,
        registry_name: CORE_REGISTRY_NAME.into(),
    })];

    let policy = permissive_policy();
    let platform = Platform {
        os: Os::Linux,
        arch: Arch::X86_64,
    };
    let opts = options_for(&tmp, manifest, true);

    let lf = sindri_resolver::resolve_with_sources(&opts, &registry, &sources, &policy, &platform)
        .expect("strict-oci must accept Oci-only lockfile");
    assert_eq!(lf.components.len(), 1);
    assert_eq!(
        lf.components[0]
            .source
            .as_ref()
            .expect("source must be set")
            .kind(),
        "oci"
    );
}

#[test]
fn non_strict_with_local_path_passes_with_warning() {
    let tmp = TempDir::new().unwrap();
    let manifest = write_bom(&tmp, &["nodejs"]);
    let registry = registry_with(&["nodejs"]);

    let local_path = tmp.path().join("local-reg");
    fs::create_dir_all(&local_path).unwrap();
    let sources = vec![RegistrySource::LocalPath(LocalPathSource::new(&local_path))];

    let policy = permissive_policy();
    let platform = Platform {
        os: Os::Linux,
        arch: Arch::X86_64,
    };
    let opts = options_for(&tmp, manifest, false);

    // Default mode: the resolver succeeds and emits a tracing::warn!
    // listing the offending source mix. The test doesn't capture logs;
    // we just assert the resolve does not error.
    sindri_resolver::resolve_with_sources(&opts, &registry, &sources, &policy, &platform)
        .expect("non-strict mode must accept LocalPath");
}
