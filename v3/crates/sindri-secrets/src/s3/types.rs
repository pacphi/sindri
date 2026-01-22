//! S3 secret storage types
//!
//! Types for S3-based encrypted secret storage with envelope encryption.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// S3 secret backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3SecretBackend {
    /// S3 bucket name
    pub bucket: String,
    /// AWS region
    pub region: String,
    /// Custom S3-compatible endpoint (optional)
    pub endpoint: Option<String>,
    /// Key prefix for secrets (e.g., "secrets/prod/")
    pub prefix: String,
    /// Encryption configuration
    pub encryption: S3EncryptionConfig,
    /// Cache configuration
    pub cache: Option<S3CacheConfig>,
}

impl Default for S3SecretBackend {
    fn default() -> Self {
        Self {
            bucket: String::new(),
            region: "us-east-1".to_string(),
            endpoint: None,
            prefix: "secrets/".to_string(),
            encryption: S3EncryptionConfig::default(),
            cache: Some(S3CacheConfig::default()),
        }
    }
}

/// Encryption configuration for S3 secrets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3EncryptionConfig {
    /// Encryption algorithm (always ChaCha20-Poly1305)
    #[serde(default)]
    pub algorithm: EncryptionAlgorithm,
    /// Where to get the master key
    pub key_source: KeySource,
    /// Environment variable name for master key (when key_source = env)
    pub key_env: Option<String>,
    /// File path for master key (when key_source = file)
    pub key_file: Option<PathBuf>,
    /// KMS key ID (when key_source = kms, future)
    pub kms_key_id: Option<String>,
}

impl Default for S3EncryptionConfig {
    fn default() -> Self {
        Self {
            algorithm: EncryptionAlgorithm::ChaCha20Poly1305,
            key_source: KeySource::File,
            key_env: None,
            key_file: Some(PathBuf::from("~/.sindri/master.key")),
            kms_key_id: None,
        }
    }
}

/// Encryption algorithm enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EncryptionAlgorithm {
    /// ChaCha20-Poly1305 AEAD (default)
    #[default]
    #[serde(rename = "chacha20poly1305")]
    ChaCha20Poly1305,
}

impl std::fmt::Display for EncryptionAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncryptionAlgorithm::ChaCha20Poly1305 => write!(f, "chacha20poly1305"),
        }
    }
}

/// Source for the master encryption key
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum KeySource {
    /// Master key from environment variable
    Env,
    /// Master key from file (default)
    #[default]
    File,
    /// Master key from AWS KMS (future)
    Kms,
}

impl std::fmt::Display for KeySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeySource::Env => write!(f, "env"),
            KeySource::File => write!(f, "file"),
            KeySource::Kms => write!(f, "kms"),
        }
    }
}

/// Cache configuration for S3 secrets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3CacheConfig {
    /// Whether caching is enabled
    #[serde(default = "default_cache_enabled")]
    pub enabled: bool,
    /// Cache TTL in seconds
    #[serde(default = "default_cache_ttl")]
    pub ttl_seconds: u64,
    /// Local cache directory path
    #[serde(default = "default_cache_path")]
    pub path: PathBuf,
}

fn default_cache_enabled() -> bool {
    true
}

fn default_cache_ttl() -> u64 {
    3600 // 1 hour
}

fn default_cache_path() -> PathBuf {
    PathBuf::from("~/.sindri/cache/secrets/")
}

impl Default for S3CacheConfig {
    fn default() -> Self {
        Self {
            enabled: default_cache_enabled(),
            ttl_seconds: default_cache_ttl(),
            path: default_cache_path(),
        }
    }
}

/// Metadata stored with encrypted secret in S3
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3SecretMetadata {
    /// Metadata version
    #[serde(default = "default_version")]
    pub version: String,
    /// Secret name
    pub secret_name: String,
    /// Creation timestamp (RFC 3339)
    pub created_at: String,
    /// Last update timestamp (RFC 3339)
    pub updated_at: String,
    /// Encryption metadata
    pub encryption: EncryptionMetadata,
    /// Encrypted DEK (age-encrypted)
    pub encrypted_dek: String,
    /// Encrypted secret value (base64)
    pub encrypted_value: String,
    /// Nonce used for encryption (base64)
    pub nonce: String,
    /// Additional metadata
    #[serde(default)]
    pub metadata: AdditionalMetadata,
}

fn default_version() -> String {
    "1.0".to_string()
}

impl S3SecretMetadata {
    /// Create new metadata for a secret
    pub fn new(
        secret_name: String,
        encrypted_dek: String,
        encrypted_value: String,
        nonce: String,
        recipients: Vec<String>,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            version: "1.0".to_string(),
            secret_name,
            created_at: now.clone(),
            updated_at: now,
            encryption: EncryptionMetadata {
                algorithm: EncryptionAlgorithm::ChaCha20Poly1305,
                key_derivation: "age-x25519".to_string(),
                recipients,
            },
            encrypted_dek,
            encrypted_value,
            nonce,
            metadata: AdditionalMetadata::default(),
        }
    }

    /// Update the metadata timestamp
    pub fn update_timestamp(&mut self) {
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }
}

