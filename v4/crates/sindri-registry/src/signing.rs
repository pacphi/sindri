//! Cosign trust-key loading + signature verification (ADR-014).
//!
//! ## On-disk layout
//!
//! ```text
//! ~/.sindri/trust/
//!   <registry-name>/
//!     cosign-<short-key-id>.pub
//! ```
//!
//! Each `.pub` file is an ECDSA P-256 public key in PEM (PKCS#8 SPKI) form,
//! the default cosign key format.
//!
//! ## Verification flow (Wave 3A.2 — operational)
//!
//! Per the [cosign signature spec][cosign-spec], a signed OCI artifact at
//! `<repo>:<tag>` (digest `sha256:<hex>`) has a companion signature manifest
//! at the *tag* `sha256-<hex>.sig`. The signature manifest's first layer is
//! a JSON document of media type
//! `application/vnd.dev.cosign.simplesigning.v1+json` whose
//! `critical.image.docker-manifest-digest` field MUST match the original
//! artifact digest. The signature itself is base64-encoded in the manifest's
//! `dev.cosignproject.cosign/signature` annotation.
//!
//! [`CosignVerifier::verify_payload`] is the pure, allocation-free verifier
//! over already-fetched bytes; [`CosignVerifier::verify_registry_signature`]
//! adds the OCI fetch wrapping needed at runtime.
//!
//! [cosign-spec]: https://github.com/sigstore/cosign/blob/main/specs/SIGNATURE_SPEC.md

use crate::error::RegistryError;
use crate::oci_ref::OciRef;
use base64::Engine as _;
use ecdsa::elliptic_curve::pkcs8::DecodePublicKey;
use ecdsa::signature::Verifier;
use oci_client::manifest::OciManifest;
use oci_client::secrets::RegistryAuth;
use oci_client::Client as OciClient;
use oci_client::Reference as OciClientReference;
use p256::ecdsa::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// A single trusted cosign key, parsed from PEM.
#[derive(Debug, Clone)]
pub struct TrustedKey {
    /// First 8 hex chars of the SHA-256 of the DER-encoded SPKI bytes.
    /// Stable, human-friendly identifier suitable for filenames + log lines.
    pub key_id: String,
    /// Original PEM (re-export / debug).
    pub spki_pem: String,
    /// The parsed verifying key.
    pub key: VerifyingKey,
}

impl TrustedKey {
    /// Parse a PEM public key, computing its short key id.
    pub fn from_pem(pem: &str) -> Result<Self, RegistryError> {
        let key = VerifyingKey::from_public_key_pem(pem).map_err(|e| {
            RegistryError::TrustKeyParseFailed {
                path: "<pem>".to_string(),
                detail: e.to_string(),
            }
        })?;
        let spki_der = key.to_encoded_point(false);
        let mut hasher = Sha256::new();
        hasher.update(spki_der.as_bytes());
        let digest = hasher.finalize();
        let key_id = hex::encode(&digest[..4]);
        Ok(TrustedKey {
            key_id,
            spki_pem: pem.to_string(),
            key,
        })
    }
}

/// Set of trusted cosign keys, indexed by registry name.
///
/// Built by [`CosignVerifier::load_from_trust_dir`] from
/// `~/.sindri/trust/<registry>/cosign-*.pub`.
#[derive(Debug)]
pub struct CosignVerifier {
    trusted_keys: HashMap<String, Vec<TrustedKey>>,
}

