# Event-Driven Extension Status Architecture

**Status:** Planning
**Created:** 2026-02-11
**Target Version:** v3.0.0

---

## Executive Summary

This document outlines a comprehensive redesign of the Sindri extension status reporting system, transitioning from a slow, verification-based approach to a fast, event-driven architecture using an append-only event ledger.

**Current Problems:**

- Status queries take 17+ seconds for 27 extensions due to synchronous subprocess verification
- Inconsistent states (e.g., "installing" + "failed" simultaneously)
- No audit trail or error context for failures
- Verification happens only during status queries, not real-time

**Proposed Solution:**

- Event bus architecture where lifecycle operations publish events
- Append-only event ledger stores all extension operations
- Status queries read from ledger (no subprocess calls) → <1 second
- Complete operation history with timestamps and error details

**Impact:**

- **17x performance improvement** (17s → <1s for 50 extensions)
- **100% accurate state** (ledger is single source of truth)
- **Complete audit trail** (90-day operation history)
- **Better debugging** (full error context in events)

---

## Table of Contents

1. [Problem Statement](#problem-statement)
2. [Current Architecture Analysis](#current-architecture-analysis)
3. [Proposed Event-Driven Architecture](#proposed-event-driven-architecture)
4. [Technical Design](#technical-design)
5. [Implementation Phases](#implementation-phases)
6. [Use Cases](#use-cases)
7. [User Journeys](#user-journeys)
8. [Migration Strategy](#migration-strategy)
9. [Success Metrics](#success-metrics)
10. [Risks and Mitigations](#risks-and-mitigations)
11. [Future Enhancements](#future-enhancements)

---

## Problem Statement

### Performance Issue

The `sindri extension status` command is unacceptably slow:

```bash
$ time sindri extension status
ℹ Checking status of all installed extensions
2026-02-11T15:48:50.111522Z  WARN Command supabase not found: No such file or directory (os error 2)
2026-02-11T15:49:02.749857Z  WARN Command npx failed with exit code: Some(1)
[Table with 31 extensions]

took 26s
```

**Root Cause:** The `verify_extension_installed()` function in `sindri-extensions/src/verifier.rs` runs synchronous subprocess commands for each installed extension:

- `mise list <tool>` for mise-managed tools
- `dpkg -l <package>` for APT packages
- `which <binary>` for binary installations
- `npm list -g --depth=0 <package>` for npm packages
- Custom validation commands (e.g., `node --version`)

With 27 extensions × ~5 commands each = **135+ sequential subprocess calls** with no caching or parallelization.

### Accuracy Issue

Extensions show inconsistent states:

```
┌────────────────┬────────────┬───────────┬──────────────────┐
│ name           │ version    │ status    │ status date/time │
├────────────────┼────────────┼───────────┼──────────────────┤
│ sdkman         │ installing │ failed    │ 2026-02-11 01:57 │  ← Inconsistent!
│ agentic-qe     │ installing │ failed    │ 2026-02-11 01:42 │  ← Inconsistent!
└────────────────┴────────────┴───────────┴──────────────────┘
```

**Root Cause:** The status field shows two conflicting values:

1. Manifest state: "installing" (process started but never completed)
2. Verification result: "failed" (verification failed)

The manifest is updated at operation start but may not reflect final state if process crashes or is interrupted.

### Debuggability Issue

Failed installations provide no error context:

```bash
$ sindri extension status kubectl
NAME     VERSION  STATUS  STATUS_DATETIME
kubectl  1.35.0   failed  2026-02-11 10:15
```

**Root Cause:** No audit trail of extension operations. Users must manually review logs to understand why an installation failed.

---

## Current Architecture Analysis

### Code Structure

**Status Command:** `v3/crates/sindri/src/commands/extension.rs` (lines 1132-1243)

```rust
async fn status(args: ExtensionStatusArgs) -> Result<()> {
    // 1. Load manifest from ~/.sindri/manifest.yaml
    let manifest = ManifestManager::load_default()?;

    // 2. Get all installed extensions
    let entries = manifest.list_all();

    // 3. For each extension with state "Installed":
    for (name, ext) in entries {
        match ext.state {
            ExtensionState::Installed => {
                // 4. Run full verification (BOTTLENECK - 17+ seconds)
                let is_verified = verify_extension_installed(&extension).await;
                // ...
            }
        }
    }
}
```

**Verification Engine:** `v3/crates/sindri-extensions/src/verifier.rs` (lines 74-442)

```rust
pub async fn verify_extension_installed(extension: &Extension) -> bool {
    // Sequential subprocess calls:
    match extension.install.method {
        InstallMethod::Mise => {
            for tool in tools {
                Command::new("mise").arg("list").arg(tool).output(); // Blocks
            }
        }
        InstallMethod::Apt => {
            for package in packages {
                Command::new("dpkg").arg("-l").arg(package).output(); // Blocks
            }
        }
        // ... more methods
    }

    // Validation commands (also sequential)
    for validation in validations {
        Command::new(&validation.name).output(); // Blocks
    }
}
```

**Manifest:** `v3/crates/sindri-extensions/src/manifest.rs`

```yaml
# ~/.sindri/manifest.yaml
schema_version: "1.0"
cli_version: "3.0.0"
last_updated: "2026-01-21T10:00:00Z"
extensions:
  python:
    version: "3.13.0"
    status_datetime: "2026-01-20T15:30:00Z"
    source: "github:pacphi/sindri"
    state: installed # Can be: installed, failed, outdated, installing, removing
```

**State Model:** `v3/crates/sindri-core/src/types/registry_types.rs` (lines 109-142)

```rust
pub enum ExtensionState {
    Installed,    // Extension is installed and working
    Failed,       // Installation failed
    Outdated,     // Needs upgrade
    Installing,   // Being installed
    Removing,     // Being removed
}

pub struct InstalledExtension {
    pub version: String,
    pub status_datetime: DateTime<Utc>,  // When extension entered current state
    pub source: String,                  // "github:pacphi/sindri"
    pub state: ExtensionState,
}
```

### Performance Analysis

**Subprocess Overhead Per Extension:**

| Operation               | Tool               | Avg Time | Count   | Total     |
| ----------------------- | ------------------ | -------- | ------- | --------- |
| Mise verification       | `mise list python` | 50ms     | 3 tools | 150ms     |
| APT verification        | `dpkg -l package`  | 30ms     | 2 pkgs  | 60ms      |
| Binary verification     | `which binary`     | 20ms     | 2 bins  | 40ms      |
| NPM verification        | `npm list -g pkg`  | 100ms    | 1 pkg   | 100ms     |
| Validation commands     | `python --version` | 50ms     | 3 cmds  | 150ms     |
| **Total per extension** |                    |          |         | **500ms** |

**Total Status Query Time:**

- 27 extensions × 500ms = **13.5 seconds** (minimum)
- Actual: **17-26 seconds** (with network delays, disk I/O, etc.)

### Issues Summary

| Issue                     | Impact                                 | Root Cause                         |
| ------------------------- | -------------------------------------- | ---------------------------------- |
| **Slow queries**          | 17+ seconds for 27 extensions          | Sequential subprocess verification |
| **Inconsistent states**   | "installing" + "failed" simultaneously | Manifest not updated on failure    |
| **No error context**      | Can't debug failures                   | No operation history               |
| **No audit trail**        | Can't track changes                    | No event log                       |
| **Verification overhead** | Every status check re-verifies         | No caching                         |

---

## Proposed Event-Driven Architecture

### Core Concept

Replace **on-demand verification** with **event-driven state tracking**:

**Old:** Manifest stores state → Status command verifies state (slow)
**New:** Operations publish events → Ledger stores events → Status reads ledger (fast)

### Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                        User Commands                            │
│  $ sindri extension install python                              │
│  $ sindri extension status                                      │
└────────────┬────────────────────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Lifecycle Operations                         │
│  install() / upgrade() / remove()                               │
└────────────┬────────────────────────────────────────────────────┘
             │
             │ (1) Publish Event
             ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Event Bus                                  │
│  EventEnvelope::new(name, state_before, state_after, event)     │
└────────────┬────────────────────────────────────────────────────┘
             │
             │ (2) Append to Ledger
             ▼
┌───────────────────────────────────────────────────────────────────┐
│                   Status Ledger                                   │
│  ~/.sindri/status_ledger.jsonl (append-only)                      │
│  {"event_id":"...","timestamp":"...","extension_name":"python",   │
│   "state_after":"installed","event":{"type":"install_completed"}} │
└────────────┬──────────────────────────────────────────────────────┘
             │
             │ (3) Query Latest Status
             ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Status Command                                │
│  SELECT latest event per extension FROM ledger                  │
│  (no subprocess verification)                                   │
└─────────────────────────────────────────────────────────────────┘
```

### Key Components

1. **Event Schema** (`events.rs`)
   - `ExtensionEvent` enum: 12 event types (InstallStarted, InstallCompleted, InstallFailed, etc.)
   - `EventEnvelope`: Metadata wrapper (event_id, timestamp, state_before, state_after)

2. **Status Ledger** (`ledger.rs`)
   - JSON Lines format: One event per line, append-only
   - File locking for concurrent writes (fs4 crate)
   - Query methods: `get_all_latest_status()`, `get_extension_history()`

3. **Event Publishers**
   - Install operation: Publish `InstallStarted` → `InstallCompleted/Failed`
   - Upgrade operation: Publish `UpgradeStarted` → `UpgradeCompleted/Failed`
   - Remove operation: Publish `RemoveStarted` → `RemoveCompleted/Failed`

4. **Status Queries**
   - `sindri extension status`: Read latest event per extension (aggregate query)
   - `sindri extension status <name>`: Read all events for extension (history)

### Benefits

| Benefit            | Old                                      | New                           | Improvement            |
| ------------------ | ---------------------------------------- | ----------------------------- | ---------------------- |
| **Query Speed**    | 17s (27 extensions)                      | <1s                           | **17x faster**         |
| **State Accuracy** | Inconsistent (manifest vs. verification) | Consistent (ledger is truth)  | **100% accurate**      |
| **Error Context**  | None                                     | Full error message + duration | **Full debuggability** |
| **Audit Trail**    | None                                     | 90-day event history          | **Complete audit**     |
| **Verification**   | Every query                              | On-demand (optional)          | **Zero overhead**      |

---

## Technical Design

### 1. Event Schema

**File:** `v3/crates/sindri-extensions/src/events.rs` (new file, ~200 lines)

#### Event Types

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sindri_core::types::ExtensionState;

/// Extension lifecycle events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExtensionEvent {
    // Installation events
    InstallStarted {
        extension_name: String,
        version: String,
        source: String,          // "github:pacphi/sindri@main"
        install_method: String,  // "Mise", "Binary", "Hybrid", etc.
    },

    InstallCompleted {
        extension_name: String,
        version: String,
        duration_secs: u64,
        components_installed: Vec<String>,  // ["python", "pip", "uv"]
    },

    InstallFailed {
        extension_name: String,
        version: String,
        error_message: String,  // "Network timeout downloading binary"
        retry_count: u32,
        duration_secs: u64,
    },

    // Upgrade events
    UpgradeStarted {
        extension_name: String,
        from_version: String,
        to_version: String,
    },

    UpgradeCompleted {
        extension_name: String,
        from_version: String,
        to_version: String,
        duration_secs: u64,
    },

    UpgradeFailed {
        extension_name: String,
        from_version: String,
        to_version: String,
        error_message: String,
        duration_secs: u64,
    },

    // Removal events
    RemoveStarted {
        extension_name: String,
        version: String,
    },

    RemoveCompleted {
        extension_name: String,
        version: String,
        duration_secs: u64,
    },

    RemoveFailed {
        extension_name: String,
        version: String,
        error_message: String,
        duration_secs: u64,
    },

    // Status events
    OutdatedDetected {
        extension_name: String,
        current_version: String,
        latest_version: String,
    },

    ValidationSucceeded {
        extension_name: String,
        version: String,
        validation_type: String,  // "post-install", "manual", "scheduled"
    },

    ValidationFailed {
        extension_name: String,
        version: String,
        validation_type: String,
        error_message: String,
    },
}
```

#### Event Envelope

```rust
/// Event metadata envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    /// Unique event ID (UUID v4)
    pub event_id: String,

    /// Event timestamp (UTC)
    pub timestamp: DateTime<Utc>,

    /// Extension name (for indexing)
    pub extension_name: String,

    /// CLI version that published event
    pub cli_version: String,

    /// State before event (None for initial install)
    pub state_before: Option<ExtensionState>,

    /// State after event
    pub state_after: ExtensionState,

    /// The actual event payload
    pub event: ExtensionEvent,
}

impl EventEnvelope {
    pub fn new(
        extension_name: String,
        state_before: Option<ExtensionState>,
        state_after: ExtensionState,
        event: ExtensionEvent,
    ) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            extension_name,
            cli_version: env!("CARGO_PKG_VERSION").to_string(),
            state_before,
            state_after,
            event,
        }
    }
}
```

#### Dependencies

Add to `v3/crates/sindri-extensions/Cargo.toml`:

```toml
uuid = { version = "1.20", features = ["v4", "serde"] }
```

---

### 2. Status Ledger Storage

**File:** `v3/crates/sindri-extensions/src/ledger.rs` (new file, ~400 lines)

#### Storage Format

**JSON Lines (`.jsonl`):** One event per line, append-only

**Location:** `~/.sindri/status_ledger.jsonl`

**Example:**

```jsonl
{"event_id":"550e8400-e29b-41d4-a716-446655440000","timestamp":"2026-02-11T10:00:00Z","extension_name":"python","cli_version":"3.0.0","state_before":null,"state_after":"installing","event":{"type":"install_started","extension_name":"python","version":"3.13.0","source":"github:pacphi/sindri","install_method":"Mise"}}
{"event_id":"550e8400-e29b-41d4-a716-446655440001","timestamp":"2026-02-11T10:02:30Z","extension_name":"python","cli_version":"3.0.0","state_before":"installing","state_after":"installed","event":{"type":"install_completed","extension_name":"python","version":"3.13.0","duration_secs":150,"components_installed":["python","pip","uv"]}}
{"event_id":"550e8400-e29b-41d4-a716-446655440002","timestamp":"2026-02-11T10:05:00Z","extension_name":"nodejs","cli_version":"3.0.0","state_before":null,"state_after":"installing","event":{"type":"install_started","extension_name":"nodejs","version":"20.11.0","source":"github:pacphi/sindri","install_method":"Mise"}}
{"event_id":"550e8400-e29b-41d4-a716-446655440003","timestamp":"2026-02-11T10:06:45Z","extension_name":"nodejs","cli_version":"3.0.0","state_before":"installing","state_after":"failed","event":{"type":"install_failed","extension_name":"nodejs","version":"20.11.0","error_message":"Network timeout downloading Node.js","retry_count":0,"duration_secs":105}}
```

#### Core API

```rust
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use crate::events::EventEnvelope;
use sindri_core::types::ExtensionState;

pub struct StatusLedger {
    ledger_path: PathBuf,
}

impl StatusLedger {
    /// Load from default location (~/.sindri/status_ledger.jsonl)
    pub fn load_default() -> Result<Self> {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .or_else(|_| dirs::home_dir().ok_or_else(|| anyhow!("Cannot determine home directory")))?;
        let ledger_path = home.join(".sindri").join("status_ledger.jsonl");

        // Create file if doesn't exist
        if !ledger_path.exists() {
            if let Some(parent) = ledger_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::File::create(&ledger_path)?;
        }

        Ok(Self { ledger_path })
    }

    /// Append event to ledger (atomic, file-locked)
    pub fn append(&self, event: EventEnvelope) -> Result<()> {
        use fs4::fs_std::FileExt;

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.ledger_path)?;

        // Acquire exclusive lock (released on drop)
        file.lock_exclusive()?;

        let json_line = serde_json::to_string(&event)?;
        writeln!(file, "{}", json_line)?;
        file.sync_all()?;  // Ensure durability

        Ok(())
    }

    /// Get latest status for all extensions (aggregate query)
    pub fn get_all_latest_status(&self) -> Result<HashMap<String, ExtensionStatus>> {
        let mut status_map: HashMap<String, ExtensionStatus> = HashMap::new();

        for event in self.read_all_events()? {
            status_map
                .entry(event.extension_name.clone())
                .and_modify(|status| {
                    if event.timestamp > status.last_event_time {
                        status.current_state = event.state_after;
                        status.last_event_time = event.timestamp;
                        status.last_event_id = event.event_id.clone();
                    }
                })
                .or_insert_with(|| ExtensionStatus {
                    extension_name: event.extension_name.clone(),
                    current_state: event.state_after,
                    last_event_time: event.timestamp,
                    last_event_id: event.event_id.clone(),
                });
        }

        Ok(status_map)
    }

    /// Get event history for specific extension (chronological)
    pub fn get_extension_history(
        &self,
        extension_name: &str,
        limit: Option<usize>,
    ) -> Result<Vec<EventEnvelope>> {
        let mut events: Vec<EventEnvelope> = self
            .read_all_events()?
            .into_iter()
            .filter(|e| e.extension_name == extension_name)
            .collect();

        // Sort by timestamp (newest first)
        events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        if let Some(limit) = limit {
            events.truncate(limit);
        }

        Ok(events)
    }

    /// Get events since timestamp
    pub fn get_events_since(&self, since: DateTime<Utc>) -> Result<Vec<EventEnvelope>> {
        Ok(self
            .read_all_events()?
            .into_iter()
            .filter(|e| e.timestamp > since)
            .collect())
    }

    /// Read all events from ledger (sequential scan)
    fn read_all_events(&self) -> Result<Vec<EventEnvelope>> {
        let file = std::fs::File::open(&self.ledger_path)?;
        let reader = BufReader::new(file);
        let mut events = Vec::new();

        for (line_num, line_result) in reader.lines().enumerate() {
            let line = line_result?;
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<EventEnvelope>(&line) {
                Ok(event) => events.push(event),
                Err(e) => {
                    tracing::warn!("Failed to parse event at line {}: {}", line_num + 1, e);
                }
            }
        }

        Ok(events)
    }

    /// Compact ledger (remove old events, keep 90-day history)
    pub fn compact(&self, retention_days: i64) -> Result<usize> {
        let cutoff = Utc::now() - chrono::Duration::days(retention_days);
        let all_events = self.read_all_events()?;

        // Group by extension
        let mut by_extension: HashMap<String, Vec<EventEnvelope>> = HashMap::new();
        for event in all_events {
            by_extension
                .entry(event.extension_name.clone())
                .or_default()
                .push(event);
        }

        let mut retained_events = Vec::new();

        for (ext_name, mut events) in by_extension {
            events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

            // Partition into old and recent
            let (old, recent): (Vec<_>, Vec<_>) = events
                .into_iter()
                .partition(|e| e.timestamp < cutoff);

            // Keep only latest event from old partition
            if let Some(last_old) = old.last() {
                retained_events.push(last_old.clone());
            }

            // Keep all recent events
            retained_events.extend(recent);
        }

        let removed_count = self.read_all_events()?.len() - retained_events.len();

        // Rewrite ledger atomically
        self.rewrite_ledger(&retained_events)?;

        Ok(removed_count)
    }

    fn rewrite_ledger(&self, events: &[EventEnvelope]) -> Result<()> {
        let temp_path = self.ledger_path.with_extension("jsonl.tmp");

        {
            let mut temp_file = std::fs::File::create(&temp_path)?;
            for event in events {
                let json_line = serde_json::to_string(event)?;
                writeln!(temp_file, "{}", json_line)?;
            }
            temp_file.sync_all()?;
        }

        // Atomic rename
        std::fs::rename(&temp_path, &self.ledger_path)?;

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionStatus {
    pub extension_name: String,
    pub current_state: ExtensionState,
    pub last_event_time: DateTime<Utc>,
    pub last_event_id: String,
}
```

#### Dependencies

Add to `v3/crates/sindri-extensions/Cargo.toml`:

```toml
fs4 = "0.13"  # File locking for concurrent writes
```

---

### 3. Event Publisher Integration

**File:** Modify `v3/crates/sindri/src/commands/extension.rs`

#### Install Operation (lines 73-206)

```rust
async fn install_single(name: &str, version: Option<&str>) -> Result<()> {
    use sindri_extensions::ledger::StatusLedger;
    use sindri_extensions::events::{ExtensionEvent, EventEnvelope};

    let start_time = std::time::Instant::now();
    let ledger = StatusLedger::load_default()?;

    // Publish InstallStarted event
    let event = EventEnvelope::new(
        name.to_string(),
        None,  // No previous state for new installs
        ExtensionState::Installing,
        ExtensionEvent::InstallStarted {
            extension_name: name.to_string(),
            version: version.unwrap_or("latest").to_string(),
            source: format!("github:pacphi/sindri"),
            install_method: format!("{:?}", extension.install.method),
        },
    );
    ledger.append(event)?;

    // ... existing installation logic ...
    let result = perform_installation(extension).await;

    // Publish result event
    let duration_secs = start_time.elapsed().as_secs();
    match result {
        Ok(_) => {
            let event = EventEnvelope::new(
                name.to_string(),
                Some(ExtensionState::Installing),
                ExtensionState::Installed,
                ExtensionEvent::InstallCompleted {
                    extension_name: name.to_string(),
                    version: installed_version,
                    duration_secs,
                    components_installed: vec![],  // Collect execution metrics from the executor
                },
            );
            ledger.append(event)?;
            Ok(())
        }
        Err(e) => {
            let event = EventEnvelope::new(
                name.to_string(),
                Some(ExtensionState::Installing),
                ExtensionState::Failed,
                ExtensionEvent::InstallFailed {
                    extension_name: name.to_string(),
                    version: version.unwrap_or("unknown").to_string(),
                    error_message: e.to_string(),
                    retry_count: 0,
                    duration_secs,
                },
            );
            ledger.append(event)?;
            Err(e)
        }
    }
}
```

**Similar integration needed for:**

- **Upgrade operation** (lines 1384-1495): Publish `UpgradeStarted` → `UpgradeCompleted/Failed`
- **Remove operation** (lines 1507-1737): Publish `RemoveStarted` → `RemoveCompleted/Failed`

---

### 4. Status Command Overhaul

**File:** Modify `v3/crates/sindri/src/commands/extension.rs` (lines 1132-1243)

#### Replace Verification with Ledger Queries

```rust
async fn status(args: ExtensionStatusArgs) -> Result<()> {
    use sindri_extensions::ledger::StatusLedger;

    let ledger = StatusLedger::load_default()?;

    if let Some(name) = &args.name {
        // Show event history for specific extension
        output::info(&format!("Status history for extension: {}", name));

        let history = ledger.get_extension_history(name, args.limit.or(Some(20)))?;

        if history.is_empty() {
            output::warning(&format!("No events found for extension '{}'", name));
            return Ok(());
        }

        if args.json {
            println!("{}", serde_json::to_string_pretty(&history)?);
        } else {
            // Display event timeline
            for event in &history {
                println!(
                    "\n{} | {} → {}",
                    event.timestamp.format("%Y-%m-%d %H:%M:%S"),
                    event.state_before
                        .map(|s| format!("{:?}", s))
                        .unwrap_or_else(|| "None".to_string()),
                    format!("{:?}", event.state_after)
                );
                println!("  Event: {}", format_event_summary(&event.event));
            }
        }
    } else {
        // Show latest status for all extensions (FAST - no verification)
        output::info("Extension status (from event ledger):");

        let status_map = ledger.get_all_latest_status()?;

        if status_map.is_empty() {
            output::info("No extensions installed yet");
            return Ok(());
        }

        if args.json {
            println!("{}", serde_json::to_string_pretty(&status_map)?);
        } else {
            let mut rows: Vec<StatusRow> = status_map
                .values()
                .map(|status| StatusRow {
                    name: status.extension_name.clone(),
                    version: "N/A".to_string(),  // Query manifest if needed
                    status: format!("{:?}", status.current_state).to_lowercase(),
                    status_datetime: status
                        .last_event_time
                        .format("%Y-%m-%d %H:%M")
                        .to_string(),
                })
                .collect();

            rows.sort_by(|a, b| a.name.cmp(&b.name));

            let table = Table::new(rows).with(Style::rounded()).to_string();
            println!("{}", table);
        }
    }

    Ok(())
}

fn format_event_summary(event: &ExtensionEvent) -> String {
    match event {
        ExtensionEvent::InstallStarted { version, install_method, .. } => {
            format!("Install started (v{}, method: {})", version, install_method)
        }
        ExtensionEvent::InstallCompleted { version, duration_secs, .. } => {
            format!("Install completed (v{}, {}s)", version, duration_secs)
        }
        ExtensionEvent::InstallFailed { version, error_message, duration_secs, .. } => {
            format!("Install failed (v{}, {}s): {}", version, duration_secs, error_message)
        }
        ExtensionEvent::UpgradeStarted { from_version, to_version, .. } => {
            format!("Upgrade started ({} → {})", from_version, to_version)
        }
        ExtensionEvent::UpgradeCompleted { from_version, to_version, duration_secs, .. } => {
            format!("Upgrade completed ({} → {}, {}s)", from_version, to_version, duration_secs)
        }
        ExtensionEvent::UpgradeFailed { from_version, to_version, error_message, .. } => {
            format!("Upgrade failed ({} → {}): {}", from_version, to_version, error_message)
        }
        ExtensionEvent::RemoveStarted { version, .. } => {
            format!("Remove started (v{})", version)
        }
        ExtensionEvent::RemoveCompleted { version, duration_secs, .. } => {
            format!("Remove completed (v{}, {}s)", version, duration_secs)
        }
        ExtensionEvent::RemoveFailed { version, error_message, .. } => {
            format!("Remove failed (v{}): {}", version, error_message)
        }
        ExtensionEvent::OutdatedDetected { current_version, latest_version, .. } => {
            format!("Outdated detected ({} → {})", current_version, latest_version)
        }
        ExtensionEvent::ValidationSucceeded { version, validation_type, .. } => {
            format!("Validation succeeded (v{}, {})", version, validation_type)
        }
        ExtensionEvent::ValidationFailed { version, validation_type, error_message, .. } => {
            format!("Validation failed (v{}, {}): {}", version, validation_type, error_message)
        }
    }
}
```

#### CLI Arguments Update

**File:** Modify `v3/crates/sindri/src/cli.rs` (line 340)

```rust
#[derive(Args)]
pub struct ExtensionStatusArgs {
    /// Extension name (shows all if not specified)
    pub name: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Limit number of history entries (default: 20)
    #[arg(long)]
    pub limit: Option<usize>,

    /// Show events since date (ISO 8601: 2026-02-10T00:00:00Z)
    #[arg(long)]
    pub since: Option<String>,

    /// Run verification checks (slow, legacy mode)
    #[arg(long)]
    pub verify: bool,
}
```

---

## Implementation Phases

### Phase 1: Event Infrastructure (Week 1)

**Goal:** Build event publishing foundation

**Tasks:**

1. **Create event types** (`events.rs`)
   - Define `ExtensionEvent` enum (12 event types)
   - Define `EventEnvelope` struct
   - Add unit tests for serialization/deserialization
   - **Lines:** ~200

2. **Create status ledger** (`ledger.rs`)
   - Implement JSON Lines storage
   - Implement `append()` with file locking (fs4)
   - Implement `get_all_latest_status()` (aggregate)
   - Implement `get_extension_history()` (filter by name)
   - Add unit tests for concurrent writes
   - **Lines:** ~400

3. **Update module exports** (`lib.rs`)
   - Export `events` and `ledger` modules
   - **Lines:** ~5

4. **Update dependencies** (`Cargo.toml`)
   - Add `uuid = { version = "1.20", features = ["v4", "serde"] }`
   - Add `fs4 = "0.13"`
   - **Lines:** ~2

**Verification:**

- Unit tests pass for all ledger operations
- Concurrent append test: Spawn 10 threads, append 100 events each, verify no corruption
- Ledger compaction test: Prune events older than 90 days

**Deliverable:** `sindri-extensions` crate with events and ledger modules

---

### Phase 2: Lifecycle Integration (Week 2)

**Goal:** Publish events during extension operations

**Tasks:**

1. **Integrate event publishing into install** (`extension.rs` lines 73-206)
   - Measure start time
   - Publish `InstallStarted` before installation
   - Publish `InstallCompleted` or `InstallFailed` after
   - Include error messages and durations
   - **Lines changed:** ~50

2. **Integrate event publishing into upgrade** (`extension.rs` lines 1384-1495)
   - Publish `UpgradeStarted` → `UpgradeCompleted/Failed`
   - **Lines changed:** ~30

3. **Integrate event publishing into remove** (`extension.rs` lines 1507-1737)
   - Publish `RemoveStarted` → `RemoveCompleted/Failed`
   - **Lines changed:** ~30

4. **Remove ManifestManager** (breaking change)
   - Delete `v3/crates/sindri-extensions/src/manifest.rs` entirely
   - Remove all manifest references from codebase
   - Update all commands to use `StatusLedger` only
   - **Lines deleted:** ~400 (manifest.rs)

5. **Update all commands to use ledger exclusively**
   - Replace `ManifestManager::load_default()` with `StatusLedger::load_default()`
   - Remove all manifest read/write operations throughout codebase
   - **Lines changed:** ~100

**Verification:**

- Integration test: Install extension → verify 2 events (Started, Completed) in ledger
- Integration test: Failed install → verify 2 events (Started, Failed) with error message
- Integration test: Upgrade → verify 2 events (UpgradeStarted, UpgradeCompleted)
- Ledger file contains valid JSON Lines format
- No manifest.yaml references remain in codebase (grep verification)
- `sindri extension status` works with empty ledger (no errors)

**Deliverable:** Lifecycle operations publish events to ledger, manifest completely removed

---

### Phase 3: Status Command Overhaul (Week 3)

**Goal:** Replace verification with ledger queries

**Tasks:**

1. **Rewrite status command** (`extension.rs` lines 1132-1243)
   - Remove call to `verify_extension_installed()`
   - Query ledger via `get_all_latest_status()`
   - Add event history display for `sindri extension status <name>`
   - Format timeline: timestamp | state_before → state_after | event summary
   - **Lines changed:** ~150

2. **Update CLI arguments** (`cli.rs` line 340)
   - Add `limit: Option<usize>` for history length
   - Add `since: Option<String>` for date filtering
   - Add `verify: bool` for legacy mode
   - **Lines changed:** ~10

3. **Performance testing**
   - Benchmark status query with 50 extensions
   - Target: <1 second (vs. current 17+ seconds)
   - Document results in ADR

**Verification:**

- Performance test: 50 extensions, status query <1s
- Output format matches old format (name, version, status, status_datetime)
- Status history shows full event timeline for specific extension
- JSON output matches expected schema

**Deliverable:** Sub-second status queries with full history

---

### Phase 4: Verification Decoupling (Week 4)

**Goal:** Move verification to explicit commands

**Tasks:**

1. **Add verify command**
   - `sindri extension verify` - verify all installed extensions
   - `sindri extension verify <name>` - verify specific extension
   - Uses existing `verify_extension_installed()` from `verifier.rs`
   - Publishes `ValidationSucceeded` or `ValidationFailed` events
   - **Lines added:** ~100

2. **Update check command** (if exists)
   - Compare installed vs. latest versions
   - Publish `OutdatedDetected` events
   - **Lines changed:** ~50

3. **Add v3.0.0 breaking change notice** (optional)
   - Display notice on first run: "v3.0.0 uses event ledger - reinstall extensions"
   - Keep it simple and minimal
   - **Lines added:** ~10

**Verification:**

- Verify command runs full verification and publishes events
- Verification events appear in status history
- No manifest.yaml dependencies remain

**Deliverable:** Verification is explicit, not automatic

---

### Phase 5: Ledger Management (Week 5)

**Goal:** Add ledger maintenance and tooling

**Tasks:**

1. **Add ledger subcommands**
   - `sindri ledger compact [--retention-days 90]` - prune old events
   - `sindri ledger export <path>` - export to JSON/CSV
   - `sindri ledger stats` - show event counts, oldest/newest, size
   - **Lines added:** ~200

2. **Add auto-compaction**
   - Compact on every 100th operation
   - Keep 90-day retention by default
   - **Lines added:** ~30

3. **Documentation**
   - Create ADR: `v3/docs/architecture/adr/046-event-driven-status.md`
   - Update `v3/docs/CLI.md` with new commands
   - Update `v3/docs/AUTHORING.md` (if relevant)
   - **Lines added:** ~500 (documentation)

4. **Update changelog**
   - Document breaking changes
   - Document migration path
   - **Lines added:** ~50

**Verification:**

- Compaction reduces ledger size correctly
- Export produces valid JSON/CSV
- Stats show correct metrics (event counts, date ranges)
- Documentation is complete and accurate

**Deliverable:** Production-ready ledger management

---

## Use Cases

### Use Case 1: Fast Status Check

**Scenario:** Developer wants to quickly check extension status

**Before:**

```bash
$ time sindri extension status
ℹ Checking status of all installed extensions
[17 seconds of subprocess verification...]
┌────────────────┬────────────┬───────────┬──────────────────┐
│ name           │ version    │ status    │ status date/time │
├────────────────┼────────────┼───────────┼──────────────────┤
│ python         │ 3.13.0     │ installed │ 2026-02-11 01:41 │
│ nodejs         │ 20.11.0    │ installed │ 2026-02-11 01:40 │
└────────────────┴────────────┴───────────┴──────────────────┘

real    0m17.324s
```

**After:**

```bash
$ time sindri extension status
ℹ Extension status (from event ledger):
┌────────────────┬───────────┬──────────────────┐
│ name           │ status    │ last updated     │
├────────────────┼───────────┼──────────────────┤
│ python         │ installed │ 2026-02-11 01:41 │
│ nodejs         │ installed │ 2026-02-11 01:40 │
└────────────────┴───────────┴──────────────────┘

real    0m0.087s
```

**Impact:** **17x faster** (17s → <1s)

---

### Use Case 2: Debug Failed Installation

**Scenario:** kubectl installation fails, user needs to understand why

**Before:**

```bash
$ sindri extension install kubectl
❌ Error: Installation failed

$ sindri extension status kubectl
[17 seconds...]
NAME     VERSION  STATUS  STATUS_DATETIME
kubectl  1.35.0   failed  2026-02-11 10:15

# No error details - user must check logs manually
```

**After:**

```bash
$ sindri extension install kubectl
❌ Error: Installation failed: Network timeout

$ sindri extension status kubectl
Status history for extension: kubectl

2026-02-11 10:15:00 | installing → failed
  Event: Install failed (v1.35.0, 120s): Network timeout downloading kubectl binary from releases.k8s.io

2026-02-11 10:13:00 | None → installing
  Event: Install started (v1.35.0, method: Binary, source: github:pacphi/sindri)
```

**Impact:**

- Full error context immediately available
- Duration shows how long it took before failing
- Retry/debugging decisions can be made without re-running

---

### Use Case 3: Audit Extension Operations

**Scenario:** Security team needs to audit all extension changes in last 30 days

**Before:**

- Not possible - no audit trail

**After:**

```bash
$ sindri extension status --since 2026-01-12T00:00:00Z --json > audit.json

$ jq '.[] | select(.event.type | contains("failed"))' audit.json
[
  {
    "event_id": "...",
    "timestamp": "2026-01-15T10:15:00Z",
    "extension_name": "kubectl",
    "event": {
      "type": "install_failed",
      "error_message": "Network timeout downloading kubectl binary",
      "duration_secs": 120
    }
  }
]

$ jq '.[] | .extension_name' audit.json | sort | uniq -c
   4 docker
   2 kubectl
   3 nodejs
   5 python
```

**Impact:**

- Complete audit trail for compliance
- Query by date range, extension, event type
- Track all operations with timestamps

---

### Use Case 4: Monitor Extension Health

**Scenario:** Admin wants to verify all extensions are working

**Before:**

```bash
$ sindri extension status
[17 seconds of verification...]
# Verification happens automatically, slowing down query
```

**After:**

```bash
# Quick status check (no verification)
$ sindri extension status
[<1 second]

# Explicit verification when needed
$ sindri extension verify
✅ Verified 27 extensions (2 failed)

$ sindri extension status --since 2026-02-11T00:00:00Z | grep validation_failed
docker: Validation failed - Command 'docker --version' not found
kubectl: Validation failed - Binary not in PATH
```

**Impact:**

- Fast status queries for routine checks
- Explicit verification when needed
- Failed verifications logged in ledger

---

### Use Case 5: Track Extension Lifecycle

**Scenario:** Developer wants to see full history of an extension

**Before:**

- Not possible - only current status available

**After:**

```bash
$ sindri extension status python
Status history for extension: python

2026-02-11 10:30:00 | installed → installed
  Event: Validation succeeded (v3.13.0, manual)

2026-02-11 10:00:00 | installing → installed
  Event: Install completed (v3.13.0, 150s)

2026-02-11 09:58:00 | None → installing
  Event: Install started (v3.13.0, method: Mise, source: github:pacphi/sindri)

2026-01-20 14:30:00 | 3.12.0 → installed
  Event: Upgrade completed (3.12.0 → 3.13.0, 180s)
```

**Impact:**

- Full lifecycle visibility
- Understand when upgrades happened
- Track validation history

---

## User Journeys

### Journey 1: Developer Installing Extensions

**Before (Current):**

1. Run `sindri extension install python` (2 minutes)
2. Want to check status → Run `sindri extension status` (17 seconds wait)
3. See status, but no context on what was installed or how long it took
4. Install another extension
5. Check status again (17 seconds again)
6. Total time: 2min + 17s + 2min + 17s = **4m34s** (34s just checking status)

**After (Event-Driven):**

1. Run `sindri extension install python` (2 minutes)
2. Want to check status → Run `sindri extension status` (<1 second)
3. See status + duration (150s) + components (python, pip, uv)
4. Install another extension
5. Check status again (<1 second)
6. Total time: 2min + 1s + 2min + 1s = **4m2s** (2s checking status)

**Impact:**

- **32 seconds saved** on status checks alone
- Full visibility into what was installed
- Better understanding of installation process

---

### Journey 2: Debugging Failed Installations

**Before (Current):**

1. Extension install fails with vague error
2. Run `sindri extension status <name>` (17 seconds)
3. See "failed" status, no error details
4. Must manually search logs or re-run with verbose flags
5. Unclear what went wrong or when
6. Total time: **17s + 5min** (manual log search)

**After (Event-Driven):**

1. Extension install fails with error message
2. Run `sindri extension status <name>` (<1 second)
3. See full event history with:
   - Error message: "Network timeout downloading binary"
   - Duration: 120s
   - Timestamp: 2026-02-11 10:15:00
4. Decide whether to retry or fix underlying issue
5. Total time: **<1s** (no manual log search needed)

**Impact:**

- **99% time reduction** in debugging
- Immediate error context
- Clear understanding of failure cause

---

### Journey 3: System Administrator Auditing Extensions

**Before (Current):**

1. Need to audit extension changes for compliance
2. No audit trail available
3. Must manually track changes or review manifest file
4. Unclear when extensions were modified or by which CLI version
5. Generate compliance report manually
6. Total time: **Hours of manual work**

**After (Event-Driven):**

1. Run `sindri ledger export audit_2026-02.json`
2. Review all extension operations with timestamps
3. Filter by operation type: `jq '.[] | select(.event.type | contains("install"))' audit.json`
4. Generate compliance report with full event details
5. Query specific time ranges with `--since` flag
6. Total time: **Minutes**

**Impact:**

- **Hours saved** on compliance reporting
- Complete audit trail out of the box
- Time-series analysis of operations

---

## Migration Strategy

### Clean Break (No Migration)

**Breaking Change Rationale:**

- v3.0.0 is in release candidate phase—acceptable time for breaking changes
- Eliminates ALL backward compatibility complexity
- Simpler codebase with no migration logic
- Users reinstall extensions (standard for major version change)
- Clean slate for event ledger—only real operations tracked

**Approach:** Complete removal of `~/.sindri/manifest.yaml`

### After v3.0.0 Upgrade

```bash
$ sindri extension status  # First run after v3.0.0 upgrade
ℹ Extension status (from event ledger):
ℹ No extensions installed yet
ℹ Install extensions with: sindri extension install <name>
```

**No Migration Code:**

- No `import_manifest_to_ledger()` function
- No checking for existing manifest.yaml
- No archiving or backup logic
- Users start with empty ledger
- Extensions must be reinstalled

**Breaking Change Summary:**

- Delete `manifest.rs` from codebase entirely
- Ledger becomes sole source of truth immediately in v3.0.0
- Users start with clean slate—no extensions in ledger
- Reinstallation required for all extensions
- Zero migration code to maintain
- No synthetic events with incomplete data

---

## Success Metrics

### 1. Performance

| Metric                       | Target | Current | Improvement    |
| ---------------------------- | ------ | ------- | -------------- |
| Status query (27 extensions) | <1s    | 17s     | **17x faster** |
| Status query (50 extensions) | <1s    | ~30s    | **30x faster** |
| Ledger append latency        | <10ms  | N/A     | New capability |

### 2. Accuracy

| Metric                | Target | Current               | Improvement       |
| --------------------- | ------ | --------------------- | ----------------- |
| State inconsistencies | 0      | ~2-3 per status check | **100% accurate** |
| Event coverage        | 100%   | 0%                    | New capability    |

### 3. Debuggability

| Metric                      | Target  | Current | Improvement             |
| --------------------------- | ------- | ------- | ----------------------- |
| Failures with error context | 100%    | 0%      | **Complete visibility** |
| Event history availability  | 90 days | 0 days  | New capability          |

### 4. Audit

| Metric                | Target    | Current       | Improvement    |
| --------------------- | --------- | ------------- | -------------- |
| Operation audit trail | 90 days   | 0 days        | New capability |
| Time-series queries   | Supported | Not supported | New capability |

### 5. Migration

| Metric                 | Target | Current | Improvement     |
| ---------------------- | ------ | ------- | --------------- |
| Clean slate for v3.0.0 | Yes    | N/A     | Breaking change |

---

## Risks and Mitigations

### Risk 1: Ledger File Corruption

**Impact:** High - Loss of operation history

**Mitigation:**

- Use file locking (fs4 crate) for atomic writes
- Use fsync() to ensure durability after each append
- Add ledger validation command: `sindri ledger validate`
- Document recovery procedure in case of corruption
- Regular ledger compaction reduces corruption surface area
- Backup strategy: Users can export ledger with `sindri ledger export`

### Risk 2: Performance Degradation at Scale

**Impact:** Medium - Slower queries as ledger grows

**Mitigation:**

- Sequential scan acceptable for <10K events (~100ms)
- Add in-memory index if needed (build on first read)
- Implement compaction to keep ledger size bounded (90-day retention)
- Future: Migrate to SQLite if scale exceeds 10K events
- Monitor ledger size and add alerts at 5K events

### Risk 3: Data Loss During Migration

**Impact:** Medium - Users might lose extension history during v3.0.0 upgrade

**Mitigation:**

- Document breaking change prominently in CHANGELOG and release notes
- Add breaking change badge to v3.0.0 release notes
- Display clear notice on first run: "v3.0.0 requires extension reinstallation"
- Provide migration guide showing how to list extensions before upgrade (v2.x)
- Document reinstallation workflow in upgrade guide
- Consider adding `sindri extension list --format json` in v2.x final release for backup
- Create v3.0.0 upgrade documentation with step-by-step reinstallation instructions

### Risk 4: Incomplete Event Coverage

**Impact:** Medium - Missing events lead to inaccurate state

**Mitigation:**

- Comprehensive event enum covers all lifecycle operations (12 event types)
- Unit tests for each event type serialization/deserialization
- Integration tests verify events published correctly
- Monitor for operations without corresponding events
- Add validation: Every operation MUST publish at least one event

### Risk 5: Concurrent Write Conflicts

**Impact:** Low - Multiple sindri processes writing simultaneously

**Mitigation:**

- File-level advisory locking using fs4 crate
- Exclusive lock acquired before each append
- Lock released automatically on file close (RAII)
- Test concurrent writes: 10 threads × 100 events each

---

## Future Enhancements

After core implementation (v3.0.0), consider:

### 1. Background Verification Daemon

**Goal:** Periodic validation without manual intervention

```bash
$ sindri daemon start
ℹ Starting background verification daemon (interval: 6h)
✅ Daemon started (PID: 12345)

$ sindri daemon status
✅ Daemon running (PID: 12345)
📊 Last validation: 2026-02-11 10:00:00
✅ 25 extensions verified, 2 failed
```

**Implementation:**

- Run verification every 6 hours
- Publish `ValidationSucceeded/Failed` events
- Store daemon PID in `~/.sindri/daemon.pid`

### 2. Real-Time Monitoring

**Goal:** Watch ledger for events and notify on changes

```bash
$ sindri extension watch
ℹ Watching extension events (Ctrl+C to stop)...

2026-02-11 10:15:00 | docker: installing → failed
  Error: Network timeout downloading Docker binary
```

**Implementation:**

- Tail ledger file using `notify` crate
- Parse new events and display formatted output
- Filter by extension name or event type

### 3. Analytics

**Goal:** Track extension health and performance

```bash
$ sindri ledger stats --analytics
┌────────────────────────────────────────────────────┐
│                Extension Analytics                 │
├────────────────────────────────────────────────────┤
│ Total operations: 156                              │
│ Install success rate: 92% (144/156)                │
│ Avg install duration: 125s                         │
│ Most failed extension: kubectl (5 failures)        │
│ Longest install: docker (340s)                     │
└────────────────────────────────────────────────────┘
```

**Implementation:**

- Aggregate events by type, extension, outcome
- Calculate success rates, durations, failure patterns
- Identify frequently failing extensions

### 4. SQLite Migration

**Goal:** Better performance for large ledgers (>10K events)

**When:** If ledger exceeds 10K events or queries become slow

**Benefits:**

- Indexed queries (B-tree on extension_name, timestamp)
- Transaction support (ACID guarantees)
- SQL queries for complex analytics
- Reduced file size (compression)

**Migration Path:**

```bash
$ sindri ledger migrate-to-sqlite
ℹ Migrating JSON Lines ledger to SQLite...
✅ Migrated 12,345 events to ~/.sindri/status_ledger.db
📊 Database size: 2.5 MB (was 8.1 MB JSON Lines)
```

### 5. Integration with Observability

**Goal:** Export events to external monitoring systems

**Prometheus:**

```bash
$ sindri extension status --prometheus
# HELP sindri_extension_status Extension status (0=failed, 1=installed)
# TYPE sindri_extension_status gauge
sindri_extension_status{extension="python"} 1
sindri_extension_status{extension="docker"} 0
```

**Grafana:**

- Dashboard showing extension health over time
- Alerts on repeated failures
- Install duration trends

---

## Appendix: File Checklist

### Files to Create

- [ ] `v3/crates/sindri-extensions/src/events.rs` (~200 lines)
- [ ] `v3/crates/sindri-extensions/src/ledger.rs` (~400 lines)
- [ ] `v3/docs/architecture/adr/046-event-driven-status.md` (~150 lines)

### Files to Modify

- [ ] `v3/crates/sindri/src/commands/extension.rs` (lines 73-206, 1132-1243, 1384-1495, 1507-1737)
- [ ] `v3/crates/sindri-extensions/src/lib.rs` (add module exports)
- [ ] `v3/crates/sindri/src/cli.rs` (line 340, add CLI args)
- [ ] `v3/crates/sindri-extensions/Cargo.toml` (add dependencies)
- [ ] `v3/crates/sindri-extensions/src/manifest.rs` (add dual-write)
- [ ] `v3/docs/CLI.md` (document new commands)
- [ ] `v3/CHANGELOG.md` (document changes)

### Files to Read (Reference)

- [x] `v3/crates/sindri/src/commands/extension.rs` (status command)
- [x] `v3/crates/sindri-extensions/src/verifier.rs` (verification logic)
- [x] `v3/crates/sindri-core/src/types/registry_types.rs` (types)
- [x] `v3/crates/sindri-extensions/src/manifest.rs` (manifest manager)

---

## Conclusion

This event-driven architecture represents a fundamental improvement to Sindri's extension status system. By replacing slow, verification-based queries with fast, ledger-based queries, we achieve:

- **17x performance improvement** (17s → <1s)
- **100% accurate state tracking**
- **Complete audit trail** (90-day history)
- **Full debuggability** (error context for all failures)

The phased implementation minimizes risk while delivering incremental value:

1. **Week 1:** Event infrastructure (foundation)
2. **Week 2:** Lifecycle integration (event publishing)
3. **Week 3:** Status queries (fast lookups)
4. **Week 4:** Verification decoupling (explicit validation)
5. **Week 5:** Ledger management (tooling and docs)

This design is production-ready with a clean breaking change for v3.0.0, and provides a solid foundation for future enhancements like real-time monitoring, analytics, and integration with external observability systems.

**Next Steps:**

1. Review and approve this plan
2. Create GitHub issue/epic for tracking
3. Begin Phase 1 implementation
4. Iterate based on feedback

---

**Document Version:** 1.0
**Last Updated:** 2026-02-11
**Status:** Awaiting Approval
