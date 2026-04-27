//! RunPod GPU pod target.
//!
//! Per ADR-017 §3 and the implementation plan §10, RunPod is one of the
//! GPU-cloud kinds we support out of the box. The control plane is the
//! REST API at `https://api.runpod.io`; the data plane is SSH (RunPod
//! assigns each pod an SSH endpoint of the form `<pod-id>@ssh.runpod.io`
//! on a per-pod port).
//!
//! Authentication is one of:
//! * `auth.token: env:RUNPOD_API_KEY`        (default)
//! * `auth.token: cli:runpodctl config -k`   (delegate to runpodctl)
//! * `auth.token: file:~/.runpod/api-key`    (read from disk)
//!
//! For Wave 3C we shell out to `runpodctl` for create/destroy/exec; the
//! native HTTP API path is documented and stubbed in `create_payload` so
//! a future change can swap it in without a breaking surface change.
use crate::auth::AuthValue;
use crate::error::TargetError;
use crate::traits::{PrereqCheck, Target};
use sindri_core::platform::{Arch, Capabilities, Os, Platform, TargetProfile};
use std::path::Path;

/// A RunPod pod target.
pub struct RunPodTarget {
    pub name: String,
    /// RunPod GPU type ID, e.g. `NVIDIA GeForce RTX 4090` or `NVIDIA A100`.
    pub gpu_type_id: String,
    /// Number of GPUs (default 1).
    pub gpu_count: u32,
    /// `SECURE` or `COMMUNITY` cloud.
    pub cloud_type: String,
    /// Optional region preference, e.g. `US-WEST`.
    pub region: Option<String>,
    /// Optional spot bid price (USD/hr); `None` means on-demand.
    pub spot_bid: Option<f64>,
    /// Container image (defaults to a generic Linux image).
    pub image: String,
    /// Resolved auth token source.
    pub auth: Option<AuthValue>,
    /// Pod ID once provisioned (returned by `create`).
    pub pod_id: Option<String>,
}

impl RunPodTarget {
    /// Construct a new RunPod target.
    pub fn new(name: &str, gpu_type_id: &str) -> Self {
        RunPodTarget {
            name: name.to_string(),
            gpu_type_id: gpu_type_id.to_string(),
            gpu_count: 1,
            cloud_type: "SECURE".into(),
            region: None,
            spot_bid: None,
            image: "runpod/pytorch:2.1.0-py3.10-cuda11.8.0-devel-ubuntu22.04".into(),
            auth: None,
            pod_id: None,
        }
    }

    /// Build the JSON body that would be POSTed to
    /// `https://api.runpod.io/v2/<endpoint>/run`. Exposed (and tested)
    /// independently of the network call so we can validate field shape.
    pub fn create_payload(&self) -> serde_json::Value {
        let mut body = serde_json::json!({
            "gpuTypeId": self.gpu_type_id,
            "count": self.gpu_count,
            "cloudType": self.cloud_type,
            "imageName": self.image,
        });
        if let Some(region) = &self.region {
            body["region"] = serde_json::Value::String(region.clone());
        }
        if let Some(bid) = self.spot_bid {
            body["spotBid"] = serde_json::json!(bid);
        }
        body
    }

    /// Detect the platform a known RunPod GPU type implies. RunPod GPU
    /// pods are all Linux; nearly all current GPU types are x86_64
    /// (NVIDIA), with a handful of ARM variants for Grace Hopper. We
    /// pattern-match on the GPU type ID and default to x86_64.
    fn known_platform(&self) -> Platform {
        let id = self.gpu_type_id.to_lowercase();
        let arch = if id.contains("grace") || id.contains("aarch64") || id.contains("arm") {
            Arch::Aarch64
        } else {
            Arch::X86_64
        };
        Platform {
            os: Os::Linux,
            arch,
        }
    }
}

impl Target for RunPodTarget {
    fn name(&self) -> &str {
        &self.name
    }
    fn kind(&self) -> &str {
        "runpod"
    }

    fn profile(&self) -> Result<TargetProfile, TargetError> {
        Ok(TargetProfile {
            platform: self.known_platform(),
            capabilities: Capabilities {
                system_package_manager: Some("apt-get".into()),
                has_docker: false,
                has_sudo: true,
                shell: Some("/bin/bash".into()),
            },
        })
    }

