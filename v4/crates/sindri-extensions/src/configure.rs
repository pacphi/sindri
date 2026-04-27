//! Configure executor (DDD-01 §Configure, ADR-024).
//!
//! [`ConfigureExecutor`] applies a component's
//! [`sindri_core::component::ConfigureConfig`] after a successful install:
//!
//! 1. **Environment variables** — for each [`EnvSetting`] with
//!    [`EnvScope::ShellRc`], a per-component fragment is written to
//!    `<env-dir>/<component>.sh` with `export NAME="value"` lines. A guarded
//!    block in the user's `~/.bashrc` and `~/.zshrc` source-globs the
//!    fragment dir on next interactive shell. Other scopes (Login, Session,
//!    UserEnvVar) emit a `tracing::warn!` and are skipped — implementation-plan
//!    §5.5 will introduce the full PATH/scope abstraction in a follow-up.
//!
//! 2. **File templates** — for each [`FileTemplate`], the inline body is
//!    Mustache-style `{{var}}`-substituted from a small context (component
//!    name, version, target name, plus the configure-pass env settings) and
//!    written to `path` (with leading `~` expanded). When `overwrite: false`
//!    the executor preserves an existing file with a `tracing::warn!`.
//!
//! ### Idempotency
//!
//! Re-running the executor with the same inputs produces the same on-disk
//! state with no error. The shell-rc guard block uses a `# sindri:auto`
//! marker so it is appended at most once.
//!
//! ### Failure surface
//!
//! Real I/O or template parse errors return [`ExtensionError::ConfigureFailed`]
//! with the offending sub-step (e.g. `environment[FOO]`, `files[/etc/x.conf]`).

use crate::error::ExtensionError;
use sindri_core::component::{ConfigureConfig, EnvScope, EnvSetting, FileTemplate};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Marker placed in `~/.bashrc` / `~/.zshrc` so the source-glob block is
/// appended at most once even across re-runs.
const SHELL_RC_MARKER: &str = "# sindri:auto";

/// Context for a configure run.
pub struct ConfigureContext<'a> {
    /// Component metadata name (e.g. `"nodejs"`).
    pub component: &'a str,
    /// Component metadata version (e.g. `"22.0.0"`).
    pub version: &'a str,
    /// The execution target name (e.g. `"local"`).
    pub target_name: &'a str,
    /// Directory used for per-component shell-rc env fragments.
    ///
    /// Production callers typically pass `~/.sindri/env`; tests pass a
    /// [`tempfile::TempDir`] path. Created (recursively) on demand.
    pub env_dir: &'a Path,
    /// Directory used as the base for `~`-expansion of [`FileTemplate::path`]
    /// and the rc-file location for the source-glob guard. Production callers
    /// pass `dirs_next::home_dir()`; tests pass a temp dir so the rc files
    /// land inside the sandbox.
    pub home_dir: &'a Path,
}

/// Capability executor for `configure` (ADR-024).
#[derive(Debug, Default, Clone, Copy)]
pub struct ConfigureExecutor;

impl ConfigureExecutor {
    /// Create a new executor.
    pub fn new() -> Self {
        Self
    }

    /// Apply the full [`ConfigureConfig`] for a component.
    ///
    /// This is idempotent — running twice produces the same state with no
    /// error.
    pub async fn apply(
        &self,
        cfg: &ConfigureConfig,
        ctx: &ConfigureContext<'_>,
    ) -> Result<(), ExtensionError> {
        // 1. Environment fragment.
        self.apply_environment(cfg, ctx)?;

        // 2. File templates.
        self.apply_files(cfg, ctx)?;

        Ok(())
    }

