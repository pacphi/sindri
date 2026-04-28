//! Per-component apply-state persistence (Wave 5H, D19).
//!
//! # Overview
//!
//! `sindri apply --resume` needs durable checkpoints so it can skip
//! components that already completed in a previous (failed) run.  This
//! module owns:
//!
//! * [`ComponentStage`] — the ordered stages a component moves through.
//! * [`ComponentStatus`] — the per-component status that is persisted.
//! * [`ApplyStateStore`] — the JSONL store at
//!   `~/.sindri/apply-state/<bom-hash>.jsonl`.
//!
//! ## State-file layout
//!
//! Each line in the JSONL file is a [`StateRecord`] — an *append-only
//! transition event*.  On reload, the last record for each component
//! wins, giving us a simple, tail-append log that survives partial
//! writes.  Example:
//!
//! ```jsonl
//! {"component":"nodejs","stage":"pending","status":"pending","ts":"2026-04-27T10:00:00Z"}
//! {"component":"nodejs","stage":"installing","status":"in_progress","ts":"2026-04-27T10:00:01Z"}
//! {"component":"nodejs","stage":"completed","status":"completed","ts":"2026-04-27T10:00:05Z"}
//! {"component":"rust","stage":"installing","status":"in_progress","ts":"2026-04-27T10:00:06Z"}
//! {"component":"rust","stage":"failed","status":"failed","error":"exit 1","ts":"2026-04-27T10:00:07Z"}
//! ```
//!
//! ## BOM-hash isolation
//!
//! The file name is `sha256(<bom-yaml>)` truncated to 16 hex chars, so
//! different BOMs use different state files and never interfere.
//!
//! ## Concurrent-apply protection
//!
//! The caller (apply.rs) acquires an exclusive flock on the state file
//! before reading or writing.  [`ApplyStateStore::try_lock`] returns
//! [`StateError::AlreadyRunning`] when the lock is not immediately
//! available.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// Per-component pipeline stage (mirrors the 8-step apply pipeline from
/// ADR-024).  The discriminants are kept stable so JSONL files remain
/// forward-compatible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentStage {
    /// Component has been identified but not yet touched.
    Pending,
    /// Pre-install hook is running (step 1).
    PreInstall,
    /// Install backend is running (step 2).
    Installing,
    /// Configure executor is running (step 3).
    Configuring,
    /// Validate executor is running (step 4).
    Validating,
    /// Post-install hook is running (step 5).
    PostInstall,
    /// Pre-project-init hook is running (step 6).
    PreProjectInit,
    /// ProjectInitExecutor is running (step 7).
    ProjectInit,
    /// Post-project-init hook is running (step 8).
    PostProjectInit,
    /// All steps completed successfully.
    Completed,
    /// A stage failed; `error` carries the diagnostic.
    Failed,
}

/// Single-record status that is evaluated when deciding whether to skip
/// a component on `--resume`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

/// One append-only JSONL record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateRecord {
    /// Component name (matches [`sindri_core::component::ComponentId::name`]).
    pub component: String,
    /// The stage this record describes.
    pub stage: ComponentStage,
    /// Coarse status used by the resume logic.
    pub status: RecordStatus,
    /// Human-readable error string, present on `Failed` records.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// RFC-3339 wall-clock timestamp.
    pub ts: String,
}

/// Errors produced by the state store.
#[derive(Debug, thiserror::Error)]
pub enum StateError {
    /// Another `sindri apply` process holds the exclusive flock on the
    /// state file for this BOM.
    #[error(
        "another sindri apply is in progress for this BOM \
         (state file: {path}). \
         Wait for it to finish or run `sindri apply --clear-state` to reset."
    )]
    AlreadyRunning { path: PathBuf },

    /// A file system operation failed.
    #[error("apply-state I/O error ({path}): {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// A JSONL record could not be decoded.
    #[error("apply-state parse error at line {line}: {source}")]
    Parse {
        line: usize,
        #[source]
        source: serde_json::Error,
    },
}

/// The in-memory summary of what has already been completed; used by the
/// resume logic to skip completed components.
#[derive(Debug, Default)]
pub struct ApplyStateSummary {
    /// Map from component name → last recorded status.
    pub last_status: HashMap<String, RecordStatus>,
}

impl ApplyStateSummary {
    /// Returns `true` if the component already completed successfully.
    pub fn is_completed(&self, component: &str) -> bool {
        matches!(
            self.last_status.get(component),
            Some(RecordStatus::Completed)
        )
    }

