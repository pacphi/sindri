//! Lifecycle hook dispatcher (ADR-030 + `v4/docs/script-contract.md`).
//!
//! [`HooksExecutor::run_phase`] runs the platform-appropriate variant
//! (POSIX shell on Linux/macOS, PowerShell on Windows) of a phase
//! script through the active [`Target`].
//!
//! ## Contract surface
//!
//! Every phase script receives:
//!
//! - **argv:** `[<phase>, <target_version>, <prior_version>]` —
//!   `<prior_version>` is the empty string on a fresh install.
//! - **env:**
//!     - `SINDRI_PHASE` — kebab-case phase token (mirrors argv\[1\]).
//!     - `SINDRI_COMPONENT_ADDRESS` — `backend:name[@qualifier]`.
//!     - `SINDRI_COMPONENT_VERSION` — target version.
//!     - `SINDRI_PRIOR_VERSION` — prior version, or empty.
//!     - `SINDRI_TARGET` — target name (e.g. `local`).
//!     - `SINDRI_LOG_DIR` — absolute path to per-phase log directory.
//!     - `SINDRI_EVENTS` — absolute path to a writable JSON-Lines file
//!       the script appends structured events to.
//!     - `SINDRI_DRY_RUN` — `1` when `apply --dry-run`; otherwise unset.
//!     - any auth-injected `SINDRI_AUTH_<id>` values redeemed by the
//!       caller before invoking the dispatcher.
//!
//! Stdout/stderr are user-facing logs — not a structured protocol.
//! All structured events go through the `$SINDRI_EVENTS` JSON-Lines
//! file, which the dispatcher creates before exec and parses after.
//!
//! Exit codes are binary: 0 = success; non-zero = failure (mapped to
//! [`ExtensionError::HookFailed`]). Skip / continue intentions are
//! conveyed via JSON events, e.g. `{"event":"skip","reason":"…"}`.

use crate::error::ExtensionError;
use serde::{Deserialize, Serialize};
use sindri_core::component::{HooksConfig, Phase, ScriptRef};
use sindri_core::platform::{Os, Platform};
use sindri_targets::Target;
use std::path::{Path, PathBuf};

/// Context passed to every hook invocation.
///
/// The lifetime parameter ties the borrowed [`Target`] reference to the
/// caller's stack frame so that `HooksExecutor` does not own the target.
pub struct HookContext<'a> {
    /// Component address (e.g. `npm:claude-code`). Carried into
    /// `SINDRI_COMPONENT_ADDRESS`.
    pub component: &'a str,
    /// Target component version (carried into argv\[2\] +
    /// `SINDRI_COMPONENT_VERSION`).
    pub version: &'a str,
    /// Prior installed version (or empty string on fresh install).
    /// Carried into argv\[3\] + `SINDRI_PRIOR_VERSION`.
    pub prior_version: &'a str,
    /// The execution target.
    pub target: &'a dyn Target,
    /// Caller-supplied env vars layered on top of the contracted set.
    /// Auth values redeemed by `Redeemer` arrive here.
    pub env: &'a [(&'a str, &'a str)],
    /// Component package root — the directory containing
    /// `component.yaml`. Phase script paths are resolved relative to
    /// this. On the local target this is the OCI cache extract dir.
    pub package_root: &'a Path,
    /// Per-component log directory. Phase logs / events files land
    /// under `<log_dir>/<phase>.{log,events.jsonl}`.
    pub log_dir: &'a Path,
    /// `--dry-run` flag from `sindri apply` (mirrors caller intent).
    pub dry_run: bool,
}

/// One phase invocation's outcome — events parsed from
/// `$SINDRI_EVENTS`, plus the dispatcher's binary verdict.
#[derive(Debug, Clone, Default)]
pub struct PhaseOutcome {
    /// Phase events emitted by the script (parsed JSON-Lines from
    /// `$SINDRI_EVENTS`). Includes the `phase-complete` event when
    /// the script honored the contract.
    pub events: Vec<HookEvent>,
    /// True iff the script wrote at least one
    /// `{"event":"phase-complete", "change": …}` line. Components
    /// that don't emit it are recorded but the dispatcher does not
    /// fail — strictness is a Phase 6.5 lift.
    pub completed: bool,
    /// Whether the script reported it changed system state.
    /// `Some(true)` for an idempotency-aware "did work" run; `Some(false)`
    /// for a no-op; `None` if the contract was not honored.
    pub changed: Option<bool>,
}

/// One JSON-Lines event read from a phase script's `$SINDRI_EVENTS`
/// file. Free-form `event` discriminator + arbitrary `detail` payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookEvent {
    pub event: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change: Option<bool>,
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub detail: serde_json::Value,
}

