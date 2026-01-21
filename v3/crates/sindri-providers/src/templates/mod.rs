//! Template rendering for provider configurations
//!
//! Uses Tera templates to generate provider-specific configuration files
//! (docker-compose.yml, fly.toml, devcontainer.json, etc.)

mod context;
mod docker;

pub use context::TemplateContext;
pub use docker::DockerTemplates;

use anyhow::Result;
use std::path::Path;
use tera::Tera;
use tracing::debug;

/// Template registry holding all provider templates
pub struct TemplateRegistry {
    tera: Tera,
}

impl TemplateRegistry {
    /// Create a new template registry with embedded templates
    pub fn new() -> Result<Self> {
        let mut tera = Tera::default();

        // Register Docker templates
        tera.add_raw_template(
            "docker-compose.yml",
            include_str!("docker-compose.yml.tera"),
        )?;
        tera.add_raw_template(
            "docker-compose.dind.yml",
            include_str!("docker-compose.dind.yml.tera"),
        )?;

        // Register Fly.io templates
        tera.add_raw_template("fly.toml", include_str!("fly.toml.tera"))?;

        // Register DevPod templates
        tera.add_raw_template("devcontainer.json", include_str!("devcontainer.json.tera"))?;

        // Register Kubernetes templates
        tera.add_raw_template(
            "k8s-deployment.yaml",
            include_str!("k8s-deployment.yaml.tera"),
        )?;

        // Register E2B templates
        tera.add_raw_template("e2b.toml", include_str!("e2b.toml.tera"))?;

        Ok(Self { tera })
    }

    /// Render a template with the given context
    pub fn render(&self, template_name: &str, context: &TemplateContext) -> Result<String> {
        debug!("Rendering template: {}", template_name);
        let tera_context = context.to_tera_context()?;
        let rendered = self.tera.render(template_name, &tera_context)?;
        Ok(rendered)
    }

    /// Render a template and write to a file
    pub fn render_to_file(
        &self,
        template_name: &str,
        context: &TemplateContext,
        output_path: &Path,
    ) -> Result<()> {
        let content = self.render(template_name, context)?;
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(output_path, content)?;
        debug!("Wrote template to: {}", output_path.display());
        Ok(())
    }
}

impl Default for TemplateRegistry {
    fn default() -> Self {
        Self::new().expect("Failed to initialize template registry")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_registry_creation() {
        let registry = TemplateRegistry::new().unwrap();
        assert!(registry.tera.get_template_names().count() > 0);
    }

    #[test]
    fn test_docker_template_render() {
        let registry = TemplateRegistry::new().unwrap();
        let context = TemplateContext::builder()
            .name("test-env")
            .profile("base")
            .memory("4GB")
            .cpus(2)
            .build();

        let result = registry.render("docker-compose.yml", &context);
        assert!(result.is_ok());
        let content = result.unwrap();
        assert!(content.contains("test-env"));
        assert!(content.contains("4GB"));
    }
}
