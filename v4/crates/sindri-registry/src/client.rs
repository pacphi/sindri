//! Live OCI registry client (ADR-003) with cosign verification (ADR-014).
//!
//! Wave 3A.2 promotes the Wave 3A.1 scaffold into an operational client:
//!
//! 1. [`RegistryClient::fetch_index`] now pulls the registry's `index.yaml`
//!    via the OCI Distribution Spec using [`oci_client::Client`].
//! 2. Before the index is returned to the caller, the cosign signature on
//!    the registry artifact is verified against the trusted-key set loaded
//!    by [`crate::CosignVerifier`].
//! 3. The content-addressed [`crate::cache::RegistryCache`] is the source of
//!    truth for cache hits; the legacy `<registry>/index.yaml` cache entry is
//!    written alongside for backwards compatibility with the resolver's
//!    `load_registry_from_cache` path until that, too, migrates to the
//!    digest layout.
//!
//! ## Authentication
//!
//! - Anonymous public-registry pulls are the default (`RegistryAuth::Anonymous`).
//! - When `~/.docker/config.json` exists and contains an `auths` entry whose
//!   key matches the registry hostname, the basic-auth credentials are
//!   extracted and used (`RegistryAuth::Basic`). `oci-client` then handles
//!   the standard `Www-Authenticate: Bearer realm=…` token exchange
//!   transparently.
//!
//! ## Insecure mode
//!
//! Callers may pass `insecure = true` to bypass cosign verification, but only
//! when the active [`InstallPolicy`] does **not** require signed registries.
//! In strict mode an `--insecure` flag is rejected with
//! [`RegistryError::InsecureForbiddenByPolicy`] — the strict-mode contract
//! (ADR-014) is non-negotiable.

use crate::cache::{BlobKind, RegistryCache};
use crate::error::RegistryError;
use crate::index::RegistryIndex;
use crate::oci_ref::{OciRef, OciReference};
use crate::signing::CosignVerifier;
use base64::Engine as _;
use oci_client::client::ClientConfig;
use oci_client::manifest::OciManifest;
use oci_client::secrets::RegistryAuth;
use oci_client::Client as OciClient;
use oci_client::Reference as OciClientReference;
use sindri_core::policy::InstallPolicy;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

/// Media type used when the registry artifact is a single raw `index.yaml`
/// blob (one layer, no tar wrapping). This is the simplest "registry-as-OCI
/// artifact" form; documented here so registry publishers have a stable
/// target to reach for.
pub const SINDRI_INDEX_MEDIA_TYPE: &str = "application/vnd.sindri.registry.index.v1+yaml";

/// Standard OCI tarball-gzip media type. Accepted as a fallback when a
/// registry publisher chose to bundle their `index.yaml` inside a tarball
/// (e.g. for hosting the index alongside other assets). Wave 5A wires this
/// through [`crate::tarball::extract_layer`] with path-traversal protection
/// and digest verification.
pub const OCI_TAR_GZIP_MEDIA_TYPE: &str = "application/vnd.oci.image.layer.v1.tar+gzip";

/// Standard OCI uncompressed-tar layer media type. Wave 5A — D6.
pub const OCI_TAR_MEDIA_TYPE: &str = "application/vnd.oci.image.layer.v1.tar";

/// Cosign simple-signing payload media type (cosign spec). The signature
/// manifest layer carrying this media type contains the canonical
/// simple-signing JSON document we verify against.
pub const COSIGN_SIMPLESIGNING_MEDIA_TYPE: &str =
    "application/vnd.dev.cosign.simplesigning.v1+json";

/// Annotation key on the cosign signature manifest holding the base64-encoded
/// signature bytes (cosign spec).
pub const COSIGN_SIGNATURE_ANNOTATION: &str = "dev.cosignproject.cosign/signature";

