//! Filesystem-path source (DDD-08, ADR-028).
//!
//! `LocalPathSource` is the seed of the Source trait surface. It walks a
//! directory containing `index.yaml` (and optional per-component
//! `component.yaml` files) and produces a [`RegistryIndex`] in memory. It is
//! the canonical inner-loop authoring path: edit a `component.yaml`, re-run
//! `sindri lock`, see the change without round-tripping through OCI.
//!
//! This module is a refactor of the previous `crate::local::LocalRegistry`.
//! The legacy type is preserved as a deprecated alias (see `crate::local`)
//! for one release.

use crate::error::RegistryError;
use crate::index::RegistryIndex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sindri_core::component::ComponentManifest;
use sindri_core::version::Version;
use std::fs;
use std::path::PathBuf;

use super::{
    ComponentBlob, ComponentId, ComponentName, Source, SourceContext, SourceDescriptor, SourceError,
};

/// A filesystem-path registry source — DDD-08 §"`LocalPathSource`".
///
/// `path` is a directory containing `index.yaml` at its root. Per-component
/// `component.yaml` files live under `components/<name>/component.yaml`
/// (or `collections/<name>/component.yaml` for collection kinds).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct LocalPathSource {
    /// Root directory of the on-disk registry layout.
    pub path: PathBuf,
    /// Optional component-name allow-list. When `Some`, the resolver skips
    /// any component whose name is not in this list and falls through to the
    /// next source in declared order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<Vec<ComponentName>>,
}

impl LocalPathSource {
    /// Construct from a path string. Convenience for migrating existing
    /// `LocalRegistry::new(path)` call sites.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            scope: None,
        }
    }

    /// Attach a scope filter. See [`LocalPathSource::scope`].
    pub fn with_scope(mut self, scope: Vec<ComponentName>) -> Self {
        self.scope = Some(scope);
        self
    }

    /// Read `index.yaml` from the root and parse it. Mirrors the legacy
    /// `LocalRegistry::load_index` behaviour exactly.
    pub fn load_index(&self) -> Result<RegistryIndex, RegistryError> {
        let content = fs::read_to_string(self.path.join("index.yaml"))?;
        RegistryIndex::from_yaml(&content).map_err(RegistryError::Yaml)
    }

    /// Load a single `component.yaml` blob by `(backend, name)`. Mirrors the
    /// legacy `LocalRegistry::load_component` behaviour exactly.
    pub fn load_component(
        &self,
        backend: &str,
        name: &str,
    ) -> Result<ComponentManifest, RegistryError> {
        let dir = if backend == "collection" {
            self.path.join("collections").join(name)
        } else {
            self.path.join("components").join(name)
        };
        let content = fs::read_to_string(dir.join("component.yaml"))?;
        serde_yaml::from_str(&content).map_err(RegistryError::Yaml)
    }

    /// Enumerate every `(backend, name)` pair in the local index.
    pub fn list_components(&self) -> Result<Vec<(String, String)>, RegistryError> {
        let index = self.load_index()?;
        Ok(index
            .components
            .iter()
            .map(|c| (c.backend.clone(), c.name.clone()))
            .collect())
    }
}

impl Source for LocalPathSource {
    fn fetch_index(&self, _ctx: &SourceContext) -> Result<RegistryIndex, SourceError> {
        let mut index = self
            .load_index()
            .map_err(|e| SourceError::InvalidData(e.to_string()))?;

        // Apply the scope filter to the in-memory catalog. The resolver also
        // applies scope at lookup time (DDD-08 §"Resolution Algorithm"), but
        // pre-filtering the index lets `--explain` show a coherent partial
        // catalog without leaking out-of-scope entries.
        if let Some(scope) = self.scope.as_ref() {
            let allow: std::collections::HashSet<&str> = scope.iter().map(|n| n.as_str()).collect();
            index.components.retain(|c| allow.contains(c.name.as_str()));
        }

        Ok(index)
    }

