//! Registry source domain (DDD-08, ADR-028).
//!
//! A [`RegistrySource`] is one typed origin of registry bytes. A registry
//! aggregate (DDD-02) is composed of an ordered slice of sources plus shared
//! cache and trust state. The resolver consults sources in declared order and
//! takes the first match per component (DDD-03 §"Resolution Algorithm").
//!
//! ## Phase status
//!
//! | Variant      | Status                                            | Phase |
//! | ------------ | ------------------------------------------------- | ----- |
//! | `LocalPath`  | Implemented (real filesystem walk)                | 1     |
//! | `Oci`        | Implemented (real index + layer streaming)        | 2 / 3 |
//! | `LocalOci`   | Implemented (real index + layer streaming)        | 2 / 3 |
//! | `Git`        | Implemented ([`git::GitSourceRuntime`])           | 3     |

pub mod git;
pub mod local_oci;
pub mod local_path;
pub mod oci;

use crate::index::RegistryIndex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sindri_core::version::Version;
use std::path::PathBuf;
use thiserror::Error;

pub use git::GitSourceRuntime;
pub use local_oci::{LocalOciSource, LocalOciSourceConfig};
pub use local_path::LocalPathSource;
pub use oci::{OciSource, OciSourceConfig};

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

// `SourceDescriptor` lives in `sindri-core` so the lockfile types can carry
// it without depending on this crate. Re-export it here so existing
// `sindri_registry::source::SourceDescriptor` paths keep working.
pub use sindri_core::source_descriptor::SourceDescriptor;

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

/// Phase-3 stub — resolves components from a Git repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GitSource {
    /// Repository URL.
    pub url: String,
    /// Branch, tag, or sha. Serialized as `ref:` in `sindri.yaml` (ADR-028
    /// §"Configuration shape"). The resolver pins this to a commit sha in the
    /// lockfile at resolution time (Phase 3).
    #[serde(rename = "ref")]
    pub git_ref: String,
    /// Optional sub-directory inside the repo where `index.yaml` lives.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subdir: Option<PathBuf>,
    /// Optional component-name allow-list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<Vec<ComponentName>>,
    /// When `true`, unsigned commits are rejected. Serialized as
    /// `require-signed:` in `sindri.yaml` (ADR-028 §"Configuration shape").
    #[serde(
        default,
        rename = "require-signed",
        skip_serializing_if = "std::ops::Not::not"
    )]
    pub require_signed: bool,
}

/// Aggregate enum that lets the resolver iterate sources without importing
/// every variant. New variants beyond the four canonicalized in ADR-028
/// require an ADR.
///
/// The serializable variants carry `*Config` payloads (plain data) rather
/// than the live `*Source` runtime objects so the enum can be
/// `Serialize/Deserialize/JsonSchema` without dragging in network/cache
/// state. To run a source, materialize it via the variant-specific
/// constructors ([`OciSource::new`] / [`LocalOciSource::new`]) or call
/// the [`RegistrySource::dispatch_*`] helpers which build a one-shot
/// runtime instance under the hood.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum RegistrySource {
    /// Filesystem path source — the canonical inner-loop authoring path.
    LocalPath(LocalPathSource),
    /// Production OCI source (Phase 2).
    Oci(OciSourceConfig),
    /// On-disk OCI image layout — the air-gap path (Phase 2).
    LocalOci(LocalOciSourceConfig),
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

    /// Short discriminator used by `--explain` and the strict-OCI report.
    pub fn kind(&self) -> &'static str {
        match self {
            RegistrySource::LocalPath(_) => "local-path",
            RegistrySource::Oci(_) => "oci",
            RegistrySource::LocalOci(_) => "local-oci",
            RegistrySource::Git(_) => "git",
        }
    }

    /// Dispatch [`Source::fetch_index`] across enum variants. Materializes
    /// the runtime source object on the fly for `Oci` / `LocalOci`.
    pub fn dispatch_fetch_index(&self, ctx: &SourceContext) -> Result<RegistryIndex, SourceError> {
        match self {
            RegistrySource::LocalPath(s) => s.fetch_index(ctx),
            RegistrySource::Oci(cfg) => OciSource::new(cfg.clone())
                .map_err(|e| SourceError::Io(e.to_string()))?
                .fetch_index(ctx),
            RegistrySource::LocalOci(cfg) => LocalOciSource::new(cfg.clone()).fetch_index(ctx),
            RegistrySource::Git(cfg) => GitSourceRuntime::new(cfg.clone()).fetch_index(ctx),
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
            RegistrySource::Oci(cfg) => OciSource::new(cfg.clone())
                .map_err(|e| SourceError::Io(e.to_string()))?
                .fetch_component_blob(id, version, ctx),
            RegistrySource::LocalOci(cfg) => {
                LocalOciSource::new(cfg.clone()).fetch_component_blob(id, version, ctx)
            }
            RegistrySource::Git(cfg) => {
                GitSourceRuntime::new(cfg.clone()).fetch_component_blob(id, version, ctx)
            }
        }
    }

    /// Dispatch [`Source::lockfile_descriptor`] across enum variants.
    ///
    /// For OCI variants the descriptor is built from the static config —
    /// callers that have already fetched and want the manifest_digest
    /// populated should hold an [`OciSource`] / [`LocalOciSource`]
    /// directly and call [`Source::lockfile_descriptor`] on it.
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
                // The static-config dispatch can only echo the user-supplied
                // ref because it has no live runtime state. Callers that
                // need the resolved sha must hold a `GitSourceRuntime`
                // directly and call [`Source::lockfile_descriptor`] on it
                // (the resolver does this after `fetch_index` succeeds).
                commit_sha: s.git_ref.clone(),
                subdir: s.subdir.clone(),
            },
        }
    }

    /// Dispatch [`Source::supports_strict_oci`] across enum variants. Note
    /// the result is conservative for `Oci` / `LocalOci` because we have no
    /// live runtime state on the `RegistrySource` enum — production callers
    /// should drive the live [`OciSource`] / [`LocalOciSource`] objects
    /// instead. The resolver's strict-OCI gate consults this exclusively
    /// after marking each materialized source verified, so the conservative
    /// answer here is "no" until the runtime version says otherwise.
    pub fn dispatch_supports_strict_oci(&self) -> bool {
        match self {
            RegistrySource::LocalPath(s) => s.supports_strict_oci(),
            // The static-config form has no verification state attached.
            // Live verification flips on through the materialized runtime
            // sources; the gate consults those, not the enum.
            RegistrySource::Oci(_) => false,
            RegistrySource::LocalOci(_) => false,
            RegistrySource::Git(_) => false,
        }
    }
}

