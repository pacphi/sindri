//! Project template system for Sindri CLI.
//!
//! This module provides a complete template system for project scaffolding with:
//! - YAML-based template definitions (embedded and runtime)
//! - Intelligent project type detection from names
//! - Variable substitution and file generation
//! - CLAUDE.md generation for AI-first development
//!
//! # Architecture
//!
//! The template system is organized into focused modules:
//! - `parser`: YAML structure parsing into strongly-typed Rust
//! - `loader`: Load templates from embedded or file sources
//! - `detector`: Auto-detect project types from names
//! - `renderer`: Render template files with variable substitution
//!
//! # Example Usage
//!
//! ```no_run
//! use sindri_projects::templates::{TemplateLoader, TypeDetector, TemplateRenderer, TemplateVars};
//! use sindri_projects::templates::parser::DetectionResult;
//!
//! // Load templates
//! let loader = TemplateLoader::from_embedded().unwrap();
//!
//! // Detect project type
//! let detector = TypeDetector::new(&loader);
//! let detection = detector.detect_from_name("my-rails-app");
//!
//! match detection {
//!     DetectionResult::Single(type_name) => {
//!         println!("Detected type: {}", type_name);
//!
//!         // Get template
//!         let template = loader.get_template(&type_name).unwrap();
//!
//!         // Prepare variables
//!         let vars = TemplateVars::new("my-rails-app".to_string())
//!             .with_author("Alice".to_string());
//!
//!         // Render files
//!         let renderer = TemplateRenderer::new();
//!         // renderer.render_files(template, &vars, target_dir);
//!     }
//!     DetectionResult::Ambiguous(types) => {
//!         println!("Multiple types match: {:?}", types);
//!         // Show user suggestions and prompt for selection
//!     }
//!     DetectionResult::None => {
//!         println!("No match found, using default");
//!     }
//! }
//! ```
//!
//! # Template Definition Format
//!
//! Templates are defined in `project-templates.yaml`:
//!
//! ```yaml
//! version: "2.0"
//! templates:
//!   node:
//!     description: "Node.js application"
//!     aliases: ["nodejs", "javascript"]
//!     extensions: ["nodejs"]
//!     detection_patterns: ["node", "npm", "express"]
//!     setup_commands:
//!       - "npm init -y"
//!     files:
//!       "package.json": |
//!         {
//!           "name": "{project_name}",
//!           "version": "1.0.0"
//!         }
//!     claude_md_template: |
//!       # {project_name}
//!       Node.js application
//! ```
//!
//! # Detection Logic
//!
//! Pattern-based detection from project names:
//! - Exact matches: "my-rails-app" → rails (unambiguous)
//! - Ambiguous matches: "api-server" → [node, go, python] (requires user input)
//! - Alias resolution: "nodejs" → node, "py" → python
//!
//! Detection rules are defined in the YAML:
//!
//! ```yaml
//! detection_rules:
//!   name_patterns:
//!     - pattern: ".*-?rails?-?.*"
//!       type: "rails"
//!     - pattern: ".*-?api.*"
//!       types: ["node", "go", "python"]  # Ambiguous
//! ```

pub mod detector;
pub mod loader;
pub mod parser;
pub mod renderer;

// Re-export main types for convenience
pub use detector::TypeDetector;
pub use loader::TemplateLoader;
pub use parser::{
    DependencyConfig, DetectPattern, DetectionResult, NamePattern, ProjectTemplate,
    TemplateConfig,
};
pub use renderer::{TemplateRenderer, TemplateVars};

use anyhow::{Context, Result};
use camino::Utf8Path;

/// High-level template manager combining all template functionality
#[derive(Debug)]
pub struct TemplateManager {
    loader: TemplateLoader,
    renderer: TemplateRenderer,
}

impl TemplateManager {
    /// Create a new template manager with embedded templates
    pub fn new() -> Result<Self> {
        Ok(Self {
            loader: TemplateLoader::from_embedded()?,
            renderer: TemplateRenderer::new(),
        })
    }

    /// Create a template manager from a file path
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        Ok(Self {
            loader: TemplateLoader::from_file(path)?,
            renderer: TemplateRenderer::new(),
        })
    }

    /// Get the loader
    pub fn loader(&self) -> &TemplateLoader {
        &self.loader
    }

    /// Get the renderer
    pub fn renderer(&self) -> &TemplateRenderer {
        &self.renderer
    }

    /// Create a type detector
    pub fn detector(&self) -> TypeDetector<'_> {
        TypeDetector::new(&self.loader)
    }

    /// Detect project type from name
    pub fn detect_type(&self, project_name: &str) -> DetectionResult {
        self.detector().detect_from_name(project_name)
    }

    /// Resolve an alias to canonical type name
    pub fn resolve_alias(&self, input: &str) -> Option<String> {
        self.loader.resolve_alias(input)
    }

    /// Get a template by type
    pub fn get_template(&self, template_type: &str) -> Option<&ProjectTemplate> {
        self.loader.get_template(template_type)
    }

    /// Get all available template types
    pub fn available_types(&self) -> Vec<String> {
        self.loader.template_types()
    }

    /// Render template files to a target directory
    pub fn render_project(
        &self,
        template_type: &str,
        vars: &TemplateVars,
        target_dir: &Utf8Path,
    ) -> Result<Vec<camino::Utf8PathBuf>> {
        let template = self
            .get_template(template_type)
            .with_context(|| format!("Template not found: {}", template_type))?;

        self.renderer.render_files(template, vars, target_dir)
    }

    /// Generate CLAUDE.md for a project
    pub fn generate_claude_md(
        &self,
        template_type: &str,
        vars: &TemplateVars,
        target_dir: &Utf8Path,
    ) -> Result<camino::Utf8PathBuf> {
        let template = self
            .get_template(template_type)
            .with_context(|| format!("Template not found: {}", template_type))?;

        self.renderer.render_claude_md(template, vars, target_dir)
    }
}

impl Default for TemplateManager {
    fn default() -> Self {
        Self::new().expect("Failed to initialize template manager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_manager_from_yaml() {
        let yaml = r#"
version: "2.0"
templates:
  node:
    description: "Node.js application"
    aliases: ["nodejs"]
    detection_patterns: ["node"]
    files:
      "package.json": '{"name": "{project_name}"}'
detection_rules:
  name_patterns:
    - pattern: ".*node.*"
      type: "node"
"#;

        let loader = TemplateLoader::from_yaml(yaml).unwrap();
        let renderer = TemplateRenderer::new();
        let manager = TemplateManager { loader, renderer };

        assert!(manager.get_template("node").is_some());
        assert_eq!(manager.available_types(), vec!["node"]);
    }

    #[test]
    fn test_detect_type() {
        let yaml = r#"
version: "2.0"
templates:
  rails:
    description: "Rails"
detection_rules:
  name_patterns:
    - pattern: ".*rails.*"
      type: "rails"
"#;

        let loader = TemplateLoader::from_yaml(yaml).unwrap();
        let renderer = TemplateRenderer::new();
        let manager = TemplateManager { loader, renderer };

        let result = manager.detect_type("my-rails-app");
        assert_eq!(result, DetectionResult::Single("rails".to_string()));
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
        let renderer = TemplateRenderer::new();
        let manager = TemplateManager { loader, renderer };

        assert_eq!(
            manager.resolve_alias("py"),
            Some("python".to_string())
        );
    }
}
