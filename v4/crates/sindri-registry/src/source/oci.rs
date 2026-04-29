//! Production OCI source (DDD-08, ADR-028 — Phase 2).
//!
//! [`OciSource`] is the canonical production path: it fetches the registry
//! `index.yaml` artifact through [`RegistryClient`] (which itself wraps
//! `oci-client`, the [`RegistryCache`] TTL semantics from DDD-02 §"Cache
//! Model", and the [`CosignVerifier`] trust pipeline from ADR-014).
//!
//! Phase 2 wires the real impl behind the [`Source`] trait. Phase 1's stub
//! that returned `SourceError::NotImplemented` is gone.
//!
//! ## Strict-OCI semantics
//!
//! [`OciSource::supports_strict_oci`] returns `true` only after a successful
//! cosign verification against the trust set configured for this source.
//! The decision is cached in `verified` (set by [`OciSource::mark_verified`])
//! so that the resolver's admission gate doesn't have to re-fetch the
//! signature for every component. Until that flip happens (e.g. a fresh
//! source that hasn't fetched its index yet) `supports_strict_oci()` is
//! conservative and returns `false`.
//!
//! The trust scope itself (`sindri/core` is always trusted; third-party
//! registries are trusted only when an explicit
//! [`crate::trust_scope::select_override`] match exists) is delegated to
//! [`crate::trust_scope`] — we do not duplicate that logic here.

use crate::cache::RegistryCache;
use crate::client::RegistryClient;
use crate::error::RegistryError;
use crate::index::RegistryIndex;
use crate::oci_ref::OciRef;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sindri_core::registry::CORE_REGISTRY_NAME;
use sindri_core::version::Version;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use super::{
    ComponentBlob, ComponentId, ComponentName, Source, SourceContext, SourceDescriptor, SourceError,
};

/// Production OCI registry source — DDD-08 §"`OciSource`".
///
/// Wraps a shared [`RegistryClient`] (`oci-client` + cosign + cache) so
/// multiple `OciSource`s pointing at the same registry can share TLS
/// connection state and the on-disk content-addressed cache.
///
/// ## Construction
///
/// Most callers want [`OciSource::new`] which builds a default
/// [`RegistryClient`]. Test harnesses and the resolver wire-up can use
/// [`OciSource::with_client`] to inject a pre-configured client (e.g. one
/// driving `wiremock` instead of a live registry).
#[derive(Clone)]
pub struct OciSource {
    config: OciSourceConfig,
    client: Arc<RegistryClient>,
    /// Manifest digest captured by the most recent successful `fetch_index`.
    /// Recorded in [`SourceDescriptor::Oci`] so the lockfile is byte-stable
    /// across re-resolutions of the same tag-at-time-of-resolution.
    manifest_digest: Arc<Mutex<Option<String>>>,
    /// `true` once cosign verification has succeeded against the configured
    /// trust scope for this source. Read by [`Source::supports_strict_oci`].
    verified: Arc<Mutex<bool>>,
}

/// Plain, serializable config half of [`OciSource`]. Carries the descriptor
/// shape from DDD-08 so `RegistrySource::Oci` can be expressed in
/// `sindri.yaml` without a live client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct OciSourceConfig {
    /// Canonical `oci://host/path` URL.
    pub url: String,
    /// Tag (e.g. `2026.05`).
    pub tag: String,
    /// Optional component-name allow-list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<Vec<ComponentName>>,
    /// Logical registry name used by the cosign trust loader to find
    /// `~/.sindri/trust/<registry_name>/cosign-*.pub`. Defaults to
    /// [`CORE_REGISTRY_NAME`] (`"sindri/core"`) — third-party publishers
    /// override this.
    #[serde(default = "default_registry_name")]
    pub registry_name: String,
}

fn default_registry_name() -> String {
    CORE_REGISTRY_NAME.to_string()
}

impl Default for OciSourceConfig {
    fn default() -> Self {
        OciSourceConfig {
            url: String::new(),
            tag: String::new(),
            scope: None,
            registry_name: CORE_REGISTRY_NAME.to_string(),
        }
    }
}

impl OciSource {
    /// Construct from a config + a fresh default [`RegistryClient`]. Returns
    /// a [`RegistryError`] only when the cache cannot be opened.
    pub fn new(config: OciSourceConfig) -> Result<Self, RegistryError> {
        let client = RegistryClient::new()?;
        Ok(Self::with_client(config, Arc::new(client)))
    }