/// OCI registry client — fetches `index.yaml` artifacts (ADR-003) and
/// verifies their cosign signatures (ADR-014) before handing them back.
///
/// See the module-level docs for the full protocol contract.
pub struct RegistryClient {
    cache: RegistryCache,
    ttl: Duration,
    /// Active install policy. Consulted by [`Self::fetch_index`] to decide
    /// whether unsigned registries are tolerated.
    policy: Option<InstallPolicy>,
    /// Trust-key set used to verify cosign signatures. `None` means "no
    /// keys loaded" — equivalent to an empty trust set.
    verifier: Option<Arc<CosignVerifier>>,
    /// When `true`, cosign verification is skipped (with a `tracing::warn!`).
    /// Cannot be combined with a strict signing policy.
    insecure: bool,
    /// Underlying OCI Distribution Spec client.
    oci: OciClient,
}

impl RegistryClient {
    /// Construct a client backed by the default user cache.
    pub fn new() -> Result<Self, RegistryError> {
        Ok(RegistryClient {
            cache: RegistryCache::new()?,
            ttl: Duration::from_secs(3600),
            policy: None,
            verifier: None,
            insecure: false,
            oci: OciClient::new(ClientConfig::default()),
        })
    }

    /// Construct a client with an explicit cache root (test harnesses).
    pub fn with_cache(cache: RegistryCache) -> Self {
        RegistryClient {
            cache,
            ttl: Duration::from_secs(3600),
            policy: None,
            verifier: None,
            insecure: false,
            oci: OciClient::new(ClientConfig::default()),
        }
    }

    /// Override the cache TTL (default: 1h).
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Attach an install policy. Consulted in [`Self::fetch_index`].
    pub fn with_policy(mut self, policy: InstallPolicy) -> Self {
        self.policy = Some(policy);
        self
    }

    /// Attach a cosign trust-key set. When unset, [`Self::fetch_index`] will
    /// only succeed if the policy does *not* require signing.
    pub fn with_verifier(mut self, verifier: CosignVerifier) -> Self {
        self.verifier = Some(Arc::new(verifier));
        self
    }

    /// Replace the underlying [`OciClient`] (test harnesses).
    ///
    /// Used by the wiremock-backed tests in `tests/oci_wiremock.rs` to swap
    /// in a client configured for plain HTTP against an in-process mock
    /// registry. Production callers should never need this.
    pub fn with_oci_client(mut self, oci: OciClient) -> Self {
        self.oci = oci;
        self
    }

    /// Bypass cosign verification with a loud warning (ADR-014 §"Escape
    /// hatches"). Forbidden in strict mode — see
    /// [`RegistryError::InsecureForbiddenByPolicy`].
    pub fn with_insecure(mut self, insecure: bool) -> Self {
        self.insecure = insecure;
        self
    }

    /// Read the policy currently attached to this client.
    pub fn policy(&self) -> Option<&InstallPolicy> {
        self.policy.as_ref()
    }

    /// Whether `--insecure` is active on this client.
    pub fn is_insecure(&self) -> bool {
        self.insecure
    }

    /// Verify the cosign signature on the most recently fetched (or cached)
    /// artifact for `registry_name`.
    ///
    /// Resolves the OCI reference + digest from the cache index built by
    /// [`Self::fetch_index`], then calls
    /// [`CosignVerifier::verify_registry_signature`].
    pub async fn verify(
        &self,
        registry_name: &str,
        oci_ref: &OciRef,
    ) -> Result<String, RegistryError> {
        let digest = self
            .cache
            .lookup_ref(registry_name, oci_ref)
            .ok_or_else(|| RegistryError::SignatureRequired {
                registry: registry_name.to_string(),
                reason: format!(
                    "no cached digest for {} — run `sindri registry refresh` first",
                    oci_ref.to_canonical()
                ),
            })?;
        let verifier = self.verifier.as_ref();
        let policy_requires = self
            .policy
            .as_ref()
            .and_then(|p| p.require_signed_registries)
            .unwrap_or(false);

        match verifier {
            Some(v) => {
                let key_id = v
                    .verify_registry_signature(
                        &self.oci,
                        registry_name,
                        oci_ref,
                        &digest,
                        policy_requires,
                    )
                    .await?;
                Ok(key_id)
            }
            None => {
                if policy_requires {
                    Err(RegistryError::SignatureRequired {
                        registry: registry_name.to_string(),
                        reason: "no trusted keys loaded".into(),
                    })
                } else {
                    tracing::warn!(
                        "no trusted keys for registry '{}'; skipping signature verification",
                        registry_name
                    );
                    Ok("<unsigned>".to_string())
                }
            }
        }
    }

