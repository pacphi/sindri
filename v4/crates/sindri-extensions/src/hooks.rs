//! Lifecycle hook executor (Sprint 4 §4.3, ADR-024).
//!
//! [`HooksExecutor`] dispatches the four optional shell commands declared
//! under a component's `capabilities.hooks` block:
//!
//! * `pre_install` / `post_install` — wrap the install backend.
//! * `pre_project_init` / `post_project_init` — wrap project-init steps.
//!
//! Each command is shelled through the active [`Target`] so that a hook
//! observes the same environment, working directory, and authentication
//! context as the install itself. A non-zero exit (i.e. any
//! [`sindri_targets::error::TargetError`]) is mapped to
//! [`ExtensionError::HookFailed`] with the failing component and command
//! captured for diagnostics.

use crate::error::ExtensionError;
use sindri_core::component::HooksConfig;
use sindri_targets::Target;

/// Context passed to every hook invocation.
///
/// The lifetime parameter ties the borrowed [`Target`] reference to the
/// caller's stack frame so that `HooksExecutor` does not own the target.
pub struct HookContext<'a> {
    /// Component metadata name (e.g. `"nodejs"`).
    pub component: &'a str,
    /// Component metadata version (e.g. `"1.2.3"`).
    pub version: &'a str,
    /// The execution target.
    pub target: &'a dyn Target,
    /// Environment variables to expose to the hook command.
    pub env: &'a [(&'a str, &'a str)],
    /// Working directory to run the hook in (advisory; passed via env).
    pub workdir: &'a str,
}

/// Capability executor for `capabilities.hooks.*` commands.
///
/// Stateless; instances are cheap to create.
#[derive(Debug, Default, Clone, Copy)]
pub struct HooksExecutor;

impl HooksExecutor {
    /// Create a new executor.
    pub fn new() -> Self {
        Self
    }

    /// Run `hooks.pre_install` if present.
    pub async fn run_pre_install(
        &self,
        hooks: &HooksConfig,
        ctx: &HookContext<'_>,
    ) -> Result<(), ExtensionError> {
        self.dispatch(hooks.pre_install.as_deref(), "pre-install", ctx)
    }

    /// Run `hooks.post_install` if present.
    pub async fn run_post_install(
        &self,
        hooks: &HooksConfig,
        ctx: &HookContext<'_>,
    ) -> Result<(), ExtensionError> {
        self.dispatch(hooks.post_install.as_deref(), "post-install", ctx)
    }

    /// Run `hooks.pre_project_init` if present.
    pub async fn run_pre_project_init(
        &self,
        hooks: &HooksConfig,
        ctx: &HookContext<'_>,
    ) -> Result<(), ExtensionError> {
        self.dispatch(hooks.pre_project_init.as_deref(), "pre-project-init", ctx)
    }

    /// Run `hooks.post_project_init` if present.
    pub async fn run_post_project_init(
        &self,
        hooks: &HooksConfig,
        ctx: &HookContext<'_>,
    ) -> Result<(), ExtensionError> {
        self.dispatch(hooks.post_project_init.as_deref(), "post-project-init", ctx)
    }

    /// Internal dispatcher: shells `cmd` through `ctx.target` if `Some`.
    fn dispatch(
        &self,
        cmd: Option<&str>,
        phase: &str,
        ctx: &HookContext<'_>,
    ) -> Result<(), ExtensionError> {
        let Some(cmd) = cmd else {
            tracing::debug!(
                component = ctx.component,
                phase,
                "no hook declared; skipping"
            );
            return Ok(());
        };

        tracing::info!(
            component = ctx.component,
            version = ctx.version,
            phase,
            command = cmd,
            "running lifecycle hook"
        );

        match ctx.target.exec(cmd, ctx.env) {
            Ok((stdout, stderr)) => {
                tracing::debug!(
                    component = ctx.component,
                    phase,
                    stdout_bytes = stdout.len(),
                    stderr_bytes = stderr.len(),
                    "hook completed"
                );
                Ok(())
            }
            Err(err) => Err(ExtensionError::HookFailed {
                component: ctx.component.to_string(),
                command: cmd.to_string(),
                detail: err.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sindri_core::platform::TargetProfile;
    use sindri_targets::error::TargetError;
    use sindri_targets::traits::PrereqCheck;
    use std::sync::Mutex;

    /// Minimal in-memory target used to assert hook dispatch.
    ///
    /// Uses [`Mutex`] for interior mutability so the test fixture is genuinely
    /// `Send + Sync`, satisfying the [`Target`] trait bounds without `unsafe`.
    struct MockTarget {
        commands: Mutex<Vec<String>>,
        fail: bool,
    }

    impl MockTarget {
        fn new() -> Self {
            Self {
                commands: Mutex::new(Vec::new()),
                fail: false,
            }
        }
        fn failing() -> Self {
            Self {
                commands: Mutex::new(Vec::new()),
                fail: true,
            }
        }
        fn captured(&self) -> Vec<String> {
            self.commands.lock().unwrap().clone()
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
            Err(TargetError::Unavailable {
                name: "mock".into(),
                reason: "test fixture".into(),
            })
        }
        fn exec(&self, cmd: &str, _env: &[(&str, &str)]) -> Result<(String, String), TargetError> {
            self.commands.lock().unwrap().push(cmd.to_string());
            if self.fail {
                Err(TargetError::ExecFailed {
                    target: "mock".into(),
                    detail: format!("simulated failure for `{cmd}`"),
                })
            } else {
                Ok((String::new(), String::new()))
            }
        }
        fn upload(&self, _local: &std::path::Path, _remote: &str) -> Result<(), TargetError> {
            Ok(())
        }
        fn download(&self, _remote: &str, _local: &std::path::Path) -> Result<(), TargetError> {
            Ok(())
        }
        fn check_prerequisites(&self) -> Vec<PrereqCheck> {
            Vec::new()
        }
    }

    fn ctx<'a>(target: &'a dyn Target) -> HookContext<'a> {
        HookContext {
            component: "nodejs",
            version: "1.0.0",
            target,
            env: &[],
            workdir: "/tmp",
        }
    }

    #[tokio::test]
    async fn pre_install_runs_on_target() {
        let target = MockTarget::new();
        let hooks = HooksConfig {
            pre_install: Some("echo hi".into()),
            ..Default::default()
        };
        HooksExecutor::new()
            .run_pre_install(&hooks, &ctx(&target))
            .await
            .expect("pre-install should succeed");
        assert_eq!(target.captured(), vec!["echo hi".to_string()]);
    }

    #[tokio::test]
    async fn missing_hook_is_a_noop() {
        let target = MockTarget::new();
        let hooks = HooksConfig::default();
        HooksExecutor::new()
            .run_post_install(&hooks, &ctx(&target))
            .await
            .expect("noop should succeed");
        assert!(target.captured().is_empty());
    }

    #[tokio::test]
    async fn post_install_failure_propagates() {
        let target = MockTarget::failing();
        let hooks = HooksConfig {
            post_install: Some("false".into()),
            ..Default::default()
        };
        let err = HooksExecutor::new()
            .run_post_install(&hooks, &ctx(&target))
            .await
            .expect_err("should fail");
        match err {
            ExtensionError::HookFailed {
                component, command, ..
            } => {
                assert_eq!(component, "nodejs");
                assert_eq!(command, "false");
            }
            other => panic!("expected HookFailed, got {other:?}"),
        }
    }
}
