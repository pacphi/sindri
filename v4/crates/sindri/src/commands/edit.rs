//! `sindri edit [target]` — open `$EDITOR` on a sindri config file with
//! save-time validation and a rollback-on-failure safety net (ADR-011).
//!
//! The `target` selector decides which file is opened:
//!
//! | target   | file                                |
//! |----------|-------------------------------------|
//! | (none)   | `sindri.yaml`                       |
//! | `policy` | `sindri.policy.yaml`                |
//!
//! `sindri edit --schema` does not open an editor — it prints the local
//! JSON-schema path so the user can wire LSP support themselves.
//!
//! Workflow:
//! 1. Snapshot the original file to `<file>.bak`.
//! 2. Spawn `$EDITOR` (default `vi`) on the file and wait for exit.
//! 3. Run `sindri validate` against the edited file. On success → delete the
//!    `.bak`. On failure → prompt the user to re-open. A second failure
//!    restores the `.bak` and exits with [`EXIT_SCHEMA_OR_RESOLVE_ERROR`].
//!
//! See ADR-011 §`edit` for the rationale.

use sindri_core::exit_codes::{EXIT_ERROR, EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Arguments for `sindri edit`.
pub struct EditArgs {
    /// Optional positional selector. Currently only `policy` has any meaning;
    /// any other value triggers an error so we don't silently fall through to
    /// editing `sindri.yaml`.
    pub target: Option<String>,
    /// `--schema`: print the local JSON-schema path and exit.
    pub schema: bool,
    /// Optional override of `$EDITOR` for tests/CI.
    pub editor_override: Option<String>,
    /// If `true`, skip the interactive re-open prompt and treat the first
    /// validation failure as terminal. Used by tests and `--no-prompt`.
    pub non_interactive: bool,
    /// Test-only override of the file path (bypasses `target` resolution).
    pub path_override: Option<PathBuf>,
}

/// Entry point for `sindri edit`.
pub fn run(args: EditArgs) -> i32 {
    if args.schema {
        return print_schema_path(args.target.as_deref());
    }

    let path = match args.path_override.clone() {
        Some(p) => p,
        None => match resolve_target(args.target.as_deref()) {
            Ok(p) => p,
            Err(msg) => {
                eprintln!("{}", msg);
                return EXIT_SCHEMA_OR_RESOLVE_ERROR;
            }
        },
    };

    if !path.exists() {
        eprintln!(
            "Error: {} not found. Run `sindri init` to create one.",
            path.display()
        );
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }

    let bak_path = path.with_extension(format!(
        "{}.bak",
        path.extension().and_then(|s| s.to_str()).unwrap_or("yaml")
    ));

    if let Err(e) = std::fs::copy(&path, &bak_path) {
        eprintln!("Failed to create backup at {}: {}", bak_path.display(), e);
        return EXIT_ERROR;
    }

    let editor = args
        .editor_override
        .clone()
        .unwrap_or_else(|| std::env::var("EDITOR").unwrap_or_else(|_| "vi".into()));

    // First edit pass.
    if let Err(e) = spawn_editor(&editor, &path) {
        eprintln!("Editor failed: {}", e);
        let _ = restore_from_bak(&bak_path, &path);
        return EXIT_ERROR;
    }

    if validate_path(&path) {
        let _ = std::fs::remove_file(&bak_path);
        return EXIT_SUCCESS;
    }

    // First validation failed.
    eprintln!("Validation failed after edit. {}", bak_path.display());
    if args.non_interactive {
        eprintln!("Non-interactive mode — restoring from backup.");
        if let Err(e) = restore_from_bak(&bak_path, &path) {
            eprintln!("Failed to restore backup: {}", e);
            return EXIT_ERROR;
        }
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }

    if !prompt_reopen() {
        if let Err(e) = restore_from_bak(&bak_path, &path) {
            eprintln!("Failed to restore backup: {}", e);
            return EXIT_ERROR;
        }
        eprintln!("Restored {} from backup.", path.display());
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }

    // Second pass.
    if let Err(e) = spawn_editor(&editor, &path) {
        eprintln!("Editor failed: {}", e);
        let _ = restore_from_bak(&bak_path, &path);
        return EXIT_ERROR;
    }

    if validate_path(&path) {
        let _ = std::fs::remove_file(&bak_path);
        EXIT_SUCCESS
    } else {
        eprintln!(
            "Validation failed twice. Restoring {} from backup.",
            path.display()
        );
        if let Err(e) = restore_from_bak(&bak_path, &path) {
            eprintln!("Failed to restore backup: {}", e);
            return EXIT_ERROR;
        }
        EXIT_SCHEMA_OR_RESOLVE_ERROR
    }
}

fn resolve_target(target: Option<&str>) -> Result<PathBuf, String> {
    match target {
        None => Ok(PathBuf::from("sindri.yaml")),
        Some("policy") => Ok(PathBuf::from("sindri.policy.yaml")),
        Some(other) => Err(format!(
            "Unknown edit target '{}'. Valid: (none) | policy. Use `--schema` to print the schema path.",
            other
        )),
    }
}

fn print_schema_path(target: Option<&str>) -> i32 {
    let schema_name = match target {
        Some("policy") => "policy.json",
        _ => "bom.json",
    };
    // Repo-relative best-guess. We don't ship a packaged schema yet, so report
    // the canonical path inside the v4 source tree.
    let p = PathBuf::from("v4/schemas").join(schema_name);
    println!("{}", p.display());
    EXIT_SUCCESS
}

fn spawn_editor(editor: &str, path: &Path) -> std::io::Result<()> {
    // Tokenise the editor string so that `EDITOR="code -w"` works.
    let mut parts = editor.split_whitespace();
    let bin = parts.next().unwrap_or("vi");
    let args: Vec<&str> = parts.collect();
    let status = Command::new(bin).args(&args).arg(path).status()?;
    if !status.success() {
        return Err(std::io::Error::other(format!(
            "editor exited with status {}",
            status
        )));
    }
    Ok(())
}

fn validate_path(path: &Path) -> bool {
    let s = path.to_string_lossy();
    // Run the in-crate validator (silently, then print on failure).
    let code = crate::validate::run(&s, false);
    code == EXIT_SUCCESS
}

fn restore_from_bak(bak: &Path, dest: &Path) -> std::io::Result<()> {
    std::fs::copy(bak, dest)?;
    let _ = std::fs::remove_file(bak);
    Ok(())
}

fn prompt_reopen() -> bool {
    eprint!("Re-open editor? [Y/n] ");
    let _ = std::io::stderr().flush();
    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_err() {
        return false;
    }
    let trimmed = input.trim().to_lowercase();
    trimmed.is_empty() || trimmed == "y" || trimmed == "yes"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_valid_yaml() -> &'static str {
        "name: demo\ncomponents: []\n"
    }

    // Editor-spawning tests rely on POSIX shebang scripts + chmod +x,
    // which std::os::unix::fs::PermissionsExt provides only on Unix.
    // Skip on Windows; the pure-logic tests below still run there.
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    #[cfg(unix)]
    use tempfile::TempDir;

    #[cfg(unix)]
    fn write_fake_editor(dir: &Path, name: &str, body: &str) -> PathBuf {
        use std::io::Write;
        let path = dir.join(name);
        // Write + fsync + explicit drop. Without sync_all, Linux sometimes
        // returns ETXTBSY ("Text file busy") when we exec the script
        // immediately afterwards because the kernel still considers the
        // inode busy from the just-released write handle.  Keeping the
        // scope tight here ensures the file handle is fully closed before
        // we chmod / spawn.
        {
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(body.as_bytes()).unwrap();
            f.sync_all().unwrap();
        }
        let mut perms = std::fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms).unwrap();
        // Tiny defensive pause: even after sync_all + drop, the kernel can
        // briefly hold the inode in a state where exec(2) returns ETXTBSY.
        // 20 ms is well below test-suite noise and reliably eliminates the
        // race in CI parallel-execution environments.
        std::thread::sleep(std::time::Duration::from_millis(20));
        path
    }

    #[cfg(unix)]
    #[test]
    fn editor_exit_zero_runs_validate() {
        let tmp = TempDir::new().unwrap();
        let yaml_path = tmp.path().join("sindri.yaml");
        std::fs::write(&yaml_path, minimal_valid_yaml()).unwrap();

        // Fake editor: no-op (just exits 0).  Use stock `true` rather than
        // writing a shell script ourselves — that avoids the Linux ETXTBSY
        // race entirely (no freshly-written script to exec).  Bare name so
        // `Command::new` resolves via PATH (`/bin/true` on Linux,
        // `/usr/bin/true` on macOS).
        let code = run(EditArgs {
            target: None,
            schema: false,
            editor_override: Some("true".to_string()),
            non_interactive: true,
            path_override: Some(yaml_path.clone()),
        });

        assert_eq!(code, EXIT_SUCCESS);
        // .bak should have been removed on success.
        assert!(!tmp.path().join("sindri.yaml.bak").exists());
    }

    #[cfg(unix)]
    #[test]
    fn editor_exit_nonzero_restores_bak() {
        let tmp = TempDir::new().unwrap();
        let yaml_path = tmp.path().join("sindri.yaml");
        let original = minimal_valid_yaml();
        std::fs::write(&yaml_path, original).unwrap();

        // Fake editor that overwrites the file with content that fails to
        // deserialize as a BomManifest (a YAML scalar instead of a mapping).
        let editor = write_fake_editor(
            tmp.path(),
            "corrupt_editor.sh",
            "#!/bin/sh\nprintf 'just a scalar string\\n' > \"$1\"\nexit 0\n",
        );

        let code = run(EditArgs {
            target: None,
            schema: false,
            editor_override: Some(editor.to_string_lossy().into_owned()),
            non_interactive: true,
            path_override: Some(yaml_path.clone()),
        });

        assert_eq!(code, EXIT_SCHEMA_OR_RESOLVE_ERROR);
        // .bak should be removed after restore.
        assert!(!tmp.path().join("sindri.yaml.bak").exists());
        // File should be restored to original valid content.
        let restored = std::fs::read_to_string(&yaml_path).unwrap();
        assert_eq!(restored, original);
    }

    #[test]
    fn unknown_target_errors() {
        let res = resolve_target(Some("bogus"));
        assert!(res.is_err());
    }

    #[test]
    fn schema_target_resolves_to_policy() {
        let res = resolve_target(Some("policy")).unwrap();
        assert_eq!(res, PathBuf::from("sindri.policy.yaml"));
    }
}
