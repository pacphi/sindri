// ADR-012: Standardized exit-code contract
//
// | Code | Name                    | Meaning                                                      |
// | 0    | SUCCESS                 | Operation completed successfully                             |
// | 1    | ERROR                   | Generic error (I/O, network, unexpected panic)               |
// | 2    | POLICY_DENIED           | One or more components denied by install policy              |
// | 3    | RESOLUTION_CONFLICT     | Dependency closure has an unresolvable conflict              |
// | 4    | SCHEMA_ERROR            | sindri.yaml or sindri.policy.yaml failed validation          |
// | 5    | STALE_LOCKFILE          | sindri.lock is absent or does not match sindri.yaml          |
// | 6    | APPLY_IN_PROGRESS       | Another `sindri apply` is already running for this BOM       |
// | 7    | STRICT_OCI_DENIED       | --strict-oci gate rejected one or more non-production sources|

pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_ERROR: i32 = 1;
pub const EXIT_POLICY_DENIED: i32 = 2;
pub const EXIT_RESOLUTION_CONFLICT: i32 = 3;
pub const EXIT_SCHEMA_OR_RESOLVE_ERROR: i32 = 4;
pub const EXIT_STALE_LOCKFILE: i32 = 5;
/// Exit code 6: another `sindri apply` process holds the state-file flock for
/// this BOM hash.  The user should wait for the in-progress apply to finish
/// (or stale-lock clean-up with `sindri apply --clear-state`).
pub const EXIT_APPLY_IN_PROGRESS: i32 = 6;
/// Exit code 7: the `--strict-oci` admission gate (ADR-028, DDD-08 Phase 2)
/// rejected the resolution because one or more components were sourced from a
/// non-production-grade source (i.e. `Source::supports_strict_oci()` returned
/// `false`). This is distinct from generic policy denial (`EXIT_POLICY_DENIED`)
/// so that CI can route strict-OCI violations to a dedicated alert channel
/// without false-positive noise from other admission failures.
///
/// Only fires when the resolver returns
/// `ResolverError::SourceNotProductionGrade`; all other admission denials
/// continue to use `EXIT_POLICY_DENIED`.
pub const EXIT_STRICT_OCI_DENIED: i32 = 7;

/// Typed exit-code enum mirroring the const values above.
#[repr(i32)]
pub enum ExitCode {
    Success = EXIT_SUCCESS,
    Error = EXIT_ERROR,
    PolicyDenied = EXIT_POLICY_DENIED,
    ResolutionConflict = EXIT_RESOLUTION_CONFLICT,
    SchemaOrResolveError = EXIT_SCHEMA_OR_RESOLVE_ERROR,
    StaleLockfile = EXIT_STALE_LOCKFILE,
    ApplyInProgress = EXIT_APPLY_IN_PROGRESS,
    StrictOciDenied = EXIT_STRICT_OCI_DENIED,
}
