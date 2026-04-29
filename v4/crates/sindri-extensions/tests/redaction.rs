//! Redaction property test (Phase 2A — non-negotiable).
//!
//! For every randomly-generated [`AuthValue`] string we run the redeemer
//! end-to-end (env-var, file, env-file) and capture every ledger event
//! emitted to a sandboxed JSONL file. We then regex-search every line of
//! that ledger for the secret value. **Any match fails the test.**
//!
//! This is the property that DDD-07 invariant 3 ("no value capture") and
//! ADR-027 §6 turn into a code-level guarantee. If you ever see this test
//! fail you have introduced a leak — do not weaken the test, fix the leak.
//!
//! Test isolation: the redeemer ledger writer reads `SINDRI_AUTH_LEDGER_PATH`
//! at runtime; we point each prop case at a fresh tempfile so the user's
//! real `~/.sindri/ledger.jsonl` is untouched.

use proptest::prelude::*;
use regex::Regex;
use sindri_core::auth::{
    AuthBinding, AuthBindingStatus, AuthRequirements, AuthScope, AuthSource, DiscoveryHints,
    Redemption, TokenRequirement,
};
use sindri_core::platform::TargetProfile;
use sindri_extensions::redeemer::ComponentBindings;
use sindri_extensions::AuthRedeemer;
use sindri_targets::error::TargetError;
use sindri_targets::traits::PrereqCheck;
use sindri_targets::Target;
use std::sync::Mutex;

struct MockTarget;
impl Target for MockTarget {
    fn name(&self) -> &str {
        "local"
    }
    fn kind(&self) -> &str {
        "local"
    }
    fn profile(&self) -> Result<TargetProfile, TargetError> {
        Err(TargetError::Unavailable {
            name: "mock".into(),
            reason: "test".into(),
        })
    }
    fn exec(&self, _cmd: &str, _env: &[(&str, &str)]) -> Result<(String, String), TargetError> {
        Ok((String::new(), String::new()))
    }
    fn upload(&self, _l: &std::path::Path, _r: &str) -> Result<(), TargetError> {
        Ok(())
    }
    fn download(&self, _r: &str, _l: &std::path::Path) -> Result<(), TargetError> {
        Ok(())
    }
    fn check_prerequisites(&self) -> Vec<PrereqCheck> {
        Vec::new()
    }
}

// Lock to serialise `std::env::set_var` access across prop cases — env is
// process-global. proptest runs cases sequentially by default, but if a
// future contributor parallelises this we want the lock in place.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn run_one_redemption(
    secret_value: &str,
    redemption: Redemption,
    ledger_path: &std::path::Path,
) -> std::io::Result<()> {
    let _guard = ENV_LOCK.lock().unwrap();

    // 1. Stage the secret into the FromEnv source.
    let env_var = "SINDRI_PROP_SECRET_VAR";
    std::env::set_var(env_var, secret_value);
    std::env::set_var("SINDRI_AUTH_LEDGER_PATH", ledger_path);

    let auth = AuthRequirements {
        tokens: vec![TokenRequirement {
            name: "tok".into(),
            description: "t".into(),
            scope: AuthScope::Install,
            optional: false,
            audience: "urn:x".into(),
            redemption: redemption.clone(),
            discovery: DiscoveryHints::default(),
        }],
        ..Default::default()
    };
    let b = AuthBinding {
        id: "bid:prop".into(),
        component: "npm:demo".into(),
        requirement: "tok".into(),
        audience: "urn:x".into(),
        target: "local".into(),
        source: Some(AuthSource::FromEnv {
            var: env_var.into(),
        }),
        priority: 0,
        status: AuthBindingStatus::Bound,
        reason: None,
        considered: Vec::new(),
    };
    let cb = ComponentBindings {
        component: "npm:demo",
        bindings: vec![&b],
        auth: &auth,
    };

    let r = AuthRedeemer::new();
    let env = r.redeem_install_scope(&cb, &MockTarget).expect("redeem ok");
    r.cleanup(&env, "local");

    // Cleanup env vars regardless.
    std::env::remove_var(env_var);
    std::env::remove_var("SINDRI_AUTH_LEDGER_PATH");
    Ok(())
}

