pub mod add;
pub mod apply;
pub mod apply_lifecycle;
pub mod auth;
pub mod backup;
pub mod bom;
pub mod completions;
pub mod diff;
pub mod doctor;
pub mod edit;
pub mod graph;
pub mod init;
pub mod ledger;
pub mod log;
pub mod ls;
pub mod manifest;
pub mod pin;
pub mod plan;
pub mod policy;
pub mod prefer;
pub mod registry;
pub mod remove;
pub mod resolve;
pub mod rollback;
pub mod search;
pub mod secrets;
pub mod self_upgrade;
pub mod show;
pub mod target;
pub mod upgrade;

/// Test-only mutex serialising every test in the `sindri` binary's test
/// binary that mutates the process environment. `std::env::set_var` is not
/// thread-safe; any test calling `env::set_var` or `env::remove_var` must
/// hold this lock first.
#[cfg(test)]
pub(crate) static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