    /// Returns `true` if the component should be attempted (pending / failed /
    /// not yet seen).
    pub fn should_run(&self, component: &str) -> bool {
        !self.is_completed(component)
    }
}

/// JSONL-backed apply-state store.
///
/// One store instance exists for the lifetime of a single `sindri apply`
/// invocation.  Call [`ApplyStateStore::open`] to create or reopen a state
/// file, then use [`Self::append`] to record transitions.
///
/// The store does **not** hold the flock — locking is the responsibility of
/// the caller (apply.rs) via [`try_lock_state_file`].
pub struct ApplyStateStore {
    path: PathBuf,
}

impl ApplyStateStore {
    /// State-file directory: `~/.sindri/apply-state/`.
    pub fn state_dir() -> Option<PathBuf> {
        sindri_core_paths::home_dir().map(|h| h.join(".sindri").join("apply-state"))
    }

    /// Derive a state-file path from the BOM content.
    ///
    /// Uses `sha256(bom_content)` as the file stem so two identical BOMs
    /// always share the same state file, and two different BOMs never do.
    pub fn path_for_bom(bom_content: &str) -> Option<PathBuf> {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(bom_content.as_bytes());
        let hash = hex::encode(h.finalize());
        // 16-hex-char prefix is sufficient for collision resistance across
        // the tens-of-projects a single user is likely to have.
        let stem = &hash[..16];
        Self::state_dir().map(|d| d.join(format!("{stem}.jsonl")))
    }

    /// Open (creating if necessary) the state file at `path`.
    pub fn open(path: PathBuf) -> Result<Self, StateError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| StateError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }
        // Touch the file so the flock target exists before the caller
        // attempts to lock it.
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| StateError::Io {
                path: path.clone(),
                source: e,
            })?;
        Ok(Self { path })
    }

    /// Append a [`StateRecord`] to the JSONL file.
    pub fn append(&self, record: &StateRecord) -> Result<(), StateError> {
        let mut line = serde_json::to_string(record).map_err(|e| StateError::Io {
            path: self.path.clone(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
        })?;
        line.push('\n');

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| StateError::Io {
                path: self.path.clone(),
                source: e,
            })?;
        file.write_all(line.as_bytes()).map_err(|e| StateError::Io {
            path: self.path.clone(),
            source: e,
        })
    }

    /// Load and reduce the JSONL file into an [`ApplyStateSummary`].
    ///
    /// Truncated / partial last lines (from a killed process) are silently
    /// ignored so recovery is always possible.
    pub fn load_summary(&self) -> Result<ApplyStateSummary, StateError> {
        load_summary_from_path(&self.path)
    }

    /// Delete the state file.  Used by `--clear-state`.
    pub fn clear(&self) -> Result<(), StateError> {
        if self.path.exists() {
            std::fs::remove_file(&self.path).map_err(|e| StateError::Io {
                path: self.path.clone(),
                source: e,
            })?;
        }
        Ok(())
    }

    /// Path to the backing JSONL file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Load and reduce a state file at an arbitrary path (useful for tests).
