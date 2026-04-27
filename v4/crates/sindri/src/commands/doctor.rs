//! `sindri doctor` — health check + auto-fix engine (Sprint 12, Wave 4C).
//!
//! `doctor` runs a typed registry of [`HealthCheck`]s. Each check has a
//! `run` function that returns a [`CheckResult`], and an optional `fix`
//! function that applies a remediation. With `--fix` the doctor will
//! invoke the remediation for any failing fixable check; with `--dry-run`
//! it will instead print what *would* be fixed without writing anything.
//!
//! Adding a new check: append a [`HealthCheck`] entry to [`all_checks`].

use serde::Serialize;
use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};
use std::path::{Path, PathBuf};

/// Marker placed in `~/.bashrc` / `~/.zshrc` so doctor's PATH guard block
/// is appended at most once even across re-runs. Mirrors the pattern in
/// `sindri_extensions::configure` (PR #215).
const DOCTOR_PATH_MARKER: &str = "# sindri:auto path";

/// Coarse grouping for human-readable output and `--json` filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HealthCategory {
    /// Filesystem paths under `~/.sindri/`.
    Paths,
    /// Shell rc-file content.
    ShellRc,
    /// Registry cache + trust state.
    Registry,
    /// Active install policy.
    Policy,
    /// Backend tools (jq, mise, gh, …).
    Component,
}

/// CLI args accepted by `sindri doctor`.
pub struct DoctorArgs {
    /// Optional target name for target-specific prerequisites.
    pub target: Option<String>,
    /// Apply remediations.
    pub fix: bool,
    /// Print remediations without writing.
    pub dry_run: bool,
    /// JSON output.
    pub json: bool,
    /// Reserved — component health check (Sprint 12.2 backlog).
    pub components: bool,
}

/// Context passed to every check. Tests construct one with a
/// [`tempfile::TempDir`] root so `$HOME` writes are sandboxed.
#[derive(Debug, Clone)]
pub struct DoctorContext {
    /// Effective `$HOME`. Production: `dirs_next::home_dir()`.
    pub home_dir: PathBuf,
    /// Whether mutating fixes are allowed.
    pub apply_fixes: bool,
    /// Whether to print "Would: …" instead of mutating.
    pub dry_run: bool,
}

impl DoctorContext {
    /// `~/.sindri/`.
    pub fn sindri_dir(&self) -> PathBuf {
        self.home_dir.join(".sindri")
    }
    /// `~/.sindri/trust/`.
    pub fn trust_dir(&self) -> PathBuf {
        self.sindri_dir().join("trust")
    }
    /// `~/.sindri/cache/registries/`.
    pub fn registry_cache_dir(&self) -> PathBuf {
        self.sindri_dir().join("cache").join("registries")
    }
}

/// Outcome of a single non-fix run.
#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    pub passed: bool,
    pub message: String,
    /// Human-readable suggestion for the user (printed when no `fix` is
    /// available, or when running without `--fix`).
    pub suggested_fix: Option<String>,
}

/// Outcome of an attempted remediation.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "outcome", rename_all = "kebab-case")]
pub enum FixOutcome {
    /// Action was performed.
    Fixed { detail: String },
    /// Already in the desired state — no-op.
    AlreadyFixed,
    /// Check has no remediation function (suggestion only).
    NotApplicable,
}

/// Errors a fix function may raise.
#[derive(Debug, thiserror::Error)]
pub enum DoctorError {
    /// I/O failure during remediation.
    #[error("doctor io error: {0}")]
    Io(#[from] std::io::Error),
    /// Generic remediation failure with a human message.
    #[error("doctor fix failed: {0}")]
    FixFailed(String),
}

/// Function pointer signature for the read-only `run` step of a check.
pub type CheckFn = fn(&DoctorContext) -> CheckResult;

/// Function pointer signature for the optional `fix` step of a check.
pub type FixFn = fn(&DoctorContext) -> Result<FixOutcome, DoctorError>;

/// A single health check. `run` is mandatory; `fix` is optional.
pub struct HealthCheck {
    /// Stable, human-readable identifier (`paths.sindri-dir`).
    pub name: &'static str,
    /// Coarse grouping.
    pub category: HealthCategory,
    /// Read-only check.
    pub run: CheckFn,
    /// Optional remediation.
    pub fix: Option<FixFn>,
}

/// Per-check structured record for `--json`.
#[derive(Debug, Serialize)]
struct CheckRecord<'a> {
    name: &'a str,
    category: HealthCategory,
    passed: bool,
    message: String,
    suggested_fix: Option<String>,
    fixable: bool,
    /// Only populated when `--fix` runs.
    #[serde(skip_serializing_if = "Option::is_none")]
    fix_outcome: Option<FixOutcome>,
}

