//! Per-extension log file management
//!
//! Writes detailed installation output (stdout/stderr) to per-extension log files
//! at `~/.sindri/logs/<extension-name>/<timestamp>.log`. These files are linked from
//! ledger events via the optional `log_file` field, enabling `sindri extension log --detail`
//! to show full tool output for any event.

use crate::executor::InstallOutput;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

/// Manages per-extension log files
pub struct ExtensionLogWriter {
    /// Root directory for log files (default: ~/.sindri/logs)
    log_dir: PathBuf,
}

impl ExtensionLogWriter {
    /// Create a writer using the default log directory (~/.sindri/logs)
    pub fn new_default() -> Result<Self> {
        let home_dir = sindri_core::get_home_dir().context("Failed to get home directory")?;
        let log_dir = home_dir.join(".sindri").join("logs");
        Ok(Self { log_dir })
    }

    /// Create a writer with a custom log directory (for tests)
    pub fn new(log_dir: PathBuf) -> Self {
        Self { log_dir }
    }

    /// Write a log file for an extension installation
    ///
    /// Returns the path to the written log file.
    pub fn write_log(
        &self,
        extension_name: &str,
        timestamp: DateTime<Utc>,
        output: &InstallOutput,
    ) -> Result<PathBuf> {
        let ext_log_dir = self.log_dir.join(extension_name);
        std::fs::create_dir_all(&ext_log_dir).context(format!(
            "Failed to create log directory: {}",
            ext_log_dir.display()
        ))?;

        let filename = format!("{}.log", timestamp.format("%Y%m%dT%H%M%SZ"));
        let log_path = ext_log_dir.join(&filename);

        let mut content = String::new();
        content.push_str(&format!("# Extension: {}\n", extension_name));
        content.push_str(&format!(
            "# Timestamp: {}\n",
            timestamp.format("%Y-%m-%dT%H:%M:%SZ")
        ));
        content.push_str(&format!("# Method: {}\n", output.install_method));
        content.push_str(&format!("# Status: {}\n", output.exit_status));

        content.push_str("# --- stdout ---\n");
        for line in &output.stdout_lines {
            content.push_str(line);
            content.push('\n');
        }

        content.push_str("# --- stderr ---\n");
        for line in &output.stderr_lines {
            content.push_str(line);
            content.push('\n');
        }

        std::fs::write(&log_path, &content)
            .context(format!("Failed to write log file: {}", log_path.display()))?;

        debug!("Wrote extension log: {}", log_path.display());
        Ok(log_path)
    }

    /// Find the most recent log file for an extension
    ///
    /// Scans `~/.sindri/logs/<name>/` and returns the path to the lexicographically
    /// greatest filename (timestamps sort correctly as `YYYYMMDDTHHMMSSz.log`).
    /// Returns `None` if no log files exist for the extension.
    pub fn find_latest_log(&self, extension_name: &str) -> Option<PathBuf> {
        let ext_log_dir = self.log_dir.join(extension_name);
        if !ext_log_dir.is_dir() {
            return None;
        }

        let entries = std::fs::read_dir(&ext_log_dir).ok()?;
        entries
            .flatten()
            .filter(|e| e.path().is_file())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "log")
                    .unwrap_or(false)
            })
            .max_by_key(|e| e.file_name())
            .map(|e| e.path())
    }

    /// Read a log file's contents
    pub fn read_log(path: &Path) -> Result<String> {
        std::fs::read_to_string(path)
            .context(format!("Failed to read log file: {}", path.display()))
    }

    /// Clean up log files older than the retention period
    ///
    /// Returns the number of files removed. Failures on individual files
    /// are logged but do not cause the overall operation to fail.
    pub fn cleanup_old_logs(&self, retention_days: i64) -> Result<usize> {
        if !self.log_dir.exists() {
            return Ok(0);
        }

        let cutoff = Utc::now() - chrono::Duration::days(retention_days);
        let mut removed = 0;

        let entries = std::fs::read_dir(&self.log_dir).context("Failed to read log directory")?;

        for entry in entries.flatten() {
            let ext_dir = entry.path();
            if !ext_dir.is_dir() {
                continue;
            }

            let log_entries = match std::fs::read_dir(&ext_dir) {
                Ok(e) => e,
                Err(e) => {
                    warn!(
                        "Failed to read extension log directory {}: {}",
                        ext_dir.display(),
                        e
                    );
                    continue;
                }
            };

            for log_entry in log_entries.flatten() {
                let log_path = log_entry.path();
                if !log_path.is_file() {
                    continue;
                }

                // Parse timestamp from filename (YYYYMMDDTHHMMSSz.log)
                if let Some(ts) = parse_log_timestamp(&log_path) {
                    if ts < cutoff {
                        if let Err(e) = std::fs::remove_file(&log_path) {
                            warn!("Failed to remove old log {}: {}", log_path.display(), e);
                        } else {
                            removed += 1;
                        }
                    }
                }
            }

            // Remove empty extension log directories
            if std::fs::read_dir(&ext_dir)
                .map(|mut d| d.next().is_none())
                .unwrap_or(false)
            {
                let _ = std::fs::remove_dir(&ext_dir);
            }
        }

        Ok(removed)
    }
}

