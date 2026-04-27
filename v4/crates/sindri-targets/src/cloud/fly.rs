//! Fly.io target.
use crate::error::TargetError;
use crate::traits::{PrereqCheck, Target};
use sindri_core::platform::{Arch, Capabilities, Os, Platform, TargetProfile};
use std::path::Path;

/// Fly.io app target. Shells out to `flyctl`; auth is delegated to
/// `flyctl auth login` (the `target auth` wizard prints a hint for this).
pub struct FlyTarget {
    pub name: String,
    pub app_name: String,
    pub region: Option<String>,
}

impl FlyTarget {
    /// Construct a new Fly target with the given local name and Fly app slug.
    pub fn new(name: &str, app_name: &str) -> Self {
        FlyTarget {
            name: name.to_string(),
            app_name: app_name.to_string(),
            region: None,
        }
    }
}

impl Target for FlyTarget {
    fn name(&self) -> &str {
        &self.name
    }
    fn kind(&self) -> &str {
        "fly"
    }

    fn profile(&self) -> Result<TargetProfile, TargetError> {
        // Fly Machines run Linux on x86_64 or aarch64. Default to x86_64; a
        // future change can probe `flyctl machine list` for the actual arch.
        Ok(TargetProfile {
            platform: Platform {
                os: Os::Linux,
                arch: Arch::X86_64,
            },
            capabilities: Capabilities::default(),
        })
    }

    fn exec(&self, cmd: &str, _env: &[(&str, &str)]) -> Result<(String, String), TargetError> {
        let output = std::process::Command::new("flyctl")
            .args(["ssh", "console", "--app", &self.app_name, "--command", cmd])
            .output()
            .map_err(|e| TargetError::Prerequisites {
                target: self.name.clone(),
                detail: format!("flyctl not found: {}", e),
            })?;
        Ok((
            String::from_utf8_lossy(&output.stdout).to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }

    fn upload(&self, _local: &Path, _remote: &str) -> Result<(), TargetError> {
        Err(TargetError::Unavailable {
            name: self.name.clone(),
            reason: "use flyctl deploy for file transfer".into(),
        })
    }

    fn download(&self, _remote: &str, _local: &Path) -> Result<(), TargetError> {
        Err(TargetError::Unavailable {
            name: self.name.clone(),
            reason: "use flyctl ssh sftp for downloads".into(),
        })
    }

    fn create(&self) -> Result<(), TargetError> {
        std::process::Command::new("flyctl")
            .args(["apps", "create", &self.app_name, "--json"])
            .status()
            .map_err(|e| TargetError::Prerequisites {
                target: self.name.clone(),
                detail: e.to_string(),
            })?;
        Ok(())
    }

    fn check_prerequisites(&self) -> Vec<PrereqCheck> {
        vec![if crate::traits::which("flyctl").is_some() {
            PrereqCheck::ok("flyctl CLI")
        } else {
            PrereqCheck::fail("flyctl CLI", "curl -L https://fly.io/install.sh | sh")
        }]
    }
}
