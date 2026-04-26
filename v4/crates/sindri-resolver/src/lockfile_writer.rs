use std::fs;
use std::path::Path;
use sindri_core::lockfile::{Lockfile, ResolvedComponent};
use sindri_core::component::{Backend, ComponentId};
use sindri_core::version::Version;
use sindri_core::registry::ComponentEntry;
use crate::error::ResolverError;

/// Compute bom_hash as sha256 of the sindri.yaml content
pub fn compute_bom_hash(bom_content: &str) -> String {
    use sha2::{Sha256, Digest};
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
    serde_json::from_str(&content)
        .map_err(|e| ResolverError::Serialization(e.to_string()))
}

/// Build ResolvedComponent from a closure node
pub fn resolved_from_entry(
    entry: &ComponentEntry,
    chosen_backend: Backend,
    bom_address: &str,
) -> ResolvedComponent {
    let id = ComponentId {
        backend: chosen_backend.clone(),
        name: entry.name.clone(),
    };
    ResolvedComponent {
        id,
        version: Version::new(&entry.latest),
        backend: chosen_backend,
        oci_digest: Some(entry.oci_ref.clone()),
        checksums: Default::default(),
        depends_on: entry.depends_on.clone(),
    }
}
