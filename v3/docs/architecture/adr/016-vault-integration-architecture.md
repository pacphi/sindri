# ADR 016: HashiCorp Vault Integration Architecture

**Status**: Accepted
**Date**: 2026-01-21
**Deciders**: Core Team
**Phase**: Phase 5 - Secrets Management
**Related**: [ADR-001: Workspace Architecture](001-rust-migration-workspace-architecture.md), [ADR-015: Secrets Resolver](015-secrets-resolver-core-architecture.md)

## Context

Sindri currently supports HashiCorp Vault as a secret source through the bash `secrets-manager` module, which shells out to the Vault CLI. The Phase 5 Rust migration requires a native, async Vault client with production-grade features.

### Current Bash Implementation Limitations

1. **CLI Dependency**: Requires `vault` binary installed on system
2. **Synchronous**: Blocks during Vault operations
3. **Limited Error Context**: CLI stderr parsing is fragile
4. **No Connection Pooling**: New process per request
5. **Token Management**: Manual renewal logic with limited lifecycle support
6. **No Caching**: Secrets fetched on every resolve

### Requirements

- **KV v1/v2 Support**: Both secret engine versions
- **Multiple Auth Methods**: Token, AppRole, Kubernetes service accounts
- **Lease Management**: Automatic renewal for dynamic secrets
- **Error Recovery**: Graceful degradation on Vault unavailability
- **Performance**: Connection pooling, configurable timeouts
- **Security**: Memory zeroing, audit logging, no secret leakage
- **Configuration**: Path templating with environment variables

## Decision

### 1. Vault Client Architecture

**Decision**: Use the `vaultrs` crate (v0.7.4+) as our Vault client library.

**Rationale**:

- Most mature async Rust client (158k downloads/month)
- Comprehensive API coverage (KV v1/v2, auth methods, dynamic secrets)
- Built on `reqwest` and `tokio` (already in our stack)
- Active maintenance and security updates
- Structured error types via `thiserror`

### 2. Vault Client Wrapper

```rust
// sindri-rs/crates/sindri-secrets/src/vault/client.rs

use anyhow::{anyhow, Context, Result};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, warn};
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};
use vaultrs::kv2;

/// Vault client with connection pooling and lease management
pub struct SindriVaultClient {
    /// Underlying vaultrs client
    client: Arc<VaultClient>,

    /// Configuration
    config: VaultConfig,

    /// Token metadata cache
    token_cache: Arc<RwLock<TokenMetadata>>,
}

#[derive(Debug, Clone)]
pub struct VaultConfig {
    /// Vault server address (VAULT_ADDR)
    pub address: String,

    /// Authentication method
    pub auth: VaultAuth,

    /// Optional namespace (VAULT_NAMESPACE)
    pub namespace: Option<String>,

    /// Request timeout
    pub timeout: Duration,

    /// Retry configuration
    pub retry: RetryConfig,

    /// TLS configuration
    pub tls: TlsConfig,

    /// Whether to cache secrets locally
    pub enable_cache: bool,

    /// Cache TTL
    pub cache_ttl: Duration,
}

#[derive(Debug, Clone)]
pub enum VaultAuth {
    /// Token-based auth (VAULT_TOKEN)
    Token(String),

    /// AppRole auth (role_id, secret_id)
    AppRole { role_id: String, secret_id: String },

    /// Kubernetes service account auth
    Kubernetes { role: String, jwt_path: String },
}

#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum retry attempts
    pub max_attempts: u32,

    /// Base delay between retries
    pub base_delay: Duration,

    /// Maximum delay (with exponential backoff)
    pub max_delay: Duration,

    /// Whether to retry on 5xx errors
    pub retry_server_errors: bool,
}

#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// Path to CA certificate (VAULT_CACERT)
    pub ca_cert: Option<String>,

    /// Path to client certificate
    pub client_cert: Option<String>,

    /// Path to client key
    pub client_key: Option<String>,

    /// Skip TLS verification (insecure, dev only)
    pub insecure_skip_verify: bool,
}

impl SindriVaultClient {
    /// Create a new Vault client from environment and configuration
    pub async fn new() -> Result<Self> {
        let config = VaultConfig::from_env()?;
        Self::with_config(config).await
    }

    /// Create with explicit configuration
    pub async fn with_config(config: VaultConfig) -> Result<Self> {
        let mut settings = VaultClientSettingsBuilder::default()
            .address(&config.address)
            .timeout(Some(config.timeout));

        // Configure namespace
        if let Some(ns) = &config.namespace {
            settings = settings.namespace(ns);
        }

        // Configure TLS
        if let Some(ca_cert) = &config.tls.ca_cert {
            settings = settings.ca_certs(&[ca_cert.as_str()]);
        }

        if config.tls.insecure_skip_verify {
            warn!("TLS verification disabled - insecure mode");
            settings = settings.verify(false);
        }

        // Build client
        let client = VaultClient::new(settings.build()?)?;

        // Authenticate
        let token = Self::authenticate(&client, &config.auth).await?;
        client.set_token(&token);

        Ok(Self {
            client: Arc::new(client),
            config,
            token_cache: Arc::new(RwLock::new(TokenMetadata::new(token))),
        })
    }

    /// Authenticate with Vault
    async fn authenticate(client: &VaultClient, auth: &VaultAuth) -> Result<String> {
        match auth {
            VaultAuth::Token(token) => {
                // Validate token
                vaultrs::auth::token::lookup(client, token).await
                    .context("Failed to validate Vault token")?;
                Ok(token.clone())
            }
            VaultAuth::AppRole { role_id, secret_id } => {
                let response = vaultrs::auth::approle::login(client, role_id, secret_id).await
                    .context("AppRole authentication failed")?;
                Ok(response.client_token)
            }
            VaultAuth::Kubernetes { role, jwt_path } => {
                let jwt = tokio::fs::read_to_string(jwt_path).await
                    .context("Failed to read Kubernetes JWT")?;
                let response = vaultrs::auth::kubernetes::login(client, role, &jwt).await
                    .context("Kubernetes authentication failed")?;
                Ok(response.client_token)
            }
        }
    }
}
```

