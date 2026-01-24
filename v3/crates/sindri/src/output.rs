//! Terminal output utilities

use console::style;
use indicatif::{ProgressBar, ProgressStyle};

/// Print a success message
pub fn success(msg: &str) {
    println!("{} {}", style("✓").green().bold(), msg);
}

/// Print an error message
pub fn error(msg: &str) {
    eprintln!("{} {}", style("✗").red().bold(), msg);
}

/// Print a warning message
pub fn warning(msg: &str) {
    eprintln!("{} {}", style("⚠").yellow().bold(), msg);
}

/// Print a warning message (alias for warning)
pub fn warn(msg: &str) {
    warning(msg);
}

/// Print an info message
pub fn info(msg: &str) {
    println!("{} {}", style("ℹ").blue().bold(), msg);
}

/// Print a header
pub fn header(msg: &str) {
    println!("\n{}", style(msg).bold().underlined());
}

/// Print a key-value pair
pub fn kv(key: &str, value: &str) {
    println!("  {}: {}", style(key).dim(), value);
}

/// Create a spinner
pub fn spinner(msg: &str) -> ProgressBar {
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

/// Create a progress bar
#[allow(dead_code)] // Reserved for future use
pub fn progress_bar(len: u64, msg: &str) -> ProgressBar {
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
