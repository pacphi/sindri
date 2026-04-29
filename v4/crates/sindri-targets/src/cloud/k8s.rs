//! Kubernetes target.
//!
//! Wave 6B (audit D2) wires the convergence engine to drive
//! `kubectl apply -f` / `kubectl delete -f` against a generated
//! Pod manifest. We deliberately do NOT pull in `kube-rs` — the simpler
//! shell-out keeps the dep graph small and re-uses whatever auth
//! kubectl already has in `~/.kube/config`. Callers that need a
//! richer client can swap this for `kube-rs` in a later wave.
//!
//! Auth: kubectl's existing config (`~/.kube/config`); sindri does not
//! drive Kubernetes OAuth flows. The `target auth` wizard preserves
//! the upstream-CLI hint path for k8s.
use crate::error::TargetError;
use crate::traits::{PrereqCheck, Target};
use sindri_core::auth::{AuthCapability, AuthSource};
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

    /// Render a minimal Pod manifest for this target. The image and
    /// command come from `desired` if present, otherwise sensible
    /// defaults (`alpine:latest`, `sleep infinity`).
    pub fn render_manifest(&self, desired: Option<&serde_json::Value>) -> String {
        let image = desired
            .and_then(|d| d.get("image"))
            .and_then(|v| v.as_str())
            .unwrap_or("alpine:latest");
        let command = desired
            .and_then(|d| d.get("command"))
            .and_then(|v| v.as_str())
            .unwrap_or("sleep infinity");
        format!(
            "apiVersion: v1\nkind: Pod\nmetadata:\n  name: {}\n  namespace: {}\n  labels:\n    app.kubernetes.io/managed-by: sindri\n    sindri.io/target: {}\nspec:\n  containers:\n  - name: main\n    image: {}\n    command: [\"sh\", \"-c\", {:?}]\n",
            self.pod_name, self.namespace, self.name, image, command
        )
    }

    /// Apply (create-or-update) the Pod via `kubectl apply -f -` with
    /// the rendered manifest piped on stdin. Returns the pod name.
    pub fn dispatch_apply(
        &self,
        desired: Option<&serde_json::Value>,
    ) -> Result<String, TargetError> {
        let manifest = self.render_manifest(desired);
        let mut child = std::process::Command::new("kubectl")
            .args(["apply", "-f", "-", "-n", &self.namespace])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| TargetError::Prerequisites {
                target: self.name.clone(),
                detail: format!("kubectl not found: {}", e),
            })?;
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin
                .write_all(manifest.as_bytes())
                .map_err(|e| TargetError::ExecFailed {
                    target: self.name.clone(),
                    detail: format!("kubectl stdin write failed: {}", e),
                })?;
        }
        let output = child
            .wait_with_output()
            .map_err(|e| TargetError::ExecFailed {
                target: self.name.clone(),
                detail: format!("kubectl wait failed: {}", e),
            })?;
        if !output.status.success() {
            return Err(TargetError::ExecFailed {
                target: self.name.clone(),
                detail: format!(
                    "kubectl apply failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            });
        }
        Ok(self.pod_name.clone())
    }

    /// Delete the Pod via `kubectl delete pod`. Idempotent: a
    /// `NotFound` error from kubectl is treated as success.
    pub fn dispatch_delete(&self) -> Result<(), TargetError> {
        let output = std::process::Command::new("kubectl")
            .args([
                "delete",
                "pod",
                &self.pod_name,
                "-n",
                &self.namespace,
                "--ignore-not-found=true",
            ])
            .output()
            .map_err(|e| TargetError::Prerequisites {
                target: self.name.clone(),
                detail: format!("kubectl not found: {}", e),
            })?;
        if !output.status.success() {
            return Err(TargetError::ExecFailed {
                target: self.name.clone(),
                detail: format!(
                    "kubectl delete failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            });
        }
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_manifest_uses_defaults_when_desired_missing() {
        let t = KubernetesTarget::new("dev", "default");
        let m = t.render_manifest(None);
        assert!(m.contains("name: sindri-dev"));
        assert!(m.contains("namespace: default"));
        assert!(m.contains("alpine:latest"));
        assert!(m.contains("sleep infinity"));
        assert!(m.contains("app.kubernetes.io/managed-by: sindri"));
    }

    #[test]
    fn render_manifest_honours_desired_image_and_command() {
        let t = KubernetesTarget::new("dev", "default");
        let desired = serde_json::json!({"image": "ubuntu:24.04", "command": "tail -f /dev/null"});
        let m = t.render_manifest(Some(&desired));
        assert!(m.contains("ubuntu:24.04"));
        assert!(m.contains("tail -f /dev/null"));
    }

    // ─── auth_capabilities() — ADR-027 §Phase 4 ────────────────────────────
    //
    // Tests that mutate `PATH` use `well_known::ENV_LOCK` to serialise.
    mod auth_cap_tests {
        use super::super::*;
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
        fn k8s_without_kubectl_empty() {
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
    }
}