/// Top-level JSON envelope.
#[derive(Debug, Serialize)]
struct DoctorReport<'a> {
    target: &'a str,
    fix: bool,
    dry_run: bool,
    checks: Vec<CheckRecord<'a>>,
    passed: bool,
}

/// Entry point used by `main`.
pub fn run(args: DoctorArgs) -> i32 {
    if args.fix && args.dry_run {
        eprintln!("error: --fix and --dry-run are mutually exclusive");
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }

    let home_dir = dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("/"));
    let ctx = DoctorContext {
        home_dir,
        apply_fixes: args.fix,
        dry_run: args.dry_run,
    };
    let target_name = args.target.as_deref().unwrap_or("local");
    let checks = all_checks();
    run_with_context(
        &ctx,
        &checks,
        target_name,
        args.json,
        args.fix,
        args.dry_run,
    )
}

/// Test-friendly entry point: caller supplies `DoctorContext` and the
/// list of checks (so tests can scope to a single check).
pub fn run_with_context(
    ctx: &DoctorContext,
    checks: &[HealthCheck],
    target_name: &str,
    json: bool,
    apply_fix: bool,
    dry_run: bool,
) -> i32 {
    let mut records: Vec<CheckRecord<'_>> = Vec::with_capacity(checks.len());
    let mut any_failed_unfixed = false;

    for check in checks {
        let res = (check.run)(ctx);
        let fixable = check.fix.is_some();
        let mut record = CheckRecord {
            name: check.name,
            category: check.category,
            passed: res.passed,
            message: res.message.clone(),
            suggested_fix: res.suggested_fix.clone(),
            fixable,
            fix_outcome: None,
        };

        if !res.passed {
            if dry_run && fixable {
                let action = res
                    .suggested_fix
                    .clone()
                    .unwrap_or_else(|| format!("apply remediation for `{}`", check.name));
                if !json {
                    println!("Would: {}", action);
                }
                // dry-run does not mutate; leave passed=false so exit reflects work outstanding
                any_failed_unfixed = true;
            } else if apply_fix {
                if let Some(fix_fn) = check.fix {
                    match fix_fn(ctx) {
                        Ok(outcome) => {
                            record.fix_outcome = Some(outcome.clone());
                            // Mark as passed if we actually fixed or it was already fixed.
                            match outcome {
                                FixOutcome::Fixed { .. } | FixOutcome::AlreadyFixed => {
                                    record.passed = true;
                                }
                                FixOutcome::NotApplicable => {
                                    any_failed_unfixed = true;
                                }
                            }
                        }
                        Err(e) => {
                            record.fix_outcome = Some(FixOutcome::Fixed {
                                detail: format!("error: {}", e),
                            });
                            // record.passed stays false
                            any_failed_unfixed = true;
                        }
                    }
                } else {
                    any_failed_unfixed = true;
                }
            } else {
                any_failed_unfixed = true;
            }
        }
        records.push(record);
    }

    if json {
        let report = DoctorReport {
            target: target_name,
            fix: apply_fix,
            dry_run,
            passed: !any_failed_unfixed,
            checks: records,
        };
        match serde_json::to_string_pretty(&report) {
            Ok(s) => println!("{}", s),
            Err(e) => {
                eprintln!("error: serialising doctor report: {}", e);
                return EXIT_SCHEMA_OR_RESOLVE_ERROR;
            }
        }
    } else {
        println!("sindri doctor — target: {}", target_name);
        println!();
        for r in &records {
            let status = if r.passed { "[OK]  " } else { "[FAIL]" };
            println!("  {} {} — {}", status, r.name, r.message);
            if !r.passed {
                match &r.fix_outcome {
                    Some(FixOutcome::Fixed { detail }) => {
                        println!("        Fixed: {}", detail);
                    }
                    Some(FixOutcome::AlreadyFixed) => {
                        println!("        Already fixed.");
                    }
                    Some(FixOutcome::NotApplicable) => {
                        if let Some(s) = &r.suggested_fix {
                            println!("        Fix: {}", s);
                        }
                    }
                    None => {
                        if let Some(s) = &r.suggested_fix {
                            println!("        Fix: {}", s);
                        }
                    }
                }
            }
        }
        println!();
        if any_failed_unfixed {
            println!("Doctor found issues.");
        } else {
            println!("All checks passed.");
        }
    }

    if any_failed_unfixed {
        EXIT_SCHEMA_OR_RESOLVE_ERROR
    } else {
        EXIT_SUCCESS
    }
}

