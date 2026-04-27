use crate::auth::AuthValue;
use crate::error::TargetError;
use crate::traits::{PrereqCheck, Target};
use sindri_core::platform::{Arch, Capabilities, Os, Platform, TargetProfile};
use std::path::Path;

/// SSH remote target (ADR-017, ADR-020)
/// Sprint 9: shells out to the `ssh`/`scp` CLI binaries.
/// Full `russh`/`ssh2-rs` native implementation is Sprint 10 hardening.
pub struct SshTarget {
    pub name: String,
    pub host: String,
    pub user: String,
    pub port: u16,
    pub key_auth: Option<AuthValue>,
}

impl SshTarget {
    pub fn new(name: &str, host: &str) -> Self {
        SshTarget {
            name: name.to_string(),
            host: host.to_string(),
            user: "root".to_string(),
            port: 22,
            key_auth: None,
        }
    }

    pub fn with_user(mut self, user: &str) -> Self {
        self.user = user.to_string();
        self
    }

    pub fn with_key(mut self, key_auth: AuthValue) -> Self {
        self.key_auth = Some(key_auth);
        self
    }

    fn ssh_args(&self) -> Vec<String> {
        let mut args = vec![
            format!("-p{}", self.port),
            "-oStrictHostKeyChecking=no".to_string(),
            "-oBatchMode=yes".to_string(),
        ];
        if let Some(AuthValue::File(ref path)) = self.key_auth {
            let expanded = path.replace('~', &dirs_next_home());
            args.push(format!("-i{}", expanded));
        }
        args
    }

    fn host_str(&self) -> String {
        format!("{}@{}", self.user, self.host)
    }
}

impl Target for SshTarget {
    fn name(&self) -> &str {
        &self.name
    }

    fn kind(&self) -> &str {
        "ssh"
    }

    fn profile(&self) -> Result<TargetProfile, TargetError> {
        let (stdout, _) = self.exec("uname -m && uname -s", &[])?;
        let parts: Vec<&str> = stdout.trim().lines().collect();
        let arch = parts
            .first()
            .map(|s| {
                if s.contains("aarch64") || s.contains("arm") {
                    Arch::Aarch64
                } else {
                    Arch::X86_64
                }
            })
            .unwrap_or(Arch::X86_64);
        let os = parts
            .get(1)
            .map(|s| {
                if s.contains("Darwin") {
                    Os::Macos
                } else {
                    Os::Linux
                }
            })
            .unwrap_or(Os::Linux);

        Ok(TargetProfile {
            platform: Platform { os, arch },
            capabilities: Capabilities {
                system_package_manager: None,
                has_docker: false,
                has_sudo: true,
                shell: Some("/bin/sh".to_string()),
            },
        })
    }

    fn exec(&self, cmd: &str, env: &[(&str, &str)]) -> Result<(String, String), TargetError> {
        let env_prefix: String = env.iter().map(|(k, v)| format!("{}={} ", k, v)).collect();
        let full_cmd = format!("{}{}", env_prefix, cmd);

        let mut args = self.ssh_args();
        args.push(self.host_str());
        args.push(full_cmd);

        let output = std::process::Command::new("ssh")
            .args(&args)
            .output()
            .map_err(|e| TargetError::Prerequisites {
                target: self.name.clone(),
                detail: format!("ssh not available: {}", e),
            })?;

        Ok((
            String::from_utf8_lossy(&output.stdout).to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }

    fn upload(&self, local: &Path, remote: &str) -> Result<(), TargetError> {
        let dest = format!("{}:{}", self.host_str(), remote);
        let mut args = self.ssh_args();
        args.push(local.to_string_lossy().to_string());
        args.push(dest);

        std::process::Command::new("scp")
            .args(&args)
            .status()
            .map_err(|e| TargetError::ExecFailed {
                target: self.name.clone(),
                detail: e.to_string(),
            })?;
        Ok(())
    }

    fn download(&self, remote: &str, local: &Path) -> Result<(), TargetError> {
        let src = format!("{}:{}", self.host_str(), remote);
        let mut args = self.ssh_args();
        args.push(src);
        args.push(local.to_string_lossy().to_string());

        std::process::Command::new("scp")
            .args(&args)
            .status()
            .map_err(|e| TargetError::ExecFailed {
                target: self.name.clone(),
                detail: e.to_string(),
            })?;
        Ok(())
    }

    fn check_prerequisites(&self) -> Vec<PrereqCheck> {
        vec![
            if crate::traits::which("ssh").is_some() {
                PrereqCheck::ok("ssh CLI")
            } else {
                PrereqCheck::fail("ssh CLI", "Install OpenSSH client")
            },
            if crate::traits::which("scp").is_some() {
                PrereqCheck::ok("scp CLI")
            } else {
                PrereqCheck::fail("scp CLI", "Install OpenSSH client (includes scp)")
            },
        ]
    }
}

fn dirs_next_home() -> String {
    dirs_next::home_dir()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_default()
}
