//! Local secret cache for S3 secrets
//!
//! Provides file-based caching to reduce S3 API calls and enable offline access.
//! Cached secrets are stored encrypted with file-based TTL management.

use crate::s3::types::{CacheEntry, S3CacheConfig};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Statistics about cache usage
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Number of expired entries encountered
    pub expired: u64,
    /// Total number of cached entries
    pub entries: usize,
    /// Total size of cached data in bytes
    pub size_bytes: u64,
}

impl CacheStats {
    /// Get hit rate as a percentage
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f64 / total as f64) * 100.0
        }
    }
}

/// Local file-based cache for S3 secrets
pub struct SecretCache {
    /// Cache configuration
    config: S3CacheConfig,
    /// Expanded cache directory path
    cache_dir: PathBuf,
    /// In-memory index of cached secrets
    index: RwLock<HashMap<String, CacheEntry>>,
    /// Cache statistics
    stats: RwLock<CacheStats>,
}

impl SecretCache {
    /// Create a new secret cache from configuration
    pub async fn new(config: S3CacheConfig) -> Result<Self> {
        let expanded_path = shellexpand::tilde(&config.path.to_string_lossy()).to_string();
        let cache_dir = PathBuf::from(&expanded_path);

        // Create cache directory if it doesn't exist
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir).await.with_context(|| {
                format!("Failed to create cache directory: {}", cache_dir.display())
            })?;

