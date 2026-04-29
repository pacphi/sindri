//! DevPod target — wraps the `devpod` CLI across seven providers.
//!
//! DevPod (loft.sh) provides a uniform CLI for spinning up dev containers
//! across many backends. Each `DevPodKind` maps onto DevPod's
//! `--provider <name>` flag. For Wave 3C we delegate every operation to
//! the upstream binary rather than reimplementing each provider.
//!
//! Provider mapping (kind → DevPod `--provider`):
//!
//! | sindri kind         | devpod provider |
//! |---------------------|-----------------|
//! | `devpod-aws`        | `aws`           |
//! | `devpod-gcp`        | `gcloud`        |
//! | `devpod-azure`      | `azure`         |
//! | `devpod-digitalocean` | `digitalocean` |
//! | `devpod-k8s`        | `kubernetes`    |
//! | `devpod-ssh`        | `ssh`           |
//! | `devpod-docker`     | `docker`        |
use crate::error::TargetError;
use crate::traits::{PrereqCheck, Target};
use sindri_core::platform::{Arch, Capabilities, Os, Platform, TargetProfile};
use std::path::Path;

/// One of the seven supported DevPod providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevPodKind {
    Aws,
    Gcp,
    Azure,
    DigitalOcean,
    Kubernetes,
    Ssh,
    Docker,
}

impl DevPodKind {
    /// The DevPod CLI `--provider` value for this kind.
    pub fn provider_flag(self) -> &'static str {
        match self {
            DevPodKind::Aws => "aws",
            DevPodKind::Gcp => "gcloud",
            DevPodKind::Azure => "azure",
            DevPodKind::DigitalOcean => "digitalocean",
            DevPodKind::Kubernetes => "kubernetes",
            DevPodKind::Ssh => "ssh",
            DevPodKind::Docker => "docker",
        }
    }

    /// The string used in sindri.yaml `kind:` for this variant.
    pub fn sindri_kind(self) -> &'static str {
        match self {
            DevPodKind::Aws => "devpod-aws",
            DevPodKind::Gcp => "devpod-gcp",
            DevPodKind::Azure => "devpod-azure",
            DevPodKind::DigitalOcean => "devpod-digitalocean",
            DevPodKind::Kubernetes => "devpod-k8s",
            DevPodKind::Ssh => "devpod-ssh",
            DevPodKind::Docker => "devpod-docker",
        }
    }

    /// Parse the `kind:` string from sindri.yaml.
    pub fn parse(kind: &str) -> Option<Self> {
        Some(match kind {
            "devpod-aws" => DevPodKind::Aws,
            "devpod-gcp" => DevPodKind::Gcp,
            "devpod-azure" => DevPodKind::Azure,
            "devpod-digitalocean" => DevPodKind::DigitalOcean,
            "devpod-k8s" => DevPodKind::Kubernetes,
            "devpod-ssh" => DevPodKind::Ssh,
            "devpod-docker" => DevPodKind::Docker,
            _ => return None,
        })
    }
}

/// A DevPod workspace target.
pub struct DevPodTarget {
    pub kind: DevPodKind,
    pub name: String,
    pub workspace: String,
}

impl DevPodTarget {
    /// Construct a new DevPod target. `workspace` is the DevPod workspace
    /// name (defaults to the local target name).
    pub fn new(kind: DevPodKind, name: &str) -> Self {
        DevPodTarget {
            kind,
            name: name.to_string(),
            workspace: name.to_string(),
        }
    }
}

impl Target for DevPodTarget {
    fn name(&self) -> &str {
        &self.name
    }
    fn kind(&self) -> &str {
        self.kind.sindri_kind()
    }

    fn profile(&self) -> Result<TargetProfile, TargetError> {
        // DevPod spins up Linux dev containers regardless of provider; the
        // host arch follows the cloud node, defaulting to x86_64.
        Ok(TargetProfile {
            platform: Platform {
                os: Os::Linux,
                arch: Arch::X86_64,
            },
            capabilities: Capabilities {
                system_package_manager: Some("apt-get".into()),
                has_docker: matches!(self.kind, DevPodKind::Docker),
                has_sudo: true,
                shell: Some("/bin/bash".into()),
            },
        })
    }

    fn exec(&self, cmd: &str, _env: &[(&str, &str)]) -> Result<(String, String), TargetError> {
        let output = std::process::Command::new("devpod")
            .args(["ssh", &self.workspace, "--command", cmd])
            .output()
            .map_err(|e| TargetError::Prerequisites {
                target: self.name.clone(),
                detail: format!("devpod CLI not found: {}", e),
            })?;
        Ok((
            String::from_utf8_lossy(&output.stdout).to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }

    fn upload(&self, _local: &Path, _remote: &str) -> Result<(), TargetError> {
        Err(TargetError::Unavailable {
            name: self.name.clone(),
            reason: "use `devpod ssh --command` with rsync/scp for transfers".into(),
        })
    }

    fn download(&self, _remote: &str, _local: &Path) -> Result<(), TargetError> {
        Err(TargetError::Unavailable {
            name: self.name.clone(),
            reason: "use `devpod ssh --command` with rsync/scp for transfers".into(),
        })
    }

    fn create(&self) -> Result<(), TargetError> {
        std::process::Command::new("devpod")
            .args([
                "up",
                &self.workspace,
                "--provider",
                self.kind.provider_flag(),
            ])
            .status()
            .map_err(|e| TargetError::Prerequisites {
                target: self.name.clone(),
                detail: e.to_string(),
            })?;
        Ok(())
    }

    fn destroy(&self) -> Result<(), TargetError> {
        std::process::Command::new("devpod")
            .args(["delete", &self.workspace])
            .status()
            .map_err(|e| TargetError::Prerequisites {
                target: self.name.clone(),
                detail: e.to_string(),
            })?;
        Ok(())
    }

    fn check_prerequisites(&self) -> Vec<PrereqCheck> {
        vec![if crate::traits::which("devpod").is_some() {
            PrereqCheck::ok("devpod CLI")
        } else {
            PrereqCheck::fail(
                "devpod CLI",
                "Install DevPod: https://devpod.sh/docs/getting-started/install",
            )
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aws_kind_passes_provider_aws() {
        let t = DevPodTarget::new(DevPodKind::Aws, "ws");
        assert_eq!(t.kind(), "devpod-aws");
        assert_eq!(t.kind.provider_flag(), "aws");
    }

    #[test]
    fn k8s_kind_uses_kubernetes_provider_string() {
        assert_eq!(DevPodKind::Kubernetes.provider_flag(), "kubernetes");
        assert_eq!(DevPodKind::Kubernetes.sindri_kind(), "devpod-k8s");
    }

    #[test]
    fn parse_round_trips_all_seven_kinds() {
        for k in [
            DevPodKind::Aws,
            DevPodKind::Gcp,
            DevPodKind::Azure,
            DevPodKind::DigitalOcean,
            DevPodKind::Kubernetes,
            DevPodKind::Ssh,
            DevPodKind::Docker,
        ] {
            assert_eq!(DevPodKind::parse(k.sindri_kind()), Some(k));
        }
    }
}
