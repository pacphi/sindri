//! Registry source domain (DDD-08, ADR-028).
//!
//! A [`RegistrySource`] is one typed origin of registry bytes. A registry
//! aggregate (DDD-02) is composed of an ordered slice of sources plus shared
//! cache and trust state. The resolver consults sources in declared order and
//! takes the first match per component (DDD-03 §"Resolution Algorithm").
//!
//! Phase 1 (this module) ships the trait surface, the [`RegistrySource`]
//! enum, the [`SourceContext`] / [`SourceError`] / [`SourceDescriptor`] types,
//! and the [`LocalPathSource`] implementation. The remaining variants
//! ([`OciSource`], [`LocalOciSource`], [`GitSource`]) exist as stubs whose
//! trait methods return [`SourceError::NotImplemented`]; their full
//! implementations land in Phase 2 (OCI / local-oci) and Phase 3 (git) of
//! the source-modes implementation plan.

pub mod local_path;

use crate::index::RegistryIndex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sindri_core::version::Version;
use std::path::PathBuf;
use thiserror::Error;

pub use local_path::LocalPathSource;
// `SourceDescriptor` lives in `sindri-core` so the lockfile types can carry
// it without depending on this crate. Re-export it here so existing
// `sindri_registry::source::SourceDescriptor` paths keep working.
pub use sindri_core::source_descriptor::SourceDescriptor;

/// Newtype wrapper around the canonical component name (the `name` field of
/// a `ComponentEntry`). Kept small and stringy in Phase 1 so we don't have
/// to thread a richer type through every `ComponentEntry` call site; future
/// phases may promote this into `sindri-core` and tighten validation.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, PartialOrd, Ord,
)]
pub struct ComponentName(pub String);

impl ComponentName {
    /// Construct from any string-like value.
    pub fn new<S: Into<String>>(s: S) -> Self {
        Self(s.into())
    }

    /// Inner string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ComponentName {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for ComponentName {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Component identifier accepted by [`Source::fetch_component_blob`].
///
/// Phase 1 exposes only `(backend, name)` — the same coordinate the resolver
/// already uses. Promoted to a richer aggregate when DDD-01 lands its
/// `ComponentId` value type in `sindri-core`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ComponentId {
    /// Backend (`mise`, `brew`, `script`, …).
    pub backend: String,
    /// Component name.
    pub name: ComponentName,
}

/// Ambient context passed to every [`Source`] call.
///
/// Sources should treat this as advisory: caches, network, signing keys live
/// outside the source itself in higher layers (DDD-02 cache model).
#[derive(Debug, Default, Clone)]
pub struct SourceContext {
    /// Whether the resolver was invoked with `--offline`. Sources MUST avoid
    /// network I/O when this is `true`.
    pub offline: bool,
}

/// Errors raised by [`Source`] implementations.
#[derive(Debug, Error)]
pub enum SourceError {
    /// The source produced no entry for this component. The resolver continues
    /// to the next source in declared order.
    #[error("component '{0}' not found in source")]
    NotFound(String),

    /// The source's bytes could not be read (filesystem error, network
    /// failure, malformed layout).
    #[error("source I/O failure: {0}")]
    Io(String),

    /// The source's catalog or component blob failed schema validation.
    #[error("source produced invalid data: {0}")]
    InvalidData(String),

    /// Signature verification was required but failed.
    #[error("source signature verification failed: {0}")]
    SignatureFailed(String),

    /// This source variant is not yet implemented in the current phase.
    /// Carries the variant name so the error is debuggable.
    #[error("source variant '{0}' is not implemented in this build")]
    NotImplemented(&'static str),
}

impl From<std::io::Error> for SourceError {
    fn from(err: std::io::Error) -> Self {
        SourceError::Io(err.to_string())
    }
}

/// A single component blob (the raw `component.yaml` bytes plus identity
/// metadata). Phase 1 only needs the bytes plus an optional digest for
/// future use; the resolver reads the blob via `serde_yaml::from_slice`.
#[derive(Debug, Clone)]
pub struct ComponentBlob {
    /// Raw `component.yaml` bytes.
    pub bytes: Vec<u8>,
    /// Optional content-addressable digest (`sha256:...`), populated by
    /// sources that have one (OCI, local-oci). `LocalPathSource` leaves this
    /// `None` because filesystem bytes are not addressable.
    pub digest: Option<String>,
}

/// Reconstruct an OCI descriptor from a legacy `registry: <ref>` string
/// recorded in pre-Phase-1.3 lockfiles. Returns `None` if the input is
/// empty.
pub fn oci_descriptor_from_legacy_ref(legacy: &str) -> Option<SourceDescriptor> {
    let trimmed = legacy.trim();
    if trimmed.is_empty() {
        return None;
    }
    match crate::oci_ref::OciRef::parse(trimmed) {
        Ok(parsed) => {
            let tag = match &parsed.reference {
                crate::oci_ref::OciReference::Tag(t) => t.clone(),
                crate::oci_ref::OciReference::Digest(d) => d.clone(),
            };
            Some(SourceDescriptor::Oci {
                url: format!("oci://{}/{}", parsed.registry, parsed.repository),
                tag,
                manifest_digest: None,
            })
        }
        Err(_) => Some(SourceDescriptor::Oci {
            url: trimmed.to_string(),
            tag: String::new(),
            manifest_digest: None,
        }),
    }
}

/// Domain-service contract every source implementation satisfies (DDD-08
/// §"Source trait").
pub trait Source {
    /// Produce the catalog this source contributes. May be partial when
    /// `scope` is set; the resolver merges partial catalogs in source order.
    fn fetch_index(&self, ctx: &SourceContext) -> Result<RegistryIndex, SourceError>;

