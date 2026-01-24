//! Utility functions for provider implementations

use anyhow::{anyhow, Result};
use std::process::Output;
use tracing::{debug, warn};

/// Check if a command is available in PATH
pub fn command_exists(cmd: &str) -> bool {
    which::which(cmd).is_ok()
}

/// Get command version
pub fn get_command_version(cmd: &str, version_flag: &str) -> Result<String> {
    let output = std::process::Command::new(cmd).arg(version_flag).output()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Some tools output version to stderr
        let version = if stdout.trim().is_empty() {
            stderr.trim().to_string()
        } else {
            stdout.trim().to_string()
        };
        Ok(version)
    } else {
        Err(anyhow!("Failed to get version for {}", cmd))
    }
}

/// Run a command and return output
#[allow(dead_code)] // Reserved for future use
pub fn run_command(cmd: &str, args: &[&str]) -> Result<Output> {
    debug!("Running: {} {}", cmd, args.join(" "));

    let output = std::process::Command::new(cmd).args(args).output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!(
            "Command failed: {} {}\nStderr: {}",
            cmd,
            args.join(" "),
            stderr
        );
    }

    Ok(output)
}

/// Run a command asynchronously
#[allow(dead_code)] // Reserved for future use
pub async fn run_command_async(cmd: &str, args: &[&str]) -> Result<Output> {
    debug!("Running async: {} {}", cmd, args.join(" "));

    let output = tokio::process::Command::new(cmd)
        .args(args)
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!(
            "Command failed: {} {}\nStderr: {}",
            cmd,
            args.join(" "),
            stderr
        );
    }

    Ok(output)
}

/// Parse memory string (e.g., "4GB", "512MB") to bytes
#[allow(dead_code)] // Reserved for future use
pub fn parse_memory(mem: &str) -> Result<u64> {
    let mem = mem.trim().to_uppercase();

    if let Some(gb) = mem.strip_suffix("GB") {
        let value: u64 = gb.parse()?;
        Ok(value * 1024 * 1024 * 1024)
    } else if let Some(mb) = mem.strip_suffix("MB") {
        let value: u64 = mb.parse()?;
        Ok(value * 1024 * 1024)
    } else {
        Err(anyhow!(
            "Invalid memory format: {}. Expected format: NGB or NMB",
            mem
        ))
    }
}

/// Format bytes as human-readable string
#[allow(dead_code)] // Reserved for future use
pub fn format_bytes(bytes: u64) -> String {
    const GB: u64 = 1024 * 1024 * 1024;
    const MB: u64 = 1024 * 1024;
    const KB: u64 = 1024;

    if bytes >= GB {
        format!("{:.1}GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}KB", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}
