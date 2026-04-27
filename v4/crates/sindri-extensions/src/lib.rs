//! Capability execution for v4 components (Sprint 4 §4.3).
//!
//! This crate hosts the runtime executors for the three capability families
//! declared by a [`sindri_core::component::ComponentManifest`]:
//!
//! - [`hooks::HooksExecutor`] — `capabilities.hooks.{pre,post}_{install,project_init}`.
//! - [`project_init::ProjectInitExecutor`] — priority-ordered project-init steps
//!   with on-disk state markers.
//! - [`collision::CollisionResolver`] — path-prefix admission (ADR-008 Gate 4)
//!   and overlap detection.
//!
//! See [`docs/plan/implementation-plan.md`](../../../docs/plan/implementation-plan.md)
//! §4.3 for the wave plan and ADR-024 for the lifecycle contract.

pub mod collision;
pub mod error;
pub mod hooks;
pub mod project_init;

pub use collision::{CollisionContext, CollisionPlan, CollisionResolver};
pub use error::ExtensionError;
pub use hooks::{HookContext, HooksExecutor};
pub use project_init::{ComponentRef, ProjectInitContext, ProjectInitExecutor};