    /// Fetch the registry index, using cache if within TTL.
    ///
    /// On a cache miss, performs a real OCI Distribution Spec pull, verifies
    /// the cosign signature, and writes both the digest-keyed cache and the
    /// legacy `<registry>/index.yaml` path before returning.
    pub async fn fetch_index(
        &self,
        registry_name: &str,
        registry_url: &str,
    ) -> Result<(RegistryIndex, Option<String>), RegistryError> {
        // Local-filesystem protocol: bypasses OCI + cosign entirely. Used
        // for development; fixtures keep working.
        if let Some(path) = registry_url.strip_prefix("registry:local:") {
            let index_path = Path::new(path).join("index.yaml");
            let content = std::fs::read_to_string(&index_path).map_err(RegistryError::Io)?;
            self.cache.put_index(registry_name, &content)?;
            let index = RegistryIndex::from_yaml(&content).map_err(RegistryError::Yaml)?;
            return Ok((index, None));
        }

        if let Some(cached) = self.cache.get_index(registry_name, self.ttl) {
            tracing::debug!("Using cached index for {}", registry_name);
            let index = RegistryIndex::from_yaml(&cached).map_err(RegistryError::Yaml)?;
            return Ok((index, None));
        }

        let (content, digest, oci_ref) = self.pull_index_blob(registry_url).await?;
        self.maybe_verify(registry_name, &oci_ref, &digest).await?;

        // Persist into both cache layouts (digest-addressed + legacy).
        self.cache
            .put_by_digest(&digest, BlobKind::Index, content.as_bytes())?;
        self.cache.link_ref(registry_name, &oci_ref, &digest)?;
        self.cache.put_index(registry_name, &content)?;

        let index = RegistryIndex::from_yaml(&content).map_err(RegistryError::Yaml)?;
        Ok((index, Some(digest)))
    }

    /// Force-refresh the registry index, bypassing cache.
    pub async fn refresh_index(
        &self,
        registry_name: &str,
        registry_url: &str,
    ) -> Result<(RegistryIndex, Option<String>), RegistryError> {
        if let Some(path) = registry_url.strip_prefix("registry:local:") {
            let index_path = Path::new(path).join("index.yaml");
            let content = std::fs::read_to_string(&index_path).map_err(RegistryError::Io)?;
            self.cache.put_index(registry_name, &content)?;
            let index = RegistryIndex::from_yaml(&content).map_err(RegistryError::Yaml)?;
            return Ok((index, None));
        }
        let (content, digest, oci_ref) = self.pull_index_blob(registry_url).await?;
        self.maybe_verify(registry_name, &oci_ref, &digest).await?;
        self.cache
            .put_by_digest(&digest, BlobKind::Index, content.as_bytes())?;
        self.cache.link_ref(registry_name, &oci_ref, &digest)?;
        self.cache.put_index(registry_name, &content)?;
        let index = RegistryIndex::from_yaml(&content).map_err(RegistryError::Yaml)?;
        Ok((index, Some(digest)))
    }

