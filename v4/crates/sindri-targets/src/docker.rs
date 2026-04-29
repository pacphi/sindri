use crate::error::TargetError;
use crate::traits::{PrereqCheck, Target};
use crate::well_known;
use sindri_core::auth::AuthCapability;
use sindri_core::platform::{Arch, Capabilities, Os, Platform, TargetProfile};
use std::path::Path;

/// Docker container target (ADR-017)
pub struct DockerTarget {
    pub name: String,
    pub image: String,
    pub container_name: String,
}

impl DockerTarget {
    pub fn new(name: &str, image: &str) -> Self {
        DockerTarget {
            name: name.to_string(),
            image: image.to_string(),
            container_name: format!("sindri-{}", name),
        }
    }

    fn run_docker(&self, args: &[&str]) -> Result<(String, String), TargetError> {
        let output = std::process::Command::new("docker")
            .args(args)
            .output()
            .map_err(|e| TargetError::ExecFailed {
                target: self.name.clone(),
                detail: format!("docker not found: {}", e),
            })?;
        Ok((
            String::from_utf8_lossy(&output.stdout).to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }

    fn is_running(&self) -> bool {
        std::process::Command::new("docker")
            .args(["inspect", "-f", "{{.State.Running}}", &self.container_name])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "true")
            .unwrap_or(false)
    }
}

impl Target for DockerTarget {
    fn name(&self) -> &str {
        &self.name
    }

    fn kind(&self) -> &str {
        "docker"
    }

    fn profile(&self) -> Result<TargetProfile, TargetError> {
        if !self.is_running() {
            return Err(TargetError::NotProvisioned {
                name: self.name.clone(),
            });
        }
        // Query the container's OS/arch
        let (stdout, _) = self.exec("uname -m", &[])?;
        let arch = if stdout.trim().contains("aarch64") || stdout.trim().contains("arm64") {
            Arch::Aarch64
        } else {
            Arch::X86_64
        };
        Ok(TargetProfile {
            platform: Platform {
                os: Os::Linux,
                arch,
            },
            capabilities: Capabilities {
                system_package_manager: detect_container_pm(self),
                has_docker: false,
                has_sudo: true,
                shell: Some("/bin/sh".to_string()),
            },
        })
    }

    fn exec(&self, cmd: &str, env: &[(&str, &str)]) -> Result<(String, String), TargetError> {
        let mut args = vec!["exec"];
        let env_pairs: Vec<String> = env.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
        for pair in &env_pairs {
            args.push("-e");
            args.push(pair.as_str());
        }
        args.push(&self.container_name);
        args.push("sh");
        args.push("-c");
        args.push(cmd);
        self.run_docker(&args)
    }

    fn upload(&self, local: &Path, remote: &str) -> Result<(), TargetError> {
        let (_, stderr) = self.run_docker(&[
            "cp",
            &local.to_string_lossy(),
            &format!("{}:{}", self.container_name, remote),
        ])?;
        if !stderr.is_empty() {
            tracing::warn!("docker cp stderr: {}", stderr);
        }
        Ok(())
    }

    fn download(&self, remote: &str, local: &Path) -> Result<(), TargetError> {
        self.run_docker(&[
            "cp",
            &format!("{}:{}", self.container_name, remote),
            &local.to_string_lossy(),
        ])?;
        Ok(())
    }

    fn create(&self) -> Result<(), TargetError> {
        tracing::info!(
            "docker: creating container {} from {}",
            self.container_name,
            self.image
        );
        let (_, stderr) = self.run_docker(&[
            "run",
            "--name",
            &self.container_name,
            "-d",
            "--rm",
            &self.image,
            "sleep",
            "infinity",
        ])?;
        if !stderr.is_empty() && stderr.contains("Error") {
            return Err(TargetError::ExecFailed {
                target: self.name.clone(),
                detail: stderr,
            });
        }
        Ok(())
    }

    fn destroy(&self) -> Result<(), TargetError> {
        tracing::info!("docker: stopping/removing {}", self.container_name);
        let _ = self.run_docker(&["stop", &self.container_name]);
        let _ = self.run_docker(&["rm", "-f", &self.container_name]);
        Ok(())
    }

    fn check_prerequisites(&self) -> Vec<PrereqCheck> {
        vec![if crate::traits::which("docker").is_some() {
            PrereqCheck::ok("docker CLI")
        } else {
            PrereqCheck::fail(
                "docker CLI",
                "Install Docker: https://docs.docker.com/get-docker/",
            )
        }]
    }

    /// Advertise ambient credentials suitable for forwarding into the
    /// container (ADR-027 §1, Phase 4).
    ///
    /// Docker doesn't ship its own credential CLI for component auth, so
    /// only env-passthrough capabilities are surfaced. The operator still
    /// needs to opt those vars into the container via the runtime config
    /// (e.g. `docker run -e ANTHROPIC_API_KEY ...`); the binding is what
    /// tells the resolver that the value will be available *if* forwarded.
    ///
    /// Priority is `5` — lower than `local` so that a user running
    /// `sindri apply` against `local` and `docker` simultaneously prefers
    /// the host-side env-var binding (which doesn't require explicit
    /// forwarding).
    fn auth_capabilities(&self) -> Vec<AuthCapability> {
        well_known::ambient_env_capabilities(5)
    }
}

fn detect_container_pm(target: &DockerTarget) -> Option<String> {
    for pm in &["apt-get", "dnf", "apk"] {
        if target
            .exec(&format!("which {}", pm), &[])
            .map(|(o, _)| !o.trim().is_empty())
            .unwrap_or(false)
        {
            return Some(pm.to_string());
        }
    }
    None
}

#[cfg(test)]
mod auth_cap_tests {
    use super::*;
    use crate::well_known::ENV_LOCK;

    #[test]
    fn docker_advertises_ambient_env_only() {
        let _g = ENV_LOCK.lock().unwrap();
        // Clean the table.
        for v in &[
            "ANTHROPIC_API_KEY",
            "OPENAI_API_KEY",
            "GEMINI_API_KEY",
            "GITHUB_TOKEN",
        ] {
            // SAFETY: caller holds ENV_LOCK.
            unsafe { std::env::remove_var(v) };
        }
        // SAFETY: caller holds ENV_LOCK.
        unsafe { std::env::set_var("OPENAI_API_KEY", "sk-x") };

        let target = DockerTarget::new("dev", "ubuntu:22.04");
        let caps = target.auth_capabilities();
        // SAFETY: caller holds ENV_LOCK.
        unsafe { std::env::remove_var("OPENAI_API_KEY") };

        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].id, "openai_api_key");
        assert_eq!(caps[0].priority, 5);
        assert!(matches!(
            caps[0].source,
            sindri_core::auth::AuthSource::FromEnv { .. }
        ));
    }
}
