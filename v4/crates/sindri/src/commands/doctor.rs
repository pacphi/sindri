//! `sindri doctor` — health check + auto-fix engine (Sprint 12, Wave 4C + Wave 6F D13).
//!
//! `doctor` runs a typed registry of [`HealthCheck`]s. Each check has a
//! `run` function that returns a [`CheckResult`], and an optional `fix`
//! function that applies a remediation. With `--fix` the doctor will
//! invoke the remediation for any failing fixable check; with `--dry-run`
//! it will instead print what *would* be fixed without writing anything.
//!
//! # D13 — `--components` (Wave 6F)
//!
//! When `--components` is set, `doctor` reads the resolved lockfile
//! (`sindri.lock`, or `sindri.<target>.lock` when `--target` is supplied),
//! iterates the [`ResolvedComponent`]s, runs each component's embedded
//! [`ValidateConfig`] against a local target, and emits a per-component
//! pass/fail summary. Exit is non-zero if any component fails. Auto-fix
//! (`--fix`) is honoured for the standard path checks but does **not** extend
//! to component failures — `--fix` cannot re-install a broken tool.
//!
//! The local target used for component validation dispatches commands via
//! `std::process::Command` on the local machine, matching the behaviour of
//! `sindri apply --target local`.
//!
//! Adding a new check: append a [`HealthCheck`] entry to [`all_checks`].

use serde::Serialize;
use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};
use sindri_core::lockfile::{Lockfile, ResolvedComponent};
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
    /// Optional target name for target-specific prerequisites and lockfile
    /// selection (`sindri.<target>.lock`).
    pub target: Option<String>,
    /// Apply remediations.
    pub fix: bool,
    /// Print remediations without writing.
    pub dry_run: bool,
    /// JSON output.
    pub json: bool,
    /// Run per-component validate checks from the resolved lockfile (D13).
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

    // D13: --components runs the lockfile component validate pass, then
    // runs the standard path/shell-rc checks on top (always).
    let component_code = if args.components {
        let lockfile_path = if target_name == "local" {
            PathBuf::from("sindri.lock")
        } else {
            PathBuf::from(format!("sindri.{}.lock", target_name))
        };
        run_component_checks(&lockfile_path, target_name, args.json)
    } else {
        EXIT_SUCCESS
    };

    let checks = all_checks();
    let standard_code = run_with_context(
        &ctx,
        &checks,
        target_name,
        args.json,
        args.fix,
        args.dry_run,
    );

    // Non-zero if either the component checks or the standard checks failed.
    if component_code != EXIT_SUCCESS || standard_code != EXIT_SUCCESS {
        EXIT_SCHEMA_OR_RESOLVE_ERROR
    } else {
        EXIT_SUCCESS
    }
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

// ---- D13: component validate checks ------------------------------------

/// Per-component outcome emitted by `--components`.
#[derive(Debug, Serialize)]
struct ComponentCheckRecord {
    component: String,
    passed: bool,
    message: String,
}

