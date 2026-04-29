//! [`VaultBackend`] — HashiCorp Vault KV v2 secret store.
//!
//! Moved from `sindri/src/commands/secrets.rs` (Wave 6F, PR #235) and
//! factored into a reusable async backend.
//!
//! ## Configuration
//!
//! | Field         | Env override              | Default                   |
//! |---------------|---------------------------|---------------------------|
//! | `addr`        | `VAULT_ADDR`              | `http://127.0.0.1:8200`   |
//! | `token`       | `VAULT_TOKEN`             | —  (required)             |
//! | `mount`       | —                         | `secret`                  |
//! | `path_prefix` | —                         | `sindri`                  |
//!
//! Secrets are stored at `<mount>/data/<path_prefix>/<name>` and read back
//! from the `data.data.value` field in the KV v2 response.

use crate::{SecretStore, SecretValue, SecretsError};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde_json::json;
use tracing::debug;

/// Vault KV v2 backend.
#[derive(Debug, Clone)]
pub struct VaultBackend {
    client: reqwest::Client,
    addr: String,
    token: String,
    mount: String,
    path_prefix: String,
}

impl VaultBackend {
    /// Construct from explicit parameters.
    pub fn new(
        addr: impl Into<String>,
        token: impl Into<String>,
        mount: impl Into<String>,
        path_prefix: impl Into<String>,
    ) -> Self {
        Self {
            client: reqwest::Client::builder()
                .use_rustls_tls()
                .build()
                .expect("reqwest TLS client init"),
            addr: addr.into(),
            token: token.into(),
            mount: mount.into(),
            path_prefix: path_prefix.into(),
        }
    }

    /// Construct from environment variables (`VAULT_ADDR`, `VAULT_TOKEN`).
    /// Returns `None` if `VAULT_TOKEN` is not set.
    pub fn from_env() -> Option<Self> {
        let token = std::env::var("VAULT_TOKEN").ok()?;
        let addr = std::env::var("VAULT_ADDR").unwrap_or_else(|_| "http://127.0.0.1:8200".into());
        Some(Self::new(addr, token, "secret", "sindri"))
    }

    fn kv_data_url(&self, name: &str) -> String {
        format!(
            "{}/v1/{}/data/{}/{}",
            self.addr.trim_end_matches('/'),
            self.mount,
            self.path_prefix,
            name
        )
    }

    fn kv_metadata_url(&self, name: &str) -> String {
        format!(
            "{}/v1/{}/metadata/{}/{}",
            self.addr.trim_end_matches('/'),
            self.mount,
            self.path_prefix,
            name
        )
    }

    fn kv_list_url(&self) -> String {
        format!(
            "{}/v1/{}/metadata/{}",
            self.addr.trim_end_matches('/'),
            self.mount,
            self.path_prefix
        )
    }

    fn default_headers(&self) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(
            "X-Vault-Token",
            HeaderValue::from_str(&self.token).expect("vault token must be ASCII"),
        );
        h.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        h
    }

    /// Map a Vault HTTP response status to a [`SecretsError`].
    fn map_status(status: reqwest::StatusCode, name: &str) -> Option<SecretsError> {
        match status.as_u16() {
            200 | 204 => None,
            403 | 401 => Some(SecretsError::VaultAuth(format!(
                "permission denied for '{}'",
                name
            ))),
            404 => Some(SecretsError::NotFound {
                name: name.to_string(),
            }),
            other => Some(SecretsError::VaultHttp(format!(
                "Vault returned HTTP {} for '{}'",
                other, name
            ))),
        }
    }
}

