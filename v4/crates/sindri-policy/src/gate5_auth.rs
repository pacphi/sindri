//! Gate 5 — auth-resolvable admission gate (ADR-027 §5).
//!
//! Phase 2B of the auth-aware implementation plan. Evaluates the
//! resolver-produced [`AuthBinding`]s in `Lockfile.auth_bindings` against
//! the operator's [`AuthPolicy`] and returns one [`PolicyCheckResult`]
//! per offence, denying apply with `EXIT_POLICY_DENIED` when any of:
//!
//! 1. A non-`optional` requirement has no bound source
//!    (`status == Failed`) — controlled by
//!    [`AuthPolicy::on_unresolved_required`]. Default `deny`.
//! 2. Any binding selected `AuthSource::FromUpstreamCredentials` while
//!    [`AuthPolicy::allow_upstream_credentials`] is `false` (default).
//! 3. Any binding selected `AuthSource::Prompt` while the run is
//!    non-interactive (no TTY OR `CI` / `SINDRI_CI` env set) AND
//!    [`AuthPolicy::allow_prompt_in_ci`] is `false` (default).
//!
//! All three knobs default to the **deny** stance. Operators must opt
//! into each individually; each opt-in is documented with a security
//! caveat in `v4/docs/policy.md`.
//!
//! `--skip-auth` does NOT bypass this gate. The bypass is for redemption
//! only; admission still has to pass. Operators who genuinely need to
//! install with required credentials missing must additionally set
//! `auth.on_unresolved_required: warn`.

use crate::check::PolicyCheckResult;
use sindri_core::auth::{AuthBinding, AuthBindingStatus, AuthSource};
use sindri_core::policy::{AuthPolicy, PolicyAction};

/// Evaluate Gate 5 against the lockfile's bindings under the given
/// `auth` policy. Returns the first deny-class result found, or
/// `PolicyCheckResult::ok()` when all bindings are admissible.
///
/// We return on first failure (matching the rest of the policy crate)
/// to keep diagnostics terse; users see one issue per `sindri apply`
/// run, fix it, re-apply, see the next.
pub fn check_gate5(bindings: &[AuthBinding], policy: &AuthPolicy) -> PolicyCheckResult {
    check_gate5_with_env(bindings, policy, &CurrentEnv)
}

/// Variant with an injected [`EnvProbe`] so unit tests can simulate CI
/// without manipulating real `CI=` env vars.
pub fn check_gate5_with_env(
    bindings: &[AuthBinding],
    policy: &AuthPolicy,
    env: &dyn EnvProbe,
) -> PolicyCheckResult {
    // Rule 1: required-and-failed.
    for b in bindings {
        if b.status == AuthBindingStatus::Failed {
            match policy.on_unresolved_required {
                PolicyAction::Deny => {
                    return PolicyCheckResult::deny(
                        "AUTH_REQUIRED_UNRESOLVED",
                        &format!(
                            "Auth-aware Gate 5 denied apply: component `{}` requirement \
                             `{}` (audience `{}`) on target `{}` has no bound source.",
                            b.component, b.requirement, b.audience, b.target
                        ),
                        Some(
                            "Bind a source via `targets.<name>.provides:`, mark the \
                             requirement `optional: true`, or relax \
                             `auth.on_unresolved_required` to `warn`.",
                        ),
                    );
                }
                PolicyAction::Warn => {
                    tracing::warn!(
                        component = b.component.as_str(),
                        requirement = b.requirement.as_str(),
                        target = b.target.as_str(),
                        "Gate 5 (auth-resolvable): required binding unresolved; \
                         policy is warn (not denying)"
                    );
                }
                PolicyAction::Prompt | PolicyAction::Allow => {
                    // Phase 5 will wire interactive resolution; for now
                    // treat as warn (don't block apply).
                }
            }
        }
    }

    // Rule 2: FromUpstreamCredentials default-deny.
    if !policy.allow_upstream_credentials {
        for b in bindings {
            if matches!(b.source, Some(AuthSource::FromUpstreamCredentials)) {
                return PolicyCheckResult::deny(
                    "AUTH_UPSTREAM_REUSE_FORBIDDEN",
                    &format!(
                        "Auth-aware Gate 5 denied apply: binding `{}` on target `{}` \
                         selected `from-upstream-credentials`, but policy \
                         `auth.allow_upstream_credentials` is `false` (default).",
                        b.id, b.target
                    ),
                    Some(
                        "Add an explicit `provides:` entry on the target with a dedicated \
                         credential source, or set `auth.allow_upstream_credentials: true` \
                         (security caveat: shares the target's session token with the \
                         child workload — see v4/docs/policy.md).",
                    ),
                );
            }
        }
    }

    // Rule 3: Prompt in CI / non-interactive.
    if !policy.allow_prompt_in_ci && !env.is_interactive() {
        for b in bindings {
            if matches!(b.source, Some(AuthSource::Prompt)) {
                return PolicyCheckResult::deny(
                    "AUTH_PROMPT_IN_CI",
                    &format!(
                        "Auth-aware Gate 5 denied apply: binding `{}` on target `{}` \
                         selected `prompt`, but the run is non-interactive (no TTY or \
                         CI env set) and `auth.allow_prompt_in_ci` is `false`.",
                        b.id, b.target
                    ),
                    Some(
                        "Resolve the credential via env var or secrets backend on the \
                         CI runner, or set `auth.allow_prompt_in_ci: true` (not \
                         recommended for production CI).",
                    ),
                );
            }
        }
    }

    PolicyCheckResult::ok()
}

