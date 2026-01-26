// Environment variable handling for configure phase

use anyhow::{Context, Result};
use sindri_core::types::{EnvironmentConfig, EnvironmentScope};
use std::path::PathBuf;
use tokio::fs as async_fs;

/// Result of setting an environment variable
#[derive(Debug)]
pub struct EnvironmentResult {
    pub key: String,
    pub value: String,
    pub scope: EnvironmentScope,
}

/// Processes environment variables for extensions
pub struct EnvironmentProcessor {
    home_dir: PathBuf,
}

impl EnvironmentProcessor {
    /// Create a new EnvironmentProcessor
    pub fn new(home_dir: PathBuf) -> Self {
        Self { home_dir }
    }

    /// Set an environment variable according to its scope
    pub async fn set_variable(&self, var: &EnvironmentConfig) -> Result<EnvironmentResult> {
        tracing::debug!(
            "Setting environment variable: {} = {} (scope: {:?})",
            var.key,
            var.value,
            var.scope
        );

        match &var.scope {
            EnvironmentScope::Bashrc => {
                self.set_bashrc(&var.key, &var.value).await?;
            }
            EnvironmentScope::Profile => {
                self.set_profile(&var.key, &var.value).await?;
            }
            EnvironmentScope::Session => {
                self.set_session(&var.key, &var.value)?;
            }
        }

        Ok(EnvironmentResult {
            key: var.key.clone(),
            value: var.value.clone(),
            scope: var.scope.clone(),
        })
    }

    /// Set variable in .bashrc file
    async fn set_bashrc(&self, key: &str, value: &str) -> Result<()> {
        let bashrc_path = self.home_dir.join(".bashrc");

        // Check if variable already exists
        if bashrc_path.exists() {
            let content = async_fs::read_to_string(&bashrc_path)
                .await
                .context("Failed to read .bashrc")?;

            if self.check_bashrc_duplicate(&content, key) {
                tracing::info!("Variable {} already set in .bashrc, updating", key);
                self.update_or_append_bashrc(&bashrc_path, &content, key, value)
                    .await?;
                return Ok(());
            }
        }

        // Append new variable
        let export_line = format!("export {}=\"{}\"", key, value);
        self.append_to_bashrc(&bashrc_path, &export_line).await?;

        tracing::info!("Added {} to .bashrc", key);

        Ok(())
    }

    /// Set variable in .profile or .bash_profile
    async fn set_profile(&self, key: &str, value: &str) -> Result<()> {
        // Try .bash_profile first (macOS), then .profile (Linux)
        let profile_paths = [
            self.home_dir.join(".bash_profile"),
            self.home_dir.join(".profile"),
        ];

        let profile_path = profile_paths
            .iter()
            .find(|p| p.exists())
            .cloned()
            .unwrap_or_else(|| self.home_dir.join(".profile"));

        // Check if variable already exists
        if profile_path.exists() {
            let content = async_fs::read_to_string(&profile_path)
                .await
                .context("Failed to read profile file")?;

            if self.check_bashrc_duplicate(&content, key) {
                tracing::info!("Variable {} already set in profile, updating", key);
                self.update_or_append_bashrc(&profile_path, &content, key, value)
                    .await?;
                return Ok(());
            }
        }

        // Append new variable
        let export_line = format!("export {}=\"{}\"", key, value);
        self.append_to_bashrc(&profile_path, &export_line).await?;

        tracing::info!("Added {} to {:?}", key, profile_path);

        Ok(())
    }

    /// Set variable in current session only
    fn set_session(&self, key: &str, value: &str) -> Result<()> {
        std::env::set_var(key, value);

        tracing::info!("Set {} in current session", key);

        Ok(())
    }

    /// Append content to .bashrc file
    async fn append_to_bashrc(&self, path: &PathBuf, content: &str) -> Result<()> {
        let mut current_content = if path.exists() {
            async_fs::read_to_string(path)
                .await
                .context("Failed to read existing file")?
        } else {
            String::new()
        };

        // Ensure trailing newline before appending
        if !current_content.is_empty() && !current_content.ends_with('\n') {
            current_content.push('\n');
        }

        current_content.push_str(content);
        current_content.push('\n');

        async_fs::write(path, current_content)
            .await
            .context("Failed to write to file")?;

        Ok(())
    }

    /// Check if a variable is already set in the file content
    fn check_bashrc_duplicate(&self, content: &str, key: &str) -> bool {
        // Look for export statements with this key
        let patterns = [
            format!("export {}=", key),
            format!("export {}=\"", key),
            format!("export {}='", key),
        ];

        patterns.iter().any(|pattern| content.contains(pattern))
    }