impl CosignVerifier {
    /// Load all trust keys under `root` (typically `~/.sindri/trust/`).
    ///
    /// - Each immediate subdirectory is treated as a registry name.
    /// - Inside each subdirectory, every file matching `cosign-*.pub` is
    ///   parsed as a P-256 public key.
    /// - A malformed key file aborts the whole load with
    ///   [`RegistryError::TrustKeyParseFailed`] — we fail closed rather than
    ///   silently dropping bad keys.
    /// - A non-existent `root` is treated as an empty trust set.
    pub fn load_from_trust_dir(root: &Path) -> Result<Self, RegistryError> {
        let mut trusted_keys: HashMap<String, Vec<TrustedKey>> = HashMap::new();
        if !root.exists() {
            return Ok(CosignVerifier { trusted_keys });
        }
        for entry in fs::read_dir(root)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let registry_name = match path.file_name().and_then(|s| s.to_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };
            let mut keys: Vec<TrustedKey> = Vec::new();
            for key_entry in fs::read_dir(&path)? {
                let key_entry = key_entry?;
                let key_path = key_entry.path();
                if !key_path.is_file() {
                    continue;
                }
                let name = key_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default();
                if !name.starts_with("cosign-") || !name.ends_with(".pub") {
                    continue;
                }
                let pem = fs::read_to_string(&key_path)?;
                let key = TrustedKey::from_pem(&pem).map_err(|e| match e {
                    RegistryError::TrustKeyParseFailed { detail, .. } => {
                        RegistryError::TrustKeyParseFailed {
                            path: key_path.display().to_string(),
                            detail,
                        }
                    }
                    other => other,
                })?;
                keys.push(key);
            }
            if !keys.is_empty() {
                trusted_keys.insert(registry_name, keys);
            }
        }
        Ok(CosignVerifier { trusted_keys })
    }

    /// Iterator over registry names with at least one trusted key.
    pub fn trusted_registries(&self) -> impl Iterator<Item = &str> {
        self.trusted_keys.keys().map(|s| s.as_str())
    }

    /// Trusted keys for `registry`, or an empty slice.
    pub fn keys_for(&self, registry: &str) -> &[TrustedKey] {
        self.trusted_keys
            .get(registry)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Verify an already-fetched cosign signature payload against the trust
    /// keys registered for `registry_name`.
    ///
    /// This is the pure-function core of the verification flow — given the
    /// raw simple-signing JSON bytes, the base64-decoded signature bytes,
    /// and the expected manifest digest of the artifact under attestation,
    /// returns the `key_id` of the first trusted key that verifies the
    /// signature, or a [`RegistryError::SignatureMismatch`].
    ///
    /// Caller responsibilities:
    ///
    /// - `payload_bytes` MUST be the canonical bytes read off the wire from
    ///   the cosign signature layer — do not pretty-print, re-serialize, or
    ///   otherwise normalise. Cosign signatures are computed over the bytes
    ///   the registry hosts, byte-for-byte.
    /// - `signature_bytes` MUST be the *raw* signature bytes after
    ///   base64-decoding the `dev.cosignproject.cosign/signature` annotation.
    /// - `expected_manifest_digest` is the `sha256:<hex>` digest of the
    ///   *original* artifact (not the signature manifest).
    pub fn verify_payload(
        &self,
        registry_name: &str,
        payload_bytes: &[u8],
        signature_bytes: &[u8],
        expected_manifest_digest: &str,
        policy_requires_signing: bool,
    ) -> Result<String, RegistryError> {
        // 1. Parse the simple-signing payload.
        let payload: serde_json::Value = serde_json::from_slice(payload_bytes).map_err(|e| {
            RegistryError::SignatureMismatch {
                registry: registry_name.to_string(),
                expected_keys: self.key_ids_for(registry_name),
                detail: format!("simple-signing payload was not valid JSON: {}", e),
            }
        })?;
        let actual_digest = payload
            .get("critical")
            .and_then(|c| c.get("image"))
            .and_then(|i| i.get("docker-manifest-digest"))
            .and_then(|d| d.as_str())
            .ok_or_else(|| RegistryError::SignatureMismatch {
                registry: registry_name.to_string(),
                expected_keys: self.key_ids_for(registry_name),
                detail: "simple-signing payload missing critical.image.docker-manifest-digest"
                    .into(),
            })?;
        if actual_digest != expected_manifest_digest {
            return Err(RegistryError::SignatureMismatch {
                registry: registry_name.to_string(),
                expected_keys: self.key_ids_for(registry_name),
                detail: format!(
                    "payload digest {} != expected {}",
                    actual_digest, expected_manifest_digest
                ),
            });
        }

        // 2. Empty trust set short-circuit. We resolve this before even
        //    attempting to parse the signature bytes so callers running
        //    with no trust keys get the precise SignatureRequired /
        //    `<unsigned>` outcome rather than a parse-error red herring.
        let keys = self.keys_for(registry_name);
        if keys.is_empty() {
            if policy_requires_signing {
                return Err(RegistryError::SignatureRequired {
                    registry: registry_name.to_string(),
                    reason: "no trusted keys configured for this registry".into(),
                });
            }
            tracing::warn!(
                "no trusted keys for registry '{}'; cosign signature not verified",
                registry_name
            );
            return Ok("<unsigned>".to_string());
        }

        // 3. Decode the signature bytes into a P-256 ECDSA Signature.
        let signature = Signature::from_der(signature_bytes)
            .or_else(|_| Signature::from_slice(signature_bytes))
            .map_err(|e| RegistryError::SignatureMismatch {
                registry: registry_name.to_string(),
                expected_keys: self.key_ids_for(registry_name),
                detail: format!(
                    "signature bytes are not a valid P-256 ECDSA signature: {}",
                    e
                ),
            })?;

        // 4. Try every trusted key for this registry.
        for key in keys {
            if key.key.verify(payload_bytes, &signature).is_ok() {
                return Ok(key.key_id.clone());
            }
        }
        Err(RegistryError::SignatureMismatch {
            registry: registry_name.to_string(),
            expected_keys: self.key_ids_for(registry_name),
            detail: "no trusted key matched the signature".into(),
        })
    }

    /// Fetch + verify a cosign signature for a registry artifact.
    ///
    /// Wraps [`Self::verify_payload`] with the OCI fetch protocol described
    /// in the module docs. Returns the `key_id` of the trusted key that
    /// verified, or `<unsigned>` when no keys are loaded and the policy
    /// permits unsigned registries.
    pub async fn verify_registry_signature(
        &self,
        oci: &OciClient,
        registry_name: &str,
        oci_ref: &OciRef,
        manifest_digest: &str,
        policy_requires_signing: bool,
    ) -> Result<String, RegistryError> {
        // Fast path: empty trust set + permissive policy → warn + skip.
        if self.keys_for(registry_name).is_empty() && !policy_requires_signing {
            tracing::warn!(
                "no trusted keys for registry '{}'; skipping cosign verification",
                registry_name
            );
            return Ok("<unsigned>".to_string());
        }
        if self.keys_for(registry_name).is_empty() && policy_requires_signing {
            return Err(RegistryError::SignatureRequired {
                registry: registry_name.to_string(),
                reason: "no trusted keys configured for this registry".into(),
            });
        }

        // 1. Compute the cosign signature tag: sha256:<hex> → sha256-<hex>.sig
        let sig_tag = cosign_signature_tag(manifest_digest).ok_or_else(|| {
            RegistryError::SignatureMismatch {
                registry: registry_name.to_string(),
                expected_keys: self.key_ids_for(registry_name),
                detail: format!(
                    "cannot derive cosign signature tag from '{}'",
                    manifest_digest
                ),
            }
        })?;
        let sig_ref = OciClientReference::with_tag(
            oci_ref.registry.clone(),
            oci_ref.repository.clone(),
            sig_tag,
        );
        let auth =
            crate::client::docker_config_auth(&oci_ref.registry).unwrap_or(RegistryAuth::Anonymous);

        // 2. Pull the signature manifest.
        let (manifest, _sig_manifest_digest) =
            oci.pull_manifest(&sig_ref, &auth).await.map_err(|e| {
                RegistryError::SignatureRequired {
                    registry: registry_name.to_string(),
                    reason: format!("could not pull signature manifest: {}", e),
                }
            })?;
        let image = match manifest {
            OciManifest::Image(m) => m,
            OciManifest::ImageIndex(_) => {
                return Err(RegistryError::SignatureMismatch {
                    registry: registry_name.to_string(),
                    expected_keys: self.key_ids_for(registry_name),
                    detail: "signature manifest unexpectedly was an image index".into(),
                });
            }
        };

        // 3. Find the simple-signing layer.
        let layer = image
            .layers
            .iter()
            .find(|l| l.media_type == crate::client::COSIGN_SIMPLESIGNING_MEDIA_TYPE)
            .ok_or_else(|| RegistryError::SignatureMismatch {
                registry: registry_name.to_string(),
                expected_keys: self.key_ids_for(registry_name),
                detail: format!(
                    "signature manifest missing layer with media type {}",
                    crate::client::COSIGN_SIMPLESIGNING_MEDIA_TYPE
                ),
            })?;

        // 4. Pull the layer (the simple-signing JSON payload).
        let mut payload_bytes: Vec<u8> = Vec::new();
        oci.pull_blob(&sig_ref, layer, &mut payload_bytes)
            .await
            .map_err(|e| RegistryError::SignatureMismatch {
                registry: registry_name.to_string(),
                expected_keys: self.key_ids_for(registry_name),
                detail: format!("could not pull simple-signing layer: {}", e),
            })?;

        // 5. Extract + base64-decode the signature annotation.
        let sig_b64 = image
            .annotations
            .as_ref()
            .and_then(|a| a.get(crate::client::COSIGN_SIGNATURE_ANNOTATION))
            .ok_or_else(|| RegistryError::SignatureMismatch {
                registry: registry_name.to_string(),
                expected_keys: self.key_ids_for(registry_name),
                detail: format!(
                    "signature manifest missing '{}' annotation",
                    crate::client::COSIGN_SIGNATURE_ANNOTATION
                ),
            })?;
        let signature_bytes = base64::engine::general_purpose::STANDARD
            .decode(sig_b64.as_bytes())
            .map_err(|e| RegistryError::SignatureMismatch {
                registry: registry_name.to_string(),
                expected_keys: self.key_ids_for(registry_name),
                detail: format!("signature annotation was not valid base64: {}", e),
            })?;

        // 6. Verify.
        self.verify_payload(
            registry_name,
            &payload_bytes,
            &signature_bytes,
            manifest_digest,
            policy_requires_signing,
        )
    }

    /// Per-component cosign verification (Wave 5A — D5).
    ///
    /// Variant of [`Self::verify_registry_signature`] that targets an
    /// individual component artifact rather than the registry-level
    /// `index.yaml`. The OCI protocol is identical (cosign hosts the
    /// signature manifest at `<repo>:sha256-<hex>.sig`); the only difference
    /// is the meaning of the `expected_digest` field — it is the digest of
    /// the component's primary OCI layer/manifest as recorded in the
    /// lockfile's `component_digest` field.
    ///
    /// Coordinates with the existing per-registry verification path: trust
    /// keys live under `~/.sindri/trust/<registry>/cosign-*.pub` regardless
    /// of whether they sign registry artifacts or component artifacts. A
    /// future enhancement may scope keys per-component, but Wave 5A keeps
    /// the trust model flat.
    pub async fn verify_component_signature(
        &self,
        oci: &OciClient,
        registry_name: &str,
        oci_ref: &OciRef,
        component_digest: &str,
        policy_requires_signing: bool,
    ) -> Result<String, RegistryError> {
        // Wraps the same fetch+verify pipeline. The registry-level helper
        // already takes the digest as an argument and treats it as opaque,
        // so the only behavioural difference is the audit log prefix.
        tracing::debug!(
            "verifying per-component cosign signature for {} ({}) under registry '{}'",
            oci_ref.to_canonical(),
            component_digest,
            registry_name
        );
        self.verify_registry_signature(
            oci,
            registry_name,
            oci_ref,
            component_digest,
            policy_requires_signing,
        )
        .await
    }

    /// Convenience wrapper around [`Self::verify_component_signature`] that
    /// constructs a default [`OciClient`] internally. Suitable for callers
    /// (e.g. `sindri apply`) that don't otherwise need to manage an OCI
    /// client lifecycle.
    pub async fn verify_component_signature_default_client(
        &self,
        registry_name: &str,
        oci_ref: &OciRef,
        component_digest: &str,
        policy_requires_signing: bool,
    ) -> Result<String, RegistryError> {
        let oci = OciClient::new(oci_client::client::ClientConfig::default());
        self.verify_component_signature(
            &oci,
            registry_name,
            oci_ref,
            component_digest,
            policy_requires_signing,
        )
        .await
    }

    fn key_ids_for(&self, registry_name: &str) -> Vec<String> {
        self.keys_for(registry_name)
            .iter()
            .map(|k| k.key_id.clone())
            .collect()
    }
}

