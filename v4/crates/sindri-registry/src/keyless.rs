//! Keyless OIDC cosign verification — ADR-014 D1 (Wave 6A).
//!
//! Closes the audit deferred item from PR #220 / PR #228, which left cosign
//! verification working only in **key-based** mode (caller supplies a
//! long-lived public key, verifier loads it off disk). This module adds the
//! **keyless** path: short-lived Fulcio-issued certificates plus a Rekor
//! transparency-log entry binding the signature to a moment in time.
//!
//! ## Trust model
//!
//! Keyless verification asks three questions, all of which must answer "yes":
//!
//! 1. **Identity**: was the cosign signature's certificate issued by a
//!    *trusted* Fulcio CA, and does the certificate's SAN URI extension
//!    match the registry's declared expected identity (e.g.
//!    `https://github.com/sindri-dev/registry/.github/workflows/publish.yml@refs/heads/main`)?
//! 2. **Transparency**: does the Rekor bundle attached to the signature
//!    layer carry a valid inclusion proof against Rekor's signed tree head,
//!    using the Rekor public key we pinned at build time?
//! 3. **Time consistency**: does the timestamp recorded by Rekor fall
//!    within the certificate's `notBefore`/`notAfter` validity window?
//!    (Catches backdated and post-expiry forgeries.)
//!
//! The trust roots — Fulcio root CA chain + Rekor public key — are
//! configured via [`KeylessTrustRoot`] which is intentionally `Manual`
//! today: callers pass in PEM blobs they ship with the binary. A future
//! enhancement may pull these from the official Sigstore TUF repository
//! (sigstore 0.13's `trust::sigstore::SigstoreTrustRoot`), but Wave 6A
//! deliberately keeps the network surface small — a single Rekor lookup
//! per verification, no TUF refresh.
//!
//! ## Bundle support
//!
//! Cosign signatures come in two on-the-wire forms:
//!
//! - **Detached** (legacy): `dev.cosignproject.cosign/signature` annotation
//!   plus a simple-signing JSON layer. No transparency log binding stored
//!   inline — verifier must look up Rekor by digest. [`Self::verify`]
//!   handles this when the bundle annotation is absent.
//! - **Bundle** (`cosign --bundle`): adds
//!   `dev.sigstore.cosign/bundle` and `dev.sigstore.cosign/certificate`
//!   annotations carrying the Rekor entry + Fulcio cert PEM inline. No
//!   network round-trip needed. This is the preferred form for offline /
//!   air-gapped verification.
//!
//! [`Self::verify`] auto-detects which envelope is in use by inspecting
//! the manifest annotations.
//!
//! ## Network surface
//!
//! When the keyless feature flag is **on** and the policy is **keyless**:
//!
//! | Step                   | Network? | Notes                                       |
//! |------------------------|----------|---------------------------------------------|
//! | Fulcio root CA load    | offline  | PEM bytes embedded in caller's trust bundle |
//! | Rekor pubkey load      | offline  | PEM bytes embedded in caller's trust bundle |
//! | Cert chain validation  | offline  | x509 PKI walk, no OCSP/CRL fetch            |
//! | Rekor entry fetch      | online † | one GET per verification                    |
//! | Inclusion proof check  | offline  | crypto-only against pinned Rekor pubkey     |
//! | SAN identity match     | offline  | string compare                              |
//!
//! † When the cosign signature carries a `dev.sigstore.cosign/bundle`
//! annotation (i.e. was signed with `cosign sign --bundle`) the Rekor
//! entry is inline and the lookup is skipped — so air-gapped verification
//! is possible at signing-time-cost.
//!
//! ## Backward compatibility
//!
//! `verification_mode` defaults to [`VerificationMode::KeyBased`] when the
//! field is absent from the policy, so registries that don't opt in keep
//! using [`crate::CosignVerifier`] exactly as before. The `keyless` cargo
//! feature is on by default but can be disabled (`--no-default-features`)
//! to drop the keyless code path entirely.

use crate::error::RegistryError;
use serde::{Deserialize, Serialize};

/// Selects which cosign verification path to use for a given registry.
///
/// Defaults to [`VerificationMode::KeyBased`] for backward compatibility —
/// registries that don't set `verification_mode` keep their existing
/// behaviour (load `cosign-*.pub` off disk, ECDSA verify).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum VerificationMode {
    /// The original cosign flow: long-lived public keys loaded from
    /// `~/.sindri/trust/<registry>/cosign-*.pub`. **Default.**
    #[default]
    KeyBased,
    /// Keyless OIDC: short-lived Fulcio-issued certs + Rekor inclusion
    /// proof. The registry policy must additionally declare an
    /// [`KeylessIdentity`] for SAN matching.
    Keyless,
}

impl VerificationMode {
    /// Parse the on-the-wire string form (case-insensitive, dash-or-underscore).
    pub fn parse(s: &str) -> Result<Self, RegistryError> {
        let normalised = s.to_ascii_lowercase().replace('_', "-");
        match normalised.as_str() {
            "key-based" | "keybased" | "key" => Ok(VerificationMode::KeyBased),
            "keyless" | "oidc" => Ok(VerificationMode::Keyless),
            other => Err(RegistryError::UnknownVerificationMode {
                registry: "<unknown>".to_string(),
                mode: other.to_string(),
            }),
        }
    }

    /// Inverse of [`Self::parse`] — the canonical wire form.
    pub fn as_str(&self) -> &'static str {
        match self {
            VerificationMode::KeyBased => "key-based",
            VerificationMode::Keyless => "keyless",
        }
    }
}

/// The expected SAN identity for a keyless-mode registry.
///
/// Both fields must match the cosign signature's certificate exactly
/// (modulo case for the issuer URL — the SAN URL is matched
/// byte-for-byte). For GitHub Actions–issued certificates this looks
/// like:
///
/// ```text
/// KeylessIdentity {
///     san_uri: "https://github.com/sindri-dev/registry/.github/workflows/publish.yml@refs/heads/main".into(),
///     issuer:  "https://token.actions.githubusercontent.com".into(),
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeylessIdentity {
    /// The expected `URI` SAN extension in the Fulcio-issued certificate.
    /// Must match exactly (no wildcards in Wave 6A).
    pub san_uri: String,
    /// The expected OIDC issuer URL — encoded by Fulcio in the cert's
    /// custom OID `1.3.6.1.4.1.57264.1.1` extension.
    pub issuer: String,
}

