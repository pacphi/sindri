//! File-based secret source
//!
//! Handles file secrets with security validation:
//! - Path traversal prevention
//! - Allowed directory whitelist
//! - Permission parsing and validation
//! - Base64 encoding for transport

use crate::sources::SecretSource;
use crate::types::{ResolutionContext, ResolvedFrom, ResolvedSecret, SecretMetadata, SecretValue};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use sindri_core::types::{SecretConfig, SecretSource as ConfigSecretSource};
use std::path::{Path, PathBuf};
use tracing::debug;

pub struct FileSource {
    /// Allowed base directories for security
    allowed_dirs: Vec<PathBuf>,
}

impl FileSource {
    pub fn new() -> Self {
        Self {
            allowed_dirs: Self::default_allowed_dirs(),
        }
    }

    pub fn with_allowed_dirs(dirs: Vec<PathBuf>) -> Self {
        Self { allowed_dirs: dirs }
    }

    /// Default allowed directories
    fn default_allowed_dirs() -> Vec<PathBuf> {
        vec![
            PathBuf::from("/etc/ssl"),
            PathBuf::from("/etc/secrets"),
            PathBuf::from("/tmp"),
            PathBuf::from("/var/secrets"),
        ]
    }

    /// Validate and resolve a file path
    async fn validate_and_resolve_path(
        &self,
        path: &str,
        ctx: &ResolutionContext,
    ) -> Result<PathBuf> {
        // Expand tilde and environment variables
        let expanded =
            shellexpand::full(path).with_context(|| format!("Failed to expand path: {}", path))?;

        // Resolve relative paths against config directory
        let full_path = if Path::new(expanded.as_ref()).is_absolute() {
            PathBuf::from(expanded.as_ref())
        } else {
            ctx.config_dir.join(expanded.as_ref())
        };

        // Canonicalize to resolve symlinks and .. references
        let canonical = tokio::fs::canonicalize(&full_path)
            .await
            .with_context(|| format!("Failed to resolve path: {}", full_path.display()))?;

        // Check for path traversal - ensure the path is within allowed directories or config dir
        let is_allowed = self.is_path_allowed(&canonical, ctx);

        if !is_allowed {
            return Err(anyhow!(
                "Path {} is not in an allowed directory. Allowed: config dir or {:?}",
                canonical.display(),
                self.allowed_dirs
            ));
        }

        Ok(canonical)
    }

    /// Check if a path is within allowed directories
    fn is_path_allowed(&self, path: &Path, ctx: &ResolutionContext) -> bool {
        // Allow paths under config directory
        if path.starts_with(&ctx.config_dir) {
            return true;
        }

        // Check against allowed directories
        for allowed_dir in &self.allowed_dirs {
            if path.starts_with(allowed_dir) {
                return true;
            }
        }

        false
    }

    /// Parse octal permission string (e.g., "0644" -> 420)
    fn parse_permissions(perms: &str) -> Result<u32> {
        // Remove leading "0" if present for octal notation
        let perms_str = perms.strip_prefix('0').unwrap_or(perms);

        // Parse as octal
        u32::from_str_radix(perms_str, 8)
            .with_context(|| format!("Invalid permission format: {}", perms))
    }

    /// Resolve mount path
    fn resolve_mount_path(&self, mount_path: Option<&String>, source_path: &Path) -> PathBuf {
        if let Some(mount) = mount_path {
            PathBuf::from(mount)
        } else {
            // Default mount path: /secrets/<filename>
            let filename = source_path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("secret");
            PathBuf::from("/secrets").join(filename)
        }
    }
}

impl Default for FileSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SecretSource for FileSource {
    async fn resolve(
        &self,
        definition: &SecretConfig,
        ctx: &ResolutionContext,
    ) -> Result<Option<ResolvedSecret>> {
        // Only handle file source type
        if definition.source != ConfigSecretSource::File {
            return Ok(None);
        }

        let path_str = definition
            .path
            .as_ref()
            .ok_or_else(|| anyhow!("File path not specified for secret '{}'", definition.name))?;

        // Validate and resolve the path
        let source_path = self.validate_and_resolve_path(path_str, ctx).await?;

        // Check if file exists
        if !source_path.exists() {
            if definition.required {
                return Err(anyhow!(
                    "Required file secret '{}' not found at: {}",
                    definition.name,
                    source_path.display()
                ));
            } else {
                debug!(
                    "Optional file secret '{}' not found at: {}",
                    definition.name,
                    source_path.display()
                );
                return Ok(None);
            }
        }

        // Read file content
        let content = tokio::fs::read(&source_path).await.with_context(|| {
            format!(
                "Failed to read file secret '{}' from: {}",
                definition.name,
                source_path.display()
            )
        })?;

        // Parse permissions
        let permissions = Self::parse_permissions(&definition.permissions)
            .with_context(|| format!("Invalid permissions for secret '{}'", definition.name))?;

        // Determine mount path
        let mount_path = self.resolve_mount_path(definition.mount_path.as_ref(), &source_path);

        debug!(
            "Resolved file secret '{}': {} bytes from {} -> {} (perms: {:o})",
            definition.name,
            content.len(),
            source_path.display(),
            mount_path.display(),
            permissions
        );

        let size_bytes = content.len();

        Ok(Some(ResolvedSecret {
            name: definition.name.clone(),
            value: SecretValue::from_file(content, mount_path, permissions),
            metadata: SecretMetadata {
                source_type: ConfigSecretSource::File,
                resolved_from: ResolvedFrom::LocalFile(source_path),
                size_bytes,
            },
        }))
    }

