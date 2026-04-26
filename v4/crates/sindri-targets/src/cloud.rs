/// Cloud target stubs (ADR-017, Sprint 10)
///
/// Each cloud target implements the Target trait. Sprint 10 provides the
/// struct/trait scaffolding with CLI delegation stubs. Full API integrations
/// are Sprint 10 hardening work.
use std::path::Path;
use sindri_core::platform::{Arch, Capabilities, Os, Platform, TargetProfile};
use crate::error::TargetError;
use crate::traits::{PrereqCheck, Target};

// ─── E2b Sandbox ────────────────────────────────────────────────────────────

pub struct E2bTarget {
    pub name: String,
    pub template: String,
    pub sandbox_id: Option<String>,
}

impl E2bTarget {
    pub fn new(name: &str, template: &str) -> Self {
        E2bTarget { name: name.to_string(), template: template.to_string(), sandbox_id: None }
    }
}

impl Target for E2bTarget {
    fn name(&self) -> &str { &self.name }
    fn kind(&self) -> &str { "e2b" }

    fn profile(&self) -> Result<TargetProfile, TargetError> {
        Ok(TargetProfile {
            platform: Platform { os: Os::Linux, arch: Arch::X86_64 },
            capabilities: Capabilities {
                system_package_manager: Some("apt-get".into()),
                has_docker: false,
                has_sudo: true,
                shell: Some("/bin/bash".into()),
            },
        })
    }

    fn exec(&self, cmd: &str, _env: &[(&str, &str)]) -> Result<(String, String), TargetError> {
        // Sprint 10 stub: shell out to e2b CLI
        let output = std::process::Command::new("e2b")
            .args(["sandbox", "exec", "--sandbox", self.sandbox_id.as_deref().unwrap_or(""), "--", "sh", "-c", cmd])
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
        Err(TargetError::Unavailable { name: self.name.clone(), reason: "upload via e2b CLI not yet implemented".into() })
    }

    fn download(&self, _remote: &str, _local: &Path) -> Result<(), TargetError> {
        Err(TargetError::Unavailable { name: self.name.clone(), reason: "download via e2b CLI not yet implemented".into() })
    }

    fn create(&self) -> Result<(), TargetError> {
        std::process::Command::new("e2b")
            .args(["sandbox", "create", "--template", &self.template])
            .status()
            .map_err(|e| TargetError::Prerequisites { target: self.name.clone(), detail: e.to_string() })?;
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

// ─── Fly.io ─────────────────────────────────────────────────────────────────

pub struct FlyTarget {
    pub name: String,
    pub app_name: String,
    pub region: Option<String>,
}

impl FlyTarget {
    pub fn new(name: &str, app_name: &str) -> Self {
        FlyTarget { name: name.to_string(), app_name: app_name.to_string(), region: None }
    }
}

impl Target for FlyTarget {
    fn name(&self) -> &str { &self.name }
    fn kind(&self) -> &str { "fly" }

    fn profile(&self) -> Result<TargetProfile, TargetError> {
        Ok(TargetProfile {
            platform: Platform { os: Os::Linux, arch: Arch::X86_64 },
            capabilities: Capabilities::default(),
        })
    }

    fn exec(&self, cmd: &str, _env: &[(&str, &str)]) -> Result<(String, String), TargetError> {
        let output = std::process::Command::new("flyctl")
            .args(["ssh", "console", "--app", &self.app_name, "--command", cmd])
            .output()
            .map_err(|e| TargetError::Prerequisites { target: self.name.clone(), detail: format!("flyctl not found: {}", e) })?;
        Ok((
            String::from_utf8_lossy(&output.stdout).to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }

    fn upload(&self, _local: &Path, _remote: &str) -> Result<(), TargetError> {
        Err(TargetError::Unavailable { name: self.name.clone(), reason: "use flyctl deploy for file transfer".into() })
    }

    fn download(&self, _remote: &str, _local: &Path) -> Result<(), TargetError> {
        Err(TargetError::Unavailable { name: self.name.clone(), reason: "use flyctl ssh sftp for downloads".into() })
    }

    fn create(&self) -> Result<(), TargetError> {
        std::process::Command::new("flyctl")
            .args(["apps", "create", &self.app_name, "--json"])
            .status()
            .map_err(|e| TargetError::Prerequisites { target: self.name.clone(), detail: e.to_string() })?;
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

// ─── Kubernetes ─────────────────────────────────────────────────────────────

pub struct KubernetesTarget {
    pub name: String,
    pub namespace: String,
    pub pod_name: String,
}

impl KubernetesTarget {
    pub fn new(name: &str, namespace: &str) -> Self {
        KubernetesTarget {
            name: name.to_string(),
            namespace: namespace.to_string(),
            pod_name: format!("sindri-{}", name),
        }
    }
}

impl Target for KubernetesTarget {
    fn name(&self) -> &str { &self.name }
    fn kind(&self) -> &str { "kubernetes" }

    fn profile(&self) -> Result<TargetProfile, TargetError> {
        Ok(TargetProfile {
            platform: Platform { os: Os::Linux, arch: Arch::X86_64 },
            capabilities: Capabilities::default(),
        })
    }

    fn exec(&self, cmd: &str, _env: &[(&str, &str)]) -> Result<(String, String), TargetError> {
        let output = std::process::Command::new("kubectl")
            .args(["exec", "-n", &self.namespace, &self.pod_name, "--", "sh", "-c", cmd])
            .output()
            .map_err(|e| TargetError::Prerequisites { target: self.name.clone(), detail: format!("kubectl not found: {}", e) })?;
        Ok((
            String::from_utf8_lossy(&output.stdout).to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }

    fn upload(&self, local: &Path, remote: &str) -> Result<(), TargetError> {
        std::process::Command::new("kubectl")
            .args(["cp", "-n", &self.namespace, &local.to_string_lossy(), &format!("{}/{}", self.pod_name, remote)])
            .status()
            .map_err(|e| TargetError::ExecFailed { target: self.name.clone(), detail: e.to_string() })?;
        Ok(())
    }

    fn download(&self, remote: &str, local: &Path) -> Result<(), TargetError> {
        std::process::Command::new("kubectl")
            .args(["cp", "-n", &self.namespace, &format!("{}/{}", self.pod_name, remote), &local.to_string_lossy()])
            .status()
            .map_err(|e| TargetError::ExecFailed { target: self.name.clone(), detail: e.to_string() })?;
        Ok(())
    }

    fn check_prerequisites(&self) -> Vec<PrereqCheck> {
        vec![if crate::traits::which("kubectl").is_some() {
            PrereqCheck::ok("kubectl CLI")
        } else {
            PrereqCheck::fail("kubectl CLI", "Install kubectl: https://kubernetes.io/docs/tasks/tools/")
        }]
    }
}
