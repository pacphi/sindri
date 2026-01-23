//! Template context for config generation
//!
//! Provides context data for rendering sindri.yaml templates.

use crate::types::Provider;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tera::Context;

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

    /// Load profile information from embedded data
    fn load_profiles() -> Vec<ProfileInfo> {
        vec![
            ProfileInfo {
                name: "minimal".to_string(),
                description: "Minimal development setup".to_string(),
                extensions: vec!["nodejs".to_string(), "python".to_string()],
            },
            ProfileInfo {
                name: "fullstack".to_string(),
                description: "Full-stack web development".to_string(),
                extensions: vec![
                    "nodejs".to_string(),
                    "python".to_string(),
                    "docker".to_string(),
                    "nodejs-devtools".to_string(),
                ],
            },
            ProfileInfo {
                name: "ai-dev".to_string(),
                description: "AI/ML development environment".to_string(),
                extensions: vec![
                    "nodejs".to_string(),
                    "python".to_string(),
                    "golang".to_string(),
                    "spec-kit".to_string(),
                    "ollama".to_string(),
                    "ai-toolkit".to_string(),
                ],
            },
            ProfileInfo {
                name: "anthropic-dev".to_string(),
                description: "AI development with Anthropic toolset (v3 default - 10x performance)"
                    .to_string(),
                extensions: vec![
                    "agent-manager".to_string(),
                    "claude-flow-v3".to_string(),
                    "agentic-qe".to_string(),
                    "golang".to_string(),
                    "ollama".to_string(),
                    "ai-toolkit".to_string(),
                ],
            },
            ProfileInfo {
                name: "systems".to_string(),
                description: "Systems programming".to_string(),
                extensions: vec![
                    "rust".to_string(),
                    "golang".to_string(),
                    "docker".to_string(),
                    "infra-tools".to_string(),
                ],
            },
            ProfileInfo {
                name: "enterprise".to_string(),
                description: "Enterprise development (all languages)".to_string(),
                extensions: vec![
                    "nodejs".to_string(),
                    "python".to_string(),
                    "golang".to_string(),
                    "rust".to_string(),
                    "ruby".to_string(),
                    "jvm".to_string(),
                    "dotnet".to_string(),
                    "docker".to_string(),
                ],
            },
            ProfileInfo {
                name: "devops".to_string(),
                description: "DevOps and infrastructure".to_string(),
                extensions: vec![
                    "docker".to_string(),
                    "infra-tools".to_string(),
                    "monitoring".to_string(),
                    "cloud-tools".to_string(),
                ],
            },
            ProfileInfo {
                name: "mobile".to_string(),
                description: "Mobile development".to_string(),
                extensions: vec![
                    "nodejs".to_string(),
                    "linear-mcp".to_string(),
                    "supabase-cli".to_string(),
                ],
            },
        ]
    }

    /// Convert to Tera context for template rendering
    pub fn to_tera_context(&self) -> Result<Context> {
        let context = Context::from_serialize(self)?;
        Ok(context)
    }
}