    fn fetch_component_blob(
        &self,
        id: &ComponentId,
        _version: &Version,
        _ctx: &SourceContext,
    ) -> Result<ComponentBlob, SourceError> {
        // Honor the scope filter at the blob level too — keeps this trait
        // honest under direct consumer use, not just resolver use.
        if !self
            .scope
            .as_ref()
            .map(|s| s.iter().any(|n| n == &id.name))
            .unwrap_or(true)
        {
            return Err(SourceError::NotFound(id.name.as_str().to_string()));
        }

        let dir = if id.backend == "collection" {
            self.path.join("collections").join(id.name.as_str())
        } else {
            self.path.join("components").join(id.name.as_str())
        };
        let yaml_path = dir.join("component.yaml");
        let bytes = fs::read(&yaml_path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => SourceError::NotFound(id.name.as_str().to_string()),
            _ => SourceError::Io(format!("{}: {}", yaml_path.display(), e)),
        })?;
        Ok(ComponentBlob {
            bytes,
            digest: None,
        })
    }

    fn lockfile_descriptor(&self) -> SourceDescriptor {
        SourceDescriptor::LocalPath {
            path: self.path.clone(),
        }
    }

    fn supports_strict_oci(&self) -> bool {
        // `LocalPath` is intentionally non-reproducible across machines.
        // DDD-08 §Invariant 4: a LocalPath source MUST NOT be admitted under
        // `--strict-oci`.
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sindri_core::registry::{ComponentEntry, ComponentKind};
    use std::fs as stdfs;
    use tempfile::TempDir;

    fn write_index(dir: &TempDir, components: &[(&str, &str)]) {
        let entries: Vec<ComponentEntry> = components
            .iter()
            .map(|(backend, name)| ComponentEntry {
                name: (*name).into(),
                backend: (*backend).into(),
                latest: "1.0.0".into(),
                versions: vec!["1.0.0".into()],
                description: "test".into(),
                kind: ComponentKind::Component,
                oci_ref: format!("local://{}", name),
                license: "MIT".into(),
                depends_on: vec![],
            })
            .collect();
        let idx = RegistryIndex {
            version: 1,
            registry: "test".into(),
            generated_at: None,
            components: entries,
        };
        let yaml = idx.to_yaml().unwrap();
        stdfs::write(dir.path().join("index.yaml"), yaml).unwrap();
    }

    fn write_component(dir: &TempDir, backend: &str, name: &str) {
        let sub = if backend == "collection" {
            dir.path().join("collections").join(name)
        } else {
            dir.path().join("components").join(name)
        };
        stdfs::create_dir_all(&sub).unwrap();
        let yaml = format!(
            r#"metadata:
  name: {name}
  version: "1.0.0"
  description: test
  license: MIT
  tags: []
platforms: []
install: {{}}
depends_on: []
"#,
            name = name
        );
        stdfs::write(sub.join("component.yaml"), yaml).unwrap();
    }

    /// Existing-behaviour smoke test (carried over from the legacy
    /// `LocalRegistry` so the alias keeps working).
    #[test]
    fn load_index_round_trips() {
        let dir = TempDir::new().unwrap();
        write_index(&dir, &[("mise", "nodejs"), ("mise", "rust")]);

        let src = LocalPathSource::new(dir.path());
        let idx = src.load_index().unwrap();
        assert_eq!(idx.components.len(), 2);
    }

    /// New scope test #1 — in-scope name yields the entry.
    #[test]
    fn fetch_index_scope_in_scope_match() {
        let dir = TempDir::new().unwrap();
        write_index(&dir, &[("mise", "nodejs"), ("mise", "rust")]);

        let src = LocalPathSource::new(dir.path()).with_scope(vec![ComponentName::from("nodejs")]);

        let idx = src.fetch_index(&SourceContext::default()).unwrap();
        assert_eq!(idx.components.len(), 1);
        assert_eq!(idx.components[0].name, "nodejs");
    }

    /// New scope test #2 — out-of-scope name is filtered out and
    /// `fetch_component_blob` returns `NotFound` so the resolver falls
    /// through to the next source.
    #[test]
    fn fetch_index_scope_out_of_scope_skip() {
        let dir = TempDir::new().unwrap();
        write_index(&dir, &[("mise", "nodejs"), ("mise", "rust")]);
        write_component(&dir, "mise", "rust");

        let src = LocalPathSource::new(dir.path()).with_scope(vec![ComponentName::from("nodejs")]);

        // Index pre-filtered.
        let idx = src.fetch_index(&SourceContext::default()).unwrap();
        assert!(idx.components.iter().all(|c| c.name == "nodejs"));

        // Blob fetch refuses the out-of-scope component.
        let id = ComponentId {
            backend: "mise".into(),
            name: ComponentName::from("rust"),
        };
        let err = src
            .fetch_component_blob(&id, &Version::new("1.0.0"), &SourceContext::default())
            .unwrap_err();
        match err {
            SourceError::NotFound(name) => assert_eq!(name, "rust"),
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    /// New scope test #3 — multi-name scope matches every listed name.
    #[test]
    fn fetch_index_scope_multi_name() {
        let dir = TempDir::new().unwrap();
        write_index(
            &dir,
            &[("mise", "nodejs"), ("mise", "rust"), ("mise", "go")],
        );

        let src = LocalPathSource::new(dir.path()).with_scope(vec![
            ComponentName::from("nodejs"),
            ComponentName::from("go"),
        ]);

        let idx = src.fetch_index(&SourceContext::default()).unwrap();
        let mut names: Vec<&str> = idx.components.iter().map(|c| c.name.as_str()).collect();
        names.sort();
        assert_eq!(names, vec!["go", "nodejs"]);
    }

    #[test]
    fn lockfile_descriptor_records_path() {
        let src = LocalPathSource::new("/tmp/registry");
        let d = src.lockfile_descriptor();
        match d {
            SourceDescriptor::LocalPath { path } => {
                assert_eq!(path, PathBuf::from("/tmp/registry"));
            }
            _ => panic!("expected LocalPath descriptor"),
        }
    }

    #[test]
    fn local_path_does_not_support_strict_oci() {
        let src = LocalPathSource::new("/tmp/x");
        assert!(!src.supports_strict_oci());
    }

    #[test]
    fn fetch_component_blob_reads_yaml_bytes() {
        let dir = TempDir::new().unwrap();
        write_index(&dir, &[("mise", "nodejs")]);
        write_component(&dir, "mise", "nodejs");

        let src = LocalPathSource::new(dir.path());
        let id = ComponentId {
            backend: "mise".into(),
            name: ComponentName::from("nodejs"),
        };
        let blob = src
            .fetch_component_blob(&id, &Version::new("1.0.0"), &SourceContext::default())
            .unwrap();
        assert!(blob.digest.is_none());
        let txt = std::str::from_utf8(&blob.bytes).unwrap();
        assert!(txt.contains("name: nodejs"));
    }
}