    fn exec(&self, cmd: &str, _env: &[(&str, &str)]) -> Result<(String, String), TargetError> {
        let pod = self
            .pod_id
            .as_deref()
            .ok_or_else(|| TargetError::NotProvisioned {
                name: self.name.clone(),
            })?;
        // RunPod ships an SSH endpoint per pod; we delegate to `runpodctl`
        // which handles the proxy (and key material) for us.
        let output = std::process::Command::new("runpodctl")
            .args(["exec", "--podId", pod, "--", "sh", "-c", cmd])
            .output()
            .map_err(|e| TargetError::Prerequisites {
                target: self.name.clone(),
                detail: format!("runpodctl not found: {}", e),
            })?;
        Ok((
            String::from_utf8_lossy(&output.stdout).to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }

    fn upload(&self, _local: &Path, _remote: &str) -> Result<(), TargetError> {
        Err(TargetError::Unavailable {
            name: self.name.clone(),
            reason: "RunPod uploads are not yet wired; use `runpodctl send`".into(),
        })
    }

    fn download(&self, _remote: &str, _local: &Path) -> Result<(), TargetError> {
        Err(TargetError::Unavailable {
            name: self.name.clone(),
            reason: "RunPod downloads are not yet wired; use `runpodctl receive`".into(),
        })
    }

    fn create(&self) -> Result<(), TargetError> {
        // Future: POST self.create_payload() to the REST API. For now we
        // delegate to runpodctl so we don't reinvent auth + retry logic.
        let _ = self.create_payload();
        std::process::Command::new("runpodctl")
            .args([
                "create",
                "pod",
                "--name",
                &self.name,
                "--gpuType",
                &self.gpu_type_id,
                "--gpuCount",
                &self.gpu_count.to_string(),
                "--imageName",
                &self.image,
            ])
            .status()
            .map_err(|e| TargetError::Prerequisites {
                target: self.name.clone(),
                detail: e.to_string(),
            })?;
        Ok(())
    }

    fn destroy(&self) -> Result<(), TargetError> {
        let pod = self
            .pod_id
            .as_deref()
            .ok_or_else(|| TargetError::NotProvisioned {
                name: self.name.clone(),
            })?;
        std::process::Command::new("runpodctl")
            .args(["remove", "pod", pod])
            .status()
            .map_err(|e| TargetError::Prerequisites {
                target: self.name.clone(),
                detail: e.to_string(),
            })?;
        Ok(())
    }

    fn check_prerequisites(&self) -> Vec<PrereqCheck> {
        let mut out = vec![if crate::traits::which("runpodctl").is_some() {
            PrereqCheck::ok("runpodctl CLI")
        } else {
            PrereqCheck::fail(
                "runpodctl CLI",
                "Install runpodctl: https://github.com/runpod/runpodctl",
            )
        }];
        if self.auth.is_none() && std::env::var("RUNPOD_API_KEY").is_err() {
            out.push(PrereqCheck::fail(
                "RunPod auth",
                "Set RUNPOD_API_KEY or configure auth.token in sindri.yaml",
            ));
        } else {
            out.push(PrereqCheck::ok("RunPod auth"));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_returns_linux_for_known_gpu_type() {
        let t = RunPodTarget::new("rp1", "NVIDIA A100");
        let p = t.profile().expect("profile");
        assert_eq!(p.platform.os, Os::Linux);
        assert_eq!(p.platform.arch, Arch::X86_64);
    }

    #[test]
    fn profile_picks_aarch64_for_grace_gpu() {
        let t = RunPodTarget::new("rp2", "NVIDIA Grace Hopper");
        let p = t.profile().expect("profile");
        assert_eq!(p.platform.arch, Arch::Aarch64);
    }

    #[test]
    fn create_payload_includes_required_fields() {
        let mut t = RunPodTarget::new("rp", "RTX 4090");
        t.region = Some("US-WEST".into());
        t.spot_bid = Some(0.25);
        let p = t.create_payload();
        assert_eq!(p["gpuTypeId"], "RTX 4090");
        assert_eq!(p["cloudType"], "SECURE");
        assert_eq!(p["region"], "US-WEST");
        assert_eq!(p["spotBid"], 0.25);
    }
}