/// Master list of checks. Order is presentation order.
pub fn all_checks() -> Vec<HealthCheck> {
    vec![
        HealthCheck {
            name: "paths.sindri-dir",
            category: HealthCategory::Paths,
            run: check_sindri_dir,
            fix: Some(fix_sindri_dir),
        },
        HealthCheck {
            name: "paths.trust-dir",
            category: HealthCategory::Paths,
            run: check_trust_dir,
            fix: Some(fix_trust_dir),
        },
        HealthCheck {
            name: "paths.registry-cache-dir",
            category: HealthCategory::Paths,
            run: check_registry_cache_dir,
            fix: Some(fix_registry_cache_dir),
        },
        HealthCheck {
            name: "shell-rc.cargo-bin-on-path",
            category: HealthCategory::ShellRc,
            run: check_cargo_bin_on_path,
            fix: Some(fix_cargo_bin_on_path),
        },
        HealthCheck {
            name: "registry.lockfile-fresh",
            category: HealthCategory::Registry,
            run: check_lockfile_fresh,
            // Suggestion only — never auto-resolve.
            fix: None,
        },
    ]
}

// ---- individual checks --------------------------------------------------

fn check_sindri_dir(ctx: &DoctorContext) -> CheckResult {
    let p = ctx.sindri_dir();
    if p.is_dir() {
        CheckResult {
            passed: true,
            message: format!("{} exists", p.display()),
            suggested_fix: None,
        }
    } else {
        CheckResult {
            passed: false,
            message: format!("{} missing", p.display()),
            suggested_fix: Some(format!("mkdir -p {}", p.display())),
        }
    }
}

fn fix_sindri_dir(ctx: &DoctorContext) -> Result<FixOutcome, DoctorError> {
    let p = ctx.sindri_dir();
    if p.is_dir() {
        return Ok(FixOutcome::AlreadyFixed);
    }
    std::fs::create_dir_all(&p)?;
    Ok(FixOutcome::Fixed {
        detail: format!("created {}", p.display()),
    })
}

fn check_trust_dir(ctx: &DoctorContext) -> CheckResult {
    let p = ctx.trust_dir();
    if p.is_dir() {
        CheckResult {
            passed: true,
            message: format!("{} exists", p.display()),
            suggested_fix: None,
        }
    } else {
        CheckResult {
            passed: false,
            message: format!("{} missing", p.display()),
            suggested_fix: Some(format!("mkdir -p {}", p.display())),
        }
    }
}

fn fix_trust_dir(ctx: &DoctorContext) -> Result<FixOutcome, DoctorError> {
    let p = ctx.trust_dir();
    if p.is_dir() {
        return Ok(FixOutcome::AlreadyFixed);
    }
    std::fs::create_dir_all(&p)?;
    Ok(FixOutcome::Fixed {
        detail: format!("created {}", p.display()),
    })
}

fn check_registry_cache_dir(ctx: &DoctorContext) -> CheckResult {
    let p = ctx.registry_cache_dir();
    if p.is_dir() {
        CheckResult {
            passed: true,
            message: format!("{} exists", p.display()),
            suggested_fix: None,
        }
    } else {
        CheckResult {
            passed: false,
            message: format!("{} missing", p.display()),
            suggested_fix: Some(format!("mkdir -p {}", p.display())),
        }
    }
}

fn fix_registry_cache_dir(ctx: &DoctorContext) -> Result<FixOutcome, DoctorError> {
    let p = ctx.registry_cache_dir();
    if p.is_dir() {
        return Ok(FixOutcome::AlreadyFixed);
    }
    std::fs::create_dir_all(&p)?;
    Ok(FixOutcome::Fixed {
        detail: format!("created {}", p.display()),
    })
}

fn check_cargo_bin_on_path(ctx: &DoctorContext) -> CheckResult {
    let cargo_bin = ctx.home_dir.join(".cargo").join("bin");
    let path = std::env::var("PATH").unwrap_or_default();
    let on_path = std::env::split_paths(&path).any(|p| p == cargo_bin);
    let bashrc_ok = rc_has_marker(&ctx.home_dir.join(".bashrc"));
    let zshrc_ok = rc_has_marker(&ctx.home_dir.join(".zshrc"));
    if on_path || (bashrc_ok && zshrc_ok) {
        CheckResult {
            passed: true,
            message: format!(
                "{} on PATH (or guarded block already in shell rc)",
                cargo_bin.display()
            ),
            suggested_fix: None,
        }
    } else {
        CheckResult {
            passed: false,
            message: format!("{} not on PATH", cargo_bin.display()),
            suggested_fix: Some(format!(
                "append `# sindri:auto path` block to ~/.bashrc and ~/.zshrc adding {} to PATH",
                cargo_bin.display()
            )),
        }
    }
}

