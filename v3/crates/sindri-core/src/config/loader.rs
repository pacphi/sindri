//! Configuration file loading and parsing

use crate::error::{Error, Result};
use crate::schema::SchemaValidator;
use crate::templates::{ConfigInitContext, ConfigTemplateRegistry};
use crate::types::{
    DeploymentConfig, ExtensionsConfig, Provider, ProvidersConfig, ResourcesConfig, SecretConfig,
    SindriConfigFile, VolumesConfig,
};
use camino::{Utf8Path, Utf8PathBuf};
use std::fs;

/// Configuration file names to search for
const CONFIG_FILE_NAMES: &[&str] = &["sindri.yaml", "sindri.yml"];

/// Loaded and validated Sindri configuration
#[derive(Debug, Clone)]
pub struct SindriConfig {
    /// The parsed configuration
    pub config: SindriConfigFile,

    /// Path to the configuration file
    pub config_path: Utf8PathBuf,

    /// Working directory
    pub working_dir: Utf8PathBuf,
}

impl SindriConfig {
    /// Load configuration from the specified path or search for it
    pub fn load(path: Option<&Utf8Path>) -> Result<Self> {
        let (config_path, content) = if let Some(p) = path {
            let content = fs::read_to_string(p).map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Error::config_not_found(p.as_str())
                } else {
                    Error::Io(e)
                }
            })?;
            (p.to_owned(), content)
        } else {
            Self::find_config()?
        };

        let working_dir = config_path
            .parent()
            .map(|p| p.to_owned())
            .unwrap_or_else(|| Utf8PathBuf::from("."));

        // Parse YAML
        let config: SindriConfigFile = serde_yaml::from_str(&content)?;

        Ok(Self {
            config,
            config_path,
            working_dir,
        })
    }

    /// Load and validate configuration
    pub fn load_and_validate(path: Option<&Utf8Path>, validator: &SchemaValidator) -> Result<Self> {
        let (config_path, content) = if let Some(p) = path {
            let content = fs::read_to_string(p).map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Error::config_not_found(p.as_str())
                } else {
                    Error::Io(e)
                }
            })?;
            (p.to_owned(), content)
        } else {
            Self::find_config()?
        };

        let working_dir = config_path
            .parent()
            .map(|p| p.to_owned())
            .unwrap_or_else(|| Utf8PathBuf::from("."));

        // Validate against schema first
        validator.validate_yaml(&content, "sindri")?;

        // Parse YAML
        let config: SindriConfigFile = serde_yaml::from_str(&content)?;

        Ok(Self {
            config,
            config_path,
            working_dir,
        })
    }

    /// Find configuration file in current directory or parent directories
    fn find_config() -> Result<(Utf8PathBuf, String)> {
        let cwd = std::env::current_dir().map_err(Error::Io)?;
        let cwd = Utf8PathBuf::try_from(cwd)
            .map_err(|_| Error::invalid_config("Current directory path is not valid UTF-8"))?;

        let mut current = cwd.as_path();

        loop {
            for name in CONFIG_FILE_NAMES {
                let path = current.join(name);
                if path.exists() {
                    let content = fs::read_to_string(&path)?;
                    return Ok((path, content));
                }
            }

            match current.parent() {
                Some(parent) => current = parent,
                None => break,
            }
        }

        Err(Error::config_not_found(
            "sindri.yaml (searched current and parent directories)",
        ))
    }

    /// Create a new default configuration
    pub fn new_default(name: &str, provider: Provider) -> SindriConfigFile {
        SindriConfigFile {
            version: "3.0".to_string(),
            name: name.to_string(),
            deployment: DeploymentConfig {
                provider,
                image: Some("ghcr.io/pacphi/sindri:v3-latest".to_string()),
                image_config: None,
                build_from_source: None,
                resources: ResourcesConfig::default(),
                volumes: VolumesConfig::default(),
            },
            extensions: ExtensionsConfig {
                profile: Some("minimal".to_string()),
                active: None,
                additional: None,
                auto_install: true,
            },
            secrets: Vec::new(),
            providers: ProvidersConfig::default(),
        }
    }

    /// Get the inner configuration file
    pub fn inner(&self) -> &SindriConfigFile {
        &self.config
    }

    /// Get deployment name
    pub fn name(&self) -> &str {
        &self.config.name
    }

    /// Get provider
    pub fn provider(&self) -> Provider {
        self.config.deployment.provider
    }

    /// Get extensions configuration
    pub fn extensions(&self) -> &ExtensionsConfig {
        &self.config.extensions
    }

    /// Get secrets configuration
    pub fn secrets(&self) -> &[SecretConfig] {
        &self.config.secrets
    }

    /// Get provider-specific configuration
    pub fn providers(&self) -> &ProvidersConfig {
        &self.config.providers
    }

    /// Get the deployment image
    pub fn image(&self) -> Option<&str> {
        self.config.deployment.image.as_deref()
    }

    /// Get resource configuration
    pub fn resources(&self) -> &ResourcesConfig {
        &self.config.deployment.resources
    }

    /// Resolve the container image reference
    ///
    /// Uses the following priority order:
    /// 1. image_config.digest (immutable)
    /// 2. image_config.tag_override (explicit tag)
    /// 3. image_config.version + resolution (semver constraint)
    /// 4. legacy image field
    /// 5. default fallback (ghcr.io/pacphi/sindri:latest)
    ///
    /// # Returns
    /// Fully resolved image reference (e.g., "ghcr.io/pacphi/sindri:v3.0.0")
    pub async fn resolve_image(&self) -> Result<String> {
        use crate::types::ResolutionStrategy;
        use sindri_image::{RegistryClient, VersionResolver};
        use tracing::{debug, info};

        // Check if image_config is provided
        if let Some(image_config) = &self.config.deployment.image_config {
            let registry = &image_config.registry;
            let repository = registry.split('/').skip(1).collect::<Vec<_>>().join("/");

            let repository = if repository.is_empty() {
                // If no slash in registry, assume it's docker.io and use registry as repo
                registry.as_str()
            } else {
                repository.as_str()
            };

            // Priority 1: Digest (immutable)
            if let Some(digest) = &image_config.digest {
                info!("Using pinned digest: {}", digest);
                return Ok(format!("{}@{}", registry, digest));
            }

            // Priority 2: Tag override (explicit)
            if let Some(tag) = &image_config.tag_override {
                info!("Using tag override: {}", tag);
                return Ok(format!("{}:{}", registry, tag));
            }

            // Priority 3: Version + resolution
            if let Some(version_constraint) = &image_config.version {
                debug!(
                    "Resolving version constraint: {} (strategy: {:?})",
                    version_constraint, image_config.resolution_strategy
                );

                // Get GitHub token from environment (optional)
                let github_token = std::env::var("GITHUB_TOKEN").ok();

                // Create registry client
                let mut registry_client = if registry.contains("ghcr.io") {
                    RegistryClient::new("ghcr.io")
                } else if registry.contains("docker.io") {
                    RegistryClient::new("docker.io")
                } else {
                    // Extract registry host from full path
                    let registry_host = registry.split('/').next().unwrap_or("ghcr.io");
                    RegistryClient::new(registry_host)
                };

                if let Some(token) = github_token {
                    registry_client = registry_client.with_token(token);
                }

                // Create version resolver
                let resolver = VersionResolver::new(registry_client);

                // Get CLI version for PinToCli strategy
                let cli_version = env!("CARGO_PKG_VERSION");

                // Convert ResolutionStrategy from config type to sindri-image type
                let strategy = match image_config.resolution_strategy {
                    ResolutionStrategy::Semver => sindri_image::ResolutionStrategy::Semver,
                    ResolutionStrategy::LatestStable => {
                        sindri_image::ResolutionStrategy::LatestStable
                    }
                    ResolutionStrategy::PinToCli => sindri_image::ResolutionStrategy::PinToCli,
                    ResolutionStrategy::Explicit => sindri_image::ResolutionStrategy::Explicit,
                };

                // Resolve version based on strategy
                let tag = resolver
                    .resolve_with_strategy(
                        repository,
                        strategy,
                        Some(version_constraint),
                        Some(cli_version),
                        image_config.allow_prerelease,
                    )
                    .await
                    .map_err(|e| {
                        Error::invalid_config(format!("Failed to resolve image version: {}", e))
                    })?;

                info!("Resolved to: {}:{}", registry, tag);
                return Ok(format!("{}:{}", registry, tag));
            }

            // No version specified but image_config exists - use latest
            info!("No version specified in image_config, using latest");
            return Ok(format!("{}:latest", registry));
        }

        // Priority 4: Legacy image field
        if let Some(image) = &self.config.deployment.image {
            debug!("Using legacy image field: {}", image);
            return Ok(image.clone());
        }

        // No image configured - this is valid for providers that build on demand
        Err(Error::invalid_config("No image configured"))
    }

    /// Serialize configuration to YAML
    pub fn to_yaml(&self) -> Result<String> {
        serde_yaml::to_string(&self.config).map_err(Error::from)
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let content = self.to_yaml()?;
        fs::write(&self.config_path, content)?;
        Ok(())
    }

    /// Save configuration to a specific path
    pub fn save_to(&self, path: &Utf8Path) -> Result<()> {
        let content = self.to_yaml()?;
        fs::write(path, content)?;
        Ok(())
    }
}

