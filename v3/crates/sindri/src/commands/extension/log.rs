//! Extension log command

use anyhow::{anyhow, Context, Result};
use sindri_extensions::{EventEnvelope, ExtensionEvent, ExtensionLogWriter, StatusLedger};

use super::common::{format_duration, truncate_string};
use crate::cli::ExtensionLogArgs;
use crate::output;

/// Maximum extension name display width in log output
const LOG_EXTENSION_NAME_WIDTH: usize = 15;

/// Maximum error message display width before truncation
const LOG_ERROR_MESSAGE_WIDTH: usize = 200;

/// Maximum event type column width
const LOG_EVENT_TYPE_WIDTH: usize = 20;

/// View extension event log with filtering, tail, and follow modes
pub(super) async fn run(args: ExtensionLogArgs) -> Result<()> {
    let ledger = StatusLedger::load_default().context("Failed to load status ledger")?;

    // Handle --detail mode
    if let Some(ref event_id) = args.detail {
        return show_event_detail(&ledger, event_id);
    }

    // Parse date filters
    let since = args
        .since
        .as_ref()
        .map(|s| parse_datetime(s))
        .transpose()
        .context("Invalid --since timestamp (expected ISO 8601, e.g. 2026-02-10T00:00:00Z)")?;

    let until = args
        .until
        .as_ref()
        .map(|s| parse_datetime(s))
        .transpose()
        .context("Invalid --until timestamp (expected ISO 8601, e.g. 2026-02-10T23:59:59Z)")?;

    // Expand event type filter
    let event_types = args
        .event_type
        .as_ref()
        .map(|t| parse_event_type_filter(t))
        .transpose()?;

    // Expand severity filter into event types
    let severity_types = args
        .level
        .as_ref()
        .map(|l| parse_severity_filter(l))
        .transpose()?;

    // Merge event type filters: intersect if both specified, union otherwise
    let merged_types = match (event_types, severity_types) {
        (Some(et), Some(st)) => {
            let intersection: Vec<String> = et.into_iter().filter(|e| st.contains(e)).collect();
            if intersection.is_empty() {
                return Err(anyhow!(
                    "No event types match both --type and --level filters"
                ));
            }
            Some(intersection)
        }
        (Some(et), None) => Some(et),
        (None, Some(st)) => Some(st),
        (None, None) => None,
    };

    // Build filter
    let filter = sindri_extensions::EventFilter {
        extension_name: args.extension.clone(),
        event_types: merged_types,
        since,
        until,
        limit: if args.no_tail { None } else { Some(args.lines) },
        reverse: !args.no_tail,
    };

    if args.follow {
        follow_logs(&ledger, filter, args.json).await
    } else {
        show_logs(&ledger, filter, args.json)
    }
}

/// One-shot log display
fn show_logs(
    ledger: &StatusLedger,
    filter: sindri_extensions::EventFilter,
    json: bool,
) -> Result<()> {
    let events = ledger.query_events(filter)?;

    if events.is_empty() {
        output::info("No events found");
        return Ok(());
    }

    for event in &events {
        if json {
            print_log_json(event)?;
        } else {
            print_log_line(event);
        }
    }

    Ok(())
}

/// Follow mode: poll for new events
async fn follow_logs(
    ledger: &StatusLedger,
    initial_filter: sindri_extensions::EventFilter,
    json: bool,
) -> Result<()> {
    use chrono::Utc;

    // Show initial tail
    let events = ledger.query_events(initial_filter)?;
    let mut last_timestamp = None;

    for event in &events {
        if json {
            print_log_json(event)?;
        } else {
            print_log_line(event);
        }
        last_timestamp = Some(event.timestamp);
    }

    if events.is_empty() {
        output::info("No events yet. Waiting for new events... (Ctrl+C to stop)");
    }

    // Poll loop
    let poll_interval = std::time::Duration::from_secs(sindri_extensions::DEFAULT_FOLLOW_POLL_SECS);

    loop {
        tokio::time::sleep(poll_interval).await;

        let poll_filter = sindri_extensions::EventFilter {
            since: last_timestamp.or_else(|| Some(Utc::now())),
            ..Default::default()
        };

        let new_events = ledger.query_events(poll_filter)?;

        for event in &new_events {
            // Skip events we've already seen (since is inclusive)
            if let Some(last_ts) = last_timestamp {
                if event.timestamp <= last_ts {
                    continue;
                }
            }

            if json {
                print_log_json(event)?;
            } else {
                print_log_line(event);
            }
            last_timestamp = Some(event.timestamp);
        }
    }
}

