//! Error types for the secrets subsystem.

use thiserror::Error;

/// All errors emitted by [`crate::SecretStore`] implementations.
#[derive(Debug, Error)]
pub enum SecretsError {
    /// The requested secret does not exist in the store.
    #[error("secret not found: {name}")]
    NotFound { name: String },

    /// The backing file could not be read or written.
    #[error("secrets file I/O: {0}")]
    Io(#[from] std::io::Error),

    /// Encryption or decryption failed (wrong passphrase, corrupt data, …).
    #[error("encryption error: {0}")]
    Crypto(String),

    /// The serialised store payload could not be parsed.
    #[error("serialisation error: {0}")]
    Serde(String),

    /// An HTTP call to Vault failed.
    #[error("vault HTTP error: {0}")]
    VaultHttp(String),

    /// The Vault server returned an auth / permission error.
    #[error("vault auth failed: {0}")]
    VaultAuth(String),

    /// The environment variable for an [`crate::EnvBackend`] secret is unset.
    #[error("env var SINDRI_SECRET_{name} is not set")]
    EnvVarMissing { name: String },

    /// The requested operation is not supported by this backend.
    #[error("operation not supported by this backend: {0}")]
    Unsupported(String),

    /// Generic catch-all.
    #[error("{0}")]
    Other(String),
}