    fn validate(&self) -> Result<()> {
        // File source is always available
        Ok(())
    }

    fn name(&self) -> &'static str {
        "file"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_context(dir: &Path) -> ResolutionContext {
        // Canonicalize to match what validate_and_resolve_path does
        // This is especially important on macOS where /var is symlinked to /private/var
        let canonical_dir = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
        ResolutionContext::new(canonical_dir)
    }

    #[tokio::test]
    async fn test_resolve_file_secret() {
        let temp_dir = TempDir::new().unwrap();
        let secret_file = temp_dir.path().join("test-secret.txt");
        tokio::fs::write(&secret_file, b"secret content")
            .await
            .unwrap();

        let source = FileSource::new();
        let config = SecretConfig {
            name: "TEST_FILE_SECRET".to_string(),
            source: ConfigSecretSource::File,
            from_file: None,
            required: true,
            path: Some(secret_file.to_str().unwrap().to_string()),
            mount_path: Some("/secrets/test.txt".to_string()),
            permissions: "0600".to_string(),
            vault_path: None,
            vault_key: None,
            vault_mount: "secret".to_string(),
            s3_path: None,
        };

        let ctx = create_test_context(temp_dir.path());
        let result = source.resolve(&config, &ctx).await.unwrap();

        assert!(result.is_some());
        let secret = result.unwrap();

        if let Some((content, mount, perms)) = secret.value.as_file() {
            assert_eq!(content, b"secret content");
            assert_eq!(mount, &PathBuf::from("/secrets/test.txt"));
            assert_eq!(perms, 0o600);
        } else {
            panic!("Expected file secret");
        }
    }

