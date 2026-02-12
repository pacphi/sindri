//! HashiCorp Vault secret source - simplified implementation for Phase 5

use crate::sources::SecretSource;
use crate::types::{ResolutionContext, ResolvedFrom, ResolvedSecret, SecretMetadata, SecretValue};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use sindri_core::types::{SecretConfig, SecretSource as ConfigSecretSource};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, warn};
use vaultrs::client::{Client, VaultClient, VaultClientSettingsBuilder};
use vaultrs::{kv2, token};

use crate::types::TokenMetadata;

pub struct VaultSource {
    /// Configuration
    config: Arc<VaultConfig>,
    /// Token metadata cache
    token_cache: Arc<RwLock<Option<TokenMetadata>>>,
}

#[derive(Debug, Clone)]
pub struct VaultConfig {
    pub address: String,
    pub token: String,
    pub namespace: Option<String>,
    pub timeout: Duration,
    pub retry: RetryConfig,
    pub insecure_skip_verify: bool,
}

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(10),
        }
    }
}

impl VaultConfig {
    pub fn from_env() -> Result<Self> {
        let address =
            std::env::var("VAULT_ADDR").context("VAULT_ADDR environment variable not set")?;
        let token =
            std::env::var("VAULT_TOKEN").context("VAULT_TOKEN environment variable not set")?;
        let namespace = std::env::var("VAULT_NAMESPACE").ok();
        let timeout_secs = std::env::var("VAULT_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);
        let insecure_skip_verify = std::env::var("VAULT_SKIP_VERIFY")
            .ok()
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        Ok(Self {
            address,
            token,
            namespace,
            timeout: Duration::from_secs(timeout_secs),
            retry: RetryConfig::default(),
            insecure_skip_verify,
        })
    }
}

impl VaultSource {
    pub fn new() -> Self {
        let config = VaultConfig::from_env().unwrap_or_else(|_| VaultConfig {
            address: String::new(),
            token: String::new(),
            namespace: None,
            timeout: Duration::from_secs(30),
            retry: RetryConfig::default(),
            insecure_skip_verify: false,
        });

        Self {
            config: Arc::new(config),
            token_cache: Arc::new(RwLock::new(None)),
        }
    }