    /// Write a per-component shell-rc env fragment, source-globbed via a
    /// guarded block in `~/.bashrc` and `~/.zshrc`.
    fn apply_environment(
        &self,
        cfg: &ConfigureConfig,
        ctx: &ConfigureContext<'_>,
    ) -> Result<(), ExtensionError> {
        // Partition by scope.
        let mut shell_rc_settings: Vec<&EnvSetting> = Vec::new();
        for setting in &cfg.environment {
            match &setting.scope {
                EnvScope::ShellRc => shell_rc_settings.push(setting),
                other => {
                    tracing::warn!(
                        component = ctx.component,
                        env_var = setting.name.as_str(),
                        scope = ?other,
                        "scope `{:?}` not yet supported on this platform; \
                         skipping (see implementation-plan §5.5)",
                        other
                    );
                }
            }
        }

        if shell_rc_settings.is_empty() && cfg.environment.is_empty() {
            return Ok(());
        }

        // Write the per-component fragment even if empty (idempotent).
        let fragment_path = ctx.env_dir.join(format!("{}.sh", ctx.component));
        std::fs::create_dir_all(ctx.env_dir).map_err(|e| ExtensionError::ConfigureFailed {
            component: ctx.component.to_string(),
            step: format!("env_dir({})", ctx.env_dir.display()),
            detail: e.to_string(),
        })?;

        let mut body = String::new();
        body.push_str(&format!(
            "# sindri-managed env fragment for component `{}`\n\
             # Re-run `sindri apply` to refresh.\n",
            ctx.component
        ));
        for s in &shell_rc_settings {
            body.push_str(&format!(
                "export {}=\"{}\"\n",
                s.name,
                shell_escape_double_quoted(&s.value)
            ));
        }

        std::fs::write(&fragment_path, body).map_err(|e| ExtensionError::ConfigureFailed {
            component: ctx.component.to_string(),
            step: format!("write fragment ({})", fragment_path.display()),
            detail: e.to_string(),
        })?;

        // Append source-glob guard to bashrc/zshrc (at most once).
        for rc_name in &[".bashrc", ".zshrc"] {
            let rc_path = ctx.home_dir.join(rc_name);
            ensure_shell_rc_block(&rc_path, ctx.env_dir, ctx.component)?;
        }

        Ok(())
    }

    /// Render and write all [`FileTemplate`]s.
    fn apply_files(
        &self,
        cfg: &ConfigureConfig,
        ctx: &ConfigureContext<'_>,
    ) -> Result<(), ExtensionError> {
        if cfg.files.is_empty() {
            return Ok(());
        }
        let vars = build_template_vars(cfg, ctx);
        for template in &cfg.files {
            apply_file_template(template, ctx, &vars)?;
        }
        Ok(())
    }
}

/// Idempotently append a guarded block to `rc_path` so interactive shells
/// source the per-component env fragments. The block is wrapped between two
/// `# sindri:auto` marker lines and is appended at most once per rc file.
fn ensure_shell_rc_block(
    rc_path: &Path,
    env_dir: &Path,
    component: &str,
) -> Result<(), ExtensionError> {
    let existing = std::fs::read_to_string(rc_path).unwrap_or_default();
    if existing.contains(SHELL_RC_MARKER) {
        return Ok(());
    }

    if let Some(parent) = rc_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| ExtensionError::ConfigureFailed {
            component: component.to_string(),
            step: format!("rc parent({})", parent.display()),
            detail: e.to_string(),
        })?;
    }

    // Use ~/.sindri/env literally in the rendered rc text where possible —
    // we render the resolved env_dir display path so tests get an absolute
    // path and production gets `~/.sindri/env` (callers should pass that).
    let block = format!(
        "\n{marker}\n\
         # Sindri-managed env fragments. Edit via `sindri apply`; do not modify by hand.\n\
         if [ -d \"{env}\" ]; then\n\
         \tfor _f in \"{env}\"/*.sh; do\n\
         \t\t[ -r \"$_f\" ] && . \"$_f\"\n\
         \tdone\n\
         \tunset _f\n\
         fi\n\
         {marker}\n",
        marker = SHELL_RC_MARKER,
        env = env_dir.display(),
    );

    let mut body = existing;
    body.push_str(&block);
    std::fs::write(rc_path, body).map_err(|e| ExtensionError::ConfigureFailed {
        component: component.to_string(),
        step: format!("append rc({})", rc_path.display()),
        detail: e.to_string(),
    })?;
    Ok(())
}