    /// Fetch the **verbatim manifest bytes** for an OCI reference (Phase 3.3
    /// follow-up, ADR-028).
    ///
    /// Unlike [`Self::fetch_index`] — which parses the manifest and then
    /// re-serializes the index — this returns the exact bytes the registry
    /// sent, together with their SHA-256 content digest. The caller can write
    /// these bytes directly into an OCI image-layout blob store and the
    /// digest will be identical to `sha256(bytes)`. That byte identity is
    /// what `sindri registry prefetch` needs to guarantee that the
    /// `LocalOciSource` consuming the layout sees the same cosign trust chain
    /// as a live `OciSource` pull.
    ///
    /// Accepted media types mirror the OCI Distribution Spec read path:
    /// `application/vnd.oci.image.manifest.v1+json` and
    /// `application/vnd.oci.image.index.v1+json`.
    pub async fn fetch_registry_manifest_bytes(
        &self,
        registry_url: &str,
    ) -> Result<(Vec<u8>, String), RegistryError> {
        let oci_ref = OciRef::parse(registry_url)?;
        let reference = oci_reference_for(&oci_ref);
        let auth = docker_config_auth(&oci_ref.registry).unwrap_or(RegistryAuth::Anonymous);

        tracing::debug!(
            "fetching verbatim manifest bytes for {} (Phase 3.3 follow-up)",
            oci_ref.to_canonical()
        );

        let accepted = &[
            "application/vnd.oci.image.manifest.v1+json",
            "application/vnd.oci.image.index.v1+json",
            "application/vnd.docker.distribution.manifest.v2+json",
        ];
        let (raw, digest) = self
            .oci
            .pull_manifest_raw(&reference, &auth, accepted)
            .await
            .map_err(|e| RegistryError::OciFetch {
                reference: oci_ref.to_canonical(),
                detail: format!("raw manifest fetch failed: {}", e),
            })?;
        Ok((raw.to_vec(), digest))
    }

    /// Fetch the SHA-256 digest of a *component's* primary OCI layer.
    ///
    /// Wave 5F — D5 (carry-over from PR #228): the resolver populates the
    /// per-component `component_digest` lockfile field via this helper so
    /// that `sindri apply`'s cosign pre-flight (added in #228) can verify a
    /// per-component signature before the install backend runs.
    ///
    /// Returns the layer descriptor digest (e.g. `"sha256:…"`) of the
    /// manifest's first layer. Unlike [`Self::fetch_index`], this does *not*
    /// pull the layer blob — only the manifest — because the layer
    /// descriptor digest is what cosign needs and the apply pipeline does
    /// the full layer pull lazily.
    ///
    /// `oci_ref_str` accepts the same forms as [`OciRef::parse`]. Anonymous
    /// auth is used unless `~/.docker/config.json` provides credentials, in
    /// which case the bearer-token flow is delegated to `oci-client`.
    pub async fn fetch_component_layer_digest(
        &self,
        oci_ref_str: &str,
    ) -> Result<String, RegistryError> {
        let oci_ref = OciRef::parse(oci_ref_str)?;
        let reference = oci_reference_for(&oci_ref);
        let auth = docker_config_auth(&oci_ref.registry).unwrap_or(RegistryAuth::Anonymous);

        tracing::debug!(
            "fetching component manifest {} for layer digest (Wave 5F D5)",
            oci_ref.to_canonical()
        );

        let (manifest, _manifest_digest) = self
            .oci
            .pull_manifest(&reference, &auth)
            .await
            .map_err(|e| RegistryError::OciFetch {
                reference: oci_ref.to_canonical(),
                detail: e.to_string(),
            })?;

        let image_manifest = match manifest {
            OciManifest::Image(m) => m,
            OciManifest::ImageIndex(_) => {
                return Err(RegistryError::OciFetch {
                    reference: oci_ref.to_canonical(),
                    detail: "expected image manifest, got image index".into(),
                });
            }
        };

        let layer = image_manifest
            .layers
            .first()
            .ok_or_else(|| RegistryError::OciFetch {
                reference: oci_ref.to_canonical(),
                detail: "manifest has no layers".into(),
            })?;

        Ok(layer.digest.clone())
    }

