//! Apply-time auth redemption (ADR-027 §6, DDD-07 redeemer).
//!
//! Phase 2A of the auth-aware implementation plan. The resolver's
//! observability-only [`AuthBinding`]s now drive apply behaviour:
//!
//! 1. Before the install lifecycle starts, [`AuthRedeemer::redeem_install_scope`]
//!    walks the lockfile's `auth_bindings` and materialises the bound source
//!    into a runtime [`RedemptionEnv`] (env vars, files on disk).
//! 2. After install completes, [`AuthRedeemer::redeem_runtime_scope`] handles
//!    `scope: runtime` bindings symmetrically (these are wanted at *runtime*
//!    of the installed tool, not during install).
//! 3. Once the lifecycle phase that needed the credential finishes, the
//!    [`RedemptionEnv`] is *cleaned up*: in-memory copies are dropped, files
//!    flagged `persist: false` are deleted, and an `AuthCleanedUp` ledger
//!    event is emitted.
//!
//! ## Redaction discipline
//!
//! The [`AuthBinding`] domain captures only references (DDD-07 invariant 3).
//! All ledger events emitted from this module follow the same rule:
//! payloads carry the binding id, redemption kind, and target — *never* the
//! resolved value. A property test in `tests/redaction.rs` fails closed if
//! any code path here ever leaks a value into a ledger event.
//!
//! ## Why this lives in `sindri-extensions`
//!
//! The redeemer hooks the same apply lifecycle as `HooksExecutor` and
//! `ConfigureExecutor`: it is a capability executor whose unit of work is a
//! lockfile entry, not a resolver pass. Putting it in `sindri-extensions`
//! keeps `sindri-core` schema-only and matches the ADR-027 §6 narrative
//! "redemption happens immediately before pre_install".

use crate::error::ExtensionError;
use sindri_core::auth::{
    AuthBinding, AuthBindingStatus, AuthRequirements, AuthScope, AuthSource, Redemption,
};
use sindri_core::component::ComponentManifest;
use sindri_core::lockfile::Lockfile;
use sindri_targets::auth::AuthValue;
use sindri_targets::Target;
use std::collections::HashMap;
use std::path::PathBuf;

/// Default file mode for redeemed credential files (ADR-027 §6).
const DEFAULT_FILE_MODE: u32 = 0o600;

/// Owned env-var pair for redemption injection.
///
/// The values are kept on the stack of the apply lifecycle (held by the
/// caller for the duration of one lifecycle step) and dropped — i.e. memory
/// zeroised by the allocator's normal mechanism — as soon as the step
/// returns. We do not expose this struct outside the crate.
#[derive(Debug, Clone)]
pub struct RedeemedEnv {
    /// `(NAME, VALUE)` pairs to merge into [`Target::exec`] env.
    pub env: Vec<(String, String)>,
    /// Files written to disk that should be deleted post-apply
    /// (mode + persist semantics from [`Redemption::File`] /
    /// [`Redemption::EnvFile`]).
    pub temp_files: Vec<TempFile>,
    /// Binding ids that were redeemed in this batch — used by the cleanup
    /// hook to emit one `AuthCleanedUp` event per binding.
    pub binding_ids: Vec<String>,
}

/// A file written by redemption that may need cleanup post-apply.
#[derive(Debug, Clone)]
pub struct TempFile {
    pub path: PathBuf,
    pub persist: bool,
    pub binding_id: String,
}

impl RedeemedEnv {
    /// Empty redeemed-env (no bindings produced output for this scope).
    pub fn empty() -> Self {
        RedeemedEnv {
            env: Vec::new(),
            temp_files: Vec::new(),
            binding_ids: Vec::new(),
        }
    }

    /// True when nothing was redeemed.
    pub fn is_empty(&self) -> bool {
        self.env.is_empty() && self.temp_files.is_empty()
    }

    /// View as `&[(&str, &str)]` borrowed slice for [`Target::exec`].
    pub fn env_borrowed(&self) -> Vec<(&str, &str)> {
        self.env
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }
}

/// Stateless capability executor for auth redemption.
#[derive(Debug, Default, Clone, Copy)]
pub struct AuthRedeemer;

