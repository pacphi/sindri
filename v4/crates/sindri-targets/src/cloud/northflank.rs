//! Northflank service target.
//!
//! Northflank exposes a REST API at `https://api.northflank.com/v1`.
//!
//! Wave 5B (deferred item D4) replaces the earlier `curl`-delegation stub
//! with a native `reqwest` call. The `create_command` helper that produces
//! curl args is preserved so existing tests continue to pass.
//!
//! Auth is via `auth.token: env:NORTHFLANK_API_TOKEN` (the prefixed-value
//! form from ADR-020).
//!
//! Endpoint used for service creation:
//!   `POST /v1/projects/{project}/services/combined`
//! This is the stable combined-service endpoint documented at
//! https://api.northflank.com/v1 and consistent with what `create_command`
//! already built in Wave 3C.
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
    /// Base URL for the Northflank REST API. Overridable in tests.
    pub base_url: String,
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
            base_url: "https://api.northflank.com".into(),
        }
    }

    /// Build the args we pass to `curl` for `create`. Preserved from
    /// Wave 3C so existing tests continue to pass and callers that use
    /// this helper for inspection are not broken.
    pub fn create_command(&self) -> Vec<String> {
        let url = format!(
            "{}/v1/projects/{}/services/combined",
            self.base_url, self.project
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

    /// Build the JSON body for the `reqwest`-based create call.
    ///
    /// Matches the payload shape `create_command` already produces but
    /// as a `serde_json::Value` rather than a serialised string.
    fn create_payload(&self) -> serde_json::Value {
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
        payload
    }

    /// Resolve the API token from the configured `AuthValue` or the
    /// `NORTHFLANK_API_TOKEN` environment variable as a fallback.
    pub fn token(&self) -> Result<String, TargetError> {
        if let Some(av) = &self.auth {
            return av.resolve();
        }
        std::env::var("NORTHFLANK_API_TOKEN").map_err(|_| TargetError::AuthFailed {
            target: self.name.clone(),
            detail: "NORTHFLANK_API_TOKEN not set and auth.token not configured".into(),
        })
    }

    /// Async HTTP dispatch — POST to
    /// `/v1/projects/{project}/services/combined` and return the service ID.
    ///
    /// Status-code mapping:
    /// * 200/201 → Ok(service_id)
    /// * 401     → `TargetError::AuthFailed` with a clear hint
    /// * 429     → `TargetError::RateLimited`
    /// * other   → `TargetError::Http` with status + body
    pub async fn dispatch_create_async(&self) -> Result<String, TargetError> {
        let token = self.token()?;
        let payload = self.create_payload();
        let url = format!(
            "{}/v1/projects/{}/services/combined",
            self.base_url, self.project
        );

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
                detail: "Northflank API returned 401 — check NORTHFLANK_API_TOKEN".to_string(),
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

        // Northflank wraps created resources under `data.id`.
        let svc_id = body
            .pointer("/data/id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| TargetError::Http {
                target: self.name.clone(),
                detail: "response missing '/data/id' field".to_string(),
            })?
            .to_string();

        Ok(svc_id)
    }

    /// Async HTTP dispatch — `DELETE /v1/projects/{project}/services/{service}`.
    /// Idempotent: a 404 response is treated as success.
    pub async fn dispatch_destroy_async(&self) -> Result<(), TargetError> {
        let token = self.token()?;
        let url = format!(
            "{}/v1/projects/{}/services/{}",
            self.base_url, self.project, self.service
        );
        let client = reqwest::Client::new();
        let resp = client
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| TargetError::Http {
                target: self.name.clone(),
                detail: format!("destroy request failed: {}", e),
            })?;
        let status = resp.status();
        if status.is_success() || status == reqwest::StatusCode::NOT_FOUND {
            return Ok(());
        }
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(TargetError::AuthFailed {
                target: self.name.clone(),
                detail: "Northflank API returned 401 — check NORTHFLANK_API_TOKEN".into(),
            });
        }
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(TargetError::RateLimited {
                target: self.name.clone(),
            });
        }
        let body = resp.text().await.unwrap_or_default();
        Err(TargetError::Http {
            target: self.name.clone(),
            detail: format!("HTTP {}: {}", status, body),
        })
    }

    /// Async HTTP dispatch — `PATCH /v1/projects/{project}/services/{service}`
    /// with the mutable subset of the desired spec. Northflank exposes
    /// `deployment.instances` and `ports` as in-place mutable; the
    /// per-kind schema in `convergence::schema::NorthflankSchema` ensures
    /// only those are routed here.
    pub async fn dispatch_update_async(
        &self,
        desired: &serde_json::Value,
    ) -> Result<serde_json::Value, TargetError> {
        let token = self.token()?;
        let url = format!(
            "{}/v1/projects/{}/services/{}",
            self.base_url, self.project, self.service
        );
        let mut patch = serde_json::Map::new();
        if let Some(v) = desired.get("deployment") {
            patch.insert("deployment".into(), v.clone());
        }
        if let Some(v) = desired.get("ports") {
            patch.insert("ports".into(), v.clone());
        }
        let body = serde_json::Value::Object(patch);
        let client = reqwest::Client::new();
        let resp = client
            .patch(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| TargetError::Http {
                target: self.name.clone(),
                detail: format!("update request failed: {}", e),
            })?;
        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(TargetError::AuthFailed {
                target: self.name.clone(),
                detail: "Northflank API returned 401 — check NORTHFLANK_API_TOKEN".into(),
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
        let parsed: serde_json::Value = resp.json().await.map_err(|e| TargetError::Http {
            target: self.name.clone(),
            detail: format!("failed to parse response: {}", e),
        })?;
        Ok(parsed)
    }

    /// Synchronous wrapper around `dispatch_create_async`. Used by the
    /// `Target::create` trait method, which is not async. Creates a
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
            "{}/v1/projects/{}/services/{}/exec",
            self.base_url, self.project, self.service
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

    /// Provision a Northflank combined service via the REST API.
    ///
    /// On success the service ID is logged. On failure, typed errors
    /// allow the CLI to give actionable messages.
    fn create(&self) -> Result<(), TargetError> {
        let svc_id = self.dispatch_create()?;
        tracing::info!(target = %self.name, service_id = %svc_id, "Northflank service created");
        Ok(())
    }

    fn destroy(&self) -> Result<(), TargetError> {
        let token = self.token()?;
        let url = format!(
            "{}/v1/projects/{}/services/{}",
            self.base_url, self.project, self.service
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
        let mut out = Vec::new();
        // Auth check — the token must be resolvable before we can call the API.
        match self.token() {
            Ok(_) => out.push(PrereqCheck::ok("Northflank API token resolves")),
            Err(_) => out.push(PrereqCheck::fail(
                "Northflank API token resolves",
                "Set NORTHFLANK_API_TOKEN or configure auth.token in sindri.yaml",
            )),
        }
        // curl is only needed for exec/destroy; not required for create.
        if crate::traits::which("curl").is_some() {
            out.push(PrereqCheck::ok("curl (for exec/destroy)"));
        } else {
            out.push(PrereqCheck::fail("curl (for exec/destroy)", "Install curl"));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── existing command-builder test (preserved from Wave 3C) ───────────────

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

    // ── HTTP dispatch tests (Wave 5B) ─────────────────────────────────────────

    fn make_target(base_url: &str) -> NorthflankTarget {
        NorthflankTarget {
            name: "test-nf".into(),
            project: "proj-abc".into(),
            service: "sindri-svc".into(),
            volume: None,
            auth: Some(AuthValue::Plain("tok-nf".into())),
            ports: vec![],
            base_url: base_url.to_string(),
        }
    }

    #[tokio::test]
    async fn http_create_success_returns_service_id() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/projects/proj-abc/services/combined"))
            .and(header("authorization", "Bearer tok-nf"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"data": {"id": "svc-xyz789"}})),
            )
            .mount(&server)
            .await;

        let t = make_target(&server.uri());
        let result = t.dispatch_create_async().await;
        assert!(result.is_ok(), "expected Ok but got: {:?}", result);
        assert_eq!(result.unwrap(), "svc-xyz789");
    }

    #[tokio::test]
    async fn http_create_401_returns_auth_failed() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/projects/proj-abc/services/combined"))
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
            .and(path("/v1/projects/proj-abc/services/combined"))
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
            .and(path("/v1/projects/proj-abc/services/combined"))
            .respond_with(ResponseTemplate::new(500).set_body_string("upstream error"))
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

    // ── destroy / update HTTP dispatch (Wave 6B) ─────────────────────────────

    #[tokio::test]
    async fn http_destroy_success() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/v1/projects/proj-abc/services/sindri-svc"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;
        let t = make_target(&server.uri());
        t.dispatch_destroy_async().await.unwrap();
    }

    #[tokio::test]
    async fn http_destroy_404_treated_as_success() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/v1/projects/proj-abc/services/sindri-svc"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        let t = make_target(&server.uri());
        t.dispatch_destroy_async().await.unwrap();
    }

    #[tokio::test]
    async fn http_update_in_place_patches_deployment() {
        use wiremock::matchers::{body_string_contains, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/v1/projects/proj-abc/services/sindri-svc"))
            .and(body_string_contains("instances"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"data": {"id": "svc-xyz789"}})),
            )
            .mount(&server)
            .await;
        let t = make_target(&server.uri());
        let desired = serde_json::json!({"deployment": {"instances": 3}});
        let got = t.dispatch_update_async(&desired).await.unwrap();
        assert_eq!(got["data"]["id"], "svc-xyz789");
    }
}
