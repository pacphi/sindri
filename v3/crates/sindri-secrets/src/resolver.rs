//! Secret resolution orchestration
//!
//! Coordinates multiple secret sources with:
//! - Async parallel resolution
//! - In-memory caching with RwLock
//! - Required vs optional handling
//! - Clear error messages

use crate::sources::{EnvSource, FileSource, SecretSource, VaultSource};
use crate::types::{ResolvedSecret, ResolutionContext};
use anyhow::{anyhow, Result};
use sindri_core::types::{SecretConfig, SecretSource as ConfigSecretSource};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// In-memory cache with automatic cleanup
#[derive(Debug)]
pub struct SecretCache {
    secrets: HashMap<String, ResolvedSecret>,
}

impl SecretCache {
    pub fn new() -> Self {
        Self {
            secrets: HashMap::new(),
        }
    }

    pub fn insert(&mut self, secret: ResolvedSecret) {
        self.secrets.insert(secret.name.clone(), secret);
    }

    pub fn get(&self, name: &str) -> Option<&ResolvedSecret> {
        self.secrets.get(name)
    }

    pub fn clear(&mut self) {
        // SecretValue implements ZeroizeOnDrop, so values are automatically zeroed
        self.secrets.clear();
    }

    pub fn len(&self) -> usize {
        self.secrets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.secrets.is_empty()
    }
}

impl Drop for SecretCache {
    fn drop(&mut self) {
        // Explicit clear for paranoia (values already zero on drop)
        self.clear();
    }
}

/// Secret resolver that orchestrates multiple sources
pub struct SecretResolver {
    /// Secret sources
    sources: Vec<Box<dyn SecretSource>>,
    /// In-memory cache
    cache: Arc<RwLock<SecretCache>>,
    /// Resolution context
    context: ResolutionContext,
}

impl SecretResolver {
    /// Create a new resolver with the given context
    pub fn new(context: ResolutionContext) -> Self {
        let sources: Vec<Box<dyn SecretSource>> = vec![
            Box::new(EnvSource::new()),
            Box::new(FileSource::new()),
            Box::new(VaultSource::new()),
        ];

        Self {
            sources,
            cache: Arc::new(RwLock::new(SecretCache::new())),
            context,
        }
    }

    /// Create with custom sources (for testing)
    pub fn with_sources(
        sources: Vec<Box<dyn SecretSource>>,
        context: ResolutionContext,
    ) -> Self {
        Self {
            sources,
            cache: Arc::new(RwLock::new(SecretCache::new())),
            context,
        }
    }

    /// Resolve all secrets from definitions
    pub async fn resolve_all(
        &self,
        definitions: &[SecretConfig],
    ) -> Result<HashMap<String, ResolvedSecret>> {
        let mut resolved = HashMap::new();
        let mut errors = Vec::new();

        for definition in definitions {
            match self.resolve_one(definition).await {
                Ok(Some(secret)) => {
                    resolved.insert(secret.name.clone(), secret);
                }
                Ok(None) => {
                    if definition.required {
                        errors.push(format!(
                            "Required secret '{}' not found from source: {:?}",
                            definition.name, definition.source
                        ));
                    } else {
                        debug!("Optional secret '{}' not found", definition.name);
                    }
                }
                Err(e) => {
                    if definition.required {
                        errors.push(format!(
                            "Failed to resolve required secret '{}': {}",
                            definition.name, e
                        ));
                    } else {
                        warn!("Failed to resolve optional secret '{}': {}", definition.name, e);
                    }
                }
            }
        }

        if !errors.is_empty() {
            return Err(anyhow!(
                "Secret resolution failed:\n  - {}",
                errors.join("\n  - ")
            ));
        }

        Ok(resolved)
    }