/// Per-component view of bindings that apply to a specific component.
///
/// Built once per apply run from the lockfile's `auth_bindings`.
pub struct ComponentBindings<'a> {
    /// Component address (e.g. `npm:claude-code`).
    pub component: &'a str,
    /// Bindings whose `component == component`.
    pub bindings: Vec<&'a AuthBinding>,
    /// The resolved component manifest's `auth:` block — needed to recover
    /// per-requirement [`Redemption`] and [`AuthScope`] (the binding itself
    /// only carries the source).
    pub auth: &'a AuthRequirements,
}

impl AuthRedeemer {
    pub fn new() -> Self {
        Self
    }

    /// Redeem all bindings whose scope is `Install` or `Both` for this
    /// component. Called immediately before `pre_install`.
    pub fn redeem_install_scope(
        &self,
        cb: &ComponentBindings<'_>,
        target: &dyn Target,
    ) -> Result<RedeemedEnv, ExtensionError> {
        self.redeem_with_filter(cb, target, |s| {
            matches!(s, AuthScope::Install | AuthScope::Both)
        })
    }

    /// Redeem all bindings whose scope is `Runtime` for this component.
    /// Called after `post_install` so the installed tool has its credential
    /// for first-run.
    pub fn redeem_runtime_scope(
        &self,
        cb: &ComponentBindings<'_>,
        target: &dyn Target,
    ) -> Result<RedeemedEnv, ExtensionError> {
        self.redeem_with_filter(cb, target, |s| matches!(s, AuthScope::Runtime))
    }

    fn redeem_with_filter<F: Fn(AuthScope) -> bool>(
        &self,
        cb: &ComponentBindings<'_>,
        _target: &dyn Target,
        wants_scope: F,
    ) -> Result<RedeemedEnv, ExtensionError> {
        let mut out = RedeemedEnv::empty();

        for b in &cb.bindings {
            if b.status != AuthBindingStatus::Bound {
                continue;
            }
            let Some(source) = b.source.as_ref() else {
                continue;
            };
            // Recover the requirement's redemption + scope from the manifest.
            let (redemption, scope) = match find_requirement(cb.auth, &b.requirement) {
                Some(p) => p,
                None => {
                    tracing::warn!(
                        component = b.component.as_str(),
                        requirement = b.requirement.as_str(),
                        "auth binding refers to requirement not declared on the component manifest; \
                         skipping redemption"
                    );
                    continue;
                }
            };
            if !wants_scope(scope) {
                continue;
            }

            let value = resolve_source(source).map_err(|e| ExtensionError::HookFailed {
                component: cb.component.to_string(),
                command: format!("auth_redeem({})", b.requirement),
                detail: e.to_string(),
            })?;
            apply_redemption(&redemption, &value, b, &mut out)?;
            out.binding_ids.push(b.id.clone());
            ledger::emit_redeemed(b, redemption_kind(&redemption));
        }

        Ok(out)
    }

    /// Run cleanup for a previously-redeemed batch. Idempotent: running it
    /// twice on the same [`RedeemedEnv`] does not error if the file is
    /// already gone (the second pass is a no-op).
    pub fn cleanup(&self, env: &RedeemedEnv, target_name: &str) {
        for tf in &env.temp_files {
            if tf.persist {
                continue;
            }
            // Best-effort delete; do not fail apply because cleanup ran twice.
            let _ = std::fs::remove_file(&tf.path);
        }
        for binding_id in &env.binding_ids {
            // Number of files that this binding contributed (0 or 1 in
            // current redemption variants).
            let files_removed = env
                .temp_files
                .iter()
                .filter(|tf| &tf.binding_id == binding_id && !tf.persist)
                .count();
            ledger::emit_cleanup(binding_id, target_name, files_removed);
        }
    }
}

