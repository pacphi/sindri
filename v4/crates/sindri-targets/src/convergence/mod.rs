//! Terraform-plan-style convergence engine for `sindri target update`.
//!
//! Closes deferred audit item D2 (Wave 5E). The engine reads two
//! *infra documents*:
//!
//! * **Desired** — `targets.<name>.infra` from `sindri.yaml` (free-form
//!   YAML/JSON value defined by the user).
//! * **Recorded** — `sindri.<name>.infra.lock` (YAML, written by
//!   `sindri target create` and updated by `sindri target update`).
//!
//! Both documents are normalised into a flat map of *resources*
//! (keyed by resource name). For each resource the engine produces an
//! [`Action`] classified per-target-kind by a [`TargetSchema`]
//! implementation. Each kind declares which fields are *immutable*
//! (changing them forces a destroy+recreate) and which are *mutable*
//! (changeable in place).
//!
//! The CLI surface lives in `sindri/src/commands/target.rs::update_target`.
//! It uses [`apply_plan`] with a `Confirm` impl wired to stdin (or the
//! `--auto-approve` flag) and an `Applier` that knows how to call
//! provider APIs. Tests inject fakes for both.
//!
//! ## ADRs honoured
//!
//! * ADR-017 — Target abstraction. Each kind decides what is mutable.
//! * ADR-019 — Plugin protocol untouched. Plugins use a permissive
//!   default schema (`PluginSchema`) that classifies *all* changes as
//!   in-place; if a plugin needs immutable-field semantics it can
//!   either ship its own classifier or rely on the user re-running
//!   `target destroy && target create`.

mod confirm;
mod lock;
mod plan;
mod render;
mod schema;

pub use confirm::{AlwaysNoConfirm, AlwaysYesConfirm, Confirm, ScriptedConfirm, StdinConfirm};
pub use lock::{write_lock_atomic, InfraDocument, InfraLock, ResourceState};
pub use plan::{build_plan, classify_resource, Action, Plan, PlanCounts, PlanEntry};
pub use render::{render_plan, RenderOptions};
pub use schema::{
    schema_for_kind, DevPodSchema, DockerSchema, E2bSchema, FlySchema, K8sSchema, LocalSchema,
    NorthflankSchema, PluginSchema, RunPodSchema, SshSchema, TargetSchema, WslSchema,
};

use crate::error::TargetError;

/// Trait abstracting the side-effects required to converge a [`Plan`].
///
/// Implementations are responsible for calling the provider API(s) for
/// each entry. Tests inject `RecordingApplier` to assert on calls.
///
/// All methods receive the *recorded* state for the resource (where
/// available) so the Applier can use IDs from the lock file. They
/// return the *new* recorded state which will be written into the
/// updated lock.
pub trait Applier {
    fn create(
        &mut self,
        name: &str,
        desired: &serde_json::Value,
    ) -> Result<ResourceState, TargetError>;
    fn destroy(&mut self, name: &str, recorded: &ResourceState) -> Result<(), TargetError>;
    fn update_in_place(
        &mut self,
        name: &str,
        recorded: &ResourceState,
        desired: &serde_json::Value,
    ) -> Result<ResourceState, TargetError>;
}

/// Outcome of [`apply_plan`] — exposed mainly so tests can assert on
/// what was applied vs what was skipped due to the prompt.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ApplyOutcome {
    pub applied: usize,
    pub skipped: usize,
    pub destructive_aborted: bool,
}

/// Apply a plan, prompting before each destructive entry unless
/// `auto_approve` is true. Returns the new [`InfraLock`] that should be
/// persisted on success and an [`ApplyOutcome`] summary.
///
/// Plain `target update` (no `--auto-approve`) MUST NOT silently
/// destroy — if the prompt is declined the function aborts before
/// touching anything destructive (Create + InPlaceUpdate entries that
/// were already applied are kept).
pub fn apply_plan<A: Applier, C: Confirm>(
    plan: &Plan,
    lock: &InfraLock,
    applier: &mut A,
    confirm: &mut C,
    auto_approve: bool,
) -> Result<(InfraLock, ApplyOutcome), TargetError> {
    let mut outcome = ApplyOutcome::default();
    let mut new_lock = lock.clone();

    // First pass — gate destructive entries behind a single prompt
    // covering the whole plan. This matches Terraform's UX: one
    // approval per `apply`, not per resource.
    let destructive = plan
        .entries
        .iter()
        .filter(|e| e.action.is_destructive())
        .count();
    if destructive > 0 && !auto_approve {
        let approved = confirm.confirm(&format!(
            "Plan contains {} destructive change(s). Apply?",
            destructive
        ));
        if !approved {
            outcome.destructive_aborted = true;
            outcome.skipped = plan.entries.len();
            return Ok((new_lock, outcome));
        }
    }

    for entry in &plan.entries {
        match &entry.action {
            Action::Noop => {
                // nothing to do; carry recorded state forward
            }
            Action::Create => {
                let desired = entry
                    .desired
                    .as_ref()
                    .ok_or_else(|| TargetError::ExecFailed {
                        target: lock.target_name.clone(),
                        detail: format!("Create entry '{}' missing desired state", entry.name),
                    })?;
                let state = applier.create(&entry.name, desired)?;
                new_lock.resources.insert(entry.name.clone(), state);
                outcome.applied += 1;
            }
            Action::Destroy => {
                let recorded = entry
                    .recorded
                    .as_ref()
                    .ok_or_else(|| TargetError::ExecFailed {
                        target: lock.target_name.clone(),
                        detail: format!("Destroy entry '{}' missing recorded state", entry.name),
                    })?;
                applier.destroy(&entry.name, recorded)?;
                new_lock.resources.remove(&entry.name);
                outcome.applied += 1;
            }
            Action::InPlaceUpdate { .. } => {
                let recorded = entry
                    .recorded
                    .as_ref()
                    .ok_or_else(|| TargetError::ExecFailed {
                        target: lock.target_name.clone(),
                        detail: format!(
                            "InPlaceUpdate entry '{}' missing recorded state",
                            entry.name
                        ),
                    })?;
                let desired = entry
                    .desired
                    .as_ref()
                    .ok_or_else(|| TargetError::ExecFailed {
                        target: lock.target_name.clone(),
                        detail: format!(
                            "InPlaceUpdate entry '{}' missing desired state",
                            entry.name
                        ),
                    })?;
                let state = applier.update_in_place(&entry.name, recorded, desired)?;
                new_lock.resources.insert(entry.name.clone(), state);
                outcome.applied += 1;
            }
            Action::DestroyAndRecreate { .. } => {
                let recorded = entry
                    .recorded
                    .as_ref()
                    .ok_or_else(|| TargetError::ExecFailed {
                        target: lock.target_name.clone(),
                        detail: format!(
                            "DestroyAndRecreate entry '{}' missing recorded state",
                            entry.name
                        ),
                    })?;
                let desired = entry
                    .desired
                    .as_ref()
                    .ok_or_else(|| TargetError::ExecFailed {
                        target: lock.target_name.clone(),
                        detail: format!(
                            "DestroyAndRecreate entry '{}' missing desired state",
                            entry.name
                        ),
                    })?;
                applier.destroy(&entry.name, recorded)?;
                let state = applier.create(&entry.name, desired)?;
                new_lock.resources.insert(entry.name.clone(), state);
                outcome.applied += 1;
            }
        }
    }

    Ok((new_lock, outcome))
}
