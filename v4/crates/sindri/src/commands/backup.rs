//! `sindri backup` / `sindri restore` (Sprint 12, Wave 4C).
//!
//! Backup produces a `tar.gz` of the user's sindri state:
//!   * Project files: `sindri.yaml`, `sindri.policy.yaml`, `sindri.lock`,
//!     and any `sindri.<target>.lock`.
//!   * `~/.sindri/ledger.jsonl`
//!   * `~/.sindri/trust/`
//!   * `~/.sindri/plugins/`
//!   * `~/.sindri/history/`
//!   * `~/.sindri/cache/registries/` only when `--include-cache` is set.
//!
//! Restore extracts an archive with default-deny overwrite semantics and
//! refuses entries with absolute paths or `..` traversal components.

use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use serde::Serialize;
use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use tar::{Archive, Builder, Header};

/// CLI args for `sindri backup`.
pub struct BackupArgs {
    /// Destination directory or full file path.
    pub output: Option<PathBuf>,
    /// Include `~/.sindri/cache/registries/` (large; off by default).
    pub include_cache: bool,
}

/// CLI args for `sindri restore`.
pub struct RestoreArgs {
    /// Path to the `.tar.gz` archive.
    pub archive: PathBuf,
    /// Print the archive's file list without writing.
    pub dry_run: bool,
    /// Overwrite existing destination files.
    pub force: bool,
}

#[derive(Debug, Serialize)]
struct BackupReport {
    archive: String,
    entries: usize,
}

/// Public entry point for `sindri backup`.
pub fn run_backup(args: BackupArgs) -> i32 {
    let home = match dirs_next::home_dir() {
        Some(h) => h,
        None => {
            eprintln!("error: cannot determine $HOME");
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };
    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: cwd: {}", e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };
    let dest = resolve_output_path(args.output.as_deref(), &cwd);

    match write_backup(&dest, &cwd, &home, args.include_cache) {
        Ok(report) => {
            println!(
                "Backup written to {} ({} entries)",
                report.archive_path.display(),
                report.entries
            );
            EXIT_SUCCESS
        }
        Err(e) => {
            eprintln!("error: backup failed: {}", e);
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        }
    }
}

/// Public entry point for `sindri restore`.
pub fn run_restore(args: RestoreArgs) -> i32 {
    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: cwd: {}", e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };
    let home = match dirs_next::home_dir() {
        Some(h) => h,
        None => {
            eprintln!("error: cannot determine $HOME");
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };
    match restore_archive(&args.archive, &cwd, &home, args.dry_run, args.force) {
        Ok(n) => {
            if args.dry_run {
                println!("(dry-run) would extract {} entries", n);
            } else {
                println!("Restored {} entries", n);
            }
            EXIT_SUCCESS
        }
        Err(e) => {
            eprintln!("error: restore failed: {}", e);
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        }
    }
}

fn resolve_output_path(output: Option<&Path>, cwd: &Path) -> PathBuf {
    let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let default_name = format!("sindri-backup-{}.tar.gz", stamp);
    match output {
        Some(p) if p.is_dir() => p.join(default_name),
        Some(p) => p.to_path_buf(),
        None => cwd.join(default_name),
    }
}

/// Result of [`write_backup`]. Public so tests can assert.
pub struct WriteReport {
    /// Path of the archive that was written.
    pub archive_path: PathBuf,
    /// Number of file entries appended.
    pub entries: usize,
}

/// Write a backup tarball at `dest`. Test-friendly: caller supplies
/// `project_dir` and `home_dir` so tests can sandbox under `TempDir`s.
pub fn write_backup(
    dest: &Path,
    project_dir: &Path,
    home_dir: &Path,
    include_cache: bool,
) -> std::io::Result<WriteReport> {
    if let Some(parent) = dest.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let f = File::create(dest)?;
    let gz = GzEncoder::new(f, Compression::default());
    let mut tar = Builder::new(gz);
    let mut entries = 0usize;

    // 1. Project files.
    let project_files: &[&str] = &["sindri.yaml", "sindri.policy.yaml", "sindri.lock"];
    for rel in project_files {
        let abs = project_dir.join(rel);
        if abs.is_file() {
            tar.append_path_with_name(&abs, format!("project/{}", rel))?;
            entries += 1;
        }
    }
    // Per-target lockfiles: sindri.*.lock
    if let Ok(rd) = std::fs::read_dir(project_dir) {
        for ent in rd.flatten() {
            let name = ent.file_name();
            let name_s = name.to_string_lossy();
            if name_s.starts_with("sindri.") && name_s.ends_with(".lock") && name_s != "sindri.lock"
            {
                tar.append_path_with_name(ent.path(), format!("project/{}", name_s))?;
                entries += 1;
            }
        }
    }

    // 2. ~/.sindri files & dirs.
    let ledger = home_dir.join(".sindri").join("ledger.jsonl");
    if ledger.is_file() {
        tar.append_path_with_name(&ledger, "home/.sindri/ledger.jsonl")?;
        entries += 1;
    }
    for sub in &["trust", "plugins", "history"] {
        let dir = home_dir.join(".sindri").join(sub);
        if dir.is_dir() {
            entries += append_dir_recursive(&mut tar, &dir, &format!("home/.sindri/{}", sub))?;
        }
    }
    if include_cache {
        let cache = home_dir.join(".sindri").join("cache").join("registries");
        if cache.is_dir() {
            entries += append_dir_recursive(&mut tar, &cache, "home/.sindri/cache/registries")?;
        }
    }

    let gz = tar.into_inner()?;
    gz.finish()?;
    Ok(WriteReport {
        archive_path: dest.to_path_buf(),
        entries,
    })
}

fn append_dir_recursive(
    tar: &mut Builder<GzEncoder<File>>,
    src: &Path,
    archive_prefix: &str,
) -> std::io::Result<usize> {
    let mut count = 0usize;
    for entry in walk(src) {
        let entry = entry?;
        if !entry.is_file() {
            continue;
        }
        let rel = entry.strip_prefix(src).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "strip_prefix failed")
        })?;
        let name = format!("{}/{}", archive_prefix, rel.to_string_lossy());
        // Use a fresh header so we don't carry over uid/gid weirdness.
        let mut f = File::open(&entry)?;
        let metadata = f.metadata()?;
        let mut header = Header::new_gnu();
        header.set_size(metadata.len());
        header.set_mode(0o644);
        header.set_mtime(
            metadata
                .modified()
                .ok()
                .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0),
        );
        header.set_cksum();
        tar.append_data(&mut header, &name, &mut f)?;
        count += 1;
    }
    Ok(count)
}