    /// Fetch the raw bytes of a *component's* primary OCI layer (Phase 3.0,
    /// ADR-028).
    ///
    /// Where [`Self::fetch_component_layer_digest`] only pulls the manifest
    /// to read the layer descriptor digest, this method also pulls the layer
    /// blob and verifies the bytes hash to the manifest's declared digest.
    ///
    /// Returns `(layer_digest, layer_bytes)`. The digest is the layer's
    /// content digest as recorded in the manifest (e.g.
    /// `"sha256:abc..."`); the caller can re-hash the bytes if they want a
    /// belt-and-braces double check (`OciSource::fetch_component_blob` does
    /// exactly that).
    ///
    /// `oci_ref_str` accepts the same forms as [`OciRef::parse`]. Anonymous
    /// auth is the default; `~/.docker/config.json` credentials are picked
    /// up automatically.
    pub async fn fetch_component_layer_bytes(
        &self,
        oci_ref_str: &str,
    ) -> Result<(String, Vec<u8>), RegistryError> {
        let oci_ref = OciRef::parse(oci_ref_str)?;
        let reference = oci_reference_for(&oci_ref);
        let auth = docker_config_auth(&oci_ref.registry).unwrap_or(RegistryAuth::Anonymous);

        tracing::debug!(
            "fetching component layer bytes for {} (Phase 3.0)",
            oci_ref.to_canonical()
        );

        let (manifest, _manifest_digest) = self
            .oci
            .pull_manifest(&reference, &auth)
            .await
            .map_err(|e| RegistryError::OciFetch {
                reference: oci_ref.to_canonical(),
                detail: e.to_string(),
            })?;

        let image_manifest = match manifest {
            OciManifest::Image(m) => m,
            OciManifest::ImageIndex(_) => {
                return Err(RegistryError::OciFetch {
                    reference: oci_ref.to_canonical(),
                    detail: "expected image manifest, got image index".into(),
                });
            }
        };

        let layer = image_manifest
            .layers
            .first()
            .ok_or_else(|| RegistryError::OciFetch {
                reference: oci_ref.to_canonical(),
                detail: "manifest has no layers".into(),
            })?;

        let mut buf: Vec<u8> = Vec::new();
        self.oci
            .pull_blob(&reference, layer, &mut buf)
            .await
            .map_err(|e| RegistryError::OciFetch {
                reference: oci_ref.to_canonical(),
                detail: format!("blob pull failed: {}", e),
            })?;

        // Verify the bytes match the layer's declared digest. The layer
        // descriptor digest from the manifest must equal sha256(buf).
        verify_layer_digest(&layer.digest, &buf, &oci_ref)?;

        Ok((layer.digest.clone(), buf))
    }

    // ------------------------------------------------------------------
    // internals
    // ------------------------------------------------------------------

    /// Pull `index.yaml` content from an OCI registry using the OCI
    /// Distribution Spec. Returns `(content, manifest_digest, parsed_ref)`.
    async fn pull_index_blob(
        &self,
        registry_url: &str,
    ) -> Result<(String, String, OciRef), RegistryError> {
        let oci_ref = OciRef::parse(registry_url)?;
        let reference = oci_reference_for(&oci_ref);
        let auth = docker_config_auth(&oci_ref.registry).unwrap_or(RegistryAuth::Anonymous);

        tracing::info!(
            "Pulling registry artifact {} via OCI Distribution Spec",
            oci_ref.to_canonical()
        );

        let (manifest, digest) = self
            .oci
            .pull_manifest(&reference, &auth)
            .await
            .map_err(|e| RegistryError::OciFetch {
                reference: oci_ref.to_canonical(),
                detail: e.to_string(),
            })?;

        let image_manifest = match manifest {
            OciManifest::Image(m) => m,
            OciManifest::ImageIndex(_) => {
                return Err(RegistryError::OciFetch {
                    reference: oci_ref.to_canonical(),
                    detail: "expected image manifest, got image index".into(),
                });
            }
        };

        let layer = image_manifest
            .layers
            .first()
            .ok_or_else(|| RegistryError::OciFetch {
                reference: oci_ref.to_canonical(),
                detail: "manifest has no layers".into(),
            })?;

        let mut buf: Vec<u8> = Vec::new();
        self.oci
            .pull_blob(&reference, layer, &mut buf)
            .await
            .map_err(|e| RegistryError::OciFetch {
                reference: oci_ref.to_canonical(),
                detail: format!("blob pull failed: {}", e),
            })?;

        let content = match layer.media_type.as_str() {
            SINDRI_INDEX_MEDIA_TYPE => {
                String::from_utf8(buf).map_err(|e| RegistryError::OciFetch {
                    reference: oci_ref.to_canonical(),
                    detail: format!("layer was not valid UTF-8: {}", e),
                })?
            }
            OCI_TAR_GZIP_MEDIA_TYPE | OCI_TAR_MEDIA_TYPE => {
                // Wave 5A — D6: extract tar/gzip layer with path-traversal
                // protection and streaming digest verification.
                extract_index_yaml_from_layer(
                    &buf,
                    layer.media_type.as_str(),
                    &layer.digest,
                    &oci_ref,
                )?
            }
            other => {
                return Err(RegistryError::UnsupportedMediaType {
                    reference: oci_ref.to_canonical(),
                    media_type: other.to_string(),
                    expected: format!(
                        "{}, {}, or {}",
                        SINDRI_INDEX_MEDIA_TYPE, OCI_TAR_GZIP_MEDIA_TYPE, OCI_TAR_MEDIA_TYPE
                    ),
                });
            }
        };

        Ok((content, digest, oci_ref))
    }