/// Dispatcher for `capabilities.hooks.<phase>` scripts.
#[derive(Debug, Default, Clone, Copy)]
pub struct HooksExecutor;

impl HooksExecutor {
    pub fn new() -> Self {
        Self
    }

    /// Run the named phase if a [`ScriptRef`] is declared for it. No-op
    /// (returns an empty outcome) when the manifest is silent on the
    /// phase. Errors when the script exists but the contract gate
    /// (file present + executable) or the exec itself fails.
    pub async fn run_phase(
        &self,
        phase: Phase,
        hooks: &HooksConfig,
        ctx: &HookContext<'_>,
    ) -> Result<PhaseOutcome, ExtensionError> {
        let Some(script_ref) = hooks.for_phase(phase) else {
            tracing::debug!(
                component = ctx.component,
                phase = phase.as_str(),
                "no hook declared; skipping"
            );
            return Ok(PhaseOutcome::default());
        };

        let platform = match ctx.target.profile() {
            Ok(p) => p.platform,
            // Fall back to the current host platform for targets whose
            // profile() isn't yet implemented; the script-backend
            // refuses non-local execution upstream so this is safe.
            Err(_) => Platform::current(),
        };

        let os = platform.os.clone();
        let script_relative =
            pick_variant(script_ref, &os).ok_or_else(|| ExtensionError::HookFailed {
                component: ctx.component.to_string(),
                command: format!("{} hook for {:?}", phase.as_str(), os),
                detail:
                    "no script variant available for this OS (need .sh on Unix or .ps1 on Windows)"
                        .into(),
            })?;
        let script_abs = ctx.package_root.join(script_relative);

        // Contract gate: file present, executable on POSIX, non-empty.
        validate_script_file(&script_abs, &os).map_err(|detail| ExtensionError::HookFailed {
            component: ctx.component.to_string(),
            command: script_abs.display().to_string(),
            detail,
        })?;

        // Per-phase log + events files. Created upfront so the
        // dispatcher can read them after exit even if the script crashed.
        std::fs::create_dir_all(ctx.log_dir).ok();
        let events_path = ctx.log_dir.join(format!("{}.events.jsonl", phase.as_str()));
        let _ = std::fs::File::create(&events_path);

        // Build env: contracted + caller-supplied. Caller-supplied wins
        // on collision (auth tokens may legitimately use the same name
        // as a contracted var, though this is rare).
        let dry = if ctx.dry_run { "1" } else { "0" };
        let log_dir_str = ctx.log_dir.display().to_string();
        let events_str = events_path.display().to_string();
        let mut env: Vec<(String, String)> = vec![
            ("SINDRI_PHASE".into(), phase.as_str().into()),
            ("SINDRI_COMPONENT_ADDRESS".into(), ctx.component.into()),
            ("SINDRI_COMPONENT_VERSION".into(), ctx.version.into()),
            ("SINDRI_PRIOR_VERSION".into(), ctx.prior_version.into()),
            ("SINDRI_TARGET".into(), ctx.target.name().into()),
            ("SINDRI_LOG_DIR".into(), log_dir_str),
            ("SINDRI_EVENTS".into(), events_str.clone()),
            ("SINDRI_DRY_RUN".into(), dry.into()),
        ];
        for (k, v) in ctx.env {
            env.push(((*k).to_string(), (*v).to_string()));
        }
        let env_borrowed: Vec<(&str, &str)> =
            env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();

        // Build the exec command. POSIX: `bash <script> <phase> <ver>
        // <prior>`. Windows: `pwsh -NonInteractive -File <script>
        // <phase> <ver> <prior>`.
        let cmd = build_command(&script_abs, phase, ctx.version, ctx.prior_version, &os);

        tracing::info!(
            component = ctx.component,
            version = ctx.version,
            phase = phase.as_str(),
            command = cmd.as_str(),
            "running lifecycle hook"
        );

        match ctx.target.exec(&cmd, &env_borrowed) {
            Ok((stdout, stderr)) => {
                tracing::debug!(
                    component = ctx.component,
                    phase = phase.as_str(),
                    stdout_bytes = stdout.len(),
                    stderr_bytes = stderr.len(),
                    "hook completed"
                );
                // Persist captured streams for post-mortem.
                let _ = std::fs::write(
                    ctx.log_dir.join(format!("{}.stdout", phase.as_str())),
                    stdout,
                );
                let _ = std::fs::write(
                    ctx.log_dir.join(format!("{}.stderr", phase.as_str())),
                    stderr,
                );

                let outcome = parse_events_file(&events_path);
                Ok(outcome)
            }
            Err(err) => Err(ExtensionError::HookFailed {
                component: ctx.component.to_string(),
                command: cmd,
                detail: err.to_string(),
            }),
        }
    }
}