fn walk(root: &Path) -> Vec<std::io::Result<PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(cur) = stack.pop() {
        match std::fs::read_dir(&cur) {
            Ok(rd) => {
                for ent in rd {
                    match ent {
                        Ok(e) => {
                            let p = e.path();
                            if p.is_dir() {
                                stack.push(p);
                            } else {
                                out.push(Ok(p));
                            }
                        }
                        Err(e) => out.push(Err(e)),
                    }
                }
            }
            Err(e) => out.push(Err(e)),
        }
    }
    out
}

/// Restore `archive` into `project_dir` (for `project/` entries) and
/// `home_dir` (for `home/.sindri/` entries). Returns the number of
/// entries written (or that would be written, in `dry_run`).
pub fn restore_archive(
    archive: &Path,
    project_dir: &Path,
    home_dir: &Path,
    dry_run: bool,
    force: bool,
) -> std::io::Result<usize> {
    let f = File::open(archive)?;
    let gz = GzDecoder::new(f);
    let mut ar = Archive::new(gz);
    ar.set_preserve_permissions(false);
    ar.set_unpack_xattrs(false);

    let mut count = 0usize;
    for entry in ar.entries()? {
        let mut e = entry?;
        let path_in_tar = e.path()?.into_owned();
        validate_entry_path(&path_in_tar)?;
        let dest = match map_destination(&path_in_tar, project_dir, home_dir) {
            Some(d) => d,
            None => continue,
        };
        if dry_run {
            println!("would extract: {}", dest.display());
            count += 1;
            continue;
        }
        if dest.exists() && !force {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("refusing to overwrite {} without --force", dest.display()),
            ));
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut buf = Vec::new();
        e.read_to_end(&mut buf)?;
        let mut out = File::create(&dest)?;
        out.write_all(&buf)?;
        count += 1;
    }
    Ok(count)
}

fn validate_entry_path(path: &Path) -> std::io::Result<()> {
    if path.is_absolute() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("archive contains absolute path: {}", path.display()),
        ));
    }
    for comp in path.components() {
        if matches!(comp, Component::ParentDir) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("archive contains parent-dir traversal: {}", path.display()),
            ));
        }
    }
    Ok(())
}

fn map_destination(archive_path: &Path, project_dir: &Path, home_dir: &Path) -> Option<PathBuf> {
    let mut comps = archive_path.components();
    match comps.next()? {
        Component::Normal(top) if top == "project" => {
            let rest: PathBuf = comps.collect();
            Some(project_dir.join(rest))
        }
        Component::Normal(top) if top == "home" => {
            let rest: PathBuf = comps.collect();
            Some(home_dir.join(rest))
        }
        _ => None,
    }
}

