use crate::error::ResolverError;
use sindri_core::component::{Backend, ComponentId};
use sindri_core::lockfile::{Lockfile, ResolvedComponent};
use sindri_core::registry::ComponentEntry;
use sindri_core::version::Version;
use std::fs;
use std::path::Path;

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

/// Read and parse an existing lockfile
pub fn read_lockfile(path: &Path) -> Result<Lockfile, ResolverError> {
    if !path.exists() {
        return Err(ResolverError::LockfileStale);
    }
    let content = fs::read_to_string(path)?;
    serde_json::from_str(&content).map_err(|e| ResolverError::Serialization(e.to_string()))
}

/// Build ResolvedComponent from a closure node.
///
/// `registry_manifest_digest` is the live OCI manifest digest returned by
/// `oci-client` when the resolver fetched the registry's `index.yaml` (Wave
/// 3A.2). When `None` (e.g. local-protocol fixtures, offline mode), the
/// lockfile entry omits `manifest_digest` for backwards compatibility.
///
/// Per ADR-003 audit-delta (Wave 3A.2): per-component manifest digests
/// (each component carrying its own OCI digest) are deferred to the SBOM
/// work in Wave 5. This field carries the *registry-level* artifact digest
/// — an integrity tie-in for "this lockfile was resolved against this
/// exact `index.yaml` snapshot."
pub fn resolved_from_entry(
    entry: &ComponentEntry,
    chosen_backend: Backend,
    _bom_address: &str,
    registry_manifest_digest: Option<&str>,
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
    }
}