    /// Update existing variable or append new one
    async fn update_or_append_bashrc(
        &self,
        path: &PathBuf,
        content: &str,
        key: &str,
        value: &str,
    ) -> Result<()> {
        // Find and replace existing export line
        let mut updated = false;
        let new_line = format!("export {}=\"{}\"", key, value);

        let updated_content: Vec<String> = content
            .lines()
            .map(|line| {
                if !updated && line.contains(&format!("export {}", key)) {
                    updated = true;
                    new_line.clone()
                } else {
                    line.to_string()
                }
            })
            .collect();

        let mut final_content = updated_content.join("\n");
        if !final_content.ends_with('\n') {
            final_content.push('\n');
        }

        async_fs::write(path, final_content)
            .await
            .context("Failed to write updated content")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_set_session() {
        let temp = TempDir::new().unwrap();
        let processor = EnvironmentProcessor::new(temp.path().to_path_buf());

        processor.set_session("TEST_VAR", "test_value").unwrap();

        assert_eq!(std::env::var("TEST_VAR").unwrap(), "test_value");
    }

    #[tokio::test]
    async fn test_set_bashrc_new_variable() {
        let temp = TempDir::new().unwrap();
        let processor = EnvironmentProcessor::new(temp.path().to_path_buf());

        processor.set_bashrc("NEW_VAR", "new_value").await.unwrap();

        let bashrc_path = temp.path().join(".bashrc");
        assert!(bashrc_path.exists());

        let content = async_fs::read_to_string(&bashrc_path).await.unwrap();
        assert!(content.contains("export NEW_VAR=\"new_value\""));
    }

    #[tokio::test]
    async fn test_set_bashrc_update_existing() {
        let temp = TempDir::new().unwrap();
        let processor = EnvironmentProcessor::new(temp.path().to_path_buf());

        // Set initial value
        processor.set_bashrc("UPDATE_VAR", "initial").await.unwrap();

        // Update value
        processor.set_bashrc("UPDATE_VAR", "updated").await.unwrap();

        let bashrc_path = temp.path().join(".bashrc");
        let content = async_fs::read_to_string(&bashrc_path).await.unwrap();

        // Should only contain the updated value, not the initial
        assert!(content.contains("export UPDATE_VAR=\"updated\""));
        assert!(!content.contains("export UPDATE_VAR=\"initial\""));

        // Should only have one occurrence
        assert_eq!(content.matches("export UPDATE_VAR=").count(), 1);
    }

    #[tokio::test]
    async fn test_check_bashrc_duplicate() {
        let temp = TempDir::new().unwrap();
        let processor = EnvironmentProcessor::new(temp.path().to_path_buf());

        let content = r#"
export PATH="/usr/local/bin:$PATH"
export MY_VAR="value"
export ANOTHER_VAR='value2'
"#;

        assert!(processor.check_bashrc_duplicate(content, "PATH"));
        assert!(processor.check_bashrc_duplicate(content, "MY_VAR"));
        assert!(processor.check_bashrc_duplicate(content, "ANOTHER_VAR"));
        assert!(!processor.check_bashrc_duplicate(content, "NONEXISTENT"));
    }

    #[tokio::test]
    async fn test_append_to_bashrc() {
        let temp = TempDir::new().unwrap();
        let processor = EnvironmentProcessor::new(temp.path().to_path_buf());
        let bashrc_path = temp.path().join(".bashrc");

        // First append
        processor
            .append_to_bashrc(&bashrc_path, "export VAR1=\"value1\"")
            .await
            .unwrap();

        // Second append
        processor
            .append_to_bashrc(&bashrc_path, "export VAR2=\"value2\"")
            .await
            .unwrap();

        let content = async_fs::read_to_string(&bashrc_path).await.unwrap();
        assert!(content.contains("export VAR1=\"value1\""));
        assert!(content.contains("export VAR2=\"value2\""));
    }

    #[tokio::test]
    async fn test_set_profile() {
        let temp = TempDir::new().unwrap();
        let processor = EnvironmentProcessor::new(temp.path().to_path_buf());

        processor
            .set_profile("PROFILE_VAR", "profile_value")
            .await
            .unwrap();

        // Should create .profile (since .bash_profile doesn't exist)
        let profile_path = temp.path().join(".profile");
        assert!(profile_path.exists());

        let content = async_fs::read_to_string(&profile_path).await.unwrap();
        assert!(content.contains("export PROFILE_VAR=\"profile_value\""));
    }
}
