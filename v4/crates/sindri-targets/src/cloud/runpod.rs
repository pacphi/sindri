//! RunPod GPU pod target.
//!
//! Per ADR-017 §3 and the implementation plan §10, RunPod is one of the
//! GPU-cloud kinds we support out of the box.
//!
//! Control plane: RunPod REST API at `https://api.runpod.io/v2/pod`
//! (Wave 5B — deferred item D4 closes the stub with real reqwest calls).
//!
//! Authentication:
//! * `auth.token: env:RUNPOD_API_KEY`          (default)
//! * `auth.token: cli:runpodctl config -k`     (delegate to runpodctl)
//! * `auth.token: file:~/.runpod/api-key`      (read from disk)
//!
//! Data plane (exec/upload/download): delegated to `runpodctl`, which
//! handles the SSH proxy and key material. Replacing exec with direct
//! SSH is tracked separately.
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
    /// Base URL for the RunPod REST API. Overridable in tests.
    pub base_url: String,
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
            base_url: "https://api.runpod.io".into(),
        }
    }

    /// Build the JSON body POSTed to `POST /v2/pod`.
    ///
    /// Exposed (and tested) independently of the network call so we can
    /// validate field shape without making a real HTTP request.
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

    /// Resolve the API token from the configured `AuthValue` or the
    /// `RUNPOD_API_KEY` environment variable as a fallback.
    fn resolve_token(&self) -> Result<String, TargetError> {
        if let Some(av) = &self.auth {
            return av.resolve();
        }
        std::env::var("RUNPOD_API_KEY").map_err(|_| TargetError::AuthFailed {
            target: self.name.clone(),
            detail: "RUNPOD_API_KEY not set and auth.token not configured in sindri.yaml".into(),
        })
    }

    /// Async HTTP dispatch — POST `/v2/pod` and return the new pod ID.
    ///
    /// Status-code mapping:
    /// * 200/201 → Ok(pod_id)
    /// * 401     → `TargetError::AuthFailed` with a clear hint
    /// * 429     → `TargetError::RateLimited`
    /// * other   → `TargetError::Http` with status + body
    pub async fn dispatch_create_async(&self) -> Result<String, TargetError> {
        let token = self.resolve_token()?;
        let payload = self.create_payload();
        let url = format!("{}/v2/pod", self.base_url);

        let client = reqwest::Client::new();
        let resp = client
            .post(&url)
            .bearer_auth(&token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| TargetError::Http {
                target: self.name.clone(),
                detail: format!("request failed: {}", e),
            })?;

        let status = resp.status();

        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(TargetError::AuthFailed {
                target: self.name.clone(),
                detail: "RunPod API returned 401 — check RUNPOD_API_KEY".to_string(),
            });
        }

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(TargetError::RateLimited {
                target: self.name.clone(),
            });
        }

        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(TargetError::Http {
                target: self.name.clone(),
                detail: format!("HTTP {}: {}", status, body),
            });
        }

        let body: serde_json::Value = resp.json().await.map_err(|e| TargetError::Http {
            target: self.name.clone(),
            detail: format!("failed to parse response: {}", e),
        })?;

        // RunPod REST API returns the pod object at the top level with `id`.
        let pod_id = body
            .get("id")
            .or_else(|| body.get("podId"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| TargetError::Http {
                target: self.name.clone(),
                detail: "response missing 'id' field".to_string(),
            })?
            .to_string();

        Ok(pod_id)
    }

    /// Synchronous wrapper around `dispatch_create_async`.  Used by the
    /// `Target::create` trait method, which is not async.  Creates a
    /// one-shot current-thread runtime for the HTTP call.
    fn dispatch_create(&self) -> Result<String, TargetError> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| TargetError::Http {
                target: self.name.clone(),
                detail: format!("failed to build tokio runtime: {}", e),
            })?
            .block_on(self.dispatch_create_async())
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

    /// Provision a new RunPod pod via the REST API (`POST /v2/pod`).
    ///
    /// On success the pod ID is logged; callers may persist it via
    /// `sindri target` infra-lock. On failure, typed errors allow the
    /// CLI to give actionable messages.
    fn create(&self) -> Result<(), TargetError> {
        let pod_id = self.dispatch_create()?;
        tracing::info!(target = %self.name, pod_id = %pod_id, "RunPod pod provisioned");
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
        let mut out = Vec::new();
        // Auth check — the token must be resolvable before we can call the API.
        match self.resolve_token() {
            Ok(_) => out.push(PrereqCheck::ok("RunPod API key resolves")),
            Err(_) => out.push(PrereqCheck::fail(
                "RunPod API key resolves",
                "Set RUNPOD_API_KEY or configure auth.token in sindri.yaml",
            )),
        }
        // runpodctl is only needed for exec/destroy; not strictly required for create.
        if crate::traits::which("runpodctl").is_some() {
            out.push(PrereqCheck::ok("runpodctl CLI (for exec/destroy)"));
        } else {
            out.push(PrereqCheck::fail(
                "runpodctl CLI (for exec/destroy)",
                "Install runpodctl: https://github.com/runpod/runpodctl",
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── existing payload/profile tests (preserved from Wave 3C) ──────────────

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

    // ── HTTP dispatch tests (Wave 5B) ─────────────────────────────────────────

    fn make_target(base_url: &str) -> RunPodTarget {
        RunPodTarget {
            name: "test-rp".into(),
            gpu_type_id: "RTX 3090".into(),
            gpu_count: 1,
            cloud_type: "SECURE".into(),
            region: None,
            spot_bid: None,
            image: "debian:bookworm-slim".into(),
            auth: Some(AuthValue::Plain("tok-runpod".into())),
            pod_id: None,
            base_url: base_url.to_string(),
        }
    }

    #[tokio::test]
    async fn http_create_success_returns_pod_id() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/pod"))
            .and(header("authorization", "Bearer tok-runpod"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "pod-abc123"})),
            )
            .mount(&server)
            .await;

        let t = make_target(&server.uri());
        let result = t.dispatch_create_async().await;
        assert!(result.is_ok(), "expected Ok but got: {:?}", result);
        assert_eq!(result.unwrap(), "pod-abc123");
    }

    #[tokio::test]
    async fn http_create_401_returns_auth_failed() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/pod"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let t = make_target(&server.uri());
        let err = t.dispatch_create_async().await.unwrap_err();
        assert!(
            matches!(err, TargetError::AuthFailed { .. }),
            "expected AuthFailed, got: {:?}",
            err
        );
        if let TargetError::AuthFailed { detail, .. } = &err {
            assert!(
                detail.contains("401"),
                "detail should mention 401: {}",
                detail
            );
        }
    }

    #[tokio::test]
    async fn http_create_429_returns_rate_limited() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/pod"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&server)
            .await;

        let t = make_target(&server.uri());
        let err = t.dispatch_create_async().await.unwrap_err();
        assert!(
            matches!(err, TargetError::RateLimited { .. }),
            "expected RateLimited, got: {:?}",
            err
        );
    }

    #[tokio::test]
    async fn http_create_500_returns_http_error() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/pod"))
            .respond_with(ResponseTemplate::new(500).set_body_string("internal server error"))
            .mount(&server)
            .await;

        let t = make_target(&server.uri());
        let err = t.dispatch_create_async().await.unwrap_err();
        assert!(
            matches!(err, TargetError::Http { .. }),
            "expected Http error, got: {:?}",
            err
        );
        if let TargetError::Http { detail, .. } = &err {
            assert!(
                detail.contains("500"),
                "detail should mention 500: {}",
                detail
            );
        }
    }
}
