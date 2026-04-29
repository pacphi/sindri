//! Per-component apply lifecycle (ADR-024).
//!
//! This module factors the per-component lifecycle out of
//! [`crate::commands::apply`] so the top-level command stays a thin
//! load-lockfile / collision-resolve / loop-and-call shell.
//!
//! Per ADR-024, a single component goes through this sequence:
//!
//! ```text
//!   1. PRE-INSTALL  hook   (HooksExecutor::run_pre_install)
//!   2. install             (sindri_backends::install_component)
//!   3. CONFIGURE           (ConfigureExecutor::apply, manifest-only)
//!   4. VALIDATE            (ValidateExecutor::run,    manifest-only)
//!   5. POST-INSTALL hook   (HooksExecutor::run_post_install)
//! ```
//!
//! When the lockfile entry has no embedded `manifest` (the resolver does
//! not yet fetch OCI manifests — Wave 3A), only steps 1, 2, and 5 run.
//! Steps 3 and 4 emit a single `tracing::debug!` and are skipped.
//!
//! Project-init runs **after** every component has been installed, in a
//! second pass driven by [`crate::commands::apply::run`].

use sindri_backends::{install_component, BackendError};
use sindri_core::component::ComponentManifest;
use sindri_core::lockfile::ResolvedComponent;
use sindri_core::platform::Platform;
use sindri_extensions::{
    AuthRedeemer, ComponentBindings, ConfigureContext, ConfigureExecutor, ExtensionError,
    HookContext, HooksExecutor, RedeemedEnv, ValidateContext, ValidateExecutor,
};
use sindri_targets::Target;
use std::path::PathBuf;
use thiserror::Error;

/// Options influencing the per-component lifecycle.
#[derive(Debug, Clone, Default)]
pub struct ApplyOptions {
    /// Filesystem root for shell-rc env fragments. Defaults to
    /// `$HOME/.sindri/env` when `None`.
    pub env_dir: Option<PathBuf>,
    /// Filesystem root for `~`-expansion in [`ConfigureExecutor`]. Defaults
    /// to `$HOME` when `None`.
    pub home_dir: Option<PathBuf>,
    /// If `true`, the redeemer is bypassed entirely for this run (apply
    /// `--skip-auth`). The bypass is logged as `AuthSkippedByUser` per
    /// component by the caller before invoking the lifecycle.
    pub skip_auth: bool,
}

/// Outcome record for a single component's apply.
///
/// Useful for human-readable reporting at the end of the run.
#[derive(Debug, Clone, Default)]
pub struct ApplyOutcome {
    /// `true` if the install backend completed (or reported "already
    /// installed") without error.
    pub installed: bool,
    /// `true` if a [`sindri_core::component::ValidateConfig`] was present
    /// and every assertion passed.
    pub validated: bool,
    /// `true` if a [`sindri_core::component::ConfigureConfig`] was applied.
    pub configured: bool,
    /// Number of lifecycle hooks that fired (0–2 for install-time;
    /// project-init hooks are counted in a separate pass).
    pub hooks_ran: u8,
}