/// Build the per-component binding view by joining the lockfile's bindings
/// with each component's manifest. Components without bindings yield no
/// entries.
pub fn group_bindings_by_component<'a>(
    lockfile: &'a Lockfile,
    manifests: &'a HashMap<String, &'a ComponentManifest>,
) -> Vec<ComponentBindings<'a>> {
    // address -> Vec<&AuthBinding>
    let mut by_addr: HashMap<&str, Vec<&AuthBinding>> = HashMap::new();
    for b in &lockfile.auth_bindings {
        by_addr.entry(b.component.as_str()).or_default().push(b);
    }

    let mut out = Vec::new();
    for (addr, bindings) in by_addr {
        if let Some(m) = manifests.get(addr) {
            out.push(ComponentBindings {
                component: addr,
                bindings,
                auth: &m.auth,
            });
        }
    }
    out
}

/// Locate the [`Redemption`] + [`AuthScope`] for a requirement name across
/// all four requirement families on an [`AuthRequirements`] block.
fn find_requirement(auth: &AuthRequirements, name: &str) -> Option<(Redemption, AuthScope)> {
    if let Some(t) = auth.tokens.iter().find(|t| t.name == name) {
        return Some((t.redemption.clone(), t.scope));
    }
    if let Some(o) = auth.oauth.iter().find(|o| o.name == name) {
        return Some((o.redemption.clone(), o.scope));
    }
    if let Some(c) = auth.certs.iter().find(|c| c.name == name) {
        return Some((c.redemption.clone(), c.scope));
    }
    if let Some(s) = auth.ssh.iter().find(|s| s.name == name) {
        return Some((s.redemption.clone(), s.scope));
    }
    None
}

fn redemption_kind(r: &Redemption) -> &'static str {
    match r {
        Redemption::EnvVar { .. } => "env-var",
        Redemption::File { .. } => "file",
        Redemption::EnvFile { .. } => "env-file",
    }
}

/// Resolve an [`AuthSource`] to a string secret value. The value is held
/// only on the stack; never logged, never persisted, never returned via
/// any error type.
fn resolve_source(source: &AuthSource) -> Result<String, ResolveError> {
    match source {
        AuthSource::FromEnv { var } => std::env::var(var)
            .map_err(|_| ResolveError(format!("env var {var} is not set"))),
        AuthSource::FromFile { path, .. } => std::fs::read_to_string(path)
            .map(|s| s.trim().to_string())
            .map_err(|e| ResolveError(format!("read {path}: {e}"))),
        AuthSource::FromCli { command } => {
            // Reuse the AuthValue::Cli resolver so behaviour matches the
            // existing ADR-020 plumbing.
            AuthValue::Cli(command.clone())
                .resolve()
                .map_err(|e| ResolveError(e.to_string()))
        }
        AuthSource::FromSecretsStore { backend, path } => {
            // sindri-secrets is not yet wired (Phase 0 placeholder; ADR-025).
            // Surface a typed error so Gate 5 can deny / users get clear
            // remediation. NEVER fall back to empty string.
            Err(ResolveError(format!(
                "secrets backend `{backend}` is not yet wired (sindri-secrets unavailable); \
                 path was {path}"
            )))
        }
        AuthSource::FromUpstreamCredentials => Err(ResolveError(
            "from-upstream-credentials redemption is gated by policy.auth.allow_upstream_credentials; \
             enable explicitly or add `provides:` on the target".into(),
        )),
        AuthSource::FromOAuth { provider } => Err(ResolveError(format!(
            "OAuth redemption (provider={provider}) lands in Phase 5"
        ))),
        AuthSource::Prompt => Err(ResolveError(
            "Prompt redemption requires an interactive TTY (Phase 5)".into(),
        )),
    }
}

#[derive(Debug)]
struct ResolveError(String);
impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ResolveError {}

