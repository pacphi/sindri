use sindri_core::exit_codes::{
    EXIT_POLICY_DENIED, EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_STALE_LOCKFILE, EXIT_STRICT_OCI_DENIED,
};
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
            // Strict-OCI source denial gets its own exit code so CI can
            // distinguish it from generic admission failures (ADR-028, ADR-012).
            ResolverError::SourceNotProductionGrade { .. } => EXIT_STRICT_OCI_DENIED,
            ResolverError::AdmissionDenied { .. } => EXIT_POLICY_DENIED,
            ResolverError::LockfileStale => EXIT_STALE_LOCKFILE,
            ResolverError::NotFound(_)
            | ResolverError::VersionConflict(_)
            | ResolverError::CycleDetected(_)
            | ResolverError::Registry(_)
            | ResolverError::Serialization(_)
            | ResolverError::Io(_) => EXIT_SCHEMA_OR_RESOLVE_ERROR,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sindri_core::exit_codes::{
        EXIT_POLICY_DENIED, EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_STALE_LOCKFILE,
        EXIT_STRICT_OCI_DENIED,
    };

    // ---------------------------------------------------------------------------
    // exit_code mapping (ADR-012)
    // ---------------------------------------------------------------------------

    #[test]
    fn not_found_returns_schema_or_resolve_error() {
        assert_eq!(
            ResolverError::NotFound("mise:nodejs".into()).exit_code(),
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        );
    }

    #[test]
    fn version_conflict_returns_schema_or_resolve_error() {
        assert_eq!(
            ResolverError::VersionConflict("mise:nodejs 1.0 vs 2.0".into()).exit_code(),
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        );
    }

    #[test]
    fn cycle_detected_returns_schema_or_resolve_error() {
        assert_eq!(
            ResolverError::CycleDetected("mise:a → mise:b → mise:a".into()).exit_code(),
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        );
    }

    #[test]
    fn registry_error_returns_schema_or_resolve_error() {
        assert_eq!(
            ResolverError::Registry("OCI fetch failed".into()).exit_code(),
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        );
    }

    #[test]
    fn serialization_returns_schema_or_resolve_error() {
        assert_eq!(
            ResolverError::Serialization("invalid JSON".into()).exit_code(),
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        );
    }

    #[test]
    fn io_error_returns_schema_or_resolve_error() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        assert_eq!(
            ResolverError::Io(io).exit_code(),
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        );
    }

    #[test]
    fn admission_denied_returns_policy_denied() {
        assert_eq!(
            ResolverError::AdmissionDenied {
                code: "ADM_BLOCKLIST".into(),
                message: "component on blocklist".into(),
            }
            .exit_code(),
            EXIT_POLICY_DENIED
        );
    }

    #[test]
    fn lockfile_stale_returns_stale_lockfile() {
        assert_eq!(
            ResolverError::LockfileStale.exit_code(),
            EXIT_STALE_LOCKFILE
        );
    }

    #[test]
    fn source_not_production_grade_returns_strict_oci_denied() {
        assert_eq!(
            ResolverError::SourceNotProductionGrade {
                offenders: vec![("mise:nodejs".into(), "local-path".into())],
            }
            .exit_code(),
            EXIT_STRICT_OCI_DENIED
        );
    }

    // ---------------------------------------------------------------------------
    // Display formatting — every variant must not panic and must contain key fields
    // ---------------------------------------------------------------------------

    #[test]
    fn display_not_found_contains_address() {
        let e = ResolverError::NotFound("mise:nodejs".into());
        assert!(e.to_string().contains("mise:nodejs"));
    }

    #[test]
    fn display_admission_denied_contains_code_and_message() {
        let e = ResolverError::AdmissionDenied {
            code: "ADM_BLOCKLIST".into(),
            message: "not allowed".into(),
        };
        let s = e.to_string();
        assert!(s.contains("ADM_BLOCKLIST"), "{s}");
        assert!(s.contains("not allowed"), "{s}");
    }

    #[test]
    fn display_source_not_production_grade_lists_offenders() {
        let e = ResolverError::SourceNotProductionGrade {
            offenders: vec![
                ("mise:nodejs".into(), "local".into()),
                ("npm:ripgrep".into(), "git".into()),
            ],
        };
        let s = e.to_string();
        assert!(s.contains("mise:nodejs"), "{s}");
        assert!(s.contains("npm:ripgrep"), "{s}");
    }

    #[test]
    fn display_lockfile_stale_mentions_resolve() {
        let s = ResolverError::LockfileStale.to_string();
        assert!(s.contains("sindri resolve"), "{s}");
    }
}
