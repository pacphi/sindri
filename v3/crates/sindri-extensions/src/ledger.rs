use crate::events::{EventEnvelope, ExtensionEvent};
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use fs4::fs_std::FileExt;
use serde::{Deserialize, Serialize};
use sindri_core::types::ExtensionState;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

/// Extension status derived from ledger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionStatus {
    pub extension_name: String,
    pub current_state: ExtensionState,
    pub last_event_time: DateTime<Utc>,
    pub last_event_id: String,
    pub version: Option<String>,
}

/// Filter criteria for querying events from the ledger
#[derive(Debug, Default)]
pub struct EventFilter {
    /// Filter by extension name
    pub extension_name: Option<String>,
    /// Filter by event type names (e.g., "install_started", "install_completed")
    pub event_types: Option<Vec<String>>,
    /// Only events after this timestamp
    pub since: Option<DateTime<Utc>>,
    /// Only events before this timestamp
    pub until: Option<DateTime<Utc>>,
    /// Maximum number of events to return
    pub limit: Option<usize>,
    /// If true, return the most recent N events (tail mode)
    pub reverse: bool,
}

/// Default number of events shown in tail mode
pub const DEFAULT_LOG_TAIL_LINES: usize = 25;

/// Default poll interval for follow mode (seconds)
pub const DEFAULT_FOLLOW_POLL_SECS: u64 = 1;

/// Default auto-compaction interval (every N operations)
const AUTO_COMPACT_INTERVAL: usize = 100;

/// Default retention period in days for auto-compaction
const AUTO_COMPACT_RETENTION_DAYS: i64 = 90;

/// Status ledger implementation
pub struct StatusLedger {
    ledger_path: PathBuf,
}

impl StatusLedger {
    /// Create/load ledger from default location (~/.sindri/status_ledger.jsonl)
    pub fn load_default() -> Result<Self> {
        let home_dir = dirs::home_dir().context("Failed to get home directory")?;
        let sindri_dir = home_dir.join(".sindri");
        fs::create_dir_all(&sindri_dir).context("Failed to create .sindri directory")?;

        let ledger_path = sindri_dir.join("status_ledger.jsonl");
        Ok(Self { ledger_path })
    }

    /// Create ledger from custom path
    pub fn new(ledger_path: PathBuf) -> Self {
        Self { ledger_path }
    }

    /// Append event to ledger (atomic, file-locked) with auto-compaction
    pub fn append(&self, event: EventEnvelope) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.ledger_path.parent() {
            fs::create_dir_all(parent).context("Failed to create ledger parent directory")?;
        }

        {
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.ledger_path)
                .context("Failed to open ledger file")?;

            // Acquire exclusive lock (released on drop)
            file.lock_exclusive()
                .context("Failed to acquire exclusive lock on ledger")?;

            let json_line = serde_json::to_string(&event).context("Failed to serialize event")?;
            writeln!(file, "{}", json_line).context("Failed to write event to ledger")?;
            file.sync_all().context("Failed to sync ledger file")?; // Ensure durability

