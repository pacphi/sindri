//! Extension status command

use anyhow::{Context, Result};
use sindri_core::types::ExtensionState;
use sindri_extensions::StatusLedger;
use tabled::{settings::Style, Table, Tabled};

use super::common::format_event_summary;
use crate::cli::ExtensionStatusArgs;
use crate::output;

#[derive(Tabled, serde::Serialize, serde::Deserialize)]
struct StatusRow {
    name: String,
    version: String,
    status: String,
    #[tabled(rename = "status date/time")]
    status_datetime: String,
}

/// Show installation status for extensions
///
/// Supports:
/// - Show all: `sindri extension status`
/// - Show specific: `sindri extension status python`
/// - JSON output: `sindri extension status --json`
/// - Event history: `sindri extension status python --limit 10`
/// - Date filtering: `sindri extension status python --since 2026-02-10T00:00:00Z`
/// - Verification: `sindri extension status --verify` (slower, checks actual installation)
pub(super) async fn run(args: ExtensionStatusArgs) -> Result<()> {
    if let Some(name) = &args.name {
        output::info(&format!("Checking status of extension: {}", name));
    } else {
        output::info("Checking status of all installed extensions");
    }

    // Load status from ledger
    let ledger = StatusLedger::load_default().context("Failed to load status ledger")?;
    let status_map = ledger
        .get_all_latest_status()
        .context("Failed to get extension status")?;

    // Filter by name if specified
    let entries: Vec<_> = if let Some(filter_name) = &args.name {
        status_map
            .iter()
            .filter(|(name, _)| *name == filter_name)
            .collect()
    } else {
        status_map.iter().collect()
    };

    if entries.is_empty() {
        if let Some(name) = &args.name {
            output::warning(&format!("Extension '{name}' is not installed"));
        } else {
            output::info("No extensions installed yet");
            output::info("Install extensions with: sindri extension install <name>");
        }
        return Ok(());
    }

    // Convert to status rows
    let mut statuses: Vec<StatusRow> = Vec::new();

    for (name, ext_status) in &entries {
        let version_str = ext_status.version.clone().unwrap_or_default();

        // Determine status string from ledger state
        let status_str = if args.verify && ext_status.current_state == ExtensionState::Installed {
            // --verify flag: run actual verification checks (slower)
            use sindri_extensions::{find_extension_yaml, verify_extension_installed};

            if let Some(yaml_path) = find_extension_yaml(name, &version_str) {
                match std::fs::read_to_string(&yaml_path) {
                    Ok(content) => {
                        match serde_yaml_ng::from_str::<sindri_core::types::Extension>(&content) {
                            Ok(extension) => {
                                if verify_extension_installed(&extension).await {
                                    "installed (verified)".to_string()
                                } else {
                                    "failed (verification)".to_string()
                                }
                            }
                            Err(_) => "failed (parse error)".to_string(),
                        }
                    }
                    Err(_) => "failed (unreadable)".to_string(),
                }
            } else {
                "not installed".to_string()
            }
        } else {
            // Default: trust the ledger state (fast path, no I/O verification)
            match ext_status.current_state {
                ExtensionState::Installed => "installed".to_string(),
                ExtensionState::Failed => "failed".to_string(),
                ExtensionState::Installing => "installing".to_string(),
                ExtensionState::Outdated => "outdated".to_string(),
                ExtensionState::Removing => "removing".to_string(),
            }
        };

        let status_datetime_str = ext_status
            .last_event_time
            .format("%Y-%m-%d %H:%M")
            .to_string();

        statuses.push(StatusRow {
            name: name.to_string(),
            version: version_str,
            status: status_str,
            status_datetime: status_datetime_str,
        });
    }

    if args.json {
        let json = serde_json::to_string_pretty(&statuses)
            .context("Failed to serialize status to JSON")?;
        println!("{}", json);
    } else {
        let mut table = Table::new(&statuses);
        table.with(Style::sharp());
        println!("{}", table);
    }

    // Show event history for a single extension
    if let Some(name) = &args.name {
        // Parse --since filter if provided
        let since_filter = if let Some(since_str) = &args.since {
            let parsed = chrono::DateTime::parse_from_rfc3339(since_str).context(format!(
                "Invalid --since date '{}'. Use ISO 8601 format: 2026-02-10T00:00:00Z",
                since_str
            ))?;
            Some(parsed.with_timezone(&chrono::Utc))
        } else {
            None
        };

        let limit = args.limit.or(Some(20));
        let history = ledger
            .get_extension_history(name, limit)
            .context("Failed to get extension history")?;

        // Apply --since filter
        let history: Vec<_> = if let Some(since) = since_filter {
            history
                .into_iter()
                .filter(|e| e.timestamp >= since)
                .collect()
        } else {
            history
        };

        if history.is_empty() {
            output::info("No event history found");
        } else {
            println!();
            output::header(&format!("Event history for '{name}'"));

            for envelope in &history {
                let ts = envelope.timestamp.format("%Y-%m-%d %H:%M:%S UTC");
                let summary = format_event_summary(&envelope.event);
                println!("  [{ts}] {summary}");
            }

            println!();
            output::info(&format!("{} event(s) shown", history.len()));
        }
    }

    Ok(())
}
