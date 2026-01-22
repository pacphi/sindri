//! S3-compatible encrypted secret storage
//!
//! This module provides secure secret storage using S3-compatible backends
//! with client-side envelope encryption (ChaCha20-Poly1305 + age).
//!
//! ## Architecture
//!
//! The S3 secret storage uses envelope encryption:
//! 1. Generate a random Data Encryption Key (DEK) per secret
//! 2. Encrypt the secret value with the DEK using ChaCha20-Poly1305
//! 3. Encrypt the DEK with the master key using age encryption
//! 4. Store the encrypted DEK + encrypted value in S3
//!
//! ## Security Layers
//!
//! - Layer 1: S3 Server-Side Encryption (SSE-S3)
//! - Layer 2: Client-Side Encryption (ChaCha20-Poly1305)
//! - Layer 3: Master Key Encryption (age)
//! - Layer 4: IAM/Bucket Policies
//! - Layer 5: TLS in Transit
//!
//! ## Usage
//!
//! ```ignore
//! use sindri_secrets::s3::{S3SecretResolver, S3SecretBackend};
//!
//! // Create configuration
//! let config = S3SecretBackend {
//!     bucket: "my-secrets".to_string(),
//!     region: "us-east-1".to_string(),
//!     prefix: "secrets/prod/".to_string(),
//!     ..Default::default()
//! };
//!
//! // Create resolver
//! let resolver = S3SecretResolver::new(config).await?;
//!
//! // Push a secret
//! resolver.push("API_KEY", "secret-value", "api/key", &[]).await?;
//!
//! // Resolve a secret
//! let value = resolver.resolve("api/key").await?;
//! ```

pub mod backend;
pub mod cache;
pub mod encryption;
pub mod resolver;
pub mod types;

// Re-export main types
pub use backend::S3Backend;
pub use cache::{CacheStats, SecretCache};
pub use encryption::{generate_identity, generate_key_file, SecretEncryptor};
pub use resolver::{LocalSecretInfo, S3SecretResolver, SyncDirection};
pub use types::{
    AdditionalMetadata, CacheEntry, EncryptionAlgorithm, EncryptionMetadata, KeySource,
    S3CacheConfig, S3EncryptionConfig, S3SecretBackend, S3SecretMetadata, S3SecretVersion,
    SyncConflict, SyncResult,
};
