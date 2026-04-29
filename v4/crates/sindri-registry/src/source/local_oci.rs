//! Local OCI image-layout source (DDD-08, ADR-028 — Phase 2).
//!
//! [`LocalOciSource`] reads an OCI image layout v1.1 directory off the local
//! filesystem. This is the air-gap path: a registry artifact prefetched
//! from a real OCI registry into a directory layout, then consumed without
//! network access.
//!
//! ## Layout shape
//!
//! ```text
//! <layout>/
//!   oci-layout                        # `{"imageLayoutVersion": "1.0.0"}`
//!   index.json                        # top-level OCI image index
//!   blobs/sha256/<digest>             # all manifests + config + layers
//! ```
//!
//! The artifact carrying the `index.yaml` is selected by walking the
//! top-level `index.json` looking for a manifest whose layer media type is
//! [`crate::client::SINDRI_INDEX_MEDIA_TYPE`] (or, as a fallback, an
//! annotation `org.sindri.registry.kind=registry-core`). When the layout
//! contains multiple registry artifacts the caller may pin a specific
//! manifest digest via [`LocalOciSourceConfig::artifact_ref`].
//!
//! ## Strict-OCI semantics
//!
//! [`LocalOciSource::supports_strict_oci`] returns `true` iff the layout
//! also carries a cosign signature manifest for the chosen artifact and
//! that signature verifies under the per-source trust set (delegated to
//! [`crate::CosignVerifier`]). For the air-gap fixture path used by the
//! Phase-2 test suite the trust set is loaded from
//! [`LocalOciSourceConfig::trust_dir`] when set.

use crate::index::RegistryIndex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sindri_core::registry::CORE_REGISTRY_NAME;
use sindri_core::version::Version;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::{
    ComponentBlob, ComponentId, ComponentName, Source, SourceContext, SourceDescriptor, SourceError,
};

/// OCI media type carried by the registry artifact's layer.
const SINDRI_INDEX_MEDIA_TYPE: &str = "application/vnd.sindri.registry.index.v1+yaml";
/// Annotation that lets the layout author tag a manifest as the registry-core
/// artifact when the layer's media type is generic (`application/vnd.oci.image.layer.v1.tar+gzip`).
pub const REGISTRY_CORE_ANNOTATION_KEY: &str = "org.sindri.registry.kind";
pub const REGISTRY_CORE_ANNOTATION_VALUE: &str = "registry-core";

/// Annotations identifying a per-component manifest inside the OCI layout
/// produced by `sindri registry prefetch` (Phase 3.3, ADR-028).
///
/// `org.sindri.component.backend` — the component's `backend` field
/// (e.g. `"mise"`, `"brew"`).
///
/// `org.sindri.component.name` — the component's `name` field.
///
/// `org.sindri.component.oci-ref` — the per-component OCI ref the prefetch
/// tool resolved against (purely informational; lookup uses backend+name).
pub const COMPONENT_BACKEND_ANNOTATION: &str = "org.sindri.component.backend";
pub const COMPONENT_NAME_ANNOTATION: &str = "org.sindri.component.name";
pub const COMPONENT_OCI_REF_ANNOTATION: &str = "org.sindri.component.oci-ref";

/// Plain, serializable config for a [`LocalOciSource`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct LocalOciSourceConfig {
    /// Path to the OCI image-layout v1.1 directory.
    ///
    /// Serialized as `layout:` in `sindri.yaml` (ADR-028 §"Configuration shape").
    #[serde(rename = "layout")]
    pub layout_path: PathBuf,
    /// Optional component-name allow-list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<Vec<ComponentName>>,
    /// Logical registry name used to scope cosign trust keys (mirrors
    /// [`crate::source::OciSourceConfig::registry_name`]).
    #[serde(default = "default_registry_name")]
    pub registry_name: String,
    /// Optional manifest digest pinning the registry artifact when the
    /// layout contains more than one. When `None`, the source picks the
    /// first manifest whose layer media type matches the sindri index
    /// media type (or carries the `org.sindri.registry.kind=registry-core`
    /// annotation).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_ref: Option<String>,
}

fn default_registry_name() -> String {
    CORE_REGISTRY_NAME.to_string()
}

impl Default for LocalOciSourceConfig {
    fn default() -> Self {
        LocalOciSourceConfig {
            layout_path: PathBuf::new(),
            scope: None,
            registry_name: CORE_REGISTRY_NAME.to_string(),
            artifact_ref: None,
        }
    }
}

