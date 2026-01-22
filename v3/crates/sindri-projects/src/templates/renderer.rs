//! Template rendering with Tera for variable substitution.
//!
//! Handles:
//! - Variable substitution ({project_name}, {author}, {date}, etc.)
//! - Multi-file creation from template definitions
//! - CLAUDE.md generation with project-specific content

use super::parser::ProjectTemplate;
use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use chrono::Local;
use serde::{Deserialize, Serialize};

/// Template variables for rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVars {
    pub project_name: String,
    pub author: String,
    pub git_user_name: String,
    pub git_user_email: String,
    pub date: String,
    pub year: String,
    pub description: String,
    pub license: String,
}

impl TemplateVars {
    /// Create template variables with defaults
    pub fn new(project_name: String) -> Self {
        let now = Local::now();
        Self {
            project_name,
            author: String::new(),
            git_user_name: String::new(),
            git_user_email: String::new(),
            date: now.format("%Y-%m-%d").to_string(),
            year: now.format("%Y").to_string(),
            description: "Project description".to_string(),
            license: "MIT".to_string(),
        }
    }

    /// Set git user information
    pub fn with_git_user(mut self, name: String, email: String) -> Self {
        self.author = name.clone();
        self.git_user_name = name;
        self.git_user_email = email;
        self
    }

    /// Set author
    pub fn with_author(mut self, author: String) -> Self {
        self.author = author.clone();
        if self.git_user_name.is_empty() {
            self.git_user_name = author;
        }
        self
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    /// Set license
    pub fn with_license(mut self, license: String) -> Self {
        self.license = license;
        self
    }

}

/// Template renderer
#[derive(Debug)]
pub struct TemplateRenderer;

impl TemplateRenderer {
    /// Create a new renderer
    pub fn new() -> Self {
        Self
    }

    /// Render a string template with variables
    ///
    /// Note: This uses simple string replacement for {var} syntax (not Tera {{ var }})
    /// to match the bash implementation's behavior.
    pub fn render_string(&self, template: &str, vars: &TemplateVars) -> Result<String> {
        let mut result = template.to_string();

        // Simple variable replacement using {var} syntax
        result = result.replace("{project_name}", &vars.project_name);
        result = result.replace("{author}", &vars.author);
        result = result.replace("{git_user_name}", &vars.git_user_name);
        result = result.replace("{git_user_email}", &vars.git_user_email);
        result = result.replace("{date}", &vars.date);
        result = result.replace("{year}", &vars.year);
        result = result.replace("{description}", &vars.description);
        result = result.replace("{license}", &vars.license);

        Ok(result)
    }

    /// Render template files to a target directory
    pub fn render_files(
        &self,
        template: &ProjectTemplate,
        vars: &TemplateVars,
        target_dir: &Utf8Path,
    ) -> Result<Vec<Utf8PathBuf>> {
        let mut created_files = Vec::new();

        for (file_path_template, content_template) in &template.files {
            // Render file path (in case it contains variables)
            let file_path = self.render_string(file_path_template, vars)?;
            let full_path = target_dir.join(&file_path);

            // Create parent directory if needed
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create directory: {}", parent))?;
            }

            // Render content
            let content = self.render_string(content_template, vars)?;

            // Write file
            std::fs::write(&full_path, content)
                .with_context(|| format!("Failed to write file: {}", full_path))?;

            created_files.push(full_path);
        }

        Ok(created_files)
    }

    /// Generate CLAUDE.md from template
    pub fn render_claude_md(
        &self,
        template: &ProjectTemplate,
        vars: &TemplateVars,
        target_dir: &Utf8Path,
    ) -> Result<Utf8PathBuf> {
        let claude_md_path = target_dir.join("CLAUDE.md");

        let content = if let Some(ref claude_template) = template.claude_md_template {
            // Use template-specific CLAUDE.md
            self.render_string(claude_template, vars)?
        } else {
            // Generate default CLAUDE.md
            self.generate_default_claude_md(vars)
        };

        std::fs::write(&claude_md_path, content)
            .context("Failed to write CLAUDE.md")?;

        Ok(claude_md_path)
    }

    /// Generate default CLAUDE.md if template doesn't provide one
    fn generate_default_claude_md(&self, vars: &TemplateVars) -> String {
        format!(
            r#"# {project_name}

## Project Overview
{description}

## Setup Instructions
[Add setup instructions]

## Development Commands
[Add development commands]

## Architecture Notes
[Add architectural decisions and patterns]

---
Created: {date}
Author: {author}
"#,
            project_name = vars.project_name,
            description = vars.description,
            date = vars.date,
            author = if vars.author.is_empty() {
                "Unknown"
            } else {
                &vars.author
            }
        )
    }
}

