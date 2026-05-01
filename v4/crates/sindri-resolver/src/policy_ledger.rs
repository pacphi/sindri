//! Policy-domain ledger events (F-POL-04, ADR-008 §"Forced overrides and
//! audit trail").
//!
//! Written to the same `~/.sindri/ledger.jsonl` consumed by `sindri log` so
//! operators have one auditable event stream. Kept separate from the
//! auth-binding events in [`crate::ledger`]: license overrides and auth
//! bindings carry different metadata; collapsing them under one envelope
//! would overload field semantics. A future "ledger v2" pass can unify
//! envelopes if event types proliferate.

use crate::license_override::{find_override, LicenseOverride};
use serde::{Deserialize, Serialize};
use sindri_core::lockfile::Lockfile;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// One audit-ledger event for a license-override admission.
///
/// Emitted once per resolved component whose license was named on the
/// command line via `--allow <license>=<reason>`. The event captures both
/// the operator's reason and the component address so a later audit can
/// answer "did this manifest pull anything in via a flag-line waiver?"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseOverrideEvent {
    /// Unix epoch seconds.
    pub timestamp: u64,
    /// Discriminator. Always `"LicenseAllowOverride"`.
    pub event_type: String,
    /// Component address (`backend:name[@qualifier]`) the override admitted.
    pub component: String,
    /// SPDX license id that was waived.
    pub license: String,
    /// Operator-supplied justification. Mandatory at parse time; carried
    /// verbatim into the event.
    pub reason: String,
}

fn ledger_path() -> Option<PathBuf> {
    dirs_next::home_dir().map(|h| h.join(".sindri").join("ledger.jsonl"))
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Best-effort append. Mirrors [`crate::ledger`]'s policy: never fail the
/// caller because the audit trail is unavailable; tracing::warn! and move on.
fn append(event: &LicenseOverrideEvent) {
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
            tracing::warn!("policy-ledger serialise failed: {}", e);
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
                tracing::warn!("policy-ledger write failed: {}", e);
            }
        }
        Err(e) => tracing::warn!("policy-ledger open failed: {}", e),
    }
}

/// Walk the resolved lockfile and emit one [`LicenseOverrideEvent`] per
/// component whose license is in `overrides`.
///
/// Called by the CLI's `resolve` command after a successful resolve. The
/// in-memory policy was extended with the override licenses upstream; this
/// function is purely the audit-trail step.
pub fn emit_license_overrides(lockfile: &Lockfile, overrides: &[LicenseOverride]) {
    if overrides.is_empty() {
        return;
    }
    for c in &lockfile.components {
        let license = match c.manifest.as_ref() {
            Some(m) => m.metadata.license.clone(),
            // No manifest: skip — we cannot attest a license we don't know.
            None => continue,
        };
        if let Some(over) = find_override(&license, overrides) {
            append(&LicenseOverrideEvent {
                timestamp: now_secs(),
                event_type: "LicenseAllowOverride".into(),
                component: c.id.to_address(),
                license: over.license.clone(),
                reason: over.reason.clone(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_round_trips_through_json() {
        let e = LicenseOverrideEvent {
            timestamp: 1700000000,
            event_type: "LicenseAllowOverride".into(),
            component: "npm:foo".into(),
            license: "MPL-2.0".into(),
            reason: "ticket-123".into(),
        };
        let s = serde_json::to_string(&e).unwrap();
        let back: LicenseOverrideEvent = serde_json::from_str(&s).unwrap();
        assert_eq!(back.event_type, "LicenseAllowOverride");
        assert_eq!(back.license, "MPL-2.0");
        assert_eq!(back.reason, "ticket-123");
    }

    #[test]
    fn empty_overrides_short_circuits() {
        let lf = Lockfile::new("hash".into(), "local".into());
        // Should not panic / not write anything.
        emit_license_overrides(&lf, &[]);
    }
}
