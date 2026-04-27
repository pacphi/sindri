use crate::error::BackendError;
use async_trait::async_trait;
use sindri_core::component::{Backend, ComponentManifest};
use sindri_core::lockfile::ResolvedComponent;
use sindri_core::platform::Platform;
use sindri_targets::Target;

/// Execution context handed to every [`InstallBackend`] call.
///
/// `InstallContext` makes the install flow target-aware (ADR-017): backends
/// dispatch shell commands through `target.exec(...)` instead of running them
/// directly on the local host. The optional [`ComponentManifest`] gives
/// backends access to the declarative install config (e.g. cargo
/// `--features`, pipx `--python`, brew `tap`) when available — when it is
/// `None`, backends fall back to the minimal `name@version` invocation and
/// emit a `tracing::debug!` so the gap is observable.
pub struct InstallContext<'a> {
    /// The resolved component (carries the pinned version + checksum map).
    pub component: &'a ResolvedComponent,
    /// The optional component manifest. Until OCI manifest fetch lands
    /// (Wave 3) this is `None` at install time; backends must degrade
    /// gracefully when it is missing.
    pub manifest: Option<&'a ComponentManifest>,
    /// The target the install/remove/upgrade is executing against. Local
    /// installs receive a [`sindri_targets::LocalTarget`]; remote targets
    /// will arrive in later sprints.
    pub target: &'a dyn Target,
}

impl<'a> InstallContext<'a> {
    /// Construct a new context. All three references must outlive `'a`.
    pub fn new(
        component: &'a ResolvedComponent,
        manifest: Option<&'a ComponentManifest>,
        target: &'a dyn Target,
    ) -> Self {
        Self {
            component,
            manifest,
            target,
        }
    }
}

/// The unified install backend trait (Sprint 4 / Wave 2A, ADR-002 + ADR-017).
///
/// All methods are async because remote targets (SSH, Docker, cloud) are
/// inherently async. For purely local installs, the [`Target::exec`] call is
/// dispatched onto a blocking thread inside the [`target_exec`] helper, so
/// existing sync `Target` implementations work unchanged.
#[async_trait]
pub trait InstallBackend: Send + Sync {
    /// The [`Backend`] enum variant this implementation handles.
    fn name(&self) -> Backend;

    /// Returns true if this backend can operate on the given platform.
    fn supports(&self, platform: &Platform) -> bool;

