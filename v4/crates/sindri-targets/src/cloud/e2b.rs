//! E2B Sandbox target.
use crate::error::TargetError;
use crate::traits::{PrereqCheck, Target};
use sindri_core::platform::{Arch, Capabilities, Os, Platform, TargetProfile};
use std::path::Path;

/// E2B sandbox target. Shells out to the `e2b` CLI; auth is delegated to the
/// CLI's own keyring (`e2b auth login`).
pub struct E2bTarget {
    pub name: String,
    pub template: String,
    pub sandbox_id: Option<String>,
}

impl E2bTarget {
    /// Construct a new E2B target with the given local name and sandbox template.
    pub fn new(name: &str, template: &str) -> Self {
        E2bTarget {
            name: name.to_string(),
            template: template.to_string(),
            sandbox_id: None,
        }
    }
}

impl Target for E2bTarget {
    fn name(&self) -> &str {
        &self.name
    }
    fn kind(&self) -> &str {
        "e2b"
    }

    fn profile(&self) -> Result<TargetProfile, TargetError> {
        // E2B sandboxes are uniformly Linux x86_64 today.
        Ok(TargetProfile {
            platform: Platform {
                os: Os::Linux,
                arch: Arch::X86_64,
            },
            capabilities: Capabilities {
                system_package_manager: Some("apt-get".into()),
                has_docker: false,
                has_sudo: true,
                shell: Some("/bin/bash".into()),
            },
        })
    }

    fn exec(&self, cmd: &str, _env: &[(&str, &str)]) -> Result<(String, String), TargetError> {
        let output = std::process::Command::new("e2b")
            .args([
                "sandbox",
                "exec",
                "--sandbox",
                self.sandbox_id.as_deref().unwrap_or(""),
                "--",
                "sh",
                "-c",
                cmd,
            ])
            .output()
            .map_err(|e| TargetError::Prerequisites {
                target: self.name.clone(),
                detail: format!("e2b CLI not found: {}", e),
            })?;
        Ok((
            String::from_utf8_lossy(&output.stdout).to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }

    fn upload(&self, _local: &Path, _remote: &str) -> Result<(), TargetError> {
        Err(TargetError::Unavailable {
            name: self.name.clone(),
            reason: "upload via e2b CLI not yet implemented".into(),
        })
    }

    fn download(&self, _remote: &str, _local: &Path) -> Result<(), TargetError> {
        Err(TargetError::Unavailable {
            name: self.name.clone(),
            reason: "download via e2b CLI not yet implemented".into(),
        })
    }

    fn create(&self) -> Result<(), TargetError> {
        std::process::Command::new("e2b")
            .args(["sandbox", "create", "--template", &self.template])
            .status()
            .map_err(|e| TargetError::Prerequisites {
                target: self.name.clone(),
                detail: e.to_string(),
            })?;
        Ok(())
    }

    fn check_prerequisites(&self) -> Vec<PrereqCheck> {
        vec![if crate::traits::which("e2b").is_some() {
            PrereqCheck::ok("e2b CLI")
        } else {
            PrereqCheck::fail("e2b CLI", "npm install -g @e2b/cli")
        }]
    }
}
