//! Shared helpers for extension subcommands

use anyhow::{Context, Result};
use sindri_extensions::ExtensionEvent;

/// Helper function to get the CLI version
pub(super) fn get_cli_version() -> Result<semver::Version> {
    semver::Version::parse(env!("CARGO_PKG_VERSION")).context("Failed to parse CLI version")
}

/// Format an extension event into a human-readable summary string
pub(super) fn format_event_summary(event: &ExtensionEvent) -> String {
    match event {
        ExtensionEvent::InstallStarted {
            version,
            install_method,
            ..
        } => format!("Install started (v{version}, method: {install_method})"),

        ExtensionEvent::InstallCompleted {
            version,
            duration_secs,
            ..
        } => format!("Install completed (v{version}, {duration_secs}s)"),

        ExtensionEvent::InstallFailed {
            version,
            error_message,
            duration_secs,
            ..
        } => format!("Install failed (v{version}, {duration_secs}s): {error_message}"),

        ExtensionEvent::UpgradeStarted {
            from_version,
            to_version,
            ..
        } => format!("Upgrade started ({from_version} \u{2192} {to_version})"),

        ExtensionEvent::UpgradeCompleted {
            from_version,
            to_version,
            duration_secs,
            ..
        } => format!("Upgrade completed ({from_version} \u{2192} {to_version}, {duration_secs}s)"),

        ExtensionEvent::UpgradeFailed {
            from_version,
            to_version,
            error_message,
            ..
        } => format!("Upgrade failed ({from_version} \u{2192} {to_version}): {error_message}"),

        ExtensionEvent::RemoveStarted { version, .. } => {
            format!("Remove started (v{version})")
        }

        ExtensionEvent::RemoveCompleted {
            version,
            duration_secs,
            ..
        } => format!("Remove completed (v{version}, {duration_secs}s)"),

        ExtensionEvent::RemoveFailed {
            version,
            error_message,
            ..
        } => format!("Remove failed (v{version}): {error_message}"),

        ExtensionEvent::OutdatedDetected {
            current_version,
            latest_version,
            ..
        } => format!("Outdated detected ({current_version} \u{2192} {latest_version})"),

        ExtensionEvent::ValidationSucceeded {
            version,
            validation_type,
            ..
        } => format!("Validation succeeded (v{version}, {validation_type})"),

        ExtensionEvent::ValidationFailed {
            version,
            validation_type,
            error_message,
            ..
        } => format!("Validation failed (v{version}, {validation_type}): {error_message}"),
    }
}

/// Format seconds into human-readable duration
pub(super) fn format_duration(secs: u64) -> String {
    if secs < 60 {
        format!("{}s", secs)
    } else {
        let mins = secs / 60;
        let remaining = secs % 60;
        if remaining == 0 {
            format!("{}m", mins)
        } else {
            format!("{}m {}s", mins, remaining)
        }
    }
}

/// Truncate a string to max width, appending "..." if truncated
pub(super) fn truncate_string(s: &str, max_width: usize) -> String {
    if s.len() <= max_width {
        s.to_string()
    } else if max_width > 3 {
        format!("{}...", &s[..max_width - 3])
    } else {
        s[..max_width].to_string()
    }
}
