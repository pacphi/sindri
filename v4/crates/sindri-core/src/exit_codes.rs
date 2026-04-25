// ADR-012: Standardized exit-code contract
//
// | Code | Name                  | Meaning                                                      |
// | 0    | SUCCESS               | Operation completed successfully                             |
// | 1    | ERROR                 | Generic error (I/O, network, unexpected panic)               |
// | 2    | POLICY_DENIED         | One or more components denied by install policy              |
// | 3    | RESOLUTION_CONFLICT   | Dependency closure has an unresolvable conflict              |
// | 4    | SCHEMA_ERROR          | sindri.yaml or sindri.policy.yaml failed validation          |
// | 5    | STALE_LOCKFILE        | sindri.lock is absent or does not match sindri.yaml          |

pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_ERROR: i32 = 1;
pub const EXIT_POLICY_DENIED: i32 = 2;
pub const EXIT_RESOLUTION_CONFLICT: i32 = 3;
pub const EXIT_SCHEMA_OR_RESOLVE_ERROR: i32 = 4;
pub const EXIT_STALE_LOCKFILE: i32 = 5;

/// Typed exit-code enum mirroring the const values above.
#[repr(i32)]
pub enum ExitCode {
    Success = EXIT_SUCCESS,
    Error = EXIT_ERROR,
    PolicyDenied = EXIT_POLICY_DENIED,
    ResolutionConflict = EXIT_RESOLUTION_CONFLICT,
    SchemaOrResolveError = EXIT_SCHEMA_OR_RESOLVE_ERROR,
    StaleLockfile = EXIT_STALE_LOCKFILE,
}
