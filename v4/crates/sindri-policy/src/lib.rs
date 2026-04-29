#![allow(dead_code)]

pub mod check;
pub mod gate5_auth;
pub mod loader;

pub use check::{check_closure, check_license, PolicyCheckResult};
pub use gate5_auth::{check_gate5, check_gate5_with_env, CurrentEnv, EnvProbe};
pub use loader::{
    load_effective_policy, preset_default, preset_offline, preset_strict, write_global_preset,
    EffectivePolicy,
};