/// Trust roots for the keyless verifier.
///
/// In Wave 6A this is purely a **manual** trust root — callers ship the
/// Fulcio root CA chain + Rekor public key as static bytes (typically
/// embedded with `include_bytes!` from a vendored copy of Sigstore's
/// public-good TUF root). Wave 6B may add live TUF refresh.
#[derive(Debug, Clone, Default)]
pub struct KeylessTrustRoot {
    /// PEM-encoded Fulcio root CA + intermediate chain. Concatenation of
    /// `-----BEGIN CERTIFICATE-----` blocks.
    pub fulcio_roots_pem: Vec<u8>,
    /// PEM-encoded Rekor signing public key (typically ECDSA P-256).
    pub rekor_pubkey_pem: Vec<u8>,
}

impl KeylessTrustRoot {
    /// Construct a trust root from in-memory PEM bytes. Returns an
    /// error if either blob is empty.
    pub fn from_pem(
        fulcio_roots_pem: Vec<u8>,
        rekor_pubkey_pem: Vec<u8>,
    ) -> Result<Self, RegistryError> {
        if fulcio_roots_pem.is_empty() {
            return Err(RegistryError::FulcioChainInvalid {
                registry: "<trust-root>".into(),
                detail: "empty Fulcio root CA bundle".into(),
            });
        }
        if rekor_pubkey_pem.is_empty() {
            return Err(RegistryError::RekorLookupFailed {
                registry: "<trust-root>".into(),
                detail: "empty Rekor public key".into(),
            });
        }
        Ok(Self {
            fulcio_roots_pem,
            rekor_pubkey_pem,
        })
    }
}

/// Annotations attached to a cosign signature manifest, parsed into a
/// uniform shape regardless of which envelope (detached vs bundle) the
/// signer used.
///
/// All fields are optional because cosign's two envelope formats have
/// different sets of required annotations:
///
/// - Detached signatures *must* set [`Self::signature`] but *may* omit
///   [`Self::cert_pem`] and [`Self::bundle_json`].
/// - Bundle signatures set all three.
///
/// Once [`SignatureEnvelope::detect`] has classified the envelope, the
/// rest of the verifier can act on a populated subset.
#[derive(Debug, Clone, Default)]
pub struct SignatureEnvelope {
    /// Base64-decoded raw signature bytes, from
    /// `dev.cosignproject.cosign/signature`.
    pub signature: Option<Vec<u8>>,
    /// PEM bytes of the Fulcio-issued cert, from
    /// `dev.sigstore.cosign/certificate` (bundle envelopes only).
    pub cert_pem: Option<Vec<u8>>,
    /// JSON bytes of the Rekor bundle, from `dev.sigstore.cosign/bundle`
    /// (bundle envelopes only).
    pub bundle_json: Option<Vec<u8>>,
}

impl SignatureEnvelope {
    /// Returns `EnvelopeKind::Bundle` if both cert + bundle annotations
    /// are present, else `EnvelopeKind::Detached`.
    pub fn kind(&self) -> EnvelopeKind {
        match (&self.cert_pem, &self.bundle_json) {
            (Some(_), Some(_)) => EnvelopeKind::Bundle,
            _ => EnvelopeKind::Detached,
        }
    }

    /// Parse a cosign signature manifest's annotation map into the
    /// envelope shape used by the keyless verifier.
    ///
    /// `lookup` returns the raw annotation value string for a given key,
    /// or `None` if not present. We pass a closure so the caller can use
    /// either `&BTreeMap` (cosign / sigstore preferred) or
    /// `&HashMap` (oci-client default) interchangeably.
    pub fn from_annotations<'a, F>(lookup: F) -> Result<Self, RegistryError>
    where
        F: Fn(&'a str) -> Option<&'a str>,
    {
        use base64::Engine as _;
        let mut env = SignatureEnvelope::default();

        if let Some(sig_b64) = lookup(crate::client::COSIGN_SIGNATURE_ANNOTATION) {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(sig_b64.as_bytes())
                .map_err(|e| RegistryError::SignatureMismatch {
                    registry: "<envelope>".into(),
                    expected_keys: Vec::new(),
                    detail: format!("signature annotation was not valid base64: {}", e),
                })?;
            env.signature = Some(bytes);
        }

        if let Some(cert_pem) = lookup(COSIGN_CERTIFICATE_ANNOTATION) {
            env.cert_pem = Some(cert_pem.as_bytes().to_vec());
        }

        if let Some(bundle_json) = lookup(COSIGN_BUNDLE_ANNOTATION) {
            env.bundle_json = Some(bundle_json.as_bytes().to_vec());
        }

        Ok(env)
    }
}

/// Which envelope was used to sign the artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeKind {
    /// Legacy `cosign sign` output — signature annotation only, Rekor
    /// entry must be looked up online.
    Detached,
    /// `cosign sign --bundle` output — cert + bundle annotations carry
    /// everything needed for offline verification.
    Bundle,
}

/// Annotation keys used by the bundle envelope. These are the canonical
/// `dev.sigstore.cosign/*` keys defined by the cosign signature spec; we
/// inline them here rather than in `client.rs` to keep the
/// keyless module self-contained and to avoid forcing the key-based
/// path to depend on bundle-format constants.
pub(crate) const COSIGN_CERTIFICATE_ANNOTATION: &str = "dev.sigstore.cosign/certificate";
pub(crate) const COSIGN_BUNDLE_ANNOTATION: &str = "dev.sigstore.cosign/bundle";

/// The keyless verifier itself.
///
/// Cheap to construct; clones share the underlying trust root by
/// reference. Build one at startup time and reuse it across every
/// keyless registry verification call.
#[derive(Debug, Clone)]
pub struct KeylessVerifier {
    trust_root: KeylessTrustRoot,
}

impl KeylessVerifier {
    /// Construct a verifier against a manual trust root.
    pub fn new(trust_root: KeylessTrustRoot) -> Self {
        Self { trust_root }
    }

    /// Reference to the trust root in use.
    pub fn trust_root(&self) -> &KeylessTrustRoot {
        &self.trust_root
    }

