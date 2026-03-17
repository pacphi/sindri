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

// ─── Default image registry constants ────────────────────────────────────────
//
// These three constants define the default container image that `sindri config init`
// generates and that the CLI falls back to when no image is configured.
//
// **Forking / private registry:**
//   1. Update the three constants below to point to your registry.
//   2. Rebuild the CLI (`cargo build --release`).
//   3. The Tera config template, default certificate identity, and fallback image
//      reference all derive from these values automatically.
//
//   The Makefile also honours `REGISTRY ?= ghcr.io/pacphi` which can be
//   overridden at build time:
//      make v3-docker-build REGISTRY=ghcr.io/myorg
//
//   End-users can always override per-project via `sindri.yaml`:
//      deployment:
//        image: myregistry.example.com/myorg/sindri:v3-latest
//   or via the structured `image_config` block.
// ─────────────────────────────────────────────────────────────────────────────

/// Default container registry host (e.g., `"ghcr.io"`, `"docker.io"`).
pub const DEFAULT_REGISTRY_HOST: &str = "ghcr.io";

/// Default registry owner / namespace.
pub const DEFAULT_REGISTRY_OWNER: &str = "pacphi";

/// Default image name.
pub const DEFAULT_IMAGE_NAME: &str = "sindri";

/// Default image tag used by [`generate_default_config`].
const DEFAULT_IMAGE_TAG: &str = "latest";