/// Probe for whether the current run is interactive (has a TTY) and not
/// flagged as CI. Real implementation reads env + isatty; tests inject
/// a deterministic value.
pub trait EnvProbe {
    /// True if Gate 5 should treat this run as interactive (TTY present
    /// and no CI marker). Equivalent to "Prompt is OK here".
    fn is_interactive(&self) -> bool;
}

/// Default env probe: looks at `CI`, `SINDRI_CI`, and stdin TTY.
pub struct CurrentEnv;

impl EnvProbe for CurrentEnv {
    fn is_interactive(&self) -> bool {
        // CI markers — both common and our own.
        if std::env::var("CI").is_ok() || std::env::var("SINDRI_CI").is_ok() {
            return false;
        }
        // No portable isatty in std — best-effort heuristic: if stdin has
        // a fd that is a terminal, we treat as interactive. We avoid
        // pulling a new dep here; users who run sindri under cron / nohup
        // typically also set CI=1.
        is_stdin_tty()
    }
}

#[cfg(unix)]
fn is_stdin_tty() -> bool {
    // SAFETY: isatty(0) is read-only; failure returns 0.
    unsafe { libc_isatty(0) != 0 }
}

#[cfg(unix)]
extern "C" {
    #[link_name = "isatty"]
    fn libc_isatty(fd: i32) -> i32;
}