#[async_trait]
impl SecretStore for VaultBackend {
    async fn read(&self, name: &str) -> Result<SecretValue, SecretsError> {
        let url = self.kv_data_url(name);
        debug!(url = %url, "VaultBackend::read");
        let resp = self
            .client
            .get(&url)
            .headers(self.default_headers())
            .send()
            .await
            .map_err(|e| SecretsError::VaultHttp(e.to_string()))?;
        if let Some(err) = Self::map_status(resp.status(), name) {
            return Err(err);
        }
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| SecretsError::Serde(e.to_string()))?;
        let value = body
            .pointer("/data/data/value")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                SecretsError::Serde(format!(
                    "Vault response for '{}' missing /data/data/value",
                    name
                ))
            })?;
        let description = body
            .pointer("/data/data/description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let mut sv = SecretValue::from_plaintext(value);
        sv.description = description;
        Ok(sv)
    }

    async fn write(&self, name: &str, value: SecretValue) -> Result<(), SecretsError> {
        let url = self.kv_data_url(name);
        debug!(url = %url, "VaultBackend::write");
        let str_val = value
            .expose_str()
            .map_err(|e| SecretsError::Other(format!("secret is not valid UTF-8: {}", e)))?;
        let mut data = json!({ "value": str_val });
        if let Some(desc) = &value.description {
            data["description"] = json!(desc);
        }
        let body = json!({ "data": data });
        let resp = self
            .client
            .post(&url)
            .headers(self.default_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| SecretsError::VaultHttp(e.to_string()))?;
        if let Some(err) = Self::map_status(resp.status(), name) {
            return Err(err);
        }
        Ok(())
    }

    async fn delete(&self, name: &str) -> Result<(), SecretsError> {
        let url = self.kv_metadata_url(name);
        debug!(url = %url, "VaultBackend::delete");
        let resp = self
            .client
            .delete(&url)
            .headers(self.default_headers())
            .send()
            .await
            .map_err(|e| SecretsError::VaultHttp(e.to_string()))?;
        if let Some(err) = Self::map_status(resp.status(), name) {
            return Err(err);
        }
        Ok(())
    }

    async fn list(&self) -> Result<Vec<String>, SecretsError> {
        // KV v2 LIST uses the metadata path with a ?list=true query or
        // the VAULT LIST verb.  We use ?list=true on GET as it is HTTP/1.1
        // compatible without a custom method.
        let url = format!("{}?list=true", self.kv_list_url());
        debug!(url = %url, "VaultBackend::list");
        let resp = self
            .client
            .get(&url)
            .headers(self.default_headers())
            .send()
            .await
            .map_err(|e| SecretsError::VaultHttp(e.to_string()))?;
        if resp.status().as_u16() == 404 {
            return Ok(Vec::new());
        }
        if let Some(err) = Self::map_status(resp.status(), "<list>") {
            return Err(err);
        }
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| SecretsError::Serde(e.to_string()))?;
        let keys = body
            .pointer("/data/keys")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|k| k.as_str())
                    .map(|s| s.trim_end_matches('/').to_string())
                    .collect()
            })
            .unwrap_or_default();
        Ok(keys)
    }
}

// ── tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// A backend pointing at an address that should always refuse connections.
    fn dead_backend() -> VaultBackend {
        VaultBackend::new(
            "http://127.0.0.1:19999", // almost certainly nothing listening
            "s.XXXXXXXX",
            "secret",
            "sindri",
        )
    }

    #[test]
    fn kv_data_url_built_correctly() {
        let b = VaultBackend::new("http://vault:8200", "tok", "kv", "myapp");
        assert_eq!(
            b.kv_data_url("targets.fly1.auth.token"),
            "http://vault:8200/v1/kv/data/myapp/targets.fly1.auth.token"
        );
    }

    #[test]
    fn kv_list_url_built_correctly() {
        let b = VaultBackend::new("http://vault:8200/", "tok", "secret", "sindri");
        assert_eq!(
            b.kv_list_url(),
            "http://vault:8200/v1/secret/metadata/sindri"
        );
    }

    #[test]
    fn map_status_401_returns_vault_auth_error() {
        let err = VaultBackend::map_status(reqwest::StatusCode::UNAUTHORIZED, "my-secret");
        assert!(matches!(err, Some(SecretsError::VaultAuth(_))));
    }

    #[test]
    fn map_status_404_returns_not_found() {
        let err = VaultBackend::map_status(reqwest::StatusCode::NOT_FOUND, "my-secret");
        assert!(matches!(err, Some(SecretsError::NotFound { .. })));
    }

    #[test]
    fn map_status_200_returns_none() {
        let err = VaultBackend::map_status(reqwest::StatusCode::OK, "x");
        assert!(err.is_none());
    }

    #[tokio::test]
    async fn read_connection_refused_returns_vault_http_error() {
        let b = dead_backend();
        let result = b.read("some-secret").await;
        assert!(
            matches!(result, Err(SecretsError::VaultHttp(_))),
            "unexpected: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn write_connection_refused_returns_vault_http_error() {
        let b = dead_backend();
        let result = b.write("k", SecretValue::from_plaintext("v")).await;
        assert!(matches!(result, Err(SecretsError::VaultHttp(_))));
    }

    #[tokio::test]
    async fn delete_connection_refused_returns_vault_http_error() {
        let b = dead_backend();
        let result = b.delete("k").await;
        assert!(matches!(result, Err(SecretsError::VaultHttp(_))));
    }

    #[tokio::test]
    async fn list_connection_refused_returns_vault_http_error() {
        let b = dead_backend();
        let result = b.list().await;
        assert!(matches!(result, Err(SecretsError::VaultHttp(_))));
    }
}
