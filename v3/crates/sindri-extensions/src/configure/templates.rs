// Template processing for configure phase

use super::path::PathResolver;
use anyhow::{bail, Context, Result};
use chrono::Local;
use sindri_core::types::{TemplateConfig, TemplateMode};
use std::fs;
use std::path::{Path, PathBuf};
use tokio::fs as async_fs;

/// Result of processing a template
#[derive(Debug)]
pub struct TemplateResult {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub backup_path: Option<PathBuf>,
    pub mode: TemplateMode,
}

/// File type detection for merge strategies
#[derive(Debug, PartialEq)]
pub enum FileType {
    Yaml,
    Json,
    Shell,
    Other,
}

/// Processes templates from extensions
pub struct TemplateProcessor {
    extension_dir: PathBuf,
    home_dir: PathBuf,
}

impl TemplateProcessor {
    /// Create a new TemplateProcessor
    pub fn new(extension_dir: PathBuf, home_dir: PathBuf) -> Self {
        Self {
            extension_dir,
            home_dir,
        }
    }

    /// Process a single template
    pub async fn process_template(
        &self,
        extension_name: &str,
        template: &TemplateConfig,
    ) -> Result<TemplateResult> {
        let resolver = PathResolver::new(self.extension_dir.clone(), self.home_dir.clone());

        // Resolve source and destination paths
        let source = resolver
            .resolve_source(extension_name, &template.source)
            .await?;
        let destination = resolver.resolve_destination(&template.destination).await?;

        tracing::debug!(
            "Processing template: {:?} -> {:?} (mode: {:?})",
            source,
            destination,
            template.mode
        );

        // Ensure parent directory exists
        if let Some(parent) = destination.parent() {
            async_fs::create_dir_all(parent)
                .await
                .context("Failed to create parent directory")?;
        }

        let backup_path = match &template.mode {
            TemplateMode::Overwrite => self.apply_overwrite(&source, &destination).await?,
            TemplateMode::Append => self.apply_append(&source, &destination).await?,
            TemplateMode::SkipIfExists => self.apply_skip_if_exists(&source, &destination).await?,
            TemplateMode::Merge => {
                self.apply_merge(&source, &destination, extension_name)
                    .await?
            }
        };

        Ok(TemplateResult {
            source,
            destination,
            backup_path,
            mode: template.mode,
        })
    }

    /// Apply overwrite mode: backup existing file and copy template
    async fn apply_overwrite(&self, source: &Path, dest: &Path) -> Result<Option<PathBuf>> {
        let backup = if dest.exists() {
            Some(self.create_backup(dest).await?)
        } else {
            None
        };

        // Atomic write: write to temp file, then rename
        let temp_path = dest.with_extension("tmp");
        async_fs::copy(source, &temp_path)
            .await
            .context("Failed to copy template to temp file")?;

        // Preserve permissions if original file existed
        if let Some(backup_path) = &backup {
            if let Ok(metadata) = fs::metadata(backup_path) {
                let permissions = metadata.permissions();
                fs::set_permissions(&temp_path, permissions)
                    .context("Failed to preserve file permissions")?;
            }
        }

        // Atomic rename
        async_fs::rename(&temp_path, dest)
            .await
            .context("Failed to rename temp file to destination")?;

        tracing::info!("Overwrote file: {:?}", dest);

        Ok(backup)
    }

    /// Apply append mode: append template content to file
    async fn apply_append(&self, source: &Path, dest: &Path) -> Result<Option<PathBuf>> {
        let content = async_fs::read_to_string(source)
            .await
            .context("Failed to read template file")?;

        let backup = if dest.exists() {
            Some(self.create_backup(dest).await?)
        } else {
            None
        };

        // Check for duplicates in .bashrc-like files
        if dest.exists() && self.is_shell_config(dest) {
            let existing = async_fs::read_to_string(dest)
                .await
                .context("Failed to read existing file")?;

            // Simple duplicate check: if content is already present, skip
            if existing.contains(content.trim()) {
                tracing::info!("Content already present in {:?}, skipping append", dest);
                return Ok(backup);
            }
        }

        // Append content
        let mut final_content = if dest.exists() {
            let existing = async_fs::read_to_string(dest)
                .await
                .context("Failed to read existing file")?;
            format!("{}\n{}", existing, content)
        } else {
            content
        };

        // Ensure trailing newline
        if !final_content.ends_with('\n') {
            final_content.push('\n');
        }

        async_fs::write(dest, final_content)
            .await
            .context("Failed to write appended content")?;

        tracing::info!("Appended to file: {:?}", dest);

        Ok(backup)
    }

