use thiserror::Error;

#[derive(Debug, Error)]
pub enum ResolverError {
    #[error("Component not found: {0}")]
    NotFound(String),
    #[error("Version conflict: {0}")]
    VersionConflict(String),
    #[error("Dependency cycle detected: {0}")]
    CycleDetected(String),
    #[error("Admission denied [{code}]: {message}")]
    AdmissionDenied { code: String, message: String },
    #[error("Registry error: {0}")]
    Registry(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Lockfile absent or stale — run `sindri resolve` first")]
    LockfileStale,
    #[error("Serialization error: {0}")]
    Serialization(String),
}

impl ResolverError {
    pub fn exit_code(&self) -> i32 {
        match self {
            ResolverError::AdmissionDenied { .. } => 2,
            ResolverError::LockfileStale => 5,
            ResolverError::NotFound(_)
            | ResolverError::VersionConflict(_)
            | ResolverError::CycleDetected(_)
            | ResolverError::Registry(_)
            | ResolverError::Serialization(_) => 4,
            ResolverError::Io(_) => 4,
        }
    }
}
