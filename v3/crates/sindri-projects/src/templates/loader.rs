//! Template loading from embedded and runtime YAML files.
//!
//! Loads project templates from:
//! - Embedded project-templates.yaml (compiled into binary)
//! - Runtime YAML files (for testing/development)

use super::parser::TemplateConfig;
use anyhow::{Context, Result};
use std::path::Path;

/// Get embedded template YAML
fn get_embedded_yaml() -> Option<&'static str> {
    // Embed the YAML file at compile time
    // Note: Path is relative to the crate's Cargo.toml directory
    const EMBEDDED_YAML: &str = include_str!("../../templates/project-templates.yaml");
    Some(EMBEDDED_YAML)
}

/// Template loader with fallback mechanisms
#[derive(Debug)]
pub struct TemplateLoader {
    config: TemplateConfig,
}

impl TemplateLoader {
    /// Load templates from embedded resources
    pub fn from_embedded() -> Result<Self> {
        let yaml_str =
            get_embedded_yaml().context("Failed to load embedded project-templates.yaml")?;

        let config = TemplateConfig::from_yaml(yaml_str)?;

        Ok(Self { config })
    }

    /// Load templates from a file path (for testing/development)
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let yaml = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read template file: {:?}", path.as_ref()))?;

        let config = TemplateConfig::from_yaml(&yaml)?;

        Ok(Self { config })
    }

    /// Load templates from YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        let config = TemplateConfig::from_yaml(yaml)?;
        Ok(Self { config })
    }

    /// Load templates with fallback: try embedded, then fall back to file if provided
    pub fn load_with_fallback(fallback_path: Option<&Path>) -> Result<Self> {
        // Try embedded first
        if let Ok(loader) = Self::from_embedded() {
            return Ok(loader);
        }

        // Fall back to file if provided
        if let Some(path) = fallback_path {
            return Self::from_file(path);
        }

        anyhow::bail!(
            "Failed to load templates from embedded resources and no fallback path provided"
        )
    }

    /// Get the underlying template configuration
    pub fn config(&self) -> &TemplateConfig {
        &self.config
    }

    /// Get a template by name
    pub fn get_template(&self, name: &str) -> Option<&super::parser::ProjectTemplate> {
        self.config.get_template(name)
    }

    /// Resolve an alias to its canonical template name
    pub fn resolve_alias(&self, input: &str) -> Option<String> {
        self.config.resolve_alias(input)
    }

    /// Get all available template types
    pub fn template_types(&self) -> Vec<String> {
        self.config.template_types()
    }

    /// Get template description
    pub fn get_description(&self, template_type: &str) -> Option<String> {
        self.config.get_description(template_type)
    }
}

impl Default for TemplateLoader {
    fn default() -> Self {
        Self::from_embedded().expect("Failed to load embedded templates")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_yaml() {
        let yaml = r#"
version: "2.0"
templates:
  node:
    description: "Node.js application"
    aliases: ["nodejs"]
    detection_patterns: ["node"]
    files:
      "package.json": "{}"
"#;

        let loader = TemplateLoader::from_yaml(yaml).unwrap();
        assert!(loader.get_template("node").is_some());
        assert_eq!(loader.template_types(), vec!["node"]);
    }

    #[test]
    fn test_resolve_alias() {
        let yaml = r#"
version: "2.0"
templates:
  python:
    description: "Python"
    aliases: ["py"]
"#;

        let loader = TemplateLoader::from_yaml(yaml).unwrap();
        assert_eq!(loader.resolve_alias("py"), Some("python".to_string()));
        assert_eq!(loader.resolve_alias("python"), Some("python".to_string()));
    }

    #[test]
    fn test_get_description() {
        let yaml = r#"
version: "2.0"
templates:
  rust:
    description: "Rust application"
"#;

        let loader = TemplateLoader::from_yaml(yaml).unwrap();
        assert_eq!(
            loader.get_description("rust"),
            Some("Rust application".to_string())
        );
        assert_eq!(loader.get_description("unknown"), None);
    }
}