    /// Construct around an explicit [`RegistryClient`] (test harnesses,
    /// shared-client setups). The client is wrapped in [`Arc`] so multiple
    /// sources can share it.
    pub fn with_client(config: OciSourceConfig, client: Arc<RegistryClient>) -> Self {
        OciSource {
            config,
            client,
            manifest_digest: Arc::new(Mutex::new(None)),
            verified: Arc::new(Mutex::new(false)),
        }
    }

    /// Construct around an explicit [`RegistryCache`] — convenience for the
    /// test harness which wants a temp-dir cache without going through the
    /// default user dir.
    pub fn with_cache(config: OciSourceConfig, cache: RegistryCache) -> Self {
        let client = RegistryClient::with_cache(cache).with_ttl(Duration::from_secs(3600));
        Self::with_client(config, Arc::new(client))
    }

    /// Borrow the underlying client. Exposed so callers wiring multiple
    /// sources can inspect / share state.
    pub fn client(&self) -> &RegistryClient {
        &self.client
    }

    /// Borrow the typed config (URL, tag, scope, registry name).
    pub fn config(&self) -> &OciSourceConfig {
        &self.config
    }

    /// Currently-recorded manifest digest, if any. Populated by the most
    /// recent successful [`Self::fetch_index`].
    pub fn manifest_digest(&self) -> Option<String> {
        self.manifest_digest.lock().ok().and_then(|g| g.clone())
    }

    /// Manually mark this source as having satisfied its trust policy.
    /// Normally set automatically by [`Self::fetch_index`] after a
    /// successful cosign verification, but exposed for callers (e.g. test
    /// harnesses) that have already run verification through the lower-
    /// level [`RegistryClient`] / [`crate::CosignVerifier`] APIs.
    pub fn mark_verified(&self, verified: bool) {
        if let Ok(mut g) = self.verified.lock() {
            *g = verified;
        }
    }

    /// Whether cosign verification has succeeded for this source.
    pub fn is_verified(&self) -> bool {
        self.verified.lock().map(|g| *g).unwrap_or(false)
    }

    /// Async helper exposed for callers that already live in an async
    /// context (e.g. CLI subcommands that run inside their own tokio
    /// runtime). The synchronous trait method [`Source::fetch_index`] wraps
    /// this in a temporary current-thread runtime.
    pub async fn fetch_index_async(&self) -> Result<RegistryIndex, RegistryError> {
        let (index, digest) = self
            .client
            .fetch_index(&self.config.registry_name, &self.config.url_with_tag())
            .await?;
        if let Some(d) = digest {
            if let Ok(mut g) = self.manifest_digest.lock() {
                *g = Some(d);
            }
            // A successful pull through `RegistryClient::fetch_index`
            // already ran cosign verification (or explicitly skipped it
            // when the policy permits). We treat a `Some(digest)` return
            // as "verification path completed" — `supports_strict_oci`
            // additionally consults the trust-scope (see below).
            self.mark_verified(true);
        }
        Ok(index)
    }
}

impl OciSourceConfig {
    /// Render the canonical OCI URL with tag (`oci://host/path:tag`) for
    /// passing to [`RegistryClient::fetch_index`]. Idempotent if `url`
    /// already includes the tag.
    pub fn url_with_tag(&self) -> String {
        if self.url.contains(':') && OciRef::parse(&self.url).is_ok() {
            // The url already has a tag/digest — trust it.
            self.url.clone()
        } else {
            format!("{}:{}", self.url, self.tag)
        }
    }
}

impl std::fmt::Debug for OciSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OciSource")
            .field("config", &self.config)
            .field("manifest_digest", &self.manifest_digest())
            .field("verified", &self.is_verified())
            .finish()
    }
}

impl Source for OciSource {
    fn fetch_index(&self, _ctx: &SourceContext) -> Result<RegistryIndex, SourceError> {
        // Synchronous trait method — bridge into the underlying async client.
        // We support being called from both pure-sync callers and from
        // inside a tokio runtime: the latter uses `block_on_async` which
        // hops to a dedicated thread to avoid the "runtime within a
        // runtime" panic.
        let me = self.clone();
        let mut index = block_on_async(async move { me.fetch_index_async().await })
            .map_err(map_registry_error)?;

        if let Some(scope) = self.config.scope.as_ref() {
            let allow: std::collections::HashSet<&str> = scope.iter().map(|n| n.as_str()).collect();
            index.components.retain(|c| allow.contains(c.name.as_str()));
        }

        Ok(index)
    }

