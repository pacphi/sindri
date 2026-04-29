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
    /// Strict-OCI admission gate failure (DDD-08, ADR-028 — Phase 2).
    ///
    /// Raised when `--strict-oci` (or `registry.policy.strict_oci`) is on
    /// and one or more lockfile components were produced by a source that
    /// returns `false` from
    /// `sindri_registry::source::Source::supports_strict_oci`.
    /// `offenders` carries `(component_address, source_kind)` pairs so the
    /// CLI can render an actionable error message.
    #[error(
        "Admission denied [ADM_SOURCE_NOT_PRODUCTION_GRADE]: {} offending component(s) produced by non-production-grade sources: {}",
        offenders.len(),
        offenders
            .iter()
            .map(|(c, s)| format!("{} (source={})", c, s))
            .collect::<Vec<_>>()
            .join(", ")
    )]
    SourceNotProductionGrade {
        /// `(component_address, source_kind)` for each offender, in
        /// lockfile order so output is deterministic.
        offenders: Vec<(String, String)>,
    },
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
            ResolverError::AdmissionDenied { .. }
            | ResolverError::SourceNotProductionGrade { .. } => 2,
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