/// Build the variable map used for `{{var}}` substitution.
fn build_template_vars(
    cfg: &ConfigureConfig,
    ctx: &ConfigureContext<'_>,
) -> HashMap<String, String> {
    let mut vars: HashMap<String, String> = HashMap::new();
    vars.insert("component.name".into(), ctx.component.into());
    vars.insert("component".into(), ctx.component.into());
    vars.insert("version".into(), ctx.version.into());
    vars.insert("target".into(), ctx.target_name.into());
    vars.insert("target.name".into(), ctx.target_name.into());
    for s in &cfg.environment {
        vars.insert(format!("env.{}", s.name), s.value.clone());
    }
    vars
}

/// Render and write a single [`FileTemplate`].
fn apply_file_template(
    template: &FileTemplate,
    ctx: &ConfigureContext<'_>,
    vars: &HashMap<String, String>,
) -> Result<(), ExtensionError> {
    let dest = expand_tilde(&template.path, ctx.home_dir);

    if dest.exists() && !template.overwrite {
        tracing::warn!(
            component = ctx.component,
            path = %dest.display(),
            "configure: file exists and overwrite=false; preserving"
        );
        return Ok(());
    }

    let rendered = render_mustache(&template.template, vars).map_err(|e| {
        ExtensionError::ConfigureFailed {
            component: ctx.component.to_string(),
            step: format!("files[{}]", dest.display()),
            detail: e,
        }
    })?;

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| ExtensionError::ConfigureFailed {
            component: ctx.component.to_string(),
            step: format!("files[{}] parent", dest.display()),
            detail: e.to_string(),
        })?;
    }
    std::fs::write(&dest, rendered).map_err(|e| ExtensionError::ConfigureFailed {
        component: ctx.component.to_string(),
        step: format!("files[{}]", dest.display()),
        detail: e.to_string(),
    })?;
    Ok(())
}

/// Expand a leading `~` or `~/` against `home_dir`.
fn expand_tilde(path: &str, home_dir: &Path) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        return home_dir.join(stripped);
    }
    if path == "~" {
        return home_dir.to_path_buf();
    }
    PathBuf::from(path)
}

