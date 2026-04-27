//! Kubernetes target.
use crate::error::TargetError;
use crate::traits::{PrereqCheck, Target};
use sindri_core::platform::{Arch, Capabilities, Os, Platform, TargetProfile};
use std::path::Path;

/// Kubernetes Pod target. Shells out to `kubectl`; auth is whatever kubectl
/// already has in `~/.kube/config`.
pub struct KubernetesTarget {
    pub name: String,
    pub namespace: String,
    pub pod_name: String,
}

impl KubernetesTarget {
    /// Construct a new Kubernetes target.
    pub fn new(name: &str, namespace: &str) -> Self {
        KubernetesTarget {
            name: name.to_string(),
            namespace: namespace.to_string(),
            pod_name: format!("sindri-{}", name),
        }
    }
}

impl Target for KubernetesTarget {
    fn name(&self) -> &str {
        &self.name
    }
    fn kind(&self) -> &str {
        "kubernetes"
    }

    fn profile(&self) -> Result<TargetProfile, TargetError> {
        // Pods inherit node arch; default to x86_64 since multi-arch probing
        // would require a `kubectl get node -o jsonpath` round-trip every
        // call. Document and revisit if/when target.profile is cached.
        Ok(TargetProfile {
            platform: Platform {
                os: Os::Linux,
                arch: Arch::X86_64,
            },
            capabilities: Capabilities::default(),
        })
    }

    fn exec(&self, cmd: &str, _env: &[(&str, &str)]) -> Result<(String, String), TargetError> {
        let output = std::process::Command::new("kubectl")
            .args([
                "exec",
                "-n",
                &self.namespace,
                &self.pod_name,
                "--",
                "sh",
                "-c",
                cmd,
            ])
            .output()
            .map_err(|e| TargetError::Prerequisites {
                target: self.name.clone(),
                detail: format!("kubectl not found: {}", e),
            })?;
        Ok((
            String::from_utf8_lossy(&output.stdout).to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }

    fn upload(&self, local: &Path, remote: &str) -> Result<(), TargetError> {
        std::process::Command::new("kubectl")
            .args([
                "cp",
                "-n",
                &self.namespace,
                &local.to_string_lossy(),
                &format!("{}/{}", self.pod_name, remote),
            ])
            .status()
            .map_err(|e| TargetError::ExecFailed {
                target: self.name.clone(),
                detail: e.to_string(),
            })?;
        Ok(())
    }

    fn download(&self, remote: &str, local: &Path) -> Result<(), TargetError> {
        std::process::Command::new("kubectl")
            .args([
                "cp",
                "-n",
                &self.namespace,
                &format!("{}/{}", self.pod_name, remote),
                &local.to_string_lossy(),
            ])
            .status()
            .map_err(|e| TargetError::ExecFailed {
                target: self.name.clone(),
                detail: e.to_string(),
            })?;
        Ok(())
    }

    fn check_prerequisites(&self) -> Vec<PrereqCheck> {
        vec![if crate::traits::which("kubectl").is_some() {
            PrereqCheck::ok("kubectl CLI")
        } else {
            PrereqCheck::fail(
                "kubectl CLI",
                "Install kubectl: https://kubernetes.io/docs/tasks/tools/",
            )
        }]
    }
}
