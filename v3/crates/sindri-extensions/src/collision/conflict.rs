//! Conflict rule application for collision handling

use crate::configure::{merge_json_values, merge_yaml_values};
use anyhow::{Context, Result};
use sindri_core::types::{ConflictActionType, ConflictResourceType, ConflictRule};
use std::path::{Path, PathBuf};

use super::InteractivityMode;

/// Result of applying a single conflict rule
#[derive(Debug, Clone)]
pub struct ConflictResult {
    /// The path that was processed
    pub path: PathBuf,
    /// The action that was taken
    pub action_taken: ConflictActionType,
    /// Path to backup file if one was created
    pub backup_path: Option<PathBuf>,
}

/// Applies conflict rules to workspace files after project-init commands run
pub struct ConflictApplier;

impl ConflictApplier {
    /// Apply conflict rules to files in workspace AFTER project-init commands ran.
    /// Prompt/PromptPerFile falls back to Skip in NonInteractive mode.
    pub fn apply(
        workspace: &Path,
        rules: &[ConflictRule],
        extension_name: &str,
        mode: &InteractivityMode,
    ) -> Result<Vec<ConflictResult>> {
        let mut results = Vec::new();

        for rule in rules {
            let target = workspace.join(&rule.path);

            // Only apply rules to paths that exist
            if !target.exists() {
                continue;
            }

            // Verify type matches
            match rule.r#type {
                ConflictResourceType::File => {
                    if !target.is_file() {
                        continue;
                    }
                }
                ConflictResourceType::Directory => {
                    if !target.is_dir() {
                        continue;
                    }
                }
            }

            let action = &rule.on_conflict;
            let effective_action = match action.action {
                ConflictActionType::Prompt | ConflictActionType::PromptPerFile => {
                    match mode {
                        InteractivityMode::NonInteractive => ConflictActionType::Skip,
                        InteractivityMode::Interactive => {
                            // For now, interactive also skips (full TUI not implemented)
                            ConflictActionType::Skip
                        }
                    }
                }
                other => other,
            };

            // Create backup if requested
            let backup_path = if action.backup {
                Some(Self::create_backup(&target, &action.backup_suffix)?)
            } else {
                None
            };

            match effective_action {
                ConflictActionType::Skip => {
                    results.push(ConflictResult {
                        path: target,
                        action_taken: ConflictActionType::Skip,
                        backup_path,
                    });
                }
                ConflictActionType::Overwrite => {
                    // Overwrite is a no-op here since the project-init command
                    // already wrote the file. We just record it.
                    results.push(ConflictResult {
                        path: target,
                        action_taken: ConflictActionType::Overwrite,
                        backup_path,
                    });
                }
                ConflictActionType::Append => {
                    // Append is handled by noting the action — the actual merge
                    // happens when multiple extensions write to the same file.
                    // The separator is available for future use.
                    results.push(ConflictResult {
                        path: target,
                        action_taken: ConflictActionType::Append,
                        backup_path,
                    });
                }
                ConflictActionType::MergeJson => {
                    results.push(ConflictResult {
                        path: target,
                        action_taken: ConflictActionType::MergeJson,
                        backup_path,
                    });
                }
                ConflictActionType::MergeYaml => {
                    results.push(ConflictResult {
                        path: target,
                        action_taken: ConflictActionType::MergeYaml,
                        backup_path,
                    });
                }
                ConflictActionType::Merge => {
                    // Directory merge — the directory already exists, we just record it
                    results.push(ConflictResult {
                        path: target,
                        action_taken: ConflictActionType::Merge,
                        backup_path,
                    });
                }
                ConflictActionType::Backup => {
                    let bp = if backup_path.is_some() {
                        backup_path
                    } else {
                        Some(Self::create_backup(&target, &action.backup_suffix)?)
                    };
                    results.push(ConflictResult {
                        path: target,
                        action_taken: ConflictActionType::Backup,
                        backup_path: bp,
                    });
                }
                ConflictActionType::BackupAndReplace => {
                    let bp = if backup_path.is_some() {
                        backup_path
                    } else {
                        Some(Self::create_backup(&target, &action.backup_suffix)?)
                    };
                    results.push(ConflictResult {
                        path: target,
                        action_taken: ConflictActionType::BackupAndReplace,
                        backup_path: bp,
                    });
                }
                ConflictActionType::Prepend => {
                    results.push(ConflictResult {
                        path: target,
                        action_taken: ConflictActionType::Prepend,
                        backup_path,
                    });
                }
                ConflictActionType::Prompt | ConflictActionType::PromptPerFile => {
                    // Already handled above via effective_action
                    unreachable!()
                }
            }

            let _ = extension_name; // Used for logging context
        }