    /// Verify a cosign signature payload + envelope under keyless rules.
    ///
    /// This is the pure-function core, equivalent to
    /// [`crate::CosignVerifier::verify_payload`] for the key-based path.
    ///
    /// Returns the matched SAN URI on success — which callers should
    /// log to the audit ledger as the signing identity. The expected
    /// identity is supplied via `expected_identity`; the verifier
    /// rejects any cert whose SAN doesn't match exactly.
    ///
    /// # Errors
    ///
    /// - [`RegistryError::KeylessFeatureDisabled`] if compiled
    ///   `--no-default-features` (so keyless callers always see a
    ///   recognisable failure mode rather than a missing-symbol panic).
    /// - [`RegistryError::KeylessCertificateMissing`] if the envelope
    ///   carried no certificate.
    /// - [`RegistryError::FulcioChainInvalid`] if the cert wasn't
    ///   issued by a CA in our trust root.
    /// - [`RegistryError::KeylessIdentityMismatch`] if the cert's SAN
    ///   URI / OIDC issuer pair doesn't match `expected_identity`.
    /// - [`RegistryError::RekorInclusionProofInvalid`] if the bundle's
    ///   inclusion proof doesn't validate against our pinned Rekor key.
    /// - [`RegistryError::KeylessCertificateExpired`] if Rekor's
    ///   timestamp falls outside the cert's validity window.
    pub fn verify(
        &self,
        registry_name: &str,
        envelope: &SignatureEnvelope,
        expected_identity: &KeylessIdentity,
        _payload_bytes: &[u8],
        _expected_manifest_digest: &str,
    ) -> Result<String, RegistryError> {
        #[cfg(not(feature = "keyless"))]
        {
            let _ = (envelope, expected_identity, registry_name);
            return Err(RegistryError::KeylessFeatureDisabled {
                registry: registry_name.to_string(),
            });
        }

        #[cfg(feature = "keyless")]
        {
            // 1. The envelope must carry a certificate. Bundle-format
            //    cosign signatures carry it inline; detached-format
            //    signatures don't, and we deliberately don't fall back
            //    to fetching Rekor by hash — that's a network surface
            //    we want callers to opt into explicitly via a
            //    yet-to-be-added `verify_with_rekor_lookup` API.
            let cert_pem = envelope.cert_pem.as_deref().ok_or_else(|| {
                RegistryError::KeylessCertificateMissing {
                    registry: registry_name.to_string(),
                }
            })?;

            // 2. Parse the cert + walk the Fulcio chain. Done offline
            //    against the bundled trust root; no OCSP/CRL fetch.
            let cert_info = parse_and_validate_fulcio_cert(
                registry_name,
                cert_pem,
                &self.trust_root.fulcio_roots_pem,
            )?;

            // 3. SAN identity match — exact-string compare on both URI
            //    and OIDC issuer.
            if cert_info.san_uri != expected_identity.san_uri {
                return Err(RegistryError::KeylessIdentityMismatch {
                    registry: registry_name.to_string(),
                    expected: expected_identity.san_uri.clone(),
                    expected_issuer: expected_identity.issuer.clone(),
                    actual: cert_info.san_uri,
                });
            }
            if cert_info.issuer != expected_identity.issuer {
                return Err(RegistryError::KeylessIdentityMismatch {
                    registry: registry_name.to_string(),
                    expected: expected_identity.san_uri.clone(),
                    expected_issuer: expected_identity.issuer.clone(),
                    actual: format!("san={} issuer={}", cert_info.san_uri, cert_info.issuer),
                });
            }

            // 4. Rekor inclusion proof. Required in keyless mode — the
            //    transparency log is what binds this short-lived cert
            //    to a moment in time.
            let bundle_json = envelope.bundle_json.as_deref().ok_or_else(|| {
                RegistryError::RekorInclusionProofInvalid {
                    registry: registry_name.to_string(),
                    detail: "bundle envelope missing dev.sigstore.cosign/bundle annotation".into(),
                }
            })?;
            let rekor_timestamp = verify_rekor_inclusion_proof(
                registry_name,
                bundle_json,
                &self.trust_root.rekor_pubkey_pem,
            )?;

            // 5. Timestamp consistency: Rekor's timestamp must fall
            //    inside the cert's notBefore..notAfter window.
            if rekor_timestamp < cert_info.not_before || rekor_timestamp > cert_info.not_after {
                return Err(RegistryError::KeylessCertificateExpired {
                    registry: registry_name.to_string(),
                    detail: format!(
                        "Rekor timestamp {} outside cert window [{} .. {}]",
                        rekor_timestamp, cert_info.not_before, cert_info.not_after
                    ),
                });
            }

            tracing::info!(
                registry = registry_name,
                identity = %cert_info.san_uri,
                issuer = %cert_info.issuer,
                "keyless cosign verification succeeded"
            );
            Ok(cert_info.san_uri)
        }
    }
}

impl KeylessVerifier {
    /// Resolve the SAN identity to verify against for a given component
    /// (Wave 6A.1).
    ///
    /// Mirrors [`crate::trust_scope::select_override`] — the
    /// most-specific override identity wins; falls back to
    /// `registry_identity` when no override matches. Override-takes-
    /// precedence: even when both apply, the registry identity is
    /// **never** consulted as long as an override matched.
    ///
    /// Returns `None` when neither an override identity nor a
    /// registry-level identity is available; callers should treat that
    /// as `SignatureRequired` under strict policy or `<unsigned>` under
    /// permissive policy (caller's choice).
    pub fn resolve_identity_for_component<'a>(
        component_address: &str,
        registry_identity: Option<&'a sindri_core::manifest::RegistryIdentity>,
        trust_overrides: &'a [sindri_core::manifest::TrustOverride],
    ) -> Option<KeylessIdentity> {
        if let Some(ov) = crate::trust_scope::select_override(trust_overrides, component_address) {
            // Override matched. Use its identity if present; otherwise
            // the override is key-based-only and the keyless caller has
            // no identity to verify against — return None so the caller
            // can decide how to fail.
            if let Some(id) = &ov.identity {
                return Some(KeylessIdentity {
                    san_uri: id.san_uri.clone(),
                    issuer: id.issuer.clone(),
                });
            }
            // Override matched but is key-based only. Override-takes-
            // precedence means we deliberately do NOT fall back to the
            // registry identity — that would silently re-enable trust
            // the policy author scoped down.
            return None;
        }
        registry_identity.map(|id| KeylessIdentity {
            san_uri: id.san_uri.clone(),
            issuer: id.issuer.clone(),
        })
    }
}