/// Read the lockfile at `lockfile_path`, iterate components, run each
/// component's `validate` config against a local `std::process::Command`
/// dispatcher, and emit a summary. Returns EXIT_SUCCESS / EXIT_SCHEMA_OR_RESOLVE_ERROR.
///
/// When a component's lockfile entry has no embedded `manifest` (the resolver
/// does not yet fetch OCI manifests for all components), that component is
/// reported as "no validate config — skipped" and counted as passed, consistent
/// with the apply lifecycle behaviour (apply_lifecycle.rs step 4).
pub fn run_component_checks(lockfile_path: &Path, target_name: &str, json: bool) -> i32 {
    // Load lockfile.
    let text = match std::fs::read_to_string(lockfile_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!(
                "error: cannot read lockfile {}: {}",
                lockfile_path.display(),
                e
            );
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };
    let lockfile: Lockfile = match serde_yaml::from_str(&text) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("error: parse lockfile {}: {}", lockfile_path.display(), e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    let mut records: Vec<ComponentCheckRecord> = Vec::new();
    let mut any_failed = false;

    for comp in &lockfile.components {
        let record = validate_one_component(comp);
        if !record.passed {
            any_failed = true;
        }
        records.push(record);
    }

    if records.is_empty() {
        if !json {
            println!("sindri doctor --components: no components in lockfile");
        }
        return EXIT_SUCCESS;
    }

    if json {
        #[derive(Serialize)]
        struct Report<'a> {
            target: &'a str,
            lockfile: &'a str,
            components: Vec<ComponentCheckRecord>,
            passed: bool,
        }
        let report = Report {
            target: target_name,
            lockfile: &lockfile_path.display().to_string(),
            passed: !any_failed,
            components: records,
        };
        match serde_json::to_string_pretty(&report) {
            Ok(s) => println!("{}", s),
            Err(e) => {
                eprintln!("error: serialising component report: {}", e);
                return EXIT_SCHEMA_OR_RESOLVE_ERROR;
            }
        }
    } else {
        println!("sindri doctor --components — target: {}", target_name);
        println!();
        for r in &records {
            let status = if r.passed { "[OK]  " } else { "[FAIL]" };
            println!("  {} {} — {}", status, r.component, r.message);
        }
        println!();
        if any_failed {
            println!("Component validation found failures.");
        } else {
            println!("All components passed validation.");
        }
    }

    if any_failed {
        EXIT_SCHEMA_OR_RESOLVE_ERROR
    } else {
        EXIT_SUCCESS
    }
}

/// Run the validate config for a single [`ResolvedComponent`] using
/// `std::process::Command` on the local host. Returns a [`ComponentCheckRecord`].
fn validate_one_component(comp: &ResolvedComponent) -> ComponentCheckRecord {
    let name = comp.id.name.clone();

    let manifest = match &comp.manifest {
        Some(m) => m,
        None => {
            return ComponentCheckRecord {
                component: name,
                passed: true,
                message: "no manifest embedded — skipped".into(),
            };
        }
    };

    let validate_cfg = match &manifest.validate {
        Some(v) => v,
        None => {
            return ComponentCheckRecord {
                component: name,
                passed: true,
                message: "no validate config — skipped".into(),
            };
        }
    };

    for cmd in &validate_cfg.commands {
        match run_local_validate_command(&cmd.command) {
            Ok(stdout) => {
                // Check expected_output substring.
                if let Some(expected) = &cmd.expected_output {
                    if !stdout.contains(expected.as_str()) {
                        return ComponentCheckRecord {
                            component: name,
                            passed: false,
                            message: format!(
                                "`{}` stdout missing expected substring `{}`; got: {}",
                                cmd.command,
                                expected,
                                truncate_str(&stdout, 128)
                            ),
                        };
                    }
                }
                // Check version_match semver.
                if let Some(spec) = &cmd.version_match {
                    match check_version_match(&name, &cmd.command, spec, &stdout) {
                        Ok(()) => {}
                        Err(msg) => {
                            return ComponentCheckRecord {
                                component: name,
                                passed: false,
                                message: msg,
                            };
                        }
                    }
                }
            }
            Err(e) => {
                return ComponentCheckRecord {
                    component: name,
                    passed: false,
                    message: format!("`{}` failed: {}", cmd.command, e),
                };
            }
        }
    }

    ComponentCheckRecord {
        component: name,
        passed: true,
        message: "all validate commands passed".into(),
    }
}