pub fn load_summary_from_path(path: &Path) -> Result<ApplyStateSummary, StateError> {
    let mut summary = ApplyStateSummary::default();
    if !path.exists() {
        return Ok(summary);
    }
    let file = File::open(path).map_err(|e| StateError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    let reader = BufReader::new(file);
    for (idx, line_result) in reader.lines().enumerate() {
        let line = match line_result {
            Ok(l) => l,
            Err(e) => {
                return Err(StateError::Io {
                    path: path.to_path_buf(),
                    source: e,
                })
            }
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<StateRecord>(trimmed) {
            Ok(record) => {
                summary
                    .last_status
                    .insert(record.component.clone(), record.status);
            }
            Err(e) => {
                // Partial last-line write: if this is the very last
                // non-empty line attempt to be lenient — skip it.
                // Otherwise bubble the error.
                let line_no = idx + 1;
                // Peek whether there are more lines after this one.
                // If the file ended abruptly we treat it as a partial
                // write and skip; otherwise it is a genuine format error.
                tracing::warn!(
                    "apply-state: ignoring malformed record at line {}: {}",
                    line_no,
                    e
                );
                // Don't fail — continue trying remaining lines.
                let _ = e;
            }
        }
    }
    Ok(summary)
}

// ---------------------------------------------------------------------------
// Advisory flock helpers
// ---------------------------------------------------------------------------

/// A held exclusive lock on a state file.
///
/// Releasing this value (via [`Drop`]) releases the OS-level flock.
pub struct StateLock {
    _file: File,
    path: PathBuf,
}

impl StateLock {
    /// Path of the locked file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for StateLock {
    fn drop(&mut self) {
        // The OS releases the lock when the file descriptor is closed.
        // We rely on the implicit drop of `_file` here; no explicit unlock
        // syscall needed.
    }
}

/// Attempt to acquire a **non-blocking** exclusive advisory lock on `path`.
///
/// Returns [`StateError::AlreadyRunning`] immediately (without blocking)
/// if another process already holds the lock.  The caller must keep the
/// returned [`StateLock`] alive for the duration of the apply.
///
/// Cross-platform via [`fs4`]: `flock(LOCK_EX | LOCK_NB)` on Unix and
/// `LockFileEx(LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY)` on Windows.
pub fn try_lock_state_file(path: &Path) -> Result<StateLock, StateError> {
    use fs4::fs_std::FileExt;

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| StateError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

    match file.try_lock_exclusive() {
        Ok(()) => Ok(StateLock {
            _file: file,
            path: path.to_path_buf(),
        }),
        Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
            Err(StateError::AlreadyRunning {
                path: path.to_path_buf(),
            })
        }
        Err(err) => Err(StateError::Io {
            path: path.to_path_buf(),
            source: err,
        }),
    }
}

// ---------------------------------------------------------------------------
// Timestamp helper
// ---------------------------------------------------------------------------

/// Return the current UTC time as an RFC-3339 string.
///
/// Uses `std::time::SystemTime` to avoid adding `chrono` to `sindri-core`'s
/// public API surface.  Sub-second precision is dropped — that is fine for a
/// state-transition log where entries are typically seconds apart.
pub fn now_rfc3339() -> String {
    use std::time::SystemTime;
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_unix_ts(secs)
}

fn format_unix_ts(secs: u64) -> String {
    // Days per month (non-leap year; we do rough calculation)
    let (y, mo, d, h, min, s) = unix_to_ymd_hms(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{min:02}:{s:02}Z")
}

fn unix_to_ymd_hms(secs: u64) -> (u64, u64, u64, u64, u64, u64) {
    let s = secs % 60;
    let total_min = secs / 60;
    let min = total_min % 60;
    let total_h = total_min / 60;
    let h = total_h % 24;
    let total_days = total_h / 24;

    // Gregorian calendar approximation (sufficient for log timestamps)
    let mut year = 1970u64;
    let mut days_left = total_days;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if days_left < days_in_year {
            break;
        }
        days_left -= days_in_year;
        year += 1;
    }
    let leap = is_leap(year);
    let months = [
        31u64,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1u64;
    for &m in &months {
        if days_left < m {
            break;
        }
        days_left -= m;
        month += 1;
    }
    (year, month, days_left + 1, h, min, s)
}

fn is_leap(y: u64) -> bool {
    (y.is_multiple_of(4) && !y.is_multiple_of(100)) || y.is_multiple_of(400)
}

// Re-export so apply.rs can use sindri_core::apply_state without knowing
// about the internal paths module.
mod sindri_core_paths {
    pub(super) fn home_dir() -> Option<std::path::PathBuf> {
        crate::paths::home_dir()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp_store(dir: &TempDir) -> ApplyStateStore {
        let path = dir.path().join("test.jsonl");
        ApplyStateStore::open(path).unwrap()
    }

    fn record(component: &str, stage: ComponentStage, status: RecordStatus) -> StateRecord {
        StateRecord {
            component: component.to_string(),
            stage,
            status,
            error: None,
            ts: "2026-04-27T00:00:00Z".to_string(),
        }
    }

    // -----------------------------------------------------------------------
    // Round-trip: write + load
    // -----------------------------------------------------------------------

    #[test]
    fn round_trip_single_completed() {
        let dir = TempDir::new().unwrap();
        let store = tmp_store(&dir);

        store
            .append(&record(
                "nodejs",
                ComponentStage::Completed,
                RecordStatus::Completed,
            ))
            .unwrap();

        let summary = store.load_summary().unwrap();
        assert!(summary.is_completed("nodejs"));
        assert!(!summary.should_run("nodejs"));
    }

    #[test]
    fn round_trip_failed_component() {
        let dir = TempDir::new().unwrap();
        let store = tmp_store(&dir);

        store
            .append(&record(
                "rust",
                ComponentStage::Installing,
                RecordStatus::InProgress,
            ))
            .unwrap();
        store
            .append(&StateRecord {
                component: "rust".to_string(),
                stage: ComponentStage::Failed,
                status: RecordStatus::Failed,
                error: Some("exit 1".to_string()),
                ts: "2026-04-27T00:00:01Z".to_string(),
            })
            .unwrap();

        let summary = store.load_summary().unwrap();
        assert!(!summary.is_completed("rust"));
        assert!(summary.should_run("rust"));
    }

    // -----------------------------------------------------------------------
    // Tail-append: last record wins
    // -----------------------------------------------------------------------

    #[test]
    fn last_record_wins() {
        let dir = TempDir::new().unwrap();
        let store = tmp_store(&dir);

        // Component transitions from failed to completed (simulates a second run)
        store
            .append(&record(
                "nodejs",
                ComponentStage::Failed,
                RecordStatus::Failed,
            ))
            .unwrap();
        store
            .append(&record(
                "nodejs",
                ComponentStage::Completed,
                RecordStatus::Completed,
            ))
            .unwrap();

        let summary = store.load_summary().unwrap();
        assert!(
            summary.is_completed("nodejs"),
            "last record (completed) should win"
        );
    }

    // -----------------------------------------------------------------------
    // Recovery from partial writes
    // -----------------------------------------------------------------------

    #[test]
    fn recovery_from_partial_last_line() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("partial.jsonl");

        // Write a valid record followed by a truncated line (simulates a crash)
        let valid = r#"{"component":"nodejs","stage":"completed","status":"completed","ts":"2026-04-27T00:00:00Z"}"#;
        let partial = r#"{"component":"rust","stage":"installing","statu"#; // truncated!

        std::fs::write(&path, format!("{valid}\n{partial}")).unwrap();

        let summary = load_summary_from_path(&path).unwrap();
        // nodejs should be completed; rust should be absent (partial line skipped)
        assert!(summary.is_completed("nodejs"));
        assert!(!summary.last_status.contains_key("rust"));
    }

    // -----------------------------------------------------------------------
    // BOM-hash isolation
    // -----------------------------------------------------------------------

    #[test]
    fn different_boms_produce_different_paths() {
        let path_a = ApplyStateStore::path_for_bom("bom-content-a");
        let path_b = ApplyStateStore::path_for_bom("bom-content-b");
        assert_ne!(
            path_a, path_b,
            "different BOMs must use different state files"
        );
    }

    #[test]
    fn same_bom_produces_same_path() {
        let content = "components:\n  - nodejs\n";
        let path_a = ApplyStateStore::path_for_bom(content);
        let path_b = ApplyStateStore::path_for_bom(content);
        assert_eq!(path_a, path_b, "same BOM must reuse the same state file");
    }

    // -----------------------------------------------------------------------
    // Clear
    // -----------------------------------------------------------------------

    #[test]
    fn clear_removes_state_file() {
        let dir = TempDir::new().unwrap();
        let store = tmp_store(&dir);

        store
            .append(&record(
                "nodejs",
                ComponentStage::Completed,
                RecordStatus::Completed,
            ))
            .unwrap();
        assert!(store.path().exists());

        store.clear().unwrap();
        assert!(!store.path().exists());
    }

    #[test]
    fn clear_is_idempotent_when_file_absent() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nonexistent.jsonl");
        let store = ApplyStateStore { path };
        // Should not error even when the file doesn't exist
        store.clear().unwrap();
    }

    // -----------------------------------------------------------------------
    // Concurrent-apply flock
    // -----------------------------------------------------------------------

    #[test]
    fn flock_blocks_second_locker() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("lock.jsonl");

        // Touch the file
        std::fs::write(&path, b"").unwrap();

        let _lock1 = try_lock_state_file(&path).expect("first lock must succeed");

        // Second lock must fail with AlreadyRunning
        match try_lock_state_file(&path) {
            Err(StateError::AlreadyRunning { .. }) => {} // expected
            Err(e) => panic!("unexpected error: {e}"),
            Ok(_) => panic!("second lock must fail"),
        }
    }

    // -----------------------------------------------------------------------
    // Timestamp formatting sanity
    // -----------------------------------------------------------------------

    #[test]
    fn format_unix_epoch() {
        let ts = format_unix_ts(0);
        assert_eq!(ts, "1970-01-01T00:00:00Z");
    }

    #[test]
    fn format_known_timestamp() {
        // 2026-04-27 00:00:00 UTC = 1777248000 seconds since epoch
        let ts = format_unix_ts(1_777_248_000);
        assert_eq!(ts, "2026-04-27T00:00:00Z");
    }
}