            // File lock is released here when `file` is dropped
        }

        // After successful append, check if auto-compaction is needed.
        // This runs outside the file lock so compaction can acquire its own locks.
        self.maybe_auto_compact();

        Ok(())
    }

    /// Auto-compact if total events reaches the compaction interval threshold.
    /// Failures are logged but do not propagate -- auto-compaction is best-effort.
    fn maybe_auto_compact(&self) {
        let event_count = match self.count_events() {
            Ok(count) => count,
            Err(e) => {
                tracing::warn!("Auto-compaction skipped: failed to count events: {}", e);
                return;
            }
        };

        if event_count > 0 && event_count % AUTO_COMPACT_INTERVAL == 0 {
            tracing::debug!("Auto-compacting ledger ({} events)", event_count);
            match self.compact(AUTO_COMPACT_RETENTION_DAYS) {
                Ok(removed) => {
                    if removed > 0 {
                        tracing::info!("Auto-compacted ledger: removed {} old events", removed);
                    }
                }
                Err(e) => {
                    tracing::warn!("Auto-compaction failed: {}", e);
                }
            }
        }
    }

    /// Count total non-empty lines in the ledger file (fast, no JSON parsing).
    fn count_events(&self) -> Result<usize> {
        if !self.ledger_path.exists() {
            return Ok(0);
        }

        let file =
            fs::File::open(&self.ledger_path).context("Failed to open ledger file for counting")?;
        let reader = BufReader::new(file);
        let count = reader
            .lines()
            .filter(|l| l.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false))
            .count();
        Ok(count)
    }

    /// Get latest status for all extensions (aggregate query)
    pub fn get_all_latest_status(&self) -> Result<HashMap<String, ExtensionStatus>> {
        if !self.ledger_path.exists() {
            return Ok(HashMap::new());
        }

        let file = fs::File::open(&self.ledger_path).context("Failed to open ledger file")?;
        let reader = BufReader::new(file);

        let mut status_map: HashMap<String, ExtensionStatus> = HashMap::new();

        for line in reader.lines() {
            let line = line.context("Failed to read line from ledger")?;
            if line.trim().is_empty() {
                continue;
            }

            let envelope: EventEnvelope =
                serde_json::from_str(&line).context("Failed to deserialize event from ledger")?;

            // Extract version from event
            let version = Self::extract_version_from_event(&envelope.event);

            // Update or insert status for this extension
            status_map
                .entry(envelope.extension_name.clone())
                .and_modify(|status| {
                    // Update only if this event is newer
                    if envelope.timestamp > status.last_event_time {
                        status.current_state = envelope.state_after;
                        status.last_event_time = envelope.timestamp;
                        status.last_event_id = envelope.event_id.clone();
                        if let Some(v) = &version {
                            status.version = Some(v.clone());
                        }
                    }
                })
                .or_insert_with(|| ExtensionStatus {
                    extension_name: envelope.extension_name.clone(),
                    current_state: envelope.state_after,
                    last_event_time: envelope.timestamp,
                    last_event_id: envelope.event_id.clone(),
                    version,
                });
        }

        Ok(status_map)
    }

    /// Get event history for specific extension (chronological)
    pub fn get_extension_history(
        &self,
        name: &str,
        limit: Option<usize>,
    ) -> Result<Vec<EventEnvelope>> {
        if !self.ledger_path.exists() {
            return Ok(Vec::new());
        }

        let file = fs::File::open(&self.ledger_path).context("Failed to open ledger file")?;
        let reader = BufReader::new(file);

        let mut events = Vec::new();

        for line in reader.lines() {
            let line = line.context("Failed to read line from ledger")?;
            if line.trim().is_empty() {
                continue;
            }

            let envelope: EventEnvelope =
                serde_json::from_str(&line).context("Failed to deserialize event from ledger")?;

            if envelope.extension_name == name {
                events.push(envelope);
            }
        }

        // Apply limit if specified (most recent events)
        if let Some(limit_val) = limit {
            if events.len() > limit_val {
                events = events.split_off(events.len() - limit_val);
            }
        }

        Ok(events)
    }

    /// Get events since timestamp
    pub fn get_events_since(&self, since: DateTime<Utc>) -> Result<Vec<EventEnvelope>> {
        if !self.ledger_path.exists() {
            return Ok(Vec::new());
        }

        let file = fs::File::open(&self.ledger_path).context("Failed to open ledger file")?;
        let reader = BufReader::new(file);

        let mut events = Vec::new();

        for line in reader.lines() {
            let line = line.context("Failed to read line from ledger")?;
            if line.trim().is_empty() {
                continue;
            }

            let envelope: EventEnvelope =
                serde_json::from_str(&line).context("Failed to deserialize event from ledger")?;

            if envelope.timestamp >= since {
                events.push(envelope);
            }
        }

        Ok(events)
    }

    /// Compact ledger (prune old events, keep retention period history)
    pub fn compact(&self, retention_days: i64) -> Result<usize> {
        if !self.ledger_path.exists() {
            return Ok(0);
        }

        let cutoff_time = Utc::now() - Duration::days(retention_days);
        let temp_path = self.ledger_path.with_extension("jsonl.tmp");

        let file = fs::File::open(&self.ledger_path).context("Failed to open ledger file")?;
        let reader = BufReader::new(file);

        let mut temp_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&temp_path)
            .context("Failed to create temp ledger file")?;

        let mut removed_count = 0;

        // Keep track of latest event per extension (always keep these)
        let latest_events = self.get_all_latest_status()?;
        let latest_event_ids: HashMap<String, String> = latest_events
            .into_iter()
            .map(|(name, status)| (name, status.last_event_id))
            .collect();

        for line in reader.lines() {
            let line = line.context("Failed to read line from ledger")?;
            if line.trim().is_empty() {
                continue;
            }

            let envelope: EventEnvelope =
                serde_json::from_str(&line).context("Failed to deserialize event from ledger")?;

            // Keep event if:
            // 1. It's within retention period, OR
            // 2. It's the latest event for its extension
            let is_latest = latest_event_ids
                .get(&envelope.extension_name)
                .map(|id| id == &envelope.event_id)
                .unwrap_or(false);

            if envelope.timestamp >= cutoff_time || is_latest {
                writeln!(temp_file, "{}", line).context("Failed to write to temp ledger")?;
            } else {
                removed_count += 1;
            }
        }

        temp_file.sync_all().context("Failed to sync temp ledger")?;
        drop(temp_file);

        // Replace original with compacted version
        fs::rename(&temp_path, &self.ledger_path)
            .context("Failed to replace ledger with compacted version")?;

        Ok(removed_count)
    }

    /// Get ledger statistics
    pub fn get_stats(&self) -> Result<LedgerStats> {
        if !self.ledger_path.exists() {
            return Ok(LedgerStats::default());
        }

        let file = fs::File::open(&self.ledger_path).context("Failed to open ledger file")?;
        let reader = BufReader::new(file);

        let mut total_events = 0;
        let mut oldest_timestamp: Option<DateTime<Utc>> = None;
        let mut newest_timestamp: Option<DateTime<Utc>> = None;
        let mut event_type_counts: HashMap<String, usize> = HashMap::new();

        for line in reader.lines() {
            let line = line.context("Failed to read line from ledger")?;
            if line.trim().is_empty() {
                continue;
            }

            let envelope: EventEnvelope =
                serde_json::from_str(&line).context("Failed to deserialize event from ledger")?;

            total_events += 1;

            // Track oldest/newest timestamps
            if oldest_timestamp.is_none() || envelope.timestamp < oldest_timestamp.unwrap() {
                oldest_timestamp = Some(envelope.timestamp);
            }
            if newest_timestamp.is_none() || envelope.timestamp > newest_timestamp.unwrap() {
                newest_timestamp = Some(envelope.timestamp);
            }

            // Count event types
            let event_type = Self::get_event_type_name(&envelope.event);
            *event_type_counts.entry(event_type).or_insert(0) += 1;
        }

        let file_size = fs::metadata(&self.ledger_path)
            .context("Failed to get ledger file metadata")?
            .len();

        Ok(LedgerStats {
            total_events,
            file_size_bytes: file_size,
            oldest_timestamp,
            newest_timestamp,
            event_type_counts,
        })
    }

    /// Query events from the ledger with filtering
    pub fn query_events(&self, filter: EventFilter) -> Result<Vec<EventEnvelope>> {
        if !self.ledger_path.exists() {
            return Ok(Vec::new());
        }

        let file = fs::File::open(&self.ledger_path).context("Failed to open ledger file")?;
        let reader = BufReader::new(file);

        let mut events = Vec::new();

        for line in reader.lines() {
            let line = line.context("Failed to read line from ledger")?;
            if line.trim().is_empty() {
                continue;
            }

            let envelope: EventEnvelope =
                serde_json::from_str(&line).context("Failed to deserialize event from ledger")?;

            // Apply extension name filter
            if let Some(ref name) = filter.extension_name {
                if envelope.extension_name != *name {
                    continue;
                }
            }

            // Apply event type filter
            if let Some(ref types) = filter.event_types {
                let event_type = Self::get_event_type_name(&envelope.event);
                if !types.contains(&event_type) {
                    continue;
                }
            }

            // Apply since filter
            if let Some(since) = filter.since {
                if envelope.timestamp < since {
                    continue;
                }
            }

            // Apply until filter
            if let Some(until) = filter.until {
                if envelope.timestamp > until {
                    continue;
                }
            }

            events.push(envelope);
        }

        // Handle tail mode: reverse + limit gives last N events in chronological order
        if filter.reverse {
            if let Some(limit) = filter.limit {
                if events.len() > limit {
                    events = events.split_off(events.len() - limit);
                }
            }
        } else if let Some(limit) = filter.limit {
            events.truncate(limit);
        }

        Ok(events)
    }

    /// Extract version from event payload
    fn extract_version_from_event(event: &ExtensionEvent) -> Option<String> {
        match event {
            ExtensionEvent::InstallStarted { version, .. }
            | ExtensionEvent::InstallCompleted { version, .. }
            | ExtensionEvent::InstallFailed { version, .. }
            | ExtensionEvent::RemoveStarted { version, .. }
            | ExtensionEvent::RemoveCompleted { version, .. }
            | ExtensionEvent::RemoveFailed { version, .. }
            | ExtensionEvent::ValidationSucceeded { version, .. }
            | ExtensionEvent::ValidationFailed { version, .. } => Some(version.clone()),
            ExtensionEvent::UpgradeCompleted { to_version, .. }
            | ExtensionEvent::UpgradeFailed { to_version, .. } => Some(to_version.clone()),
            ExtensionEvent::UpgradeStarted { to_version, .. } => Some(to_version.clone()),
            ExtensionEvent::OutdatedDetected { latest_version, .. } => Some(latest_version.clone()),
        }
    }

    /// Get event type name for statistics and display
    pub fn get_event_type_name(event: &ExtensionEvent) -> String {
        match event {
            ExtensionEvent::InstallStarted { .. } => "install_started",
            ExtensionEvent::InstallCompleted { .. } => "install_completed",
            ExtensionEvent::InstallFailed { .. } => "install_failed",
            ExtensionEvent::UpgradeStarted { .. } => "upgrade_started",
            ExtensionEvent::UpgradeCompleted { .. } => "upgrade_completed",
            ExtensionEvent::UpgradeFailed { .. } => "upgrade_failed",
            ExtensionEvent::RemoveStarted { .. } => "remove_started",
            ExtensionEvent::RemoveCompleted { .. } => "remove_completed",
            ExtensionEvent::RemoveFailed { .. } => "remove_failed",
            ExtensionEvent::OutdatedDetected { .. } => "outdated_detected",
            ExtensionEvent::ValidationSucceeded { .. } => "validation_succeeded",
            ExtensionEvent::ValidationFailed { .. } => "validation_failed",
        }
        .to_string()
    }
}