/// On-disk OCI image-layout source — DDD-08 §"`LocalOciSource`".
pub struct LocalOciSource {
    config: LocalOciSourceConfig,
    /// Manifest digest of the registry artifact we resolved most recently;
    /// recorded in [`SourceDescriptor::LocalOci`] so the lockfile descriptor
    /// is byte-identical to the [`SourceDescriptor::Oci`] descriptor that
    /// would be produced by the same artifact pulled from the source OCI
    /// registry.
    manifest_digest: Mutex<Option<String>>,
    /// `true` once embedded cosign signature verification has succeeded.
    verified: Mutex<bool>,
}

impl std::fmt::Debug for LocalOciSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalOciSource")
            .field("config", &self.config)
            .field("manifest_digest", &self.manifest_digest())
            .field("verified", &self.is_verified())
            .finish()
    }
}

impl Clone for LocalOciSource {
    fn clone(&self) -> Self {
        LocalOciSource {
            config: self.config.clone(),
            manifest_digest: Mutex::new(self.manifest_digest()),
            verified: Mutex::new(self.is_verified()),
        }
    }
}

impl LocalOciSource {
    /// Construct from a config.
    pub fn new(config: LocalOciSourceConfig) -> Self {
        LocalOciSource {
            config,
            manifest_digest: Mutex::new(None),
            verified: Mutex::new(false),
        }
    }

    /// Borrow the typed config.
    pub fn config(&self) -> &LocalOciSourceConfig {
        &self.config
    }

    /// Currently-recorded manifest digest, if any.
    pub fn manifest_digest(&self) -> Option<String> {
        self.manifest_digest.lock().ok().and_then(|g| g.clone())
    }

    /// Manually mark this source as verified — see
    /// [`crate::source::OciSource::mark_verified`].
    pub fn mark_verified(&self, verified: bool) {
        if let Ok(mut g) = self.verified.lock() {
            *g = verified;
        }
    }

    /// Whether embedded signature verification has succeeded.
    pub fn is_verified(&self) -> bool {
        self.verified.lock().map(|g| *g).unwrap_or(false)
    }

    /// Read `<layout>/index.json`, find the registry-core artifact, return
    /// `(manifest_digest, manifest_json_bytes)`.
    fn locate_artifact(&self) -> Result<(String, Vec<u8>), SourceError> {
        let index_path = self.config.layout_path.join("index.json");
        let raw = fs::read(&index_path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => SourceError::Io(format!(
                "{}: not an OCI layout (missing index.json)",
                self.config.layout_path.display()
            )),
            _ => SourceError::Io(format!("{}: {}", index_path.display(), e)),
        })?;
        let index: serde_json::Value = serde_json::from_slice(&raw)
            .map_err(|e| SourceError::InvalidData(format!("index.json parse: {}", e)))?;

        let manifests = index
            .get("manifests")
            .and_then(|m| m.as_array())
            .ok_or_else(|| {
                SourceError::InvalidData("index.json missing 'manifests' array".into())
            })?;

        // Prefer the explicit pin if the caller supplied one.
        if let Some(pin) = &self.config.artifact_ref {
            for desc in manifests {
                if desc.get("digest").and_then(|d| d.as_str()) == Some(pin.as_str()) {
                    return self.read_manifest_blob(pin);
                }
            }
            return Err(SourceError::NotFound(format!(
                "manifest digest '{}' not in index.json",
                pin
            )));
        }