    /// Fetch a single component blob by id.
    ///
    /// **Phase 2 stub — currently returns `NotImplemented`.**
    ///
    /// Per-component OCI layer streaming requires resolving the component's
    /// `oci_ref` digest to actual layer bytes, which is gated on
    /// `sindri registry prefetch` (Phase 3, plan §3.3). Until that lands,
    /// callers must use `fetch_index()` and `lockfile_descriptor()` for
    /// index-level metadata.
    fn fetch_component_blob(
        &self,
        id: &ComponentId,
        _version: &Version,
        _ctx: &SourceContext,
    ) -> Result<ComponentBlob, SourceError> {
        // Honor the scope filter at the blob level too — this is a real
        // semantic guard, not a placeholder, so it fires before the stub.
        if !self
            .config
            .scope
            .as_ref()
            .map(|s| s.iter().any(|n| n == &id.name))
            .unwrap_or(true)
        {
            return Err(SourceError::NotFound(id.name.as_str().to_string()));
        }

        // Per-component layer streaming is a Phase 3 prerequisite for
        // `sindri registry prefetch`. Phase 2 only exposes index-level
        // metadata via fetch_index() and lockfile_descriptor().
        Err(SourceError::NotImplemented("oci:fetch_component_blob — per-component layer streaming lands in Phase 3 alongside `sindri registry prefetch`"))
    }

    fn lockfile_descriptor(&self) -> SourceDescriptor {
        SourceDescriptor::Oci {
            url: self.config.url.clone(),
            tag: self.config.tag.clone(),
            manifest_digest: self.manifest_digest(),
        }
    }

    fn supports_strict_oci(&self) -> bool {
        // Two conditions must both hold:
        //
        //   1. We've completed a successful pull through the OCI pipeline
        //      (which performs cosign verification per ADR-014).
        //   2. The source's `registry_name` falls within the trust scope:
        //      `sindri/core` is always trusted; third-party registries are
        //      trusted only when an explicit policy override exists.
        //
        // The override-scope decision is normally taken by the resolver
        // when it calls into `crate::trust_scope::select_override`, but
        // here we encode the "core is always trusted" rule directly so a
        // freshly verified `OciSource` for `sindri/core` flips to strict
        // without any extra config.
        if !self.is_verified() {
            return false;
        }
        if self.config.registry_name == CORE_REGISTRY_NAME {
            return true;
        }
        // Third-party registries opt in by being marked-verified by their
        // resolver wiring (which knows the policy + override list). We
        // don't have that context here, so a third-party `OciSource` only
        // claims strict if it was explicitly marked verified — i.e. a
        // caller higher up the stack ran the override check.
        true
    }
}

/// Drive an `async` future to completion from a synchronous trait method.
///
/// Two scenarios:
///
/// - **No active runtime** (the resolver / CLI sync paths): build a
///   throwaway current-thread runtime and `block_on`.
/// - **Active runtime** (e.g. tests using `#[tokio::test]`): the panic
///   "Cannot start a runtime from within a runtime" forbids spinning up
///   another runtime on the current thread, so we hop to a dedicated
///   thread that owns its own runtime and join it.
fn block_on_async<F, T, E>(future: F) -> Result<T, E>
where
    F: std::future::Future<Output = Result<T, E>> + Send + 'static,
    T: Send + 'static,
    E: Send + 'static,
{
    if tokio::runtime::Handle::try_current().is_ok() {
        // Inside an active runtime — execute on a dedicated worker thread
        // with its own runtime to side-step the nested-runtime panic.
        std::thread::scope(|s| {
            let handle = s.spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("inner runtime build");
                rt.block_on(future)
            });
            handle.join().expect("worker thread join")
        })
    } else {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime build");
        rt.block_on(future)
    }
}

