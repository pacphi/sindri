use crate::error::TargetError;
use crate::traits::{PrereqCheck, Target};
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