/// Print a single log line with color coding
fn print_log_line(envelope: &EventEnvelope) {
    use console::style;

    let timestamp = envelope.timestamp.format("%Y-%m-%d %H:%M:%S");
    let ext_name = truncate_string(&envelope.extension_name, LOG_EXTENSION_NAME_WIDTH);
    let event_type = StatusLedger::get_event_type_name(&envelope.event);
    let (icon, color) = event_icon_and_color(&event_type);
    let details = format_event_details(&envelope.event);

    let event_display = truncate_string(&event_type, LOG_EVENT_TYPE_WIDTH);

    let colored_event = match color {
        LogColor::Green => format!("{}", style(format!("{} {}", icon, event_display)).green()),
        LogColor::Red => format!("{}", style(format!("{} {}", icon, event_display)).red()),
        LogColor::Yellow => {
            format!("{}", style(format!("{} {}", icon, event_display)).yellow())
        }
        LogColor::Blue => format!("{}", style(format!("{} {}", icon, event_display)).blue()),
    };

    println!(
        "{} {:<width$} {} {}",
        style(timestamp).dim(),
        ext_name,
        colored_event,
        style(details).dim(),
        width = LOG_EXTENSION_NAME_WIDTH,
    );
}

/// Print a single event as JSON
fn print_log_json(envelope: &EventEnvelope) -> Result<()> {
    let json = serde_json::to_string(envelope).context("Failed to serialize event")?;
    println!("{}", json);
    Ok(())
}

/// Color classification for log events
enum LogColor {
    Green,
    Red,
    Yellow,
    Blue,
}

/// Get icon and color for an event type
fn event_icon_and_color(event_type: &str) -> (&'static str, LogColor) {
    match event_type {
        s if s.ends_with("_completed") || s.ends_with("_succeeded") => {
            ("\u{2713}", LogColor::Green)
        }
        s if s.ends_with("_failed") => ("\u{2717}", LogColor::Red),
        "outdated_detected" => ("!", LogColor::Yellow),
        _ => ("\u{25cb}", LogColor::Blue), // *_started
    }
}

/// Format event-specific details (version, duration, error)
fn format_event_details(event: &ExtensionEvent) -> String {
    match event {
        ExtensionEvent::InstallStarted { version, .. } => format!("v{}", version),
        ExtensionEvent::InstallCompleted {
            version,
            duration_secs,
            ..
        } => format!("v{} ({})", version, format_duration(*duration_secs)),
        ExtensionEvent::InstallFailed {
            version,
            error_message,
            duration_secs,
            ..
        } => format!(
            "v{} ({}) {}",
            version,
            format_duration(*duration_secs),
            truncate_string(error_message, LOG_ERROR_MESSAGE_WIDTH)
        ),
        ExtensionEvent::UpgradeStarted {
            from_version,
            to_version,
            ..
        } => format!("v{} -> v{}", from_version, to_version),
        ExtensionEvent::UpgradeCompleted {
            from_version,
            to_version,
            duration_secs,
            ..
        } => format!(
            "v{} -> v{} ({})",
            from_version,
            to_version,
            format_duration(*duration_secs)
        ),
        ExtensionEvent::UpgradeFailed {
            from_version,
            to_version,
            error_message,
            ..
        } => format!(
            "v{} -> v{} {}",
            from_version,
            to_version,
            truncate_string(error_message, LOG_ERROR_MESSAGE_WIDTH)
        ),
        ExtensionEvent::RemoveStarted { version, .. } => format!("v{}", version),
        ExtensionEvent::RemoveCompleted {
            version,
            duration_secs,
            ..
        } => format!("v{} ({})", version, format_duration(*duration_secs)),
        ExtensionEvent::RemoveFailed {
            version,
            error_message,
            ..
        } => format!(
            "v{} {}",
            version,
            truncate_string(error_message, LOG_ERROR_MESSAGE_WIDTH)
        ),
        ExtensionEvent::OutdatedDetected {
            current_version,
            latest_version,
            ..
        } => format!("v{} -> v{} available", current_version, latest_version),
        ExtensionEvent::ValidationSucceeded {
            version,
            validation_type,
            ..
        } => format!("v{} ({})", version, validation_type),
        ExtensionEvent::ValidationFailed {
            version,
            error_message,
            ..
        } => format!(
            "v{} {}",
            version,
            truncate_string(error_message, LOG_ERROR_MESSAGE_WIDTH)
        ),
    }
}