/// Compute the cosign signature tag for a given manifest digest.
///
/// `sha256:<hex>` becomes `sha256-<hex>.sig` per the cosign signature spec.
/// Returns `None` if the input is not a valid `<alg>:<hex>` digest string.
pub(crate) fn cosign_signature_tag(manifest_digest: &str) -> Option<String> {
    let (alg, hex) = manifest_digest.split_once(':')?;
    if alg.is_empty() || hex.is_empty() {
        return None;
    }
    Some(format!("{}-{}.sig", alg, hex))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ecdsa::signature::Signer;
    use p256::ecdsa::SigningKey;
    use p256::pkcs8::EncodePublicKey;
    use rand_core::OsRng;
    use std::fs;
    use tempfile::TempDir;

    fn write_test_key(dir: &Path, registry: &str, key_idx: usize) -> String {
        let registry_dir = dir.join(registry);
        fs::create_dir_all(&registry_dir).unwrap();
        let signing = SigningKey::random(&mut OsRng);
        let verifying = VerifyingKey::from(&signing);
        let pem = verifying
            .to_public_key_pem(p256::pkcs8::LineEnding::LF)
            .unwrap();
        let path = registry_dir.join(format!("cosign-test-{}.pub", key_idx));
        fs::write(&path, &pem).unwrap();
        pem
    }

    /// Build a `(signing_key, verifier_with_one_key)` pair under registry name
    /// `registry`. The verifier mirrors the on-disk shape produced by
    /// `sindri registry trust`.
    fn fixture_verifier(registry: &str) -> (SigningKey, CosignVerifier) {
        let tmp = TempDir::new().unwrap();
        // Persist `tmp` for the lifetime of the verifier by leaking a Box —
        // tests are short-lived processes so this is fine and avoids a
        // lifetime parameter on `fixture_verifier`.
        let dir = Box::leak(Box::new(tmp));
        let registry_dir = dir.path().join(registry);
        fs::create_dir_all(&registry_dir).unwrap();
        let signing = SigningKey::random(&mut OsRng);
        let verifying = VerifyingKey::from(&signing);
        let pem = verifying
            .to_public_key_pem(p256::pkcs8::LineEnding::LF)
            .unwrap();
        fs::write(registry_dir.join("cosign-test-0.pub"), &pem).unwrap();
        let verifier = CosignVerifier::load_from_trust_dir(dir.path()).unwrap();
        (signing, verifier)
    }

    fn simple_signing_payload(manifest_digest: &str) -> Vec<u8> {
        // Cosign payload body — only `critical.image.docker-manifest-digest`
        // is consulted by the verifier; the rest is structural ballast.
        let payload = serde_json::json!({
            "critical": {
                "identity": { "docker-reference": "ghcr.io/example/repo" },
                "image": { "docker-manifest-digest": manifest_digest },
                "type": "cosign container image signature"
            },
            "optional": null
        });
        serde_json::to_vec(&payload).unwrap()
    }

    #[test]
    fn loads_fixture_key() {
        let tmp = TempDir::new().unwrap();
        write_test_key(tmp.path(), "ghcr.io_sindri", 0);
        let verifier = CosignVerifier::load_from_trust_dir(tmp.path()).unwrap();
        let registries: Vec<&str> = verifier.trusted_registries().collect();
        assert_eq!(registries, vec!["ghcr.io_sindri"]);
        let keys = verifier.keys_for("ghcr.io_sindri");
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].key_id.len(), 8);
        assert!(keys[0].spki_pem.contains("BEGIN PUBLIC KEY"));
    }

    #[test]
    fn rejects_malformed_pem() {
        let tmp = TempDir::new().unwrap();
        let registry_dir = tmp.path().join("acme");
        fs::create_dir_all(&registry_dir).unwrap();
        fs::write(
            registry_dir.join("cosign-bad.pub"),
            "-----BEGIN PUBLIC KEY-----\nnot-pem\n-----END PUBLIC KEY-----\n",
        )
        .unwrap();
        let err = CosignVerifier::load_from_trust_dir(tmp.path()).unwrap_err();
        assert!(
            matches!(err, RegistryError::TrustKeyParseFailed { ref path, .. } if path.contains("cosign-bad.pub"))
        );
    }

    #[test]
    fn supports_multiple_keys_per_registry() {
        let tmp = TempDir::new().unwrap();
        write_test_key(tmp.path(), "acme", 0);
        write_test_key(tmp.path(), "acme", 1);
        let verifier = CosignVerifier::load_from_trust_dir(tmp.path()).unwrap();
        let keys = verifier.keys_for("acme");
        assert_eq!(keys.len(), 2);
        assert_ne!(keys[0].key_id, keys[1].key_id);
    }

    #[test]
    fn empty_trust_dir_yields_empty_verifier() {
        let tmp = TempDir::new().unwrap();
        let verifier = CosignVerifier::load_from_trust_dir(tmp.path()).unwrap();
        assert_eq!(verifier.trusted_registries().count(), 0);
    }

    #[test]
    fn nonexistent_root_yields_empty_verifier() {
        let tmp = TempDir::new().unwrap();
        let missing = tmp.path().join("does-not-exist");
        let verifier = CosignVerifier::load_from_trust_dir(&missing).unwrap();
        assert_eq!(verifier.trusted_registries().count(), 0);
    }

    #[test]
    fn ignores_non_cosign_files() {
        let tmp = TempDir::new().unwrap();
        write_test_key(tmp.path(), "acme", 0);
        let registry_dir = tmp.path().join("acme");
        fs::write(registry_dir.join("README.md"), "not a key").unwrap();
        fs::write(registry_dir.join("other.pub"), "not loaded").unwrap();
        let verifier = CosignVerifier::load_from_trust_dir(tmp.path()).unwrap();
        assert_eq!(verifier.keys_for("acme").len(), 1);
    }

    // -- Wave 3A.2: verify_payload pure-function tests -----------------------

    #[test]
    fn verify_succeeds_with_test_signature_against_trusted_key() {
        let (signing, verifier) = fixture_verifier("ghcr.io_sindri-dev");
        let manifest_digest = format!("sha256:{}", "a".repeat(64));
        let payload = simple_signing_payload(&manifest_digest);
        // Sign the canonical payload bytes with the test key.
        let sig: Signature = signing.sign(&payload);
        let sig_bytes = sig.to_der().as_bytes().to_vec();
        let key_id = verifier
            .verify_payload(
                "ghcr.io_sindri-dev",
                &payload,
                &sig_bytes,
                &manifest_digest,
                true,
            )
            .expect("verification should succeed against the trusted key");
        // The matched key id should be the only one in the trust set.
        assert_eq!(key_id, verifier.keys_for("ghcr.io_sindri-dev")[0].key_id);
    }

    #[test]
    fn verify_fails_with_wrong_payload_digest() {
        let (signing, verifier) = fixture_verifier("ghcr.io_sindri-dev");
        let real_digest = format!("sha256:{}", "a".repeat(64));
        let tampered_digest = format!("sha256:{}", "b".repeat(64));
        // Build a payload where the docker-manifest-digest claims a
        // different artifact than the one we say we expect. The signature
        // itself is over the tampered payload (so the crypto is valid) but
        // the payload→expected mismatch must be caught.
        let payload = simple_signing_payload(&real_digest);
        let sig: Signature = signing.sign(&payload);
        let sig_bytes = sig.to_der().as_bytes().to_vec();
        let err = verifier
            .verify_payload(
                "ghcr.io_sindri-dev",
                &payload,
                &sig_bytes,
                &tampered_digest,
                true,
            )
            .unwrap_err();
        assert!(
            matches!(err, RegistryError::SignatureMismatch { ref detail, .. } if detail.contains("payload digest")),
            "expected SignatureMismatch with payload-digest detail, got {:?}",
            err
        );
    }

    #[test]
    fn verify_fails_with_wrong_key() {
        // Trust set contains key A; we sign with key B.
        let (_signing_a, verifier) = fixture_verifier("acme");
        let signing_b = SigningKey::random(&mut OsRng);
        let manifest_digest = format!("sha256:{}", "c".repeat(64));
        let payload = simple_signing_payload(&manifest_digest);
        let sig: Signature = signing_b.sign(&payload);
        let sig_bytes = sig.to_der().as_bytes().to_vec();
        let err = verifier
            .verify_payload("acme", &payload, &sig_bytes, &manifest_digest, true)
            .unwrap_err();
        assert!(
            matches!(err, RegistryError::SignatureMismatch { ref detail, .. } if detail.contains("no trusted key matched")),
            "expected SignatureMismatch with 'no trusted key matched', got {:?}",
            err
        );
    }

    #[test]
    fn strict_policy_no_keys_fails() {
        let tmp = TempDir::new().unwrap();
        let verifier = CosignVerifier::load_from_trust_dir(tmp.path()).unwrap();
        let digest = format!("sha256:{}", "0".repeat(64));
        let payload = simple_signing_payload(&digest);
        let err = verifier
            .verify_payload("nope", &payload, &[0u8; 64], &digest, true)
            .unwrap_err();
        assert!(
            matches!(err, RegistryError::SignatureRequired { .. }),
            "expected SignatureRequired in strict mode with no keys, got {:?}",
            err
        );
    }

    #[test]
    fn permissive_policy_no_keys_warns_only() {
        let tmp = TempDir::new().unwrap();
        let verifier = CosignVerifier::load_from_trust_dir(tmp.path()).unwrap();
        let digest = format!("sha256:{}", "0".repeat(64));
        let payload = simple_signing_payload(&digest);
        // Garbage signature bytes — but with no keys + permissive policy we
        // never get to the crypto check, so this should succeed with the
        // sentinel `<unsigned>` key id.
        let key_id = verifier
            .verify_payload("nope", &payload, &[0u8; 64], &digest, false)
            .unwrap();
        assert_eq!(key_id, "<unsigned>");
    }

    #[test]
    fn cosign_signature_tag_round_trip() {
        let digest = "sha256:abcdef0123456789";
        assert_eq!(
            cosign_signature_tag(digest),
            Some("sha256-abcdef0123456789.sig".to_string())
        );
        assert_eq!(cosign_signature_tag("not-a-digest"), None);
        assert_eq!(cosign_signature_tag(":empty-alg"), None);
        assert_eq!(cosign_signature_tag("sha256:"), None);
    }
}
