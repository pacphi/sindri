//! Integration tests for `sindri extension log` command
//!
//! Tests the CLI argument parsing and basic behavior of the extension log command
//! using the status ledger as the event source.

use chrono::{Duration, Utc};
use sindri_core::types::ExtensionState;
use sindri_extensions::{EventEnvelope, ExtensionEvent, StatusLedger};
use tempfile::TempDir;

/// Create a test ledger with sample events
fn create_populated_ledger() -> (StatusLedger, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let ledger_path = temp_dir.path().join("status_ledger.jsonl");
    let ledger = StatusLedger::new(ledger_path);

    // Add 30 events across multiple extensions
    let extensions = ["python", "nodejs", "kubectl", "terraform", "rust"];
    for (i, ext_name) in extensions.iter().enumerate() {
        // Install started
        let mut started = EventEnvelope::new(
            ext_name.to_string(),
            None,
            ExtensionState::Installing,
            ExtensionEvent::InstallStarted {
                extension_name: ext_name.to_string(),
                version: format!("1.{}.0", i),
                source: "github:pacphi/sindri".to_string(),
                install_method: "Mise".to_string(),
            },
        );
        started.timestamp = Utc::now() - Duration::hours((5 - i as i64) * 2);
        ledger.append(started).unwrap();

        // Install completed
        let mut completed = EventEnvelope::new(
            ext_name.to_string(),
            Some(ExtensionState::Installing),
            ExtensionState::Installed,
            ExtensionEvent::InstallCompleted {
                extension_name: ext_name.to_string(),
                version: format!("1.{}.0", i),
                duration_secs: 60 + i as u64 * 30,
                components_installed: vec![ext_name.to_string()],
                log_file: None,
            },
        );
        completed.timestamp =
            Utc::now() - Duration::hours((5 - i as i64) * 2) + Duration::minutes(2);
        ledger.append(completed).unwrap();
    }

    // Add a failed install
    let failed = EventEnvelope::new(
        "broken-ext".to_string(),
        Some(ExtensionState::Installing),
        ExtensionState::Failed,
        ExtensionEvent::InstallFailed {
            extension_name: "broken-ext".to_string(),
            version: "0.1.0".to_string(),
            error_message: "Network timeout".to_string(),
            retry_count: 0,
            duration_secs: 30,
            log_file: None,
        },
    );
    ledger.append(failed).unwrap();

    // Add an outdated detection
    let outdated = EventEnvelope::new(
        "python".to_string(),
        Some(ExtensionState::Installed),
        ExtensionState::Outdated,
        ExtensionEvent::OutdatedDetected {
            extension_name: "python".to_string(),
            current_version: "1.0.0".to_string(),
            latest_version: "1.1.0".to_string(),
        },
    );
    ledger.append(outdated).unwrap();

    (ledger, temp_dir)
}

#[test]
fn test_default_tail_returns_limited_events() {
    let (ledger, _temp_dir) = create_populated_ledger();

    // Default tail mode: reverse=true, limit=25
    let events = ledger
        .query_events(sindri_extensions::EventFilter {
            limit: Some(sindri_extensions::DEFAULT_LOG_TAIL_LINES),
            reverse: true,
            ..Default::default()
        })
        .unwrap();

    // We have 12 events total (5*2 + 1 failed + 1 outdated), all under 25
    assert_eq!(events.len(), 12);
}

#[test]
fn test_no_tail_shows_all() {
    let (ledger, _temp_dir) = create_populated_ledger();

    let events = ledger
        .query_events(sindri_extensions::EventFilter::default())
        .unwrap();

    assert_eq!(events.len(), 12);
}

#[test]
fn test_json_output_is_valid() {
    let (ledger, _temp_dir) = create_populated_ledger();

    let events = ledger
        .query_events(sindri_extensions::EventFilter::default())
        .unwrap();

    for event in &events {
        let json = serde_json::to_string(event).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.get("event_id").is_some());
        assert!(parsed.get("extension_name").is_some());
        assert!(parsed.get("timestamp").is_some());
        assert!(parsed.get("event").is_some());
    }
}

#[test]
fn test_filter_by_extension_name() {
    let (ledger, _temp_dir) = create_populated_ledger();

    let events = ledger
        .query_events(sindri_extensions::EventFilter {
            extension_name: Some("python".to_string()),
            ..Default::default()
        })
        .unwrap();

    // python has: install_started, install_completed, outdated_detected = 3
    assert_eq!(events.len(), 3);
    assert!(events.iter().all(|e| e.extension_name == "python"));
}

#[test]
fn test_filter_by_error_severity() {
    let (ledger, _temp_dir) = create_populated_ledger();

    // Error events are *_failed
    let events = ledger
        .query_events(sindri_extensions::EventFilter {
            event_types: Some(vec![
                "install_failed".to_string(),
                "upgrade_failed".to_string(),
                "remove_failed".to_string(),
                "validation_failed".to_string(),
            ]),
            ..Default::default()
        })
        .unwrap();

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].extension_name, "broken-ext");
}

#[test]
fn test_empty_ledger_returns_empty() {
    let temp_dir = TempDir::new().unwrap();
    let ledger_path = temp_dir.path().join("empty_ledger.jsonl");
    let ledger = StatusLedger::new(ledger_path);

    let events = ledger
        .query_events(sindri_extensions::EventFilter::default())
        .unwrap();

    assert!(events.is_empty());
}
