//! `sindri rollback <component>` — roll one component back to its previous
//! pinned version in `sindri.lock` (ADR-011).
//!
//! Wave 3B scope:
//! - Read the current `sindri.lock` (JSON).
//! - Look up the component's per-component history file at
//!   `~/.sindri/history/<bom-hash>/<component>.jsonl`.
//! - If no history exists, return a clear error (we never invent a previous
//!   version — the user must run `sindri resolve` first to start tracking).
//! - Otherwise pop the most-recent JSONL entry, swap it into the lock,
//!   write atomically and append a `RolledBack` ledger event.
//!
//! The user must run `sindri apply` afterwards — rollback does not touch
//! installed state.

use crate::commands::log::{append_event, LedgerEvent};
use sindri_core::exit_codes::{
    EXIT_ERROR, EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_STALE_LOCKFILE, EXIT_SUCCESS,
};
use sindri_core::lockfile::{Lockfile, ResolvedComponent};
use std::path::{Path, PathBuf};

/// Arguments for `sindri rollback`.
pub struct RollbackArgs {
    /// Component name (matches `ResolvedComponent.id.name`).
    pub component: String,
    /// Lockfile path. Defaults to `sindri.lock`.
    pub lockfile: Option<String>,
    /// Override `~/.sindri/history` (test-only).
    pub history_root: Option<PathBuf>,
    /// Optional reason recorded in the ledger event.
    pub reason: Option<String>,
}