    async fn maybe_verify(
        &self,
        registry_name: &str,
        oci_ref: &OciRef,
        digest: &str,
    ) -> Result<(), RegistryError> {
        let policy_requires = self
            .policy
            .as_ref()
            .and_then(|p| p.require_signed_registries)
            .unwrap_or(false);

        if self.insecure {
            if policy_requires {
                return Err(RegistryError::InsecureForbiddenByPolicy {
                    registry: registry_name.to_string(),
                });
            }
            tracing::warn!(
                "INSECURE: skipping cosign verification for registry '{}' ({})",
                registry_name,
                oci_ref.to_canonical()
            );
            return Ok(());
        }

        match self.verifier.as_ref() {
            Some(v) => {
                let key_id = v
                    .verify_registry_signature(
                        &self.oci,
                        registry_name,
                        oci_ref,
                        digest,
                        policy_requires,
                    )
                    .await?;
                if key_id != "<unsigned>" {
                    tracing::info!(
                        "Verified registry '{}' signature against trusted key {}",
                        registry_name,
                        key_id
                    );
                }
                Ok(())
            }
            None => {
                if policy_requires {
                    Err(RegistryError::SignatureRequired {
                        registry: registry_name.to_string(),
                        reason: "no trusted keys loaded for registry".into(),
                    })
                } else {
                    tracing::warn!(
                        "no trusted keys for registry '{}'; skipping cosign verification \
                         (policy.require_signed_registries=false)",
                        registry_name
                    );
                    Ok(())
                }
            }
        }
    }
}

/// Extract `index.yaml` content from a tar (or tar+gzip) OCI layer blob.
///
/// Wraps [`crate::tarball::extract_layer`] with the registry-error
/// translation and the convention that the layer's *root* must contain an
/// `index.yaml` entry. Anything else is treated as a malformed registry
/// artifact.
fn extract_index_yaml_from_layer(
    blob: &[u8],
    media_type: &str,
    descriptor_digest: &str,
    oci_ref: &OciRef,
) -> Result<String, RegistryError> {
    use crate::tarball::{read_entry_from_layer, LayerCompression, TarballError};
    use std::path::Path;
    let compression = LayerCompression::from_media_type(media_type).ok_or_else(|| {
        RegistryError::UnsupportedMediaType {
            reference: oci_ref.to_canonical(),
            media_type: media_type.to_string(),
            expected: format!("{} or {}", OCI_TAR_GZIP_MEDIA_TYPE, OCI_TAR_MEDIA_TYPE),
        }
    })?;
    let entry = read_entry_from_layer(
        blob,
        compression,
        descriptor_digest,
        Path::new("index.yaml"),
    )
    .map_err(|e| match e {
        TarballError::DigestMismatch { expected, actual } => RegistryError::OciFetch {
            reference: oci_ref.to_canonical(),
            detail: format!(
                "layer digest mismatch — expected {}, computed sha256:{}",
                expected, actual
            ),
        },
        TarballError::UnsafePath { path, reason } => RegistryError::LayerExtraction {
            reference: oci_ref.to_canonical(),
            detail: format!("unsafe entry '{}': {}", path, reason),
        },
        TarballError::BadDescriptorDigest(d) => RegistryError::OciFetch {
            reference: oci_ref.to_canonical(),
            detail: format!("malformed descriptor digest '{}'", d),
        },
        TarballError::Io(io) => RegistryError::Io(io),
    })?;
    let bytes = entry.ok_or_else(|| RegistryError::IndexMissingFromLayer {
        reference: oci_ref.to_canonical(),
    })?;
    String::from_utf8(bytes).map_err(|e| RegistryError::OciFetch {
        reference: oci_ref.to_canonical(),
        detail: format!("index.yaml in tar layer was not valid UTF-8: {}", e),
    })
}

