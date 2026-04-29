/// Check if a binary is available in PATH
pub fn which(name: &str) -> Option<std::path::PathBuf> {
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

use crate::error::TargetError;
/// The Target trait — replaces Provider from v3 (ADR-017)
use sindri_core::auth::AuthCapability;
use sindri_core::platform::TargetProfile;

pub trait Target: Send + Sync {
    /// Human-readable name of this target instance
    fn name(&self) -> &str;

    /// The kind of target (local, docker, ssh, e2b, fly, ...)
    fn kind(&self) -> &str;

    /// Detect or return the platform/capabilities of this target
    fn profile(&self) -> Result<TargetProfile, TargetError>;

    /// Describe the credential slots this target can fulfill (ADR-027 §1).
    ///
    /// Default: empty — targets opt in. Phase 4 of the auth-aware
    /// implementation plan fills these in for built-in targets (`local`,
    /// `docker`, `ssh`, ...). The resolver's auth-binding pass walks this
    /// list (plus per-target `provides:` overrides from the BOM manifest)
    /// to discover candidate sources for each component-declared
    /// [`AuthRequirement`](sindri_core::auth::AuthRequirements).
    fn auth_capabilities(&self) -> Vec<AuthCapability> {
        Vec::new()
    }

    /// Execute a shell command on the target, return (stdout, stderr)
    fn exec(&self, cmd: &str, env: &[(&str, &str)]) -> Result<(String, String), TargetError>;

    /// Upload a local file to the target
    fn upload(&self, local: &std::path::Path, remote: &str) -> Result<(), TargetError>;

    /// Download a file from the target to local
    fn download(&self, remote: &str, local: &std::path::Path) -> Result<(), TargetError>;

    /// Provision the target resource (create container, start VM, etc.)
    fn create(&self) -> Result<(), TargetError> {
        Err(TargetError::Unavailable {
            name: self.name().to_string(),
            reason: "create not supported for this target kind".into(),
        })
    }

    /// Destroy the target resource
    fn destroy(&self) -> Result<(), TargetError> {
        Err(TargetError::Unavailable {
            name: self.name().to_string(),
            reason: "destroy not supported for this target kind".into(),
        })
    }

    /// Start a previously-created target resource.
    ///
    /// Default impl returns `Unavailable` so kinds without an
    /// independent run-time lifecycle (the host machine, for example)
    /// can simply opt out.
    fn start(&self) -> Result<(), TargetError> {
        Err(TargetError::Unavailable {
            name: self.name().to_string(),
            reason: "start not supported for this target kind".into(),
        })
    }

    /// Stop a target resource without destroying it.
    fn stop(&self) -> Result<(), TargetError> {
        Err(TargetError::Unavailable {
            name: self.name().to_string(),
            reason: "stop not supported for this target kind".into(),
        })
    }

    /// Check prerequisites (docker installed, ssh key exists, etc.)
    fn check_prerequisites(&self) -> Vec<PrereqCheck>;

    /// Prompt for an interactive credential value (Phase 2A of the
    /// auth-aware plan, ADR-027 §6 / §"Open Questions Q2").
    ///
    /// Default impl reads from the local process's stdin — appropriate for
    /// the local target. Remote / cloud targets override to forward the
    /// prompt over their plugin's RPC channel so the user sees it in their
    /// target session, not on the operator's terminal.
    ///
    /// `secret == true` means "do not echo the input"; the default impl
    /// uses [`rpassword`-style behaviour by reading without echoing] when
    /// possible and falls back to a plain read otherwise.
    ///
    /// `timeout_secs` of 0 means "block indefinitely". The default impl
    /// honours the timeout best-effort (full enforcement requires a
    /// per-target raw-tty capability and may be a no-op on non-TTY stdin).
    fn prompt_for_credential(
        &self,
        prompt: &str,
        _secret: bool,
        _timeout_secs: u64,
    ) -> Result<String, TargetError> {
        // Default: echo prompt to stderr and read one line from stdin. This
        // is the local-target behaviour; remote targets override.
        use std::io::{BufRead, Write};
        let stderr = std::io::stderr();
        let mut h = stderr.lock();
        let _ = write!(h, "{prompt}");
        let _ = h.flush();
        let mut line = String::new();
        let stdin = std::io::stdin();
        let mut g = stdin.lock();
        g.read_line(&mut line)
            .map_err(|e| TargetError::AuthFailed {
                target: self.name().to_string(),
                detail: format!("stdin read failed: {e}"),
            })?;
        Ok(line.trim_end_matches(['\r', '\n']).to_string())
    }
}

#[derive(Debug)]
pub struct PrereqCheck {
    pub name: String,
    pub passed: bool,
    pub fix: Option<String>,
}

impl PrereqCheck {
    pub fn ok(name: &str) -> Self {
        PrereqCheck {
            name: name.to_string(),
            passed: true,
            fix: None,
        }
    }

    pub fn fail(name: &str, fix: &str) -> Self {
        PrereqCheck {
            name: name.to_string(),
            passed: false,
            fix: Some(fix.to_string()),
        }
    }
}
