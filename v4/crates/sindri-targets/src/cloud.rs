use crate::error::TargetError;
use crate::traits::{PrereqCheck, Target};
use sindri_core::auth::{AuthCapability, AuthSource};
use sindri_core::platform::{Arch, Capabilities, Os, Platform, TargetProfile};

/// Cloud target stubs (ADR-017, Sprint 10)
///
/// Each cloud target implements the Target trait. Sprint 10 provides the
/// struct/trait scaffolding with CLI delegation stubs. Full API integrations
/// are Sprint 10 hardening work.
use std::path::Path;

// ─── E2b Sandbox ────────────────────────────────────────────────────────────

pub struct E2bTarget {
    pub name: String,
    pub template: String,
    pub sandbox_id: Option<String>,
}

impl E2bTarget {
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
        // Sprint 10 stub: shell out to e2b CLI
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

    /// E2B sandboxes don't have a native secret-store API the resolver
    /// can target — secrets land in the sandbox via the `e2b` CLI's
    /// `--env` flag at create time. We surface no capabilities by
    /// default; operators wire forwarded vars via `provides:` in the
    /// target manifest (ADR-027 §1, Phase 4).
    fn auth_capabilities(&self) -> Vec<AuthCapability> {
        Vec::new()
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

    /// Fly.io advertises:
    /// 1. **`flyctl auth token`** — the OAuth-result token from the
    ///    operator's logged-in `flyctl` session (audience GitHub-style
    ///    `https://api.fly.io`). Priority `15`.
    /// 2. **`flyctl secrets`** — per-app secrets group accessible via
    ///    `flyctl secrets list/get`. Modelled as a `FromCli` source with
    ///    a `{key}` template the resolver expands when binding (Phase 4
    ///    advertises a generic `flyctl secrets` capability id; per-secret
    ///    refinement happens in Phase 2 when redemption is wired). The
    ///    audience is `urn:fly:secrets` so component manifests can
    ///    declare a generic Fly-secrets requirement.
    ///
    /// Both are conditional on `flyctl` being on `PATH` — without the
    /// CLI neither path is reachable.
    fn auth_capabilities(&self) -> Vec<AuthCapability> {
        if crate::traits::which("flyctl").is_none() {
            return Vec::new();
        }
        vec![
            AuthCapability {
                id: "fly_auth_token".to_string(),
                audience: "https://api.fly.io".to_string(),
                source: AuthSource::FromCli {
                    command: "flyctl auth token".to_string(),
                },
                priority: 15,
            },
            AuthCapability {
                id: "fly_secrets".to_string(),
                audience: "urn:fly:secrets".to_string(),
                source: AuthSource::FromCli {
                    command: format!("flyctl secrets list --app {} --json", self.app_name),
                },
                priority: 12,
            },
        ]
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
    fn name(&self) -> &str {
        &self.name
    }
    fn kind(&self) -> &str {
        "kubernetes"
    }

    fn profile(&self) -> Result<TargetProfile, TargetError> {
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

    /// Kubernetes targets advertise the cluster's projected-secret
    /// mechanism (`valueFrom: { secretKeyRef }`) as a generic
    /// [`AuthSource::FromSecretsStore`] with backend `k8s` (ADR-027 §1,
    /// Phase 4).
    ///
    /// The `path` is the namespace — per-secret resolution happens at
    /// apply time (Phase 2) when a concrete `secretKeyRef.name` and
    /// `secretKeyRef.key` are projected into the workload pod. Audience
    /// is `urn:k8s:secrets` so component manifests can opt-in.
    ///
    /// Conditional on `kubectl` being on `PATH`.
    fn auth_capabilities(&self) -> Vec<AuthCapability> {
        if crate::traits::which("kubectl").is_none() {
            return Vec::new();
        }
        vec![AuthCapability {
            id: "k8s_secret_keyref".to_string(),
            audience: "urn:k8s:secrets".to_string(),
            source: AuthSource::FromSecretsStore {
                backend: "k8s".to_string(),
                path: self.namespace.clone(),
            },
            priority: 18,
        }]
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────
//
// Auth-capability tests for cloud targets live at the bottom of this file
// to keep the unit-test surface co-located with the implementations. Tests
// that mutate `PATH` use `well_known::ENV_LOCK` to serialise.

#[cfg(test)]
mod auth_cap_tests {
    use super::*;
    use crate::well_known::ENV_LOCK;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    fn fake_bin_dir(name: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let bin = dir.path().join(name);
        fs::write(&bin, "#!/bin/sh\nexit 0\n").unwrap();
        let mut perms = fs::metadata(&bin).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&bin, perms).unwrap();
        dir
    }

    #[test]
    fn fly_without_flyctl_yields_empty() {
        let _g = ENV_LOCK.lock().unwrap();
        // SAFETY: caller holds ENV_LOCK.
        unsafe { std::env::set_var("PATH", "/nonexistent-sindri-path-xyz") };
        let target = FlyTarget::new("prod", "my-app");
        assert!(target.auth_capabilities().is_empty());
    }

    #[test]
    fn fly_with_flyctl_advertises_oauth_and_secrets() {
        let _g = ENV_LOCK.lock().unwrap();
        let dir = fake_bin_dir("flyctl");
        // SAFETY: caller holds ENV_LOCK.
        unsafe { std::env::set_var("PATH", dir.path()) };

        let target = FlyTarget::new("prod", "my-app");
        let caps = target.auth_capabilities();

        let token = caps
            .iter()
            .find(|c| c.id == "fly_auth_token")
            .expect("fly_auth_token missing");
        assert_eq!(token.audience, "https://api.fly.io");
        match &token.source {
            AuthSource::FromCli { command } => assert_eq!(command, "flyctl auth token"),
            other => panic!("expected FromCli, got {:?}", other),
        }

        let secrets = caps
            .iter()
            .find(|c| c.id == "fly_secrets")
            .expect("fly_secrets missing");
        match &secrets.source {
            AuthSource::FromCli { command } => assert!(command.contains("my-app")),
            other => panic!("expected FromCli, got {:?}", other),
        }
    }

    #[test]
    fn k8s_without_kubectl_yields_empty() {
        let _g = ENV_LOCK.lock().unwrap();
        // SAFETY: caller holds ENV_LOCK.
        unsafe { std::env::set_var("PATH", "/nonexistent-sindri-path-xyz") };
        let target = KubernetesTarget::new("prod", "default");
        assert!(target.auth_capabilities().is_empty());
    }

    #[test]
    fn k8s_with_kubectl_advertises_secrets_store() {
        let _g = ENV_LOCK.lock().unwrap();
        let dir = fake_bin_dir("kubectl");
        // SAFETY: caller holds ENV_LOCK.
        unsafe { std::env::set_var("PATH", dir.path()) };

        let target = KubernetesTarget::new("prod", "my-namespace");
        let caps = target.auth_capabilities();
        assert_eq!(caps.len(), 1);
        let c = &caps[0];
        assert_eq!(c.id, "k8s_secret_keyref");
        assert_eq!(c.audience, "urn:k8s:secrets");
        match &c.source {
            AuthSource::FromSecretsStore { backend, path } => {
                assert_eq!(backend, "k8s");
                assert_eq!(path, "my-namespace");
            }
            other => panic!("expected FromSecretsStore, got {:?}", other),
        }
    }

    #[test]
    fn e2b_advertises_no_capabilities() {
        let target = E2bTarget::new("sandbox", "default");
        assert!(target.auth_capabilities().is_empty());
    }
}
