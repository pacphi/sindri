//! Configuration file loading and parsing

use crate::error::{Error, Result};
use crate::schema::SchemaValidator;
use crate::templates::{ConfigInitContext, ConfigTemplateRegistry};
use crate::types::{
    DeploymentConfig, ExtensionsConfig, Provider, ProvidersConfig, ResolutionStrategy,
    ResourcesConfig, SecretConfig, SindriConfigFile, VolumesConfig,
};
use camino::{Utf8Path, Utf8PathBuf};
use std::fs;
use std::future::Future;
use std::pin::Pin;

/// Trait for resolving image versions from a container registry.
///
/// This abstracts the registry interaction so that sindri-core does not
/// depend on the concrete sindri-image crate.  Implementors live in
/// sindri-image (or tests can provide a mock).
pub trait ImageVersionResolver: Send + Sync {
    /// Resolve a version constraint to a concrete image tag.
    ///
    /// # Arguments
    /// * `repository` - Repository path (e.g., "pacphi/sindri")
    /// * `strategy`   - Resolution strategy from the config
    /// * `constraint`  - Optional semver constraint (e.g., "^3.0.0")
    /// * `cli_version` - Optional CLI version for PinToCli strategy
    /// * `allow_prerelease` - Whether to include prerelease versions
    fn resolve<'a>(
        &'a self,
        repository: &'a str,
        strategy: ResolutionStrategy,
        constraint: Option<&'a str>,
        cli_version: Option<&'a str>,
        allow_prerelease: bool,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<String>> + Send + 'a>>;
}

/// Configuration file names to search for
const CONFIG_FILE_NAMES: &[&str] = &["sindri.yaml", "sindri.yml"];

/// Default image registry
const DEFAULT_IMAGE_REGISTRY: &str = "ghcr.io/pacphi/sindri";

/// Default image tag
const DEFAULT_IMAGE_TAG: &str = "latest";

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
        let config: SindriConfigFile = serde_yaml_ng::from_str(&content)?;

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
        let config: SindriConfigFile = serde_yaml_ng::from_str(&content)?;

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
                image: Some(format!("{}:{}", DEFAULT_IMAGE_REGISTRY, DEFAULT_IMAGE_TAG)),
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
    /// 5. default fallback (see DEFAULT_IMAGE_REGISTRY and DEFAULT_IMAGE_TAG constants)
    ///
    /// # Arguments
    /// * `resolver` - An optional image version resolver for registry-based version
    ///   resolution (Priority 3). Required only when `image_config.version` is set.
    ///
    /// # Returns
    /// Fully resolved image reference (e.g., "ghcr.io/pacphi/sindri:v3.0.0")
    pub async fn resolve_image(
        &self,
        resolver: Option<&dyn ImageVersionResolver>,
    ) -> Result<String> {
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

                let resolver = resolver.ok_or_else(|| {
                    Error::invalid_config(
                        "image_config.version requires an image version resolver, \
                         but none was provided",
                    )
                })?;

                // Get CLI version for PinToCli strategy
                let cli_version = env!("CARGO_PKG_VERSION");

                // Resolve version based on strategy
                let tag = resolver
                    .resolve(
                        repository,
                        image_config.resolution_strategy,
                        Some(version_constraint),
                        Some(cli_version),
                        image_config.allow_prerelease,
                    )
                    .await
                    .map_err(|e| {
                        Error::invalid_config(format!("Failed to resolve image version: {:#}", e))
                    })?;

                info!("Resolved to: {}:{}", registry, tag);
                return Ok(format!("{}:{}", registry, tag));
            }

            // No version specified but image_config exists - use default tag
            info!(
                "No version specified in image_config, using default tag: {}",
                DEFAULT_IMAGE_TAG
            );
            return Ok(format!("{}:{}", registry, DEFAULT_IMAGE_TAG));
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
        serde_yaml_ng::to_string(&self.config).map_err(Error::from)
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
  image: {registry}:{tag}
  resources:
    memory: 4GB
    cpus: 2

extensions:
  profile: minimal
"#,
            name = name,
            provider = format!("{:?}", provider).to_lowercase(),
            registry = DEFAULT_IMAGE_REGISTRY,
            tag = DEFAULT_IMAGE_TAG
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
        let config: SindriConfigFile = serde_yaml_ng::from_str(yaml).unwrap();
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
        let config: SindriConfigFile = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(config.name, "full-project");
        assert_eq!(config.deployment.provider, Provider::Fly);
        assert!(config.deployment.resources.gpu.is_some());
        assert_eq!(config.secrets.len(), 1);
    }

    // --- Error path tests ---

    #[test]
    fn test_load_nonexistent_file() {
        let path = Utf8Path::new("/tmp/nonexistent-sindri-config-12345.yaml");
        let result = SindriConfig::load(Some(path));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, Error::ConfigNotFound { .. }),
            "Expected ConfigNotFound, got: {:?}",
            err
        );
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn test_load_invalid_yaml_syntax() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config_path = temp_dir.path().join("sindri.yaml");
        std::fs::write(
            &config_path,
            "version: \"3.0\"\nname: test\n  bad_indent: [[[",
        )
        .unwrap();

        let utf8_path =
            Utf8PathBuf::from_path_buf(config_path).expect("path should be valid UTF-8");
        let result = SindriConfig::load(Some(utf8_path.as_path()));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, Error::YamlParse(_)),
            "Expected YamlParse, got: {:?}",
            err
        );
    }

    #[test]
    fn test_load_yaml_missing_required_fields() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config_path = temp_dir.path().join("sindri.yaml");
        // Missing deployment and extensions - should fail deserialization
        std::fs::write(&config_path, "version: \"3.0\"\nname: test\n").unwrap();

        let utf8_path =
            Utf8PathBuf::from_path_buf(config_path).expect("path should be valid UTF-8");
        let result = SindriConfig::load(Some(utf8_path.as_path()));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("missing field"),
            "Expected 'missing field' in error, got: {}",
            err
        );
    }

    #[test]
    fn test_load_yaml_wrong_provider_type() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config_path = temp_dir.path().join("sindri.yaml");
        let yaml = r#"
version: "3.0"
name: test
deployment:
  provider: nonexistent-provider
extensions:
  profile: minimal
"#;
        std::fs::write(&config_path, yaml).unwrap();

        let utf8_path =
            Utf8PathBuf::from_path_buf(config_path).expect("path should be valid UTF-8");
        let result = SindriConfig::load(Some(utf8_path.as_path()));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("unknown variant"),
            "Expected 'unknown variant' in error, got: {}",
            err
        );
    }

    #[test]
    fn test_load_and_validate_schema_failure() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config_path = temp_dir.path().join("sindri.yaml");
        // Valid YAML but invalid schema: name uses uppercase (violates pattern)
        let yaml = r#"
version: "3.0"
name: INVALID-UPPERCASE
deployment:
  provider: docker
extensions:
  profile: minimal
"#;
        std::fs::write(&config_path, yaml).unwrap();

        let validator = crate::schema::SchemaValidator::new().unwrap();
        let utf8_path =
            Utf8PathBuf::from_path_buf(config_path).expect("path should be valid UTF-8");
        let result = SindriConfig::load_and_validate(Some(utf8_path.as_path()), &validator);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, Error::SchemaValidation { .. }),
            "Expected SchemaValidation, got: {:?}",
            err
        );
    }
}
