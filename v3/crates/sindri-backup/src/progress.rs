//! Progress reporting for backup operations.
//!
//! Provides visual feedback during long-running backup and restore operations.

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::sync::Arc;

/// Progress reporter for backup operations.
#[derive(Debug, Clone)]
pub struct BackupProgress {
    multi: Arc<MultiProgress>,
    scan_bar: Option<ProgressBar>,
    archive_bar: Option<ProgressBar>,
}

impl BackupProgress {
    /// Creates a new backup progress reporter.
    pub fn new() -> Self {
        Self {
            multi: Arc::new(MultiProgress::new()),
            scan_bar: None,
            archive_bar: None,
        }
    }

    /// Starts the scanning phase progress.
    pub fn start_scan(&mut self, message: &str) {
        let bar = self.multi.add(ProgressBar::new_spinner());
        bar.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        bar.set_message(message.to_string());
        bar.enable_steady_tick(std::time::Duration::from_millis(100));
        self.scan_bar = Some(bar);
    }

    /// Updates the scan progress message.
    pub fn update_scan(&self, message: &str) {
        if let Some(bar) = &self.scan_bar {
            bar.set_message(message.to_string());
        }
    }

    /// Finishes the scanning phase.
    pub fn finish_scan(&self, message: &str) {
        if let Some(bar) = &self.scan_bar {
            bar.finish_with_message(message.to_string());
        }
    }

    /// Starts the archive creation phase with a known file count.
    pub fn start_archive(&mut self, total_files: u64, message: &str) {
        let bar = self.multi.add(ProgressBar::new(total_files));
        bar.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} files ({percent}%)")
                .unwrap()
                .progress_chars("#>-"),
        );
        bar.set_message(message.to_string());
        self.archive_bar = Some(bar);
    }

    /// Increments the archive progress by one file.
    pub fn inc_archive(&self) {
        if let Some(bar) = &self.archive_bar {
            bar.inc(1);
        }
    }

    /// Increments the archive progress by multiple files.
    pub fn inc_archive_by(&self, n: u64) {
        if let Some(bar) = &self.archive_bar {
            bar.inc(n);
        }
    }

    /// Updates the archive progress message.
    pub fn update_archive(&self, message: &str) {
        if let Some(bar) = &self.archive_bar {
            bar.set_message(message.to_string());
        }
    }

    /// Finishes the archive phase.
    pub fn finish_archive(&self, message: &str) {
        if let Some(bar) = &self.archive_bar {
            bar.finish_with_message(message.to_string());
        }
    }

    /// Finishes all progress bars.
    pub fn finish_all(&self) {
        if let Some(bar) = &self.scan_bar {
            bar.finish_and_clear();
        }
        if let Some(bar) = &self.archive_bar {
            bar.finish_and_clear();
        }
    }
}

impl Default for BackupProgress {
    fn default() -> Self {
        Self::new()
    }
}

/// Progress reporter for restore operations.
#[derive(Debug, Clone)]
pub struct RestoreProgress {
    multi: Arc<MultiProgress>,
    extract_bar: Option<ProgressBar>,
}

impl RestoreProgress {
    /// Creates a new restore progress reporter.
    pub fn new() -> Self {
        Self {
            multi: Arc::new(MultiProgress::new()),
            extract_bar: None,
        }
    }

    /// Starts the extraction phase with a known file count.
    pub fn start_extract(&mut self, total_files: u64, message: &str) {
        let bar = self.multi.add(ProgressBar::new(total_files));
        bar.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.green/blue}] {pos}/{len} files ({percent}%)")
                .unwrap()
                .progress_chars("#>-"),
        );
        bar.set_message(message.to_string());
        self.extract_bar = Some(bar);
    }

    /// Increments the extraction progress by one file.
    pub fn inc_extract(&self) {
        if let Some(bar) = &self.extract_bar {
            bar.inc(1);
        }
    }

    /// Increments the extraction progress by multiple files.
    pub fn inc_extract_by(&self, n: u64) {
        if let Some(bar) = &self.extract_bar {
            bar.inc(n);
        }
    }

    /// Updates the extraction progress message.
    pub fn update_extract(&self, message: &str) {
        if let Some(bar) = &self.extract_bar {
            bar.set_message(message.to_string());
        }
    }

    /// Finishes the extraction phase.
    pub fn finish_extract(&self, message: &str) {
        if let Some(bar) = &self.extract_bar {
            bar.finish_with_message(message.to_string());
        }
    }

    /// Finishes all progress bars.
    pub fn finish_all(&self) {
        if let Some(bar) = &self.extract_bar {
            bar.finish_and_clear();
        }
    }
}

impl Default for RestoreProgress {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple spinner progress for operations without known size.
#[derive(Debug)]
pub struct SpinnerProgress {
    bar: ProgressBar,
}

impl SpinnerProgress {
    /// Creates a new spinner progress indicator.
    pub fn new(message: &str) -> Self {
        let bar = ProgressBar::new_spinner();
        bar.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        bar.set_message(message.to_string());
        bar.enable_steady_tick(std::time::Duration::from_millis(100));
        Self { bar }
    }

    /// Updates the progress message.
    pub fn update(&self, message: &str) {
        self.bar.set_message(message.to_string());
    }

    /// Finishes the progress with a final message.
    pub fn finish(&self, message: &str) {
        self.bar.finish_with_message(message.to_string());
    }

    /// Finishes and clears the progress bar.
    pub fn finish_and_clear(&self) {
        self.bar.finish_and_clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_progress_creation() {
        let progress = BackupProgress::new();
        assert!(progress.scan_bar.is_none());
        assert!(progress.archive_bar.is_none());
    }

    #[test]
    fn test_restore_progress_creation() {
        let progress = RestoreProgress::new();
        assert!(progress.extract_bar.is_none());
    }

    #[test]
    fn test_spinner_progress_creation() {
        let progress = SpinnerProgress::new("Testing...");
        // Just ensure it creates without panicking
        progress.finish("Done");
    }

    #[test]
    fn test_backup_progress_lifecycle() {
        let mut progress = BackupProgress::new();

        // Start scan
        progress.start_scan("Scanning files...");
        assert!(progress.scan_bar.is_some());

        // Update scan
        progress.update_scan("Found 100 files");

        // Finish scan
        progress.finish_scan("Scan complete");

        // Start archive
        progress.start_archive(100, "Creating archive...");
        assert!(progress.archive_bar.is_some());

        // Increment progress
        progress.inc_archive();
        progress.inc_archive_by(5);

        // Update archive
        progress.update_archive("Compressing...");

        // Finish archive
        progress.finish_archive("Archive complete");

        // Finish all
        progress.finish_all();
    }

    #[test]
    fn test_restore_progress_lifecycle() {
        let mut progress = RestoreProgress::new();

        // Start extraction
        progress.start_extract(50, "Extracting files...");
        assert!(progress.extract_bar.is_some());

        // Increment progress
        progress.inc_extract();
        progress.inc_extract_by(10);

        // Update extraction
        progress.update_extract("Extracting configs...");

        // Finish extraction
        progress.finish_extract("Extraction complete");

        // Finish all
        progress.finish_all();
    }
}
