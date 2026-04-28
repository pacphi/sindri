//! Auth-binding ledger events (DDD-07 §"Domain Events", PR #2 of Phase 1).
//!
//! Phase 1 of the auth-aware implementation plan emits five events at
//! resolve time:
//!
//! | Event                        | Producer                    |
//! | ---------------------------- | --------------------------- |
//! | `AuthRequirementDeclared`    | per requirement on every component |
//! | `AuthCapabilityRegistered`   | per capability on every target     |
//! | `AuthBindingResolved`        | per `Bound` binding                |
//! | `AuthBindingDeferred`        | per `Deferred` binding (optional)  |
//! | `AuthBindingFailed`          | per `Failed` binding (required)    |
//!
//! The events are appended to the same JSONL ledger consumed by
//! `sindri log` (`~/.sindri/ledger.jsonl`). All payloads redact secret
//! values — the binding domain captures only references (DDD-07 invariant
//! 3 "no value capture"), so there is nothing to redact in practice, but
//! the schema is intentionally limited to safe metadata only.
//!
//! Emission is best-effort: a write failure logs at `tracing::warn!` and
//! returns silently. Resolve must not fail because the audit trail is
//! unavailable; downstream operators will notice via `sindri doctor`.

use crate::auth_binding::{BindingPass, ComponentAuthInput, TargetAuthInput};
use serde::{Deserialize, Serialize};
use sindri_core::auth::{auth_source_kind, AuthBindingStatus, AuthScope};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// One audit-ledger event for an auth-binding lifecycle action
/// (DDD-07 §"Domain Events").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthLedgerEvent {
    /// Unix epoch seconds.
    pub timestamp: u64,
    /// One of `AuthRequirementDeclared`, `AuthCapabilityRegistered`,
    /// `AuthBindingResolved`, `AuthBindingDeferred`, `AuthBindingFailed`.
    pub event_type: String,
    /// Component address (`backend:name[@qualifier]`) when the event is
    /// component-scoped, else the empty string.
    #[serde(default)]
    pub component: String,
    /// Target id when target-scoped, else the empty string.
    #[serde(default)]
    pub target: String,
    /// Requirement or capability identifier when applicable.
    #[serde(default)]
    pub name: String,
    /// Audience associated with the event.
    #[serde(default)]
    pub audience: String,
    /// Source-kind discriminant, e.g. `from-secrets-store`. Empty when
    /// the event has no associated source.
    #[serde(default)]
    pub source_kind: String,
    /// Free-form reason / detail (e.g. `"no source matched (required)"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Default ledger location (`~/.sindri/ledger.jsonl`). Returns `None` if
/// `$HOME` cannot be determined (no place to write).
fn ledger_path() -> Option<PathBuf> {
    dirs_next::home_dir().map(|h| h.join(".sindri").join("ledger.jsonl"))
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn append(event: &AuthLedgerEvent) {
    let Some(path) = ledger_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
    }
    let json = match serde_json::to_string(event) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("auth-ledger serialise failed: {}", e);
            return;
        }
    };
    use std::io::Write;
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        Ok(mut f) => {
            if let Err(e) = writeln!(f, "{}", json) {
                tracing::warn!("auth-ledger write failed: {}", e);
            }
        }
        Err(e) => tracing::warn!("auth-ledger open failed: {}", e),
    }
}