/// Pick the platform-appropriate variant from a [`ScriptRef`].
fn pick_variant<'a>(s: &'a ScriptRef, os: &Os) -> Option<&'a Path> {
    match os {
        Os::Windows => s.ps1.as_deref().or(s.sh.as_deref()),
        _ => s.sh.as_deref().or(s.ps1.as_deref()),
    }
}

/// Validate the on-disk script obeys the contract: file exists,
/// executable bit on POSIX, non-empty.
fn validate_script_file(path: &Path, os: &Os) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("script not found: {}", path.display()));
    }
    let meta = std::fs::metadata(path).map_err(|e| format!("stat failed: {}", e))?;
    if meta.len() == 0 {
        return Err(format!("script is empty: {}", path.display()));
    }
    #[cfg(unix)]
    {
        if matches!(os, Os::Linux | Os::Macos) {
            use std::os::unix::fs::PermissionsExt;
            let mode = meta.permissions().mode();
            if mode & 0o111 == 0 {
                return Err(format!(
                    "script not executable (chmod +x missing): {}",
                    path.display()
                ));
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = os;
    }
    Ok(())
}

/// Build the dispatcher command string. The script is invoked via its
/// interpreter rather than directly so we don't depend on the +x bit
/// being preserved through OCI extraction (which often strips modes
/// outside ustar's set).
///
/// The interpreter is chosen by script extension, not by OS: `.ps1` files
/// always use `pwsh`, `.sh` files always use `bash`. This matters on Windows
/// when only a `.sh` variant exists — `pick_variant` falls back to it and
/// the script must still be executed by bash (available via Git for Windows
/// on all GitHub-hosted Windows runners).
fn build_command(script: &Path, phase: Phase, version: &str, prior: &str, _os: &Os) -> String {
    let s = script.display();
    let is_ps1 = script
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("ps1"));
    if is_ps1 {
        format!(
            "pwsh -NonInteractive -File {} {} {} {}",
            shquote(&s.to_string()),
            phase.as_str(),
            shquote(version),
            shquote(prior),
        )
    } else {
        format!(
            "bash {} {} {} {}",
            shquote(&s.to_string()),
            phase.as_str(),
            shquote(version),
            shquote(prior),
        )
    }
}

/// Conservative single-quote shell quoting. Sufficient for our argv
/// shapes (versions, kebab-case phase tokens, paths without single
/// quotes); panics-free for arbitrary input by replacing `'` with
/// `'\''`.
fn shquote(s: &str) -> String {
    if s.is_empty() {
        return "''".to_string();
    }
    if s.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | '+'))
    {
        return s.to_string();
    }
    let escaped = s.replace('\'', r"'\''");
    format!("'{}'", escaped)
}

/// Read a JSON-Lines events file produced by a phase script and
/// distill it into a [`PhaseOutcome`]. Best-effort: malformed lines
/// are logged and skipped, missing file is treated as "no events."
fn parse_events_file(path: &Path) -> PhaseOutcome {
    let mut outcome = PhaseOutcome::default();
    let Ok(content) = std::fs::read_to_string(path) else {
        return outcome;
    };
    for (lineno, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<HookEvent>(trimmed) {
            Ok(e) => {
                if e.event == "phase-complete" {
                    outcome.completed = true;
                    if let Some(c) = e.change {
                        outcome.changed = Some(c);
                    }
                }
                outcome.events.push(e);
            }
            Err(err) => {
                tracing::warn!(
                    file = %path.display(),
                    line = lineno + 1,
                    error = %err,
                    "malformed hook event JSON; skipping"
                );
            }
        }
    }
    outcome
}