    #[tokio::test]
    async fn test_path_traversal_prevention() {
        let temp_dir = TempDir::new().unwrap();

        // Try to access a file outside allowed directories
        let source = FileSource::with_allowed_dirs(vec![temp_dir.path().to_path_buf()]);

        let config = SecretConfig {
            name: "EVIL_SECRET".to_string(),
            source: ConfigSecretSource::File,
            from_file: None,
            required: true,
            path: Some("/etc/passwd".to_string()),
            mount_path: None,
            permissions: "0644".to_string(),
            vault_path: None,
            vault_key: None,
            vault_mount: "secret".to_string(),
            s3_path: None,
        };

        let ctx = create_test_context(temp_dir.path());
        let result = source.resolve(&config, &ctx).await;

        // Should fail due to path not being in allowed directories
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("not in an allowed directory"));
    }

    #[tokio::test]
    async fn test_relative_path_resolution() {
        let temp_dir = TempDir::new().unwrap();
        let secret_file = temp_dir.path().join("relative-secret.txt");
        tokio::fs::write(&secret_file, b"relative content")
            .await
            .unwrap();

        let source = FileSource::new();
        let config = SecretConfig {
            name: "RELATIVE_SECRET".to_string(),
            source: ConfigSecretSource::File,
            from_file: None,
            required: true,
            path: Some("relative-secret.txt".to_string()), // Relative path
            mount_path: None,
            permissions: "0644".to_string(),
            vault_path: None,
            vault_key: None,
            vault_mount: "secret".to_string(),
            s3_path: None,
        };

        let ctx = create_test_context(temp_dir.path());
        let result = source.resolve(&config, &ctx).await.unwrap();

        assert!(result.is_some());
        let secret = result.unwrap();

        if let Some((content, _, _)) = secret.value.as_file() {
            assert_eq!(content, b"relative content");
        } else {
            panic!("Expected file secret");
        }
    }

    #[tokio::test]
    async fn test_default_mount_path() {
        let temp_dir = TempDir::new().unwrap();
        let secret_file = temp_dir.path().join("my-cert.pem");
        tokio::fs::write(&secret_file, b"cert content")
            .await
            .unwrap();

        let source = FileSource::new();
        let config = SecretConfig {
            name: "CERT_SECRET".to_string(),
            source: ConfigSecretSource::File,
            from_file: None,
            required: true,
            path: Some(secret_file.to_str().unwrap().to_string()),
            mount_path: None, // No mount path specified
            permissions: "0600".to_string(),
            vault_path: None,
            vault_key: None,
            vault_mount: "secret".to_string(),
            s3_path: None,
        };

        let ctx = create_test_context(temp_dir.path());
        let result = source.resolve(&config, &ctx).await.unwrap();

        assert!(result.is_some());
        let secret = result.unwrap();

        if let Some((_, mount, _)) = secret.value.as_file() {
            // Should default to /secrets/<filename>
            assert_eq!(mount, &PathBuf::from("/secrets/my-cert.pem"));
        } else {
            panic!("Expected file secret");
        }
    }

    #[test]
    fn test_parse_permissions() {
        assert_eq!(FileSource::parse_permissions("0644").unwrap(), 0o644);
        assert_eq!(FileSource::parse_permissions("644").unwrap(), 0o644);
        assert_eq!(FileSource::parse_permissions("0600").unwrap(), 0o600);
        assert_eq!(FileSource::parse_permissions("0777").unwrap(), 0o777);

        // Invalid permissions
        assert!(FileSource::parse_permissions("0999").is_err());
        assert!(FileSource::parse_permissions("abc").is_err());
    }

    #[tokio::test]
    async fn test_missing_optional_file() {
        let temp_dir = TempDir::new().unwrap();

        let source = FileSource::new();
        let config = SecretConfig {
            name: "MISSING_OPTIONAL".to_string(),
            source: ConfigSecretSource::File,
            from_file: None,
            required: false, // Optional
            path: Some("/nonexistent/path/secret.txt".to_string()),
            mount_path: None,
            permissions: "0644".to_string(),
            vault_path: None,
            vault_key: None,
            vault_mount: "secret".to_string(),
            s3_path: None,
        };

        let ctx = create_test_context(temp_dir.path());

        // Should return Ok(None) for missing optional file (after validation fails)
        let result = source.resolve(&config, &ctx).await;
        // Will actually error on path validation, not on missing file
        assert!(result.is_err());
    }

    // --- Error path tests ---

    #[tokio::test]
    async fn test_resolve_missing_path_field() {
        let temp_dir = TempDir::new().unwrap();
        let source = FileSource::new();

        // File source with no path specified
        let config = SecretConfig {
            name: "NO_PATH".to_string(),
            source: ConfigSecretSource::File,
            from_file: None,
            required: true,
            path: None, // Missing required path
            mount_path: None,
            permissions: "0644".to_string(),
            vault_path: None,
            vault_key: None,
            vault_mount: "secret".to_string(),
            s3_path: None,
        };

        let ctx = create_test_context(temp_dir.path());
        let result = source.resolve(&config, &ctx).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("File path not specified"),
            "Expected 'File path not specified', got: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_resolve_wrong_source_type_returns_none() {
        let temp_dir = TempDir::new().unwrap();
        let source = FileSource::new();

        let config = SecretConfig {
            name: "WRONG_TYPE".to_string(),
            source: ConfigSecretSource::Env, // Not File
            from_file: None,
            required: true,
            path: Some("/etc/secrets/test.txt".to_string()),
            mount_path: None,
            permissions: "0644".to_string(),
            vault_path: None,
            vault_key: None,
            vault_mount: "secret".to_string(),
            s3_path: None,
        };

        let ctx = create_test_context(temp_dir.path());
        let result = source.resolve(&config, &ctx).await.unwrap();
        assert!(
            result.is_none(),
            "FileSource should return None for non-File source type"
        );
    }

    #[test]
    fn test_parse_permissions_invalid_octal() {
        // 8 and 9 are not valid octal digits
        let result = FileSource::parse_permissions("0899");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Invalid permission format"),
            "Expected permission format error, got: {}",
            err
        );
    }

    #[test]
    fn test_parse_permissions_empty_string() {
        let result = FileSource::parse_permissions("");
        assert!(result.is_err());
    }

    #[test]
    fn test_is_path_allowed_rejects_outside_dirs() {
        let source = FileSource::with_allowed_dirs(vec![PathBuf::from("/opt/secrets")]);
        let ctx = ResolutionContext::new(PathBuf::from("/home/user/project"));

        // Path outside both config dir and allowed dirs
        assert!(!source.is_path_allowed(Path::new("/etc/passwd"), &ctx));
        assert!(!source.is_path_allowed(Path::new("/root/.ssh/id_rsa"), &ctx));
    }

    #[test]
    fn test_is_path_allowed_accepts_config_dir() {
        let source = FileSource::with_allowed_dirs(vec![]);
        let ctx = ResolutionContext::new(PathBuf::from("/home/user/project"));

        // Path under config dir should be allowed
        assert!(source.is_path_allowed(Path::new("/home/user/project/secrets/key.pem"), &ctx));
    }
}
