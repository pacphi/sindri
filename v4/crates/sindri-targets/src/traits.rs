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
use sindri_core::platform::TargetProfile;

pub trait Target: Send + Sync {
    /// Human-readable name of this target instance
    fn name(&self) -> &str;

    /// The kind of target (local, docker, ssh, e2b, fly, ...)
    fn kind(&self) -> &str;

    /// Detect or return the platform/capabilities of this target
    fn profile(&self) -> Result<TargetProfile, TargetError>;

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