/// Compute a default per-component log dir under
/// `~/.sindri/logs/<address>/<run>/` so callers that don't yet
/// thread their own path can still drive the dispatcher.
pub fn default_log_dir(component: &str, run_id: &str) -> PathBuf {
    let safe = component.replace([':', '/'], "_");
    sindri_core::paths::home_dir()
        .unwrap_or_default()
        .join(".sindri")
        .join("logs")
        .join(safe)
        .join(run_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sindri_core::platform::TargetProfile;
    use sindri_targets::error::TargetError;
    use sindri_targets::traits::PrereqCheck;
    use std::sync::Mutex;
    use tempfile::TempDir;

    /// Mock target that records the cmd it was asked to exec.
    struct MockTarget {
        commands: Mutex<Vec<String>>,
        envs: Mutex<Vec<Vec<(String, String)>>>,
        fail: bool,
    }
    impl MockTarget {
        fn new() -> Self {
            Self {
                commands: Mutex::new(Vec::new()),
                envs: Mutex::new(Vec::new()),
                fail: false,
            }
        }
        fn failing() -> Self {
            Self {
                commands: Mutex::new(Vec::new()),
                envs: Mutex::new(Vec::new()),
                fail: true,
            }
        }
    }
    impl Target for MockTarget {
        fn name(&self) -> &str {
            "mock-local"
        }
        fn kind(&self) -> &str {
            "local"
        }
        fn profile(&self) -> Result<TargetProfile, TargetError> {
            Err(TargetError::Unavailable {
                name: "mock".into(),
                reason: "test fixture".into(),
            })
        }
        fn exec(&self, cmd: &str, env: &[(&str, &str)]) -> Result<(String, String), TargetError> {
            self.commands.lock().unwrap().push(cmd.to_string());
            self.envs.lock().unwrap().push(
                env.iter()
                    .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                    .collect(),
            );
            if self.fail {
                Err(TargetError::ExecFailed {
                    target: "mock".into(),
                    detail: "simulated".into(),
                })
            } else {
                Ok((String::new(), String::new()))
            }
        }
        fn upload(&self, _: &Path, _: &str) -> Result<(), TargetError> {
            Ok(())
        }
        fn download(&self, _: &str, _: &Path) -> Result<(), TargetError> {
            Ok(())
        }
        fn check_prerequisites(&self) -> Vec<PrereqCheck> {
            Vec::new()
        }
    }

    /// Build a complete on-disk fixture: a package root containing
    /// `scripts/<name>.sh` (executable, non-empty) and a tempdir for
    /// logs. Returns both so the test can also inspect events.
    fn fixture(script_rel: &str) -> (TempDir, TempDir) {
        let pkg = TempDir::new().unwrap();
        let script_abs = pkg.path().join(script_rel);
        std::fs::create_dir_all(script_abs.parent().unwrap()).unwrap();
        // Write a stub script that echoes a phase-complete event so
        // contract honor can be observed on the parse side. The real
        // script never runs here (MockTarget short-circuits).
        std::fs::write(&script_abs, "#!/usr/bin/env bash\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&script_abs).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&script_abs, perms).unwrap();
        }
        let log = TempDir::new().unwrap();
        (pkg, log)
    }

    fn ctx_for<'a>(
        component: &'a str,
        version: &'a str,
        prior: &'a str,
        target: &'a dyn Target,
        pkg: &'a Path,
        log: &'a Path,
    ) -> HookContext<'a> {
        HookContext {
            component,
            version,
            prior_version: prior,
            target,
            env: &[],
            package_root: pkg,
            log_dir: log,
            dry_run: false,
        }
    }

    #[tokio::test]
    async fn missing_phase_is_a_noop() {
        let target = MockTarget::new();
        let (pkg, log) = fixture("scripts/install.sh");
        let hooks = HooksConfig::default();
        let outcome = HooksExecutor::new()
            .run_phase(
                Phase::Install,
                &hooks,
                &ctx_for("npm:foo", "1.0.0", "", &target, pkg.path(), log.path()),
            )
            .await
            .unwrap();
        assert!(outcome.events.is_empty());
        assert!(target.commands.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn install_phase_invokes_bash_with_argv() {
        let target = MockTarget::new();
        let (pkg, log) = fixture("scripts/install.sh");
        let hooks = HooksConfig {
            install: Some(ScriptRef {
                sh: Some(PathBuf::from("scripts/install.sh")),
                ps1: None,
            }),
            ..Default::default()
        };
        HooksExecutor::new()
            .run_phase(
                Phase::Install,
                &hooks,
                &ctx_for("npm:foo", "1.0.0", "0.9.0", &target, pkg.path(), log.path()),
            )
            .await
            .unwrap();
        let cmds = target.commands.lock().unwrap();
        assert_eq!(cmds.len(), 1);
        let cmd = &cmds[0];
        assert!(cmd.starts_with("bash "), "expected bash invocation: {cmd}");
        assert!(cmd.contains(" install "));
        assert!(cmd.contains(" 1.0.0 "));
        assert!(cmd.contains(" 0.9.0"));
    }

    #[tokio::test]
    async fn env_contract_includes_all_documented_keys() {
        let target = MockTarget::new();
        let (pkg, log) = fixture("scripts/install.sh");
        let hooks = HooksConfig {
            install: Some(ScriptRef {
                sh: Some(PathBuf::from("scripts/install.sh")),
                ps1: None,
            }),
            ..Default::default()
        };
        HooksExecutor::new()
            .run_phase(
                Phase::Install,
                &hooks,
                &ctx_for("npm:foo", "1.0.0", "", &target, pkg.path(), log.path()),
            )
            .await
            .unwrap();
        let envs = target.envs.lock().unwrap();
        let env: std::collections::HashMap<String, String> = envs[0].iter().cloned().collect();
        for k in [
            "SINDRI_PHASE",
            "SINDRI_COMPONENT_ADDRESS",
            "SINDRI_COMPONENT_VERSION",
            "SINDRI_PRIOR_VERSION",
            "SINDRI_TARGET",
            "SINDRI_LOG_DIR",
            "SINDRI_EVENTS",
            "SINDRI_DRY_RUN",
        ] {
            assert!(env.contains_key(k), "env missing {k}: have {:?}", env);
        }
        assert_eq!(env["SINDRI_PHASE"], "install");
        assert_eq!(env["SINDRI_COMPONENT_ADDRESS"], "npm:foo");
        assert_eq!(env["SINDRI_COMPONENT_VERSION"], "1.0.0");
        assert_eq!(env["SINDRI_PRIOR_VERSION"], "");
        assert_eq!(env["SINDRI_DRY_RUN"], "0");
    }

    #[tokio::test]
    async fn missing_script_file_returns_hook_failed() {
        let target = MockTarget::new();
        let (pkg, log) = fixture("scripts/install.sh");
        let hooks = HooksConfig {
            install: Some(ScriptRef {
                sh: Some(PathBuf::from("scripts/does-not-exist.sh")),
                ps1: None,
            }),
            ..Default::default()
        };
        let err = HooksExecutor::new()
            .run_phase(
                Phase::Install,
                &hooks,
                &ctx_for("npm:foo", "1.0.0", "", &target, pkg.path(), log.path()),
            )
            .await
            .expect_err("missing script must fail");
        match err {
            ExtensionError::HookFailed { detail, .. } => {
                assert!(detail.contains("script not found"), "got: {detail}");
            }
            other => panic!("expected HookFailed, got {other:?}"),
        }
        // No exec attempted.
        assert!(target.commands.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn target_exec_failure_propagates_as_hook_failed() {
        let target = MockTarget::failing();
        let (pkg, log) = fixture("scripts/install.sh");
        let hooks = HooksConfig {
            install: Some(ScriptRef {
                sh: Some(PathBuf::from("scripts/install.sh")),
                ps1: None,
            }),
            ..Default::default()
        };
        let err = HooksExecutor::new()
            .run_phase(
                Phase::Install,
                &hooks,
                &ctx_for("npm:foo", "1.0.0", "", &target, pkg.path(), log.path()),
            )
            .await
            .expect_err("target failure must propagate");
        assert!(matches!(err, ExtensionError::HookFailed { .. }));
    }

    #[test]
    fn parse_events_file_collects_phase_complete() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("events.jsonl");
        std::fs::write(
            &path,
            r#"{"event":"info","detail":"started"}
{"event":"phase-complete","change":true}
malformed-line
{"event":"info"}
"#,
        )
        .unwrap();
        let outcome = parse_events_file(&path);
        assert!(outcome.completed);
        assert_eq!(outcome.changed, Some(true));
        // 3 valid events; the malformed line is skipped.
        assert_eq!(outcome.events.len(), 3);
    }

    #[test]
    fn shquote_passes_simple_versions_unmodified() {
        assert_eq!(shquote("1.2.3"), "1.2.3");
        assert_eq!(shquote("install"), "install");
        assert_eq!(shquote("npm:foo"), "'npm:foo'");
        assert_eq!(shquote(""), "''");
        assert_eq!(shquote("it's"), r"'it'\''s'");
    }

    #[test]
    fn pick_variant_prefers_os_native_then_falls_back() {
        let r = ScriptRef {
            sh: Some(PathBuf::from("a.sh")),
            ps1: Some(PathBuf::from("b.ps1")),
        };
        assert_eq!(pick_variant(&r, &Os::Linux), Some(Path::new("a.sh")));
        assert_eq!(pick_variant(&r, &Os::Macos), Some(Path::new("a.sh")));
        assert_eq!(pick_variant(&r, &Os::Windows), Some(Path::new("b.ps1")));

        let only_ps1 = ScriptRef {
            sh: None,
            ps1: Some(PathBuf::from("b.ps1")),
        };
        // Fallback when sh is missing on a unix host.
        assert_eq!(
            pick_variant(&only_ps1, &Os::Linux),
            Some(Path::new("b.ps1"))
        );
    }
}