// ---- tests --------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tar::Builder as TarBuilder;
    use tempfile::TempDir;

    fn populate_state(project: &Path, home: &Path) {
        std::fs::write(project.join("sindri.yaml"), "name: test\ncomponents: []\n").unwrap();
        std::fs::write(project.join("sindri.lock"), "lock-v1").unwrap();
        std::fs::write(project.join("sindri.local.lock"), "per-target").unwrap();

        let sindri_home = home.join(".sindri");
        std::fs::create_dir_all(sindri_home.join("trust/registry-a")).unwrap();
        std::fs::write(sindri_home.join("trust/registry-a/cosign-1.pub"), "PEM").unwrap();
        std::fs::create_dir_all(sindri_home.join("plugins")).unwrap();
        std::fs::write(sindri_home.join("plugins/local.json"), "{}").unwrap();
        std::fs::create_dir_all(sindri_home.join("history")).unwrap();
        std::fs::write(sindri_home.join("history/2026-01.log"), "rolled").unwrap();
        std::fs::write(
            sindri_home.join("ledger.jsonl"),
            "{\"event\":\"install\"}\n",
        )
        .unwrap();
    }

    #[test]
    fn round_trip_preserves_files() {
        let project = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        populate_state(project.path(), home.path());

        let archive = TempDir::new().unwrap();
        let dest = archive.path().join("backup.tar.gz");
        let report = write_backup(&dest, project.path(), home.path(), false).unwrap();
        assert!(report.entries >= 6);
        assert!(dest.is_file());

        // Wipe original state.
        let project2 = TempDir::new().unwrap();
        let home2 = TempDir::new().unwrap();

        let n = restore_archive(&dest, project2.path(), home2.path(), false, false).unwrap();
        assert_eq!(n, report.entries);

        assert_eq!(
            std::fs::read_to_string(project2.path().join("sindri.yaml")).unwrap(),
            "name: test\ncomponents: []\n"
        );
        assert_eq!(
            std::fs::read_to_string(project2.path().join("sindri.local.lock")).unwrap(),
            "per-target"
        );
        assert_eq!(
            std::fs::read_to_string(home2.path().join(".sindri/ledger.jsonl")).unwrap(),
            "{\"event\":\"install\"}\n"
        );
        assert_eq!(
            std::fs::read_to_string(home2.path().join(".sindri/trust/registry-a/cosign-1.pub"))
                .unwrap(),
            "PEM"
        );
        assert_eq!(
            std::fs::read_to_string(home2.path().join(".sindri/history/2026-01.log")).unwrap(),
            "rolled"
        );
    }

    #[test]
    fn restore_dry_run_does_not_write() {
        let project = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        populate_state(project.path(), home.path());
        let archive = TempDir::new().unwrap();
        let dest = archive.path().join("backup.tar.gz");
        write_backup(&dest, project.path(), home.path(), false).unwrap();

        let project2 = TempDir::new().unwrap();
        let home2 = TempDir::new().unwrap();
        let n = restore_archive(&dest, project2.path(), home2.path(), true, false).unwrap();
        assert!(n > 0);
        // Nothing actually written.
        assert!(!project2.path().join("sindri.yaml").exists());
        assert!(!home2.path().join(".sindri/ledger.jsonl").exists());
    }

    #[test]
    fn restore_refuses_overwrite_without_force() {
        let project = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        populate_state(project.path(), home.path());
        let archive = TempDir::new().unwrap();
        let dest = archive.path().join("backup.tar.gz");
        write_backup(&dest, project.path(), home.path(), false).unwrap();

        // Restore destination already has a sindri.yaml.
        let project2 = TempDir::new().unwrap();
        let home2 = TempDir::new().unwrap();
        std::fs::write(project2.path().join("sindri.yaml"), "EXISTING").unwrap();

        let err = restore_archive(&dest, project2.path(), home2.path(), false, false).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
        // Pre-existing content untouched.
        assert_eq!(
            std::fs::read_to_string(project2.path().join("sindri.yaml")).unwrap(),
            "EXISTING"
        );

        // With --force it succeeds.
        let n = restore_archive(&dest, project2.path(), home2.path(), false, true).unwrap();
        assert!(n > 0);
        assert_ne!(
            std::fs::read_to_string(project2.path().join("sindri.yaml")).unwrap(),
            "EXISTING"
        );
    }

    #[test]
    fn restore_rejects_path_traversal() {
        let archive_dir = TempDir::new().unwrap();
        let archive = archive_dir.path().join("evil.tar.gz");
        // Hand-craft an archive with a traversal entry. The `tar` builder
        // refuses to write a `..` filename, so we write the raw header
        // bytes directly and seed the name field manually.
        let f = File::create(&archive).unwrap();
        let mut gz = GzEncoder::new(f, Compression::default());
        let mut header = Header::new_gnu();
        let payload = b"pwned";
        header.set_size(payload.len() as u64);
        header.set_mode(0o644);
        header.set_entry_type(tar::EntryType::Regular);
        // Set the legacy USTAR name field directly (bypasses the
        // safety checks in `Builder::append_data`).
        let bytes = header.as_old_mut().name.as_mut();
        let evil = b"../etc/passwd";
        bytes[..evil.len()].copy_from_slice(evil);
        header.set_cksum();
        gz.write_all(header.as_bytes()).unwrap();
        // Pad the data block to 512 bytes.
        let mut block = [0u8; 512];
        block[..payload.len()].copy_from_slice(payload);
        gz.write_all(&block).unwrap();
        // Two zero blocks → end of archive.
        gz.write_all(&[0u8; 1024]).unwrap();
        gz.finish().unwrap();
        let _ = TarBuilder::<File>::new; // keep import alive for other tests

        let project2 = TempDir::new().unwrap();
        let home2 = TempDir::new().unwrap();
        let err =
            restore_archive(&archive, project2.path(), home2.path(), false, true).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("parent-dir"));
    }
}
