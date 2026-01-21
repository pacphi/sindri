//! Environment variable secret source
//!
//! Precedence chain (highest to lowest):
//! 1. Shell environment variables
//! 2. .env.local file
//! 3. .env file
//! 4. fromFile property (read value from specified file)

use crate::sources::SecretSource;
use crate::types::{ResolvedFrom, ResolvedSecret, ResolutionContext, SecretMetadata, SecretValue};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use sindri_core::types::{SecretConfig, SecretSource as ConfigSecretSource};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::debug;

pub struct EnvSource {
    /// Cached .env files
    env_cache: tokio::sync::RwLock<Option<EnvFiles>>,
}

#[derive(Debug, Clone)]
struct EnvFiles {
    env_local: HashMap<String, String>,
    env: HashMap<String, String>,
    config_dir: PathBuf,
}

impl EnvSource {
    pub fn new() -> Self {
        Self {
            env_cache: tokio::sync::RwLock::new(None),
        }
    }

    /// Load .env files from the config directory
    async fn load_env_files(&self, ctx: &ResolutionContext) -> Result<EnvFiles> {
        // Check cache first
        {
            let cache = self.env_cache.read().await;
            if let Some(cached) = cache.as_ref() {
                if cached.config_dir == ctx.config_dir {
                    return Ok(cached.clone());
                }
            }
        }

        // Load .env.local
        let env_local_path = ctx.config_dir.join(".env.local");
        let env_local = if env_local_path.exists() {
            debug!("Loading .env.local from: {}", env_local_path.display());
            Self::parse_env_file(&env_local_path)
                .context("Failed to parse .env.local")?
        } else {
            HashMap::new()
        };

        // Load .env
        let env_path = ctx.config_dir.join(".env");
        let env = if env_path.exists() {
            debug!("Loading .env from: {}", env_path.display());
            Self::parse_env_file(&env_path)
                .context("Failed to parse .env")?
        } else {
            HashMap::new()
        };

        let files = EnvFiles {
            env_local,
            env,
            config_dir: ctx.config_dir.clone(),
        };

        // Cache the loaded files
        {
            let mut cache = self.env_cache.write().await;
            *cache = Some(files.clone());
        }

        Ok(files)
    }

    /// Parse a .env file using dotenvy
    fn parse_env_file(path: &Path) -> Result<HashMap<String, String>> {
        let mut vars = HashMap::new();

        // Read file content
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read env file: {}", path.display()))?;

        // Parse line by line
        for (idx, line) in content.lines().enumerate() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse KEY=VALUE format
            if let Some(pos) = line.find('=') {
                let key = line[..pos].trim().to_string();
                let value = line[pos + 1..].trim();

                // Handle quoted values
                let value = if (value.starts_with('"') && value.ends_with('"'))
                    || (value.starts_with('\'') && value.ends_with('\''))
                {
                    // Remove quotes
                    value[1..value.len() - 1].to_string()
                } else {
                    value.to_string()
                };

                vars.insert(key, value);
            } else {
                debug!(
                    "Skipping malformed line {} in {}: {}",
                    idx + 1,
                    path.display(),
                    line
                );
            }
        }

        Ok(vars)
    }

    /// Resolve from file specified in fromFile property
    async fn resolve_from_file(&self, path: &str, ctx: &ResolutionContext) -> Result<(String, PathBuf)> {
        // Expand tilde and environment variables
        let expanded = shellexpand::full(path)
            .with_context(|| format!("Failed to expand path: {}", path))?;

        // Resolve relative paths against config directory
        let full_path = if Path::new(expanded.as_ref()).is_absolute() {
            PathBuf::from(expanded.as_ref())
        } else {
            ctx.config_dir.join(expanded.as_ref())
        };

        // Read file content
        let content = tokio::fs::read_to_string(&full_path)
            .await
            .with_context(|| format!("Failed to read file: {}", full_path.display()))?;

        Ok((content.trim().to_string(), full_path))
    }
}

impl Default for EnvSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SecretSource for EnvSource {
    async fn resolve(
        &self,
        definition: &SecretConfig,
        ctx: &ResolutionContext,
    ) -> Result<Option<ResolvedSecret>> {
        // Only handle env source type
        if definition.source != ConfigSecretSource::Env {
            return Ok(None);
        }

        let name = &definition.name;
        let mut value: Option<String> = None;
        let mut resolved_from: Option<ResolvedFrom> = None;

        // 1. Try shell environment (highest priority)
        if let Ok(env_value) = std::env::var(name) {
            debug!("Resolved {} from shell environment", name);
            value = Some(env_value);
            resolved_from = Some(ResolvedFrom::ShellEnv);
        }

        // 2. Try .env.local file
        if value.is_none() {
            let env_files = self.load_env_files(ctx).await?;
            if let Some(env_value) = env_files.env_local.get(name) {
                debug!("Resolved {} from .env.local", name);
                value = Some(env_value.clone());
                resolved_from = Some(ResolvedFrom::EnvLocalFile);
            }
        }

        // 3. Try .env file
        if value.is_none() {
            let env_files = self.load_env_files(ctx).await?;
            if let Some(env_value) = env_files.env.get(name) {
                debug!("Resolved {} from .env", name);
                value = Some(env_value.clone());
                resolved_from = Some(ResolvedFrom::EnvFile);
            }
        }

        // 4. Try fromFile property (lowest priority)
        if value.is_none() {
            if let Some(from_file) = &definition.from_file {
                match self.resolve_from_file(from_file, ctx).await {
                    Ok((file_value, file_path)) => {
                        debug!("Resolved {} from file: {}", name, file_path.display());
                        value = Some(file_value);
                        resolved_from = Some(ResolvedFrom::FromFile(file_path));
                    }
                    Err(e) => {
                        if definition.required {
                            return Err(e).context(format!(
                                "Failed to resolve required secret '{}' from file",
                                name
                            ));
                        } else {
                            debug!("Optional secret '{}' fromFile failed: {}", name, e);
                        }
                    }
                }
            }
        }

        // Return None if not found
        let Some(secret_value) = value else {
            return Ok(None);
        };

        let size_bytes = secret_value.len();

        Ok(Some(ResolvedSecret {
            name: name.clone(),
            value: SecretValue::from_env(secret_value),
            metadata: SecretMetadata {
                source_type: ConfigSecretSource::Env,
                resolved_from: resolved_from.unwrap(),
                size_bytes,
            },
        }))
    }

