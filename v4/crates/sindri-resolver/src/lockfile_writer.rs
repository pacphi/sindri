use crate::error::ResolverError;
use sindri_core::component::{Backend, ComponentId};
use sindri_core::lockfile::{Lockfile, ResolvedComponent};
use sindri_core::platform::Platform;
use sindri_core::registry::ComponentEntry;
use sindri_core::source_descriptor::SourceDescriptor;
use sindri_core::version::Version;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

/// Compute bom_hash as sha256 of the sindri.yaml content
pub fn compute_bom_hash(bom_content: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bom_content.as_bytes());
    hex::encode(hasher.finalize())
}

/// Write a lockfile atomically (write to .tmp, then rename) (ADR-018)
pub fn write_lockfile(path: &Path, lockfile: &Lockfile) -> Result<(), ResolverError> {
    let json = serde_json::to_string_pretty(lockfile)
        .map_err(|e| ResolverError::Serialization(e.to_string()))?;
    let tmp = path.with_extension("lock.tmp");
    fs::write(&tmp, json)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

/// Returns `true` when `oci_ref_str` looks like an OCI reference (registry
/// host plus `:tag` or `@sha256:...`). Returns `false` for non-OCI sources
/// (`registry:local:...`, `file://`, `git://`, `https://` tarballs, empty
/// strings). Wave 5F — D5 uses this to decide whether the resolver should
/// attempt a per-component digest fetch.
pub fn is_oci_source(oci_ref_str: &str) -> bool {
    let trimmed = oci_ref_str.trim();
    if trimmed.is_empty() {
        return false;
    }
    for prefix in [
        "registry:local:",
        "file://",
        "file:",
        "git://",
        "git+",
        "git@",
        "http://",
        "https://",
        "ssh://",
    ] {
        if trimmed.starts_with(prefix) {
            return false;
        }
    }
    // OCI refs may be bare ("ghcr.io/...") or prefixed with `oci://`. Both
    // parse via OciRef::parse; we use that as the canonical recogniser.
    sindri_registry::OciRef::parse(trimmed).is_ok()
}

/// Read and parse an existing lockfile.
///
/// Applies the Phase-1.3 legacy-source backfill (see
/// [`backfill_legacy_source`]) before returning, so callers always observe a
/// `source: SourceDescriptor` on every component (when reconstructable)
/// regardless of when the lockfile was written.
pub fn read_lockfile(path: &Path) -> Result<Lockfile, ResolverError> {
    if !path.exists() {
        return Err(ResolverError::LockfileStale);
    }
    let content = fs::read_to_string(path)?;
    let mut lockfile: Lockfile =
        serde_json::from_str(&content).map_err(|e| ResolverError::Serialization(e.to_string()))?;
    backfill_legacy_source(&mut lockfile);
    Ok(lockfile)
}

/// One-shot guard for the legacy-lockfile warning emitted by
/// [`backfill_legacy_source`]. Keeps the warning to a single line per
/// process, matching the deprecation note in the source-modes plan §1.3.
static LEGACY_LOCKFILE_WARNING_EMITTED: AtomicBool = AtomicBool::new(false);

/// Backfill missing `source:` descriptors on a [`Lockfile`] read from disk
/// (Phase 1.3 backwards compatibility — DDD-08, ADR-028).
///
/// For each [`ResolvedComponent`] whose `source` is `None`, reconstructs an
/// `Oci { ... }` descriptor from the entry's `oci_digest` field (the
/// pre-Phase-1.3 location of the OCI ref). Emits a single deprecation
/// warning via `tracing::warn!` the first time a legacy lockfile is read in
/// a process, then mutates the lockfile in place.
///
/// Idempotent: no-op when every component already has a `source`.
pub fn backfill_legacy_source(lockfile: &mut Lockfile) {
    let mut backfilled = 0usize;
    for component in &mut lockfile.components {
        if component.source.is_some() {
            continue;
        }
        if let Some(legacy_ref) = component.oci_digest.as_deref() {
            if let Some(desc) = sindri_registry::source::oci_descriptor_from_legacy_ref(legacy_ref)
            {
                component.source = Some(desc);
                backfilled += 1;
            }
        }
    }
    if backfilled > 0 && !LEGACY_LOCKFILE_WARNING_EMITTED.swap(true, Ordering::Relaxed) {
        tracing::warn!(
            count = backfilled,
            "lockfile is missing per-component `source:` descriptors (pre-Phase-1.3 schema); reconstructed Oci{{...}} descriptors from legacy `oci_digest` field — re-run `sindri lock` to upgrade"
        );
    }
}

/// Build ResolvedComponent from a closure node.
///
/// `registry_manifest_digest` is the live OCI manifest digest returned by
/// `oci-client` when the resolver fetched the registry's `index.yaml` (Wave
/// 3A.2). When `None` (e.g. local-protocol fixtures, offline mode), the
/// lockfile entry omits `manifest_digest` for backwards compatibility.
///
/// `component_digest` is the SHA-256 digest of the component's primary OCI
/// layer, pre-fetched by the CLI for OCI-backed components (Wave 5F — D5
/// carry-over from PR #228). Components sourced from non-OCI locations
/// (local file, git URL, raw HTTP tarball) leave this `None`; under
/// `policy.require_signed_registries=true`, apply will fail closed for
/// components missing this field.
///
/// `platforms` carries the component's platform constraints (from its
/// `component.yaml`) so that `--offline` resolves can run Gate 1 without a
/// network call (Wave 6A / ADR-008). Pass `None` when the component manifest
/// has not been loaded.
///
/// Per ADR-003 audit-delta (Wave 3A.2): the registry-level `manifest_digest`
/// remains as an integrity tie-in for "this lockfile was resolved against
/// this exact `index.yaml` snapshot." The new `component_digest` is the
/// per-component analogue used by the cosign pre-flight in apply.
pub fn resolved_from_entry(
    entry: &ComponentEntry,
    chosen_backend: Backend,
    _bom_address: &str,
    registry_manifest_digest: Option<&str>,
    component_digest: Option<&str>,
    platforms: Option<Vec<Platform>>,
    source: Option<SourceDescriptor>,
) -> ResolvedComponent {
    let id = ComponentId {
        backend: chosen_backend.clone(),
        name: entry.name.clone(),
        qualifier: None,
    };
    ResolvedComponent {
        id,
        version: Version::new(&entry.latest),
        backend: chosen_backend,
        oci_digest: Some(entry.oci_ref.clone()),
        checksums: Default::default(),
        depends_on: entry.depends_on.clone(),
        // Wave 3A will fetch manifests from OCI; until then, the apply
        // pipeline degrades to install + hooks only when manifest is None.
        manifest: None,
        manifest_digest: registry_manifest_digest.map(|s| s.to_string()),
        component_digest: component_digest.map(|s| s.to_string()),
        // Wave 6A: platform constraints from the component manifest, used by
        // the offline Gate 1 path (ADR-008).
        platforms,
        // Phase 1.3 (DDD-08, ADR-028): source descriptor recorded so
        // `sindri apply` can refetch identically. Pre-Phase-1.3 lockfiles
        // omit this; legacy reads reconstruct it via
        // [`backfill_legacy_source`].
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sindri_core::registry::ComponentKind;

    fn entry(name: &str, oci_ref: &str) -> ComponentEntry {
        ComponentEntry {
            name: name.into(),
            backend: "binary".into(),
            latest: "1.0.0".into(),
            versions: vec!["1.0.0".into()],
            description: "test".into(),
            kind: ComponentKind::Component,
            oci_ref: oci_ref.into(),
            license: "MIT".into(),
            depends_on: vec![],
        }
    }

    #[test]
    fn oci_source_detection() {
        // Wave 5F — D5: classify a ref as OCI vs non-OCI.
        assert!(is_oci_source(
            "ghcr.io/sindri-dev/registry-core/nodejs:22.0.0"
        ));
        assert!(is_oci_source(
            "oci://ghcr.io/sindri-dev/registry-core/nodejs:22.0.0"
        ));
        assert!(is_oci_source(
            "registry.example.com/foo/bar@sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        ));

        assert!(!is_oci_source("registry:local:/tmp/fixtures/registry"));
        assert!(!is_oci_source("file:///tmp/foo.tar.gz"));
        assert!(!is_oci_source("https://example.com/foo.tar.gz"));
        assert!(!is_oci_source("git+https://github.com/foo/bar.git"));
        assert!(!is_oci_source(""));
    }

    #[test]
    fn resolved_component_omits_digest_for_non_oci_source() {
        // Wave 5F — D5: a component sourced from a local registry (or any
        // non-OCI location) MUST leave `component_digest` as None. The
        // contract is documented on `resolved_from_entry`.
        let e = entry("local-tool", "registry:local:/tmp/fixtures/registry");
        let resolved = resolved_from_entry(
            &e,
            Backend::Binary,
            "binary:local-tool",
            None,
            None,
            None,
            None,
        );
        assert!(resolved.component_digest.is_none());
    }

    #[test]
    fn resolved_component_carries_digest_when_provided() {
        let e = entry("nodejs", "ghcr.io/sindri-dev/registry-core/nodejs:22.0.0");
        let digest = "sha256:deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
        let resolved = resolved_from_entry(
            &e,
            Backend::Mise,
            "mise:nodejs",
            None,
            Some(digest),
            None,
            None,
        );
        assert_eq!(resolved.component_digest.as_deref(), Some(digest));
    }
}
