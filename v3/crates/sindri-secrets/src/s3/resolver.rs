//! S3 secret resolver
//!
//! Orchestrates secret resolution from S3 with encryption and caching.
//! Implements the full workflow: cache check -> S3 fetch -> decrypt -> cache.

use crate::s3::backend::S3Backend;
use crate::s3::cache::SecretCache;
use crate::s3::encryption::SecretEncryptor;
use crate::s3::types::{KeySource, S3SecretBackend, S3SecretMetadata, SyncConflict, SyncResult};
use anyhow::{anyhow, Context, Result};
use tracing::{debug, info, warn};

/// S3 secret resolver
///
/// Combines S3 backend, encryption, and caching for complete secret management.
pub struct S3SecretResolver {
    /// S3 backend for storage
    backend: S3Backend,
    /// Encryptor for encryption/decryption
    encryptor: SecretEncryptor,
    /// Local cache (optional)
    cache: Option<SecretCache>,
    /// Configuration
    config: S3SecretBackend,
}

impl S3SecretResolver {
    /// Create a new resolver from configuration
    pub async fn new(config: S3SecretBackend) -> Result<Self> {
        // Create S3 backend
        let backend = S3Backend::new(&config).await?;

        // Create encryptor based on key source
        let encryptor = Self::create_encryptor(&config)?;

        // Create cache if enabled
        let cache = if let Some(ref cache_config) = config.cache {
            if cache_config.enabled {
                Some(SecretCache::new(cache_config.clone()).await?)
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            backend,
            encryptor,
            cache,
            config,
        })
    }

    /// Create resolver with explicit components (for testing)
    pub fn with_components(
        backend: S3Backend,
        encryptor: SecretEncryptor,
        cache: Option<SecretCache>,
        config: S3SecretBackend,
    ) -> Self {
        Self {
            backend,
            encryptor,
            cache,
            config,
        }
    }

    /// Create encryptor from configuration
    fn create_encryptor(config: &S3SecretBackend) -> Result<SecretEncryptor> {
        match config.encryption.key_source {
            KeySource::Env => {
                let env_var = config
                    .encryption
                    .key_env
                    .as_ref()
                    .ok_or_else(|| anyhow!("key_env must be set when key_source is 'env'"))?;
                SecretEncryptor::from_env(env_var)
            }
            KeySource::File => {
                let key_file = config
                    .encryption
                    .key_file
                    .as_ref()
                    .ok_or_else(|| anyhow!("key_file must be set when key_source is 'file'"))?;
                SecretEncryptor::from_key_file(key_file)
            }
            KeySource::Kms => Err(anyhow!("KMS key source is not yet implemented")),
        }
    }

    /// Get the public key for this resolver's identity
    pub fn public_key(&self) -> String {
        self.encryptor.public_key()
    }

    /// Get the S3 backend
    pub fn backend(&self) -> &S3Backend {
        &self.backend
    }

    /// Resolve a secret by S3 path
    ///
    /// 1. Check local cache (if enabled)
    /// 2. Fetch from S3 if not cached
    /// 3. Decrypt the secret
    /// 4. Update cache
    pub async fn resolve(&self, s3_path: &str) -> Result<String> {
        // Check cache first
        if let Some(ref cache) = self.cache {
            if let Some(cached_value) = cache.get(s3_path).await? {
                debug!("Resolved secret from cache: {}", s3_path);
                return Ok(cached_value);
            }
        }

        // Fetch from S3
        debug!("Fetching secret from S3: {}", s3_path);
        let data = self.backend.get_secret(s3_path).await?;
        let metadata: S3SecretMetadata = serde_json::from_slice(&data)
            .with_context(|| format!("Failed to parse secret metadata for: {}", s3_path))?;

        // Decrypt
        let secret_value = self.encryptor.decrypt_secret(&metadata)?;

        // Update cache
        if let Some(ref cache) = self.cache {
            cache.set(s3_path, &secret_value).await?;
        }

        info!("Resolved secret from S3: {}", s3_path);
        Ok(secret_value)
    }

