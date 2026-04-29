//! Git-source cache eviction (ADR-028 §4.5 — Phase 4.5).
//!
//! The `GitSource` cache lives at
//! `~/.sindri/cache/git/<sha256(url)>/<commit-sha>/`. Entries are
//! immutable once written — a particular `(url, sha)` checkout never
//! changes — so eviction is safe as long as nothing is currently
//! reading the entry being deleted.
//!
//! ## Policy
//!
//! Two thresholds are read from `~/.sindri/config.yaml`:
//!
//! - `cache.git.max_size` (default `10GB`) — total bytes under
//!   `~/.sindri/cache/git/`. When exceeded we evict commit-sha
//!   directories oldest-mtime-first until the total drops below the
//!   cap.
//! - `cache.git.max_age` (default `90d`) — any commit-sha directory
//!   whose mtime is older than this is evicted regardless of size.
//!
//! Both thresholds fire independently: a commit-sha dir is evicted if
//! *either* it's too old *or* removing it brings the cache back under
//! the size cap.
//!
//! ## Concurrency
//!
//! Eviction is best-effort with a single-process advisory file lock at
//! `<cache_root>/.eviction.lock`. When another process holds the lock
//! we log and skip eviction for this run — the cache will get cleaned
//! up by the next caller. We never wait for the lock; eviction runs at
//! the start of every `fetch_index` call and must not stall it.
//!
//! ## Triggering
//!
//! [`run_eviction`] is called once at the top of
//! `GitSourceRuntime::fetch_index`. Cost: one stat per cached commit
//! dir, plus one `remove_dir_all` per evicted dir — bounded by the
//! number of cached commits which is small in practice.

use fs4::fs_std::FileExt;
use sindri_core::cache_config::GitCacheConfig;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Lock file name placed at `<cache_root>/.eviction.lock` to serialise
/// concurrent eviction attempts within a single host.
const LOCK_FILE: &str = ".eviction.lock";

/// Bare-mirror sub-directory name laid down by `GitSourceRuntime`.
/// We never evict it — it's the upstream we re-clone from.
const BARE_DIR: &str = "_bare";

/// One discovered cache entry: a `<commit-sha>` directory under a
/// per-URL parent, with its disk size and last-modified mtime.
#[derive(Debug, Clone)]
struct CacheEntry {
    /// `<cache_root>/<sha256(url)>/<commit-sha>/`.
    path: PathBuf,
    /// Size in bytes (sum of every regular file under `path`).
    size_bytes: u64,
    /// Directory mtime (falls back to `UNIX_EPOCH` on stat failure so
    /// such entries are evicted first as "very old").
    mtime: SystemTime,
}

/// Run eviction at the cache root. Best-effort: returns `Ok(0)` when
/// the cache root does not exist, when the lock is held by another
/// process, or when no entries exceed the thresholds. Errors are
/// returned only for catastrophic IO failures — typical "couldn't
/// stat one entry" cases are logged and skipped.
///
/// Returns the number of commit-sha directories evicted.
pub fn run_eviction(cache_root: &Path, cfg: &GitCacheConfig) -> std::io::Result<usize> {
    if !cache_root.exists() {
        return Ok(0);
    }

    // Acquire the advisory lock. If another process holds it, log and
    // bail — we'll catch up on the next call.
    fs::create_dir_all(cache_root)?;
    let lock_path = cache_root.join(LOCK_FILE);
    let lock_file = fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&lock_path)?;
    if FileExt::try_lock_exclusive(&lock_file).is_err() {
        tracing::debug!(
            "git-cache eviction skipped: lock held at {}",
            lock_path.display()
        );
        return Ok(0);
    }

    let mut entries = collect_entries(cache_root);
    if entries.is_empty() {
        let _ = FileExt::unlock(&lock_file);
        return Ok(0);
    }

    let max_size = cfg.max_size_bytes();
    let max_age = cfg.max_age_duration();
    let now = SystemTime::now();

    // Phase 1: age-based eviction.
    let mut evicted = 0usize;
    entries.retain(|e| {
        let age = now.duration_since(e.mtime).unwrap_or(Duration::ZERO);
        if age > max_age {
            if remove_entry(e, max_age, "age").is_some() {
                evicted += 1;
            }
            false
        } else {
            true
        }
    });

    // Phase 2: size-based eviction. Sort survivors oldest-mtime-first,
    // pop until total ≤ cap.
    let total: u64 = entries.iter().map(|e| e.size_bytes).sum();
    if total > max_size {
        entries.sort_by_key(|e| e.mtime);
        let mut running = total;
        for e in &entries {
            if running <= max_size {
                break;
            }
            if remove_entry(e, max_age, "size").is_some() {
                running = running.saturating_sub(e.size_bytes);
                evicted += 1;
            }
        }
    }

    let _ = FileExt::unlock(&lock_file);
    Ok(evicted)
}

