use thiserror::Error;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("Registry not reachable: {0}")]
    Unreachable(String),
    #[error("Component not found: {0}")]
    NotFound(String),
    #[error("Schema validation failed: {0}")]
    SchemaError(String),
    #[error("Signature verification failed: {0}")]
    SignatureError(String),
    #[error("Cache error: {0}")]
    CacheError(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}