    /// Push a secret to S3
    ///
    /// 1. Encrypt the secret
    /// 2. Upload to S3
    /// 3. Invalidate cache
    pub async fn push(
        &self,
        name: &str,
        value: &str,
        s3_path: &str,
        additional_recipients: &[String],
    ) -> Result<String> {
        // Encrypt
        let metadata = self
            .encryptor
            .encrypt_secret(name, value, additional_recipients)?;

        // Upload to S3
        let version_id = self.backend.put_secret_metadata(s3_path, &metadata).await?;

        // Invalidate cache (we have new data now)
        if let Some(ref cache) = self.cache {
            cache.invalidate(s3_path).await?;
            // Re-cache with the new value
            cache.set(s3_path, value).await?;
        }

        info!("Pushed secret to S3: {} (version: {})", s3_path, version_id);
        Ok(version_id)
    }

    /// Check if a secret exists in S3
    pub async fn exists(&self, s3_path: &str) -> Result<bool> {
        self.backend.secret_exists(s3_path).await
    }

    /// Delete a secret from S3
    pub async fn delete(&self, s3_path: &str) -> Result<()> {
        // Delete from S3
        self.backend.delete_secret(s3_path).await?;

        // Invalidate cache
        if let Some(ref cache) = self.cache {
            cache.invalidate(s3_path).await?;
        }

        info!("Deleted secret from S3: {}", s3_path);
        Ok(())
    }

    /// List all secrets in S3
    pub async fn list(&self) -> Result<Vec<String>> {
        self.backend.list_secrets().await
    }

    /// Sync secrets between local cache and S3
    ///
    /// Compares local cache state with S3 and returns what needs to be synced.
    pub async fn sync_status(&self, local_secrets: &[LocalSecretInfo]) -> Result<SyncResult> {
        let mut result = SyncResult::default();

        // Get all secrets from S3
        let remote_secrets = self.backend.list_secrets().await?;

        // Build a map of remote secrets for quick lookup
        let remote_set: std::collections::HashSet<&str> =
            remote_secrets.iter().map(|s| s.as_str()).collect();

        // Check each local secret
        for local in local_secrets {
            if remote_set.contains(local.s3_path.as_str()) {
                // Secret exists in both places
                if let Some(ref cache) = self.cache {
                    if let Some(entry) = cache.get_entry(&local.s3_path).await {
                        // Check if versions match (if version info available)
                        if entry.version_id.is_some() || entry.etag.is_some() {
                            // We have version info, consider it in sync
                            result.in_sync.push(local.s3_path.clone());
                        } else {
                            // No version info, might need sync
                            result.to_pull.push(local.s3_path.clone());
                        }
                    } else {
                        // Not in cache, need to pull
                        result.to_pull.push(local.s3_path.clone());
                    }
                } else {
                    // No cache, always need to pull
                    result.to_pull.push(local.s3_path.clone());
                }
            } else {
                // Secret only exists locally, need to push
                result.to_push.push(local.s3_path.clone());
            }
        }

        // Check for secrets that exist in S3 but not locally
        let local_set: std::collections::HashSet<&str> =
            local_secrets.iter().map(|s| s.s3_path.as_str()).collect();

        for remote_path in &remote_secrets {
            if !local_set.contains(remote_path.as_str()) {
                // Secret only exists in S3, might need to pull or is orphaned
                result.to_pull.push(remote_path.clone());
            }
        }

        debug!(
            "Sync status: {} to push, {} to pull, {} in sync, {} conflicts",
            result.to_push.len(),
            result.to_pull.len(),
            result.in_sync.len(),
            result.conflicts.len()
        );

        Ok(result)
    }