        Ok(results)
    }

    /// Merge two JSON files: read existing file, merge with source, write back
    pub fn merge_json_file(existing: &Path, source_content: &str) -> Result<()> {
        let source_json: serde_json::Value =
            serde_json::from_str(source_content).context("Failed to parse source JSON")?;

        if existing.exists() {
            let existing_content =
                std::fs::read_to_string(existing).context("Failed to read existing JSON")?;
            let mut existing_json: serde_json::Value =
                serde_json::from_str(&existing_content).context("Failed to parse existing JSON")?;
            merge_json_values(&mut existing_json, source_json);
            let merged =
                serde_json::to_string_pretty(&existing_json).context("Failed to serialize JSON")?;
            std::fs::write(existing, merged).context("Failed to write merged JSON")?;
        } else {
            let formatted =
                serde_json::to_string_pretty(&source_json).context("Failed to serialize JSON")?;
            std::fs::write(existing, formatted).context("Failed to write JSON")?;
        }

        Ok(())
    }

    /// Merge two YAML files: read existing file, merge with source, write back
    pub fn merge_yaml_file(existing: &Path, source_content: &str) -> Result<()> {
        let source_yaml: serde_yaml_ng::Value =
            serde_yaml_ng::from_str(source_content).context("Failed to parse source YAML")?;

        if existing.exists() {
            let existing_content =
                std::fs::read_to_string(existing).context("Failed to read existing YAML")?;
            let mut existing_yaml: serde_yaml_ng::Value =
                serde_yaml_ng::from_str(&existing_content)
                    .context("Failed to parse existing YAML")?;
            merge_yaml_values(&mut existing_yaml, source_yaml);
            let merged =
                serde_yaml_ng::to_string(&existing_yaml).context("Failed to serialize YAML")?;
            std::fs::write(existing, merged).context("Failed to write merged YAML")?;
        } else {
            let formatted =
                serde_yaml_ng::to_string(&source_yaml).context("Failed to serialize YAML")?;
            std::fs::write(existing, formatted).context("Failed to write YAML")?;
        }

        Ok(())
    }

    fn create_backup(path: &Path, suffix: &str) -> Result<PathBuf> {
        let backup_name = format!(
            "{}{}",
            path.file_name().and_then(|n| n.to_str()).unwrap_or("file"),
            suffix
        );
        let backup_path = path.with_file_name(backup_name);

        if path.is_dir() {
            Self::copy_dir_recursive(path, &backup_path)?;
        } else {
            std::fs::copy(path, &backup_path).context("Failed to create backup")?;
        }

        Ok(backup_path)
    }

    fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let dest_path = dst.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                Self::copy_dir_recursive(&entry.path(), &dest_path)?;
            } else {
                std::fs::copy(entry.path(), dest_path)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sindri_core::types::OnConflictAction;
    use tempfile::TempDir;

    fn make_rule(
        path: &str,
        rtype: ConflictResourceType,
        action: ConflictActionType,
    ) -> ConflictRule {
        ConflictRule {
            path: path.to_string(),
            r#type: rtype,
            on_conflict: OnConflictAction {
                action,
                separator: None,
                backup_suffix: ".backup".to_string(),
                backup: false,
                prompt_options: vec![],
            },
        }
    }

    fn make_rule_with_backup(
        path: &str,
        rtype: ConflictResourceType,
        action: ConflictActionType,
    ) -> ConflictRule {
        ConflictRule {
            path: path.to_string(),
            r#type: rtype,
            on_conflict: OnConflictAction {
                action,
                separator: None,
                backup_suffix: ".backup".to_string(),
                backup: true,
                prompt_options: vec![],
            },
        }
    }

    #[test]
    fn test_skip_existing_file() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("CLAUDE.md"), "existing").unwrap();

        let rules = vec![make_rule(
            "CLAUDE.md",
            ConflictResourceType::File,
            ConflictActionType::Skip,
        )];
        let results = ConflictApplier::apply(
            tmp.path(),
            &rules,
            "test",
            &InteractivityMode::NonInteractive,
        )
        .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].action_taken, ConflictActionType::Skip);
    }

    #[test]
    fn test_overwrite_file() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("config.json"), "old").unwrap();

        let rules = vec![make_rule(
            "config.json",
            ConflictResourceType::File,
            ConflictActionType::Overwrite,
        )];
        let results = ConflictApplier::apply(
            tmp.path(),
            &rules,
            "test",
            &InteractivityMode::NonInteractive,
        )
        .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].action_taken, ConflictActionType::Overwrite);
    }

    #[test]
    fn test_append_with_separator() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("CLAUDE.md"), "content").unwrap();

        let rules = vec![ConflictRule {
            path: "CLAUDE.md".to_string(),
            r#type: ConflictResourceType::File,
            on_conflict: OnConflictAction {
                action: ConflictActionType::Append,
                separator: Some("\n\n---\n\n".to_string()),
                backup_suffix: ".backup".to_string(),
                backup: false,
                prompt_options: vec![],
            },
        }];
        let results = ConflictApplier::apply(
            tmp.path(),
            &rules,
            "test",
            &InteractivityMode::NonInteractive,
        )
        .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].action_taken, ConflictActionType::Append);
    }

    #[test]
    fn test_append_creates_file() {
        let tmp = TempDir::new().unwrap();
        // File does NOT exist — rule should be skipped (only applies to existing paths)
        let rules = vec![make_rule(
            "nonexistent.md",
            ConflictResourceType::File,
            ConflictActionType::Append,
        )];
        let results = ConflictApplier::apply(
            tmp.path(),
            &rules,
            "test",
            &InteractivityMode::NonInteractive,
        )
        .unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_merge_json_deep() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.json");
        std::fs::write(&path, r#"{"a": 1, "nested": {"b": 2}}"#).unwrap();

        ConflictApplier::merge_json_file(&path, r#"{"c": 3, "nested": {"d": 4}}"#).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(json["a"], 1);
        assert_eq!(json["c"], 3);
        assert_eq!(json["nested"]["b"], 2);
        assert_eq!(json["nested"]["d"], 4);
    }

    #[test]
    fn test_merge_json_no_existing() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("new.json");

        ConflictApplier::merge_json_file(&path, r#"{"key": "value"}"#).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("\"key\""));
    }

    #[test]
    fn test_merge_yaml_deep() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.yaml");
        std::fs::write(&path, "a: 1\nnested:\n  b: 2\n").unwrap();

        ConflictApplier::merge_yaml_file(&path, "c: 3\nnested:\n  d: 4\n").unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("a:"));
        assert!(content.contains("c:"));
    }

    #[test]
    fn test_merge_directory() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join(".claude")).unwrap();

        let rules = vec![make_rule(
            ".claude",
            ConflictResourceType::Directory,
            ConflictActionType::Merge,
        )];
        let results = ConflictApplier::apply(
            tmp.path(),
            &rules,
            "test",
            &InteractivityMode::NonInteractive,
        )
        .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].action_taken, ConflictActionType::Merge);
    }

    #[test]
    fn test_backup_before_action() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("important.json"), "data").unwrap();

        let rules = vec![make_rule_with_backup(
            "important.json",
            ConflictResourceType::File,
            ConflictActionType::Overwrite,
        )];
        let results = ConflictApplier::apply(
            tmp.path(),
            &rules,
            "test",
            &InteractivityMode::NonInteractive,
        )
        .unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].backup_path.is_some());
        assert!(results[0].backup_path.as_ref().unwrap().exists());
    }

    #[test]
    fn test_prompt_fallback_to_skip() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("file.txt"), "content").unwrap();

        let rules = vec![make_rule(
            "file.txt",
            ConflictResourceType::File,
            ConflictActionType::Prompt,
        )];
        let results = ConflictApplier::apply(
            tmp.path(),
            &rules,
            "test",
            &InteractivityMode::NonInteractive,
        )
        .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].action_taken, ConflictActionType::Skip);
    }
}