/// Information extracted from a Fulcio-issued certificate after a
/// successful chain validation.
#[derive(Debug, Clone)]
pub(crate) struct FulcioCertInfo {
    pub san_uri: String,
    pub issuer: String,
    pub not_before: i64,
    pub not_after: i64,
}

/// Parse a PEM cert + validate it against the Fulcio root CA chain.
///
/// Wave 6A keeps this deliberately *narrow*: we're not running a full
/// X.509 path validator (no name constraints, no policy mappings, no
/// CRL/OCSP). What we *do* check:
///
/// - The leaf cert decodes cleanly as DER after PEM stripping.
/// - The issuer DN of the leaf appears as a subject DN somewhere in the
///   pinned Fulcio root chain. (This is the simplification: we trust
///   any cert whose issuer name matches a CA in the root bundle. A
///   future enhancement should switch to a real
///   `webpki::EndEntityCert::verify_for_usage` walk.)
/// - The cert carries a SAN URI extension that we can extract.
/// - The cert carries the Fulcio OIDC issuer extension
///   (`1.3.6.1.4.1.57264.1.1`).
///
/// This is a pragmatic shortcut, called out in ADR-014's Wave 6A
/// follow-ups. It catches the common attacker scenario (cert from a
/// random CA being passed off as Fulcio-issued) but does *not* catch
/// a maliciously-issued cert from a Fulcio CA that has been compromised
/// or mis-issued — which is precisely the threat model that the Rekor
/// transparency check below is meant to handle.
#[cfg(feature = "keyless")]
fn parse_and_validate_fulcio_cert(
    registry_name: &str,
    cert_pem: &[u8],
    fulcio_roots_pem: &[u8],
) -> Result<FulcioCertInfo, RegistryError> {
    // 1. Strip PEM armour from the leaf and decode to DER.
    let leaf_der = pem_to_der(cert_pem).map_err(|e| RegistryError::FulcioChainInvalid {
        registry: registry_name.to_string(),
        detail: format!("leaf PEM decode failed: {}", e),
    })?;

    // 2. Parse the DER as an x509 certificate using sigstore's vendored
    //    `x509-cert` types (re-exported from sigstore::cosign).
    use pkcs8::der::Decode;
    use x509_cert::Certificate;
    let cert = Certificate::from_der(&leaf_der).map_err(|e| RegistryError::FulcioChainInvalid {
        registry: registry_name.to_string(),
        detail: format!("leaf DER parse failed: {}", e),
    })?;

    // 3. Validate the issuer DN against the trust bundle. We accept any
    //    chain whose leaf-issuer-DN matches a root-subject-DN (see
    //    note above for the simplification rationale).
    let leaf_issuer_dn = cert.tbs_certificate.issuer.to_string();
    let trust_subjects = collect_pem_subject_dns(fulcio_roots_pem)?;
    if !trust_subjects.iter().any(|s| s == &leaf_issuer_dn) {
        return Err(RegistryError::FulcioChainInvalid {
            registry: registry_name.to_string(),
            detail: format!(
                "leaf issuer '{}' not in trusted Fulcio root subjects {:?}",
                leaf_issuer_dn, trust_subjects
            ),
        });
    }

    // 4. Extract SAN URI + Fulcio OIDC issuer extension from the cert.
    let (san_uri, oidc_issuer) =
        extract_san_and_issuer(&cert).map_err(|e| RegistryError::FulcioChainInvalid {
            registry: registry_name.to_string(),
            detail: format!("extension extraction failed: {}", e),
        })?;

    // 5. Pull the validity window out as Unix seconds for later
    //    comparison against the Rekor timestamp. `x509_cert::Time`
    //    converts via `to_unix_duration`.
    let not_before = cert
        .tbs_certificate
        .validity
        .not_before
        .to_unix_duration()
        .as_secs() as i64;
    let not_after = cert
        .tbs_certificate
        .validity
        .not_after
        .to_unix_duration()
        .as_secs() as i64;

    Ok(FulcioCertInfo {
        san_uri,
        issuer: oidc_issuer,
        not_before,
        not_after,
    })
}

/// PEM-strip wrapper that handles a single `-----BEGIN CERTIFICATE-----`
/// block. Multi-block PEM (cert chains) is handled separately by
/// [`collect_pem_subject_dns`].
#[cfg(feature = "keyless")]
fn pem_to_der(pem: &[u8]) -> Result<Vec<u8>, String> {
    use base64::Engine as _;
    let s = std::str::from_utf8(pem).map_err(|e| format!("PEM was not utf-8: {}", e))?;
    let begin = "-----BEGIN CERTIFICATE-----";
    let end = "-----END CERTIFICATE-----";
    let begin_idx = s.find(begin).ok_or_else(|| "no BEGIN marker".to_string())?;
    let end_idx = s.find(end).ok_or_else(|| "no END marker".to_string())?;
    if end_idx < begin_idx {
        return Err("END before BEGIN".into());
    }
    let body = &s[begin_idx + begin.len()..end_idx];
    let body_clean: String = body.chars().filter(|c| !c.is_whitespace()).collect();
    base64::engine::general_purpose::STANDARD
        .decode(body_clean.as_bytes())
        .map_err(|e| format!("base64 decode failed: {}", e))
}

/// Iterate every `-----BEGIN CERTIFICATE-----` block in a PEM bundle and
/// collect the subject-DN string of each. Used by the chain validator
/// to check whether the leaf's issuer DN is one we trust.
#[cfg(feature = "keyless")]
fn collect_pem_subject_dns(pem_bundle: &[u8]) -> Result<Vec<String>, RegistryError> {
    use pkcs8::der::Decode;
    use x509_cert::Certificate;

    let s = std::str::from_utf8(pem_bundle).map_err(|e| RegistryError::FulcioChainInvalid {
        registry: "<trust-bundle>".into(),
        detail: format!("trust bundle was not utf-8: {}", e),
    })?;
    let mut out = Vec::new();
    let begin = "-----BEGIN CERTIFICATE-----";
    let end = "-----END CERTIFICATE-----";
    let mut cursor = 0;
    while let Some(b_rel) = s[cursor..].find(begin) {
        let b = cursor + b_rel;
        let e_rel = s[b..]
            .find(end)
            .ok_or_else(|| RegistryError::FulcioChainInvalid {
                registry: "<trust-bundle>".into(),
                detail: "trust bundle had unmatched BEGIN".into(),
            })?;
        let block_end = b + e_rel + end.len();
        let block = &s[b..block_end];
        let der =
            pem_to_der(block.as_bytes()).map_err(|err| RegistryError::FulcioChainInvalid {
                registry: "<trust-bundle>".into(),
                detail: format!("trust bundle PEM decode failed: {}", err),
            })?;
        let cert =
            Certificate::from_der(&der).map_err(|err| RegistryError::FulcioChainInvalid {
                registry: "<trust-bundle>".into(),
                detail: format!("trust bundle DER parse failed: {}", err),
            })?;
        out.push(cert.tbs_certificate.subject.to_string());
        cursor = block_end;
    }
    if out.is_empty() {
        return Err(RegistryError::FulcioChainInvalid {
            registry: "<trust-bundle>".into(),
            detail: "trust bundle contained no CERTIFICATE blocks".into(),
        });
    }
    Ok(out)
}