/// Errors produced by the apply lifecycle.
///
/// Wraps the underlying domain errors so the CLI can render a single
/// uniform diagnostic with the right exit code.
#[derive(Debug, Error)]
pub enum ApplyError {
    /// Backend install (or pre-flight dispatch) failed.
    #[error(transparent)]
    Backend(#[from] BackendError),
    /// A capability executor (hooks, configure, validate, project-init)
    /// returned an error.
    #[error(transparent)]
    Extension(#[from] ExtensionError),
}

/// Run the install-time lifecycle for one component.
///
/// `manifest` is `Option<&ComponentManifest>` because OCI manifest fetch
/// lands in Wave 3A; until then the resolver always emits `None` and the
/// configure/validate steps are skipped with a `tracing::debug!`.
///
/// Returns a [`ApplyOutcome`] summarising what actually ran.
pub async fn install_one(
    comp: &ResolvedComponent,
    manifest: Option<&ComponentManifest>,
    target: &dyn Target,
    platform: &Platform,
    options: &ApplyOptions,
) -> Result<ApplyOutcome, ApplyError> {
    install_one_with_bindings(comp, manifest, target, platform, options, None).await
}

/// Variant of [`install_one`] that also runs the auth redeemer for this
/// component's bindings (Phase 2A). When `bindings` is `None` (or empty),
/// behaviour is identical to [`install_one`].
///
/// The redemption flow per ADR-027 §6:
/// 1. **Install / Both** scope bindings are redeemed *before* `pre_install`,
///    so the credential reaches the install command's environment.
/// 2. **Runtime** scope bindings are redeemed *after* `post_install`, so
///    the installed tool sees them on first run; cleanup happens at the
///    end of this function regardless of which scope ran.
pub async fn install_one_with_bindings(
    comp: &ResolvedComponent,
    manifest: Option<&ComponentManifest>,
    target: &dyn Target,
    platform: &Platform,
    options: &ApplyOptions,
    bindings: Option<&ComponentBindings<'_>>,
) -> Result<ApplyOutcome, ApplyError> {
    let mut outcome = ApplyOutcome::default();
    let component_name = comp.id.name.as_str();
    let version = comp.version.0.as_str();
    let hooks = manifest.and_then(|m| m.capabilities.hooks.as_ref());

    let hooks_executor = HooksExecutor::new();
    let redeemer = AuthRedeemer::new();

    // Step 0a: redeem Install/Both bindings (before pre-install).
    let mut install_env = RedeemedEnv::empty();
    if !options.skip_auth {
        if let Some(cb) = bindings {
            install_env = redeemer
                .redeem_install_scope(cb, target)
                .map_err(|e: ExtensionError| ApplyError::Extension(e))?;
        }
    }

    // Step 1: pre-install hook (with redeemed env, if any).
    if let Some(h) = hooks {
        let env_pairs = install_env.env_borrowed();
        let ctx = hook_ctx_with_env(component_name, version, target, &env_pairs);
        hooks_executor.run_pre_install(h, &ctx).await?;
        if h.pre_install.is_some() {
            outcome.hooks_ran += 1;
        }
    }

    // Step 2: install backend.
    install_component(comp, manifest, target).await?;
    outcome.installed = true;

    // Steps 3 & 4: configure + validate, manifest-only.
    if let Some(m) = manifest {
        if let Some(cfg) = m.effective_configure(platform) {
            let env_dir = options.env_dir.clone().unwrap_or_else(default_env_dir);
            let home_dir = options.home_dir.clone().unwrap_or_else(default_home_dir);
            let cfg_ctx = ConfigureContext {
                component: component_name,
                version,
                target_name: target.name(),
                env_dir: env_dir.as_path(),
                home_dir: home_dir.as_path(),
            };
            ConfigureExecutor::new().apply(cfg, &cfg_ctx).await?;
            outcome.configured = true;
        }
        if let Some(v) = m.effective_validate(platform) {
            let v_ctx = ValidateContext {
                component: component_name,
                target,
                env: &[],
            };
            ValidateExecutor::new().run(v, &v_ctx).await?;
            outcome.validated = true;
        }
    } else {
        tracing::debug!(
            component = component_name,
            "manifest not yet plumbed; skipping configure/validate \
             (Wave 3A will fetch ComponentManifest from OCI)"
        );
    }

    // Step 5: post-install hook.
    if let Some(h) = hooks {
        let env_pairs = install_env.env_borrowed();
        let ctx = hook_ctx_with_env(component_name, version, target, &env_pairs);
        hooks_executor.run_post_install(h, &ctx).await?;
        if h.post_install.is_some() {
            outcome.hooks_ran += 1;
        }
    }

    // Step 5b: redeem Runtime-scope bindings (after install completes).
    let mut runtime_env = RedeemedEnv::empty();
    if !options.skip_auth {
        if let Some(cb) = bindings {
            runtime_env = redeemer
                .redeem_runtime_scope(cb, target)
                .map_err(|e: ExtensionError| ApplyError::Extension(e))?;
        }
    }

    // Step 6: cleanup. Always runs — idempotent, best-effort. Persist=true
    // entries survive; transient files are deleted.
    redeemer.cleanup(&install_env, target.name());
    redeemer.cleanup(&runtime_env, target.name());

    Ok(outcome)
}

/// Build a [`HookContext`] for a target. Static lifetimes are easy here
/// because the caller (apply.rs) holds component/version strings on the
/// stack across each `install_one` invocation.
fn hook_ctx<'a>(component: &'a str, version: &'a str, target: &'a dyn Target) -> HookContext<'a> {
    HookContext {
        component,
        version,
        target,
        env: &[],
        workdir: ".",
    }
}