    pub fn with_config(config: VaultConfig) -> Self {
        Self {
            config: Arc::new(config),
            token_cache: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a new Vault client
    fn create_client(&self) -> Result<VaultClient> {
        let mut settings = VaultClientSettingsBuilder::default();
        settings.address(&self.config.address);
        settings.timeout(Some(self.config.timeout));

        if let Some(ns) = &self.config.namespace {
            settings.namespace(Some(ns.clone()));
        }

        if self.config.insecure_skip_verify {
            warn!("TLS verification disabled");
            settings.verify(false);
        }

        let mut client = VaultClient::new(settings.build()?)?;
        client.set_token(&self.config.token);
        Ok(client)
    }

    async fn read_secret_with_retry(&self, mount: &str, path: &str, key: &str) -> Result<String> {
        let mut attempt = 0;
        let mut delay = self.config.retry.base_delay;

        loop {
            let client = self.create_client()?;

            match kv2::read::<serde_json::Value>(&client, mount, path).await {
                Ok(response) => {
                    let value = response
                        .get(key)
                        .ok_or_else(|| anyhow!("Key '{}' not found in secret", key))?
                        .as_str()
                        .ok_or_else(|| anyhow!("Secret value is not a string"))?;

                    debug!("Successfully read secret from Vault: {}/{}", mount, path);
                    return Ok(value.to_string());
                }
                Err(e) if attempt < self.config.retry.max_attempts => {
                    warn!(
                        "Vault request failed (attempt {}/{}): {}",
                        attempt + 1,
                        self.config.retry.max_attempts,
                        e
                    );
                    tokio::time::sleep(delay).await;
                    delay = std::cmp::min(delay * 2, self.config.retry.max_delay);
                    attempt += 1;
                }
                Err(e) => {
                    return Err(e).context(format!(
                        "Failed to read from Vault after {} attempts",
                        self.config.retry.max_attempts
                    ));
                }
            }
        }
    }

    pub fn is_configured(&self) -> bool {
        !self.config.address.is_empty() && !self.config.token.is_empty()
    }

    pub async fn validate_token(&self) -> Result<()> {
        let client = self.create_client()?;
        let token = &self.config.token;

        match token::lookup(&client, token).await {
            Ok(token_info) => {
                debug!("Token validation successful");

                let mut token_lock = self.token_cache.write().await;
                *token_lock = Some(TokenMetadata {
                    token: token.clone(),
                    ttl: token_info.ttl,
                    renewable: token_info.renewable,
                    last_renewed: std::time::SystemTime::now(),
                });

                Ok(())
            }
            Err(e) => Err(e).context("Token validation failed"),
        }
    }

    pub async fn check_health(&self) -> Result<bool> {
        let client = reqwest::Client::new();
        let url = format!("{}/v1/sys/health", self.config.address);

        match client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
        {
            Ok(response) => Ok(response.status().is_success()),
            Err(e) => Err(e).context("Failed to check Vault health"),
        }
    }
}

impl Default for VaultSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SecretSource for VaultSource {
    async fn resolve(
        &self,
        definition: &SecretConfig,
        _ctx: &ResolutionContext,
    ) -> Result<Option<ResolvedSecret>> {
        if definition.source != ConfigSecretSource::Vault {
            return Ok(None);
        }

        if !self.is_configured() {
            if definition.required {
                return Err(anyhow!(
                    "Vault not configured (VAULT_ADDR and VAULT_TOKEN required)"
                ));
            } else {
                debug!("Vault not configured, skipping optional secret");
                return Ok(None);
            }
        }

        let vault_path = definition
            .vault_path
            .as_ref()
            .ok_or_else(|| anyhow!("Vault path not specified"))?;
        let vault_key = definition
            .vault_key
            .as_ref()
            .ok_or_else(|| anyhow!("Vault key not specified"))?;
        let mount = &definition.vault_mount;

        let value = self
            .read_secret_with_retry(mount, vault_path, vault_key)
            .await?;
        let size_bytes = value.len();

        debug!(
            "Resolved secret '{}' from Vault: {}/{}",
            definition.name, mount, vault_path
        );

        Ok(Some(ResolvedSecret {
            name: definition.name.clone(),
            value: SecretValue::from_env(value),
            metadata: SecretMetadata {
                source_type: ConfigSecretSource::Vault,
                resolved_from: ResolvedFrom::Vault {
                    path: vault_path.clone(),
                    mount: mount.clone(),
                },
                size_bytes,
            },
        }))
    }

    fn validate(&self) -> Result<()> {
        if !self.is_configured() {
            return Err(anyhow!("Vault not configured"));
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "vault"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_config_from_env() {
        std::env::set_var("VAULT_ADDR", "https://vault.example.com");
        std::env::set_var("VAULT_TOKEN", "test-token");
        std::env::set_var("VAULT_NAMESPACE", "test-ns");

        let config = VaultConfig::from_env().unwrap();
        assert_eq!(config.address, "https://vault.example.com");
        assert_eq!(config.token, "test-token");
        assert_eq!(config.namespace, Some("test-ns".to_string()));

        std::env::remove_var("VAULT_ADDR");
        std::env::remove_var("VAULT_TOKEN");
        std::env::remove_var("VAULT_NAMESPACE");
    }

    #[test]
    fn test_is_configured() {
        let config = VaultConfig {
            address: "https://vault.example.com".to_string(),
            token: "test-token".to_string(),
            namespace: None,
            timeout: Duration::from_secs(30),
            retry: RetryConfig::default(),
            insecure_skip_verify: false,
        };

        let source = VaultSource::with_config(config);
        assert!(source.is_configured());
    }

    // --- Error path tests ---

    #[test]
    fn test_vault_config_from_env_missing_addr() {
        std::env::remove_var("VAULT_ADDR");
        std::env::set_var("VAULT_TOKEN", "test-token");

        let result = VaultConfig::from_env();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("VAULT_ADDR"),
            "Expected VAULT_ADDR error, got: {}",
            err
        );

        std::env::remove_var("VAULT_TOKEN");
    }

    #[test]
    fn test_vault_config_from_env_missing_token() {
        std::env::set_var("VAULT_ADDR", "https://vault.example.com");
        std::env::remove_var("VAULT_TOKEN");

        let result = VaultConfig::from_env();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("VAULT_TOKEN"),
            "Expected VAULT_TOKEN error, got: {}",
            err
        );

        std::env::remove_var("VAULT_ADDR");
    }

    #[test]
    fn test_is_not_configured_empty_address() {
        let config = VaultConfig {
            address: String::new(),
            token: "test-token".to_string(),
            namespace: None,
            timeout: Duration::from_secs(30),
            retry: RetryConfig::default(),
            insecure_skip_verify: false,
        };

        let source = VaultSource::with_config(config);
        assert!(!source.is_configured());
    }

    #[test]
    fn test_is_not_configured_empty_token() {
        let config = VaultConfig {
            address: "https://vault.example.com".to_string(),
            token: String::new(),
            namespace: None,
            timeout: Duration::from_secs(30),
            retry: RetryConfig::default(),
            insecure_skip_verify: false,
        };

        let source = VaultSource::with_config(config);
        assert!(!source.is_configured());
    }

    #[test]
    fn test_validate_unconfigured_vault() {
        let config = VaultConfig {
            address: String::new(),
            token: String::new(),
            namespace: None,
            timeout: Duration::from_secs(30),
            retry: RetryConfig::default(),
            insecure_skip_verify: false,
        };

        let source = VaultSource::with_config(config);
        let result = source.validate();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Vault not configured"),
            "Expected 'Vault not configured', got: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_resolve_required_secret_unconfigured_vault() {
        let config = VaultConfig {
            address: String::new(),
            token: String::new(),
            namespace: None,
            timeout: Duration::from_secs(30),
            retry: RetryConfig::default(),
            insecure_skip_verify: false,
        };

        let source = VaultSource::with_config(config);
        let secret_config = sindri_core::types::SecretConfig {
            name: "DB_PASSWORD".to_string(),
            source: ConfigSecretSource::Vault,
            from_file: None,
            required: true,
            path: None,
            mount_path: None,
            permissions: "0644".to_string(),
            vault_path: Some("secret/data/db".to_string()),
            vault_key: Some("password".to_string()),
            vault_mount: "secret".to_string(),
            s3_path: None,
        };

        let ctx = crate::types::ResolutionContext::new(std::path::PathBuf::from("/tmp"));
        let result = source.resolve(&secret_config, &ctx).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Vault not configured"),
            "Expected 'Vault not configured', got: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_resolve_optional_secret_unconfigured_returns_none() {
        let config = VaultConfig {
            address: String::new(),
            token: String::new(),
            namespace: None,
            timeout: Duration::from_secs(30),
            retry: RetryConfig::default(),
            insecure_skip_verify: false,
        };

        let source = VaultSource::with_config(config);
        let secret_config = sindri_core::types::SecretConfig {
            name: "OPTIONAL_SECRET".to_string(),
            source: ConfigSecretSource::Vault,
            from_file: None,
            required: false,
            path: None,
            mount_path: None,
            permissions: "0644".to_string(),
            vault_path: Some("secret/data/test".to_string()),
            vault_key: Some("key".to_string()),
            vault_mount: "secret".to_string(),
            s3_path: None,
        };

        let ctx = crate::types::ResolutionContext::new(std::path::PathBuf::from("/tmp"));
        let result = source.resolve(&secret_config, &ctx).await.unwrap();
        assert!(
            result.is_none(),
            "Optional secret on unconfigured vault should return None"
        );
    }

    #[tokio::test]
    async fn test_resolve_missing_vault_path() {
        let config = VaultConfig {
            address: "https://vault.example.com".to_string(),
            token: "test-token".to_string(),
            namespace: None,
            timeout: Duration::from_secs(30),
            retry: RetryConfig::default(),
            insecure_skip_verify: false,
        };

        let source = VaultSource::with_config(config);
        let secret_config = sindri_core::types::SecretConfig {
            name: "MISSING_PATH".to_string(),
            source: ConfigSecretSource::Vault,
            from_file: None,
            required: true,
            path: None,
            mount_path: None,
            permissions: "0644".to_string(),
            vault_path: None, // Missing
            vault_key: Some("key".to_string()),
            vault_mount: "secret".to_string(),
            s3_path: None,
        };

        let ctx = crate::types::ResolutionContext::new(std::path::PathBuf::from("/tmp"));
        let result = source.resolve(&secret_config, &ctx).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Vault path not specified"),
            "Expected 'Vault path not specified', got: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_resolve_missing_vault_key() {
        let config = VaultConfig {
            address: "https://vault.example.com".to_string(),
            token: "test-token".to_string(),
            namespace: None,
            timeout: Duration::from_secs(30),
            retry: RetryConfig::default(),
            insecure_skip_verify: false,
        };

        let source = VaultSource::with_config(config);
        let secret_config = sindri_core::types::SecretConfig {
            name: "MISSING_KEY".to_string(),
            source: ConfigSecretSource::Vault,
            from_file: None,
            required: true,
            path: None,
            mount_path: None,
            permissions: "0644".to_string(),
            vault_path: Some("secret/data/test".to_string()),
            vault_key: None, // Missing
            vault_mount: "secret".to_string(),
            s3_path: None,
        };

        let ctx = crate::types::ResolutionContext::new(std::path::PathBuf::from("/tmp"));
        let result = source.resolve(&secret_config, &ctx).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Vault key not specified"),
            "Expected 'Vault key not specified', got: {}",
            err
        );
    }
}