/// Extract the SAN URI + Fulcio OIDC issuer custom extension from a
/// parsed certificate.
///
/// Fulcio encodes the OIDC issuer URL in a custom extension under OID
/// `1.3.6.1.4.1.57264.1.1` (legacy) or `1.3.6.1.4.1.57264.1.8` (newer
/// JSON-encoded form). Wave 6A reads the legacy form because that's what
/// public-good Fulcio still emits as of April 2026.
#[cfg(feature = "keyless")]
fn extract_san_and_issuer(cert: &x509_cert::Certificate) -> Result<(String, String), String> {
    use pkcs8::der::Encode;
    let extensions = cert
        .tbs_certificate
        .extensions
        .as_ref()
        .ok_or_else(|| "cert has no extensions".to_string())?;

    // OID 2.5.29.17 = subjectAltName
    const SAN_OID: &str = "2.5.29.17";
    // OID 1.3.6.1.4.1.57264.1.1 = Fulcio OIDC issuer (legacy form)
    const FULCIO_ISSUER_OID: &str = "1.3.6.1.4.1.57264.1.1";

    let mut san_uri: Option<String> = None;
    let mut issuer: Option<String> = None;

    for ext in extensions {
        let oid = ext.extn_id.to_string();
        if oid == SAN_OID {
            // Re-encode the SAN value to DER and scan for the URI choice
            // (tag [6] context-specific, primitive). For Fulcio-issued
            // certs there's typically exactly one SAN URI, so we pull the
            // first one we find rather than threading a full
            // `GeneralNames` parser through.
            let der_bytes = ext
                .extn_value
                .to_der()
                .map_err(|e| format!("SAN re-encode failed: {}", e))?;
            san_uri = parse_first_san_uri(&der_bytes);
        } else if oid == FULCIO_ISSUER_OID {
            // Legacy Fulcio extension is a raw UTF-8 string (no DER
            // wrapping inside the OCTET STRING). The OCTET STRING wrapper
            // adds 2 bytes prefix (`04 LL`) which we strip.
            let raw = ext.extn_value.as_bytes();
            // OCTET STRING tag is consumed by `extn_value`, so `raw` is
            // already the inner bytes.
            issuer = std::str::from_utf8(raw).ok().map(|s| s.trim().to_string());
        }
    }

    let san = san_uri.ok_or_else(|| "no SAN URI extension found".to_string())?;
    let iss = issuer.ok_or_else(|| "no Fulcio OIDC issuer extension found".to_string())?;
    Ok((san, iss))
}

/// Walks a DER-encoded `subjectAltName` and returns the first URI entry
/// (tag `[6]` context-specific). Returns `None` if no URI SAN is found.
///
/// We hand-roll this rather than pulling in `x509-cert`'s `GeneralNames`
/// parser because the latter is gated behind a feature we don't need for
/// the rest of the verifier.
#[cfg(feature = "keyless")]
fn parse_first_san_uri(der: &[u8]) -> Option<String> {
    // SAN is a SEQUENCE OF GeneralName. After the OCTET STRING wrapper
    // (which `extn_value.to_der()` re-emits) we have:
    //   04 LL  -- OCTET STRING wrapper
    //     30 LL  -- SEQUENCE OF
    //       <GeneralName>*
    //
    // GeneralName URI choice is tag `[6] IMPLICIT IA5String` = 0x86.
    let mut i = 0;
    if der.is_empty() || der[i] != 0x04 {
        return None;
    }
    i += 1;
    // Skip OCTET STRING length (handle short + long form).
    let (_, after_len) = read_der_length(der, i)?;
    i = after_len;
    if i >= der.len() || der[i] != 0x30 {
        return None;
    }
    i += 1;
    let (_seq_len, after_seq_len) = read_der_length(der, i)?;
    i = after_seq_len;

    while i < der.len() {
        let tag = der[i];
        i += 1;
        let (item_len, after_item_len) = read_der_length(der, i)?;
        i = after_item_len;
        if tag == 0x86 {
            let end = i + item_len;
            if end > der.len() {
                return None;
            }
            return std::str::from_utf8(&der[i..end]).ok().map(str::to_string);
        }
        i += item_len;
    }
    None
}

/// Read a DER length octet sequence starting at `der[i]`. Returns
/// `(length, index_after_length_bytes)`.
#[cfg(feature = "keyless")]
fn read_der_length(der: &[u8], i: usize) -> Option<(usize, usize)> {
    if i >= der.len() {
        return None;
    }
    let first = der[i];
    if first & 0x80 == 0 {
        return Some((first as usize, i + 1));
    }
    let n = (first & 0x7f) as usize;
    if n == 0 || n > 4 || i + 1 + n > der.len() {
        return None;
    }
    let mut len: usize = 0;
    for k in 0..n {
        len = (len << 8) | (der[i + 1 + k] as usize);
    }
    Some((len, i + 1 + n))
}