### 3. Secret Retrieval with Lease Management

```rust
// sindri-rs/crates/sindri-secrets/src/vault/retrieval.rs

impl SindriVaultClient {
    /// Read secret from KV v2 store with caching
    pub async fn read_secret_kv2(
        &self,
        mount: &str,
        path: &str,
        key: &str,
    ) -> Result<VaultSecret> {
        // Check cache first
        if self.config.enable_cache {
            if let Some(cached) = self.get_cached_secret(path, key).await? {
                debug!("Using cached secret for {}/{}", path, key);
                return Ok(cached);
            }
        }

        // Fetch from Vault with retries
        let secret = self.read_kv2_with_retry(mount, path, key).await?;

        // Cache if enabled
        if self.config.enable_cache {
            self.cache_secret(path, key, &secret).await?;
        }

        Ok(secret)
    }

    /// Read with exponential backoff retry
    async fn read_kv2_with_retry(
        &self,
        mount: &str,
        path: &str,
        key: &str,
    ) -> Result<VaultSecret> {
        let mut attempt = 0;
        let mut delay = self.config.retry.base_delay;

        loop {
            // Ensure token is valid
            self.ensure_token_valid().await?;

            match kv2::read(&self.client, mount, path).await {
                Ok(response) => {
                    let value = response.data.get(key)
                        .ok_or_else(|| anyhow!("Key '{}' not found in secret", key))?
                        .as_str()
                        .ok_or_else(|| anyhow!("Secret value is not a string"))?
                        .to_string();

                    return Ok(VaultSecret {
                        value,
                        version: response.metadata.version,
                        lease_id: None,
                        lease_duration: None,
                        renewable: false,
                    });
                }
                Err(e) if attempt < self.config.retry.max_attempts => {
                    warn!("Vault request failed (attempt {}): {}", attempt + 1, e);
                    tokio::time::sleep(delay).await;
                    delay = std::cmp::min(delay * 2, self.config.retry.max_delay);
                    attempt += 1;
                }
                Err(e) => return Err(e).context("Failed to read secret from Vault"),
            }
        }
    }

    /// Ensure token is valid and renew if needed
    async fn ensure_token_valid(&self) -> Result<()> {
        let mut token_meta = self.token_cache.write().await;

        // Check if renewal needed (< 1 hour remaining)
        if token_meta.needs_renewal() {
            info!("Renewing Vault token (TTL: {}s)", token_meta.ttl);

            match vaultrs::auth::token::renew(&self.client, None, None).await {
                Ok(renewed) => {
                    token_meta.update_from_renewal(renewed);
                    info!("Token renewed successfully");
                }
                Err(e) => {
                    warn!("Token renewal failed: {}", e);
                    // Continue with existing token, may fail downstream
                }
            }
        }

        Ok(())
    }
}
```