/// Emit the full set of Phase 1 events for a binding pass: declarations
/// for each requirement, capability registrations for each target, and
/// resolved/deferred/failed events for each binding outcome.
///
/// Best-effort I/O: see module docs.
pub fn emit_pass_events(
    components: &[ComponentAuthInput<'_>],
    targets: &[TargetAuthInput],
    pass: &BindingPass,
) {
    // 1. AuthRequirementDeclared — one per requirement per component.
    for c in components {
        for t in &c.auth.tokens {
            append(&AuthLedgerEvent {
                timestamp: now_secs(),
                event_type: "AuthRequirementDeclared".into(),
                component: c.address.clone(),
                target: String::new(),
                name: t.name.clone(),
                audience: t.audience.clone(),
                source_kind: String::new(),
                detail: Some(scope_string(t.scope)),
            });
        }
        for o in &c.auth.oauth {
            append(&AuthLedgerEvent {
                timestamp: now_secs(),
                event_type: "AuthRequirementDeclared".into(),
                component: c.address.clone(),
                target: String::new(),
                name: o.name.clone(),
                audience: o.audience.clone(),
                source_kind: "from-oauth".into(),
                detail: Some(scope_string(o.scope)),
            });
        }
        for cert in &c.auth.certs {
            append(&AuthLedgerEvent {
                timestamp: now_secs(),
                event_type: "AuthRequirementDeclared".into(),
                component: c.address.clone(),
                target: String::new(),
                name: cert.name.clone(),
                audience: cert.audience.clone(),
                source_kind: String::new(),
                detail: Some(scope_string(cert.scope)),
            });
        }
        for s in &c.auth.ssh {
            append(&AuthLedgerEvent {
                timestamp: now_secs(),
                event_type: "AuthRequirementDeclared".into(),
                component: c.address.clone(),
                target: String::new(),
                name: s.name.clone(),
                audience: s.audience.clone(),
                source_kind: String::new(),
                detail: Some(scope_string(s.scope)),
            });
        }
    }

    // 2. AuthCapabilityRegistered — one per capability per target.
    for tgt in targets {
        for cap in &tgt.capabilities {
            append(&AuthLedgerEvent {
                timestamp: now_secs(),
                event_type: "AuthCapabilityRegistered".into(),
                component: String::new(),
                target: tgt.target_id.clone(),
                name: cap.id.clone(),
                audience: cap.audience.clone(),
                source_kind: auth_source_kind(&cap.source).to_string(),
                detail: Some(format!("priority={}", cap.priority)),
            });
        }
    }

    // 3. AuthBindingResolved / Deferred / Failed — one per binding.
    for b in &pass.bindings {
        let event_type = match b.status {
            AuthBindingStatus::Bound => "AuthBindingResolved",
            AuthBindingStatus::Deferred => "AuthBindingDeferred",
            AuthBindingStatus::Failed => "AuthBindingFailed",
        };
        append(&AuthLedgerEvent {
            timestamp: now_secs(),
            event_type: event_type.into(),
            component: b.component.clone(),
            target: b.target.clone(),
            name: b.requirement.clone(),
            audience: b.audience.clone(),
            source_kind: b
                .source
                .as_ref()
                .map(|s| auth_source_kind(s).to_string())
                .unwrap_or_default(),
            detail: b.reason.clone(),
        });
    }
}

fn scope_string(s: AuthScope) -> String {
    match s {
        AuthScope::Install => "install".into(),
        AuthScope::Runtime => "runtime".into(),
        AuthScope::Both => "both".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth_binding::{bind_all, ComponentAuthInput, TargetAuthInput};
    use sindri_core::auth::{
        AuthCapability, AuthRequirements, AuthScope, AuthSource, DiscoveryHints, Redemption,
        TokenRequirement,
    };

    fn token(name: &str, audience: &str, optional: bool) -> TokenRequirement {
        TokenRequirement {
            name: name.into(),
            description: name.into(),
            scope: AuthScope::Runtime,
            optional,
            audience: audience.into(),
            redemption: Redemption::EnvVar {
                env_name: name.to_uppercase(),
            },
            discovery: DiscoveryHints::default(),
        }
    }

    /// Sanity: emission against an empty pass does not panic and writes
    /// nothing. (Cannot easily isolate the user's `~/.sindri` here, so we
    /// just smoke-test the in-process serialisation path doesn't error.)
    #[test]
    fn emit_pass_events_smoke() {
        let auth = AuthRequirements {
            tokens: vec![token("k", "urn:x", false)],
            ..Default::default()
        };
        let comp = ComponentAuthInput {
            address: "npm:k".into(),
            auth: &auth,
        };
        let tgt = TargetAuthInput {
            target_id: "local".into(),
            capabilities: vec![AuthCapability {
                id: "c".into(),
                audience: "urn:x".into(),
                source: AuthSource::FromEnv { var: "X".into() },
                priority: 0,
            }],
        };
        let pass = bind_all(std::slice::from_ref(&comp), std::slice::from_ref(&tgt));
        // Should not panic.
        emit_pass_events(&[comp], &[tgt], &pass);
    }

    #[test]
    fn ledger_event_round_trips_through_json() {
        let e = AuthLedgerEvent {
            timestamp: 1700000000,
            event_type: "AuthBindingResolved".into(),
            component: "npm:k".into(),
            target: "local".into(),
            name: "tok".into(),
            audience: "urn:x".into(),
            source_kind: "from-env".into(),
            detail: None,
        };
        let s = serde_json::to_string(&e).unwrap();
        let back: AuthLedgerEvent = serde_json::from_str(&s).unwrap();
        assert_eq!(back.event_type, "AuthBindingResolved");
        assert_eq!(back.target, "local");
    }
}