/// Tiny Mustache-style substitutor: replaces `{{key}}` (with optional
/// surrounding whitespace) with `vars[key]`. An unknown key returns an error
/// so misspelled placeholders surface during apply rather than silently
/// rendering blank.
fn render_mustache(input: &str, vars: &HashMap<String, String>) -> Result<String, String> {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let end = after
            .find("}}")
            .ok_or_else(|| "unclosed `{{` in template".to_string())?;
        let key = after[..end].trim();
        let value = vars
            .get(key)
            .ok_or_else(|| format!("unknown template variable `{}`", key))?;
        out.push_str(value);
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

/// Escape a value for inclusion inside a double-quoted shell string.
/// Conservative: backslashes the four characters that have meaning inside
/// `"..."` for POSIX shells.
fn shell_escape_double_quoted(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\\' | '"' | '$' | '`' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use sindri_core::component::EnvSetting;
    use tempfile::TempDir;

    fn ctx<'a>(env: &'a Path, home: &'a Path) -> ConfigureContext<'a> {
        ConfigureContext {
            component: "nodejs",
            version: "22.5.1",
            target_name: "local",
            env_dir: env,
            home_dir: home,
        }
    }

    #[tokio::test]
    async fn env_setting_writes_per_component_fragment() {
        let env_tmp = TempDir::new().unwrap();
        let home_tmp = TempDir::new().unwrap();
        let cfg = ConfigureConfig {
            environment: vec![EnvSetting {
                name: "NODE_PATH".into(),
                value: "/opt/node/lib".into(),
                scope: EnvScope::ShellRc,
            }],
            files: Vec::new(),
        };
        ConfigureExecutor::new()
            .apply(&cfg, &ctx(env_tmp.path(), home_tmp.path()))
            .await
            .expect("apply ok");

        let fragment = env_tmp.path().join("nodejs.sh");
        let body = std::fs::read_to_string(&fragment).expect("fragment exists");
        assert!(body.contains("export NODE_PATH=\"/opt/node/lib\""));

        // Both rc files should contain the guard block.
        let bashrc = std::fs::read_to_string(home_tmp.path().join(".bashrc")).unwrap();
        assert!(bashrc.contains(SHELL_RC_MARKER));
        let zshrc = std::fs::read_to_string(home_tmp.path().join(".zshrc")).unwrap();
        assert!(zshrc.contains(SHELL_RC_MARKER));
    }

    #[tokio::test]
    async fn rerun_does_not_double_append_rc_block() {
        let env_tmp = TempDir::new().unwrap();
        let home_tmp = TempDir::new().unwrap();
        let cfg = ConfigureConfig {
            environment: vec![EnvSetting {
                name: "FOO".into(),
                value: "bar".into(),
                scope: EnvScope::ShellRc,
            }],
            files: Vec::new(),
        };
        let c = ctx(env_tmp.path(), home_tmp.path());
        ConfigureExecutor::new().apply(&cfg, &c).await.unwrap();
        ConfigureExecutor::new().apply(&cfg, &c).await.unwrap();
        let bashrc = std::fs::read_to_string(home_tmp.path().join(".bashrc")).unwrap();
        // Marker pair: opening + closing per block, exactly one block → 2 occurrences.
        assert_eq!(bashrc.matches(SHELL_RC_MARKER).count(), 2);
    }

    #[tokio::test]
    async fn file_template_substitutes_vars() {
        let env_tmp = TempDir::new().unwrap();
        let home_tmp = TempDir::new().unwrap();
        let cfg = ConfigureConfig {
            environment: Vec::new(),
            files: vec![FileTemplate {
                path: "~/.config/nodejs/info.txt".into(),
                template: "name={{component.name}} v={{version}} target={{target}}".into(),
                overwrite: true,
            }],
        };
        ConfigureExecutor::new()
            .apply(&cfg, &ctx(env_tmp.path(), home_tmp.path()))
            .await
            .expect("apply ok");
        let dest = home_tmp.path().join(".config/nodejs/info.txt");
        let body = std::fs::read_to_string(&dest).expect("file exists");
        assert_eq!(body, "name=nodejs v=22.5.1 target=local");
    }

    #[tokio::test]
    async fn overwrite_false_preserves_existing() {
        let env_tmp = TempDir::new().unwrap();
        let home_tmp = TempDir::new().unwrap();
        let dest = home_tmp.path().join(".config/keep.txt");
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
        std::fs::write(&dest, "ORIGINAL").unwrap();

        let cfg = ConfigureConfig {
            environment: Vec::new(),
            files: vec![FileTemplate {
                path: "~/.config/keep.txt".into(),
                template: "REPLACED".into(),
                overwrite: false,
            }],
        };
        ConfigureExecutor::new()
            .apply(&cfg, &ctx(env_tmp.path(), home_tmp.path()))
            .await
            .expect("apply ok");
        let body = std::fs::read_to_string(&dest).unwrap();
        assert_eq!(body, "ORIGINAL", "existing file must be preserved");
    }

    #[tokio::test]
    async fn unknown_template_var_errors() {
        let env_tmp = TempDir::new().unwrap();
        let home_tmp = TempDir::new().unwrap();
        let cfg = ConfigureConfig {
            environment: Vec::new(),
            files: vec![FileTemplate {
                path: "~/oops.txt".into(),
                template: "{{nope}}".into(),
                overwrite: true,
            }],
        };
        let err = ConfigureExecutor::new()
            .apply(&cfg, &ctx(env_tmp.path(), home_tmp.path()))
            .await
            .expect_err("unknown var must fail");
        match err {
            ExtensionError::ConfigureFailed { component, .. } => assert_eq!(component, "nodejs"),
            other => panic!("expected ConfigureFailed, got {other:?}"),
        }
    }

    #[test]
    fn shell_escape_handles_quotes_and_dollars() {
        assert_eq!(shell_escape_double_quoted(r#"a"b$c\d`e"#), r#"a\"b\$c\\d\`e"#);
    }
}