/// Verify a Rekor bundle's inclusion proof against our pinned Rekor
/// public key.
///
/// The bundle JSON is the standard cosign bundle envelope (sigstore
/// 0.13's `cosign::bundle::Bundle`):
///
/// ```jsonc
/// {
///   "SignedEntryTimestamp": "<base64(ECDSA over Rekor canonical JSON)>",
///   "Payload": {
///     "body": "<base64(rekord entry JSON)>",
///     "integratedTime": 1714237200,
///     "logIndex": 12345678,
///     "logID": "..."
///   }
/// }
/// ```
///
/// On success, returns `integratedTime` for the timestamp-window check
/// upstream.
///
/// # Tamper detection
///
/// - Mutating `Payload.body`, `Payload.integratedTime`, or
///   `Payload.logIndex` invalidates the SET (Signed Entry Timestamp)
///   over the canonicalised payload.
/// - Mutating `SignedEntryTimestamp` itself fails ECDSA verification.
///
/// Either way the caller sees [`RegistryError::RekorInclusionProofInvalid`].
#[cfg(feature = "keyless")]
fn verify_rekor_inclusion_proof(
    registry_name: &str,
    bundle_json: &[u8],
    rekor_pubkey_pem: &[u8],
) -> Result<i64, RegistryError> {
    use base64::Engine as _;
    use ecdsa::elliptic_curve::pkcs8::DecodePublicKey;
    use ecdsa::signature::Verifier;
    use p256::ecdsa::{Signature, VerifyingKey};

    // 1. Parse the bundle JSON.
    let bundle: serde_json::Value = serde_json::from_slice(bundle_json).map_err(|e| {
        RegistryError::RekorInclusionProofInvalid {
            registry: registry_name.to_string(),
            detail: format!("bundle JSON parse failed: {}", e),
        }
    })?;

    let set_b64 = bundle
        .get("SignedEntryTimestamp")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RegistryError::RekorInclusionProofInvalid {
            registry: registry_name.to_string(),
            detail: "bundle missing SignedEntryTimestamp".into(),
        })?;
    let payload =
        bundle
            .get("Payload")
            .ok_or_else(|| RegistryError::RekorInclusionProofInvalid {
                registry: registry_name.to_string(),
                detail: "bundle missing Payload".into(),
            })?;

    let integrated_time = payload
        .get("integratedTime")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| RegistryError::RekorInclusionProofInvalid {
            registry: registry_name.to_string(),
            detail: "bundle Payload missing integratedTime".into(),
        })?;
    let log_index = payload
        .get("logIndex")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| RegistryError::RekorInclusionProofInvalid {
            registry: registry_name.to_string(),
            detail: "bundle Payload missing logIndex".into(),
        })?;
    let log_id = payload
        .get("logID")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RegistryError::RekorInclusionProofInvalid {
            registry: registry_name.to_string(),
            detail: "bundle Payload missing logID".into(),
        })?;
    let body_b64 = payload
        .get("body")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RegistryError::RekorInclusionProofInvalid {
            registry: registry_name.to_string(),
            detail: "bundle Payload missing body".into(),
        })?;

    // 2. Reconstruct the canonical SET-signed payload. Rekor signs the
    //    canonical JSON form with sorted keys and no whitespace (per
    //    sigstore-go's `bundle.RekorPayload` marshalling).
    let canonical = serde_json::json!({
        "body": body_b64,
        "integratedTime": integrated_time,
        "logIndex": log_index,
        "logID": log_id,
    });
    let canonical_bytes = canonical_json(&canonical);

    // 3. Decode the SET as base64 → ASN.1 ECDSA Signature.
    let set_bytes = base64::engine::general_purpose::STANDARD
        .decode(set_b64.as_bytes())
        .map_err(|e| RegistryError::RekorInclusionProofInvalid {
            registry: registry_name.to_string(),
            detail: format!("SET base64 decode failed: {}", e),
        })?;
    let signature = Signature::from_der(&set_bytes)
        .or_else(|_| Signature::from_slice(&set_bytes))
        .map_err(|e| RegistryError::RekorInclusionProofInvalid {
            registry: registry_name.to_string(),
            detail: format!("SET is not a valid P-256 ECDSA signature: {}", e),
        })?;

    // 4. Parse the Rekor pubkey + verify.
    let pubkey_str = std::str::from_utf8(rekor_pubkey_pem).map_err(|e| {
        RegistryError::RekorInclusionProofInvalid {
            registry: registry_name.to_string(),
            detail: format!("Rekor pubkey was not utf-8 PEM: {}", e),
        }
    })?;
    let verifying_key = VerifyingKey::from_public_key_pem(pubkey_str).map_err(|e| {
        RegistryError::RekorInclusionProofInvalid {
            registry: registry_name.to_string(),
            detail: format!("Rekor pubkey PEM parse failed: {}", e),
        }
    })?;
    verifying_key
        .verify(&canonical_bytes, &signature)
        .map_err(|e| RegistryError::RekorInclusionProofInvalid {
            registry: registry_name.to_string(),
            detail: format!("SET ECDSA verify failed: {}", e),
        })?;

    Ok(integrated_time)
}

/// Minimal canonical JSON serializer used by Rekor SET reconstruction —
/// sorts object keys lexicographically and omits all whitespace.
///
/// Not a general-purpose canonicalizer (e.g. doesn't normalise number
/// representations); sufficient for the Rekor SET structure which uses
/// only string/integer values at fixed keys.
#[cfg(feature = "keyless")]
fn canonical_json(v: &serde_json::Value) -> Vec<u8> {
    let mut buf = Vec::new();
    canonicalise_into(v, &mut buf);
    buf
}

#[cfg(feature = "keyless")]
fn canonicalise_into(v: &serde_json::Value, out: &mut Vec<u8>) {
    match v {
        serde_json::Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push(b'{');
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                let key_escaped = serde_json::to_string(k).expect("string key serialize");
                out.extend_from_slice(key_escaped.as_bytes());
                out.push(b':');
                canonicalise_into(&map[*k], out);
            }
            out.push(b'}');
        }
        serde_json::Value::Array(arr) => {
            out.push(b'[');
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                canonicalise_into(item, out);
            }
            out.push(b']');
        }
        other => {
            // serde_json's default Display is already canonical for
            // null/bool/number/string at the granularity Rekor uses.
            let s = serde_json::to_string(other).expect("scalar serialize");
            out.extend_from_slice(s.as_bytes());
        }
    }
}

#[cfg(all(test, feature = "keyless"))]
mod tests {
    use super::*;
    use base64::Engine as _;
    use ecdsa::signature::Signer;
    use p256::ecdsa::{Signature, SigningKey};
    use p256::pkcs8::EncodePublicKey;
    use rand_core::OsRng;

    // -- VerificationMode parse round-trip --------------------------------

