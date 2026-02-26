//! Terminal output utilities
//!
//! When JSON mode is active (`--json` flag), all informational output
//! (info, success, header, kv) is redirected to stderr so that stdout
//! contains only the JSON payload. Spinners and progress bars become
//! no-ops in JSON mode.

use std::sync::OnceLock;

use console::style;
use indicatif::{ProgressBar, ProgressStyle};

// ── JSON mode state ──────────────────────────────────────────────────────────

static JSON_MODE: OnceLock<bool> = OnceLock::new();

/// Enable or disable JSON mode (call once, before any output).
pub fn set_json_mode(enabled: bool) {
    JSON_MODE.set(enabled).ok();
}

/// Returns `true` when the CLI was invoked with `--json`.
pub fn is_json_mode() -> bool {
    JSON_MODE.get().copied().unwrap_or(false)
}

// ── Output helpers ───────────────────────────────────────────────────────────

/// Print a success message (stderr in JSON mode)
pub fn success(msg: &str) {
    if is_json_mode() {
        eprintln!("{} {}", style("✓").green().bold(), msg);
    } else {
        println!("{} {}", style("✓").green().bold(), msg);
    }
}

/// Print an error message (always stderr)
pub fn error(msg: &str) {
    eprintln!("{} {}", style("✗").red().bold(), msg);
}

/// Print a warning message (always stderr)
pub fn warning(msg: &str) {
    eprintln!("{} {}", style("⚠").yellow().bold(), msg);
}

/// Print a warning message (alias for warning)
pub fn warn(msg: &str) {
    warning(msg);
}

/// Print an info message (stderr in JSON mode)
pub fn info(msg: &str) {
    if is_json_mode() {
        eprintln!("{} {}", style("ℹ").blue().bold(), msg);
    } else {
        println!("{} {}", style("ℹ").blue().bold(), msg);
    }
}

/// Print a header (stderr in JSON mode)
pub fn header(msg: &str) {
    if is_json_mode() {
        eprintln!("\n{}", style(msg).bold().underlined());
    } else {
        println!("\n{}", style(msg).bold().underlined());
    }
}

/// Print a key-value pair (stderr in JSON mode)
pub fn kv(key: &str, value: &str) {
    if is_json_mode() {
        eprintln!("  {}: {}", style(key).dim(), value);
    } else {
        println!("  {}: {}", style(key).dim(), value);
    }
}

/// Create a spinner (hidden no-op in JSON mode)
pub fn spinner(msg: &str) -> ProgressBar {
    if is_json_mode() {
        let pb = ProgressBar::hidden();
        pb.set_message(msg.to_string());
        return pb;
    }
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.blue} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    pb
}

/// Create a progress bar (hidden no-op in JSON mode)
pub fn progress_bar(len: u64, msg: &str) -> ProgressBar {
    if is_json_mode() {
        let pb = ProgressBar::hidden();
        pb.set_message(msg.to_string());
        return pb;
    }
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}",
        )
        .unwrap()
        .progress_chars("#>-"),
    );
    pb.set_message(msg.to_string());
    pb
}
