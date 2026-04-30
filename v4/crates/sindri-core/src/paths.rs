//! Filesystem path helpers, with a test-friendly home-directory override.
//!
//! All sindri code that needs to locate the user's home directory should
//! call [`home_dir`] (or one of the convenience helpers below) instead of
//! `dirs_next::home_dir()` directly.
//!
//! ## Why
//!
//! On Windows, `dirs_next::home_dir()` consults the Win32
//! `SHGetKnownFolderPath(FOLDERID_Profile)` API rather than environment
//! variables. That makes it impossible for an integration test to point
//! the sindri process at a tempdir-based `~/.sindri/...` layout by
//! setting `HOME` (Unix-only) or `USERPROFILE` (which `dirs_next` also
//! ignores in modern versions).
//!
//! The `SINDRI_HOME` env var sidesteps the platform difference: when set,
//! it overrides any OS-level lookup. Production users never need to set
//! it; tests do.

use std::path::PathBuf;

/// The env var that, when set, overrides the user's home directory for
/// every sindri lookup (cache, trust, ledger, plugins, history, ...).
pub const SINDRI_HOME_ENV: &str = "SINDRI_HOME";

/// Resolve the user's home directory, honouring [`SINDRI_HOME_ENV`] first.
///
/// Returns `None` only if both the env var is unset/empty AND
/// `dirs_next::home_dir()` cannot determine a home — extremely rare in
/// practice (would require an unconfigured user profile on Windows or a
/// missing `$HOME` on Unix). Callers should treat `None` as fatal.
pub fn home_dir() -> Option<PathBuf> {
    if let Ok(s) = std::env::var(SINDRI_HOME_ENV) {
        if !s.is_empty() {
            return Some(PathBuf::from(s));
        }
    }
    dirs_next::home_dir()
}

/// Convenience: `~/.sindri/<rest>`. Returns `None` on home-dir lookup
/// failure.
pub fn sindri_subpath(rest: &[&str]) -> Option<PathBuf> {
    let mut p = home_dir()?.join(".sindri");
    for seg in rest {
        p = p.join(seg);
    }
    Some(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------------------
    // home_dir — SINDRI_HOME override
    // ---------------------------------------------------------------------------

    #[test]
    fn home_dir_honours_sindri_home_env() {
        unsafe { std::env::set_var(SINDRI_HOME_ENV, "/tmp/fake-home") };
        let h = home_dir();
        unsafe { std::env::remove_var(SINDRI_HOME_ENV) };
        assert_eq!(h, Some(PathBuf::from("/tmp/fake-home")));
    }

    #[test]
    fn home_dir_empty_sindri_home_falls_back_to_dirs_next() {
        unsafe { std::env::set_var(SINDRI_HOME_ENV, "") };
        let h = home_dir();
        unsafe { std::env::remove_var(SINDRI_HOME_ENV) };
        // Falls back to dirs_next::home_dir() — may be Some or None depending on CI
        // environment, but must not panic.
        let _ = h;
    }

    #[test]
    fn home_dir_without_env_does_not_panic() {
        unsafe { std::env::remove_var(SINDRI_HOME_ENV) };
        let _ = home_dir(); // must not panic
    }

    // ---------------------------------------------------------------------------
    // sindri_subpath — path construction
    // ---------------------------------------------------------------------------

    #[test]
    fn sindri_subpath_empty_rest_is_dot_sindri() {
        unsafe { std::env::set_var(SINDRI_HOME_ENV, "/tmp/h") };
        let p = sindri_subpath(&[]);
        unsafe { std::env::remove_var(SINDRI_HOME_ENV) };
        assert_eq!(p, Some(PathBuf::from("/tmp/h/.sindri")));
    }

    #[test]
    fn sindri_subpath_single_segment() {
        unsafe { std::env::set_var(SINDRI_HOME_ENV, "/tmp/h") };
        let p = sindri_subpath(&["cache"]);
        unsafe { std::env::remove_var(SINDRI_HOME_ENV) };
        assert_eq!(p, Some(PathBuf::from("/tmp/h/.sindri/cache")));
    }

    #[test]
    fn sindri_subpath_multiple_segments() {
        unsafe { std::env::set_var(SINDRI_HOME_ENV, "/tmp/h") };
        let p = sindri_subpath(&["trust", "keys", "registry.pub"]);
        unsafe { std::env::remove_var(SINDRI_HOME_ENV) };
        assert_eq!(
            p,
            Some(PathBuf::from("/tmp/h/.sindri/trust/keys/registry.pub"))
        );
    }

    #[test]
    fn sindri_home_env_constant_value() {
        assert_eq!(SINDRI_HOME_ENV, "SINDRI_HOME");
    }
}