    /// Produce a single `component.yaml` blob by id and version.
    fn fetch_component_blob(
        &self,
        id: &ComponentId,
        version: &Version,
        ctx: &SourceContext,
    ) -> Result<ComponentBlob, SourceError>;

    /// Identity recorded in the lockfile so apply-time fetch is reproducible.
    fn lockfile_descriptor(&self) -> SourceDescriptor;

    /// Whether this source's bytes carry a verified signature chain that
    /// satisfies `--strict-oci`. Only `OciSource` and (transitively)
    /// `LocalOciSource` may return `true`.
    fn supports_strict_oci(&self) -> bool;
}

/// Phase-2 stub — the production OCI client wrapper. Carries the descriptor
/// shape from DDD-08 so the enum variant is real today, but every trait method
/// returns [`SourceError::NotImplemented`] until Phase 2 wires it to
/// [`crate::client::RegistryClient`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct OciSource {
    /// Canonical `oci://host/path` URL.
    pub url: String,
    /// Tag (e.g. `2026.05`).
    pub tag: String,
    /// Optional component-name allow-list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<Vec<ComponentName>>,
}

/// Phase-2 stub — reads OCI image layouts on disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct LocalOciSource {
    /// OCI image-layout directory (v1.1).
    pub layout_path: PathBuf,
    /// Optional component-name allow-list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<Vec<ComponentName>>,
}

/// Phase-3 stub — resolves components from a Git repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GitSource {
    /// Repository URL.
    pub url: String,
    /// Branch, tag, or sha.
    pub git_ref: String,
    /// Optional sub-directory inside the repo where `index.yaml` lives.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subdir: Option<PathBuf>,
    /// Optional component-name allow-list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<Vec<ComponentName>>,
    /// When `true`, unsigned commits are rejected.
    #[serde(default)]
    pub require_signed: bool,
}

/// Aggregate enum that lets the resolver iterate sources without importing
/// every variant. New variants beyond the four canonicalized in ADR-028
/// require an ADR.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum RegistrySource {
    /// Filesystem path source — the canonical inner-loop authoring path.
    LocalPath(LocalPathSource),
    /// Production OCI source (Phase 2).
    Oci(OciSource),
    /// On-disk OCI image layout — the air-gap path (Phase 2).
    LocalOci(LocalOciSource),
    /// Git source (Phase 3).
    Git(GitSource),
}

impl RegistrySource {
    /// Borrow the optional scope filter for this source.
    pub fn scope(&self) -> Option<&[ComponentName]> {
        match self {
            RegistrySource::LocalPath(s) => s.scope.as_deref(),
            RegistrySource::Oci(s) => s.scope.as_deref(),
            RegistrySource::LocalOci(s) => s.scope.as_deref(),
            RegistrySource::Git(s) => s.scope.as_deref(),
        }
    }

    /// `true` when the component name passes this source's scope filter.
    /// A source with no `scope:` matches everything (DDD-08 §Source scope).
    pub fn scope_matches(&self, name: &ComponentName) -> bool {
        match self.scope() {
            None => true,
            Some(allow) => allow.iter().any(|n| n == name),
        }
    }

    /// Dispatch [`Source::fetch_index`] across enum variants.
    pub fn dispatch_fetch_index(&self, ctx: &SourceContext) -> Result<RegistryIndex, SourceError> {
        match self {
            RegistrySource::LocalPath(s) => s.fetch_index(ctx),
            RegistrySource::Oci(_) => Err(SourceError::NotImplemented("oci")),
            RegistrySource::LocalOci(_) => Err(SourceError::NotImplemented("local-oci")),
            RegistrySource::Git(_) => Err(SourceError::NotImplemented("git")),
        }
    }

