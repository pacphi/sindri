//! Wave 5F — D18: per-target lockfile integration test.
//!
//! Drives [`sindri_resolver::resolve`] end-to-end with two `ResolveOptions`
//! variants — one defaulted (`target_name = "local"`) and one with a
//! container-style target_kind — and asserts:
//!
//! * the default mode picks brew/mise (platform default chain on macOS, or
//!   mise/apt on Linux) just as it always has;
//! * the `k8s` mode picks `mise` (the first container-friendly backend) and
//!   never `brew`, even on macOS;
//! * round-trip: the lockfile written to disk parses back to a `Lockfile`
//!   that round-trips the chosen backends.

use sindri_core::component::Backend;
use sindri_core::platform::{Arch, Os, Platform};
use sindri_core::policy::{InstallPolicy, PolicyPreset};
use sindri_core::registry::{ComponentEntry, ComponentKind, RegistryIndex};
use sindri_resolver::{lockfile_writer, resolve, ResolveOptions};
use std::collections::HashMap;
use tempfile::TempDir;

fn macos_platform() -> Platform {
    Platform {
        os: Os::Macos,
        arch: Arch::Aarch64,
    }
}

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
    }
}

fn registry() -> HashMap<String, ComponentEntry> {
    let entries = vec![
        ComponentEntry {
            name: "nodejs".into(),
            backend: "".into(), // empty -> follow preference chain
            latest: "22.0.0".into(),
            versions: vec!["22.0.0".into()],
            description: "node".into(),
            kind: ComponentKind::Component,
            oci_ref: "ghcr.io/sindri-dev/registry-core/nodejs:22.0.0".into(),
            license: "MIT".into(),
            depends_on: vec![],
        },
        ComponentEntry {
            name: "kubectl".into(),
            backend: "".into(),
            latest: "1.35.4".into(),
            versions: vec!["1.35.4".into()],
            description: "kubectl".into(),
            kind: ComponentKind::Component,
            oci_ref: "ghcr.io/sindri-dev/registry-core/kubectl:1.35.4".into(),
            license: "Apache-2.0".into(),
            depends_on: vec![],
        },
    ];
    let _index_for_serialisation_check = RegistryIndex {
        version: 1,
        registry: "test/core".into(),
        components: entries.clone(),
    };
    let mut map = HashMap::new();
    for e in entries {
        map.insert(format!("{}:{}", "mise", e.name), e.clone());
        map.insert(format!("{}:{}", "binary", e.name), e);
    }
    map
}

fn write_bom(dir: &std::path::Path) -> std::path::PathBuf {
    let bom = r#"
name: integration-fixture
components:
  - address: "mise:nodejs"
  - address: "mise:kubectl"
"#;
    let path = dir.join("sindri.yaml");
    std::fs::write(&path, bom).unwrap();
    path
}

#[test]
fn default_mode_writes_sindri_lock_with_platform_default_backends() {
    let tmp = TempDir::new().unwrap();
    let manifest = write_bom(tmp.path());
    let lockfile_path = tmp.path().join("sindri.lock");

    let opts = ResolveOptions {
        manifest_path: manifest.clone(),
        lockfile_path: lockfile_path.clone(),
        target_name: "local".into(),
        offline: true,
        strict: false,
        explain: None,
        registry_manifest_digest: None,
        target_kind: Some("local".into()),
        component_digests: HashMap::new(),
    };

    let lock = resolve(&opts, &registry(), &permissive_policy(), &macos_platform())
        .expect("resolve should succeed");
    assert_eq!(lock.target, "local");
    assert!(lockfile_path.exists());

    // macOS default chain heads with brew.
    let nodejs = lock
        .components
        .iter()
        .find(|c| c.id.name == "nodejs")
        .unwrap();
    assert_eq!(nodejs.backend, Backend::Brew);

    // Round-trip via the writer's reader.
    let reread = lockfile_writer::read_lockfile(&lockfile_path).unwrap();
    assert_eq!(reread.components.len(), lock.components.len());
}

