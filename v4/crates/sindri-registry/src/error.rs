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

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}