/// Entry point for `sindri rollback`.
pub fn run(args: RollbackArgs) -> i32 {
    let lock_path = PathBuf::from(args.lockfile.as_deref().unwrap_or("sindri.lock"));
    if !lock_path.exists() {
        eprintln!(
            "Lockfile '{}' not found — run `sindri resolve` first",
            lock_path.display()
        );
        return EXIT_STALE_LOCKFILE;
    }

    let content = match std::fs::read_to_string(&lock_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Cannot read lockfile: {}", e);
            return EXIT_STALE_LOCKFILE;
        }
    };

    let mut lockfile: Lockfile = match serde_json::from_str(&content) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Malformed lockfile: {}", e);
            return EXIT_STALE_LOCKFILE;
        }
    };

    let idx = match lockfile
        .components
        .iter()
        .position(|c| c.id.name == args.component)
    {
        Some(i) => i,
        None => {
            eprintln!(
                "Component '{}' is not present in {}",
                args.component,
                lock_path.display()
            );
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    let history_root = args
        .history_root
        .clone()
        .unwrap_or_else(default_history_root);
    let history_file = history_root
        .join(&lockfile.bom_hash)
        .join(format!("{}.jsonl", args.component));

    if !history_file.exists() {
        eprintln!(
            "No rollback history available for '{}'; first sindri resolve will start tracking.",
            args.component
        );
        eprintln!("Looked in: {}", history_file.display());
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }

    let previous = match pop_history_entry(&history_file) {
        Ok(Some(p)) => p,
        Ok(None) => {
            eprintln!("Rollback history for '{}' is empty.", args.component);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
        Err(e) => {
            eprintln!("Cannot read rollback history: {}", e);
            return EXIT_ERROR;
        }
    };

    let from_version = lockfile.components[idx].version.clone();
    let to_version = previous.version.clone();
    lockfile.components[idx] = previous;

    if let Err(e) = atomic_write_lock(&lock_path, &lockfile) {
        eprintln!("Failed to write lockfile: {}", e);
        return EXIT_ERROR;
    }

    let event = LedgerEvent {
        timestamp: now_secs(),
        event_type: "RolledBack".into(),
        component: args.component.clone(),
        version: format!("{} -> {}", from_version, to_version),
        target: lockfile.target.clone(),
        success: true,
        detail: args.reason.clone().or_else(|| {
            Some(format!(
                "rolled back from {} to {}",
                from_version, to_version
            ))
        }),
    };
    if let Err(e) = append_event(&event) {
        eprintln!(
            "Warning: rollback succeeded but ledger append failed: {}",
            e
        );
    }

    println!(
        "Rolled back '{}' from {} to {}.",
        args.component, from_version, to_version
    );
    println!("Run `sindri apply` to make the rollback take effect.");
    EXIT_SUCCESS
}

/// Append a `ResolvedComponent` to the per-component history file. Called by
/// the resolver whenever it overwrites an existing entry in `sindri.lock`.
///
/// Each line is a single JSON-encoded `ResolvedComponent` (JSONL).
pub fn append_history_entry(
    history_root: &Path,
    bom_hash: &str,
    component_name: &str,
    entry: &ResolvedComponent,
) -> std::io::Result<()> {
    use std::io::Write;
    let dir = history_root.join(bom_hash);
    std::fs::create_dir_all(&dir)?;
    let file = dir.join(format!("{}.jsonl", component_name));
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file)?;
    let line = serde_json::to_string(entry)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    writeln!(f, "{}", line)
}

/// Default location of the rollback history root: `~/.sindri/history`.
pub fn default_history_root() -> PathBuf {
    sindri_core::paths::home_dir()
        .unwrap_or_default()
        .join(".sindri")
        .join("history")
}

/// Pop (remove + return) the most-recent line of a JSONL history file.
/// Returns `Ok(None)` if the file is empty.
fn pop_history_entry(path: &Path) -> std::io::Result<Option<ResolvedComponent>> {
    let content = std::fs::read_to_string(path)?;
    let mut lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
    let last = match lines.pop() {
        Some(l) => l,
        None => return Ok(None),
    };
    let parsed: ResolvedComponent = serde_json::from_str(last)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    // Re-write the file atomically without the popped entry.
    let remaining = if lines.is_empty() {
        String::new()
    } else {
        let mut s = lines.join("\n");
        s.push('\n');
        s
    };
    let tmp = path.with_extension("jsonl.tmp");
    std::fs::write(&tmp, remaining)?;
    std::fs::rename(&tmp, path)?;
    Ok(Some(parsed))
}

fn atomic_write_lock(path: &Path, lockfile: &Lockfile) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(lockfile)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let tmp = path.with_extension("lock.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, path)
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sindri_core::component::{Backend, ComponentId};
    use sindri_core::version::Version;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn rc(name: &str, version: &str) -> ResolvedComponent {
        ResolvedComponent {
            id: ComponentId {
                backend: Backend::Brew,
                name: name.into(),
                qualifier: None,
            },
            version: Version::new(version),
            backend: Backend::Brew,
            oci_digest: None,
            checksums: HashMap::new(),
            depends_on: vec![],
            manifest: None,
            manifest_digest: None,
            component_digest: None,
            platforms: None,
        }
    }

    fn write_lock(path: &Path, lock: &Lockfile) {
        std::fs::write(path, serde_json::to_string_pretty(lock).unwrap()).unwrap();
    }

    #[test]
    fn no_history_errors_clearly() {
        let tmp = TempDir::new().unwrap();

        let lock = Lockfile {
            version: 1,
            bom_hash: "abc123".into(),
            target: "local".into(),
            components: vec![rc("git", "2.45.0")],
        };
        let lock_path = tmp.path().join("sindri.lock");
        write_lock(&lock_path, &lock);

        let history_root = tmp.path().join("history-root");
        let code = run(RollbackArgs {
            component: "git".into(),
            lockfile: Some(lock_path.to_string_lossy().into_owned()),
            history_root: Some(history_root),
            reason: None,
        });

        assert_eq!(code, EXIT_SCHEMA_OR_RESOLVE_ERROR);
    }

    #[test]
    fn pops_most_recent_entry_and_writes_lock() {
        let tmp = TempDir::new().unwrap();

        let lock = Lockfile {
            version: 1,
            bom_hash: "deadbeef".into(),
            target: "local".into(),
            components: vec![rc("git", "2.45.0")],
        };
        let lock_path = tmp.path().join("sindri.lock");
        write_lock(&lock_path, &lock);

        let history_root = tmp.path().join("history-root");
        // Pre-populate two history entries (oldest first).
        append_history_entry(&history_root, "deadbeef", "git", &rc("git", "2.43.0")).unwrap();
        append_history_entry(&history_root, "deadbeef", "git", &rc("git", "2.44.0")).unwrap();

        // Redirect the ledger so the test doesn't touch the user's $HOME.
        let fake_home = tmp.path().join("home");
        std::fs::create_dir_all(&fake_home).unwrap();
        let prev_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", &fake_home);

        let code = run(RollbackArgs {
            component: "git".into(),
            lockfile: Some(lock_path.to_string_lossy().into_owned()),
            history_root: Some(history_root.clone()),
            reason: Some("test".into()),
        });

        if let Some(h) = prev_home {
            std::env::set_var("HOME", h);
        } else {
            std::env::remove_var("HOME");
        }

        assert_eq!(code, EXIT_SUCCESS);

        // Lockfile should now reference 2.44.0 (the most-recent history entry).
        let after: Lockfile =
            serde_json::from_str(&std::fs::read_to_string(&lock_path).unwrap()).unwrap();
        assert_eq!(after.components[0].version.0, "2.44.0");

        // History file should still contain the older 2.43.0 entry.
        let remaining =
            std::fs::read_to_string(history_root.join("deadbeef").join("git.jsonl")).unwrap();
        assert!(remaining.contains("2.43.0"));
        assert!(!remaining.contains("2.44.0"));
    }
}
