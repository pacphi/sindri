//! Ledger management commands

use anyhow::{Context, Result};
use sindri_extensions::StatusLedger;

use crate::cli::{LedgerCommands, LedgerCompactArgs, LedgerExportArgs};
use crate::output;

/// Handle ledger subcommands
pub async fn handle_ledger_command(command: LedgerCommands) -> Result<()> {
    match command {
        LedgerCommands::Compact(args) => compact(args).await,
        LedgerCommands::Export(args) => export(args).await,
        LedgerCommands::Stats => stats().await,
    }
}

async fn compact(args: LedgerCompactArgs) -> Result<()> {
    output::info(&format!(
        "Compacting ledger (retaining {} days of events)...",
        args.retention_days
    ));

    let ledger = StatusLedger::load_default().context("Failed to load status ledger")?;
    let removed = ledger
        .compact(args.retention_days)
        .context("Failed to compact ledger")?;

    output::success(&format!("Compacted ledger: removed {} old events", removed));
    Ok(())
}

async fn export(args: LedgerExportArgs) -> Result<()> {
    output::info(&format!("Exporting ledger to: {}", args.path));

    let ledger = StatusLedger::load_default().context("Failed to load status ledger")?;
    let all_events = ledger
        .get_events_since(chrono::DateTime::<chrono::Utc>::MIN_UTC)
        .context("Failed to read ledger events")?;

    let json = serde_json::to_string_pretty(&all_events).context("Failed to serialize events")?;
    std::fs::write(&args.path, json).context("Failed to write export file")?;

    output::success(&format!(
        "Exported {} events to {}",
        all_events.len(),
        args.path
    ));
    Ok(())
}

async fn stats() -> Result<()> {
    let ledger = StatusLedger::load_default().context("Failed to load status ledger")?;
    let stats = ledger.get_stats().context("Failed to get ledger stats")?;

    output::header("Ledger Statistics");
    println!();
    output::kv("Total Events", &stats.total_events.to_string());
    output::kv("File Size", &format_bytes(stats.file_size_bytes));

    if let Some(oldest) = stats.oldest_timestamp {
        output::kv(
            "Oldest Event",
            &oldest.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        );
    }
    if let Some(newest) = stats.newest_timestamp {
        output::kv(
            "Newest Event",
            &newest.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        );
    }

    if !stats.event_type_counts.is_empty() {
        println!();
        output::header("Event Type Counts");
        println!();

        let mut counts: Vec<_> = stats.event_type_counts.iter().collect();
        counts.sort_by(|a, b| b.1.cmp(a.1));

        for (event_type, count) in counts {
            output::kv(event_type, &count.to_string());
        }
    }

    Ok(())
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
