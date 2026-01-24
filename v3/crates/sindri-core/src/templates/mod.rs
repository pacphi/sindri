//! Template rendering for config generation
//!
//! Uses Tera templates to generate provider-specific sindri.yaml files.

mod context;

pub use context::{ConfigInitContext, ProfileInfo};

use anyhow::Result;
use tera::Tera;
use tracing::debug;

/// Template registry for config file generation
pub struct ConfigTemplateRegistry {
    tera: Tera,
}

impl ConfigTemplateRegistry {
    /// Create a new template registry with embedded templates
    pub fn new() -> Result<Self> {
        let mut tera = Tera::default();

        // Register the sindri.yaml template
        tera.add_raw_template("sindri.yaml", include_str!("sindri.yaml.tera"))?;

        Ok(Self { tera })
    }

    /// Render the sindri.yaml template with the given context
    pub fn render_config(&self, context: &ConfigInitContext) -> Result<String> {
        debug!(
            "Rendering sindri.yaml template for provider: {}",
            context.provider
        );
        let tera_context = context.to_tera_context()?;
        let rendered = self.tera.render("sindri.yaml", &tera_context)?;
        Ok(rendered)
    }
}

impl Default for ConfigTemplateRegistry {
    fn default() -> Self {
        Self::new().expect("Failed to initialize config template registry")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Provider;

    #[test]
    fn test_template_registry_creation() {
        let registry = ConfigTemplateRegistry::new().unwrap();
        assert!(registry.tera.get_template_names().count() > 0);
    }

    #[test]
    fn test_render_config_docker() {
        let registry = ConfigTemplateRegistry::new().unwrap();
        let context = ConfigInitContext::new("test-project", Provider::Docker, "minimal");

        let result = registry.render_config(&context);
        if let Err(ref e) = result {
            eprintln!("Template error: {}", e);
        }
        assert!(result.is_ok(), "Template error: {:?}", result.err());
        let content = result.unwrap();
        assert!(content.contains("name: test-project"));
        assert!(content.contains("provider: docker"));
        assert!(content.contains("profile: minimal"));
    }

    #[test]
    fn test_render_config_fly() {
        let registry = ConfigTemplateRegistry::new().unwrap();
        let context = ConfigInitContext::new("my-app", Provider::Fly, "fullstack");

        let result = registry.render_config(&context);
        assert!(result.is_ok());
        let content = result.unwrap();
        assert!(content.contains("name: my-app"));
        assert!(content.contains("provider: fly"));
        assert!(content.contains("profile: fullstack"));
        // Fly-specific content
        assert!(content.contains("region:"));
    }

    #[test]
    fn test_render_config_e2b_no_gpu() {
        let registry = ConfigTemplateRegistry::new().unwrap();
        let context = ConfigInitContext::new("sandbox", Provider::E2b, "minimal");

        let result = registry.render_config(&context);
        assert!(result.is_ok());
        let content = result.unwrap();
        // E2B doesn't support GPU, so GPU section should not appear
        assert!(!content.contains("gpu:") || content.contains("# Note: E2B"));
    }

    #[test]
    fn test_context_profiles_loaded() {
        let context = ConfigInitContext::new("test", Provider::Docker, "minimal");
        assert!(!context.profiles.is_empty());
        assert!(context.profiles.iter().any(|p| p.name == "minimal"));
        assert!(context.profiles.iter().any(|p| p.name == "fullstack"));
    }
}
