//! Integration tests for event-driven ledger system

use sindri_core::types::ExtensionState;
use sindri_extensions::events::{EventEnvelope, ExtensionEvent};
use sindri_extensions::ledger::StatusLedger;
use tempfile::TempDir;

fn create_test_ledger() -> (StatusLedger, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let ledger_path = temp_dir.path().join("status_ledger.jsonl");
    let ledger = StatusLedger::new(ledger_path);
    (ledger, temp_dir)
}

#[test]
fn test_install_lifecycle_events() {
    let (ledger, _temp) = create_test_ledger();

    // Publish InstallStarted event
    let started = EventEnvelope::new(
        "python".to_string(),
        None,
        ExtensionState::Installing,
        ExtensionEvent::InstallStarted {
            extension_name: "python".to_string(),
            version: "3.13.0".to_string(),
            source: "github:pacphi/sindri".to_string(),
            install_method: "Mise".to_string(),
        },
    );
    ledger.append(started).unwrap();

    // Publish InstallCompleted event
    let completed = EventEnvelope::new(
        "python".to_string(),
        Some(ExtensionState::Installing),
        ExtensionState::Installed,
        ExtensionEvent::InstallCompleted {
            extension_name: "python".to_string(),
            version: "3.13.0".to_string(),
            duration_secs: 150,
            components_installed: vec!["python".to_string(), "pip".to_string()],
            log_file: None,
        },
    );
    ledger.append(completed).unwrap();

    // Verify 2 events in history
    let history = ledger.get_extension_history("python", None).unwrap();
    assert_eq!(history.len(), 2);

    // Verify latest status is Installed
    let status_map = ledger.get_all_latest_status().unwrap();
    let status = status_map.get("python").unwrap();
    assert_eq!(status.current_state, ExtensionState::Installed);
    assert_eq!(status.version, Some("3.13.0".to_string()));
}

#[test]
fn test_failed_install_events() {
    let (ledger, _temp) = create_test_ledger();

    // Publish InstallStarted
    let started = EventEnvelope::new(
        "kubectl".to_string(),
        None,
        ExtensionState::Installing,
        ExtensionEvent::InstallStarted {
            extension_name: "kubectl".to_string(),
            version: "1.35.0".to_string(),
            source: "github:pacphi/sindri".to_string(),
            install_method: "Binary".to_string(),
        },
    );
    ledger.append(started).unwrap();

    // Publish InstallFailed
    let failed = EventEnvelope::new(
        "kubectl".to_string(),
        Some(ExtensionState::Installing),
        ExtensionState::Failed,
        ExtensionEvent::InstallFailed {
            extension_name: "kubectl".to_string(),
            version: "1.35.0".to_string(),
            error_message: "Network timeout".to_string(),
            retry_count: 0,
            duration_secs: 120,
            log_file: None,
        },
    );
    ledger.append(failed).unwrap();

    // Verify 2 events
    let history = ledger.get_extension_history("kubectl", None).unwrap();
    assert_eq!(history.len(), 2);

    // Verify status is Failed
    let status_map = ledger.get_all_latest_status().unwrap();
    let status = status_map.get("kubectl").unwrap();
    assert_eq!(status.current_state, ExtensionState::Failed);
}

#[test]
fn test_upgrade_lifecycle_events() {
    let (ledger, _temp) = create_test_ledger();

    // Initial install
    ledger
        .append(EventEnvelope::new(
            "python".to_string(),
            None,
            ExtensionState::Installed,
            ExtensionEvent::InstallCompleted {
                extension_name: "python".to_string(),
                version: "3.12.0".to_string(),
                duration_secs: 100,
                components_installed: vec![],
                log_file: None,
            },
        ))
        .unwrap();

    // Upgrade started
    ledger
        .append(EventEnvelope::new(
            "python".to_string(),
            Some(ExtensionState::Installed),
            ExtensionState::Installing,
            ExtensionEvent::UpgradeStarted {
                extension_name: "python".to_string(),
                from_version: "3.12.0".to_string(),
                to_version: "3.13.0".to_string(),
            },
        ))
        .unwrap();

    // Upgrade completed
    ledger
        .append(EventEnvelope::new(
            "python".to_string(),
            Some(ExtensionState::Installing),
            ExtensionState::Installed,
            ExtensionEvent::UpgradeCompleted {
                extension_name: "python".to_string(),
                from_version: "3.12.0".to_string(),
                to_version: "3.13.0".to_string(),
                duration_secs: 90,
                log_file: None,
            },
        ))
        .unwrap();

    // Verify version updated
    let status_map = ledger.get_all_latest_status().unwrap();
    let status = status_map.get("python").unwrap();
    assert_eq!(status.current_state, ExtensionState::Installed);
    assert_eq!(status.version, Some("3.13.0".to_string()));

    // Verify all events recorded
    let history = ledger.get_extension_history("python", None).unwrap();
    assert_eq!(history.len(), 3);
}

