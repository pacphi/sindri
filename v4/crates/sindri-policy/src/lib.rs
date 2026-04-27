#![allow(dead_code)]

pub mod check;
pub mod loader;

pub use check::{check_closure, check_license, PolicyCheckResult};
pub use loader::{
    load_effective_policy, preset_default, preset_offline, preset_strict, write_global_preset,
    EffectivePolicy,
};