    /// Perform full sync operation
    pub async fn sync(
        &self,
        local_secrets: &[LocalSecretInfo],
        direction: SyncDirection,
        dry_run: bool,
    ) -> Result<SyncResult> {
        let mut result = self.sync_status(local_secrets).await?;

        if dry_run {
            return Ok(result);
        }

        match direction {
            SyncDirection::Push => {
                // Push local secrets to S3
                for local in local_secrets {
                    if result.to_push.contains(&local.s3_path) {
                        if let Some(ref value) = local.value {
                            self.push(&local.name, value, &local.s3_path, &[]).await?;
                        }
                    }
                }
            }
            SyncDirection::Pull => {
                // Pull secrets from S3 to cache
                for path in &result.to_pull {
                    match self.resolve(path).await {
                        Ok(_) => {
                            debug!("Pulled secret: {}", path);
                        }
                        Err(e) => {
                            warn!("Failed to pull secret {}: {}", path, e);
                            result.conflicts.push(SyncConflict {
                                name: path.clone(),
                                local_modified: None,
                                remote_modified: None,
                                reason: format!("Failed to pull: {}", e),
                            });
                        }
                    }
                }
            }
            SyncDirection::Both => {
                // Push first, then pull
                for local in local_secrets {
                    if result.to_push.contains(&local.s3_path) {
                        if let Some(ref value) = local.value {
                            self.push(&local.name, value, &local.s3_path, &[]).await?;
                        }
                    }
                }

                for path in &result.to_pull {
                    match self.resolve(path).await {
                        Ok(_) => {
                            debug!("Pulled secret: {}", path);
                        }
                        Err(e) => {
                            warn!("Failed to pull secret {}: {}", path, e);
                            result.conflicts.push(SyncConflict {
                                name: path.clone(),
                                local_modified: None,
                                remote_modified: None,
                                reason: format!("Failed to pull: {}", e),
                            });
                        }
                    }
                }
            }
        }

        info!(
            "Sync complete: {} pushed, {} pulled, {} conflicts",
            result.to_push.len(),
            result.to_pull.len(),
            result.conflicts.len()
        );

        Ok(result)
    }

    /// Rotate secrets to a new key
    pub async fn rotate_key(
        &self,
        new_encryptor: &SecretEncryptor,
        secrets_to_rotate: &[String],
    ) -> Result<Vec<String>> {
        let mut rotated = Vec::new();

        for s3_path in secrets_to_rotate {
            // Fetch and decrypt with old key
            let data = self.backend.get_secret(s3_path).await?;
            let metadata: S3SecretMetadata = serde_json::from_slice(&data)?;
            let plaintext = self.encryptor.decrypt_secret(&metadata)?;

            // Re-encrypt with new key
            let new_metadata = new_encryptor.encrypt_secret(
                &metadata.secret_name,
                &plaintext,
                &[], // Could include additional recipients
            )?;

            // Upload re-encrypted secret
            self.backend
                .put_secret_metadata(s3_path, &new_metadata)
                .await?;

            // Invalidate cache
            if let Some(ref cache) = self.cache {
                cache.invalidate(s3_path).await?;
            }

            rotated.push(s3_path.clone());
            info!("Rotated secret: {}", s3_path);
        }

        Ok(rotated)
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> Option<crate::s3::cache::CacheStats> {
        if let Some(ref cache) = self.cache {
            Some(cache.stats().await)
        } else {
            None
        }
    }

    /// Clear the cache
    pub async fn clear_cache(&self) -> Result<()> {
        if let Some(ref cache) = self.cache {
            cache.clear().await?;
        }
        Ok(())
    }
}

impl std::fmt::Debug for S3SecretResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("S3SecretResolver")
            .field("backend", &self.backend)
            .field("cache", &self.cache.is_some())
            .field("bucket", &self.config.bucket)
            .finish_non_exhaustive()
    }
}

/// Information about a local secret for sync operations
#[derive(Debug, Clone)]
pub struct LocalSecretInfo {
    /// Secret name
    pub name: String,
    /// S3 path
    pub s3_path: String,
    /// Optional value (for pushing)
    pub value: Option<String>,
    /// Last modified time (local)
    pub last_modified: Option<String>,
}

/// Sync direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncDirection {
    /// Push local to remote
    Push,
    /// Pull remote to local
    Pull,
    /// Bidirectional sync
    Both,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_secret_info() {
        let info = LocalSecretInfo {
            name: "TEST_SECRET".to_string(),
            s3_path: "test/secret".to_string(),
            value: Some("secret-value".to_string()),
            last_modified: None,
        };

        assert_eq!(info.name, "TEST_SECRET");
        assert_eq!(info.s3_path, "test/secret");
        assert!(info.value.is_some());
    }

    #[test]
    fn test_sync_direction() {
        assert_eq!(SyncDirection::Push, SyncDirection::Push);
        assert_ne!(SyncDirection::Push, SyncDirection::Pull);
    }
}