#[test]
fn test_jsonl_format() {
    let (ledger, temp) = create_test_ledger();
    let ledger_path = temp.path().join("status_ledger.jsonl");

    // Append events
    ledger
        .append(EventEnvelope::new(
            "python".to_string(),
            None,
            ExtensionState::Installing,
            ExtensionEvent::InstallStarted {
                extension_name: "python".to_string(),
                version: "3.13.0".to_string(),
                source: "github:pacphi/sindri".to_string(),
                install_method: "Mise".to_string(),
            },
        ))
        .unwrap();

    ledger
        .append(EventEnvelope::new(
            "nodejs".to_string(),
            None,
            ExtensionState::Installing,
            ExtensionEvent::InstallStarted {
                extension_name: "nodejs".to_string(),
                version: "22.0.0".to_string(),
                source: "github:pacphi/sindri".to_string(),
                install_method: "Mise".to_string(),
            },
        ))
        .unwrap();

    // Read file and verify JSON Lines format (one JSON object per line)
    let content = std::fs::read_to_string(&ledger_path).unwrap();
    let lines: Vec<&str> = content.trim().split('\n').collect();
    assert_eq!(lines.len(), 2);

    // Each line should be valid JSON
    for line in &lines {
        serde_json::from_str::<serde_json::Value>(line).unwrap();
    }
}

#[test]
fn test_empty_ledger_status() {
    let (ledger, _temp) = create_test_ledger();

    // Empty ledger should return empty map
    let status_map = ledger.get_all_latest_status().unwrap();
    assert!(status_map.is_empty());

    // Empty history
    let history = ledger.get_extension_history("nonexistent", None).unwrap();
    assert!(history.is_empty());
}

#[test]
fn test_multiple_extensions_status() {
    let (ledger, _temp) = create_test_ledger();

    // Install 5 extensions
    for name in ["python", "nodejs", "kubectl", "terraform", "docker"] {
        ledger
            .append(EventEnvelope::new(
                name.to_string(),
                None,
                ExtensionState::Installed,
                ExtensionEvent::InstallCompleted {
                    extension_name: name.to_string(),
                    version: "1.0.0".to_string(),
                    duration_secs: 10,
                    components_installed: vec![],
                    log_file: None,
                },
            ))
            .unwrap();
    }

    // All 5 should show as installed
    let status_map = ledger.get_all_latest_status().unwrap();
    assert_eq!(status_map.len(), 5);
    for status in status_map.values() {
        assert_eq!(status.current_state, ExtensionState::Installed);
    }
}

#[test]
fn test_validation_events() {
    let (ledger, _temp) = create_test_ledger();

    // Install extension
    ledger
        .append(EventEnvelope::new(
            "python".to_string(),
            None,
            ExtensionState::Installed,
            ExtensionEvent::InstallCompleted {
                extension_name: "python".to_string(),
                version: "3.13.0".to_string(),
                duration_secs: 100,
                components_installed: vec![],
                log_file: None,
            },
        ))
        .unwrap();

    // Validation succeeded
    ledger
        .append(EventEnvelope::new(
            "python".to_string(),
            Some(ExtensionState::Installed),
            ExtensionState::Installed,
            ExtensionEvent::ValidationSucceeded {
                extension_name: "python".to_string(),
                version: "3.13.0".to_string(),
                validation_type: "manual".to_string(),
            },
        ))
        .unwrap();

    // Still installed after validation
    let status_map = ledger.get_all_latest_status().unwrap();
    let status = status_map.get("python").unwrap();
    assert_eq!(status.current_state, ExtensionState::Installed);

    // Both events in history
    let history = ledger.get_extension_history("python", None).unwrap();
    assert_eq!(history.len(), 2);
}

#[test]
fn test_ledger_stats() {
    let (ledger, _temp) = create_test_ledger();

    // Add various events
    ledger
        .append(EventEnvelope::new(
            "python".to_string(),
            None,
            ExtensionState::Installing,
            ExtensionEvent::InstallStarted {
                extension_name: "python".to_string(),
                version: "3.13.0".to_string(),
                source: "github:pacphi/sindri".to_string(),
                install_method: "Mise".to_string(),
            },
        ))
        .unwrap();

    ledger
        .append(EventEnvelope::new(
            "python".to_string(),
            Some(ExtensionState::Installing),
            ExtensionState::Installed,
            ExtensionEvent::InstallCompleted {
                extension_name: "python".to_string(),
                version: "3.13.0".to_string(),
                duration_secs: 150,
                components_installed: vec![],
                log_file: None,
            },
        ))
        .unwrap();

    let stats = ledger.get_stats().unwrap();
    assert_eq!(stats.total_events, 2);
    assert!(stats.file_size_bytes > 0);
    assert!(stats.oldest_timestamp.is_some());
    assert!(stats.newest_timestamp.is_some());
    assert_eq!(stats.event_type_counts.get("install_started"), Some(&1));
    assert_eq!(stats.event_type_counts.get("install_completed"), Some(&1));
}