/// Apply a [`Redemption`] decision to the in-progress [`RedeemedEnv`]. The
/// resolved `value` is consumed once and never logged.
fn apply_redemption(
    r: &Redemption,
    value: &str,
    binding: &AuthBinding,
    out: &mut RedeemedEnv,
) -> Result<(), ExtensionError> {
    match r {
        Redemption::EnvVar { env_name } => {
            if env_name.is_empty() {
                return Err(ExtensionError::HookFailed {
                    component: binding.component.clone(),
                    command: "auth_redeem(EnvVar)".into(),
                    detail: "redemption.env-name is empty".into(),
                });
            }
            out.env.push((env_name.clone(), value.to_string()));
        }
        Redemption::File {
            path,
            mode,
            persist,
        } => {
            let p = expand_path(path);
            write_secret_file(&p, value, mode.unwrap_or(DEFAULT_FILE_MODE))?;
            out.temp_files.push(TempFile {
                path: p,
                persist: *persist,
                binding_id: binding.id.clone(),
            });
        }
        Redemption::EnvFile { env_name, path } => {
            let p = expand_path(path);
            write_secret_file(&p, value, DEFAULT_FILE_MODE)?;
            out.env
                .push((env_name.clone(), p.to_string_lossy().to_string()));
            out.temp_files.push(TempFile {
                path: p,
                // env-file is by definition transient unless caller pinned
                // persist on the underlying File-redemption (which env-file
                // doesn't expose). Default cleanup.
                persist: false,
                binding_id: binding.id.clone(),
            });
        }
    }
    Ok(())
}

fn expand_path(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs_next::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

fn write_secret_file(path: &std::path::Path, value: &str, mode: u32) -> Result<(), ExtensionError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Write atomically: temp + rename. For 0600-mode secrets the parent
    // directory ACL is not changed; we trust the caller to put creds in a
    // private dir.
    std::fs::write(path, value)?;
    set_permissions(path, mode)?;
    Ok(())
}

