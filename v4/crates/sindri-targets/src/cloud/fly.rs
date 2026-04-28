//! Fly.io target.
//!
//! Wave 6B (audit D2/D3) wires native HTTP calls to the public Fly
//! Machines API at `https://api.machines.dev/v1` so the convergence
//! engine can drive create / update / destroy without shelling out to
//! `flyctl`. The `flyctl ssh console` path is preserved for `exec`
//! because the Machines API does not expose an exec endpoint over
//! HTTPS — that's still a flyctl wormhole.
//!
//! Auth: Personal Access Token via `auth.token`. fly.io does not
//! publish a documented OAuth Device Authorization Grant endpoint, so
//! the `target auth` wizard preserves the upstream-CLI hint for fly.
//! Operators issue tokens via the dashboard or `flyctl auth login`.
use crate::auth::AuthValue;
use crate::error::TargetError;
use crate::traits::{PrereqCheck, Target};
use sindri_core::platform::{Arch, Capabilities, Os, Platform, TargetProfile};
use std::path::Path;

/// Fly.io app target. The Machines API drives create/update/destroy;
/// `flyctl` is still used for `exec` (SSH).
pub struct FlyTarget {
    pub name: String,
    pub app_name: String,
    pub region: Option<String>,
    /// Image to launch (defaults to a generic Linux image — callers
    /// override via `targets.<name>.infra.image`).
    pub image: String,
    /// Resolved auth token source.
    pub auth: Option<AuthValue>,
    /// Machine ID once provisioned.
    pub machine_id: Option<String>,
    /// Base URL for the Fly Machines API. Overridable in tests.
    pub base_url: String,
}

impl FlyTarget {
    /// Construct a new Fly target.
    pub fn new(name: &str, app_name: &str) -> Self {
        FlyTarget {
            name: name.to_string(),
            app_name: app_name.to_string(),
            region: None,
            image: "flyio/hellofly:latest".into(),
            auth: None,
            machine_id: None,
            base_url: "https://api.machines.dev".into(),
        }
    }

    /// Resolve the API token from the configured `AuthValue` or the
    /// `FLY_API_TOKEN` env var as a fallback (matches `flyctl`).
    fn resolve_token(&self) -> Result<String, TargetError> {
        if let Some(av) = &self.auth {
            return av.resolve();
        }
        std::env::var("FLY_API_TOKEN")
            .or_else(|_| std::env::var("FLY_ACCESS_TOKEN"))
            .map_err(|_| TargetError::AuthFailed {
                target: self.name.clone(),
                detail: "FLY_API_TOKEN not set and auth.token not configured".into(),
            })
    }

    /// Build the JSON body posted to `POST /v1/apps/{app}/machines`.
    pub fn create_payload(&self, desired: Option<&serde_json::Value>) -> serde_json::Value {
        let image = desired
            .and_then(|d| d.get("image"))
            .and_then(|v| v.as_str())
            .unwrap_or(&self.image)
            .to_string();
        let region = desired
            .and_then(|d| d.get("region"))
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .or_else(|| self.region.clone());
        let mut payload = serde_json::json!({
            "config": {
                "image": image,
            },
        });
        if let Some(r) = region {
            payload["region"] = serde_json::Value::String(r);
        }
        payload
    }