/// Generate a default sindri.yaml content using templates
///
/// # Arguments
/// * `name` - Project name
/// * `provider` - Deployment provider
/// * `profile` - Extension profile to use
///
/// # Returns
/// Generated YAML configuration string
pub fn generate_config(name: &str, provider: Provider, profile: &str) -> Result<String> {
    let registry = ConfigTemplateRegistry::new().map_err(|e| Error::Template(e.to_string()))?;
    let context = ConfigInitContext::new(name, provider, profile);
    registry
        .render_config(&context)
        .map_err(|e| Error::Template(e.to_string()))
}

/// Generate a default sindri.yaml content (legacy wrapper)
///
/// This function is kept for backward compatibility.
/// Prefer using `generate_config` which accepts a profile parameter.
pub fn generate_default_config(name: &str, provider: Provider) -> String {
    generate_config(name, provider, "minimal").unwrap_or_else(|_| {
        // Fallback to minimal hardcoded config if template fails
        format!(
            r#"---
# Sindri Configuration
version: "3.0"
name: {name}

deployment:
  provider: {provider}
  image: ghcr.io/pacphi/sindri:v3-latest
  resources:
    memory: 4GB
    cpus: 2

extensions:
  profile: minimal
"#,
            name = name,
            provider = provider
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_generation() {
        let config = generate_default_config("my-project", Provider::Docker);
        assert!(config.contains("name: my-project"));
        assert!(config.contains("provider: docker"));
    }

    #[test]
    fn test_parse_minimal_config() {
        let yaml = r#"
version: "3.0"
name: test-project
deployment:
  provider: docker
extensions:
  profile: minimal
"#;
        let config: SindriConfigFile = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.name, "test-project");
        assert_eq!(config.deployment.provider, Provider::Docker);
        assert_eq!(config.extensions.profile, Some("minimal".to_string()));
    }

    #[test]
    fn test_parse_full_config() {
        let yaml = r#"
version: "3.0"
name: full-project
deployment:
  provider: fly
  image: ghcr.io/org/sindri:v1
  resources:
    memory: 8GB
    cpus: 4
    gpu:
      enabled: true
      type: nvidia
      tier: gpu-medium
extensions:
  profile: anthropic-dev
  additional:
    - docker
secrets:
  - name: ANTHROPIC_API_KEY
    source: env
    required: true
providers:
  fly:
    region: ord
    autoStopMachines: true
"#;
        let config: SindriConfigFile = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.name, "full-project");
        assert_eq!(config.deployment.provider, Provider::Fly);
        assert!(config.deployment.resources.gpu.is_some());
        assert_eq!(config.secrets.len(), 1);
    }
}