/// Show detailed log output for a specific event by event_id
fn show_event_detail(ledger: &StatusLedger, event_id: &str) -> Result<()> {
    use std::path::Path;

    // Find the event in the ledger
    let all_events = ledger.query_events(sindri_extensions::EventFilter::default())?;
    let envelope = all_events
        .iter()
        .find(|e| e.event_id == event_id)
        .ok_or_else(|| anyhow!("Event not found: {}", event_id))?;

    // Print event summary
    output::header("Event Detail");
    println!();
    output::kv("Event ID", &envelope.event_id);
    output::kv(
        "Timestamp",
        &envelope
            .timestamp
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string(),
    );
    output::kv("Extension", &envelope.extension_name);
    output::kv(
        "Event Type",
        &StatusLedger::get_event_type_name(&envelope.event),
    );
    output::kv("Details", &format_event_details(&envelope.event));
    println!();

    // Extract log_file from the event
    match extract_log_file(&envelope.event) {
        Some(log_path) => {
            let path = Path::new(log_path);
            if path.exists() {
                output::header("Installation Log");
                println!();
                let content =
                    ExtensionLogWriter::read_log(path).context("Failed to read log file")?;
                print!("{}", content);
            } else {
                output::warning(&format!(
                    "Log file no longer exists: {} (may have been cleaned up during compaction)",
                    log_path
                ));
            }
        }
        None => {
            output::info("No log file linked to this event");
        }
    }

    Ok(())
}

/// Extract the log_file path from an event variant, if present
fn extract_log_file(event: &ExtensionEvent) -> Option<&str> {
    match event {
        ExtensionEvent::InstallCompleted { log_file, .. }
        | ExtensionEvent::InstallFailed { log_file, .. }
        | ExtensionEvent::UpgradeCompleted { log_file, .. }
        | ExtensionEvent::UpgradeFailed { log_file, .. }
        | ExtensionEvent::RemoveCompleted { log_file, .. }
        | ExtensionEvent::RemoveFailed { log_file, .. } => log_file.as_deref(),
        _ => None,
    }
}

/// Parse an event type group into individual event type names
fn parse_event_type_filter(type_str: &str) -> Result<Vec<String>> {
    let types: Vec<String> = match type_str.to_lowercase().as_str() {
        "install" => vec![
            "install_started".to_string(),
            "install_completed".to_string(),
            "install_failed".to_string(),
        ],
        "upgrade" => vec![
            "upgrade_started".to_string(),
            "upgrade_completed".to_string(),
            "upgrade_failed".to_string(),
        ],
        "remove" => vec![
            "remove_started".to_string(),
            "remove_completed".to_string(),
            "remove_failed".to_string(),
        ],
        "validation" => vec![
            "validation_succeeded".to_string(),
            "validation_failed".to_string(),
        ],
        "outdated" => vec!["outdated_detected".to_string()],
        other => {
            return Err(anyhow!(
            "Unknown event type '{}'. Valid types: install, upgrade, remove, validation, outdated",
            other
        ))
        }
    };
    Ok(types)
}

/// Parse a severity level into matching event type names
fn parse_severity_filter(level: &str) -> Result<Vec<String>> {
    let types: Vec<String> = match level.to_lowercase().as_str() {
        "info" => vec![
            "install_started".to_string(),
            "install_completed".to_string(),
            "upgrade_started".to_string(),
            "upgrade_completed".to_string(),
            "remove_started".to_string(),
            "remove_completed".to_string(),
            "validation_succeeded".to_string(),
        ],
        "warn" => vec!["outdated_detected".to_string()],
        "error" => vec![
            "install_failed".to_string(),
            "upgrade_failed".to_string(),
            "remove_failed".to_string(),
            "validation_failed".to_string(),
        ],
        other => {
            return Err(anyhow!(
                "Unknown severity '{}'. Valid levels: info, warn, error",
                other
            ))
        }
    };
    Ok(types)
}

/// Parse an ISO 8601 datetime string
fn parse_datetime(s: &str) -> Result<chrono::DateTime<chrono::Utc>> {
    // Try full RFC 3339 first
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&chrono::Utc));
    }

    // Try date-only format (YYYY-MM-DD)
    if let Ok(date) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let dt = date.and_hms_opt(0, 0, 0).context("Invalid date")?;
        return Ok(chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
            dt,
            chrono::Utc,
        ));
    }

    Err(anyhow!(
        "Cannot parse '{}' as datetime. Use ISO 8601 format (e.g. 2026-02-10 or 2026-02-10T12:00:00Z)",
        s
    ))
}