    /// Async HTTP dispatch — `POST /v1/apps/{app}/machines`. Returns
    /// the machine ID.
    pub async fn dispatch_create_async(
        &self,
        desired: Option<&serde_json::Value>,
    ) -> Result<String, TargetError> {
        let token = self.resolve_token()?;
        let url = format!("{}/v1/apps/{}/machines", self.base_url, self.app_name);
        let payload = self.create_payload(desired);
        let client = reqwest::Client::new();
        let resp = client
            .post(&url)
            .bearer_auth(&token)
            .json(&payload)
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
                detail: "Fly API returned 401 — check FLY_API_TOKEN".into(),
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
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| TargetError::Http {
                target: self.name.clone(),
                detail: "response missing 'id' field".into(),
            })?
            .to_string();
        Ok(id)
    }

    /// Async HTTP dispatch — `DELETE /v1/apps/{app}/machines/{id}`.
    /// Idempotent: 404 is treated as success.
    pub async fn dispatch_destroy_async(&self, machine_id: &str) -> Result<(), TargetError> {
        let token = self.resolve_token()?;
        let url = format!(
            "{}/v1/apps/{}/machines/{}?force=true",
            self.base_url, self.app_name, machine_id
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
                detail: "Fly API returned 401 — check FLY_API_TOKEN".into(),
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

    /// Async HTTP dispatch — `POST /v1/apps/{app}/machines/{id}` (Fly's
    /// "update" verb on a machine, which replaces the config in place).
    pub async fn dispatch_update_async(
        &self,
        machine_id: &str,
        desired: &serde_json::Value,
    ) -> Result<serde_json::Value, TargetError> {
        let token = self.resolve_token()?;
        let url = format!(
            "{}/v1/apps/{}/machines/{}",
            self.base_url, self.app_name, machine_id
        );
        let payload = self.create_payload(Some(desired));
        let client = reqwest::Client::new();
        let resp = client
            .post(&url)
            .bearer_auth(&token)
            .json(&payload)
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
                detail: "Fly API returned 401 — check FLY_API_TOKEN".into(),
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
}

impl Target for FlyTarget {
    fn name(&self) -> &str {
        &self.name
    }
    fn kind(&self) -> &str {
        "fly"
    }

    fn profile(&self) -> Result<TargetProfile, TargetError> {
        // Fly Machines run Linux on x86_64 or aarch64. Default to x86_64; a
        // future change can probe the Machines API for the actual arch.
        Ok(TargetProfile {
            platform: Platform {
                os: Os::Linux,
                arch: Arch::X86_64,
            },
            capabilities: Capabilities::default(),
        })
    }

    fn exec(&self, cmd: &str, _env: &[(&str, &str)]) -> Result<(String, String), TargetError> {
        // The Machines API does not expose exec; flyctl's SSH console is
        // still the supported path.
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
        // Sync wrapper around the async dispatch for `Target::create`.
        let id = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| TargetError::Http {
                target: self.name.clone(),
                detail: format!("failed to build tokio runtime: {}", e),
            })?
            .block_on(self.dispatch_create_async(None))?;
        tracing::info!(target = %self.name, machine_id = %id, "Fly machine provisioned");
        Ok(())
    }

    fn check_prerequisites(&self) -> Vec<PrereqCheck> {
        let mut out = Vec::new();
        match self.resolve_token() {
            Ok(_) => out.push(PrereqCheck::ok("Fly API token resolves")),
            Err(_) => out.push(PrereqCheck::fail(
                "Fly API token resolves",
                "Set FLY_API_TOKEN or configure auth.token in sindri.yaml",
            )),
        }
        out.push(if crate::traits::which("flyctl").is_some() {
            PrereqCheck::ok("flyctl CLI (for exec)")
        } else {
            PrereqCheck::fail(
                "flyctl CLI (for exec)",
                "curl -L https://fly.io/install.sh | sh",
            )
        });
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_target(base_url: &str) -> FlyTarget {
        FlyTarget {
            name: "test-fly".into(),
            app_name: "myapp".into(),
            region: Some("ord".into()),
            image: "img:1".into(),
            auth: Some(AuthValue::Plain("tok-fly".into())),
            machine_id: None,
            base_url: base_url.to_string(),
        }
    }

    #[test]
    fn create_payload_carries_image_and_region() {
        let t = FlyTarget::new("t", "myapp");
        let p = t.create_payload(Some(&serde_json::json!({
            "image": "myrepo/myimg:v2",
            "region": "iad",
        })));
        assert_eq!(p["config"]["image"], "myrepo/myimg:v2");
        assert_eq!(p["region"], "iad");
    }

    #[tokio::test]
    async fn http_create_success_returns_machine_id() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/apps/myapp/machines"))
            .and(header("authorization", "Bearer tok-fly"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "mach-1"})),
            )
            .mount(&server)
            .await;
        let t = make_target(&server.uri());
        let id = t.dispatch_create_async(None).await.unwrap();
        assert_eq!(id, "mach-1");
    }

    #[tokio::test]
    async fn http_destroy_success() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/v1/apps/myapp/machines/mach-1"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;
        let t = make_target(&server.uri());
        t.dispatch_destroy_async("mach-1").await.unwrap();
    }

    #[tokio::test]
    async fn http_destroy_404_treated_as_success() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/v1/apps/myapp/machines/gone"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        let t = make_target(&server.uri());
        t.dispatch_destroy_async("gone").await.unwrap();
    }

    #[tokio::test]
    async fn http_update_replaces_config() {
        use wiremock::matchers::{body_string_contains, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/apps/myapp/machines/mach-1"))
            .and(body_string_contains("img:2"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "mach-1"})),
            )
            .mount(&server)
            .await;
        let t = make_target(&server.uri());
        let desired = serde_json::json!({"image": "img:2"});
        let got = t.dispatch_update_async("mach-1", &desired).await.unwrap();
        assert_eq!(got["id"], "mach-1");
    }

    #[tokio::test]
    async fn http_create_401_returns_auth_failed() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/apps/myapp/machines"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;
        let t = make_target(&server.uri());
        let err = t.dispatch_create_async(None).await.unwrap_err();
        assert!(matches!(err, TargetError::AuthFailed { .. }));
    }
}