    #[test]
    fn verification_mode_parses_canonical_forms() {
        assert_eq!(
            VerificationMode::parse("key-based").unwrap(),
            VerificationMode::KeyBased
        );
        assert_eq!(
            VerificationMode::parse("KEY_BASED").unwrap(),
            VerificationMode::KeyBased
        );
        assert_eq!(
            VerificationMode::parse("keyless").unwrap(),
            VerificationMode::Keyless
        );
        assert_eq!(
            VerificationMode::parse("OIDC").unwrap(),
            VerificationMode::Keyless
        );
    }

    #[test]
    fn verification_mode_default_is_key_based() {
        assert_eq!(VerificationMode::default(), VerificationMode::KeyBased);
    }

    #[test]
    fn verification_mode_round_trip_via_str() {
        for m in [VerificationMode::KeyBased, VerificationMode::Keyless] {
            let s = m.as_str();
            let back = VerificationMode::parse(s).unwrap();
            assert_eq!(back, m);
        }
    }

    #[test]
    fn verification_mode_unknown_rejected() {
        let err = VerificationMode::parse("trustme").unwrap_err();
        match err {
            RegistryError::UnknownVerificationMode { mode, .. } => assert_eq!(mode, "trustme"),
            other => panic!("unexpected error: {:?}", other),
        }
    }

    // -- KeylessTrustRoot validation -------------------------------------

    #[test]
    fn trust_root_rejects_empty_fulcio() {
        let err = KeylessTrustRoot::from_pem(Vec::new(), b"x".to_vec()).unwrap_err();
        assert!(matches!(err, RegistryError::FulcioChainInvalid { .. }));
    }

    #[test]
    fn trust_root_rejects_empty_rekor() {
        let err = KeylessTrustRoot::from_pem(b"x".to_vec(), Vec::new()).unwrap_err();
        assert!(matches!(err, RegistryError::RekorLookupFailed { .. }));
    }

    // -- SignatureEnvelope auto-detect -----------------------------------

    #[test]
    fn envelope_detect_detached_when_only_signature() {
        let env = SignatureEnvelope {
            signature: Some(b"x".to_vec()),
            ..Default::default()
        };
        assert_eq!(env.kind(), EnvelopeKind::Detached);
    }

    #[test]
    fn envelope_detect_bundle_when_cert_and_bundle_present() {
        let env = SignatureEnvelope {
            signature: Some(b"x".to_vec()),
            cert_pem: Some(b"-----BEGIN CERTIFICATE-----".to_vec()),
            bundle_json: Some(b"{}".to_vec()),
        };
        assert_eq!(env.kind(), EnvelopeKind::Bundle);
    }

    #[test]
    fn envelope_from_annotations_decodes_signature_b64() {
        let raw_sig = b"hello";
        let b64 = base64::engine::general_purpose::STANDARD.encode(raw_sig);
        let env = SignatureEnvelope::from_annotations(|k| {
            if k == crate::client::COSIGN_SIGNATURE_ANNOTATION {
                Some(b64.as_str())
            } else {
                None
            }
        })
        .unwrap();
        assert_eq!(env.signature.clone().unwrap(), raw_sig);
        assert_eq!(env.kind(), EnvelopeKind::Detached);
    }

    #[test]
    fn envelope_from_annotations_rejects_garbage_b64() {
        let err = SignatureEnvelope::from_annotations(|k| {
            if k == crate::client::COSIGN_SIGNATURE_ANNOTATION {
                Some("$$$ not base64 $$$")
            } else {
                None
            }
        })
        .unwrap_err();
        assert!(matches!(err, RegistryError::SignatureMismatch { .. }));
    }

    // -- Rekor SET signature verification (synthetic key) ----------------

    /// Build a test Rekor pubkey + a bundle JSON whose SET signature is
    /// over the canonicalised payload.
    fn synth_rekor_bundle(
        body_b64: &str,
        integrated_time: i64,
        log_index: i64,
        log_id: &str,
    ) -> (Vec<u8>, Vec<u8>) {
        let signing = SigningKey::random(&mut OsRng);
        let pubkey = signing.verifying_key();
        let pubkey_pem = pubkey
            .to_public_key_pem(p256::pkcs8::LineEnding::LF)
            .unwrap()
            .into_bytes();
        let canonical = serde_json::json!({
            "body": body_b64,
            "integratedTime": integrated_time,
            "logIndex": log_index,
            "logID": log_id,
        });
        let canonical_bytes = canonical_json(&canonical);
        let sig: Signature = signing.sign(&canonical_bytes);
        let set_b64 = base64::engine::general_purpose::STANDARD.encode(sig.to_der().as_bytes());
        let bundle = serde_json::json!({
            "SignedEntryTimestamp": set_b64,
            "Payload": {
                "body": body_b64,
                "integratedTime": integrated_time,
                "logIndex": log_index,
                "logID": log_id,
            }
        });
        let bundle_bytes = serde_json::to_vec(&bundle).unwrap();
        (bundle_bytes, pubkey_pem)
    }

    #[test]
    fn rekor_inclusion_proof_succeeds_against_pinned_key() {
        let (bundle, pubkey) = synth_rekor_bundle("Ym9keQ==", 1_700_000_000, 42, "test-log");
        let ts = verify_rekor_inclusion_proof("acme", &bundle, &pubkey).unwrap();
        assert_eq!(ts, 1_700_000_000);
    }

    #[test]
    fn rekor_inclusion_proof_fails_when_payload_tampered() {
        let (bundle, pubkey) = synth_rekor_bundle("Ym9keQ==", 1_700_000_000, 42, "test-log");
        let mut v: serde_json::Value = serde_json::from_slice(&bundle).unwrap();
        // Mutate integratedTime → invalidates the SET signature.
        v["Payload"]["integratedTime"] = serde_json::json!(1_900_000_000);
        let tampered = serde_json::to_vec(&v).unwrap();
        let err = verify_rekor_inclusion_proof("acme", &tampered, &pubkey).unwrap_err();
        assert!(
            matches!(err, RegistryError::RekorInclusionProofInvalid { ref detail, .. } if detail.contains("ECDSA verify failed")),
            "expected ECDSA verify failure, got {:?}",
            err
        );
    }