/// Verify that `bytes` hash to `expected_digest` (a `sha256:<hex>` string).
///
/// Used by [`RegistryClient::fetch_component_layer_bytes`] to give callers
/// the reassurance that the layer they pulled matches the digest the
/// manifest declared. A mismatch surfaces as
/// [`RegistryError::OciFetch`] — it would be either an upstream tampering
/// scenario or a transport-level corruption, both of which the caller
/// must reject.
fn verify_layer_digest(
    expected_digest: &str,
    bytes: &[u8],
    oci_ref: &OciRef,
) -> Result<(), RegistryError> {
    use sha2::{Digest, Sha256};
    let want = expected_digest
        .strip_prefix("sha256:")
        .ok_or_else(|| RegistryError::OciFetch {
            reference: oci_ref.to_canonical(),
            detail: format!("layer digest {} is not sha256-prefixed", expected_digest),
        })?;
    let actual = hex::encode(Sha256::digest(bytes));
    if actual != want {
        return Err(RegistryError::OciFetch {
            reference: oci_ref.to_canonical(),
            detail: format!(
                "layer digest mismatch — expected {}, computed sha256:{}",
                expected_digest, actual
            ),
        });
    }
    Ok(())
}

/// Convert an [`OciRef`] into the [`oci_client::Reference`] type expected by
/// the underlying client.
fn oci_reference_for(oci: &OciRef) -> OciClientReference {
    match &oci.reference {
        OciReference::Tag(tag) => {
            OciClientReference::with_tag(oci.registry.clone(), oci.repository.clone(), tag.clone())
        }
        OciReference::Digest(d) => {
            OciClientReference::with_digest(oci.registry.clone(), oci.repository.clone(), d.clone())
        }
    }
}

/// Decode `~/.docker/config.json` and return basic-auth credentials for the
/// requested registry hostname, if present. Failures are silently treated as
/// "no credentials" — anonymous pull is the safe default.
pub(crate) fn docker_config_auth(registry: &str) -> Option<RegistryAuth> {
    let path = dirs_next::home_dir()?.join(".docker").join("config.json");
    let raw = std::fs::read_to_string(&path).ok()?;
    parse_docker_config_auth(&raw, registry)
}

/// Pure-function helper for [`docker_config_auth`] — exposed for unit tests
/// so we don't depend on the user's actual `~/.docker/config.json`.
pub(crate) fn parse_docker_config_auth(raw: &str, registry: &str) -> Option<RegistryAuth> {
    let v: serde_json::Value = serde_json::from_str(raw).ok()?;
    let auths = v.get("auths")?.as_object()?;
    // Match the registry hostname against the auths map. Docker's keys can
    // include a scheme prefix (https://) and a trailing path; normalise both
    // sides before comparing.
    let normalised_target = normalise_registry_host(registry);
    let entry = auths.iter().find_map(|(k, v)| {
        if normalise_registry_host(k) == normalised_target {
            Some(v)
        } else {
            None
        }
    })?;
    let auth_b64 = entry.get("auth")?.as_str()?;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(auth_b64.as_bytes())
        .ok()?;
    let s = std::str::from_utf8(&decoded).ok()?;
    let (user, password) = s.split_once(':')?;
    Some(RegistryAuth::Basic(user.to_string(), password.to_string()))
}

