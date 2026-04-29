//! E2B Sandbox target.
//!
//! Wave 6B wires native HTTP calls to the E2B REST API at
//! `https://api.e2b.dev` so the convergence engine can drive create
//! and destroy without shelling out to the `e2b` CLI. Exec/upload
//! still use the CLI because they require the websocket protocol that
//! the CLI implements internally.
//!
//! Auth: API key from the E2B dashboard. There is no documented OAuth
//! device flow for E2B today; the `target auth` wizard preserves the
//! upstream-CLI hint path for E2B (`e2b auth login`).
use crate::auth::AuthValue;
use crate::error::TargetError;
use crate::traits::{PrereqCheck, Target};
use sindri_core::auth::AuthCapability;
use sindri_core::platform::{Arch, Capabilities, Os, Platform, TargetProfile};
use std::path::Path;

/// E2B sandbox target.
pub struct E2bTarget {
    pub name: String,
    pub template: String,
    pub sandbox_id: Option<String>,
    /// Resolved auth source.
    pub auth: Option<AuthValue>,
    /// Base URL for the E2B REST API. Overridable in tests.
    pub base_url: String,
}

impl E2bTarget {
    /// Construct a new E2B target with the given local name and sandbox template.
    pub fn new(name: &str, template: &str) -> Self {
        E2bTarget {
            name: name.to_string(),
            template: template.to_string(),
            sandbox_id: None,
            auth: None,
            base_url: "https://api.e2b.dev".into(),
        }
    }

    fn resolve_token(&self) -> Result<String, TargetError> {
        if let Some(av) = &self.auth {
            return av.resolve();
        }
        std::env::var("E2B_API_KEY").map_err(|_| TargetError::AuthFailed {
            target: self.name.clone(),
            detail: "E2B_API_KEY not set and auth.token not configured".into(),
        })
    }

    /// `POST /sandboxes` — returns the sandbox ID.
    pub async fn dispatch_create_async(
        &self,
        desired: Option<&serde_json::Value>,
    ) -> Result<String, TargetError> {
        let token = self.resolve_token()?;
        let url = format!("{}/sandboxes", self.base_url);
        let template = desired
            .and_then(|d| d.get("template"))
            .and_then(|v| v.as_str())
            .unwrap_or(&self.template);
        let body = serde_json::json!({"templateID": template});
        let client = reqwest::Client::new();
        let resp = client
            .post(&url)
            .header("X-API-KEY", &token)
            .json(&body)
            .send()
            .await
            .map_err(|e| TargetError::Http {
                target: self.name.clone(),
                detail: format!("create request failed: {}", e),
            })?;
        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(TargetError::AuthFailed {
                target: self.name.clone(),
                detail: "E2B API returned 401 — check E2B_API_KEY".into(),
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
        let id = body
            .get("sandboxID")
            .or_else(|| body.get("sandbox_id"))
            .or_else(|| body.get("id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| TargetError::Http {
                target: self.name.clone(),
                detail: "response missing 'sandboxID' field".into(),
            })?
            .to_string();
        Ok(id)
    }

    /// `DELETE /sandboxes/{id}`. Idempotent: 404 is treated as success.
    pub async fn dispatch_destroy_async(&self, sandbox_id: &str) -> Result<(), TargetError> {
        let token = self.resolve_token()?;
        let url = format!("{}/sandboxes/{}", self.base_url, sandbox_id);
        let client = reqwest::Client::new();
        let resp = client
            .delete(&url)
            .header("X-API-KEY", &token)
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
                detail: "E2B API returned 401 — check E2B_API_KEY".into(),
            });
        }
        let body = resp.text().await.unwrap_or_default();
        Err(TargetError::Http {
            target: self.name.clone(),
            detail: format!("HTTP {}: {}", status, body),
        })
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
        // E2B sandboxes are uniformly Linux x86_64 today.
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
        let id = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| TargetError::Http {
                target: self.name.clone(),
                detail: format!("failed to build tokio runtime: {}", e),
            })?
            .block_on(self.dispatch_create_async(None))?;
        tracing::info!(target = %self.name, sandbox_id = %id, "E2B sandbox created");
        Ok(())
    }

    fn check_prerequisites(&self) -> Vec<PrereqCheck> {
        let mut out = Vec::new();
        match self.resolve_token() {
            Ok(_) => out.push(PrereqCheck::ok("E2B API key resolves")),
            Err(_) => out.push(PrereqCheck::fail(
                "E2B API key resolves",
                "Set E2B_API_KEY or configure auth.token in sindri.yaml",
            )),
        }
        out.push(if crate::traits::which("e2b").is_some() {
            PrereqCheck::ok("e2b CLI (for exec)")
        } else {
            PrereqCheck::fail("e2b CLI (for exec)", "npm install -g @e2b/cli")
        });
        out
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_target(base_url: &str) -> E2bTarget {
        E2bTarget {
            name: "test-e2b".into(),
            template: "base".into(),
            sandbox_id: None,
            auth: Some(AuthValue::Plain("tok-e2b".into())),
            base_url: base_url.to_string(),
        }
    }

    #[tokio::test]
    async fn http_create_success_returns_sandbox_id() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/sandboxes"))
            .and(header("x-api-key", "tok-e2b"))
            .respond_with(
                ResponseTemplate::new(201).set_body_json(serde_json::json!({"sandboxID": "sb-1"})),
            )
            .mount(&server)
            .await;
        let t = make_target(&server.uri());
        let id = t.dispatch_create_async(None).await.unwrap();
        assert_eq!(id, "sb-1");
    }

    #[tokio::test]
    async fn http_destroy_success() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/sandboxes/sb-1"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;
        let t = make_target(&server.uri());
        t.dispatch_destroy_async("sb-1").await.unwrap();
    }

    #[tokio::test]
    async fn http_destroy_404_treated_as_success() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/sandboxes/gone"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        let t = make_target(&server.uri());
        t.dispatch_destroy_async("gone").await.unwrap();
    }

    #[tokio::test]
    async fn http_create_401_returns_auth_failed() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/sandboxes"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;
        let t = make_target(&server.uri());
        let err = t.dispatch_create_async(None).await.unwrap_err();
        assert!(matches!(err, TargetError::AuthFailed { .. }));
    }

    // ─── auth_capabilities() — ADR-027 §Phase 4 ────────────────────────────

    #[test]
    fn e2b_empty() {
        let target = E2bTarget::new("sandbox", "default");
        assert!(target.auth_capabilities().is_empty());
    }
}