impl Default for TemplateRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_template_vars_new() {
        let vars = TemplateVars::new("test-project".to_string());
        assert_eq!(vars.project_name, "test-project");
        assert_eq!(vars.license, "MIT");
        assert!(!vars.year.is_empty());
        assert!(!vars.date.is_empty());
    }

    #[test]
    fn test_template_vars_builder() {
        let vars = TemplateVars::new("test-project".to_string())
            .with_author("John Doe".to_string())
            .with_description("A test project".to_string())
            .with_license("Apache-2.0".to_string());

        assert_eq!(vars.author, "John Doe");
        assert_eq!(vars.description, "A test project");
        assert_eq!(vars.license, "Apache-2.0");
    }

    #[test]
    fn test_render_string() {
        let renderer = TemplateRenderer::new();
        let vars = TemplateVars::new("my-app".to_string())
            .with_author("Jane Doe".to_string());

        let template = "Project: {project_name}\nAuthor: {author}\nYear: {year}";
        let result = renderer.render_string(template, &vars).unwrap();

        assert!(result.contains("Project: my-app"));
        assert!(result.contains("Author: Jane Doe"));
        assert!(result.contains("Year:"));
    }

    #[test]
    fn test_render_package_json() {
        let renderer = TemplateRenderer::new();
        let vars = TemplateVars::new("my-node-app".to_string());

        let template = r#"{
  "name": "{project_name}",
  "version": "1.0.0",
  "author": "{author}"
}"#;

        let result = renderer.render_string(template, &vars).unwrap();
        assert!(result.contains(r#""name": "my-node-app""#));
        assert!(result.contains(r#""version": "1.0.0""#));
    }

    #[test]
    fn test_generate_default_claude_md() {
        let renderer = TemplateRenderer::new();
        let vars = TemplateVars::new("test-project".to_string())
            .with_author("Alice".to_string())
            .with_description("A test project".to_string());

        let result = renderer.generate_default_claude_md(&vars);

        assert!(result.contains("# test-project"));
        assert!(result.contains("A test project"));
        assert!(result.contains("Author: Alice"));
    }

    #[test]
    fn test_render_claude_md_with_template() {
        let renderer = TemplateRenderer::new();
        let vars = TemplateVars::new("my-rails-app".to_string());

        let mut template = ProjectTemplate {
            description: "Rails app".to_string(),
            aliases: vec![],
            extensions: vec![],
            detection_patterns: vec![],
            setup_commands: vec![],
            dependencies: None,
            files: HashMap::new(),
            claude_md_template: Some(
                r#"# {project_name}

## Rails Project
Built with Ruby on Rails.

Author: {author}
"#
                .to_string(),
            ),
        };

        let rendered = renderer
            .render_string(
                template.claude_md_template.as_ref().unwrap(),
                &vars,
            )
            .unwrap();

        assert!(rendered.contains("# my-rails-app"));
        assert!(rendered.contains("## Rails Project"));
    }

    #[test]
    fn test_render_files() {
        use tempfile::tempdir;

        let renderer = TemplateRenderer::new();
        let vars = TemplateVars::new("test-app".to_string());

        let mut files = HashMap::new();
        files.insert(
            "README.md".to_string(),
            "# {project_name}\n\nCreated: {date}".to_string(),
        );
        files.insert(
            "src/main.rs".to_string(),
            "// {project_name}\nfn main() {{\n    println!(\"Hello\");\n}}".to_string(),
        );

        let template = ProjectTemplate {
            description: "Test".to_string(),
            aliases: vec![],
            extensions: vec![],
            detection_patterns: vec![],
            setup_commands: vec![],
            dependencies: None,
            files,
            claude_md_template: None,
        };

        let temp_dir = tempdir().unwrap();
        let target = Utf8PathBuf::try_from(temp_dir.path().to_path_buf()).unwrap();

        let created = renderer.render_files(&template, &vars, &target).unwrap();

        assert_eq!(created.len(), 2);
        assert!(target.join("README.md").exists());
        assert!(target.join("src/main.rs").exists());

        let readme = std::fs::read_to_string(target.join("README.md")).unwrap();
        assert!(readme.contains("# test-app"));
    }
}