/// Walk `<cache_root>/<sha256(url)>/<commit-sha>/` and collect each
/// commit-sha directory's path, size, and mtime.
fn collect_entries(cache_root: &Path) -> Vec<CacheEntry> {
    let mut out = Vec::new();
    let url_dirs = match fs::read_dir(cache_root) {
        Ok(d) => d,
        Err(e) => {
            tracing::debug!(
                "git-cache: read_dir({}) failed: {}",
                cache_root.display(),
                e
            );
            return out;
        }
    };
    for url_entry in url_dirs.flatten() {
        let url_path = url_entry.path();
        if !url_path.is_dir() {
            continue;
        }
        // Top-level lock file is not a URL hash dir.
        if url_path.file_name().and_then(|s| s.to_str()) == Some(LOCK_FILE) {
            continue;
        }
        let commits = match fs::read_dir(&url_path) {
            Ok(d) => d,
            Err(_) => continue,
        };
        for commit_entry in commits.flatten() {
            let commit_path = commit_entry.path();
            if !commit_path.is_dir() {
                continue;
            }
            // Never touch the bare mirror — it's the upstream.
            if commit_path.file_name().and_then(|s| s.to_str()) == Some(BARE_DIR) {
                continue;
            }
            let mtime = fs::metadata(&commit_path)
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            let size_bytes = dir_size_bytes(&commit_path);
            out.push(CacheEntry {
                path: commit_path,
                size_bytes,
                mtime,
            });
        }
    }
    out
}

/// Recursively sum the size of every regular file under `root`.
fn dir_size_bytes(root: &Path) -> u64 {
    let mut total = 0u64;
    let walker = match fs::read_dir(root) {
        Ok(w) => w,
        Err(_) => return 0,
    };
    for entry in walker.flatten() {
        let path = entry.path();
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.is_dir() {
            total = total.saturating_add(dir_size_bytes(&path));
        } else if meta.is_file() {
            total = total.saturating_add(meta.len());
        }
    }
    total
}