/// Run a validate command locally and return stdout on success.
fn run_local_validate_command(command: &str) -> Result<String, String> {
    // Split naively on whitespace (same approach used by the existing
    // local target implementation in sindri-targets).
    let mut parts = command.split_whitespace();
    let program = match parts.next() {
        Some(p) => p,
        None => return Err("empty command".into()),
    };
    let args: Vec<&str> = parts.collect();
    let output = std::process::Command::new(program)
        .args(&args)
        .output()
        .map_err(|e| format!("spawn `{}`: {}", program, e))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(format!(
            "exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// Check a semver `version_match` spec against the stdout of a validate command.
fn check_version_match(
    component: &str,
    command: &str,
    spec: &str,
    stdout: &str,
) -> Result<(), String> {
    let req = semver::VersionReq::parse(spec).map_err(|e| {
        format!(
            "component `{}` validate `{}`: bad version spec `{}`: {}",
            component, command, spec, e
        )
    })?;
    let token = extract_semver_from_str(stdout).ok_or_else(|| {
        format!(
            "component `{}` validate `{}`: no semver token in stdout: {}",
            component,
            command,
            truncate_str(stdout, 128)
        )
    })?;
    let actual = semver::Version::parse(&token).map_err(|e| {
        format!(
            "component `{}` validate `{}`: invalid semver `{}`: {}",
            component, command, token, e
        )
    })?;
    if req.matches(&actual) {
        Ok(())
    } else {
        Err(format!(
            "component `{}` validate `{}`: version `{}` does not match `{}`",
            component, command, actual, spec
        ))
    }
}

/// Extract the first `MAJOR.MINOR.PATCH` token from a string. Mirrors the
/// logic in `sindri_extensions::validate` to keep behaviour consistent.
fn extract_semver_from_str(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let n = bytes.len();
    let mut i = 0;
    while i < n {
        let mut j = i;
        if bytes[j] == b'v' || bytes[j] == b'V' {
            j += 1;
        }
        if let Some((token, _end)) = match_semver_bytes(bytes, j) {
            if i == 0 || !bytes[i - 1].is_ascii_alphanumeric() {
                return Some(token);
            }
        }
        i += 1;
    }
    None
}

fn match_semver_bytes(bytes: &[u8], start: usize) -> Option<(String, usize)> {
    let n = bytes.len();
    let (a_start, a_end) = scan_ascii_digits(bytes, start)?;
    if a_end >= n || bytes[a_end] != b'.' {
        return None;
    }
    let (_, b_end) = scan_ascii_digits(bytes, a_end + 1)?;
    if b_end >= n || bytes[b_end] != b'.' {
        return None;
    }
    let (_, c_end) = scan_ascii_digits(bytes, b_end + 1)?;
    let token = std::str::from_utf8(&bytes[a_start..c_end])
        .ok()?
        .to_string();
    Some((token, c_end))
}

fn scan_ascii_digits(bytes: &[u8], start: usize) -> Option<(usize, usize)> {
    let n = bytes.len();
    if start >= n || !bytes[start].is_ascii_digit() {
        return None;
    }
    let mut end = start;
    while end < n && bytes[end].is_ascii_digit() {
        end += 1;
    }
    Some((start, end))
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
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

    // ---- D13: --components tests -------------------------------------------

    use sindri_core::component::{
        Backend, ComponentCapabilities, ComponentId, ComponentManifest, ComponentMetadata,
        InstallConfig, ValidateCommand, ValidateConfig,
    };
    use sindri_core::lockfile::{Lockfile, ResolvedComponent};
    use sindri_core::version::Version;
    use std::collections::HashMap;

    fn make_lockfile_with_component(
        component_name: &str,
        validate_cmds: Option<Vec<ValidateCommand>>,
    ) -> Lockfile {
        let manifest = ComponentManifest {
            metadata: ComponentMetadata {
                name: component_name.to_string(),
                version: "1.0.0".to_string(),
                description: "test".to_string(),
                license: "MIT".to_string(),
                tags: vec![],
                homepage: None,
            },
            platforms: vec![],
            install: InstallConfig::default(),
            depends_on: vec![],
            capabilities: ComponentCapabilities::default(),
            options: Default::default(),
            validate: validate_cmds.map(|cmds| ValidateConfig { commands: cmds }),
            configure: None,
            remove: None,
            overrides: HashMap::new(),
            auth: Default::default(),
        };
        let comp = ResolvedComponent {
            id: ComponentId {
                backend: Backend::Script,
                name: component_name.to_string(),
                qualifier: None,
            },
            version: Version::new("1.0.0"),
            backend: Backend::Script,
            oci_digest: None,
            checksums: HashMap::new(),
            depends_on: vec![],
            manifest: Some(manifest),
            manifest_digest: None,
            component_digest: None,
            platforms: None,
        };
        Lockfile {
            version: 1,
            bom_hash: "abc".to_string(),
            target: "local".to_string(),
            components: vec![comp],
            auth_bindings: Vec::new(),
        }
    }

    fn write_lockfile(dir: &TempDir, lockfile: &Lockfile) -> PathBuf {
        let p = dir.path().join("sindri.lock");
        let text = serde_yaml::to_string(lockfile).unwrap();
        std::fs::write(&p, text).unwrap();
        p
    }

    #[test]
    fn components_no_manifest_skips_gracefully() {
        // Component with no embedded manifest → should pass (skip).
        let comp = ResolvedComponent {
            id: ComponentId {
                backend: Backend::Script,
                name: "no-manifest".to_string(),
                qualifier: None,
            },
            version: Version::new("1.0.0"),
            backend: Backend::Script,
            oci_digest: None,
            checksums: HashMap::new(),
            depends_on: vec![],
            manifest: None,
            manifest_digest: None,
            component_digest: None,
            platforms: None,
        };
        let lf = Lockfile {
            version: 1,
            bom_hash: "x".to_string(),
            target: "local".to_string(),
            components: vec![comp],
            auth_bindings: Vec::new(),
        };
        let dir = TempDir::new().unwrap();
        let path = write_lockfile(&dir, &lf);
        let code = run_component_checks(&path, "local", false);
        assert_eq!(code, EXIT_SUCCESS, "no manifest → skip → pass");
    }

    #[test]
    fn components_passing_validate_command() {
        // Use `echo` as a stub validator — always exits 0 and outputs something.
        let lf = make_lockfile_with_component(
            "echo-tool",
            Some(vec![ValidateCommand {
                command: "echo hello".to_string(),
                expected_output: Some("hello".to_string()),
                version_match: None,
            }]),
        );
        let dir = TempDir::new().unwrap();
        let path = write_lockfile(&dir, &lf);
        let code = run_component_checks(&path, "local", false);
        assert_eq!(code, EXIT_SUCCESS, "echo passes substring check");
    }

    #[test]
    fn components_failing_expected_output() {
        // Command exits 0 but stdout doesn't contain the expected substring.
        let lf = make_lockfile_with_component(
            "bad-output",
            Some(vec![ValidateCommand {
                command: "echo hello".to_string(),
                expected_output: Some("NOPE_NOT_THERE".to_string()),
                version_match: None,
            }]),
        );
        let dir = TempDir::new().unwrap();
        let path = write_lockfile(&dir, &lf);
        let code = run_component_checks(&path, "local", false);
        assert_eq!(
            code, EXIT_SCHEMA_OR_RESOLVE_ERROR,
            "missing substring → fail"
        );
    }

    #[test]
    fn components_failing_nonzero_exit() {
        // Command that always exits non-zero.
        let lf = make_lockfile_with_component(
            "broken-tool",
            Some(vec![ValidateCommand {
                // `false` (or `exit 1`) exits non-zero on all platforms.
                command: "sh -c 'exit 1'".to_string(),
                expected_output: None,
                version_match: None,
            }]),
        );
        let dir = TempDir::new().unwrap();
        let path = write_lockfile(&dir, &lf);
        let code = run_component_checks(&path, "local", false);
        assert_eq!(code, EXIT_SCHEMA_OR_RESOLVE_ERROR, "non-zero exit → fail");
    }

    #[test]
    fn components_empty_lockfile_passes() {
        let lf = Lockfile {
            version: 1,
            bom_hash: "x".to_string(),
            target: "local".to_string(),
            components: vec![],
            auth_bindings: Vec::new(),
        };
        let dir = TempDir::new().unwrap();
        let path = write_lockfile(&dir, &lf);
        let code = run_component_checks(&path, "local", false);
        assert_eq!(
            code, EXIT_SUCCESS,
            "empty lockfile → pass (nothing to fail)"
        );
    }

    #[test]
    fn components_missing_lockfile_returns_error() {
        let dir = TempDir::new().unwrap();
        let missing = dir.path().join("nonexistent.lock");
        let code = run_component_checks(&missing, "local", false);
        assert_eq!(code, EXIT_SCHEMA_OR_RESOLVE_ERROR);
    }
}
