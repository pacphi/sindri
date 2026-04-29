//! Phase 1.3 acceptance test for the source-modes refactor (DDD-08, ADR-028).
//!
//! Resolves a single component while passing a `LocalPathSource` to
//! [`sindri_resolver::resolve_with_sources`] and asserts the lockfile
//! records a `SourceDescriptor::LocalPath { path }`. This is the new
//! acceptance test called out in the plan §1.3.

use sindri_core::platform::{Arch, Os, Platform};
use sindri_core::policy::{InstallPolicy, PolicyPreset};
use sindri_core::registry::{ComponentEntry, ComponentKind};
use sindri_core::source_descriptor::SourceDescriptor;
use sindri_registry::source::{LocalPathSource, RegistrySource};
use sindri_resolver::{resolve_with_sources, ResolveOptions};
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;

fn permissive_policy() -> InstallPolicy {
    InstallPolicy {
        preset: PolicyPreset::Default,
        allowed_licenses: Vec::new(),
        denied_licenses: Vec::new(),
        on_unknown_license: None,
        require_signed_registries: None,
        require_checksums: None,
        offline: Some(true),
        audit: None,
        auth: Default::default(),
    }
}

fn linux_platform() -> Platform {
    Platform {
        os: Os::Linux,
        arch: Arch::X86_64,
    }
}

fn write_bom(dir: &std::path::Path) -> PathBuf {
    let bom = r#"
name: phase1-fixture
components:
  - address: "mise:nodejs"
"#;
    let path = dir.join("sindri.yaml");
    std::fs::write(&path, bom).unwrap();
    path
}

fn registry_with_nodejs() -> HashMap<String, ComponentEntry> {
    let entry = ComponentEntry {
        name: "nodejs".into(),
        backend: "".into(),
        latest: "22.0.0".into(),
        versions: vec!["22.0.0".into()],
        description: "node".into(),
        kind: ComponentKind::Component,
        oci_ref: "ghcr.io/sindri-dev/registry-core/nodejs:22.0.0".into(),
        license: "MIT".into(),
        depends_on: vec![],
    };
    let mut map = HashMap::new();
    map.insert("mise:nodejs".to_string(), entry);
    map
}

#[test]
fn local_path_source_lockfile_records_local_path_descriptor() {
    let tmp = TempDir::new().unwrap();
    let manifest = write_bom(tmp.path());
    let lockfile_path = tmp.path().join("sindri.lock");
    let local_root = tmp.path().join("local-overrides");
    std::fs::create_dir_all(&local_root).unwrap();

    let sources = vec![RegistrySource::LocalPath(LocalPathSource::new(
        local_root.clone(),
    ))];

    let opts = ResolveOptions {
        manifest_path: manifest,
        lockfile_path: lockfile_path.clone(),
        target_name: "local".into(),
        // Offline keeps the test free of cache lookups; the source
        // descriptor pick is independent of fetch path.
        offline: true,
        strict: false,
        explain: None,
        registry_manifest_digest: None,
        target_kind: Some("local".into()),
        component_digests: HashMap::new(),
        registry_cache_root: None,
        strict_oci: false,
    };

    let lock = resolve_with_sources(
        &opts,
        &registry_with_nodejs(),
        &sources,
        &permissive_policy(),
        &linux_platform(),
    )
    .expect("resolve should succeed");

    assert_eq!(lock.components.len(), 1);
    let nodejs = &lock.components[0];
    let source = nodejs
        .source
        .as_ref()
        .expect("source descriptor should be populated by Phase 1.3");
    match source {
        SourceDescriptor::LocalPath { path } => {
            assert_eq!(path, &local_root);
        }
        other => panic!("expected SourceDescriptor::LocalPath, got {other:?}"),
    }
}

#[test]
fn empty_sources_falls_back_to_oci_descriptor_from_legacy_ref() {
    // When the caller passes no sources (the back-compat path used by
    // existing CLI call sites), `resolve` must still populate a
    // `SourceDescriptor::Oci { ... }` reconstructed from the entry's
    // legacy `oci_ref` so that downstream apply code can rely on the field.
    let tmp = TempDir::new().unwrap();
    let manifest = write_bom(tmp.path());
    let lockfile_path = tmp.path().join("sindri.lock");

    let opts = ResolveOptions {
        manifest_path: manifest,
        lockfile_path,
        target_name: "local".into(),
        offline: true,
        strict: false,
        explain: None,
        registry_manifest_digest: None,
        target_kind: Some("local".into()),
        component_digests: HashMap::new(),
        registry_cache_root: None,
        strict_oci: false,
    };

    let lock = sindri_resolver::resolve(
        &opts,
        &registry_with_nodejs(),
        &permissive_policy(),
        &linux_platform(),
    )
    .expect("legacy resolve path should succeed");

    let source = lock.components[0]
        .source
        .as_ref()
        .expect("descriptor backfilled from legacy oci_ref");
    match source {
        SourceDescriptor::Oci { url, tag, .. } => {
            assert!(url.starts_with("oci://"), "url: {url}");
            assert_eq!(tag, "22.0.0");
        }
        other => panic!("expected Oci descriptor, got {other:?}"),
    }
}
