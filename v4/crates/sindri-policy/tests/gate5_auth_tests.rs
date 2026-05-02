//! Integration tests for `sindri_policy::gate5_auth` covering the two arms
//! of Rule 1 that the inline unit tests do not reach:
//!
//! - `on_unresolved_required = PolicyAction::Allow`  → allow despite failed binding
//! - `on_unresolved_required = PolicyAction::Prompt` → treated as warn (allow) in Phase 2B
//!
//! These tests call the public `check_gate5_with_env` function through a
//! deterministic `FakeEnv` so they are hermetic (no real TTY or CI env needed).

use sindri_core::auth::{AuthBinding, AuthBindingStatus, AuthSource};
use sindri_core::policy::{AuthPolicy, PolicyAction};
use sindri_policy::gate5_auth::{check_gate5_with_env, EnvProbe};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

struct Interactive;
impl EnvProbe for Interactive {
    fn is_interactive(&self) -> bool {
        true
    }
}

struct NonInteractive;
impl EnvProbe for NonInteractive {
    fn is_interactive(&self) -> bool {
        false
    }
}

fn failed_binding(id: &str) -> AuthBinding {
    AuthBinding {
        id: id.into(),
        component: "npm:demo".into(),
        requirement: "api-token".into(),
        audience: "urn:sindri:npm".into(),
        target: "local".into(),
        source: None,
        priority: 0,
        status: AuthBindingStatus::Failed,
        reason: None,
        considered: Vec::new(),
    }
}

fn bound_binding_with_source(id: &str, source: AuthSource) -> AuthBinding {
    AuthBinding {
        id: id.into(),
        component: "npm:demo".into(),
        requirement: "token".into(),
        audience: "urn:sindri:npm".into(),
        target: "local".into(),
        source: Some(source),
        priority: 0,
        status: AuthBindingStatus::Bound,
        reason: None,
        considered: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Rule 1: on_unresolved_required = Allow
// ---------------------------------------------------------------------------

#[test]
fn rule1_allow_action_passes_despite_failed_binding() {
    let bs = vec![failed_binding("tok-a")];
    let policy = AuthPolicy {
        on_unresolved_required: PolicyAction::Allow,
        ..AuthPolicy::default()
    };
    let r = check_gate5_with_env(&bs, &policy, &NonInteractive);
    assert!(
        r.allowed,
        "PolicyAction::Allow must not block a failed binding; got code={}",
        r.code
    );
}

#[test]
fn rule1_allow_action_with_multiple_failed_bindings_all_pass() {
    let bs = vec![
        failed_binding("a"),
        failed_binding("b"),
        failed_binding("c"),
    ];
    let policy = AuthPolicy {
        on_unresolved_required: PolicyAction::Allow,
        ..AuthPolicy::default()
    };
    let r = check_gate5_with_env(&bs, &policy, &NonInteractive);
    assert!(
        r.allowed,
        "all failed bindings must be tolerated under Allow"
    );
}

// ---------------------------------------------------------------------------
// Rule 1: on_unresolved_required = Prompt
// ---------------------------------------------------------------------------

#[test]
fn rule1_prompt_action_passes_despite_failed_binding() {
    // Phase 2B: Prompt is wired as "treat like Warn" — apply is not blocked.
    let bs = vec![failed_binding("tok-b")];
    let policy = AuthPolicy {
        on_unresolved_required: PolicyAction::Prompt,
        ..AuthPolicy::default()
    };
    let r = check_gate5_with_env(&bs, &policy, &NonInteractive);
    assert!(
        r.allowed,
        "PolicyAction::Prompt must not block a failed binding in Phase 2B; got code={}",
        r.code
    );
}

#[test]
fn rule1_prompt_action_interactive_env_also_passes() {
    let bs = vec![failed_binding("tok-c")];
    let policy = AuthPolicy {
        on_unresolved_required: PolicyAction::Prompt,
        ..AuthPolicy::default()
    };
    // Even in an interactive session, Prompt for Rule 1 must not block.
    let r = check_gate5_with_env(&bs, &policy, &Interactive);
    assert!(r.allowed);
}

// ---------------------------------------------------------------------------
// Rule 1 variants do not accidentally suppress Rule 2 or Rule 3
// ---------------------------------------------------------------------------

#[test]
fn rule1_allow_still_enforces_rule2_upstream_denied() {
    // Rule 1 → Allow: but a bound-upstream-creds binding must still trip Rule 2.
    let bs = vec![
        failed_binding("failed"),
        bound_binding_with_source("upstream", AuthSource::FromUpstreamCredentials),
    ];
    let policy = AuthPolicy {
        on_unresolved_required: PolicyAction::Allow,
        allow_upstream_credentials: false,
        ..AuthPolicy::default()
    };
    let r = check_gate5_with_env(&bs, &policy, &NonInteractive);
    assert!(
        !r.allowed,
        "Rule 2 must still trigger even when Rule 1 is Allow"
    );
    assert!(
        r.code.contains("UPSTREAM") || r.code.contains("AUTH"),
        "expected upstream-denied code, got: {}",
        r.code
    );
}

#[test]
fn rule1_allow_still_enforces_rule3_prompt_in_ci() {
    // Rule 1 → Allow: but a Prompt source in non-interactive env must trip Rule 3.
    let bs = vec![
        failed_binding("failed"),
        bound_binding_with_source("prompt-src", AuthSource::Prompt),
    ];
    let policy = AuthPolicy {
        on_unresolved_required: PolicyAction::Allow,
        allow_upstream_credentials: true,
        allow_prompt_in_ci: false,
    };
    let r = check_gate5_with_env(&bs, &policy, &NonInteractive);
    assert!(
        !r.allowed,
        "Rule 3 must still trigger even when Rule 1 is Allow"
    );
    assert!(
        r.code.contains("PROMPT") || r.code.contains("CI") || r.code.contains("AUTH"),
        "expected prompt-in-ci code, got: {}",
        r.code
    );
}

// ---------------------------------------------------------------------------
// Empty binding list is always OK regardless of on_unresolved_required
// ---------------------------------------------------------------------------

#[test]
fn empty_bindings_always_ok_under_all_rule1_variants() {
    for action in [
        PolicyAction::Deny,
        PolicyAction::Warn,
        PolicyAction::Allow,
        PolicyAction::Prompt,
    ] {
        let policy = AuthPolicy {
            on_unresolved_required: action.clone(),
            ..AuthPolicy::default()
        };
        let r = check_gate5_with_env(&[], &policy, &NonInteractive);
        assert!(
            r.allowed,
            "empty bindings must always be ok, failed for {action:?}"
        );
    }
}