#[cfg(not(unix))]
fn is_stdin_tty() -> bool {
    // Conservative on non-Unix: assume non-TTY so Gate 5 denies Prompt.
    // Operators on Windows can set `auth.allow_prompt_in_ci: true` if
    // they really want interactive prompts.
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use sindri_core::auth::{AuthBinding, AuthBindingStatus, AuthSource};

    struct FakeEnv {
        interactive: bool,
    }
    impl EnvProbe for FakeEnv {
        fn is_interactive(&self) -> bool {
            self.interactive
        }
    }

    fn binding(id: &str, status: AuthBindingStatus, source: Option<AuthSource>) -> AuthBinding {
        AuthBinding {
            id: id.into(),
            component: "npm:demo".into(),
            requirement: "tok".into(),
            audience: "urn:x".into(),
            target: "local".into(),
            source,
            priority: 0,
            status,
            reason: None,
            considered: Vec::new(),
        }
    }

    #[test]
    fn ok_when_no_bindings() {
        let r = check_gate5(&[], &AuthPolicy::default());
        assert!(r.allowed);
    }

    #[test]
    fn ok_when_all_bound_and_safe() {
        let bs = vec![binding(
            "a",
            AuthBindingStatus::Bound,
            Some(AuthSource::FromEnv { var: "X".into() }),
        )];
        let r = check_gate5(&bs, &AuthPolicy::default());
        assert!(r.allowed);
    }

    #[test]
    fn deny_when_required_failed() {
        let bs = vec![binding("a", AuthBindingStatus::Failed, None)];
        let r = check_gate5(&bs, &AuthPolicy::default());
        assert!(!r.allowed);
        assert_eq!(r.code, "AUTH_REQUIRED_UNRESOLVED");
    }

    #[test]
    fn warn_relaxes_required_failed() {
        let bs = vec![binding("a", AuthBindingStatus::Failed, None)];
        let policy = AuthPolicy {
            on_unresolved_required: PolicyAction::Warn,
            ..AuthPolicy::default()
        };
        let r = check_gate5(&bs, &policy);
        assert!(r.allowed);
    }

    #[test]
    fn deferred_is_not_denied() {
        // `Deferred` means optional + unbound. Gate 5 ignores it.
        let bs = vec![binding("a", AuthBindingStatus::Deferred, None)];
        let r = check_gate5(&bs, &AuthPolicy::default());
        assert!(r.allowed);
    }

    #[test]
    fn upstream_credentials_denied_by_default() {
        let bs = vec![binding(
            "a",
            AuthBindingStatus::Bound,
            Some(AuthSource::FromUpstreamCredentials),
        )];
        let r = check_gate5(&bs, &AuthPolicy::default());
        assert!(!r.allowed);
        assert_eq!(r.code, "AUTH_UPSTREAM_REUSE_FORBIDDEN");
    }

    #[test]
    fn upstream_credentials_allowed_when_opted_in() {
        let bs = vec![binding(
            "a",
            AuthBindingStatus::Bound,
            Some(AuthSource::FromUpstreamCredentials),
        )];
        let policy = AuthPolicy {
            allow_upstream_credentials: true,
            ..AuthPolicy::default()
        };
        let r = check_gate5(&bs, &policy);
        assert!(r.allowed);
    }

    #[test]
    fn prompt_denied_in_ci() {
        let bs = vec![binding(
            "a",
            AuthBindingStatus::Bound,
            Some(AuthSource::Prompt),
        )];
        let r = check_gate5_with_env(&bs, &AuthPolicy::default(), &FakeEnv { interactive: false });
        assert!(!r.allowed);
        assert_eq!(r.code, "AUTH_PROMPT_IN_CI");
    }

    #[test]
    fn prompt_ok_when_interactive() {
        let bs = vec![binding(
            "a",
            AuthBindingStatus::Bound,
            Some(AuthSource::Prompt),
        )];
        let r = check_gate5_with_env(&bs, &AuthPolicy::default(), &FakeEnv { interactive: true });
        assert!(r.allowed);
    }

    #[test]
    fn prompt_ok_in_ci_when_opted_in() {
        let bs = vec![binding(
            "a",
            AuthBindingStatus::Bound,
            Some(AuthSource::Prompt),
        )];
        let policy = AuthPolicy {
            allow_prompt_in_ci: true,
            ..AuthPolicy::default()
        };
        let r = check_gate5_with_env(&bs, &policy, &FakeEnv { interactive: false });
        assert!(r.allowed);
    }

    #[test]
    fn first_failure_wins_required_over_upstream() {
        // Both rules trip; required-failed reports first (rule order).
        let bs = vec![
            binding("a", AuthBindingStatus::Failed, None),
            binding(
                "b",
                AuthBindingStatus::Bound,
                Some(AuthSource::FromUpstreamCredentials),
            ),
        ];
        let r = check_gate5(&bs, &AuthPolicy::default());
        assert!(!r.allowed);
        assert_eq!(r.code, "AUTH_REQUIRED_UNRESOLVED");
    }

    #[test]
    fn skip_auth_does_not_bypass_gate_at_this_layer() {
        // Gate 5 is layer-agnostic — it sees the bindings, not the
        // CLI flag. Caller (apply.rs) decides whether to evaluate
        // before or after honouring `--skip-auth`. We assert here that
        // the gate's verdict is independent of redemption-bypass.
        let bs = vec![binding("a", AuthBindingStatus::Failed, None)];
        let r = check_gate5(&bs, &AuthPolicy::default());
        assert!(!r.allowed, "skip-auth must not relax Gate 5 by itself");
    }
}