// =============================================================================
// Config DTO → trait enum conversions (ADR-028 §"Resolver wiring", Phase 4.1)
// =============================================================================

/// Convert a slice of `sindri-core` config DTOs into a `Vec<RegistrySource>`
/// for use by the resolver. Called by `sindri resolve` after reading the BOM.
///
/// Scope lists are converted from `Vec<String>` to `Vec<ComponentName>`.
/// Registry-level fields (`registry_name`, `artifact_ref`) are forwarded to
/// the runtime source configs in `sindri-registry`.
pub fn sources_from_config(
    cfgs: &[sindri_core::manifest::RegistrySourceConfig],
) -> Vec<RegistrySource> {
    cfgs.iter().map(registry_source_from_config).collect()
}

fn registry_source_from_config(
    cfg: &sindri_core::manifest::RegistrySourceConfig,
) -> RegistrySource {
    use sindri_core::manifest::RegistrySourceConfig as C;
    match cfg {
        C::Oci(c) => {
            let rn = c
                .registry_name
                .clone()
                .unwrap_or_else(|| sindri_core::registry::CORE_REGISTRY_NAME.to_string());
            RegistrySource::Oci(OciSourceConfig {
                url: c.url.clone(),
                tag: c.tag.clone(),
                scope: c
                    .scope
                    .as_ref()
                    .map(|v| v.iter().map(|s| ComponentName::from(s.as_str())).collect()),
                registry_name: rn,
            })
        }
        C::LocalPath(c) => RegistrySource::LocalPath(LocalPathSource {
            path: c.path.clone(),
            scope: c
                .scope
                .as_ref()
                .map(|v| v.iter().map(|s| ComponentName::from(s.as_str())).collect()),
        }),
        C::Git(c) => RegistrySource::Git(GitSource {
            url: c.url.clone(),
            git_ref: c.git_ref.clone(),
            subdir: c.subdir.clone(),
            scope: c
                .scope
                .as_ref()
                .map(|v| v.iter().map(|s| ComponentName::from(s.as_str())).collect()),
            require_signed: c.require_signed,
        }),
        C::LocalOci(c) => {
            let rn = c
                .registry_name
                .clone()
                .unwrap_or_else(|| sindri_core::registry::CORE_REGISTRY_NAME.to_string());
            RegistrySource::LocalOci(LocalOciSourceConfig {
                layout_path: c.layout.clone(),
                scope: c
                    .scope
                    .as_ref()
                    .map(|v| v.iter().map(|s| ComponentName::from(s.as_str())).collect()),
                registry_name: rn,
                artifact_ref: c.artifact_ref.clone(),
            })
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
        let d = super::oci_descriptor_from_legacy_ref("ghcr.io/sindri-dev/registry-core:2026.04")
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
    fn enum_oci_descriptor_records_url_and_tag() {
        let s = RegistrySource::Oci(OciSourceConfig {
            url: "oci://example/x".into(),
            tag: "1.0".into(),
            scope: None,
            registry_name: "example".into(),
        });
        match s.dispatch_lockfile_descriptor() {
            SourceDescriptor::Oci { url, tag, .. } => {
                assert_eq!(url, "oci://example/x");
                assert_eq!(tag, "1.0");
            }
            _ => panic!("expected Oci descriptor"),
        }
    }

    #[test]
    fn enum_kinds_are_stable() {
        let lp = RegistrySource::LocalPath(LocalPathSource::new("/x"));
        let o = RegistrySource::Oci(OciSourceConfig {
            url: "oci://x/y".into(),
            tag: "1".into(),
            scope: None,
            registry_name: "x".into(),
        });
        let lo = RegistrySource::LocalOci(LocalOciSourceConfig {
            layout_path: PathBuf::from("/x"),
            scope: None,
            registry_name: "x".into(),
            artifact_ref: None,
        });
        let g = RegistrySource::Git(GitSource {
            url: "https://x".into(),
            git_ref: "main".into(),
            subdir: None,
            scope: None,
            require_signed: false,
        });
        assert_eq!(lp.kind(), "local-path");
        assert_eq!(o.kind(), "oci");
        assert_eq!(lo.kind(), "local-oci");
        assert_eq!(g.kind(), "git");
    }

    #[test]
    fn enum_oci_does_not_claim_strict_without_runtime_verification() {
        let s = RegistrySource::Oci(OciSourceConfig {
            url: "oci://example/x".into(),
            tag: "1.0".into(),
            scope: None,
            registry_name: "example".into(),
        });
        assert!(!s.dispatch_supports_strict_oci());
    }
}