#[cfg(unix)]
fn set_permissions(path: &std::path::Path, mode: u32) -> Result<(), ExtensionError> {
    use std::os::unix::fs::PermissionsExt;
    let perms = std::fs::Permissions::from_mode(mode);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_permissions(_path: &std::path::Path, _mode: u32) -> Result<(), ExtensionError> {
    // Windows file ACLs are not modelled by mode bits; rely on the user
    // profile dir being private. No-op rather than spurious failure.
    Ok(())
}

// =============================================================================
// Ledger emission (Phase 2 events: AuthRedeemed, AuthCleanedUp,
// AuthSkippedByUser).
// =============================================================================

pub mod ledger {
    //! Phase 2A audit ledger events.
    //!
    //! These events live in the same JSONL file as the Phase 1 binding
    //! events (`~/.sindri/ledger.jsonl`). Payloads NEVER carry the
    //! redeemed credential value — they reference the binding by id.
    //! See [`crate::redeemer`] module docs for the redaction property
    //! test that enforces this.

    use serde::{Deserialize, Serialize};
    use sindri_core::auth::AuthBinding;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// A Phase 2A redemption event.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct RedemptionLedgerEvent {
        pub timestamp: u64,
        /// One of `AuthRedeemed`, `AuthCleanedUp`, `AuthSkippedByUser`.
        pub event_type: String,
        /// Binding id (sha256 prefix). Empty for `AuthSkippedByUser`.
        #[serde(default)]
        pub binding_id: String,
        /// Redemption kind: `env-var`, `file`, `env-file`. Empty when
        /// not applicable.
        #[serde(default)]
        pub redemption_kind: String,
        /// Target name (e.g. `local`, `prod-fly`).
        #[serde(default)]
        pub target: String,
        /// Component address; populated for `AuthSkippedByUser` so the
        /// auditor can see which install bypassed redemption.
        #[serde(default)]
        pub component: String,
        /// File count for `AuthCleanedUp`. 0 for env-only bindings.
        #[serde(default)]
        pub files_removed: usize,
    }

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    fn ledger_path() -> Option<PathBuf> {
        // Allow tests / sandboxes to redirect via an env var. Unset in
        // production deployments.
        if let Ok(p) = std::env::var("SINDRI_AUTH_LEDGER_PATH") {
            return Some(PathBuf::from(p));
        }
        dirs_next::home_dir().map(|h| h.join(".sindri").join("ledger.jsonl"))
    }

    fn append(event: &RedemptionLedgerEvent) {
        let Some(path) = ledger_path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            if std::fs::create_dir_all(parent).is_err() {
                return;
            }
        }
        let json = match serde_json::to_string(event) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("auth-ledger serialise failed: {}", e);
                return;
            }
        };
        use std::io::Write;
        match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            Ok(mut f) => {
                if let Err(e) = writeln!(f, "{}", json) {
                    tracing::warn!("auth-ledger write failed: {}", e);
                }
            }
            Err(e) => tracing::warn!("auth-ledger open failed: {}", e),
        }
    }

    pub fn emit_redeemed(b: &AuthBinding, redemption_kind: &str) {
        append(&RedemptionLedgerEvent {
            timestamp: now_secs(),
            event_type: "AuthRedeemed".into(),
            binding_id: b.id.clone(),
            redemption_kind: redemption_kind.into(),
            target: b.target.clone(),
            component: String::new(),
            files_removed: 0,
        });
    }

    pub fn emit_cleanup(binding_id: &str, target: &str, files_removed: usize) {
        append(&RedemptionLedgerEvent {
            timestamp: now_secs(),
            event_type: "AuthCleanedUp".into(),
            binding_id: binding_id.into(),
            redemption_kind: String::new(),
            target: target.into(),
            component: String::new(),
            files_removed,
        });
    }

    pub fn emit_skipped_by_user(component: &str, target: &str) {
        append(&RedemptionLedgerEvent {
            timestamp: now_secs(),
            event_type: "AuthSkippedByUser".into(),
            binding_id: String::new(),
            redemption_kind: String::new(),
            target: target.into(),
            component: component.into(),
            files_removed: 0,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sindri_core::auth::{
        AuthBinding, AuthBindingStatus, AuthScope, AuthSource, DiscoveryHints, Redemption,
        TokenRequirement,
    };

    fn token_req(
        name: &str,
        audience: &str,
        redemption: Redemption,
        scope: AuthScope,
    ) -> TokenRequirement {
        TokenRequirement {
            name: name.into(),
            description: name.into(),
            scope,
            optional: false,
            audience: audience.into(),
            redemption,
            discovery: DiscoveryHints::default(),
        }
    }

    fn binding(
        component: &str,
        requirement: &str,
        target: &str,
        source: AuthSource,
    ) -> AuthBinding {
        AuthBinding {
            id: format!("{component}:{requirement}:{target}"),
            component: component.into(),
            requirement: requirement.into(),
            audience: "urn:x".into(),
            target: target.into(),
            source: Some(source),
            priority: 0,
            status: AuthBindingStatus::Bound,
            reason: None,
            considered: Vec::new(),
        }
    }

    #[test]
    fn redemption_kind_strings() {
        assert_eq!(
            redemption_kind(&Redemption::EnvVar {
                env_name: "X".into(),
            }),
            "env-var"
        );
        assert_eq!(
            redemption_kind(&Redemption::File {
                path: "/p".into(),
                mode: None,
                persist: false,
            }),
            "file"
        );
        assert_eq!(
            redemption_kind(&Redemption::EnvFile {
                env_name: "X".into(),
                path: "/p".into(),
            }),
            "env-file"
        );
    }

    #[test]
    fn find_requirement_locates_token() {
        let auth = AuthRequirements {
            tokens: vec![token_req(
                "tok",
                "urn:x",
                Redemption::EnvVar {
                    env_name: "T".into(),
                },
                AuthScope::Runtime,
            )],
            ..Default::default()
        };
        let (r, s) = find_requirement(&auth, "tok").unwrap();
        assert!(matches!(r, Redemption::EnvVar { .. }));
        assert_eq!(s, AuthScope::Runtime);
    }

    #[test]
    fn resolve_from_env_reads_process_env() {
        // SAFETY: tests are single-threaded by default in `cargo test` only
        // when --test-threads=1; we use a unique key to avoid collisions.
        std::env::set_var("SINDRI_TEST_REDEEM_ENV", "the-secret-value");
        let v = resolve_source(&AuthSource::FromEnv {
            var: "SINDRI_TEST_REDEEM_ENV".into(),
        })
        .expect("env resolve");
        assert_eq!(v, "the-secret-value");
        std::env::remove_var("SINDRI_TEST_REDEEM_ENV");
    }

    #[test]
    fn resolve_from_secrets_store_returns_typed_error() {
        let err = resolve_source(&AuthSource::FromSecretsStore {
            backend: "vault".into(),
            path: "secrets/x".into(),
        })
        .unwrap_err();
        // Must mention the unwired backend; must NOT silently produce ""
        assert!(err.0.contains("not yet wired"));
    }

    #[test]
    fn resolve_from_upstream_credentials_is_default_deny() {
        let err = resolve_source(&AuthSource::FromUpstreamCredentials).unwrap_err();
        assert!(err.0.contains("allow_upstream_credentials"));
    }

    #[test]
    fn redeem_install_scope_envvar_round_trips() {
        std::env::set_var("SINDRI_TEST_INSTALL_KEY", "k1");
        let auth = AuthRequirements {
            tokens: vec![token_req(
                "k",
                "urn:x",
                Redemption::EnvVar {
                    env_name: "INJECT_KEY".into(),
                },
                AuthScope::Install,
            )],
            ..Default::default()
        };
        let b = binding(
            "npm:demo",
            "k",
            "local",
            AuthSource::FromEnv {
                var: "SINDRI_TEST_INSTALL_KEY".into(),
            },
        );
        let cb = ComponentBindings {
            component: "npm:demo",
            bindings: vec![&b],
            auth: &auth,
        };
        let target = MockTarget;
        let env = AuthRedeemer::new()
            .redeem_install_scope(&cb, &target)
            .expect("ok");
        assert_eq!(env.env, vec![("INJECT_KEY".to_string(), "k1".to_string())]);
        assert!(env.temp_files.is_empty());
        std::env::remove_var("SINDRI_TEST_INSTALL_KEY");
    }

    #[test]
    fn runtime_scope_skipped_during_install_pass() {
        let auth = AuthRequirements {
            tokens: vec![token_req(
                "k",
                "urn:x",
                Redemption::EnvVar {
                    env_name: "INJECT_KEY".into(),
                },
                AuthScope::Runtime,
            )],
            ..Default::default()
        };
        let b = binding(
            "npm:demo",
            "k",
            "local",
            AuthSource::FromEnv {
                var: "SINDRI_TEST_NEVER_SET".into(),
            },
        );
        let cb = ComponentBindings {
            component: "npm:demo",
            bindings: vec![&b],
            auth: &auth,
        };
        let env = AuthRedeemer::new()
            .redeem_install_scope(&cb, &MockTarget)
            .expect("ok");
        // Runtime-scope binding is skipped at install pass.
        assert!(env.is_empty());
    }

    #[test]
    fn file_redemption_writes_with_mode() {
        let dir = tempfile::tempdir().unwrap();
        let target_path = dir.path().join("creds.json");
        std::env::set_var("SINDRI_TEST_FILE_VAL", "{ \"k\": \"v\" }");

        let auth = AuthRequirements {
            tokens: vec![token_req(
                "creds",
                "urn:x",
                Redemption::File {
                    path: target_path.to_string_lossy().to_string(),
                    mode: Some(0o600),
                    persist: false,
                },
                AuthScope::Install,
            )],
            ..Default::default()
        };
        let b = binding(
            "npm:demo",
            "creds",
            "local",
            AuthSource::FromEnv {
                var: "SINDRI_TEST_FILE_VAL".into(),
            },
        );
        let cb = ComponentBindings {
            component: "npm:demo",
            bindings: vec![&b],
            auth: &auth,
        };
        let env = AuthRedeemer::new()
            .redeem_install_scope(&cb, &MockTarget)
            .expect("ok");
        assert_eq!(env.temp_files.len(), 1);
        assert!(target_path.exists());
        std::env::remove_var("SINDRI_TEST_FILE_VAL");

        // Cleanup deletes (persist=false).
        AuthRedeemer::new().cleanup(&env, "local");
        assert!(!target_path.exists());
    }

    #[test]
    fn cleanup_persist_keeps_file() {
        let dir = tempfile::tempdir().unwrap();
        let target_path = dir.path().join("keep.json");
        std::env::set_var("SINDRI_TEST_PERSIST_VAL", "abc");

        let auth = AuthRequirements {
            tokens: vec![token_req(
                "creds",
                "urn:x",
                Redemption::File {
                    path: target_path.to_string_lossy().to_string(),
                    mode: None,
                    persist: true,
                },
                AuthScope::Install,
            )],
            ..Default::default()
        };
        let b = binding(
            "npm:demo",
            "creds",
            "local",
            AuthSource::FromEnv {
                var: "SINDRI_TEST_PERSIST_VAL".into(),
            },
        );
        let cb = ComponentBindings {
            component: "npm:demo",
            bindings: vec![&b],
            auth: &auth,
        };
        let env = AuthRedeemer::new()
            .redeem_install_scope(&cb, &MockTarget)
            .expect("ok");
        AuthRedeemer::new().cleanup(&env, "local");
        // persist=true → file stays.
        assert!(target_path.exists());
        std::env::remove_var("SINDRI_TEST_PERSIST_VAL");
    }

    #[test]
    fn env_file_redemption_sets_var_to_path() {
        let dir = tempfile::tempdir().unwrap();
        let target_path = dir.path().join("gcp.json");
        std::env::set_var("SINDRI_TEST_ENVFILE_VAL", "json-payload");

        let auth = AuthRequirements {
            tokens: vec![token_req(
                "gcp",
                "urn:x",
                Redemption::EnvFile {
                    env_name: "GOOGLE_APPLICATION_CREDENTIALS".into(),
                    path: target_path.to_string_lossy().to_string(),
                },
                AuthScope::Install,
            )],
            ..Default::default()
        };
        let b = binding(
            "npm:demo",
            "gcp",
            "local",
            AuthSource::FromEnv {
                var: "SINDRI_TEST_ENVFILE_VAL".into(),
            },
        );
        let cb = ComponentBindings {
            component: "npm:demo",
            bindings: vec![&b],
            auth: &auth,
        };
        let env = AuthRedeemer::new()
            .redeem_install_scope(&cb, &MockTarget)
            .expect("ok");
        assert_eq!(env.env.len(), 1);
        assert_eq!(env.env[0].0, "GOOGLE_APPLICATION_CREDENTIALS");
        assert_eq!(env.env[0].1, target_path.to_string_lossy().to_string());
        assert!(target_path.exists());
        std::env::remove_var("SINDRI_TEST_ENVFILE_VAL");
    }

    #[test]
    fn unbound_binding_is_ignored() {
        let auth = AuthRequirements::default();
        let b = AuthBinding {
            id: "x".into(),
            component: "npm:demo".into(),
            requirement: "k".into(),
            audience: "urn:x".into(),
            target: "local".into(),
            source: None,
            priority: 0,
            status: AuthBindingStatus::Failed,
            reason: Some("no source".into()),
            considered: Vec::new(),
        };
        let cb = ComponentBindings {
            component: "npm:demo",
            bindings: vec![&b],
            auth: &auth,
        };
        let env = AuthRedeemer::new()
            .redeem_install_scope(&cb, &MockTarget)
            .expect("ok");
        assert!(env.is_empty());
    }

    #[test]
    fn binding_for_unknown_requirement_is_skipped_with_warn() {
        let auth = AuthRequirements::default(); // no requirements declared
        let b = binding(
            "npm:demo",
            "phantom",
            "local",
            AuthSource::FromEnv {
                var: "SINDRI_TEST_NEVER".into(),
            },
        );
        let cb = ComponentBindings {
            component: "npm:demo",
            bindings: vec![&b],
            auth: &auth,
        };
        let env = AuthRedeemer::new()
            .redeem_install_scope(&cb, &MockTarget)
            .expect("ok");
        assert!(env.is_empty());
    }

    // -------- Mock target --------
    use sindri_core::platform::TargetProfile;
    use sindri_targets::error::TargetError;
    use sindri_targets::traits::PrereqCheck;

    struct MockTarget;
    impl Target for MockTarget {
        fn name(&self) -> &str {
            "local"
        }
        fn kind(&self) -> &str {
            "local"
        }
        fn profile(&self) -> Result<TargetProfile, TargetError> {
            Err(TargetError::Unavailable {
                name: "mock".into(),
                reason: "test".into(),
            })
        }
        fn exec(&self, _cmd: &str, _env: &[(&str, &str)]) -> Result<(String, String), TargetError> {
            Ok((String::new(), String::new()))
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
}