/// Encryption details in metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionMetadata {
    /// Encryption algorithm used
    pub algorithm: EncryptionAlgorithm,
    /// Key derivation method
    pub key_derivation: String,
    /// List of age recipient public keys
    pub recipients: Vec<String>,
}

/// Additional secret metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AdditionalMetadata {
    /// Number of times this secret has been rotated
    #[serde(default)]
    pub rotation_count: u32,
    /// Who last rotated the secret
    pub last_rotated_by: Option<String>,
    /// Description of the secret
    pub description: Option<String>,
    /// Custom tags
    #[serde(default)]
    pub tags: HashMap<String, String>,
}

/// S3 secret version info (from S3 versioning)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3SecretVersion {
    /// Version ID from S3
    pub version_id: String,
    /// When this version was created
    pub last_modified: String,
    /// ETag for integrity
    pub etag: String,
    /// Size in bytes
    pub size: u64,
    /// Whether this is the current version
    pub is_latest: bool,
}

/// Result of a sync operation
#[derive(Debug, Clone, Default)]
pub struct SyncResult {
    /// Secrets that need to be pushed to S3
    pub to_push: Vec<String>,
    /// Secrets that need to be pulled from S3
    pub to_pull: Vec<String>,
    /// Secrets that are in conflict
    pub conflicts: Vec<SyncConflict>,
    /// Secrets that are in sync
    pub in_sync: Vec<String>,
}

/// A sync conflict
#[derive(Debug, Clone)]
pub struct SyncConflict {
    /// Secret name
    pub name: String,
    /// Local last modified time
    pub local_modified: Option<String>,
    /// Remote last modified time
    pub remote_modified: Option<String>,
    /// Conflict reason
    pub reason: String,
}

/// Cache entry for a secret
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// The decrypted secret value
    pub value: String,
    /// When the cache entry was created
    pub cached_at: String,
    /// TTL in seconds
    pub ttl_seconds: u64,
    /// S3 version ID
    pub version_id: Option<String>,
    /// ETag for integrity checking
    pub etag: Option<String>,
}

impl CacheEntry {
    /// Create a new cache entry
    pub fn new(value: String, ttl_seconds: u64) -> Self {
        Self {
            value,
            cached_at: chrono::Utc::now().to_rfc3339(),
            ttl_seconds,
            version_id: None,
            etag: None,
        }
    }

    /// Check if the cache entry has expired
    pub fn is_expired(&self) -> bool {
        if let Ok(cached_time) = chrono::DateTime::parse_from_rfc3339(&self.cached_at) {
            let now = chrono::Utc::now();
            let elapsed = now.signed_duration_since(cached_time);
            elapsed.num_seconds() as u64 > self.ttl_seconds
        } else {
            // If we can't parse the time, consider it expired
            true
        }
    }

    /// Get remaining TTL in seconds
    pub fn remaining_ttl(&self) -> u64 {
        if let Ok(cached_time) = chrono::DateTime::parse_from_rfc3339(&self.cached_at) {
            let now = chrono::Utc::now();
            let elapsed = now.signed_duration_since(cached_time).num_seconds() as u64;
            self.ttl_seconds.saturating_sub(elapsed)
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_algorithm_serialize() {
        let algo = EncryptionAlgorithm::ChaCha20Poly1305;
        let json = serde_json::to_string(&algo).unwrap();
        assert_eq!(json, "\"chacha20poly1305\"");
    }

    #[test]
    fn test_key_source_serialize() {
        let source = KeySource::File;
        let json = serde_json::to_string(&source).unwrap();
        assert_eq!(json, "\"file\"");
    }

    #[test]
    fn test_s3_secret_metadata_new() {
        let metadata = S3SecretMetadata::new(
            "TEST_SECRET".to_string(),
            "encrypted-dek".to_string(),
            "encrypted-value".to_string(),
            "nonce".to_string(),
            vec!["age1recipient".to_string()],
        );

        assert_eq!(metadata.version, "1.0");
        assert_eq!(metadata.secret_name, "TEST_SECRET");
        assert_eq!(
            metadata.encryption.algorithm,
            EncryptionAlgorithm::ChaCha20Poly1305
        );
        assert_eq!(metadata.encryption.recipients.len(), 1);
    }

    #[test]
    fn test_cache_entry_expiry() {
        let entry = CacheEntry::new("value".to_string(), 1); // 1 second TTL
        assert!(!entry.is_expired()); // Should not be expired immediately

        // Sleep would make this flaky, so just test remaining_ttl
        assert!(entry.remaining_ttl() <= 1);
    }

    #[test]
    fn test_s3_backend_default() {
        let backend = S3SecretBackend::default();
        assert_eq!(backend.region, "us-east-1");
        assert_eq!(backend.prefix, "secrets/");
        assert!(backend.cache.is_some());
    }
}
