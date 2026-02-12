//! Utility functions shared across CLI commands

use anyhow::Result;
use std::path::PathBuf;

// Re-export the canonical home directory function from sindri-core
pub use sindri_core::get_home_dir;

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