    /// Dispatch [`Source::fetch_component_blob`] across enum variants.
    pub fn dispatch_fetch_component_blob(
        &self,
        id: &ComponentId,
        version: &Version,
        ctx: &SourceContext,
    ) -> Result<ComponentBlob, SourceError> {
        match self {
            RegistrySource::LocalPath(s) => s.fetch_component_blob(id, version, ctx),
            RegistrySource::Oci(_) => Err(SourceError::NotImplemented("oci")),
            RegistrySource::LocalOci(_) => Err(SourceError::NotImplemented("local-oci")),
            RegistrySource::Git(_) => Err(SourceError::NotImplemented("git")),
        }
    }

    /// Dispatch [`Source::lockfile_descriptor`] across enum variants. The
    /// stub variants synthesize a best-effort descriptor from their config so
    /// the lockfile shape is testable in Phase 1; later phases replace this
    /// with the real signed-fetch result.
    pub fn dispatch_lockfile_descriptor(&self) -> SourceDescriptor {
        match self {
            RegistrySource::LocalPath(s) => s.lockfile_descriptor(),
            RegistrySource::Oci(s) => SourceDescriptor::Oci {
                url: s.url.clone(),
                tag: s.tag.clone(),
                manifest_digest: None,
            },
            RegistrySource::LocalOci(s) => SourceDescriptor::LocalOci {
                layout_path: s.layout_path.clone(),
                manifest_digest: None,
            },
            RegistrySource::Git(s) => SourceDescriptor::Git {
                url: s.url.clone(),
                // The unresolved ref is recorded as a placeholder until
                // Phase 3 resolves it to a sha. Apply-time refetch will
                // reject this until the lockfile is regenerated.
                commit_sha: s.git_ref.clone(),
                subdir: s.subdir.clone(),
            },
        }
    }

    /// Dispatch [`Source::supports_strict_oci`] across enum variants.
    pub fn dispatch_supports_strict_oci(&self) -> bool {
        match self {
            RegistrySource::LocalPath(s) => s.supports_strict_oci(),
            // The OCI / LocalOCI variants only flip to `true` once Phase 2
            // wires real signature verification. Today they're stubs and
            // therefore non-strict.
            RegistrySource::Oci(_) => false,
            RegistrySource::LocalOci(_) => false,
            RegistrySource::Git(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local(path: &str, scope: Option<Vec<&str>>) -> RegistrySource {
        RegistrySource::LocalPath(LocalPathSource {
            path: PathBuf::from(path),
            scope: scope.map(|v| v.into_iter().map(ComponentName::from).collect()),
        })
    }

    #[test]
    fn scope_matches_when_no_scope_set() {
        let s = local("/tmp/reg", None);
        assert!(s.scope_matches(&ComponentName::from("anything")));
    }

    #[test]
    fn scope_matches_when_name_in_allowlist() {
        let s = local("/tmp/reg", Some(vec!["nodejs", "rust"]));
        assert!(s.scope_matches(&ComponentName::from("nodejs")));
    }

    #[test]
    fn scope_skips_when_name_not_in_allowlist() {
        let s = local("/tmp/reg", Some(vec!["nodejs"]));
        assert!(!s.scope_matches(&ComponentName::from("rust")));
    }

    #[test]
    fn descriptor_kind_strings() {
        let d = SourceDescriptor::LocalPath {
            path: PathBuf::from("/x"),
        };
        assert_eq!(d.kind(), "local-path");
    }

    #[test]
    fn legacy_ref_reconstructs_oci_descriptor() {
        let d = oci_descriptor_from_legacy_ref("ghcr.io/sindri-dev/registry-core:2026.04")
            .expect("valid ref");
        match d {
            SourceDescriptor::Oci { url, tag, .. } => {
                assert!(url.starts_with("oci://"));
                assert_eq!(tag, "2026.04");
            }
            _ => panic!("expected Oci"),
        }
    }

    #[test]
    fn stub_variants_return_not_implemented_for_fetch() {
        let oci = RegistrySource::Oci(OciSource {
            url: "oci://example/x".into(),
            tag: "1.0".into(),
            scope: None,
        });
        let ctx = SourceContext::default();
        match oci.dispatch_fetch_index(&ctx) {
            Err(SourceError::NotImplemented(v)) => assert_eq!(v, "oci"),
            other => panic!("expected NotImplemented, got {other:?}"),
        }
    }

    #[test]
    fn stub_variants_are_not_strict_oci() {
        let oci = RegistrySource::Oci(OciSource {
            url: "oci://example/x".into(),
            tag: "1.0".into(),
            scope: None,
        });
        assert!(!oci.dispatch_supports_strict_oci());
    }
}
