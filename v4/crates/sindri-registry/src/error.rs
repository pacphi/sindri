use thiserror::Error;

/// Errors raised by the registry layer (ADR-003 / ADR-014).
#[derive(Debug, Error)]
pub enum RegistryError {
    /// A registry endpoint could not be reached.
    #[error("Registry not reachable: {0}")]
    Unreachable(String),
    /// The requested component / blob was not found in the registry.
    #[error("Component not found: {0}")]
    NotFound(String),
    /// The fetched payload failed schema validation.
    #[error("Schema validation failed: {0}")]
    SchemaError(String),
    /// A signature verification step failed.
    #[error("Signature verification failed: {0}")]
    SignatureError(String),
    /// A cache I/O or layout error occurred.
    #[error("Cache error: {0}")]
    CacheError(String),

    /// The given OCI reference could not be parsed (ADR-003).
    #[error("Invalid OCI reference '{input}': {reason}")]
    InvalidOciRef { input: String, reason: String },

    /// A registry that requires a signature was added or refreshed without
    /// one (ADR-014, "fail closed" trust model).
    #[error("Signature required for registry '{registry}': {reason}")]
    SignatureRequired { registry: String, reason: String },

    /// A signature was present but did not match any trusted key.
    #[error(
        "Signature mismatch for registry '{registry}': expected one of {expected_keys:?} ({detail})"
    )]
    SignatureMismatch {
        registry: String,
        expected_keys: Vec<String>,
        detail: String,
    },

    /// A trust key on disk could not be parsed as an ECDSA P-256 PEM.
    #[error("Failed to parse cosign trust key '{path}': {detail}")]
    TrustKeyParseFailed { path: String, detail: String },

    /// An OCI Distribution Spec call (manifest pull, blob pull) failed.
    #[error("OCI fetch failed for '{reference}': {detail}")]
    OciFetch { reference: String, detail: String },

    /// The pulled OCI artifact had a layer media type the registry layer
    /// does not know how to interpret.
    #[error(
        "Unsupported OCI layer media type '{media_type}' for reference '{reference}'; expected one of: {expected}"
    )]
    UnsupportedMediaType {
        reference: String,
        media_type: String,
        expected: String,
    },

    /// The user passed `--insecure` while running under a policy that
    /// requires signed registries (ADR-014, strict preset).
    #[error(
        "policy requires signing for registry '{registry}'; --insecure is not allowed in strict mode"
    )]
    InsecureForbiddenByPolicy { registry: String },

    /// Layer extraction failed (Wave 5A — D6). Wraps tar/gzip errors and
    /// per-entry path-traversal violations.
    #[error("tar layer extraction failed for '{reference}': {detail}")]
    LayerExtraction { reference: String, detail: String },

    /// The pulled OCI artifact's tar/tar+gzip layer did not contain an
    /// `index.yaml` at the layer root.
    #[error("OCI artifact '{reference}' tar layer did not contain index.yaml")]
    IndexMissingFromLayer { reference: String },

    // -- Wave 6A: keyless OIDC cosign verification (ADR-014 D1) ----------------
    /// Keyless verification was requested but the build was compiled
    /// without the `keyless` feature enabled.
    #[error(
        "keyless verification requested for registry '{registry}' but the `keyless` cargo feature is disabled in this build"
    )]
    KeylessFeatureDisabled { registry: String },

    /// Fulcio cert chain validation failed — typically because the embedded
    /// Fulcio root CA did not sign the certificate carried by the cosign
    /// signature layer (i.e. the artifact was signed by something other than
    /// the public-good Sigstore Fulcio instance, or by a Fulcio whose root
    /// is not in our pinned trust bundle).
    #[error("Fulcio certificate chain validation failed for registry '{registry}': {detail}")]
    FulcioChainInvalid { registry: String, detail: String },

    /// The cosign signature carried no x509 certificate at all — keyless
    /// mode requires one (it's how we identify the signer). Typically means
    /// the artifact was signed in key-based mode but the registry policy
    /// declares `verification_mode: keyless`.
    #[error(
        "no Fulcio-issued certificate found in cosign signature layer for registry '{registry}'"
    )]
    KeylessCertificateMissing { registry: String },

    /// The certificate's SAN (Subject Alternative Name) URI did not match
    /// the registry's declared expected identity.
    #[error(
        "certificate SAN mismatch for registry '{registry}': expected '{expected}' (issuer '{expected_issuer}'), got '{actual}'"
    )]
    KeylessIdentityMismatch {
        registry: String,
        expected: String,
        expected_issuer: String,
        actual: String,
    },

    /// Rekor transparency log lookup failed (network error, 404, malformed
    /// response). Distinguishable from inclusion-proof tampering because the
    /// failure happens before signature verification.
    #[error("Rekor transparency log lookup failed for registry '{registry}': {detail}")]
    RekorLookupFailed { registry: String, detail: String },

    /// The Rekor inclusion proof did not validate against Rekor's signed
    /// tree head — i.e. the bundle in the cosign annotation has been
    /// tampered with, or Rekor's public key has rotated and our pinned
    /// copy is stale.
    #[error("Rekor inclusion proof failed verification for registry '{registry}': {detail}")]
    RekorInclusionProofInvalid { registry: String, detail: String },

    /// The signature timestamp recorded in the Rekor entry falls outside
    /// the certificate's `notBefore`/`notAfter` validity window — either
    /// the cert was forged after expiry or backdated before issuance.
    #[error(
        "certificate validity window does not cover Rekor signature timestamp for registry '{registry}': {detail}"
    )]
    KeylessCertificateExpired { registry: String, detail: String },

    /// The registry policy declared `verification_mode` as something
    /// other than `key-based` or `keyless`.
    #[error(
        "unknown verification_mode '{mode}' for registry '{registry}'; expected 'key-based' or 'keyless'"
    )]
    UnknownVerificationMode { registry: String, mode: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}