/// Best-effort recursive remove with logging. Returns `Some(())` on
/// success so callers can count successful evictions.
fn remove_entry(e: &CacheEntry, _max_age: Duration, reason: &str) -> Option<()> {
    let now = SystemTime::now();
    let age_secs = now
        .duration_since(e.mtime)
        .unwrap_or(Duration::ZERO)
        .as_secs();
    let url_hash = e
        .path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("?")
        .to_string();
    let commit_sha = e
        .path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("?")
        .to_string();
    match fs::remove_dir_all(&e.path) {
        Ok(()) => {
            tracing::info!(
                url_hash = %url_hash,
                commit_sha = %commit_sha,
                reclaimed_bytes = e.size_bytes,
                age_days = age_secs / 86400,
                reason = %reason,
                "git-cache: evicted commit-sha directory",
            );
            Some(())
        }
        Err(err) => {
            tracing::warn!("git-cache: failed to evict {}: {}", e.path.display(), err);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::TempDir;

    /// Build a fixture cache layout:
    ///
    ///     root/<url_hash>/<commit_sha>/<files...>
    ///
    /// `bytes_per_file` is the size each file gets; `mtime_secs_ago` is
    /// passed to `filetime::set_file_mtime` to age the directory.
    fn make_entry(
        root: &Path,
        url_hash: &str,
        commit_sha: &str,
        bytes_per_file: u64,
        mtime_secs_ago: u64,
    ) -> PathBuf {
        let dir = root.join(url_hash).join(commit_sha);
        fs::create_dir_all(&dir).unwrap();
        let payload = vec![0u8; bytes_per_file as usize];
        fs::write(dir.join("blob.bin"), &payload).unwrap();
        // Age the directory by reaching back through filetime.
        let new_mtime = SystemTime::now()
            .checked_sub(Duration::from_secs(mtime_secs_ago))
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let new_mtime_filetime = filetime::FileTime::from_system_time(new_mtime);
        filetime::set_file_mtime(&dir, new_mtime_filetime).unwrap();
        dir
    }

    fn cfg(max_size: &str, max_age: &str) -> GitCacheConfig {
        GitCacheConfig {
            max_size: Some(max_size.to_string()),
            max_age: Some(max_age.to_string()),
        }
    }

    #[test]
    fn no_eviction_when_below_thresholds() {
        let tmp = TempDir::new().unwrap();
        make_entry(tmp.path(), "urlA", "shaA", 100, 60);
        let n = run_eviction(tmp.path(), &cfg("10GB", "90d")).unwrap();
        assert_eq!(n, 0);
        assert!(tmp.path().join("urlA").join("shaA").exists());
    }

    #[test]
    fn evicts_oldest_when_over_size_cap() {
        let tmp = TempDir::new().unwrap();
        // Two entries of 1MB each, total 2MB. Cap at 1MB.
        make_entry(tmp.path(), "urlA", "old", 1024 * 1024, 600);
        make_entry(tmp.path(), "urlA", "new", 1024 * 1024, 60);
        let n = run_eviction(tmp.path(), &cfg("1500KB", "90d")).unwrap();
        assert!(n >= 1, "expected eviction, got {}", n);
        // Oldest should be gone.
        assert!(!tmp.path().join("urlA").join("old").exists());
        // Newest survives.
        assert!(tmp.path().join("urlA").join("new").exists());
    }

    #[test]
    fn evicts_old_entries_regardless_of_size() {
        let tmp = TempDir::new().unwrap();
        // Tiny entry, well under any size cap, but very old.
        make_entry(tmp.path(), "urlA", "ancient", 100, 100 * 86400); // 100 days
        let n = run_eviction(tmp.path(), &cfg("10GB", "90d")).unwrap();
        assert_eq!(n, 1);
        assert!(!tmp.path().join("urlA").join("ancient").exists());
    }

    #[test]
    fn never_touches_bare_mirror_dir() {
        let tmp = TempDir::new().unwrap();
        // The `_bare` dir is the upstream mirror — eviction must skip
        // it even if it's old and large.
        make_entry(tmp.path(), "urlA", "_bare", 5 * 1024 * 1024, 200 * 86400);
        let n = run_eviction(tmp.path(), &cfg("1KB", "1d")).unwrap();
        assert_eq!(n, 0, "_bare should never be evicted");
        assert!(tmp.path().join("urlA").join("_bare").exists());
    }

    #[test]
    fn defaults_apply_when_config_absent() {
        // Empty config falls back to 10GB / 90d — entries below those
        // are kept.
        let tmp = TempDir::new().unwrap();
        make_entry(tmp.path(), "urlA", "shaA", 1024, 60);
        let n = run_eviction(tmp.path(), &GitCacheConfig::default()).unwrap();
        assert_eq!(n, 0);
        assert!(tmp.path().join("urlA").join("shaA").exists());
    }

    #[test]
    fn custom_values_drive_eviction() {
        let tmp = TempDir::new().unwrap();
        // 1MB entry; max_size 100KB → eviction.
        make_entry(tmp.path(), "urlA", "shaA", 1024 * 1024, 60);
        let n = run_eviction(tmp.path(), &cfg("100KB", "90d")).unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn lock_held_by_other_process_skips_eviction() {
        let tmp = TempDir::new().unwrap();
        make_entry(tmp.path(), "urlA", "shaA", 1024, 100 * 86400);

        // Hold the lock externally.
        fs::create_dir_all(tmp.path()).unwrap();
        let other = fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(tmp.path().join(LOCK_FILE))
            .unwrap();
        FileExt::lock_exclusive(&other).unwrap();

        let n = run_eviction(tmp.path(), &cfg("10GB", "90d")).unwrap();
        assert_eq!(n, 0, "should skip when lock held");
        // Old entry is still present because eviction was skipped.
        assert!(tmp.path().join("urlA").join("shaA").exists());

        FileExt::unlock(&other).unwrap();
    }

    #[test]
    fn empty_cache_root_is_a_no_op() {
        let tmp = TempDir::new().unwrap();
        // Don't create any entries.
        let n = run_eviction(tmp.path(), &cfg("10GB", "90d")).unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn nonexistent_cache_root_is_a_no_op() {
        let tmp = TempDir::new().unwrap();
        let nonexistent = tmp.path().join("does-not-exist");
        let n = run_eviction(&nonexistent, &cfg("10GB", "90d")).unwrap();
        assert_eq!(n, 0);
    }
}