fn normalise_registry_host(s: &str) -> String {
    let s = s.trim();
    let s = s.strip_prefix("https://").unwrap_or(s);
    let s = s.strip_prefix("http://").unwrap_or(s);
    let s = s.split('/').next().unwrap_or(s);
    s.trim_end_matches('/').to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_client() -> (TempDir, RegistryClient) {
        let tmp = TempDir::new().unwrap();
        let cache = RegistryCache::with_path(tmp.path().to_path_buf()).unwrap();
        (tmp, RegistryClient::with_cache(cache))
    }

    #[tokio::test]
    async fn registry_local_protocol_unaffected() {
        // The `registry:local:` protocol must bypass OCI/cosign entirely so
        // local development workflows never hit the network and never touch
        // the verifier.
        let tmp = TempDir::new().unwrap();
        let registry_dir = tmp.path().join("local-reg");
        std::fs::create_dir_all(&registry_dir).unwrap();
        let index = "version: 1\nregistry: local\ncomponents: []\n";
        std::fs::write(registry_dir.join("index.yaml"), index).unwrap();

        let (_cache_tmp, client) = temp_client();
        let url = format!("registry:local:{}", registry_dir.display());
        let (parsed, digest) = client.fetch_index("dev-local", &url).await.unwrap();
        assert!(digest.is_none(), "local protocol must not produce a digest");
        assert_eq!(parsed.components.len(), 0);
    }

    #[test]
    fn parse_docker_config_extracts_basic_auth() {
        let cfg = r#"{
            "auths": {
                "ghcr.io": { "auth": "dXNlcjpwYXNz" }
            }
        }"#;
        let auth = parse_docker_config_auth(cfg, "ghcr.io").unwrap();
        match auth {
            RegistryAuth::Basic(u, p) => {
                assert_eq!(u, "user");
                assert_eq!(p, "pass");
            }
            other => panic!("expected Basic auth, got {:?}", other),
        }
    }

    #[test]
    fn parse_docker_config_normalises_https_prefix() {
        let cfg = r#"{
            "auths": {
                "https://index.docker.io/v1/": { "auth": "Zm9vOmJhcg==" }
            }
        }"#;
        let auth = parse_docker_config_auth(cfg, "index.docker.io").unwrap();
        assert!(matches!(auth, RegistryAuth::Basic(_, _)));
    }

    #[test]
    fn parse_docker_config_returns_none_when_registry_missing() {
        let cfg = r#"{ "auths": { "ghcr.io": { "auth": "Zm9vOmJhcg==" } } }"#;
        assert!(parse_docker_config_auth(cfg, "registry.example.com").is_none());
    }

    #[test]
    fn parse_docker_config_returns_none_for_garbage() {
        assert!(parse_docker_config_auth("not-json", "ghcr.io").is_none());
        assert!(parse_docker_config_auth("{}", "ghcr.io").is_none());
        // Missing `auth` field.
        assert!(parse_docker_config_auth(r#"{"auths":{"ghcr.io":{}}}"#, "ghcr.io").is_none());
    }

    #[test]
    fn oci_reference_for_tag_and_digest() {
        let r = OciRef::parse("ghcr.io/sindri-dev/registry-core:1.0.0").unwrap();
        let conv = oci_reference_for(&r);
        assert_eq!(conv.registry(), "ghcr.io");
        assert_eq!(conv.repository(), "sindri-dev/registry-core");
        assert_eq!(conv.tag(), Some("1.0.0"));

        let digest = format!("sha256:{}", "a".repeat(64));
        let r2 = OciRef::parse(&format!("ghcr.io/foo/bar@{}", digest)).unwrap();
        let conv2 = oci_reference_for(&r2);
        assert_eq!(conv2.digest(), Some(digest.as_str()));
    }
}
