//! Lockfile / desired-document I/O.
//!
//! The on-disk format for both the desired infra (extracted from
//! `sindri.yaml::targets.<name>.infra`) and the recorded lockfile
//! (`sindri.<name>.infra.lock`) is a single YAML mapping:
//!
//! ```yaml
//! kind: docker
//! resources:
//!   web:
//!     image: ghcr.io/example/web:1.2.3
//!     env:
//!       LOG: info
//!   cache:
//!     image: redis:7
//! ```
//!
//! `kind` is required in the lock file; in the desired document it
//! comes from `targets.<name>.kind` (a sibling of `infra:`) and is
//! injected by the loader. `resources:` is required and may be empty.
//!
//! The lock file is written *atomically* — see [`write_lock_atomic`].

use crate::error::TargetError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

/// In-memory shape used by the convergence engine for both desired and
/// recorded documents.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct InfraDocument {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub resources: BTreeMap<String, Value>,
}

impl InfraDocument {
    /// Build from the raw `targets.<name>.infra` YAML value. `kind`
    /// comes from the sibling `targets.<name>.kind` field.
    ///
    /// The infra value is expected to be a mapping with a top-level
    /// `resources:` key. As a backwards-compatibility shim, if the
    /// value is a mapping that does *not* contain `resources:`, the
    /// whole mapping is treated as a single resource named `default`.
    pub fn from_infra_value(kind: &str, infra: Option<&Value>) -> Self {
        let mut doc = InfraDocument {
            kind: kind.to_string(),
            ..Default::default()
        };
        let infra = match infra {
            Some(v) => v,
            None => return doc,
        };
        if let Some(map) = infra.as_object() {
            if let Some(resources) = map.get("resources").and_then(|v| v.as_object()) {
                for (k, v) in resources {
                    doc.resources.insert(k.clone(), v.clone());
                }
            } else if !map.is_empty() {
                // Single-resource shim — wrap the mapping under `default`.
                doc.resources
                    .insert("default".to_string(), Value::Object(map.clone()));
            }
        }
        doc
    }

    /// Read a lock file from disk. Returns an empty document if the
    /// path does not exist.
    pub fn read_lock(path: &Path) -> Result<Self, TargetError> {
        if !path.exists() {
            return Ok(InfraDocument::default());
        }
        let content = std::fs::read_to_string(path).map_err(TargetError::Io)?;
        let doc: InfraDocument =
            serde_yaml::from_str(&content).map_err(|e| TargetError::ExecFailed {
                target: path.display().to_string(),
                detail: format!("failed to parse infra lock: {}", e),
            })?;
        Ok(doc)
    }
}

/// Recorded state for a single resource. We use `serde_json::Value`
/// internally so the lock format stays free-form (matches the desired
/// document's free-form `infra:`).
pub type ResourceState = Value;

/// The lock file as a whole — the on-disk representation of every
/// provisioned resource for a given target.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InfraLock {
    pub target_name: String,
    pub kind: String,
    #[serde(default)]
    pub resources: BTreeMap<String, ResourceState>,
}

impl InfraLock {
    pub fn new(target_name: &str, kind: &str) -> Self {
        InfraLock {
            target_name: target_name.to_string(),
            kind: kind.to_string(),
            resources: BTreeMap::new(),
        }
    }

    /// Read the lockfile for a target; returns an empty lock if the
    /// file does not exist.
    pub fn read(path: &Path, target_name: &str, kind: &str) -> Result<Self, TargetError> {
        if !path.exists() {
            return Ok(InfraLock::new(target_name, kind));
        }
        let content = std::fs::read_to_string(path).map_err(TargetError::Io)?;
        let doc: InfraDocument =
            serde_yaml::from_str(&content).map_err(|e| TargetError::ExecFailed {
                target: path.display().to_string(),
                detail: format!("failed to parse infra lock: {}", e),
            })?;
        Ok(InfraLock {
            target_name: target_name.to_string(),
            kind: if doc.kind.is_empty() {
                kind.to_string()
            } else {
                doc.kind
            },
            resources: doc.resources,
        })
    }
}

/// Atomically write `lock` to `path` — write to a sibling `.tmp` file
/// and `rename(2)` it into place. The serialised form is the same
/// `kind` + `resources` mapping read by [`InfraDocument::read_lock`].
pub fn write_lock_atomic(path: &Path, lock: &InfraLock) -> Result<(), TargetError> {
    let doc = InfraDocument {
        kind: lock.kind.clone(),
        resources: lock.resources.clone(),
    };
    let serialised = serde_yaml::to_string(&doc).map_err(|e| TargetError::ExecFailed {
        target: lock.target_name.clone(),
        detail: format!("failed to serialise infra lock: {}", e),
    })?;
    let tmp = path.with_extension("lock.tmp");
    std::fs::write(&tmp, &serialised).map_err(TargetError::Io)?;
    std::fs::rename(&tmp, path).map_err(TargetError::Io)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn from_infra_value_with_resources_key() {
        let v = json!({
            "resources": {
                "web": {"image": "ubuntu:24.04"}
            }
        });
        let doc = InfraDocument::from_infra_value("docker", Some(&v));
        assert_eq!(doc.kind, "docker");
        assert_eq!(doc.resources.len(), 1);
        assert!(doc.resources.contains_key("web"));
    }

    #[test]
    fn from_infra_value_single_resource_shim() {
        let v = json!({"image": "ubuntu:24.04"});
        let doc = InfraDocument::from_infra_value("docker", Some(&v));
        assert_eq!(doc.resources.len(), 1);
        assert!(doc.resources.contains_key("default"));
    }

    #[test]
    fn write_then_read_lock_round_trips() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sindri.web.infra.lock");
        let mut lock = InfraLock::new("web", "docker");
        lock.resources.insert(
            "web".to_string(),
            json!({"image": "ubuntu:24.04", "id": "abc123"}),
        );
        write_lock_atomic(&path, &lock).unwrap();

        let read = InfraLock::read(&path, "web", "docker").unwrap();
        assert_eq!(read.kind, "docker");
        assert_eq!(read.resources.len(), 1);
        assert_eq!(
            read.resources.get("web").unwrap().get("id").unwrap(),
            &json!("abc123")
        );
    }

    #[test]
    fn read_missing_lock_returns_empty() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("does-not-exist.lock");
        let lock = InfraLock::read(&path, "web", "docker").unwrap();
        assert!(lock.resources.is_empty());
    }

    #[test]
    fn atomic_write_does_not_leave_tmp_behind() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("a.lock");
        let lock = InfraLock::new("a", "docker");
        write_lock_atomic(&path, &lock).unwrap();
        assert!(path.exists());
        let tmp = path.with_extension("lock.tmp");
        assert!(!tmp.exists(), "tmp file should be gone after rename");
    }
}
