//! Canonical admission-code constants (ADR-008, ADR-027 §5).
//!
//! Every Gate emits a code in the `ADM_*` family. Constants live here so
//! call sites (`gate5_auth`, `check`, `capability_trust`, the resolver's
//! platform/license/capability checks) can `use admission_codes::*`
//! instead of stringly-typed literals.
//!
//! The codes are user-visible: CI alerting, log dashboards, and the
//! `--explain` admission trace all surface them. They are stable within a
//! major version. Renames are breaking changes that must land in CHANGELOG.

// Gate 1 — platform eligibility.
pub const ADM_PLATFORM_SKIPPED: &str = "ADM_PLATFORM_SKIPPED";
pub const ADM_PLATFORM_UNSUPPORTED: &str = "ADM_PLATFORM_UNSUPPORTED";

// Gate 2 — policy eligibility (license, version-pinning, source-quality).
pub const ADM_LICENSE_DENIED: &str = "ADM_LICENSE_DENIED";
pub const ADM_LICENSE_UNKNOWN: &str = "ADM_LICENSE_UNKNOWN";
pub const ADM_VERSION_NOT_PINNED: &str = "ADM_VERSION_NOT_PINNED";
pub const ADM_SCRIPT_DENIED: &str = "ADM_SCRIPT_DENIED";
pub const ADM_PRIVILEGED_DENIED: &str = "ADM_PRIVILEGED_DENIED";
pub const ADM_CHECKSUM_MISSING: &str = "ADM_CHECKSUM_MISSING";

// Gate 3 — dependency closure. Closure-time codes are typed; this is the
// catch-all for blocklist hits surfaced via `ResolverError::AdmissionDenied`.
pub const ADM_BLOCKLIST: &str = "ADM_BLOCKLIST";

// Gate 4 — capability trust (collision-handling path prefix).
pub const ADM_CAPABILITY_TRUST_SKIPPED: &str = "ADM_CAPABILITY_TRUST_SKIPPED";
pub const ADM_CAPABILITY_TRUST_VIOLATION: &str = "ADM_CAPABILITY_TRUST_VIOLATION";

// Gate 5 — auth-resolvable (ADR-027 §5).
pub const ADM_AUTH_UNRESOLVED: &str = "ADM_AUTH_UNRESOLVED";
pub const ADM_AUTH_UPSTREAM_DENIED: &str = "ADM_AUTH_UPSTREAM_DENIED";
pub const ADM_AUTH_PROMPT_IN_CI: &str = "ADM_AUTH_PROMPT_IN_CI";

#[cfg(test)]
mod tests {
    use super::*;

    /// Every code starts with `ADM_` — the prefix family is the contract.
    #[test]
    fn every_code_has_adm_prefix() {
        let all = [
            ADM_PLATFORM_SKIPPED,
            ADM_PLATFORM_UNSUPPORTED,
            ADM_LICENSE_DENIED,
            ADM_LICENSE_UNKNOWN,
            ADM_VERSION_NOT_PINNED,
            ADM_SCRIPT_DENIED,
            ADM_PRIVILEGED_DENIED,
            ADM_CHECKSUM_MISSING,
            ADM_BLOCKLIST,
            ADM_CAPABILITY_TRUST_SKIPPED,
            ADM_CAPABILITY_TRUST_VIOLATION,
            ADM_AUTH_UNRESOLVED,
            ADM_AUTH_UPSTREAM_DENIED,
            ADM_AUTH_PROMPT_IN_CI,
        ];
        for code in all {
            assert!(code.starts_with("ADM_"), "code without ADM_ prefix: {code}");
        }
    }

    /// Constant value matches the variable name (cheap typo guard).
    #[test]
    fn constant_value_matches_name() {
        assert_eq!(ADM_AUTH_UNRESOLVED, "ADM_AUTH_UNRESOLVED");
        assert_eq!(ADM_AUTH_UPSTREAM_DENIED, "ADM_AUTH_UPSTREAM_DENIED");
        assert_eq!(ADM_AUTH_PROMPT_IN_CI, "ADM_AUTH_PROMPT_IN_CI");
    }
}
