//! S3-compatible encrypted secret storage
//!
//! This module provides secure secret storage using S3-compatible backends
//! with client-side envelope encryption (ChaCha20-Poly1305 + age).

pub mod backend;
pub mod cache;
pub mod encryption;
pub mod resolver;
pub mod types;

// Re-export main types
pub use backend::S3Backend;
pub use cache::{CacheStats, SecretCache};
pub use encryption::SecretEncryptor;
pub use resolver::S3SecretResolver;
pub use types::{
    AdditionalMetadata, EncryptionAlgorithm, EncryptionMetadata, KeySource, S3CacheConfig,
    S3EncryptionConfig, S3SecretBackend, S3SecretMetadata, S3SecretVersion,
};