/// Ledger statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LedgerStats {
    pub total_events: usize,
    pub file_size_bytes: u64,
    pub oldest_timestamp: Option<DateTime<Utc>>,
    pub newest_timestamp: Option<DateTime<Utc>>,
    pub event_type_counts: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::ExtensionEvent;
    use std::thread;
    use tempfile::TempDir;

    fn create_test_ledger() -> (StatusLedger, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let ledger_path = temp_dir.path().join("status_ledger.jsonl");
        let ledger = StatusLedger::new(ledger_path);
        (ledger, temp_dir)
    }

    #[test]
    fn test_append_event() {
        let (ledger, _temp_dir) = create_test_ledger();

        let event = ExtensionEvent::InstallStarted {
            extension_name: "python".to_string(),
            version: "3.13.0".to_string(),
            source: "github:pacphi/sindri".to_string(),
            install_method: "Mise".to_string(),
        };

        let envelope = EventEnvelope::new(
            "python".to_string(),
            None,
            ExtensionState::Installing,
            event,
        );

        ledger.append(envelope).unwrap();

        // Verify file was created
        assert!(ledger.ledger_path.exists());

        // Verify content
        let content = fs::read_to_string(&ledger.ledger_path).unwrap();
        assert!(content.contains(r#""extension_name":"python"#));
    }

    #[test]
    fn test_get_all_latest_status() {
        let (ledger, _temp_dir) = create_test_ledger();

        // Add install started event
        let event1 = EventEnvelope::new(
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
        ledger.append(event1).unwrap();

        // Add install completed event
        let event2 = EventEnvelope::new(
            "python".to_string(),
            Some(ExtensionState::Installing),
            ExtensionState::Installed,
            ExtensionEvent::InstallCompleted {
                extension_name: "python".to_string(),
                version: "3.13.0".to_string(),
                duration_secs: 150,
                components_installed: vec!["python".to_string()],
            },
        );
        ledger.append(event2).unwrap();

        // Query status
        let status_map = ledger.get_all_latest_status().unwrap();

        assert_eq!(status_map.len(), 1);
        let status = status_map.get("python").unwrap();
        assert_eq!(status.current_state, ExtensionState::Installed);
        assert_eq!(status.version, Some("3.13.0".to_string()));
    }

    #[test]
    fn test_get_extension_history() {
        let (ledger, _temp_dir) = create_test_ledger();

        // Add multiple events for python
        for i in 0..5 {
            let event = EventEnvelope::new(
                "python".to_string(),
                None,
                ExtensionState::Installing,
                ExtensionEvent::InstallStarted {
                    extension_name: "python".to_string(),
                    version: format!("3.{}.0", i),
                    source: "github:pacphi/sindri".to_string(),
                    install_method: "Mise".to_string(),
                },
            );
            ledger.append(event).unwrap();
        }

        // Query history without limit
        let history = ledger.get_extension_history("python", None).unwrap();
        assert_eq!(history.len(), 5);

        // Query history with limit
        let history_limited = ledger.get_extension_history("python", Some(3)).unwrap();
        assert_eq!(history_limited.len(), 3);
    }

    #[test]
    fn test_concurrent_appends() {
        let (ledger, temp_dir) = create_test_ledger();
        let ledger_path = ledger.ledger_path.clone();

        let mut handles = vec![];

        for i in 0..10 {
            let path = ledger_path.clone();
            let handle = thread::spawn(move || {
                let ledger = StatusLedger::new(path);
                let event = EventEnvelope::new(
                    format!("ext{}", i),
                    None,
                    ExtensionState::Installing,
                    ExtensionEvent::InstallStarted {
                        extension_name: format!("ext{}", i),
                        version: "1.0.0".to_string(),
                        source: "github:pacphi/sindri".to_string(),
                        install_method: "Mise".to_string(),
                    },
                );
                ledger.append(event).unwrap();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all events were written
        let status_map = ledger.get_all_latest_status().unwrap();
        assert_eq!(status_map.len(), 10);

        drop(temp_dir);
    }

    #[test]
    fn test_compact() {
        let (ledger, _temp_dir) = create_test_ledger();

        // Add old event (100 days ago) - install started
        let mut old_event1 = EventEnvelope::new(
            "old_ext".to_string(),
            None,
            ExtensionState::Installing,
            ExtensionEvent::InstallStarted {
                extension_name: "old_ext".to_string(),
                version: "1.0.0".to_string(),
                source: "github:pacphi/sindri".to_string(),
                install_method: "Mise".to_string(),
            },
        );
        old_event1.timestamp = Utc::now() - Duration::days(100);
        ledger.append(old_event1).unwrap();

        // Add old event (100 days ago) - install completed (this is the latest for old_ext)
        let mut old_event2 = EventEnvelope::new(
            "old_ext".to_string(),
            Some(ExtensionState::Installing),
            ExtensionState::Installed,
            ExtensionEvent::InstallCompleted {
                extension_name: "old_ext".to_string(),
                version: "1.0.0".to_string(),
                duration_secs: 100,
                components_installed: vec![],
            },
        );
        old_event2.timestamp = Utc::now() - Duration::days(100);
        ledger.append(old_event2).unwrap();

        // Add recent event
        let recent_event = EventEnvelope::new(
            "new_ext".to_string(),
            None,
            ExtensionState::Installed,
            ExtensionEvent::InstallCompleted {
                extension_name: "new_ext".to_string(),
                version: "2.0.0".to_string(),
                duration_secs: 50,
                components_installed: vec![],
            },
        );
        ledger.append(recent_event).unwrap();

        // Compact with 90-day retention
        let removed = ledger.compact(90).unwrap();
        // Should remove old_event1 (InstallStarted) but keep old_event2 (latest for old_ext)
        assert_eq!(removed, 1);

        // Verify both extensions remain (latest events are always kept)
        let status_map = ledger.get_all_latest_status().unwrap();
        assert_eq!(status_map.len(), 2);
        assert!(status_map.contains_key("new_ext"));
        assert!(status_map.contains_key("old_ext")); // Latest event is always kept
    }

    #[test]
    fn test_get_stats() {
        let (ledger, _temp_dir) = create_test_ledger();

        // Add multiple events
        let event1 = EventEnvelope::new(
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
        ledger.append(event1).unwrap();

        let event2 = EventEnvelope::new(
            "python".to_string(),
            Some(ExtensionState::Installing),
            ExtensionState::Installed,
            ExtensionEvent::InstallCompleted {
                extension_name: "python".to_string(),
                version: "3.13.0".to_string(),
                duration_secs: 150,
                components_installed: vec![],
            },
        );
        ledger.append(event2).unwrap();

        // Get stats
        let stats = ledger.get_stats().unwrap();
        assert_eq!(stats.total_events, 2);
        assert!(stats.file_size_bytes > 0);
        assert!(stats.oldest_timestamp.is_some());
        assert!(stats.newest_timestamp.is_some());
        assert_eq!(stats.event_type_counts.get("install_started"), Some(&1));
        assert_eq!(stats.event_type_counts.get("install_completed"), Some(&1));
    }

    #[test]
    fn test_empty_ledger() {
        let (ledger, _temp_dir) = create_test_ledger();

        // Query empty ledger
        let status_map = ledger.get_all_latest_status().unwrap();
        assert!(status_map.is_empty());

        let history = ledger.get_extension_history("python", None).unwrap();
        assert!(history.is_empty());

        let stats = ledger.get_stats().unwrap();
        assert_eq!(stats.total_events, 0);
    }

    #[test]
    fn test_count_events() {
        let (ledger, _temp_dir) = create_test_ledger();

        // Empty ledger
        assert_eq!(ledger.count_events().unwrap(), 0);

        // Append events and verify count
        for i in 0..5 {
            let event = EventEnvelope::new(
                format!("ext{}", i),
                None,
                ExtensionState::Installing,
                ExtensionEvent::InstallStarted {
                    extension_name: format!("ext{}", i),
                    version: "1.0.0".to_string(),
                    source: "github:pacphi/sindri".to_string(),
                    install_method: "Mise".to_string(),
                },
            );
            ledger.append(event).unwrap();
        }

        assert_eq!(ledger.count_events().unwrap(), 5);
    }

    #[test]
    fn test_auto_compaction_triggers_at_interval() {
        let (ledger, _temp_dir) = create_test_ledger();

        // Add old events (older than 90 days) for many distinct extensions
        // to ensure compaction has something to remove
        for i in 0..50 {
            let mut event = EventEnvelope::new(
                format!("old_ext_{}", i),
                None,
                ExtensionState::Installing,
                ExtensionEvent::InstallStarted {
                    extension_name: format!("old_ext_{}", i),
                    version: "1.0.0".to_string(),
                    source: "github:pacphi/sindri".to_string(),
                    install_method: "Mise".to_string(),
                },
            );
            event.timestamp = Utc::now() - Duration::days(120);
            // Write directly to bypass auto-compact during setup
            let json_line = serde_json::to_string(&event).unwrap();
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&ledger.ledger_path)
                .unwrap();
            writeln!(file, "{}", json_line).unwrap();
        }

        // Now add "latest" events for each old extension so they have a
        // more recent latest event and the old ones become pruneable
        for i in 0..50 {
            let event = EventEnvelope::new(
                format!("old_ext_{}", i),
                Some(ExtensionState::Installing),
                ExtensionState::Installed,
                ExtensionEvent::InstallCompleted {
                    extension_name: format!("old_ext_{}", i),
                    version: "1.0.0".to_string(),
                    duration_secs: 10,
                    components_installed: vec![],
                },
            );
            // Write directly to bypass auto-compact during setup
            let json_line = serde_json::to_string(&event).unwrap();
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&ledger.ledger_path)
                .unwrap();
            writeln!(file, "{}", json_line).unwrap();
        }

        // We now have exactly 100 events written directly (bypassing auto-compact).
        // Verify count before the triggering append.
        assert_eq!(ledger.count_events().unwrap(), 100);

        // The next append should NOT trigger auto-compact (101 events, not a multiple of 100).
        // But first verify that our setup is correct with a stats check.
        let stats_before = ledger.get_stats().unwrap();
        assert_eq!(stats_before.total_events, 100);

        // Append events via the normal append method until we reach 200.
        // At event 200, auto-compaction should trigger and remove the 50 old
        // InstallStarted events (they are older than 90 days and not the latest
        // for their respective extensions).
        for i in 0..100 {
            let event = EventEnvelope::new(
                format!("new_ext_{}", i),
                None,
                ExtensionState::Installed,
                ExtensionEvent::InstallCompleted {
                    extension_name: format!("new_ext_{}", i),
                    version: "2.0.0".to_string(),
                    duration_secs: 5,
                    components_installed: vec![],
                },
            );
            ledger.append(event).unwrap();
        }

        // After auto-compaction at 200 events, the 50 old InstallStarted
        // events (> 90 days) should have been removed.
        // Remaining: 50 old InstallCompleted (latest, always kept) + 100 new = 150
        let stats_after = ledger.get_stats().unwrap();
        assert_eq!(stats_after.total_events, 150);
    }

    #[test]
    fn test_auto_compaction_no_trigger_below_interval() {
        let (ledger, _temp_dir) = create_test_ledger();

        // Add 50 events (below the 100-event interval)
        for i in 0..50 {
            let event = EventEnvelope::new(
                format!("ext_{}", i),
                None,
                ExtensionState::Installed,
                ExtensionEvent::InstallCompleted {
                    extension_name: format!("ext_{}", i),
                    version: "1.0.0".to_string(),
                    duration_secs: 10,
                    components_installed: vec![],
                },
            );
            ledger.append(event).unwrap();
        }

        // All 50 events should still be present (no compaction triggered)
        let stats = ledger.get_stats().unwrap();
        assert_eq!(stats.total_events, 50);
    }

    #[test]
    fn test_count_events_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let ledger_path = temp_dir.path().join("nonexistent_ledger.jsonl");
        let ledger = StatusLedger::new(ledger_path);

        // count_events on a nonexistent file should return 0
        assert_eq!(ledger.count_events().unwrap(), 0);
    }

    // ========================================================================
    // query_events() tests
    // ========================================================================

    fn append_test_event(
        ledger: &StatusLedger,
        name: &str,
        event: ExtensionEvent,
        state: ExtensionState,
    ) -> EventEnvelope {
        let envelope = EventEnvelope::new(name.to_string(), None, state, event);
        ledger.append(envelope.clone()).unwrap();
        envelope
    }

    #[test]
    fn test_query_events_no_filter() {
        let (ledger, _temp_dir) = create_test_ledger();

        for i in 0..5 {
            append_test_event(
                &ledger,
                &format!("ext{}", i),
                ExtensionEvent::InstallStarted {
                    extension_name: format!("ext{}", i),
                    version: "1.0.0".to_string(),
                    source: "github:pacphi/sindri".to_string(),
                    install_method: "Mise".to_string(),
                },
                ExtensionState::Installing,
            );
        }

        let events = ledger.query_events(EventFilter::default()).unwrap();
        assert_eq!(events.len(), 5);
    }

    #[test]
    fn test_query_events_filter_by_extension() {
        let (ledger, _temp_dir) = create_test_ledger();

        append_test_event(
            &ledger,
            "python",
            ExtensionEvent::InstallStarted {
                extension_name: "python".to_string(),
                version: "3.13.0".to_string(),
                source: "github:pacphi/sindri".to_string(),
                install_method: "Mise".to_string(),
            },
            ExtensionState::Installing,
        );
        append_test_event(
            &ledger,
            "nodejs",
            ExtensionEvent::InstallStarted {
                extension_name: "nodejs".to_string(),
                version: "22.0.0".to_string(),
                source: "github:pacphi/sindri".to_string(),
                install_method: "Mise".to_string(),
            },
            ExtensionState::Installing,
        );
        append_test_event(
            &ledger,
            "python",
            ExtensionEvent::InstallCompleted {
                extension_name: "python".to_string(),
                version: "3.13.0".to_string(),
                duration_secs: 60,
                components_installed: vec!["python".to_string()],
            },
            ExtensionState::Installed,
        );

        let events = ledger
            .query_events(EventFilter {
                extension_name: Some("python".to_string()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(events.len(), 2);
        assert!(events.iter().all(|e| e.extension_name == "python"));
    }

    #[test]
    fn test_query_events_filter_by_event_types() {
        let (ledger, _temp_dir) = create_test_ledger();

        append_test_event(
            &ledger,
            "python",
            ExtensionEvent::InstallStarted {
                extension_name: "python".to_string(),
                version: "3.13.0".to_string(),
                source: "github:pacphi/sindri".to_string(),
                install_method: "Mise".to_string(),
            },
            ExtensionState::Installing,
        );
        append_test_event(
            &ledger,
            "python",
            ExtensionEvent::InstallCompleted {
                extension_name: "python".to_string(),
                version: "3.13.0".to_string(),
                duration_secs: 60,
                components_installed: vec![],
            },
            ExtensionState::Installed,
        );
        append_test_event(
            &ledger,
            "nodejs",
            ExtensionEvent::InstallFailed {
                extension_name: "nodejs".to_string(),
                version: "22.0.0".to_string(),
                error_message: "Network error".to_string(),
                retry_count: 0,
                duration_secs: 10,
            },
            ExtensionState::Failed,
        );

        let events = ledger
            .query_events(EventFilter {
                event_types: Some(vec![
                    "install_completed".to_string(),
                    "install_failed".to_string(),
                ]),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_query_events_filter_by_time_range() {
        let (ledger, _temp_dir) = create_test_ledger();

        // Add event in the past
        let mut old_event = EventEnvelope::new(
            "python".to_string(),
            None,
            ExtensionState::Installing,
            ExtensionEvent::InstallStarted {
                extension_name: "python".to_string(),
                version: "3.12.0".to_string(),
                source: "github:pacphi/sindri".to_string(),
                install_method: "Mise".to_string(),
            },
        );
        old_event.timestamp = Utc::now() - Duration::days(10);
        ledger.append(old_event).unwrap();

        // Add recent event
        append_test_event(
            &ledger,
            "python",
            ExtensionEvent::InstallStarted {
                extension_name: "python".to_string(),
                version: "3.13.0".to_string(),
                source: "github:pacphi/sindri".to_string(),
                install_method: "Mise".to_string(),
            },
            ExtensionState::Installing,
        );

        let events = ledger
            .query_events(EventFilter {
                since: Some(Utc::now() - Duration::days(1)),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(events.len(), 1);

        let events_until = ledger
            .query_events(EventFilter {
                until: Some(Utc::now() - Duration::days(5)),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(events_until.len(), 1);
    }

    #[test]
    fn test_query_events_tail_mode() {
        let (ledger, _temp_dir) = create_test_ledger();

        for i in 0..10 {
            append_test_event(
                &ledger,
                &format!("ext{}", i),
                ExtensionEvent::InstallStarted {
                    extension_name: format!("ext{}", i),
                    version: "1.0.0".to_string(),
                    source: "github:pacphi/sindri".to_string(),
                    install_method: "Mise".to_string(),
                },
                ExtensionState::Installing,
            );
        }

        // Tail mode: reverse=true, limit=3 should return last 3 in chronological order
        let events = ledger
            .query_events(EventFilter {
                limit: Some(3),
                reverse: true,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(events.len(), 3);
        // Should be ext7, ext8, ext9 (last 3)
        assert_eq!(events[0].extension_name, "ext7");
        assert_eq!(events[1].extension_name, "ext8");
        assert_eq!(events[2].extension_name, "ext9");
    }

    #[test]
    fn test_query_events_combined_filters() {
        let (ledger, _temp_dir) = create_test_ledger();

        append_test_event(
            &ledger,
            "python",
            ExtensionEvent::InstallStarted {
                extension_name: "python".to_string(),
                version: "3.13.0".to_string(),
                source: "github:pacphi/sindri".to_string(),
                install_method: "Mise".to_string(),
            },
            ExtensionState::Installing,
        );
        append_test_event(
            &ledger,
            "python",
            ExtensionEvent::InstallCompleted {
                extension_name: "python".to_string(),
                version: "3.13.0".to_string(),
                duration_secs: 60,
                components_installed: vec![],
            },
            ExtensionState::Installed,
        );
        append_test_event(
            &ledger,
            "nodejs",
            ExtensionEvent::InstallCompleted {
                extension_name: "nodejs".to_string(),
                version: "22.0.0".to_string(),
                duration_secs: 30,
                components_installed: vec![],
            },
            ExtensionState::Installed,
        );

        // Filter by extension AND event type
        let events = ledger
            .query_events(EventFilter {
                extension_name: Some("python".to_string()),
                event_types: Some(vec!["install_completed".to_string()]),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].extension_name, "python");
    }

    #[test]
    fn test_query_events_empty_ledger() {
        let (ledger, _temp_dir) = create_test_ledger();

        let events = ledger.query_events(EventFilter::default()).unwrap();
        assert!(events.is_empty());
    }
}