/// Parse a timestamp from a log filename like "20260213T143022Z.log"
fn parse_log_timestamp(path: &Path) -> Option<DateTime<Utc>> {
    let stem = path.file_stem()?.to_str()?;
    chrono::NaiveDateTime::parse_from_str(stem, "%Y%m%dT%H%M%SZ")
        .ok()
        .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_write_and_read_log() {
        let temp_dir = TempDir::new().unwrap();
        let writer = ExtensionLogWriter::new(temp_dir.path().to_path_buf());

        let output = InstallOutput {
            stdout_lines: vec!["line 1".to_string(), "line 2".to_string()],
            stderr_lines: vec!["warn: something".to_string()],
            install_method: "mise".to_string(),
            exit_status: "success".to_string(),
        };

        let timestamp = Utc::now();
        let path = writer.write_log("python", timestamp, &output).unwrap();

        assert!(path.exists());
        let content = ExtensionLogWriter::read_log(&path).unwrap();
        assert!(content.contains("# Extension: python"));
        assert!(content.contains("# Method: mise"));
        assert!(content.contains("# Status: success"));
        assert!(content.contains("line 1"));
        assert!(content.contains("line 2"));
        assert!(content.contains("warn: something"));
    }

    #[test]
    fn test_cleanup_old_logs() {
        let temp_dir = TempDir::new().unwrap();
        let writer = ExtensionLogWriter::new(temp_dir.path().to_path_buf());

        // Create an "old" log file with a past timestamp in the filename
        let ext_dir = temp_dir.path().join("python");
        std::fs::create_dir_all(&ext_dir).unwrap();
        let old_name = "20240101T000000Z.log";
        std::fs::write(ext_dir.join(old_name), "old log content").unwrap();

        // Create a "recent" log file
        let recent_name = "20260213T120000Z.log";
        std::fs::write(ext_dir.join(recent_name), "recent log content").unwrap();

        let removed = writer.cleanup_old_logs(90).unwrap();
        assert_eq!(removed, 1);
        assert!(!ext_dir.join(old_name).exists());
        assert!(ext_dir.join(recent_name).exists());
    }

    #[test]
    fn test_parse_log_timestamp() {
        let path = PathBuf::from("/tmp/logs/python/20260213T143022Z.log");
        let ts = parse_log_timestamp(&path);
        assert!(ts.is_some());
        let ts = ts.unwrap();
        assert_eq!(ts.format("%Y-%m-%d").to_string(), "2026-02-13");
    }

    #[test]
    fn test_parse_log_timestamp_invalid() {
        let path = PathBuf::from("/tmp/logs/python/garbage.log");
        assert!(parse_log_timestamp(&path).is_none());
    }

    #[test]
    fn test_find_latest_log() {
        let temp_dir = TempDir::new().unwrap();
        let writer = ExtensionLogWriter::new(temp_dir.path().to_path_buf());

        // No logs yet
        assert!(writer.find_latest_log("python").is_none());

        // Create some log files
        let ext_dir = temp_dir.path().join("python");
        std::fs::create_dir_all(&ext_dir).unwrap();
        std::fs::write(ext_dir.join("20260101T100000Z.log"), "old").unwrap();
        std::fs::write(ext_dir.join("20260213T143022Z.log"), "newest").unwrap();
        std::fs::write(ext_dir.join("20260201T120000Z.log"), "middle").unwrap();

        let latest = writer.find_latest_log("python").unwrap();
        assert!(latest.ends_with("20260213T143022Z.log"));

        // Different extension has no logs
        assert!(writer.find_latest_log("ruby").is_none());
    }

    #[test]
    fn test_cleanup_empty_dir() {
        let temp_dir = TempDir::new().unwrap();
        let writer = ExtensionLogWriter::new(temp_dir.path().to_path_buf());

        // No log dir yet
        let removed = writer.cleanup_old_logs(90).unwrap();
        assert_eq!(removed, 0);
    }
}