/// Same as [`hook_ctx`] but threads a borrowed env slice through to the
/// hook command. Used when the redeemer has produced env vars that must
/// reach the install command.
fn hook_ctx_with_env<'a>(
    component: &'a str,
    version: &'a str,
    target: &'a dyn Target,
    env: &'a [(&'a str, &'a str)],
) -> HookContext<'a> {
    HookContext {
        component,
        version,
        target,
        env,
        workdir: ".",
    }
}

fn default_env_dir() -> PathBuf {
    if let Some(home) = sindri_core::paths::home_dir() {
        home.join(".sindri").join("env")
    } else {
        PathBuf::from(".sindri/env")
    }
}

fn default_home_dir() -> PathBuf {
    sindri_core::paths::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sindri_core::component::{
        Backend, ComponentCapabilities, ComponentId, ComponentManifest, ComponentMetadata,
        HooksConfig, InstallConfig, ValidateCommand, ValidateConfig,
    };
    use sindri_core::platform::TargetProfile;
    use sindri_core::version::Version;
    use sindri_targets::error::TargetError;
    use sindri_targets::traits::PrereqCheck;
    use std::sync::Mutex;
    use tempfile::TempDir;

    /// Mock target that records every command and can return scripted stdout.
    struct MockTarget {
        commands: Mutex<Vec<String>>,
        responses: Mutex<Vec<(String, String)>>, // popped per call after recording
    }

    impl MockTarget {
        fn new() -> Self {
            Self {
                commands: Mutex::new(Vec::new()),
                responses: Mutex::new(Vec::new()),
            }
        }
        fn with_responses(responses: Vec<&str>) -> Self {
            Self {
                commands: Mutex::new(Vec::new()),
                responses: Mutex::new(
                    responses
                        .into_iter()
                        .map(|s| (s.to_string(), String::new()))
                        .collect(),
                ),
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
            // Pretend to be a local target so the Script backend proceeds
            // (it rejects non-local targets with a Wave-3 stub error). The
            // script backend then no-ops cleanly when no cached script is
            // present, which is exactly what we want for these unit tests.
            "local"
        }
        fn profile(&self) -> Result<TargetProfile, TargetError> {
            Err(TargetError::Unavailable {
                name: "mock".into(),
                reason: "test fixture".into(),
            })
        }
        fn exec(&self, cmd: &str, _env: &[(&str, &str)]) -> Result<(String, String), TargetError> {
            self.commands.lock().unwrap().push(cmd.to_string());
            let mut g = self.responses.lock().unwrap();
            if g.is_empty() {
                Ok((String::new(), String::new()))
            } else {
                Ok(g.remove(0))
            }
        }
        fn upload(&self, _l: &std::path::Path, _r: &str) -> Result<(), TargetError> {
            Ok(())
        }
        fn download(&self, _r: &str, _l: &std::path::Path) -> Result<(), TargetError> {
            Ok(())
        }
        fn check_prerequisites(&self) -> Vec<PrereqCheck> {
            Vec::new()
        }
    }

    /// Build a script-backend ResolvedComponent so `install_component`
    /// dispatches a `sh:noop` install — which the script backend handles
    /// without requiring real binaries on the host.
    fn comp_script(name: &str) -> ResolvedComponent {
        ResolvedComponent {
            id: ComponentId {
                backend: Backend::Script,
                name: name.into(),
                qualifier: None,
            },
            version: Version::new("1.0.0"),
            backend: Backend::Script,
            oci_digest: None,
            checksums: Default::default(),
            depends_on: vec![],
            manifest: None,
            manifest_digest: None,
            component_digest: None,
            platforms: None,
            source: None,
        }
    }

    fn manifest_for(
        name: &str,
        hooks: Option<HooksConfig>,
        validate: Option<ValidateConfig>,
    ) -> ComponentManifest {
        ComponentManifest {
            metadata: ComponentMetadata {
                name: name.into(),
                version: "1.0.0".into(),
                description: "t".into(),
                license: "MIT".into(),
                tags: Vec::new(),
                homepage: None,
            },
            platforms: Vec::new(),
            install: InstallConfig {
                script: Some(sindri_core::component::ScriptInstallConfig {
                    sh: Some("true".into()),
                    ps1: None,
                }),
                ..Default::default()
            },
            depends_on: Vec::new(),
            capabilities: ComponentCapabilities {
                collision_handling: None,
                hooks,
                project_init: None,
            },
            options: Default::default(),
            validate,
            configure: None,
            remove: None,
            overrides: Default::default(),
            auth: Default::default(),
        }
    }

    fn options_with_temp(env: &TempDir, home: &TempDir) -> ApplyOptions {
        ApplyOptions {
            env_dir: Some(env.path().to_path_buf()),
            home_dir: Some(home.path().to_path_buf()),
            skip_auth: false,
        }
    }

    #[tokio::test]
    async fn install_one_runs_pre_install_then_install_then_post_install() {
        let env = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let target = MockTarget::new();
        let platform = Platform::current();
        let comp = comp_script("nodejs");
        let manifest = manifest_for(
            "nodejs",
            Some(HooksConfig {
                pre_install: Some("echo PRE".into()),
                post_install: Some("echo POST".into()),
                ..Default::default()
            }),
            None,
        );

        let outcome = install_one(
            &comp,
            Some(&manifest),
            &target,
            &platform,
            &options_with_temp(&env, &home),
        )
        .await
        .expect("lifecycle ok");

        assert!(outcome.installed);
        assert_eq!(outcome.hooks_ran, 2);
        let captured = target.captured();
        let pre = captured
            .iter()
            .position(|c| c == "echo PRE")
            .expect("pre captured");
        let post = captured
            .iter()
            .position(|c| c == "echo POST")
            .expect("post captured");
        assert!(pre < post, "pre-install must run before post-install");
    }

    #[tokio::test]
    async fn install_one_skips_validate_when_manifest_absent() {
        let env = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let target = MockTarget::new();
        let platform = Platform::current();
        let comp = comp_script("nodejs");

        let outcome = install_one(
            &comp,
            None, // manifest absent
            &target,
            &platform,
            &options_with_temp(&env, &home),
        )
        .await
        .expect("lifecycle ok without manifest");

        assert!(outcome.installed);
        assert!(!outcome.validated);
        assert!(!outcome.configured);
        assert_eq!(outcome.hooks_ran, 0);
    }

    #[tokio::test]
    async fn install_one_runs_validate_when_manifest_present() {
        let env = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        // The script backend is a no-op when there is no cached script,
        // so validate is the FIRST exec call to the mock. Script the
        // first response with the version string.
        let target = MockTarget::with_responses(vec!["v22.5.1\n"]);
        let platform = Platform::current();
        let comp = comp_script("nodejs");
        let manifest = manifest_for(
            "nodejs",
            None,
            Some(ValidateConfig {
                commands: vec![ValidateCommand {
                    command: "node --version".into(),
                    expected_output: None,
                    version_match: Some(">=22.0.0".into()),
                }],
            }),
        );

        let outcome = install_one(
            &comp,
            Some(&manifest),
            &target,
            &platform,
            &options_with_temp(&env, &home),
        )
        .await
        .expect("validate should pass");
        assert!(outcome.validated);
    }

    #[tokio::test]
    async fn validate_failure_aborts_lifecycle() {
        let env = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        // Script backend is a no-op (no cached script); validate is the
        // first exec call → return an incompatible version.
        let target = MockTarget::with_responses(vec!["v18.20.0\n"]);
        let platform = Platform::current();
        let comp = comp_script("nodejs");
        let manifest = manifest_for(
            "nodejs",
            Some(HooksConfig {
                post_install: Some("echo SHOULD_NOT_RUN".into()),
                ..Default::default()
            }),
            Some(ValidateConfig {
                commands: vec![ValidateCommand {
                    command: "node --version".into(),
                    expected_output: None,
                    version_match: Some(">=22.0.0".into()),
                }],
            }),
        );

        let err = install_one(
            &comp,
            Some(&manifest),
            &target,
            &platform,
            &options_with_temp(&env, &home),
        )
        .await
        .expect_err("v18 must abort lifecycle");
        match err {
            ApplyError::Extension(ExtensionError::ValidateFailed { component, .. }) => {
                assert_eq!(component, "nodejs");
            }
            other => panic!("expected ValidateFailed, got {other:?}"),
        }

        // Post-install must NOT have fired.
        assert!(
            !target.captured().iter().any(|c| c == "echo SHOULD_NOT_RUN"),
            "post-install hook must not run after a validate failure"
        );
    }
}
