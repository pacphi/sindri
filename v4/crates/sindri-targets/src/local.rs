use crate::error::TargetError;
use crate::traits::{PrereqCheck, Target};
use crate::well_known;
use sindri_core::auth::{AuthCapability, AuthSource};
use sindri_core::platform::{Capabilities, Platform, TargetProfile};
use std::path::Path;

/// Local machine target — the implicit default (ADR-023)
pub struct LocalTarget {
    name: String,
}

impl Default for LocalTarget {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalTarget {
    pub fn new() -> Self {
        LocalTarget {
            name: "local".to_string(),
        }
    }

    pub fn named(name: &str) -> Self {
        LocalTarget {
            name: name.to_string(),
        }
    }
}

impl Target for LocalTarget {
    fn name(&self) -> &str {
        &self.name
    }

    fn kind(&self) -> &str {
        "local"
    }

    fn profile(&self) -> Result<TargetProfile, TargetError> {
        let platform = Platform::current();
        let caps = detect_capabilities();
        Ok(TargetProfile {
            platform,
            capabilities: caps,
        })
    }

    fn exec(&self, cmd: &str, env: &[(&str, &str)]) -> Result<(String, String), TargetError> {
        let mut command = std::process::Command::new("sh");
        command.arg("-c").arg(cmd);
        for (k, v) in env {
            command.env(k, v);
        }
        let output = command.output().map_err(|e| TargetError::ExecFailed {
            target: self.name.clone(),
            detail: e.to_string(),
        })?;
        Ok((
            String::from_utf8_lossy(&output.stdout).to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }

    fn upload(&self, local: &Path, remote: &str) -> Result<(), TargetError> {
        std::fs::copy(local, remote)?;
        Ok(())
    }

    fn download(&self, remote: &str, local: &Path) -> Result<(), TargetError> {
        std::fs::copy(remote, local)?;
        Ok(())
    }

    fn check_prerequisites(&self) -> Vec<PrereqCheck> {
        vec![PrereqCheck::ok("local shell (sh)")]
    }

    /// Advertise ambient credentials available to the local user (ADR-027 §1,
    /// Phase 4 of the auth-aware plan).
    ///
    /// Three classes of capability are surfaced:
    /// 1. **Well-known env vars** — anything in [`well_known::TABLE`] that is
    ///    currently set. Priority `10`.
    /// 2. **`gh` CLI delegation** — if `gh` is on `PATH` we advertise
    ///    `cli:gh auth token` as a `github_token` source for the GitHub API
    ///    audience. Priority `20` so a logged-in `gh` beats a stale
    ///    `GITHUB_TOKEN` env-var.
    ///
    /// All checks are lexical (`PATH` / `env::var`) — no subprocesses are
    /// spawned, so this is safe on the resolver's hot path.
    fn auth_capabilities(&self) -> Vec<AuthCapability> {
        let mut caps = well_known::ambient_env_capabilities(10);

        if crate::traits::which("gh").is_some() {
            caps.push(AuthCapability {
                id: "github_token".to_string(),
                audience: "https://api.github.com".to_string(),
                source: AuthSource::FromCli {
                    command: "gh auth token".to_string(),
                },
                priority: 20,
            });
        }

        caps
    }
}

fn detect_capabilities() -> Capabilities {
    let system_pm = detect_system_pm();
    let has_docker = crate::traits::which("docker").is_some();
    let has_sudo = crate::traits::which("sudo").is_some();
    let shell = std::env::var("SHELL").ok();
    Capabilities {
        system_package_manager: system_pm,
        has_docker,
        has_sudo,
        shell,
    }
}

fn detect_system_pm() -> Option<String> {
    for pm in &["apt-get", "dnf", "zypper", "pacman", "apk", "brew"] {
        if crate::traits::which(pm).is_some() {
            return Some(pm.to_string());
        }
    }
    None
}

// =============================================================================
// Tests — auth_capabilities
// =============================================================================

#[cfg(test)]
mod auth_cap_tests {
    use super::*;
    use crate::well_known::ENV_LOCK;
    use std::fs;

    /// All known well-known env-vars, used to scrub ambient state for tests.
    const KNOWN_VARS: &[&str] = &[
        "ANTHROPIC_API_KEY",
        "OPENAI_API_KEY",
        "GEMINI_API_KEY",
        "GOOGLE_API_KEY",
        "GROQ_API_KEY",
        "MISTRAL_API_KEY",
        "COHERE_API_KEY",
        "GITHUB_TOKEN",
        "GH_TOKEN",
        "GITLAB_TOKEN",
        "HF_TOKEN",
        "HUGGING_FACE_HUB_TOKEN",
    ];

    fn clear_known_vars() {
        for v in KNOWN_VARS {
            // SAFETY: caller holds ENV_LOCK.
            unsafe { std::env::remove_var(v) };
        }
    }

    /// Create a temp dir containing a file named `name` and return the dir.
    /// Production `traits::which()` only checks `is_file()`, so the file does
    /// not need to be executable — this keeps the helper cross-platform without
    /// platform-specific permissions or extension handling. Caller is
    /// responsible for cleanup.
    fn fake_bin_dir(name: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let bin = dir.path().join(name);
        fs::write(&bin, b"").unwrap();
        dir
    }

    #[test]
    fn no_gh_no_env_yields_empty_caps() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_known_vars();
        // SAFETY: caller holds ENV_LOCK.
        unsafe { std::env::set_var("PATH", "/nonexistent-sindri-path-xyz") };

        let target = LocalTarget::new();
        let caps = target.auth_capabilities();

        assert!(
            caps.is_empty(),
            "expected no capabilities with empty PATH and no env vars, got {:?}",
            caps
        );
    }

    #[test]
    fn gh_on_path_advertises_cli_capability() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_known_vars();
        let dir = fake_bin_dir("gh");
        // SAFETY: caller holds ENV_LOCK.
        unsafe { std::env::set_var("PATH", dir.path()) };

        let target = LocalTarget::new();
        let caps = target.auth_capabilities();

        let gh = caps
            .iter()
            .find(|c| matches!(&c.source, AuthSource::FromCli { command } if command == "gh auth token"))
            .expect("expected gh CLI capability");
        assert_eq!(gh.id, "github_token");
        assert_eq!(gh.audience, "https://api.github.com");
        assert_eq!(gh.priority, 20);
    }