### 4. Configuration and Path Templating

```rust
// crates/sindri-secrets/src/vault/config.rs

impl VaultConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let address = env::var("VAULT_ADDR")
            .context("VAULT_ADDR not set")?;

        let auth = Self::auth_from_env()?;

        Ok(Self {
            address,
            auth,
            namespace: env::var("VAULT_NAMESPACE").ok(),
            timeout: Duration::from_secs(
                env::var("VAULT_TIMEOUT")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(30)
            ),
            retry: RetryConfig::default(),
            tls: TlsConfig::from_env(),
            enable_cache: env::var("SINDRI_VAULT_CACHE")
                .map(|v| v == "true")
                .unwrap_or(true),
            cache_ttl: Duration::from_secs(300), // 5 minutes
        })
    }
}

/// Template path with environment variable substitution
pub fn template_vault_path(path: &str, env_vars: &HashMap<String, String>) -> String {
    let mut result = path.to_string();

    // Replace ${VAR} and $VAR patterns
    for (key, value) in env_vars {
        result = result.replace(&format!("${{{}}}", key), value);
        result = result.replace(&format!("${}", key), value);
    }

    result
}
```

### 5. Security Considerations

**Memory Safety**:

```rust
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SecureString {
    inner: String,
}
```

**Audit Logging**:

```rust
info!(
    vault.operation = "read",
    vault.path = path,
    duration_ms = start.elapsed().as_millis() as u64,
    "Vault secret retrieved successfully"
);
```

## Consequences

### Positive

1. **Native Async**: Non-blocking Vault operations, concurrent secret fetching
2. **Production-Ready**: Connection pooling, retries, lease renewal
3. **Multi-Auth Support**: Token, AppRole, Kubernetes service accounts
4. **Type Safety**: Compile-time guarantees for Vault API calls
5. **Better Errors**: Structured error types with context
6. **Performance**: Local caching reduces Vault load
7. **Security**: Memory zeroing, audit logging
8. **Testability**: Mock Vault client for unit tests

### Negative

1. **Binary Size**: +500KB for vaultrs and dependencies
2. **Complexity**: More code than CLI shelling
3. **Maintenance**: Must track vaultrs updates

## Configuration Reference

### Environment Variables

```bash
# Required
VAULT_ADDR=https://vault.company.com

# Authentication (choose one)
VAULT_TOKEN=hvs.xxxxx
VAULT_ROLE_ID=xxx
VAULT_SECRET_ID=xxx
VAULT_K8S_ROLE=sindri

# Optional
VAULT_NAMESPACE=sindri
VAULT_TIMEOUT=30
VAULT_CACERT=/etc/ssl/certs/vault.crt
VAULT_SKIP_VERIFY=false
SINDRI_VAULT_CACHE=true
```

### sindri.yaml Example

```yaml
secrets:
  - name: DATABASE_PASSWORD
    source: vault
    vaultPath: secret/data/sindri/prod/database
    vaultKey: password
    vaultMount: secret
    required: true

  - name: API_KEY
    source: vault
    vaultPath: secret/data/sindri/${ENV}/${SERVICE}/api
    vaultKey: key
    required: true
```

## Related Decisions

- [ADR-001: Workspace Architecture](001-rust-migration-workspace-architecture.md)
- [ADR-004: Async Runtime](004-async-runtime-command-execution.md)
- [ADR-015: Secrets Resolver](015-secrets-resolver-core-architecture.md)
- [Secrets Management Documentation](../../../../v2/docs/SECRETS_MANAGEMENT.md)
