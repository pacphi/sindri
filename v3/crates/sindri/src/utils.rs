//! Utility functions shared across CLI commands

use anyhow::{anyhow, Result};
use std::path::PathBuf;

/// Get the user's home directory
///
/// Prefers the HOME environment variable over dirs::home_dir() because:
/// - In Docker containers with volume mounts, HOME may be set to ALT_HOME
/// - dirs::home_dir() reads from /etc/passwd which doesn't respect env overrides
/// - Shell scripts use $HOME, so we need consistency with bootstrap/entrypoint
pub fn get_home_dir() -> Result<PathBuf> {
    // First check HOME environment variable (respects Docker ALT_HOME setup)
    if let Ok(home) = std::env::var("HOME") {
        return Ok(PathBuf::from(home));
    }

    // Fallback to dirs::home_dir() for non-container environments
    dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))
}

/// Get the sindri configuration directory (~/.sindri)
pub fn get_sindri_dir() -> Result<PathBuf> {
    Ok(get_home_dir()?.join(".sindri"))
}

/// Get the cache directory (~/.sindri/cache)
pub fn get_cache_dir() -> Result<PathBuf> {
    Ok(get_sindri_dir()?.join("cache"))
}

/// Get the extensions directory
///
/// Returns the appropriate extensions directory based on deployment mode:
/// - SINDRI_EXT_HOME: Custom or bundled path (e.g., /opt/sindri/extensions)
/// - Fallback: ~/.sindri/extensions (respects HOME env var for Docker compatibility)
pub fn get_extensions_dir() -> Result<PathBuf> {
    // Check for explicit SINDRI_EXT_HOME environment variable
    if let Ok(ext_home) = std::env::var("SINDRI_EXT_HOME") {
        return Ok(PathBuf::from(ext_home));
    }

    // Fallback to home directory for downloaded extensions
    Ok(get_sindri_dir()?.join("extensions"))
}

/// Get the manifest path (~/.sindri/manifest.yaml)
pub fn get_manifest_path() -> Result<PathBuf> {
    Ok(get_sindri_dir()?.join("manifest.yaml"))
}