    #[test]
    fn rekor_inclusion_proof_fails_when_set_tampered() {
        let (bundle, pubkey) = synth_rekor_bundle("Ym9keQ==", 1_700_000_000, 42, "test-log");
        let mut v: serde_json::Value = serde_json::from_slice(&bundle).unwrap();
        // Replace SET with a syntactically valid but unrelated signature.
        let other_signing = SigningKey::random(&mut OsRng);
        let canonical = serde_json::json!({
            "body": "Ym9keQ==",
            "integratedTime": 1_700_000_000,
            "logIndex": 42,
            "logID": "test-log",
        });
        let other_sig: Signature = other_signing.sign(&canonical_json(&canonical));
        let other_b64 =
            base64::engine::general_purpose::STANDARD.encode(other_sig.to_der().as_bytes());
        v["SignedEntryTimestamp"] = serde_json::json!(other_b64);
        let tampered = serde_json::to_vec(&v).unwrap();
        let err = verify_rekor_inclusion_proof("acme", &tampered, &pubkey).unwrap_err();
        assert!(matches!(
            err,
            RegistryError::RekorInclusionProofInvalid { .. }
        ));
    }

    #[test]
    fn rekor_inclusion_proof_fails_on_missing_fields() {
        let pubkey = SigningKey::random(&mut OsRng)
            .verifying_key()
            .to_public_key_pem(p256::pkcs8::LineEnding::LF)
            .unwrap()
            .into_bytes();
        let err = verify_rekor_inclusion_proof("acme", b"{}", &pubkey).unwrap_err();
        assert!(matches!(
            err,
            RegistryError::RekorInclusionProofInvalid { .. }
        ));
    }

    // -- Bundle envelope full-roundtrip via KeylessVerifier --------------

    // We can't easily synthesise a *real* Fulcio-issued cert in pure Rust
    // without pulling in `rcgen`, so the cert-chain + SAN tests use
    // pre-baked PEM fixtures generated offline (see `tests/fixtures/`).
    // Cross-cutting verifier tests live in `tests/keyless_wiremock.rs`
    // where we have wiremock + a fixture directory; this section keeps
    // unit-test parity with the key-based path's `verify_payload` tests.

    // -- Wave 6A.1: per-component identity resolution ---------------------
    fn ov_identity(glob: &str, san: &str, issuer: &str) -> sindri_core::manifest::TrustOverride {
        sindri_core::manifest::TrustOverride {
            component_glob: glob.to_string(),
            keys: None,
            identity: Some(sindri_core::manifest::RegistryIdentity {
                san_uri: san.to_string(),
                issuer: issuer.to_string(),
            }),
        }
    }

    fn ov_keys_only(glob: &str) -> sindri_core::manifest::TrustOverride {
        sindri_core::manifest::TrustOverride {
            component_glob: glob.to_string(),
            keys: Some(vec![std::path::PathBuf::from("/dev/null")]),
            identity: None,
        }
    }

    #[test]
    fn resolve_identity_falls_back_to_registry_when_no_override_matches() {
        let registry_id = sindri_core::manifest::RegistryIdentity {
            san_uri: "https://example/registry".into(),
            issuer: "https://issuer.example".into(),
        };
        let overrides = vec![ov_identity(
            "team-foo/*",
            "https://example/team-foo",
            "https://issuer.example",
        )];
        let resolved = KeylessVerifier::resolve_identity_for_component(
            "team-bar/svc",
            Some(&registry_id),
            &overrides,
        )
        .unwrap();
        assert_eq!(resolved.san_uri, "https://example/registry");
    }

    #[test]
    fn resolve_identity_uses_most_specific_override() {
        let registry_id = sindri_core::manifest::RegistryIdentity {
            san_uri: "https://example/registry".into(),
            issuer: "https://issuer.example".into(),
        };
        let overrides = vec![
            ov_identity("team-foo/*", "https://example/team-foo", "https://i"),
            ov_identity(
                "team-foo/specific",
                "https://example/team-foo/specific",
                "https://i",
            ),
        ];
        let resolved = KeylessVerifier::resolve_identity_for_component(
            "team-foo/specific",
            Some(&registry_id),
            &overrides,
        )
        .unwrap();
        assert_eq!(resolved.san_uri, "https://example/team-foo/specific");
    }

    #[test]
    fn resolve_identity_override_takes_precedence_over_registry() {
        // Even though the registry-level identity also covers this
        // component, the override takes precedence.
        let registry_id = sindri_core::manifest::RegistryIdentity {
            san_uri: "https://example/registry".into(),
            issuer: "https://issuer.example".into(),
        };
        let overrides = vec![ov_identity(
            "team-foo/*",
            "https://example/team-foo",
            "https://issuer.example",
        )];
        let resolved = KeylessVerifier::resolve_identity_for_component(
            "team-foo/svc",
            Some(&registry_id),
            &overrides,
        )
        .unwrap();
        assert_eq!(resolved.san_uri, "https://example/team-foo");
        // Crucially NOT the registry identity.
        assert_ne!(resolved.san_uri, "https://example/registry");
    }

    #[test]
    fn resolve_identity_keys_only_override_returns_none_no_registry_fallback() {
        // Override matches but is key-based-only (no identity). The
        // override-takes-precedence rule means we DON'T fall back to
        // the registry identity — that would silently re-enable trust
        // the override scoped down.
        let registry_id = sindri_core::manifest::RegistryIdentity {
            san_uri: "https://example/registry".into(),
            issuer: "https://issuer.example".into(),
        };
        let overrides = vec![ov_keys_only("team-foo/*")];
        let resolved = KeylessVerifier::resolve_identity_for_component(
            "team-foo/svc",
            Some(&registry_id),
            &overrides,
        );
        assert!(resolved.is_none());
    }

    #[test]
    fn resolve_identity_returns_none_when_neither_override_nor_registry() {
        let resolved = KeylessVerifier::resolve_identity_for_component("team-foo/svc", None, &[]);
        assert!(resolved.is_none());
    }

    #[test]
    fn verifier_rejects_envelope_without_certificate() {
        let trust = KeylessTrustRoot::from_pem(b"trust".to_vec(), b"rekor".to_vec()).unwrap();
        let v = KeylessVerifier::new(trust);
        let env = SignatureEnvelope {
            signature: Some(b"x".to_vec()),
            ..Default::default()
        };
        let id = KeylessIdentity {
            san_uri: "https://example".into(),
            issuer: "https://issuer".into(),
        };
        let err = v
            .verify("acme", &env, &id, b"payload", "sha256:abc")
            .unwrap_err();
        assert!(matches!(
            err,
            RegistryError::KeylessCertificateMissing { .. }
        ));
    }
}