fn fix_cargo_bin_on_path(ctx: &DoctorContext) -> Result<FixOutcome, DoctorError> {
    let cargo_bin = ctx.home_dir.join(".cargo").join("bin");
    let mut wrote_any = false;
    let mut already_any = false;
    for rc_name in &[".bashrc", ".zshrc"] {
        let rc_path = ctx.home_dir.join(rc_name);
        match ensure_path_block(&rc_path, &cargo_bin)? {
            FixOutcome::Fixed { .. } => wrote_any = true,
            FixOutcome::AlreadyFixed => already_any = true,
            FixOutcome::NotApplicable => {}
        }
    }
    if wrote_any {
        Ok(FixOutcome::Fixed {
            detail: format!(
                "appended PATH guard to bashrc/zshrc for {}",
                cargo_bin.display()
            ),
        })
    } else if already_any {
        Ok(FixOutcome::AlreadyFixed)
    } else {
        Ok(FixOutcome::NotApplicable)
    }
}

fn check_lockfile_fresh(_ctx: &DoctorContext) -> CheckResult {
    let lockfile = std::path::Path::new("sindri.lock");
    let manifest = std::path::Path::new("sindri.yaml");
    if !lockfile.exists() {
        // No lockfile → not stale, just not yet resolved. Treat as informational pass.
        return CheckResult {
            passed: true,
            message: "no sindri.lock found (run `sindri resolve` to create one)".into(),
            suggested_fix: None,
        };
    }
    if !manifest.exists() {
        return CheckResult {
            passed: true,
            message: "sindri.lock present, no sindri.yaml to compare".into(),
            suggested_fix: None,
        };
    }
    // We can't recompute the bom_hash without the full resolver pipeline here,
    // so we use a coarser signal: lockfile mtime vs manifest mtime.
    let mtime = |p: &Path| -> Option<std::time::SystemTime> {
        std::fs::metadata(p).ok().and_then(|m| m.modified().ok())
    };
    let stale = match (mtime(manifest), mtime(lockfile)) {
        (Some(m), Some(l)) => m > l,
        _ => false,
    };
    if stale {
        CheckResult {
            passed: false,
            message: "sindri.yaml is newer than sindri.lock".into(),
            suggested_fix: Some("Run `sindri resolve`".into()),
        }
    } else {
        CheckResult {
            passed: true,
            message: "sindri.lock is up-to-date with sindri.yaml".into(),
            suggested_fix: None,
        }
    }
}

// ---- shell-rc helpers ---------------------------------------------------

fn rc_has_marker(rc_path: &Path) -> bool {
    std::fs::read_to_string(rc_path)
        .map(|s| s.contains(DOCTOR_PATH_MARKER))
        .unwrap_or(false)
}

/// Append a guarded block to `rc_path` that prepends `bin_dir` to `$PATH`.
/// Idempotent: the marker line is checked first.
fn ensure_path_block(rc_path: &Path, bin_dir: &Path) -> Result<FixOutcome, DoctorError> {
    let existing = std::fs::read_to_string(rc_path).unwrap_or_default();
    if existing.contains(DOCTOR_PATH_MARKER) {
        return Ok(FixOutcome::AlreadyFixed);
    }
    if let Some(parent) = rc_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let block = format!(
        "\n{marker}\n\
         # Sindri-managed PATH addition. Edit via `sindri doctor --fix`; do not modify by hand.\n\
         case \":$PATH:\" in\n\
         \t*\":{bin}:\"*) ;;\n\
         \t*) export PATH=\"{bin}:$PATH\" ;;\n\
         esac\n\
         {marker}\n",
        marker = DOCTOR_PATH_MARKER,
        bin = bin_dir.display(),
    );
    let mut body = existing;
    body.push_str(&block);
    std::fs::write(rc_path, body)?;
    Ok(FixOutcome::Fixed {
        detail: format!("wrote PATH guard to {}", rc_path.display()),
    })
}