            // Set restrictive permissions on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&cache_dir, std::fs::Permissions::from_mode(0o700))
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to set permissions on cache directory: {}",
                            cache_dir.display()
                        )
                    })?;
            }

            info!("Created cache directory: {}", cache_dir.display());
        }

        let cache = Self {
            config,
            cache_dir,
            index: RwLock::new(HashMap::new()),
            stats: RwLock::new(CacheStats::default()),
        };

        // Load existing cache index
        cache.load_index().await?;

        Ok(cache)
    }

    /// Check if caching is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get cached secret value if available and not expired
    pub async fn get(&self, s3_path: &str) -> Result<Option<String>> {
        if !self.config.enabled {
            return Ok(None);
        }

        let index = self.index.read().await;

        if let Some(entry) = index.get(s3_path) {
            if entry.is_expired() {
                debug!("Cache entry expired for: {}", s3_path);
                drop(index);

                let mut stats = self.stats.write().await;
                stats.expired += 1;
                stats.misses += 1;

                // Remove expired entry
                self.invalidate(s3_path).await?;
                return Ok(None);
            }

            debug!(
                "Cache hit for: {} (TTL remaining: {}s)",
                s3_path,
                entry.remaining_ttl()
            );

            let mut stats = self.stats.write().await;
            stats.hits += 1;

            return Ok(Some(entry.value.clone()));
        }

        let mut stats = self.stats.write().await;
        stats.misses += 1;
        debug!("Cache miss for: {}", s3_path);

        Ok(None)
    }

    /// Store a secret value in the cache
    pub async fn set(&self, s3_path: &str, value: &str) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let entry = CacheEntry::new(value.to_string(), self.config.ttl_seconds);

        // Write to disk
        let cache_file = self.get_cache_file_path(s3_path);

        // Create parent directories if needed
        if let Some(parent) = cache_file.parent() {
            fs::create_dir_all(parent).await?;
        }

        let json =
            serde_json::to_string_pretty(&entry).context("Failed to serialize cache entry")?;

        fs::write(&cache_file, &json)
            .await
            .with_context(|| format!("Failed to write cache file: {}", cache_file.display()))?;

        // Set restrictive permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&cache_file, std::fs::Permissions::from_mode(0o600))
                .await
                .ok(); // Ignore errors, not critical
        }

        // Update index
        let mut index = self.index.write().await;
        index.insert(s3_path.to_string(), entry);

        debug!(
            "Cached secret: {} (TTL: {}s)",
            s3_path, self.config.ttl_seconds
        );

        Ok(())
    }

    /// Update cache entry with version information
    pub async fn set_with_version(
        &self,
        s3_path: &str,
        value: &str,
        version_id: Option<String>,
        etag: Option<String>,
    ) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let mut entry = CacheEntry::new(value.to_string(), self.config.ttl_seconds);
        entry.version_id = version_id;
        entry.etag = etag;

        // Write to disk
        let cache_file = self.get_cache_file_path(s3_path);

        if let Some(parent) = cache_file.parent() {
            fs::create_dir_all(parent).await?;
        }

        let json =
            serde_json::to_string_pretty(&entry).context("Failed to serialize cache entry")?;

        fs::write(&cache_file, &json)
            .await
            .with_context(|| format!("Failed to write cache file: {}", cache_file.display()))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&cache_file, std::fs::Permissions::from_mode(0o600))
                .await
                .ok();
        }

        let mut index = self.index.write().await;
        index.insert(s3_path.to_string(), entry);

        debug!("Cached secret with version: {}", s3_path);

        Ok(())
    }

    /// Invalidate (remove) a cached secret
    pub async fn invalidate(&self, s3_path: &str) -> Result<()> {
        // Remove from index
        let mut index = self.index.write().await;
        index.remove(s3_path);

        // Remove from disk
        let cache_file = self.get_cache_file_path(s3_path);
        if cache_file.exists() {
            fs::remove_file(&cache_file).await.with_context(|| {
                format!("Failed to remove cache file: {}", cache_file.display())
            })?;
        }

        debug!("Invalidated cache entry: {}", s3_path);

        Ok(())
    }

    /// Clear all cached secrets
    pub async fn clear(&self) -> Result<()> {
        // Clear index
        let mut index = self.index.write().await;
        index.clear();

        // Clear disk cache
        if self.cache_dir.exists() {
            // Remove all files in cache directory
            let mut entries = fs::read_dir(&self.cache_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.is_file() {
                    fs::remove_file(&path).await?;
                } else if path.is_dir() {
                    fs::remove_dir_all(&path).await?;
                }
            }
        }

        // Reset stats
        let mut stats = self.stats.write().await;
        *stats = CacheStats::default();

        info!("Cleared secret cache");

        Ok(())
    }

    /// Clean up expired cache entries
    pub async fn cleanup_expired(&self) -> Result<usize> {
        let mut removed = 0;
        let expired_paths: Vec<String>;

        {
            let index = self.index.read().await;
            expired_paths = index
                .iter()
                .filter(|(_, entry)| entry.is_expired())
                .map(|(path, _)| path.clone())
                .collect();
        }

        for path in expired_paths {
            self.invalidate(&path).await?;
            removed += 1;
        }

        if removed > 0 {
            info!("Cleaned up {} expired cache entries", removed);
        }

        Ok(removed)
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let stats = self.stats.read().await;
        let index = self.index.read().await;

        let mut result = stats.clone();
        result.entries = index.len();

        // Calculate total size
        result.size_bytes = index.values().map(|e| e.value.len() as u64).sum();

        result
    }

    /// List all cached secret paths
    pub async fn list(&self) -> Vec<String> {
        let index = self.index.read().await;
        index.keys().cloned().collect()
    }

    /// Check if a secret is cached (and not expired)
    pub async fn contains(&self, s3_path: &str) -> bool {
        let index = self.index.read().await;
        if let Some(entry) = index.get(s3_path) {
            !entry.is_expired()
        } else {
            false
        }
    }

    /// Get cache entry metadata (for sync operations)
    pub async fn get_entry(&self, s3_path: &str) -> Option<CacheEntry> {
        let index = self.index.read().await;
        index.get(s3_path).cloned()
    }

    /// Load cache index from disk
    async fn load_index(&self) -> Result<()> {
        if !self.cache_dir.exists() {
            return Ok(());
        }

        let mut loaded = 0;
        let mut expired = 0;

        let entries = Self::scan_cache_dir(&self.cache_dir).await?;

        let mut index = self.index.write().await;

        for (s3_path, entry) in entries {
            if entry.is_expired() {
                expired += 1;
                // Don't load expired entries, but we'll clean them up later
                continue;
            }
            index.insert(s3_path, entry);
            loaded += 1;
        }

        if loaded > 0 || expired > 0 {
            debug!("Loaded {} cache entries ({} expired)", loaded, expired);
        }

        Ok(())
    }

    /// Scan cache directory for entries
    async fn scan_cache_dir(dir: &Path) -> Result<Vec<(String, CacheEntry)>> {
        let mut entries = Vec::new();

        let mut read_dir = fs::read_dir(dir).await?;

        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();

            if path.is_dir() {
                // Recurse into subdirectories
                let sub_entries = Box::pin(Self::scan_cache_dir(&path)).await?;
                entries.extend(sub_entries);
            } else if path.extension().map(|e| e == "json").unwrap_or(false) {
                // Try to load cache entry
                match fs::read_to_string(&path).await {
                    Ok(content) => {
                        match serde_json::from_str::<CacheEntry>(&content) {
                            Ok(cache_entry) => {
                                // Extract s3_path from file path
                                let relative =
                                    path.strip_prefix(dir).unwrap_or(&path).with_extension("");
                                let s3_path = relative.to_string_lossy().replace('\\', "/");
                                entries.push((s3_path, cache_entry));
                            }
                            Err(e) => {
                                warn!("Failed to parse cache entry {}: {}", path.display(), e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to read cache file {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(entries)
    }

    /// Get the file path for a cached secret
    fn get_cache_file_path(&self, s3_path: &str) -> PathBuf {
        let sanitized = s3_path.replace(['/', '\\'], std::path::MAIN_SEPARATOR_STR);
        self.cache_dir.join(format!("{}.json", sanitized))
    }
}

impl std::fmt::Debug for SecretCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecretCache")
            .field("enabled", &self.config.enabled)
            .field("ttl_seconds", &self.config.ttl_seconds)
            .field("cache_dir", &self.cache_dir)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    async fn create_test_cache() -> (SecretCache, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let config = S3CacheConfig {
            enabled: true,
            ttl_seconds: 3600,
            path: dir.path().to_path_buf(),
        };
        let cache = SecretCache::new(config).await.unwrap();
        (cache, dir)
    }

    #[tokio::test]
    async fn test_cache_set_get() {
        let (cache, _dir) = create_test_cache().await;

        cache.set("test/secret", "secret-value").await.unwrap();

        let value = cache.get("test/secret").await.unwrap();
        assert_eq!(value, Some("secret-value".to_string()));
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let (cache, _dir) = create_test_cache().await;

        let value = cache.get("nonexistent").await.unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_cache_invalidate() {
        let (cache, _dir) = create_test_cache().await;

        cache.set("test/secret", "value").await.unwrap();
        assert!(cache.contains("test/secret").await);

        cache.invalidate("test/secret").await.unwrap();
        assert!(!cache.contains("test/secret").await);
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let (cache, _dir) = create_test_cache().await;

        cache.set("secret1", "value1").await.unwrap();
        cache.set("secret2", "value2").await.unwrap();

        let list = cache.list().await;
        assert_eq!(list.len(), 2);

        cache.clear().await.unwrap();

        let list = cache.list().await;
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let (cache, _dir) = create_test_cache().await;

        cache.set("secret", "value").await.unwrap();

        // Miss
        cache.get("nonexistent").await.unwrap();

        // Hit
        cache.get("secret").await.unwrap();

        let stats = cache.stats().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.entries, 1);
        assert_eq!(stats.hit_rate(), 50.0);
    }

    #[tokio::test]
    async fn test_cache_disabled() {
        let dir = tempdir().unwrap();
        let config = S3CacheConfig {
            enabled: false,
            ttl_seconds: 3600,
            path: dir.path().to_path_buf(),
        };
        let cache = SecretCache::new(config).await.unwrap();

        // Set should be no-op when disabled
        cache.set("test", "value").await.unwrap();

        // Get should return None when disabled
        let value = cache.get("test").await.unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_cache_persistence() {
        let dir = tempdir().unwrap();
        let config = S3CacheConfig {
            enabled: true,
            ttl_seconds: 3600,
            path: dir.path().to_path_buf(),
        };

        // Create cache and set value
        {
            let cache = SecretCache::new(config.clone()).await.unwrap();
            cache.set("persistent", "value").await.unwrap();
        }

        // Create new cache instance and verify persistence
        {
            let cache = SecretCache::new(config).await.unwrap();
            let value = cache.get("persistent").await.unwrap();
            assert_eq!(value, Some("value".to_string()));
        }
    }

    #[tokio::test]
    async fn test_cache_with_version() {
        let (cache, _dir) = create_test_cache().await;

        cache
            .set_with_version(
                "test",
                "value",
                Some("v1".to_string()),
                Some("etag123".to_string()),
            )
            .await
            .unwrap();

        let entry = cache.get_entry("test").await.unwrap();
        assert_eq!(entry.value, "value");
        assert_eq!(entry.version_id, Some("v1".to_string()));
        assert_eq!(entry.etag, Some("etag123".to_string()));
    }
}