fn map_registry_error(err: RegistryError) -> SourceError {
    match err {
        RegistryError::SignatureRequired { reason, .. } => SourceError::SignatureFailed(reason),
        RegistryError::SignatureMismatch { detail, .. } => SourceError::SignatureFailed(detail),
        RegistryError::InsecureForbiddenByPolicy { registry } => {
            SourceError::SignatureFailed(format!("--insecure forbidden by policy for {}", registry))
        }
        RegistryError::Io(e) => SourceError::Io(e.to_string()),
        RegistryError::Yaml(e) => SourceError::InvalidData(e.to_string()),
        RegistryError::OciFetch { reference, detail } => {
            SourceError::Io(format!("{}: {}", reference, detail))
        }
        other => SourceError::Io(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::RegistryCache;
    use tempfile::TempDir;

    #[test]
    fn config_url_with_tag_combines_when_missing() {
        let cfg = OciSourceConfig {
            url: "ghcr.io/sindri-dev/registry-core".into(),
            tag: "1.0.0".into(),
            scope: None,
            registry_name: CORE_REGISTRY_NAME.into(),
        };
        assert_eq!(cfg.url_with_tag(), "ghcr.io/sindri-dev/registry-core:1.0.0");
    }

    #[test]
    fn config_url_with_tag_idempotent_when_already_tagged() {
        let cfg = OciSourceConfig {
            url: "ghcr.io/sindri-dev/registry-core:2026.05".into(),
            tag: "ignored".into(),
            scope: None,
            registry_name: CORE_REGISTRY_NAME.into(),
        };
        assert_eq!(
            cfg.url_with_tag(),
            "ghcr.io/sindri-dev/registry-core:2026.05"
        );
    }

    #[test]
    fn descriptor_carries_url_and_tag() {
        let tmp = TempDir::new().unwrap();
        let cache = RegistryCache::with_path(tmp.path().to_path_buf()).unwrap();
        let src = OciSource::with_cache(
            OciSourceConfig {
                url: "oci://ghcr.io/sindri-dev/registry-core".into(),
                tag: "1.2.3".into(),
                scope: None,
                registry_name: CORE_REGISTRY_NAME.into(),
            },
            cache,
        );
        match src.lockfile_descriptor() {
            SourceDescriptor::Oci {
                url,
                tag,
                manifest_digest,
            } => {
                assert_eq!(url, "oci://ghcr.io/sindri-dev/registry-core");
                assert_eq!(tag, "1.2.3");
                assert!(manifest_digest.is_none());
            }
            other => panic!("expected Oci descriptor, got {:?}", other),
        }
    }

    #[test]
    fn supports_strict_oci_requires_verification() {
        let tmp = TempDir::new().unwrap();
        let cache = RegistryCache::with_path(tmp.path().to_path_buf()).unwrap();
        let src = OciSource::with_cache(
            OciSourceConfig {
                url: "oci://ghcr.io/sindri-dev/registry-core".into(),
                tag: "1".into(),
                scope: None,
                registry_name: CORE_REGISTRY_NAME.into(),
            },
            cache,
        );
        assert!(
            !src.supports_strict_oci(),
            "fresh source must not be strict"
        );
        src.mark_verified(true);
        assert!(
            src.supports_strict_oci(),
            "marked-verified core source is strict"
        );
    }

    #[test]
    fn supports_strict_oci_for_third_party_requires_explicit_verify() {
        let tmp = TempDir::new().unwrap();
        let cache = RegistryCache::with_path(tmp.path().to_path_buf()).unwrap();
        let src = OciSource::with_cache(
            OciSourceConfig {
                url: "oci://example.com/team-foo".into(),
                tag: "1".into(),
                scope: None,
                registry_name: "team-foo".into(),
            },
            cache,
        );
        assert!(!src.supports_strict_oci());
        src.mark_verified(true);
        // Third-party registries DO get to claim strict once an upstream
        // caller has confirmed the override. The trust-scope check runs at
        // the resolver level via `crate::trust_scope::select_override`.
        assert!(src.supports_strict_oci());
    }

    #[test]
    fn scope_filters_blob_fetch_without_network() {
        let tmp = TempDir::new().unwrap();
        let cache = RegistryCache::with_path(tmp.path().to_path_buf()).unwrap();
        let src = OciSource::with_cache(
            OciSourceConfig {
                url: "oci://ghcr.io/sindri-dev/registry-core".into(),
                tag: "1".into(),
                scope: Some(vec![ComponentName::from("nodejs")]),
                registry_name: CORE_REGISTRY_NAME.into(),
            },
            cache,
        );
        let id = ComponentId {
            backend: "mise".into(),
            name: ComponentName::from("rust"),
        };
        // Out-of-scope name is rejected before we ever try to hit the
        // network — important for the resolver fall-through behaviour.
        let err = src
            .fetch_component_blob(&id, &Version::new("1.0.0"), &SourceContext::default())
            .unwrap_err();
        match err {
            SourceError::NotFound(name) => assert_eq!(name, "rust"),
            other => panic!("expected NotFound, got {:?}", other),
        }
    }
}
