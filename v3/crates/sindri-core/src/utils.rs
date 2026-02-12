//! Shared utility functions for Sindri crates

use anyhow::anyhow;
use std::path::PathBuf;

/// Get the user's home directory
///
/// Prefers the HOME environment variable over dirs::home_dir() because:
/// - In Docker containers with volume mounts, HOME may be set to ALT_HOME
/// - dirs::home_dir() reads from /etc/passwd which doesn't respect env overrides
/// - Shell scripts use $HOME, so we need consistency with bootstrap/entrypoint
pub fn get_home_dir() -> anyhow::Result<PathBuf> {
    // First check HOME environment variable (respects Docker ALT_HOME setup)
    if let Ok(home) = std::env::var("HOME") {
        return Ok(PathBuf::from(home));
    }

    // Fallback to dirs::home_dir() for non-container environments
    dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_home_dir_from_env() {
        // HOME is typically set in CI/test environments
        if std::env::var("HOME").is_ok() {
            let home = get_home_dir().unwrap();
            assert!(!home.as_os_str().is_empty());
        }
    }
}