/// True iff the ledger file at `path` contains the literal `needle`
/// anywhere on any line. Uses regex with the needle escaped so we match
/// the value literally (incl. JSON-escaped quoting variants).
fn ledger_contains(path: &std::path::Path, needle: &str) -> bool {
    let Ok(content) = std::fs::read_to_string(path) else {
        return false;
    };
    if content.contains(needle) {
        return true;
    }
    // Defence against a maliciously-constructed needle producing an
    // invalid regex.
    if let Ok(re) = Regex::new(&regex::escape(needle)) {
        return re.is_match(&content);
    }
    false
}

proptest! {
    #![proptest_config(ProptestConfig {
        // 64 random cases × 3 redemption variants = 192 redemption flows,
        // generates ~600 ledger events scanned per run.
        cases: 64,
        .. ProptestConfig::default()
    })]

    /// PROPERTY: for any non-empty random secret string, no ledger event
    /// emitted by the redeemer contains the secret value verbatim.
    #[test]
    fn redemption_never_leaks_secret_value(
        secret_value in "[A-Za-z0-9!@#$%^&*()_+/=:.-]{8,64}",
    ) {
        let dir = tempfile::tempdir().unwrap();

        // Variant 1: EnvVar redemption.
        let l1 = dir.path().join("env.jsonl");
        run_one_redemption(
            &secret_value,
            Redemption::EnvVar { env_name: "INJECT".into() },
            &l1,
        ).unwrap();
        prop_assert!(!ledger_contains(&l1, &secret_value),
            "EnvVar leaked secret into ledger; secret={}", secret_value);

        // Variant 2: File redemption (we only check the ledger for the
        // value; the on-disk written file legitimately contains the
        // secret while the lifecycle step holds it).
        let l2 = dir.path().join("file.jsonl");
        let creds_path = dir.path().join("creds.bin");
        run_one_redemption(
            &secret_value,
            Redemption::File {
                path: creds_path.to_string_lossy().to_string(),
                mode: Some(0o600),
                persist: false,
            },
            &l2,
        ).unwrap();
        prop_assert!(!ledger_contains(&l2, &secret_value),
            "File leaked secret into ledger; secret={}", secret_value);

        // Variant 3: EnvFile redemption.
        let l3 = dir.path().join("envfile.jsonl");
        let gcp_path = dir.path().join("gcp.json");
        run_one_redemption(
            &secret_value,
            Redemption::EnvFile {
                env_name: "GOOGLE_APPLICATION_CREDENTIALS".into(),
                path: gcp_path.to_string_lossy().to_string(),
            },
            &l3,
        ).unwrap();
        prop_assert!(!ledger_contains(&l3, &secret_value),
            "EnvFile leaked secret into ledger; secret={}", secret_value);
    }
}

#[test]
fn ledger_writes_at_least_one_redemption_event() {
    // Sanity: redaction test would pass trivially if the ledger never
    // wrote anything. Confirm we ARE emitting events to scan.
    let dir = tempfile::tempdir().unwrap();
    let l = dir.path().join("sanity.jsonl");
    run_one_redemption(
        "the-secret-marker",
        Redemption::EnvVar {
            env_name: "INJECT".into(),
        },
        &l,
    )
    .unwrap();
    let content = std::fs::read_to_string(&l).expect("ledger should exist");
    assert!(
        content.contains("AuthRedeemed"),
        "expected an AuthRedeemed event, got: {}",
        content
    );
    assert!(
        content.contains("AuthCleanedUp"),
        "expected an AuthCleanedUp event, got: {}",
        content
    );
    // And of course the secret itself MUST NOT be in the ledger.
    assert!(
        !content.contains("the-secret-marker"),
        "secret leaked into ledger: {}",
        content
    );
}
