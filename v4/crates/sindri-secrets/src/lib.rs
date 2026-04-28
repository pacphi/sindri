//! `sindri-secrets` — pluggable secret store (ADR-025).
//!
//! # Overview
//!
//! Defines a [`SecretStore`] async trait with three built-in backends:
//!
//! - [`FileBackend`] — encrypted-at-rest file under `~/.sindri/secrets.enc`
//!   using ChaCha20-Poly1305 with an HKDF-derived key.
//! - [`VaultBackend`] — thin HTTP client over HashiCorp Vault KV v2.
//! - [`EnvBackend`] — read-only, sources from `SINDRI_SECRET_<NAME>` env vars.
//!
//! # Migration
//!
//! Auth tokens that were stored with the legacy `plain:` prefix are silently
//! migrated to the secret store on first read.  A one-time `tracing::warn!`
//! is emitted per migrated key.  The `secret:` prefix in manifests is the
//! forward-compatible pointer.

pub mod backends;
pub mod error;
pub mod migrate;
pub mod value;

pub use backends::{EnvBackend, FileBackend, VaultBackend};
pub use error::SecretsError;
pub use value::SecretValue;

use async_trait::async_trait;

/// Core trait every secret backend must implement.
///
/// All names are opaque strings (e.g. `"targets.fly1.auth.token"`).
#[async_trait]
pub trait SecretStore: Send + Sync {
    /// Read the secret stored under `name`.
    async fn read(&self, name: &str) -> Result<SecretValue, SecretsError>;

    /// Write (upsert) a secret under `name`.
    async fn write(&self, name: &str, value: SecretValue) -> Result<(), SecretsError>;

    /// Delete the secret stored under `name`.
    async fn delete(&self, name: &str) -> Result<(), SecretsError>;

    /// List all stored secret names.
    async fn list(&self) -> Result<Vec<String>, SecretsError>;
}
