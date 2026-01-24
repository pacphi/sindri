//! Secrets management for Sindri
//!
//! This crate provides a comprehensive secrets management system with:
//! - **Multi-source resolution**: Environment variables, files, HashiCorp Vault, S3
//! - **Security**: Memory zeroing with zeroize, audit logging, path validation
//! - **Encryption**: ChaCha20-Poly1305 + age envelope encryption for S3 secrets
//! - **Performance**: Async resolution, caching, retry logic

// Core modules
pub mod resolver;
pub mod s3;
pub mod security;
pub mod sources;
pub mod types;

// Re-export commonly used items
pub use resolver::{SecretCache, SecretResolver};
pub use security::{AuditLog, SecureString};
pub use sources::{EnvSource, FileSource, SecretSource, VaultSource};
pub use types::{
    ResolutionContext, ResolvedFrom, ResolvedSecret, SecretMetadata, SecretValue, TokenMetadata,
    VaultSecret,
};

use anyhow::Result;
use sindri_core::types::SecretConfig;
use std::collections::HashMap;

/// Convenience function to resolve all secrets from config
pub async fn resolve_secrets(
    secrets: &[SecretConfig],
    config_dir: std::path::PathBuf,
) -> Result<HashMap<String, ResolvedSecret>> {
    let context = ResolutionContext::new(config_dir);
    let resolver = SecretResolver::new(context);
    resolver.resolve_all(secrets).await
}

/// Validate that all required secrets can be resolved
pub async fn validate_secrets(
    secrets: &[SecretConfig],
    config_dir: std::path::PathBuf,
) -> Result<()> {
    let context = ResolutionContext::new(config_dir).with_validation_mode(true);
    let resolver = SecretResolver::new(context);
    resolver.validate_sources()?;
    let _ = resolver.resolve_all(secrets).await?;
    Ok(())
}
