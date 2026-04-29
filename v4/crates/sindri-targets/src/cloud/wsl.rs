//! Windows Subsystem for Linux target.
//!
//! WSL is auto-detected only when the host OS is Windows. On any other OS
//! `supports()` returns `false` and the factory in `lib.rs` will refuse
//! to construct one. This avoids surprising errors for callers who write
//! `kind: wsl` in a manifest applied on macOS or Linux.
use crate::error::TargetError;
use crate::traits::{PrereqCheck, Target};
use sindri_core::platform::{Arch, Capabilities, Os, Platform, TargetProfile};
use std::path::Path;

/// A WSL distribution target.
pub struct WslTarget {
    pub name: String,
    /// WSL distribution name, e.g. `Ubuntu-22.04` or `Debian`.
    pub distro: String,
}

impl WslTarget {
    /// Construct a new WSL target.
    pub fn new(name: &str, distro: &str) -> Self {
        WslTarget {
            name: name.to_string(),
            distro: distro.to_string(),
        }
    }

    /// Returns true only when running on Windows.
    pub fn supports() -> bool {
        cfg!(target_os = "windows")
    }
}

impl Target for WslTarget {
    fn name(&self) -> &str {
        &self.name
    }
    fn kind(&self) -> &str {
        "wsl"
    }

    fn profile(&self) -> Result<TargetProfile, TargetError> {
        if !Self::supports() {
            return Err(TargetError::Unavailable {
                name: self.name.clone(),
                reason: "WSL targets are only supported on Windows hosts".into(),
            });
        }
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
        if !Self::supports() {
            return Err(TargetError::Unavailable {
                name: self.name.clone(),
                reason: "WSL exec requires a Windows host".into(),
            });
        }
        let output = std::process::Command::new("wsl")
            .args(["-d", &self.distro, "-e", "sh", "-c", cmd])
            .output()
            .map_err(|e| TargetError::Prerequisites {
                target: self.name.clone(),
                detail: format!("wsl.exe not found: {}", e),
            })?;
        Ok((
            String::from_utf8_lossy(&output.stdout).to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }

    fn upload(&self, local: &Path, remote: &str) -> Result<(), TargetError> {
        // WSL exposes the Windows filesystem under `/mnt/c/...`; copying
        // host→guest is just a `cp` once we know the WSL mount path.
        if !Self::supports() {
            return Err(TargetError::Unavailable {
                name: self.name.clone(),
                reason: "WSL upload requires a Windows host".into(),
            });
        }
        let cmd = format!("cp {} {}", local.to_string_lossy(), remote);
        let _ = self.exec(&cmd, &[])?;
        Ok(())
    }

    fn download(&self, remote: &str, local: &Path) -> Result<(), TargetError> {
        if !Self::supports() {
            return Err(TargetError::Unavailable {
                name: self.name.clone(),
                reason: "WSL download requires a Windows host".into(),
            });
        }
        let cmd = format!("cp {} {}", remote, local.to_string_lossy());
        let _ = self.exec(&cmd, &[])?;
        Ok(())
    }

    fn create(&self) -> Result<(), TargetError> {
        if !Self::supports() {
            return Err(TargetError::Unavailable {
                name: self.name.clone(),
                reason: "WSL create requires a Windows host".into(),
            });
        }
        std::process::Command::new("wsl")
            .args(["--install", "--distribution", &self.distro])
            .status()
            .map_err(|e| TargetError::Prerequisites {
                target: self.name.clone(),
                detail: e.to_string(),
            })?;
        Ok(())
    }

    fn destroy(&self) -> Result<(), TargetError> {
        if !Self::supports() {
            return Err(TargetError::Unavailable {
                name: self.name.clone(),
                reason: "WSL destroy requires a Windows host".into(),
            });
        }
        std::process::Command::new("wsl")
            .args(["--unregister", &self.distro])
            .status()
            .map_err(|e| TargetError::Prerequisites {
                target: self.name.clone(),
                detail: e.to_string(),
            })?;
        Ok(())
    }

    fn check_prerequisites(&self) -> Vec<PrereqCheck> {
        if !Self::supports() {
            return vec![PrereqCheck::fail(
                "WSL host OS",
                "WSL targets are only supported on Windows",
            )];
        }
        vec![if crate::traits::which("wsl").is_some() {
            PrereqCheck::ok("wsl.exe")
        } else {
            PrereqCheck::fail(
                "wsl.exe",
                "Enable WSL: `wsl --install` from an elevated PowerShell",
            )
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supports_only_on_windows() {
        // The supports() flag must agree with the build target.
        if cfg!(target_os = "windows") {
            assert!(WslTarget::supports());
        } else {
            assert!(!WslTarget::supports());
        }
    }

    #[test]
    fn create_on_non_windows_returns_unavailable() {
        if cfg!(target_os = "windows") {
            // Skip: on Windows we'd actually try to install.
            return;
        }
        let t = WslTarget::new("w1", "Ubuntu-22.04");
        let err = t
            .create()
            .expect_err("should be unavailable on non-Windows");
        let msg = err.to_string();
        assert!(msg.contains("Windows host"), "unexpected: {}", msg);
    }
}
