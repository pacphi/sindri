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

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}
