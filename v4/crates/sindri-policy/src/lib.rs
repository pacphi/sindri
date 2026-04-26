#![allow(dead_code)]

pub mod check;
pub mod loader;

pub use check::{check_license, check_closure, PolicyCheckResult};
pub use loader::{load_effective_policy, write_global_preset, preset_default, preset_strict, preset_offline, EffectivePolicy};
