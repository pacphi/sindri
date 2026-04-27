//! Northflank target.
//!
//! Northflank exposes a REST API at `https://api.northflank.com/v1`. For
//! Wave 3C we shell out to `curl` rather than pulling reqwest into
//! sindri-targets; the audit explicitly endorses CLI/HTTP delegation
//! at this stage.
//!
//! Auth is via `auth.token: env:NORTHFLANK_API_TOKEN` (the prefixed-value
//! form from ADR-020). Northflank's exec endpoint is
//! `POST /projects/{project}/services/{service}/exec`.
use crate::auth::AuthValue;
use crate::error::TargetError;
use crate::traits::{PrereqCheck, Target};
use sindri_core::platform::{Arch, Capabilities, Os, Platform, TargetProfile};
use std::path::Path;

/// A Northflank service target.
pub struct NorthflankTarget {
    pub name: String,
    pub project: String,
    pub service: String,
    /// Optional volume mount (Northflank "addon").
    pub volume: Option<String>,
    /// Auth token source.
    pub auth: Option<AuthValue>,
    /// Exposed ports (forwarded into the container spec).
    pub ports: Vec<u16>,
}

impl NorthflankTarget {
    /// Construct a new Northflank target.
    pub fn new(name: &str, project: &str, service: &str) -> Self {
        NorthflankTarget {
            name: name.to_string(),
            project: project.to_string(),
            service: service.to_string(),
            volume: None,
            auth: None,
            ports: Vec::new(),
        }
    }

    /// Build the args we pass to `curl` for `create`. Exposed for testing
    /// so we can assert the URL + payload shape without making a network
    /// call.
    pub fn create_command(&self) -> Vec<String> {
        let url = format!(
            "https://api.northflank.com/v1/projects/{}/services/combined",
            self.project
        );
        let mut payload = serde_json::json!({
            "name": self.service,
            "deployment": { "instances": 1 },
        });
        if !self.ports.is_empty() {
            payload["ports"] = serde_json::Value::Array(
                self.ports
                    .iter()
                    .map(|p| serde_json::json!({ "name": format!("p{}", p), "internalPort": p }))
                    .collect(),
            );
        }
        if let Some(volume) = &self.volume {
            payload["volume"] = serde_json::Value::String(volume.clone());
        }
        vec![
            "-sSfL".into(),
            "-X".into(),
            "POST".into(),
            url,
            "-H".into(),
            "Content-Type: application/json".into(),
            "--data".into(),
            payload.to_string(),
        ]
    }

    fn token(&self) -> Result<String, TargetError> {
        if let Some(av) = &self.auth {
            return av.resolve();
        }
        std::env::var("NORTHFLANK_API_TOKEN").map_err(|_| TargetError::AuthFailed {
            target: self.name.clone(),
            detail: "NORTHFLANK_API_TOKEN not set and auth.token not configured".into(),
        })
    }
}

impl Target for NorthflankTarget {
    fn name(&self) -> &str {
        &self.name
    }
    fn kind(&self) -> &str {
        "northflank"
    }

    fn profile(&self) -> Result<TargetProfile, TargetError> {
        // Northflank schedules Linux containers; arch defaults to x86_64
        // but Northflank is rolling out aarch64 in select regions.
        Ok(TargetProfile {
            platform: Platform {
                os: Os::Linux,
                arch: Arch::X86_64,
            },
            capabilities: Capabilities {
                system_package_manager: Some("apt-get".into()),
                has_docker: false,
                has_sudo: false,
                shell: Some("/bin/sh".into()),
            },
        })
    }

    fn exec(&self, cmd: &str, _env: &[(&str, &str)]) -> Result<(String, String), TargetError> {
        let token = self.token()?;
        let url = format!(
            "https://api.northflank.com/v1/projects/{}/services/{}/exec",
            self.project, self.service
        );
        let body = serde_json::json!({ "command": ["sh", "-c", cmd] }).to_string();
        let output = std::process::Command::new("curl")
            .args([
                "-sSfL",
                "-X",
                "POST",
                &url,
                "-H",
                &format!("Authorization: Bearer {}", token),
                "-H",
                "Content-Type: application/json",
                "--data",
                &body,
            ])
            .output()
            .map_err(|e| TargetError::Prerequisites {
                target: self.name.clone(),
                detail: format!("curl not found: {}", e),
            })?;
        Ok((
            String::from_utf8_lossy(&output.stdout).to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }

    fn upload(&self, _local: &Path, _remote: &str) -> Result<(), TargetError> {
        Err(TargetError::Unavailable {
            name: self.name.clone(),
            reason: "Northflank file upload requires a build context; not yet wired".into(),
        })
    }

    fn download(&self, _remote: &str, _local: &Path) -> Result<(), TargetError> {
        Err(TargetError::Unavailable {
            name: self.name.clone(),
            reason: "Northflank file download is not supported via the public API".into(),
        })
    }

    fn create(&self) -> Result<(), TargetError> {
        let token = self.token()?;
        let mut args = vec![
            "-sSfL".to_string(),
            "-H".to_string(),
            format!("Authorization: Bearer {}", token),
        ];
        args.extend(self.create_command());
        std::process::Command::new("curl")
            .args(&args)
            .status()
            .map_err(|e| TargetError::Prerequisites {
                target: self.name.clone(),
                detail: e.to_string(),
            })?;
        Ok(())
    }

    fn destroy(&self) -> Result<(), TargetError> {
        let token = self.token()?;
        let url = format!(
            "https://api.northflank.com/v1/projects/{}/services/{}",
            self.project, self.service
        );
        std::process::Command::new("curl")
            .args([
                "-sSfL",
                "-X",
                "DELETE",
                &url,
                "-H",
                &format!("Authorization: Bearer {}", token),
            ])
            .status()
            .map_err(|e| TargetError::Prerequisites {
                target: self.name.clone(),
                detail: e.to_string(),
            })?;
        Ok(())
    }

    fn check_prerequisites(&self) -> Vec<PrereqCheck> {
        let mut out = vec![if crate::traits::which("curl").is_some() {
            PrereqCheck::ok("curl")
        } else {
            PrereqCheck::fail("curl", "Install curl (required for Northflank API calls)")
        }];
        if self.token().is_ok() {
            out.push(PrereqCheck::ok("Northflank auth"));
        } else {
            out.push(PrereqCheck::fail(
                "Northflank auth",
                "Set NORTHFLANK_API_TOKEN or configure auth.token in sindri.yaml",
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_command_includes_project_and_service() {
        let mut t = NorthflankTarget::new("nf1", "myproj", "mysvc");
        t.ports.push(8080);
        let cmd = t.create_command();
        let url = cmd
            .iter()
            .find(|s| s.contains("api.northflank.com"))
            .unwrap();
        assert!(url.contains("/projects/myproj/"));
        let body = cmd.last().unwrap();
        assert!(body.contains("\"name\":\"mysvc\""));
        assert!(body.contains("8080"));
    }
}