    /// Apply skip-if-exists mode: copy only if destination doesn't exist
    async fn apply_skip_if_exists(&self, source: &Path, dest: &Path) -> Result<Option<PathBuf>> {
        if dest.exists() {
            tracing::info!("File exists, skipping: {:?}", dest);
            return Ok(None);
        }

        async_fs::copy(source, dest)
            .await
            .context("Failed to copy template")?;

        tracing::info!("Copied template (skip-if-exists): {:?}", dest);

        Ok(None)
    }

    /// Apply merge mode: intelligently merge template with existing file
    async fn apply_merge(
        &self,
        source: &Path,
        dest: &Path,
        extension_name: &str,
    ) -> Result<Option<PathBuf>> {
        let file_type = self.detect_file_type(dest);

        let backup = if dest.exists() {
            Some(self.create_backup(dest).await?)
        } else {
            None
        };

        match file_type {
            FileType::Yaml => self.merge_yaml(source, dest).await?,
            FileType::Json => self.merge_json(source, dest).await?,
            FileType::Shell => self.merge_shell(source, dest, extension_name).await?,
            FileType::Other => {
                // For unknown file types, fall back to marker-based merge
                self.merge_shell(source, dest, extension_name).await?
            }
        }

        tracing::info!("Merged template: {:?}", dest);

        Ok(backup)
    }

    /// Merge YAML files (deep merge)
    async fn merge_yaml(&self, source: &Path, dest: &Path) -> Result<()> {
        let source_content = async_fs::read_to_string(source)
            .await
            .context("Failed to read source YAML")?;
        let source_yaml: serde_yaml::Value =
            serde_yaml::from_str(&source_content).context("Failed to parse source YAML")?;

        let merged = if dest.exists() {
            let dest_content = async_fs::read_to_string(dest)
                .await
                .context("Failed to read destination YAML")?;
            let mut dest_yaml: serde_yaml::Value =
                serde_yaml::from_str(&dest_content).context("Failed to parse destination YAML")?;

            // Deep merge: source values take precedence
            merge_yaml_values(&mut dest_yaml, source_yaml);
            dest_yaml
        } else {
            source_yaml
        };

        let merged_content =
            serde_yaml::to_string(&merged).context("Failed to serialize merged YAML")?;

        async_fs::write(dest, merged_content)
            .await
            .context("Failed to write merged YAML")?;

        Ok(())
    }

    /// Merge JSON files (deep merge)
    async fn merge_json(&self, source: &Path, dest: &Path) -> Result<()> {
        let source_content = async_fs::read_to_string(source)
            .await
            .context("Failed to read source JSON")?;
        let source_json: serde_json::Value =
            serde_json::from_str(&source_content).context("Failed to parse source JSON")?;

        let merged = if dest.exists() {
            let dest_content = async_fs::read_to_string(dest)
                .await
                .context("Failed to read destination JSON")?;
            let mut dest_json: serde_json::Value =
                serde_json::from_str(&dest_content).context("Failed to parse destination JSON")?;

            // Deep merge: source values take precedence
            merge_json_values(&mut dest_json, source_json);
            dest_json
        } else {
            source_json
        };

        let merged_content =
            serde_json::to_string_pretty(&merged).context("Failed to serialize merged JSON")?;

        async_fs::write(dest, merged_content)
            .await
            .context("Failed to write merged JSON")?;

        Ok(())
    }

    /// Merge shell config files using markers
    async fn merge_shell(&self, source: &Path, dest: &Path, extension_name: &str) -> Result<()> {
        let source_content = async_fs::read_to_string(source)
            .await
            .context("Failed to read source file")?;

        let marker_begin = format!("# sindri-{} BEGIN", extension_name);
        let marker_end = format!("# sindri-{} END", extension_name);

        let new_section = format!(
            "{}\n{}\n{}",
            marker_begin,
            source_content.trim(),
            marker_end
        );

        let merged = if dest.exists() {
            let dest_content = async_fs::read_to_string(dest)
                .await
                .context("Failed to read destination file")?;

            // Check if marker section already exists
            if dest_content.contains(&marker_begin) {
                // Replace existing section
                replace_marker_section(&dest_content, &marker_begin, &marker_end, &new_section)?
            } else {
                // Append new section
                format!("{}\n\n{}", dest_content.trim_end(), new_section)
            }
        } else {
            new_section
        };

        async_fs::write(dest, merged)
            .await
            .context("Failed to write merged file")?;

        Ok(())
    }

    /// Create a backup of a file
    async fn create_backup(&self, path: &Path) -> Result<PathBuf> {
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let backup_path = path.with_extension(format!("backup_{}", timestamp));

        async_fs::copy(path, &backup_path)
            .await
            .context("Failed to create backup")?;

        tracing::debug!("Created backup: {:?}", backup_path);

        Ok(backup_path)
    }

