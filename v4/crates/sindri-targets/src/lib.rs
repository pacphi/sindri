#![allow(dead_code)]

pub mod auth;
pub mod cloud;
pub mod docker;
pub mod error;
pub mod local;
pub mod ssh;
pub mod traits;

pub use auth::AuthValue;
// ADR-027 §1: re-export the target-side capability vocabulary that lives in
// `sindri-core` so target implementations can reach it via this crate's
// public surface (`sindri_targets::AuthCapability`, etc.). Phase 0 only —
// `Target::auth_capabilities()` is added in Phase 1.
pub use docker::DockerTarget;
pub use error::TargetError;
pub use local::LocalTarget;
pub use sindri_core::auth::{Audience, AuthCapability, AuthSource};
pub use ssh::SshTarget;
pub use traits::{PrereqCheck, Target};
