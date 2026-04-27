//! Error type for the `sindri-extensions` capability execution surface.
//!
//! Per the Sprint 4 §4.3 plan, every capability executor reports failures via
//! [`ExtensionError`], which carries enough context (component name, command,
//! step) to render an actionable diagnostic without re-plumbing the call site.

use thiserror::Error;

/// Errors produced by capability executors.
///
/// Variants intentionally include the offending component name so that
/// orchestrators can attribute failures back to the manifest that declared
/// them, rather than only naming the lifecycle phase.
#[derive(Debug, Error)]
pub enum ExtensionError {
    /// A `capabilities.hooks.*` command exited non-zero (or could not be
    /// dispatched at all).
    #[error("hook failed for component '{component}': command `{command}` — {detail}")]
    HookFailed {
        /// Component metadata name.
        component: String,
        /// Verbatim command string that failed.
        command: String,
        /// Human-readable reason (target error message, exit summary, etc.).
        detail: String,
    },

    /// A `capabilities.project_init[*]` step failed.
    #[error(
        "project-init step failed for component '{component}' (priority {priority}): \
         command `{command}` — {detail}"
    )]
    ProjectInitFailed {
        /// Component metadata name.
        component: String,
        /// Priority of the failing step (lower = earlier).
        priority: u32,
        /// Verbatim command string that failed.
        command: String,
        /// Human-readable reason.
        detail: String,
    },

    /// A component's declared `collision_handling.path_prefix` violates the
    /// v4 path-prefix admission rule (ADR-008 Gate 4) at apply time.
    #[error(
        "collision violation for component '{component}': prefix `{prefix}` is invalid — {reason}; fix: {fix}"
    )]
    CollisionViolation {
        /// Component metadata name.
        component: String,
        /// The offending `path_prefix` value.
        prefix: String,
        /// Why the prefix was rejected.
        reason: String,
        /// Suggested remediation.
        fix: String,
    },

    /// A `configure` step (env settings or file template) failed (ADR-024).
    #[error("configure failed for component '{component}': {step} — {detail}")]
    ConfigureFailed {
        /// Component metadata name.
        component: String,
        /// Which configure sub-step failed (e.g. `environment[FOO]`, `files[/etc/x.conf]`).
        step: String,
        /// Human-readable reason.
        detail: String,
    },

    /// A `validate` command failed an assertion (ADR-024).
    #[error(
        "validate failed for component '{component}': command `{command}` — \
         expected {expected}, got `{got}`"
    )]
    ValidateFailed {
        /// Component metadata name.
        component: String,
        /// Verbatim command string that failed.
        command: String,
        /// Human-readable description of the failed assertion.
        expected: String,
        /// The actual stdout (truncated) the command produced.
        got: String,
    },

    /// A `remove` step (custom command or file deletion) failed.
    #[error("remove failed for component '{component}': {step} — {detail}")]
    RemoveFailed {
        /// Component metadata name.
        component: String,
        /// Which remove sub-step failed (e.g. `commands[0]`, `files[/etc/x]`).
        step: String,
        /// Human-readable reason.
        detail: String,
    },

    /// Underlying target dispatch failed.
    #[error(transparent)]
    Target(#[from] sindri_targets::error::TargetError),

    /// Filesystem or process I/O failure.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Serialization failure (state file read/write).
    #[error("state serialization failed: {0}")]
    Serde(#[from] serde_json::Error),
}