    #[test]
    fn gh_absent_omits_cli_capability() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_known_vars();
        // SAFETY: caller holds ENV_LOCK.
        unsafe { std::env::set_var("PATH", "/nonexistent-sindri-path-xyz") };

        let target = LocalTarget::new();
        let caps = target.auth_capabilities();

        assert!(
            caps.iter()
                .all(|c| !matches!(&c.source, AuthSource::FromCli { .. })),
            "did not expect any CLI capability, got {:?}",
            caps
        );
    }

    #[test]
    fn ambient_env_var_advertised() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_known_vars();
        // SAFETY: caller holds ENV_LOCK.
        unsafe { std::env::set_var("PATH", "/nonexistent-sindri-path-xyz") };
        unsafe { std::env::set_var("ANTHROPIC_API_KEY", "sk-test") };

        let target = LocalTarget::new();
        let caps = target.auth_capabilities();
        // SAFETY: caller holds ENV_LOCK.
        unsafe { std::env::remove_var("ANTHROPIC_API_KEY") };

        let cap = caps
            .iter()
            .find(|c| c.id == "anthropic_api_key")
            .expect("expected anthropic capability");
        assert_eq!(cap.audience, "urn:anthropic:api");
        match &cap.source {
            AuthSource::FromEnv { var } => assert_eq!(var, "ANTHROPIC_API_KEY"),
            other => panic!("expected FromEnv, got {:?}", other),
        }
    }
}