    fn validate(&self) -> Result<()> {
        // Env source is always available
        Ok(())
    }

    fn name(&self) -> &'static str {
        "env"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    fn create_test_context(dir: &Path) -> ResolutionContext {
        ResolutionContext::new(dir.to_path_buf())
    }

    #[tokio::test]
    async fn test_resolve_from_shell_env() {
        env::set_var("TEST_SECRET_ENV", "from-shell");

        let source = EnvSource::new();
        let config = SecretConfig {
            name: "TEST_SECRET_ENV".to_string(),
            source: ConfigSecretSource::Env,
            from_file: None,
            required: false,
            path: None,
            mount_path: None,
            permissions: "0644".to_string(),
            vault_path: None,
            vault_key: None,
            vault_mount: "secret".to_string(),
            s3_path: None,
        };

        let ctx = create_test_context(Path::new("."));
        let result = source.resolve(&config, &ctx).await.unwrap();

        assert!(result.is_some());
        let secret = result.unwrap();
        assert_eq!(secret.value.as_string(), Some("from-shell"));
        assert!(matches!(secret.metadata.resolved_from, ResolvedFrom::ShellEnv));

        env::remove_var("TEST_SECRET_ENV");
    }

    #[tokio::test]
    async fn test_resolve_from_env_file() {
        let temp_dir = TempDir::new().unwrap();
        let env_path = temp_dir.path().join(".env");

        std::fs::write(&env_path, "TEST_ENV_FILE=from-env-file\n").unwrap();

        let source = EnvSource::new();
        let config = SecretConfig {
            name: "TEST_ENV_FILE".to_string(),
            source: ConfigSecretSource::Env,
            from_file: None,
            required: false,
            path: None,
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
        assert_eq!(secret.value.as_string(), Some("from-env-file"));
        assert!(matches!(secret.metadata.resolved_from, ResolvedFrom::EnvFile));
    }

    #[tokio::test]
    async fn test_precedence_shell_over_file() {
        env::set_var("TEST_PRECEDENCE", "from-shell");

        let temp_dir = TempDir::new().unwrap();
        let env_path = temp_dir.path().join(".env");
        std::fs::write(&env_path, "TEST_PRECEDENCE=from-file\n").unwrap();

        let source = EnvSource::new();
        let config = SecretConfig {
            name: "TEST_PRECEDENCE".to_string(),
            source: ConfigSecretSource::Env,
            from_file: None,
            required: false,
            path: None,
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
        // Shell env should win
        assert_eq!(secret.value.as_string(), Some("from-shell"));
        assert!(matches!(secret.metadata.resolved_from, ResolvedFrom::ShellEnv));

        env::remove_var("TEST_PRECEDENCE");
    }

    #[tokio::test]
    async fn test_resolve_from_file_property() {
        let temp_dir = TempDir::new().unwrap();
        let secret_file = temp_dir.path().join("secret.txt");
        std::fs::write(&secret_file, "secret-content").unwrap();

        let source = EnvSource::new();
        let config = SecretConfig {
            name: "TEST_FROM_FILE".to_string(),
            source: ConfigSecretSource::Env,
            from_file: Some(secret_file.to_str().unwrap().to_string()),
            required: false,
            path: None,
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
        assert_eq!(secret.value.as_string(), Some("secret-content"));
    }

    #[test]
    fn test_parse_env_file() {
        let content = r#"
# Comment line
KEY1=value1
KEY2="quoted value"
KEY3='single quoted'
EMPTY_LINE_ABOVE=

# Another comment
KEY4=value with spaces
"#;

        let temp_dir = TempDir::new().unwrap();
        let env_path = temp_dir.path().join(".env");
        std::fs::write(&env_path, content).unwrap();

        let vars = EnvSource::parse_env_file(&env_path).unwrap();

        assert_eq!(vars.get("KEY1"), Some(&"value1".to_string()));
        assert_eq!(vars.get("KEY2"), Some(&"quoted value".to_string()));
        assert_eq!(vars.get("KEY3"), Some(&"single quoted".to_string()));
        assert_eq!(vars.get("KEY4"), Some(&"value with spaces".to_string()));
        assert_eq!(vars.get("EMPTY_LINE_ABOVE"), Some(&"".to_string()));
    }
}