    /// Detect file type based on extension
    fn detect_file_type(&self, path: &Path) -> FileType {
        match path.extension().and_then(|e| e.to_str()) {
            Some("yaml") | Some("yml") => FileType::Yaml,
            Some("json") => FileType::Json,
            Some("sh") | Some("bash") | Some("zsh") => FileType::Shell,
            _ => {
                // Check common shell config files without extensions
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with('.') && (name.contains("rc") || name.contains("profile")) {
                        return FileType::Shell;
                    }
                }
                FileType::Other
            }
        }
    }

    /// Check if a file is a shell configuration file
    fn is_shell_config(&self, path: &Path) -> bool {
        matches!(self.detect_file_type(path), FileType::Shell)
    }
}

/// Deep merge YAML values (source takes precedence)
fn merge_yaml_values(dest: &mut serde_yaml::Value, source: serde_yaml::Value) {
    use serde_yaml::Value;

    match (dest, source) {
        (Value::Mapping(dest_map), Value::Mapping(source_map)) => {
            for (key, value) in source_map {
                dest_map
                    .entry(key)
                    .and_modify(|dest_value| merge_yaml_values(dest_value, value.clone()))
                    .or_insert(value);
            }
        }
        (dest_value, source_value) => {
            *dest_value = source_value;
        }
    }
}

/// Deep merge JSON values (source takes precedence)
fn merge_json_values(dest: &mut serde_json::Value, source: serde_json::Value) {
    use serde_json::Value;

    match (dest, source) {
        (Value::Object(dest_map), Value::Object(source_map)) => {
            for (key, value) in source_map {
                dest_map
                    .entry(key)
                    .and_modify(|dest_value| merge_json_values(dest_value, value.clone()))
                    .or_insert(value);
            }
        }
        (dest_value, source_value) => {
            *dest_value = source_value;
        }
    }
}

/// Replace a marker section in a file
fn replace_marker_section(
    content: &str,
    marker_begin: &str,
    marker_end: &str,
    new_section: &str,
) -> Result<String> {
    // Find the section between markers
    if let Some(begin_pos) = content.find(marker_begin) {
        if let Some(end_pos) = content[begin_pos..].find(marker_end) {
            let end_pos = begin_pos + end_pos + marker_end.len();
            let before = &content[..begin_pos];
            let after = &content[end_pos..];
            return Ok(format!("{}{}{}", before.trim_end(), new_section, after));
        }
    }

    bail!("Marker section not found properly");
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_detect_file_type() {
        let temp = TempDir::new().unwrap();
        let processor =
            TemplateProcessor::new(temp.path().to_path_buf(), temp.path().to_path_buf());

        assert_eq!(
            processor.detect_file_type(Path::new("config.yaml")),
            FileType::Yaml
        );
        assert_eq!(
            processor.detect_file_type(Path::new("data.json")),
            FileType::Json
        );
        assert_eq!(
            processor.detect_file_type(Path::new("script.sh")),
            FileType::Shell
        );
        assert_eq!(
            processor.detect_file_type(Path::new(".bashrc")),
            FileType::Shell
        );
        assert_eq!(
            processor.detect_file_type(Path::new("readme.txt")),
            FileType::Other
        );
    }

    #[tokio::test]
    async fn test_create_backup() {
        let temp = TempDir::new().unwrap();
        let processor =
            TemplateProcessor::new(temp.path().to_path_buf(), temp.path().to_path_buf());

        let test_file = temp.path().join("test.txt");
        async_fs::write(&test_file, "original content")
            .await
            .unwrap();

        let backup = processor.create_backup(&test_file).await.unwrap();

        assert!(backup.exists());
        let backup_content = async_fs::read_to_string(&backup).await.unwrap();
        assert_eq!(backup_content, "original content");
    }

    #[tokio::test]
    async fn test_merge_yaml_values() {
        let mut dest: serde_yaml::Value = serde_yaml::from_str(
            r#"
            key1: value1
            nested:
              a: 1
              b: 2
            "#,
        )
        .unwrap();

        let source: serde_yaml::Value = serde_yaml::from_str(
            r#"
            key2: value2
            nested:
              b: 3
              c: 4
            "#,
        )
        .unwrap();

        merge_yaml_values(&mut dest, source);

        let result = serde_yaml::to_string(&dest).unwrap();
        assert!(result.contains("key1"));
        assert!(result.contains("key2"));
        assert!(result.contains("c: 4"));
    }

    #[test]
    fn test_replace_marker_section() {
        let content = r#"
# Some content
# sindri-test BEGIN
old content
# sindri-test END
# More content
"#;

        let new_section = "# sindri-test BEGIN\nnew content\n# sindri-test END";
        let result = replace_marker_section(
            content,
            "# sindri-test BEGIN",
            "# sindri-test END",
            new_section,
        )
        .unwrap();

        assert!(result.contains("new content"));
        assert!(!result.contains("old content"));
    }
}