/// Compose the full default image registry reference.
///
/// Returns `"{host}/{owner}/{image}"` (e.g., `"ghcr.io/pacphi/sindri"`).
/// All template rendering and fallback logic should call this rather than
/// assembling the string ad-hoc.
pub fn default_image_registry() -> String {
    format!(
        "{}/{}/{}",
        DEFAULT_REGISTRY_HOST, DEFAULT_REGISTRY_OWNER, DEFAULT_IMAGE_NAME
    )
}

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
                distro: crate::types::Distro::default(),
                image: Some(format!(
                    "{}:{}",
                    default_image_registry(),
                    DEFAULT_IMAGE_TAG
                )),
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

    /// Validate that the image tag matches the declared distro.
    /// Returns a warning message if mismatched, None if consistent.
    pub fn validate_distro_image_consistency(&self) -> Option<String> {
        use crate::types::Distro;

        let distro = self.config.deployment.distro;

        // Determine the effective image reference
        let image_ref = if let Some(ic) = &self.config.deployment.image_config {
            // Skip check for digest-pinned images
            if ic.digest.is_some() {
                return None;
            }
            // Skip check for version-based resolution (resolved at deploy time)
            if ic.version.is_some() {
                return None;
            }
            ic.tag_override.as_deref().map(|t| format!(":{}", t))
        } else {
            self.config.deployment.image.clone()
        };

        let image_ref = image_ref?;

        // Extract the tag portion (after the last ':')
        let tag = match image_ref.rsplit_once(':') {
            Some((_, tag)) => tag,
            None => return None, // No tag, can't validate
        };

        // Skip digest references
        if tag.contains("sha256") {
            return None;
        }

        match distro {
            Distro::Ubuntu => {
                if tag.contains("-fedora") || tag.contains("-opensuse") {
                    Some(format!(
                        "Distro is 'ubuntu' but image tag '{}' appears to target a different distro",
                        tag
                    ))
                } else {
                    None
                }
            }
            Distro::Fedora => {
                if !tag.contains("-fedora") {
                    Some(format!(
                        "Distro is 'fedora' but image tag '{}' does not contain '-fedora'",
                        tag
                    ))
                } else {
                    None
                }
            }
            Distro::Opensuse => {
                if !tag.contains("-opensuse") {
                    Some(format!(
                        "Distro is 'opensuse' but image tag '{}' does not contain '-opensuse'",
                        tag
                    ))
                } else {
                    None
                }
            }
        }
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
/// * `distro` - Target Linux distribution (ubuntu, fedora, opensuse)
///
/// # Returns
/// Generated YAML configuration string
pub fn generate_config(
    name: &str,
    provider: Provider,
    profile: &str,
    distro: &str,
) -> Result<String> {
    let registry = ConfigTemplateRegistry::new().map_err(|e| Error::Template(e.to_string()))?;
    let context = ConfigInitContext::new(name, provider, profile, distro);
    registry
        .render_config(&context)
        .map_err(|e| Error::Template(e.to_string()))
}

/// Generate a default sindri.yaml content (legacy wrapper)
///
/// This function is kept for backward compatibility.
/// Prefer using `generate_config` which accepts a profile parameter.
pub fn generate_default_config(name: &str, provider: Provider) -> String {
    generate_config(name, provider, "minimal", "ubuntu").unwrap_or_else(|_| {
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
            registry = default_image_registry(),
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

    #[test]
    fn test_config_without_distro_defaults_to_ubuntu() {
        let yaml = r#"
version: "3.0"
name: test-project
deployment:
  provider: docker
extensions:
  profile: minimal
"#;
        let config: SindriConfigFile = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(
            config.deployment.distro,
            crate::types::Distro::Ubuntu,
            "missing distro should default to ubuntu"
        );
    }

    #[test]
    fn test_distro_image_match_ubuntu_unsuffixed() {
        let config = make_config_with_distro_and_image(
            crate::types::Distro::Ubuntu,
            Some("ghcr.io/pacphi/sindri:v3-latest"),
        );
        assert!(config.validate_distro_image_consistency().is_none());
    }

    #[test]
    fn test_distro_image_mismatch_fedora_ubuntu_image() {
        let config = make_config_with_distro_and_image(
            crate::types::Distro::Fedora,
            Some("ghcr.io/pacphi/sindri:v3-latest"),
        );
        let warning = config.validate_distro_image_consistency();
        assert!(warning.is_some(), "fedora with unsuffixed tag should warn");
        assert!(warning.unwrap().contains("-fedora"));
    }

    #[test]
    fn test_distro_image_match_fedora() {
        let config = make_config_with_distro_and_image(
            crate::types::Distro::Fedora,
            Some("ghcr.io/pacphi/sindri:v3-latest-fedora"),
        );
        assert!(config.validate_distro_image_consistency().is_none());
    }

    #[test]
    fn test_distro_image_mismatch_ubuntu_with_fedora_tag() {
        let config = make_config_with_distro_and_image(
            crate::types::Distro::Ubuntu,
            Some("ghcr.io/pacphi/sindri:v3-latest-fedora"),
        );
        let warning = config.validate_distro_image_consistency();
        assert!(warning.is_some(), "ubuntu with fedora tag should warn");
    }

    #[test]
    fn test_distro_image_skip_digest() {
        let yaml = r#"
version: "3.0"
name: test-project
deployment:
  provider: docker
  distro: fedora
  image_config:
    registry: ghcr.io/pacphi/sindri
    digest: sha256:abc123def456
extensions:
  profile: minimal
"#;
        let config_file: SindriConfigFile = serde_yaml_ng::from_str(yaml).unwrap();
        let config = SindriConfig {
            config: config_file,
            config_path: "test.yaml".into(),
            working_dir: ".".into(),
        };
        assert!(
            config.validate_distro_image_consistency().is_none(),
            "digest-pinned images should skip check"
        );
    }

    #[test]
    fn test_distro_image_skip_no_image() {
        let config = make_config_with_distro_and_image(crate::types::Distro::Fedora, None);
        assert!(
            config.validate_distro_image_consistency().is_none(),
            "no image configured should skip check"
        );
    }

    fn make_config_with_distro_and_image(
        distro: crate::types::Distro,
        image: Option<&str>,
    ) -> SindriConfig {
        SindriConfig {
            config: SindriConfigFile {
                version: "3.0".to_string(),
                name: "test".to_string(),
                deployment: DeploymentConfig {
                    provider: Provider::Docker,
                    distro,
                    image: image.map(|s| s.to_string()),
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
            },
            config_path: "test.yaml".into(),
            working_dir: ".".into(),
        }
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
