// Configure processing module for Sindri V3 extensions
//
// This module handles the `configure` section in extension.yaml files,
// processing templates and environment variables to configure the system.

mod environment;
mod path;
mod templates;

pub use environment::{EnvironmentProcessor, EnvironmentResult};
pub use path::PathResolver;
pub use templates::{FileType, TemplateProcessor, TemplateResult};

use anyhow::{Context, Result};
use sindri_core::types::ConfigureConfig;
use std::path::PathBuf;

/// Result of executing the configure phase
#[derive(Debug)]
pub struct ConfigureResult {
    pub templates_processed: usize,
    pub environment_vars_set: usize,
    pub backups_created: Vec<PathBuf>,
}

/// Main orchestrator for configure processing
pub struct ConfigureProcessor {
    extension_dir: PathBuf,
    workspace_dir: PathBuf,
    home_dir: PathBuf,
}

impl ConfigureProcessor {
    /// Create a new ConfigureProcessor
    pub fn new(extension_dir: PathBuf, workspace_dir: PathBuf, home_dir: PathBuf) -> Self {
        Self {
            extension_dir,
            workspace_dir,
            home_dir,
        }
    }

    /// Execute the configure phase for an extension
    pub async fn execute(
        &self,
        extension_name: &str,
        config: &ConfigureConfig,
    ) -> Result<ConfigureResult> {
        tracing::info!("Starting configure phase for extension: {}", extension_name);

        let mut templates_processed = 0;
        let mut environment_vars_set = 0;
        let mut backups_created = Vec::new();

        // Process templates
        if !config.templates.is_empty() {
            let template_results = self
                .process_templates(extension_name, &config.templates)
                .await
                .context("Failed to process templates")?;

            templates_processed = template_results.len();
            for result in template_results {
                if let Some(backup) = result.backup_path {
                    backups_created.push(backup);
                }
            }
        }

        // Process environment variables
        if !config.environment.is_empty() {
            let env_results = self
                .process_environment(&config.environment)
                .await
                .context("Failed to process environment variables")?;

            environment_vars_set = env_results.len();
        }

        tracing::info!(
            "Configure phase completed: {} templates, {} env vars",
            templates_processed,
            environment_vars_set
        );

        Ok(ConfigureResult {
            templates_processed,
            environment_vars_set,
            backups_created,
        })
    }

    /// Process all templates
    async fn process_templates(
        &self,
        extension_name: &str,
        templates: &[sindri_core::types::TemplateConfig],
    ) -> Result<Vec<TemplateResult>> {
        let template_processor = TemplateProcessor::new(
            self.extension_dir.clone(),
            self.home_dir.clone(),
        );

        let mut results = Vec::new();
        for template in templates {
            let result = template_processor
                .process_template(extension_name, template)
                .await
                .with_context(|| {
                    format!(
                        "Failed to process template: {} -> {}",
                        template.source, template.destination
                    )
                })?;
            results.push(result);
        }

        Ok(results)
    }

    /// Process all environment variables
    async fn process_environment(
        &self,
        environment: &[sindri_core::types::EnvironmentConfig],
    ) -> Result<Vec<EnvironmentResult>> {
        let env_processor = EnvironmentProcessor::new(self.home_dir.clone());

        let mut results = Vec::new();
        for env_var in environment {
            let result = env_processor
                .set_variable(env_var)
                .await
                .with_context(|| {
                    format!("Failed to set environment variable: {}", env_var.key)
                })?;
            results.push(result);
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_configure_processor_creation() {
        let temp = TempDir::new().unwrap();
        let processor = ConfigureProcessor::new(
            temp.path().to_path_buf(),
            temp.path().to_path_buf(),
            temp.path().to_path_buf(),
        );

        assert_eq!(processor.extension_dir, temp.path());
        assert_eq!(processor.workspace_dir, temp.path());
        assert_eq!(processor.home_dir, temp.path());
    }
}