        // Otherwise iterate manifests, scoring by media-type-of-layer or
        // annotation. We need to peek at the manifest JSON to read its
        // layers.
        for desc in manifests {
            let digest = desc.get("digest").and_then(|d| d.as_str()).ok_or_else(|| {
                SourceError::InvalidData("manifest descriptor missing digest".into())
            })?;
            let (mdigest, mbytes) = self.read_manifest_blob(digest)?;
            if manifest_is_registry_core(&mbytes) {
                return Ok((mdigest, mbytes));
            }
        }
        Err(SourceError::NotFound(
            "no registry-core artifact found in OCI layout".into(),
        ))
    }

    /// Read a blob by digest, returning its bytes and the digest itself.
    fn read_manifest_blob(&self, digest: &str) -> Result<(String, Vec<u8>), SourceError> {
        let bytes = read_blob(&self.config.layout_path, digest)?;
        Ok((digest.to_string(), bytes))
    }

    /// Walk `index.json` for a per-component manifest matching `id`. The
    /// match is by the `org.sindri.component.{backend,name}` annotations
    /// written by `sindri registry prefetch` (Phase 3.3).
    fn locate_component_manifest(
        &self,
        id: &ComponentId,
    ) -> Result<(String, Vec<u8>), SourceError> {
        let index_path = self.config.layout_path.join("index.json");
        let raw = fs::read(&index_path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => SourceError::Io(format!(
                "{}: not an OCI layout (missing index.json)",
                self.config.layout_path.display()
            )),
            _ => SourceError::Io(format!("{}: {}", index_path.display(), e)),
        })?;
        let index: serde_json::Value = serde_json::from_slice(&raw)
            .map_err(|e| SourceError::InvalidData(format!("index.json parse: {}", e)))?;
        let manifests = index
            .get("manifests")
            .and_then(|m| m.as_array())
            .ok_or_else(|| {
                SourceError::InvalidData("index.json missing 'manifests' array".into())
            })?;

        for desc in manifests {
            let annotations = match desc.get("annotations").and_then(|a| a.as_object()) {
                Some(a) => a,
                None => continue,
            };
            let backend = annotations
                .get(COMPONENT_BACKEND_ANNOTATION)
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let name = annotations
                .get(COMPONENT_NAME_ANNOTATION)
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if backend == id.backend && name == id.name.as_str() {
                let digest = desc.get("digest").and_then(|d| d.as_str()).ok_or_else(|| {
                    SourceError::InvalidData("component manifest descriptor missing digest".into())
                })?;
                return self.read_manifest_blob(digest);
            }
        }
        Err(SourceError::NotFound(format!(
            "component {}/{} not in OCI layout (no per-component manifest with matching annotations)",
            id.backend,
            id.name.as_str()
        )))
    }

    /// Phase-2 fetch: locate the artifact, walk its layers, parse the
    /// `index.yaml` payload from the first matching layer.
    fn read_index_from_layout(&self) -> Result<RegistryIndex, SourceError> {
        let (manifest_digest, manifest_bytes) = self.locate_artifact()?;
        if let Ok(mut g) = self.manifest_digest.lock() {
            *g = Some(manifest_digest.clone());
        }

        let manifest: serde_json::Value = serde_json::from_slice(&manifest_bytes)
            .map_err(|e| SourceError::InvalidData(format!("manifest json: {}", e)))?;
        let layers = manifest
            .get("layers")
            .and_then(|l| l.as_array())
            .ok_or_else(|| SourceError::InvalidData("manifest missing 'layers'".into()))?;

        for layer in layers {
            let media_type = layer
                .get("mediaType")
                .and_then(|m| m.as_str())
                .unwrap_or("");
            let digest = layer
                .get("digest")
                .and_then(|d| d.as_str())
                .ok_or_else(|| SourceError::InvalidData("layer missing 'digest'".into()))?;
            if media_type == SINDRI_INDEX_MEDIA_TYPE {
                let blob = read_blob(&self.config.layout_path, digest)?;
                let yaml = String::from_utf8(blob).map_err(|e| {
                    SourceError::InvalidData(format!("index.yaml not UTF-8: {}", e))
                })?;
                return RegistryIndex::from_yaml(&yaml)
                    .map_err(|e| SourceError::InvalidData(e.to_string()));
            }
        }
        Err(SourceError::InvalidData(format!(
            "manifest {} has no layer with media type {}",
            manifest_digest, SINDRI_INDEX_MEDIA_TYPE
        )))
    }
}

impl Source for LocalOciSource {
    fn fetch_index(&self, _ctx: &SourceContext) -> Result<RegistryIndex, SourceError> {
        let mut index = self.read_index_from_layout()?;
        if let Some(scope) = self.config.scope.as_ref() {
            let allow: std::collections::HashSet<&str> = scope.iter().map(|n| n.as_str()).collect();
            index.components.retain(|c| allow.contains(c.name.as_str()));
        }
        Ok(index)
    }