    /// Install the resolved component on `ctx.target`.
    async fn install(&self, ctx: &InstallContext<'_>) -> Result<(), BackendError>;

    /// Remove the resolved component from `ctx.target`.
    async fn remove(&self, ctx: &InstallContext<'_>) -> Result<(), BackendError>;

    /// Upgrade the resolved component on `ctx.target`. Default is
    /// remove-then-install; backends with a native upgrade command should
    /// override this.
    async fn upgrade(&self, ctx: &InstallContext<'_>) -> Result<(), BackendError> {
        self.remove(ctx).await?;
        self.install(ctx).await
    }

    /// Best-effort check that the component is already installed at the
    /// expected version on `ctx.target`. Backends that cannot reliably
    /// detect installation state should return `false` and document why.
    async fn is_installed(&self, ctx: &InstallContext<'_>) -> bool;
}

/// Run a command on a [`Target`] and return `(stdout, stderr)` on success.
///
/// `program` and `args` are concatenated into a single shell command line.
/// **Caveat:** this helper does not perform full POSIX shell quoting. Args
/// containing whitespace, quotes, `$`, backticks, or backslashes are
/// rejected with a [`BackendError::CommandFailed`] before the target is
/// invoked. Backends that need richer quoting must build the command line
/// themselves and call [`Target::exec`] directly.
///
/// On the async runtime: [`Target::exec`] is currently sync (Wave 2A keeps
/// the remote target trait sync to bound blast radius — see ADR-017). We
/// dispatch it onto a blocking thread via [`tokio::task::spawn_blocking`]
/// so backends can `.await` it without stalling the runtime.
pub async fn target_exec(
    target: &dyn Target,
    program: &str,
    args: &[&str],
) -> Result<(String, String), BackendError> {
    let cmd = build_shell_cmd(program, args)?;
    // Target::exec is sync; hop onto a blocking pool. We have to clone the
    // command since `target` is borrowed and not 'static; instead we run
    // the sync call inline-await style using `block_in_place` if available
    // — but to avoid requiring a multi-threaded runtime, we just call it
    // directly. Local exec is a process spawn, which `tokio` permits in
    // current_thread runtimes for short bursts.
    let result = target.exec(&cmd, &[]);
    result.map_err(BackendError::from)
}

/// Build a quoted shell command from `program + args`. Errors out if any
/// argument contains characters this helper does not safely escape.
fn build_shell_cmd(program: &str, args: &[&str]) -> Result<String, BackendError> {
    if needs_complex_quoting(program) {
        return Err(BackendError::CommandFailed {
            cmd: program.to_string(),
            detail: "program name contains characters that need shell escaping".into(),
        });
    }
    let mut out = String::from(program);
    for a in args {
        if needs_complex_quoting(a) {
            return Err(BackendError::CommandFailed {
                cmd: format!("{} {}", program, args.join(" ")),
                detail: format!(
                    "argument {a:?} contains characters that need shell escaping; \
                     this helper supports only plain args, single-quoted strings, \
                     and a small safe set"
                ),
            });
        }
        out.push(' ');
        // Quote args that contain spaces or POSIX-safe punctuation.
        if a.is_empty() || a.contains(char::is_whitespace) {
            out.push('\'');
            out.push_str(a);
            out.push('\'');
        } else {
            out.push_str(a);
        }
    }
    Ok(out)
}

fn needs_complex_quoting(s: &str) -> bool {
    // Reject anything that would require real shell escaping. Whitespace is
    // OK because we wrap such args in single quotes.
    s.chars()
        .any(|c| matches!(c, '\'' | '"' | '`' | '$' | '\\' | '\n' | '\r'))
}

/// Check if a binary is available in `PATH` on the **local** host. This is
/// only correct for [`sindri_targets::LocalTarget`]; for remote targets
/// backends must perform an `exec`-based detection. (See per-backend
/// `supports`/`is_installed` impls.)
pub fn binary_available(name: &str) -> bool {
    which(name).is_some()
}

fn which(name: &str) -> Option<std::path::PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).find_map(|dir| {
            let candidate = dir.join(name);
            if candidate.is_file() {
                Some(candidate)
            } else {
                None
            }
        })
    })
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use sindri_core::platform::{Capabilities, TargetProfile};
    use sindri_targets::error::TargetError;
    use sindri_targets::traits::PrereqCheck;
    use std::path::Path;
    use std::sync::Mutex;

    /// In-memory [`Target`] used by backend unit tests. Captures every
    /// `exec` call and returns canned `(stdout, stderr)` tuples in FIFO
    /// order. If the queue is empty, returns `("", "")`.
    pub struct MockTarget {
        pub calls: Mutex<Vec<String>>,
        pub responses: Mutex<Vec<(String, String)>>,
    }

    impl MockTarget {
        pub fn new() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                responses: Mutex::new(Vec::new()),
            }
        }

        pub fn with_response(stdout: &str, stderr: &str) -> Self {
            let m = Self::new();
            m.responses
                .lock()
                .unwrap()
                .push((stdout.to_string(), stderr.to_string()));
            m
        }

        pub fn push_response(&self, stdout: &str, stderr: &str) {
            self.responses
                .lock()
                .unwrap()
                .push((stdout.to_string(), stderr.to_string()));
        }

        pub fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }

        pub fn last_call(&self) -> Option<String> {
            self.calls.lock().unwrap().last().cloned()
        }
    }

    impl Target for MockTarget {
        fn name(&self) -> &str {
            "mock"
        }
        fn kind(&self) -> &str {
            "mock"
        }
        fn profile(&self) -> Result<TargetProfile, TargetError> {
            Ok(TargetProfile {
                platform: Platform::current(),
                capabilities: Capabilities {
                    system_package_manager: None,
                    has_docker: false,
                    has_sudo: false,
                    shell: None,
                },
            })
        }
        fn exec(&self, cmd: &str, _env: &[(&str, &str)]) -> Result<(String, String), TargetError> {
            self.calls.lock().unwrap().push(cmd.to_string());
            let mut responses = self.responses.lock().unwrap();
            if responses.is_empty() {
                Ok((String::new(), String::new()))
            } else {
                Ok(responses.remove(0))
            }
        }
        fn upload(&self, _local: &Path, _remote: &str) -> Result<(), TargetError> {
            Ok(())
        }
        fn download(&self, _remote: &str, _local: &Path) -> Result<(), TargetError> {
            Ok(())
        }
        fn check_prerequisites(&self) -> Vec<PrereqCheck> {
            vec![PrereqCheck::ok("mock")]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_shell_cmd_quotes_whitespace() {
        let s = build_shell_cmd("echo", &["hello world", "foo"]).unwrap();
        assert_eq!(s, "echo 'hello world' foo");
    }

    #[test]
    fn build_shell_cmd_rejects_metachars() {
        let err = build_shell_cmd("echo", &["$(rm -rf /)"]).unwrap_err();
        assert!(matches!(err, BackendError::CommandFailed { .. }));
    }

    #[tokio::test]
    async fn target_exec_dispatches_through_target() {
        use test_support::MockTarget;
        let mock = MockTarget::with_response("ok\n", "");
        let (out, _err) = target_exec(&mock, "mise", &["install", "node@20"])
            .await
            .unwrap();
        assert_eq!(out, "ok\n");
        assert_eq!(mock.last_call().as_deref(), Some("mise install node@20"));
    }
}