#[test]
fn target_mode_writes_sindri_target_lock_with_container_friendly_backends() {
    let tmp = TempDir::new().unwrap();
    let manifest = write_bom(tmp.path());
    // Wave 5F — D18: per-target lockfile naming pattern is
    // `sindri.<target>.lock`.
    let lockfile_path = tmp.path().join("sindri.k8s.lock");

    let opts = ResolveOptions {
        manifest_path: manifest.clone(),
        lockfile_path: lockfile_path.clone(),
        target_name: "k8s".into(),
        offline: true,
        strict: false,
        explain: None,
        registry_manifest_digest: None,
        target_kind: Some("k8s".into()),
        component_digests: HashMap::new(),
    };

    let lock = resolve(&opts, &registry(), &permissive_policy(), &macos_platform())
        .expect("resolve should succeed under k8s target");
    assert_eq!(lock.target, "k8s");
    assert!(lockfile_path.exists());

    let nodejs = lock
        .components
        .iter()
        .find(|c| c.id.name == "nodejs")
        .unwrap();
    // The k8s chain MUST NOT pick brew (host-only manager).
    assert_ne!(nodejs.backend, Backend::Brew);
    assert_eq!(nodejs.backend, Backend::Mise);

    // Round-trip
    let reread = lockfile_writer::read_lockfile(&lockfile_path).unwrap();
    assert_eq!(reread.target, "k8s");
}

#[test]
fn default_and_target_lockfiles_coexist() {
    // Both modes can be invoked in the same project directory and produce
    // distinct files — this is the contract apply.rs depends on.
    let tmp = TempDir::new().unwrap();
    let manifest = write_bom(tmp.path());

    let local_lock = tmp.path().join("sindri.lock");
    let k8s_lock = tmp.path().join("sindri.k8s.lock");

    resolve(
        &ResolveOptions {
            manifest_path: manifest.clone(),
            lockfile_path: local_lock.clone(),
            target_name: "local".into(),
            offline: true,
            strict: false,
            explain: None,
            registry_manifest_digest: None,
            target_kind: Some("local".into()),
            component_digests: HashMap::new(),
        },
        &registry(),
        &permissive_policy(),
        &macos_platform(),
    )
    .unwrap();

    resolve(
        &ResolveOptions {
            manifest_path: manifest.clone(),
            lockfile_path: k8s_lock.clone(),
            target_name: "k8s".into(),
            offline: true,
            strict: false,
            explain: None,
            registry_manifest_digest: None,
            target_kind: Some("k8s".into()),
            component_digests: HashMap::new(),
        },
        &registry(),
        &permissive_policy(),
        &macos_platform(),
    )
    .unwrap();

    assert!(local_lock.exists());
    assert!(k8s_lock.exists());

    let l = lockfile_writer::read_lockfile(&local_lock).unwrap();
    let k = lockfile_writer::read_lockfile(&k8s_lock).unwrap();

    let l_node = l.components.iter().find(|c| c.id.name == "nodejs").unwrap();
    let k_node = k.components.iter().find(|c| c.id.name == "nodejs").unwrap();
    // Distinct backend choices per target on the same platform.
    assert_ne!(l_node.backend, k_node.backend);
}

#[test]
fn component_digests_propagate_into_lockfile() {
    let tmp = TempDir::new().unwrap();
    let manifest = write_bom(tmp.path());
    let lockfile_path = tmp.path().join("sindri.lock");

    let mut digests = HashMap::new();
    digests.insert(
        "mise:nodejs".to_string(),
        "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
    );

    let opts = ResolveOptions {
        manifest_path: manifest,
        lockfile_path: lockfile_path.clone(),
        target_name: "local".into(),
        offline: true,
        strict: false,
        explain: None,
        registry_manifest_digest: None,
        target_kind: Some("local".into()),
        component_digests: digests,
    };

    let lock = resolve(&opts, &registry(), &permissive_policy(), &macos_platform()).unwrap();
    let nodejs = lock
        .components
        .iter()
        .find(|c| c.id.name == "nodejs")
        .unwrap();
    assert!(
        nodejs.component_digest.is_some(),
        "nodejs digest threaded through"
    );

    // kubectl had no entry in the digest map — must remain None.
    let kubectl = lock
        .components
        .iter()
        .find(|c| c.id.name == "kubectl")
        .unwrap();
    assert!(kubectl.component_digest.is_none());
}