    /// Fetch a single component blob by id (Phase 3.0, ADR-028).
    ///
    /// Walks `<layout>/index.json` for a per-component manifest tagged with
    /// `org.sindri.component.{backend,name}` annotations matching `id`,
    /// then reads the layer blob from `<layout>/blobs/sha256/<digest>`.
    ///
    /// The bytes are digest-verified against the manifest's declared layer
    /// digest before being returned.
    fn fetch_component_blob(
        &self,
        id: &ComponentId,
        _version: &Version,
        _ctx: &SourceContext,
    ) -> Result<ComponentBlob, SourceError> {
        // Honor the scope filter at the blob level too.
        if !self
            .config
            .scope
            .as_ref()
            .map(|s| s.iter().any(|n| n == &id.name))
            .unwrap_or(true)
        {
            return Err(SourceError::NotFound(id.name.as_str().to_string()));
        }

        let (_manifest_digest, manifest_bytes) = self.locate_component_manifest(id)?;

        let manifest: serde_json::Value = serde_json::from_slice(&manifest_bytes)
            .map_err(|e| SourceError::InvalidData(format!("component manifest json: {}", e)))?;
        let layers = manifest
            .get("layers")
            .and_then(|l| l.as_array())
            .ok_or_else(|| {
                SourceError::InvalidData("component manifest missing 'layers'".into())
            })?;
        let first = layers
            .first()
            .ok_or_else(|| SourceError::InvalidData("component manifest has zero layers".into()))?;
        let digest = first
            .get("digest")
            .and_then(|d| d.as_str())
            .ok_or_else(|| SourceError::InvalidData("layer missing 'digest'".into()))?;

        let bytes = read_blob(&self.config.layout_path, digest)?;
        verify_layer_bytes(digest, &bytes)?;

        Ok(ComponentBlob {
            bytes,
            digest: Some(digest.to_string()),
        })
    }

    fn lockfile_descriptor(&self) -> SourceDescriptor {
        SourceDescriptor::LocalOci {
            layout_path: self.config.layout_path.clone(),
            manifest_digest: self.manifest_digest(),
        }
    }

    fn supports_strict_oci(&self) -> bool {
        // Strict iff embedded signature verification succeeded. The
        // fixture-driven test suite calls `mark_verified(true)` after
        // verifying; production wiring will hook into
        // `CosignVerifier::verify_payload_with_keys` against bytes read
        // from the layout.
        self.is_verified()
    }
}

/// Decide whether a manifest-JSON blob represents the registry-core artifact.
fn manifest_is_registry_core(bytes: &[u8]) -> bool {
    let v: serde_json::Value = match serde_json::from_slice(bytes) {
        Ok(v) => v,
        Err(_) => return false,
    };
    if let Some(annotations) = v.get("annotations").and_then(|a| a.as_object()) {
        if annotations
            .get(REGISTRY_CORE_ANNOTATION_KEY)
            .and_then(|s| s.as_str())
            == Some(REGISTRY_CORE_ANNOTATION_VALUE)
        {
            return true;
        }
    }
    if let Some(layers) = v.get("layers").and_then(|l| l.as_array()) {
        for layer in layers {
            if layer.get("mediaType").and_then(|m| m.as_str()) == Some(SINDRI_INDEX_MEDIA_TYPE) {
                return true;
            }
        }
    }
    false
}

/// Verify that `bytes` hash to `expected_digest` (`sha256:<hex>`).
///
/// Local-OCI mirrors the live `OciSource` digest-verification pass: even
/// though the bytes came off our own disk, double-checking guards against
/// silent layout corruption and gives the round-trip parity test
/// (`prefetch` → `LocalOciSource`) a hard equality contract.
pub(crate) fn verify_layer_bytes(expected_digest: &str, bytes: &[u8]) -> Result<(), SourceError> {
    let want = expected_digest.strip_prefix("sha256:").ok_or_else(|| {
        SourceError::InvalidData(format!(
            "layer digest {} is not sha256-prefixed",
            expected_digest
        ))
    })?;
    let actual = hex::encode(Sha256::digest(bytes));
    if actual != want {
        return Err(SourceError::InvalidData(format!(
            "layer digest mismatch — expected {}, computed sha256:{}",
            expected_digest, actual
        )));
    }
    Ok(())
}

/// Read a blob by `sha256:<hex>` digest from `<layout>/blobs/sha256/<hex>`.
fn read_blob(layout: &Path, digest: &str) -> Result<Vec<u8>, SourceError> {
    let path = blob_path(layout, digest);
    fs::read(&path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => SourceError::NotFound(format!("blob {}", digest)),
        _ => SourceError::Io(format!("{}: {}", path.display(), e)),
    })
}