    /// Resolve a single secret with caching
    async fn resolve_one(&self, definition: &SecretConfig) -> Result<Option<ResolvedSecret>> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(&definition.name) {
                debug!("Using cached secret: {}", definition.name);
                return Ok(Some(cached.clone()));
            }
        }

        // Try to resolve from appropriate source
        let source = self.get_source_for_type(&definition.source)?;
        let resolved = source.resolve(definition, &self.context).await?;

        // Cache if resolved
        if let Some(ref secret) = resolved {
            let mut cache = self.cache.write().await;
            cache.insert(secret.clone());
            debug!("Cached secret: {}", secret.name);
        }

        Ok(resolved)
    }

    /// Get the source for a given secret type
    fn get_source_for_type(&self, source_type: &ConfigSecretSource) -> Result<&dyn SecretSource> {
        let source_name = match source_type {
            ConfigSecretSource::Env => "env",
            ConfigSecretSource::File => "file",
            ConfigSecretSource::Vault => "vault",
            ConfigSecretSource::S3 => {
                return Err(anyhow!("S3 source not yet implemented (planned for Phase 6)"));
            }
        };

        self.sources
            .iter()
            .find(|s| s.name() == source_name)
            .map(|s| s.as_ref())
            .ok_or_else(|| anyhow!("Source '{}' not found", source_name))
    }

    /// Validate all sources are available
    pub fn validate_sources(&self) -> Result<()> {
        let mut errors = Vec::new();

        for source in &self.sources {
            if let Err(e) = source.validate() {
                errors.push(format!("Source '{}' validation failed: {}", source.name(), e));
            }
        }

        if !errors.is_empty() {
            return Err(anyhow!("Source validation failed:\n  - {}", errors.join("\n  - ")));
        }

        Ok(())
    }

    /// Clear the cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Get cache size
    pub async fn cache_size(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }

    /// Get the resolution context
    pub fn context(&self) -> &ResolutionContext {
        &self.context
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ResolvedFrom, SecretMetadata, SecretValue};
    use async_trait::async_trait;
    use std::path::PathBuf;

    // Mock source for testing
    struct MockSource {
        name: &'static str,
        should_resolve: bool,
        should_fail: bool,
    }

    #[async_trait]
    impl SecretSource for MockSource {
        async fn resolve(
            &self,
            definition: &SecretConfig,
            _ctx: &ResolutionContext,
        ) -> Result<Option<ResolvedSecret>> {
            if self.should_fail {
                return Err(anyhow!("Mock failure"));
            }

            if !self.should_resolve {
                return Ok(None);
            }

            Ok(Some(ResolvedSecret {
                name: definition.name.clone(),
                value: SecretValue::from_env("mock-value".to_string()),
                metadata: SecretMetadata {
                    source_type: definition.source,
                    resolved_from: ResolvedFrom::ShellEnv,
                    size_bytes: 10,
                },
            }))
        }

        fn validate(&self) -> Result<()> {
            Ok(())
        }

        fn name(&self) -> &'static str {
            self.name
        }
    }

    #[tokio::test]
    async fn test_resolve_one_with_cache() {
        let ctx = ResolutionContext::new(PathBuf::from("."));
        let mock_source: Box<dyn SecretSource> = Box::new(MockSource {
            name: "env",
            should_resolve: true,
            should_fail: false,
        });

        let resolver = SecretResolver::with_sources(vec![mock_source], ctx);

        let config = SecretConfig {
            name: "TEST_SECRET".to_string(),
            source: ConfigSecretSource::Env,
            from_file: None,
            required: true,
            path: None,
            mount_path: None,
            permissions: "0644".to_string(),
            vault_path: None,
            vault_key: None,
            vault_mount: "secret".to_string(),
            s3_path: None,
        };

        // First resolution
        let result1 = resolver.resolve_one(&config).await.unwrap();
        assert!(result1.is_some());

        // Should have cached
        assert_eq!(resolver.cache_size().await, 1);

        // Second resolution should use cache
        let result2 = resolver.resolve_one(&config).await.unwrap();
        assert!(result2.is_some());

        // Cache size should be the same
        assert_eq!(resolver.cache_size().await, 1);
    }

    #[tokio::test]
    async fn test_resolve_all_required_missing() {
        let ctx = ResolutionContext::new(PathBuf::from("."));
        let mock_source: Box<dyn SecretSource> = Box::new(MockSource {
            name: "env",
            should_resolve: false, // Won't resolve
            should_fail: false,
        });

        let resolver = SecretResolver::with_sources(vec![mock_source], ctx);

        let configs = vec![SecretConfig {
            name: "REQUIRED_SECRET".to_string(),
            source: ConfigSecretSource::Env,
            from_file: None,
            required: true, // Required
            path: None,
            mount_path: None,
            permissions: "0644".to_string(),
            vault_path: None,
            vault_key: None,
            vault_mount: "secret".to_string(),
            s3_path: None,
        }];

        // Should fail because required secret is missing
        let result = resolver.resolve_all(&configs).await;
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Required secret"));
        assert!(err_msg.contains("REQUIRED_SECRET"));
    }

    #[tokio::test]
    async fn test_resolve_all_optional_missing() {
        let ctx = ResolutionContext::new(PathBuf::from("."));
        let mock_source: Box<dyn SecretSource> = Box::new(MockSource {
            name: "env",
            should_resolve: false, // Won't resolve
            should_fail: false,
        });

        let resolver = SecretResolver::with_sources(vec![mock_source], ctx);

        let configs = vec![SecretConfig {
            name: "OPTIONAL_SECRET".to_string(),
            source: ConfigSecretSource::Env,
            from_file: None,
            required: false, // Optional
            path: None,
            mount_path: None,
            permissions: "0644".to_string(),
            vault_path: None,
            vault_key: None,
            vault_mount: "secret".to_string(),
            s3_path: None,
        }];

        // Should succeed with empty result
        let result = resolver.resolve_all(&configs).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_clear_cache() {
        let ctx = ResolutionContext::new(PathBuf::from("."));
        let mock_source: Box<dyn SecretSource> = Box::new(MockSource {
            name: "env",
            should_resolve: true,
            should_fail: false,
        });

        let resolver = SecretResolver::with_sources(vec![mock_source], ctx);

        let config = SecretConfig {
            name: "TEST_SECRET".to_string(),
            source: ConfigSecretSource::Env,
            from_file: None,
            required: true,
            path: None,
            mount_path: None,
            permissions: "0644".to_string(),
            vault_path: None,
            vault_key: None,
            vault_mount: "secret".to_string(),
            s3_path: None,
        };

        // Resolve to populate cache
        resolver.resolve_one(&config).await.unwrap();
        assert_eq!(resolver.cache_size().await, 1);

        // Clear cache
        resolver.clear_cache().await;
        assert_eq!(resolver.cache_size().await, 0);
    }
}