// ---- tests --------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn ctx_for(home: &TempDir, apply_fixes: bool, dry_run: bool) -> DoctorContext {
        DoctorContext {
            home_dir: home.path().to_path_buf(),
            apply_fixes,
            dry_run,
        }
    }

    #[test]
    fn missing_sindri_dir_check_detects_and_fixes() {
        let home = TempDir::new().unwrap();
        let ctx = ctx_for(&home, true, false);

        // Detect.
        let res = check_sindri_dir(&ctx);
        assert!(!res.passed, "expected sindri dir to be missing");
        assert!(res.suggested_fix.is_some());

        // Fix.
        let outcome = fix_sindri_dir(&ctx).unwrap();
        match outcome {
            FixOutcome::Fixed { .. } => {}
            other => panic!("expected Fixed, got {:?}", other),
        }
        assert!(ctx.sindri_dir().is_dir());

        // Re-run check is now passing.
        let res2 = check_sindri_dir(&ctx);
        assert!(res2.passed);
    }

    #[test]
    fn dry_run_does_not_modify_filesystem() {
        let home = TempDir::new().unwrap();
        let ctx = ctx_for(&home, false, true);
        let checks = all_checks();
        let code = run_with_context(&ctx, &checks, "local", false, false, true);
        // Dry-run keeps failures unfixed → non-zero.
        assert_eq!(code, EXIT_SCHEMA_OR_RESOLVE_ERROR);
        // No files created.
        assert!(!ctx.sindri_dir().exists());
        assert!(!ctx.trust_dir().exists());
        assert!(!ctx.registry_cache_dir().exists());
        assert!(!home.path().join(".bashrc").exists());
        assert!(!home.path().join(".zshrc").exists());
    }

    #[test]
    fn fix_is_idempotent() {
        let home = TempDir::new().unwrap();
        let ctx = ctx_for(&home, true, false);
        let first = fix_sindri_dir(&ctx).unwrap();
        assert!(matches!(first, FixOutcome::Fixed { .. }));
        let second = fix_sindri_dir(&ctx).unwrap();
        assert!(matches!(second, FixOutcome::AlreadyFixed));
    }

    #[test]
    fn shell_rc_fix_appends_guarded_block_only_once() {
        let home = TempDir::new().unwrap();
        let ctx = ctx_for(&home, true, false);

        let first = fix_cargo_bin_on_path(&ctx).unwrap();
        assert!(matches!(first, FixOutcome::Fixed { .. }));
        let bashrc = std::fs::read_to_string(home.path().join(".bashrc")).unwrap();
        let zshrc = std::fs::read_to_string(home.path().join(".zshrc")).unwrap();
        assert_eq!(
            bashrc.matches(DOCTOR_PATH_MARKER).count(),
            2,
            "open + close marker"
        );
        assert_eq!(zshrc.matches(DOCTOR_PATH_MARKER).count(), 2);

        let second = fix_cargo_bin_on_path(&ctx).unwrap();
        assert!(matches!(second, FixOutcome::AlreadyFixed));
        let bashrc2 = std::fs::read_to_string(home.path().join(".bashrc")).unwrap();
        assert_eq!(bashrc, bashrc2, "second fix must not touch the file");
    }

    #[test]
    fn stale_lockfile_emits_suggestion_only() {
        // Run inside a tempdir CWD so we don't accidentally pick up the
        // workspace's real sindri.yaml/sindri.lock.
        let cwd_keep = std::env::current_dir().unwrap();
        let work = TempDir::new().unwrap();
        std::env::set_current_dir(work.path()).unwrap();

        // Create sindri.yaml newer than sindri.lock.
        std::fs::write(work.path().join("sindri.lock"), "old").unwrap();
        // Sleep an instant so mtimes differ on coarse-grained filesystems.
        std::thread::sleep(std::time::Duration::from_millis(50));
        std::fs::write(work.path().join("sindri.yaml"), "newer").unwrap();

        let home = TempDir::new().unwrap();
        let ctx = ctx_for(&home, false, false);
        let res = check_lockfile_fresh(&ctx);

        // Restore CWD before any panics.
        std::env::set_current_dir(cwd_keep).unwrap();

        assert!(!res.passed, "expected stale lockfile fail");
        assert_eq!(res.suggested_fix.as_deref(), Some("Run `sindri resolve`"));
        // No `fix` function attached → suggestion-only.
        let entry = all_checks()
            .into_iter()
            .find(|c| c.name == "registry.lockfile-fresh")
            .unwrap();
        assert!(entry.fix.is_none());
    }
}