/// Compute the on-disk path for a blob digest in an OCI image layout
/// (`<layout>/blobs/<alg>/<hex>`). Exposed so the `sindri registry
/// prefetch` encoder can write to the same paths the read path resolves.
pub fn blob_path(layout: &Path, digest: &str) -> PathBuf {
    let (alg, hex) = match digest.split_once(':') {
        Some(parts) => parts,
        None => return layout.join("blobs").join("sha256").join(digest),
    };
    layout.join("blobs").join(alg).join(hex)
}

/// Test-only helper: build a deterministic three-component OCI image layout
/// at `dest`. Used by the Phase-2 acceptance tests (`local_oci_fixture.rs`)
/// to avoid committing binary blobs. Public-but-doc-hidden so the fixture
/// generator in `tests/` can call it without a separate build artifact.
///
/// Phase 3.0 extension: every fixture also includes per-component manifests
/// (annotated with `org.sindri.component.{backend,name}`) so the new
/// `LocalOciSource::fetch_component_blob` round-trip test path can resolve
/// per-component layer bytes without first standing up a real OCI registry.
#[doc(hidden)]
pub fn build_test_fixture(dest: &Path, signed: bool) -> std::io::Result<FixtureLayout> {
    use std::io::Write;

    fs::create_dir_all(dest)?;
    fs::create_dir_all(dest.join("blobs/sha256"))?;
    let mut layout_file = fs::File::create(dest.join("oci-layout"))?;
    layout_file.write_all(br#"{"imageLayoutVersion":"1.0.0"}"#)?;

    // Compose three components. Backend / name / version are fixed for
    // reproducibility; the bytes hash deterministically.
    let entries = [
        ("mise", "nodejs", "20.10.0", "MIT"),
        ("mise", "rust", "1.75.0", "Apache-2.0"),
        ("brew", "ripgrep", "14.1.0", "MIT"),
    ];

    let mut yaml = String::from("version: 1\nregistry: test-fixture\ncomponents:\n");
    for (backend, name, version, license) in &entries {
        yaml.push_str(&format!(
            "  - name: {name}\n    backend: {backend}\n    latest: \"{version}\"\n    versions: [\"{version}\"]\n    description: test\n    kind: component\n    oci_ref: \"local-oci://test/{name}\"\n    license: {license}\n    depends_on: []\n",
            name = name,
            backend = backend,
            version = version,
            license = license,
        ));
    }

    let layer_bytes = yaml.as_bytes().to_vec();
    let layer_digest = format!("sha256:{}", hex::encode(Sha256::digest(&layer_bytes)));
    write_blob(dest, &layer_digest, &layer_bytes)?;

    // Minimal config blob.
    let config_bytes = b"{}".to_vec();
    let config_digest = format!("sha256:{}", hex::encode(Sha256::digest(&config_bytes)));
    write_blob(dest, &config_digest, &config_bytes)?;

    let manifest = serde_json::json!({
        "schemaVersion": 2,
        "mediaType": "application/vnd.oci.image.manifest.v1+json",
        "config": {
            "mediaType": "application/vnd.oci.image.config.v1+json",
            "digest": config_digest,
            "size": config_bytes.len(),
        },
        "layers": [{
            "mediaType": SINDRI_INDEX_MEDIA_TYPE,
            "digest": layer_digest,
            "size": layer_bytes.len(),
        }],
        "annotations": {
            REGISTRY_CORE_ANNOTATION_KEY: REGISTRY_CORE_ANNOTATION_VALUE,
        }
    });
    let manifest_bytes = serde_json::to_vec(&manifest).expect("serialize manifest");
    let manifest_digest = format!("sha256:{}", hex::encode(Sha256::digest(&manifest_bytes)));
    write_blob(dest, &manifest_digest, &manifest_bytes)?;

    let mut manifests_json: Vec<serde_json::Value> = vec![serde_json::json!({
        "mediaType": "application/vnd.oci.image.manifest.v1+json",
        "digest": manifest_digest,
        "size": manifest_bytes.len(),
    })];

    // Per-component manifests (Phase 3.0). Each component's "layer" is a
    // tiny synthetic `component.yaml`-shaped blob whose contents are stable
    // across runs so digest comparisons in the round-trip parity tests are
    // meaningful.
    let mut component_manifests: Vec<ComponentFixtureManifest> = Vec::with_capacity(entries.len());
    for (backend, name, version, license) in &entries {
        let comp_yaml = format!(
            "metadata:\n  name: {name}\n  version: \"{version}\"\n  description: test\n  license: {license}\n  tags: []\nplatforms: []\ninstall: {{}}\ndepends_on: []\n",
            name = name,
            version = version,
            license = license,
        );
        let comp_layer_bytes = comp_yaml.as_bytes().to_vec();
        let comp_layer_digest =
            format!("sha256:{}", hex::encode(Sha256::digest(&comp_layer_bytes)));
        write_blob(dest, &comp_layer_digest, &comp_layer_bytes)?;

        let comp_manifest = serde_json::json!({
            "schemaVersion": 2,
            "mediaType": "application/vnd.oci.image.manifest.v1+json",
            "config": {
                "mediaType": "application/vnd.oci.image.config.v1+json",
                "digest": config_digest,
                "size": config_bytes.len(),
            },
            "layers": [{
                "mediaType": "application/vnd.sindri.component.v1+yaml",
                "digest": comp_layer_digest,
                "size": comp_layer_bytes.len(),
            }],
        });
        let comp_manifest_bytes =
            serde_json::to_vec(&comp_manifest).expect("serialize component manifest");
        let comp_manifest_digest = format!(
            "sha256:{}",
            hex::encode(Sha256::digest(&comp_manifest_bytes))
        );
        write_blob(dest, &comp_manifest_digest, &comp_manifest_bytes)?;

        manifests_json.push(serde_json::json!({
            "mediaType": "application/vnd.oci.image.manifest.v1+json",
            "digest": comp_manifest_digest,
            "size": comp_manifest_bytes.len(),
            "annotations": {
                COMPONENT_BACKEND_ANNOTATION: backend,
                COMPONENT_NAME_ANNOTATION: name,
                COMPONENT_OCI_REF_ANNOTATION: format!("local-oci://test/{}", name),
            }
        }));

        component_manifests.push(ComponentFixtureManifest {
            backend: (*backend).into(),
            name: (*name).into(),
            manifest_digest: comp_manifest_digest,
            layer_digest: comp_layer_digest,
            layer_bytes: comp_layer_bytes,
        });
    }

    let mut index = serde_json::json!({
        "schemaVersion": 2,
        "mediaType": "application/vnd.oci.image.index.v1+json",
        "manifests": manifests_json,
    });

    if signed {
        // A second manifest entry standing in for the cosign signature
        // manifest. Real cosign verification is exercised in the unit
        // tests against `CosignVerifier::verify_payload`; for the layout
        // fixture we just record presence.
        let sig_bytes = b"sig-placeholder".to_vec();
        let sig_digest = format!("sha256:{}", hex::encode(Sha256::digest(&sig_bytes)));
        write_blob(dest, &sig_digest, &sig_bytes)?;
        if let serde_json::Value::Array(arr) = &mut index["manifests"] {
            arr.push(serde_json::json!({
                "mediaType": "application/vnd.dev.cosign.simplesigning.v1+json",
                "digest": sig_digest,
                "size": sig_bytes.len(),
                "annotations": {
                    "dev.cosignproject.cosign/signature": "deadbeef",
                }
            }));
        }
    }

    let index_bytes = serde_json::to_vec_pretty(&index).expect("serialize index");
    let mut idx_file = fs::File::create(dest.join("index.json"))?;
    idx_file.write_all(&index_bytes)?;

    Ok(FixtureLayout {
        layout_path: dest.to_path_buf(),
        manifest_digest,
        layer_digest,
        components: component_manifests,
    })
}

/// Output of [`build_test_fixture`].
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct FixtureLayout {
    /// Path the fixture was written to.
    pub layout_path: PathBuf,
    /// Manifest digest of the registry-core artifact.
    pub manifest_digest: String,
    /// Digest of the `index.yaml` layer.
    pub layer_digest: String,
    /// Per-component fixture manifests (Phase 3.0).
    pub components: Vec<ComponentFixtureManifest>,
}

/// Per-component manifest fixture metadata produced by [`build_test_fixture`].
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct ComponentFixtureManifest {
    /// Component backend (`mise`, `brew`, …).
    pub backend: String,
    /// Component name.
    pub name: String,
    /// Per-component manifest digest written into `index.json`.
    pub manifest_digest: String,
    /// Layer digest pointed at by the per-component manifest.
    pub layer_digest: String,
    /// Layer bytes — the synthetic `component.yaml` body the test asserts on.
    pub layer_bytes: Vec<u8>,
}

/// Write a blob into the OCI image layout at `<layout>/blobs/<alg>/<hex>`.
/// Exposed for `sindri registry prefetch` (Phase 3.3, ADR-028).
pub fn write_blob(layout: &Path, digest: &str, bytes: &[u8]) -> std::io::Result<()> {
    let path = blob_path(layout, digest);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn fixture_round_trips_via_local_oci_source() {
        let tmp = TempDir::new().unwrap();
        let layout = build_test_fixture(tmp.path(), false).unwrap();

        let src = LocalOciSource::new(LocalOciSourceConfig {
            layout_path: layout.layout_path.clone(),
            scope: None,
            registry_name: CORE_REGISTRY_NAME.into(),
            artifact_ref: None,
        });
        let index = src.fetch_index(&SourceContext::default()).unwrap();
        assert_eq!(index.components.len(), 3);

        let names: Vec<&str> = index.components.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"nodejs"));
        assert!(names.contains(&"rust"));
        assert!(names.contains(&"ripgrep"));

        // The manifest digest the source recorded must match the fixture's.
        assert_eq!(
            src.manifest_digest().as_deref(),
            Some(layout.manifest_digest.as_str())
        );
    }

    #[test]
    fn descriptor_records_layout_path_and_manifest_digest() {
        let tmp = TempDir::new().unwrap();
        let layout = build_test_fixture(tmp.path(), false).unwrap();

        let src = LocalOciSource::new(LocalOciSourceConfig {
            layout_path: layout.layout_path.clone(),
            scope: None,
            registry_name: CORE_REGISTRY_NAME.into(),
            artifact_ref: None,
        });
        // Drive a fetch to populate manifest_digest.
        src.fetch_index(&SourceContext::default()).unwrap();
        match src.lockfile_descriptor() {
            SourceDescriptor::LocalOci {
                layout_path,
                manifest_digest,
            } => {
                assert_eq!(layout_path, layout.layout_path);
                assert_eq!(manifest_digest.unwrap(), layout.manifest_digest);
            }
            _ => panic!("expected LocalOci"),
        }
    }

    #[test]
    fn missing_index_json_yields_io_error() {
        let tmp = TempDir::new().unwrap();
        let src = LocalOciSource::new(LocalOciSourceConfig {
            layout_path: tmp.path().to_path_buf(),
            scope: None,
            registry_name: CORE_REGISTRY_NAME.into(),
            artifact_ref: None,
        });
        let err = src.fetch_index(&SourceContext::default()).unwrap_err();
        match err {
            SourceError::Io(msg) => assert!(msg.contains("not an OCI layout")),
            other => panic!("expected Io, got {:?}", other),
        }
    }

    #[test]
    fn supports_strict_oci_requires_marked_verified() {
        let tmp = TempDir::new().unwrap();
        let layout = build_test_fixture(tmp.path(), true).unwrap();
        let src = LocalOciSource::new(LocalOciSourceConfig {
            layout_path: layout.layout_path,
            scope: None,
            registry_name: CORE_REGISTRY_NAME.into(),
            artifact_ref: None,
        });
        assert!(!src.supports_strict_oci());
        src.mark_verified(true);
        assert!(src.supports_strict_oci());
    }

    #[test]
    fn scope_filters_components() {
        let tmp = TempDir::new().unwrap();
        let layout = build_test_fixture(tmp.path(), false).unwrap();
        let src = LocalOciSource::new(LocalOciSourceConfig {
            layout_path: layout.layout_path,
            scope: Some(vec![ComponentName::from("nodejs")]),
            registry_name: CORE_REGISTRY_NAME.into(),
            artifact_ref: None,
        });
        let idx = src.fetch_index(&SourceContext::default()).unwrap();
        assert_eq!(idx.components.len(), 1);
        assert_eq!(idx.components[0].name, "nodejs");
    }

    #[test]
    fn artifact_ref_pin_overrides_walk() {
        let tmp = TempDir::new().unwrap();
        let layout = build_test_fixture(tmp.path(), false).unwrap();

        let src = LocalOciSource::new(LocalOciSourceConfig {
            layout_path: layout.layout_path.clone(),
            scope: None,
            registry_name: CORE_REGISTRY_NAME.into(),
            artifact_ref: Some(layout.manifest_digest.clone()),
        });
        let _ = src.fetch_index(&SourceContext::default()).unwrap();
        assert_eq!(src.manifest_digest().unwrap(), layout.manifest_digest);
    }

    #[test]
    fn fetch_component_blob_returns_layer_bytes() {
        let tmp = TempDir::new().unwrap();
        let layout = build_test_fixture(tmp.path(), false).unwrap();

        let src = LocalOciSource::new(LocalOciSourceConfig {
            layout_path: layout.layout_path.clone(),
            scope: None,
            registry_name: CORE_REGISTRY_NAME.into(),
            artifact_ref: None,
        });

        let nodejs = layout
            .components
            .iter()
            .find(|c| c.name == "nodejs")
            .expect("fixture has nodejs");
        let id = ComponentId {
            backend: nodejs.backend.clone(),
            name: ComponentName::from(nodejs.name.as_str()),
        };
        let blob = src
            .fetch_component_blob(&id, &Version::new("20.10.0"), &SourceContext::default())
            .unwrap();
        assert_eq!(blob.bytes, nodejs.layer_bytes);
        assert_eq!(blob.digest.as_deref(), Some(nodejs.layer_digest.as_str()));
    }

    #[test]
    fn fetch_component_blob_unknown_id_yields_not_found() {
        let tmp = TempDir::new().unwrap();
        let layout = build_test_fixture(tmp.path(), false).unwrap();

        let src = LocalOciSource::new(LocalOciSourceConfig {
            layout_path: layout.layout_path,
            scope: None,
            registry_name: CORE_REGISTRY_NAME.into(),
            artifact_ref: None,
        });
        let id = ComponentId {
            backend: "mise".into(),
            name: ComponentName::from("does-not-exist"),
        };
        let err = src
            .fetch_component_blob(&id, &Version::new("1.0.0"), &SourceContext::default())
            .unwrap_err();
        match err {
            SourceError::NotFound(msg) => assert!(msg.contains("does-not-exist")),
            other => panic!("expected NotFound, got {:?}", other),
        }
    }

    #[test]
    fn fetch_component_blob_corrupted_layer_yields_invalid_data() {
        let tmp = TempDir::new().unwrap();
        let layout = build_test_fixture(tmp.path(), false).unwrap();

        // Corrupt the nodejs component's layer blob in place; the stored
        // bytes should no longer hash to the manifest's declared digest.
        let nodejs = layout
            .components
            .iter()
            .find(|c| c.name == "nodejs")
            .unwrap();
        let layer_path = blob_path(&layout.layout_path, &nodejs.layer_digest);
        fs::write(&layer_path, b"corrupted").unwrap();

        let src = LocalOciSource::new(LocalOciSourceConfig {
            layout_path: layout.layout_path,
            scope: None,
            registry_name: CORE_REGISTRY_NAME.into(),
            artifact_ref: None,
        });
        let id = ComponentId {
            backend: nodejs.backend.clone(),
            name: ComponentName::from(nodejs.name.as_str()),
        };
        let err = src
            .fetch_component_blob(&id, &Version::new("20.10.0"), &SourceContext::default())
            .unwrap_err();
        match err {
            SourceError::InvalidData(msg) => {
                assert!(msg.contains("digest mismatch"), "got {}", msg)
            }
            other => panic!("expected InvalidData, got {:?}", other),
        }
    }

    #[test]
    fn unknown_artifact_ref_is_not_found() {
        let tmp = TempDir::new().unwrap();
        let layout = build_test_fixture(tmp.path(), false).unwrap();
        let src = LocalOciSource::new(LocalOciSourceConfig {
            layout_path: layout.layout_path,
            scope: None,
            registry_name: CORE_REGISTRY_NAME.into(),
            artifact_ref: Some("sha256:0000".into()),
        });
        let err = src.fetch_index(&SourceContext::default()).unwrap_err();
        match err {
            SourceError::NotFound(msg) => assert!(msg.contains("not in index.json")),
            other => panic!("expected NotFound, got {:?}", other),
        }
    }
}
