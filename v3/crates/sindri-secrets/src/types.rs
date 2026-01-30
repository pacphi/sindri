//! Core types for secrets management

use serde::{Deserialize, Serialize};
use sindri_core::types::SecretSource as ConfigSecretSource;
use std::path::PathBuf;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Resolved secret with metadata
#[derive(Debug, Clone)]
pub struct ResolvedSecret {
    pub name: String,
    pub value: SecretValue,
    pub metadata: SecretMetadata,
}

/// Secret value with automatic zeroing
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub enum SecretValue {
    /// Environment variable value (string)
    Env(String),
    /// File content with mount information
    File {
        #[zeroize(skip)]
        content: Vec<u8>,
        #[zeroize(skip)]
        mount_path: PathBuf,
        #[zeroize(skip)]
        permissions: u32, // Octal as decimal (e.g., 0o600 = 384)
    },
}

impl SecretValue {
    /// Get the string value (for env secrets)
    pub fn as_string(&self) -> Option<&str> {
        match self {
            SecretValue::Env(s) => Some(s),
            SecretValue::File { .. } => None,
        }
    }

    /// Get file content (for file secrets)
    pub fn as_file(&self) -> Option<(&[u8], &PathBuf, u32)> {
        match self {
            SecretValue::File {
                content,
                mount_path,
                permissions,
            } => Some((content, mount_path, *permissions)),
            SecretValue::Env(_) => None,
        }
    }

    /// Get the raw bytes
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            SecretValue::Env(s) => s.as_bytes(),
            SecretValue::File { content, .. } => content,
        }
    }

    /// Get the length of the secret value in bytes
    pub fn len(&self) -> usize {
        self.as_bytes().len()
    }

    /// Check if the secret value is empty
    pub fn is_empty(&self) -> bool {
        self.as_bytes().is_empty()
    }

    /// Create from environment variable
    pub fn from_env(value: String) -> Self {
        SecretValue::Env(value)
    }

    /// Create from file with metadata
    pub fn from_file(content: Vec<u8>, mount_path: PathBuf, permissions: u32) -> Self {
        SecretValue::File {
            content,
            mount_path,
            permissions,
        }
    }
}

impl std::fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecretValue::Env(_) => write!(f, "Env([REDACTED {} bytes])", self.as_bytes().len()),
            SecretValue::File {
                mount_path,
                permissions,
                content,
            } => {
                write!(
                    f,
                    "File(path={}, perms={:o}, size={} bytes)",
                    mount_path.display(),
                    permissions,
                    content.len()
                )
            }
        }
    }
}

/// Metadata about secret resolution
#[derive(Debug, Clone)]
pub struct SecretMetadata {
    pub source_type: ConfigSecretSource,
    pub resolved_from: ResolvedFrom,
    pub size_bytes: usize,
}

/// Information about where a secret was resolved from
#[derive(Debug, Clone)]
pub enum ResolvedFrom {
    ShellEnv,
    EnvLocalFile,
    EnvFile,
    FromFile(PathBuf),
    Vault { path: String, mount: String },
    LocalFile(PathBuf),
}

impl std::fmt::Display for ResolvedFrom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolvedFrom::ShellEnv => write!(f, "shell environment"),
            ResolvedFrom::EnvLocalFile => write!(f, ".env.local file"),
            ResolvedFrom::EnvFile => write!(f, ".env file"),
            ResolvedFrom::FromFile(path) => write!(f, "file: {}", path.display()),
            ResolvedFrom::Vault { path, mount } => write!(f, "vault: {}/{}", mount, path),
            ResolvedFrom::LocalFile(path) => write!(f, "local file: {}", path.display()),
        }
    }
}

/// Resolution context for secret resolution
#[derive(Debug, Clone)]
pub struct ResolutionContext {
    /// Directory containing sindri.yaml (for relative path resolution)
    pub config_dir: PathBuf,

    /// Whether to allow optional secrets to fail silently
    pub allow_optional_failures: bool,

    /// Validation mode (don't actually resolve, just check availability)
    pub validation_mode: bool,

    /// Custom .env file path (overrides default .env/.env.local)
    pub custom_env_file: Option<PathBuf>,
}

impl ResolutionContext {
    pub fn new(config_dir: PathBuf) -> Self {
        Self {
            config_dir,
            allow_optional_failures: true,
            validation_mode: false,
            custom_env_file: None,
        }
    }

    pub fn with_validation_mode(mut self, mode: bool) -> Self {
        self.validation_mode = mode;
        self
    }

    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.allow_optional_failures = !strict;
        self
    }

    pub fn with_custom_env_file(mut self, path: Option<PathBuf>) -> Self {
        self.custom_env_file = path;
        self
    }
}

/// Token metadata for Vault
#[derive(Debug, Clone)]
pub struct TokenMetadata {
    pub token: String,
    pub ttl: u64,
    pub renewable: bool,
    pub last_renewed: std::time::SystemTime,
}

impl TokenMetadata {
    pub fn new(token: String) -> Self {
        Self {
            token,
            ttl: 0,
            renewable: false,
            last_renewed: std::time::SystemTime::now(),
        }
    }

    /// Check if token needs renewal (< 1 hour remaining)
    pub fn needs_renewal(&self) -> bool {
        if !self.renewable {
            return false;
        }

        let elapsed = self.last_renewed.elapsed().unwrap_or_default().as_secs();

        // Renew if less than 1 hour remaining
        self.ttl.saturating_sub(elapsed) < 3600
    }

    /// Update metadata after renewal
    pub fn update_ttl(&mut self, ttl: u64) {
        self.ttl = ttl;
        self.last_renewed = std::time::SystemTime::now();
    }
}

/// Vault secret with lease information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultSecret {
    pub value: String,
    pub version: Option<u64>,
    pub lease_id: Option<String>,
    pub lease_duration: Option<u64>,
    pub renewable: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_value_env() {
        let secret = SecretValue::from_env("test-value".to_string());
        assert_eq!(secret.as_string(), Some("test-value"));
        assert_eq!(secret.as_file(), None);
    }

    #[test]
    fn test_secret_value_file() {
        let content = b"file content".to_vec();
        let mount_path = PathBuf::from("/secrets/test.txt");
        let secret = SecretValue::from_file(content.clone(), mount_path.clone(), 0o600);

        assert_eq!(secret.as_string(), None);
        if let Some((c, p, perms)) = secret.as_file() {
            assert_eq!(c, &content[..]);
            assert_eq!(p, &mount_path);
            assert_eq!(perms, 0o600);
        } else {
            panic!("Expected file secret");
        }
    }

    #[test]
    fn test_secret_value_debug() {
        let env_secret = SecretValue::from_env("sensitive".to_string());
        let debug_str = format!("{:?}", env_secret);
        assert!(debug_str.contains("REDACTED"));
        assert!(!debug_str.contains("sensitive"));
    }

    #[test]
    fn test_token_metadata_renewal() {
        let mut meta = TokenMetadata::new("token".to_string());
        meta.renewable = true;
        meta.ttl = 7200; // 2 hours

        // Fresh token doesn't need renewal
        assert!(!meta.needs_renewal());

        // Token with low TTL needs renewal
        meta.ttl = 3000; // Less than 1 hour
        assert!(meta.needs_renewal());
    }
}
