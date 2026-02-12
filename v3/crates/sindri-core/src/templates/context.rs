//! Template context for config generation
//!
//! Provides context data for rendering sindri.yaml templates.

use crate::types::Provider;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tera::Context;

/// Embedded profiles.yaml content (compile-time inclusion)
/// This ensures profiles.yaml is the single source of truth
const PROFILES_YAML: &str = include_str!("../../../../profiles.yaml");

/// Root structure of profiles.yaml
#[derive(Debug, Clone, Deserialize)]
struct ProfilesFile {
    #[allow(dead_code)]
    version: String,
    display_order: Vec<String>,
    profiles: HashMap<String, ProfileDefinition>,
}

/// Profile definition as stored in profiles.yaml
#[derive(Debug, Clone, Deserialize)]
struct ProfileDefinition {
    description: String,
    extensions: Vec<String>,
}

/// Information about an extension profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileInfo {
    /// Profile name (e.g., "minimal", "fullstack")
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// List of extensions in this profile
    pub extensions: Vec<String>,
}

/// Context for rendering sindri.yaml templates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigInitContext {
    /// Project name
    pub name: String,
    /// Selected provider
    pub provider: String,
    /// Selected profile
    pub profile: String,
    /// Available profiles with descriptions
    pub profiles: Vec<ProfileInfo>,
    /// Whether the provider supports GPU
    pub provider_supports_gpu: bool,
    /// Whether the provider supports Docker-in-Docker
    pub provider_supports_dind: bool,
    /// Whether the provider supports SSH
    pub provider_supports_ssh: bool,
    /// Provider-specific region/location default
    pub default_region: String,
}

impl ConfigInitContext {
    /// Create a new config init context
    pub fn new(name: &str, provider: Provider, profile: &str) -> Self {
        let provider_str = provider.to_string();
        let (supports_gpu, supports_dind, supports_ssh, default_region) = match provider {
            Provider::Fly => (true, false, true, "sjc".to_string()),
            Provider::Docker | Provider::DockerCompose => (true, true, true, "".to_string()),
            Provider::Kubernetes => (true, true, true, "default".to_string()),
            Provider::Devpod => (true, true, true, "us-west-2".to_string()),
            Provider::E2b => (false, false, false, "".to_string()),
        };

        Self {
            name: name.to_string(),
            provider: provider_str,
            profile: profile.to_string(),
            profiles: Self::load_profiles(),
            provider_supports_gpu: supports_gpu,
            provider_supports_dind: supports_dind,
            provider_supports_ssh: supports_ssh,
            default_region,
        }
    }

    /// Load profile information from embedded profiles.yaml
    ///
    /// Reads from the compile-time embedded profiles.yaml file,
    /// ensuring profiles are always in sync with the source of truth.
    /// Display order is controlled by `display_order` in profiles.yaml.
    fn load_profiles() -> Vec<ProfileInfo> {
        let profiles_file: ProfilesFile = serde_yaml_ng::from_str(PROFILES_YAML)
            .expect("Failed to parse embedded profiles.yaml - this is a build error");

        profiles_file
            .display_order
            .iter()
            .filter_map(|name| {
                profiles_file.profiles.get(name).map(|def| ProfileInfo {
                    name: name.clone(),
                    description: def.description.clone(),
                    extensions: def.extensions.clone(),
                })
            })
            .collect()
    }

    /// Convert to Tera context for template rendering
    pub fn to_tera_context(&self) -> Result<Context> {
        let context = Context::from_serialize(self)?;
        Ok(context)
    }
}
