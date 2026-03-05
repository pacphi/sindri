//! Collision handling for project-init
//!
//! This module provides runtime orchestration for collision-handling declarations
//! in extension.yaml files. It reads collision-handling configs and enforces them
//! during `sindri project new/clone`.
//!
//! The module mirrors the `configure/` pattern and is callable from `enhance.rs`.

pub mod conflict;
pub mod detection;
pub mod ordering;
pub mod scenarios;

pub use conflict::ConflictResult;
pub use detection::DetectedVersion;
pub use scenarios::ScenarioOutcome;

use anyhow::Result;
use chrono::Utc;
use sindri_core::types::{AuthProvider, Extension};
use std::path::PathBuf;
use tracing::{debug, info, warn};

use crate::log_files::ExtensionLogWriter;
use conflict::ConflictApplier;
use detection::VersionDetector;
use ordering::priority_sort;
use scenarios::ScenarioEvaluator;

/// Interactivity mode for conflict resolution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractivityMode {
    Interactive,
    NonInteractive,
}

/// An extension entry prepared for project initialization
#[derive(Debug, Clone)]
pub struct ProjectInitEntry {
    /// Extension name
    pub extension_name: String,
    /// The full extension definition
    pub extension: Extension,
    /// Priority (lower = earlier)
    pub priority: u32,
}

/// Outcome of processing a single extension's project-init
#[derive(Debug)]
pub enum InitOutcome {
    /// Commands executed successfully
    Executed {
        commands_run: usize,
        conflicts_resolved: Vec<ConflictResult>,
    },
    /// Skipped due to scenario evaluation
    Skipped { reason: String },
    /// Stopped due to scenario evaluation (more severe than skip)
    Stopped { reason: String },
    /// Failed with error
    Failed { error: String },
}

/// Result of processing a single extension
#[derive(Debug)]
pub struct ProjectInitResult {
    /// Extension name
    pub extension_name: String,
    /// What happened
    pub outcome: InitOutcome,
}

/// Main orchestrator for collision-aware project initialization
pub struct CollisionResolver {
    workspace: PathBuf,
    mode: InteractivityMode,
}

impl CollisionResolver {
    pub fn new(workspace: PathBuf, mode: InteractivityMode) -> Self {
        Self { workspace, mode }
    }

    /// Main entry point: sort by priority, detect versions, evaluate scenarios,
    /// execute commands, apply conflict rules.
    pub fn resolve_and_execute(
        &self,
        entries: Vec<ProjectInitEntry>,
        auth_checker: &dyn Fn(&Extension, &AuthProvider) -> bool,
    ) -> Result<Vec<ProjectInitResult>> {
        let sorted = priority_sort(entries);
        let total = sorted.len();
        let mut results = Vec::new();

        for (idx, entry) in sorted.into_iter().enumerate() {
            let position = idx + 1;
            info!(
                "[{}/{}] Processing {} (priority {})",
                position, total, entry.extension_name, entry.priority
            );

            let result = self.process_entry(&entry, auth_checker, position, total);

            // Write collision log
            self.write_collision_log(&entry.extension_name, &result);

            results.push(result);
        }

        Ok(results)
    }

    fn process_entry(
        &self,
        entry: &ProjectInitEntry,
        auth_checker: &dyn Fn(&Extension, &AuthProvider) -> bool,
        position: usize,
        total: usize,
    ) -> ProjectInitResult {
        let name = &entry.extension_name;
        let capabilities = match &entry.extension.capabilities {
            Some(c) => c,
            None => {
                return ProjectInitResult {
                    extension_name: name.clone(),
                    outcome: InitOutcome::Skipped {
                        reason: "No capabilities defined".to_string(),
                    },
                };
            }
        };

        let project_init = match &capabilities.project_init {
            Some(pi) if pi.enabled => pi,
            _ => {
                return ProjectInitResult {
                    extension_name: name.clone(),
                    outcome: InitOutcome::Skipped {
                        reason: "Project-init not enabled".to_string(),
                    },
                };
            }
        };

        // Step 1: Collision handling evaluation (if configured)
        if let Some(collision) = &capabilities.collision_handling {
            if collision.enabled {
                // Detect versions
                let detected = VersionDetector::detect(&self.workspace, &collision.version_markers);
                debug!(
                    "[{}/{}] {} detected {} version markers",
                    position,
                    total,
                    name,
                    detected.len()
                );

                // Evaluate scenarios
                let outcome = ScenarioEvaluator::evaluate(
                    &detected,
                    &collision.scenarios,
                    &entry.extension.metadata.version,
                );

                match outcome {
                    ScenarioOutcome::Skip { message } => {
                        info!(
                            "[{}/{}] {} -> skip: {}",
                            position,
                            total,
                            name,
                            message.trim()
                        );
                        return ProjectInitResult {
                            extension_name: name.clone(),
                            outcome: InitOutcome::Skipped { reason: message },
                        };
                    }
                    ScenarioOutcome::Stop { message } => {
                        warn!(
                            "[{}/{}] {} -> stop: {}",
                            position,
                            total,
                            name,
                            message.trim()
                        );
                        return ProjectInitResult {
                            extension_name: name.clone(),
                            outcome: InitOutcome::Stopped { reason: message },
                        };
                    }
                    ScenarioOutcome::Proceed => {
                        debug!("[{}/{}] {} -> proceed", position, total, name);
                    }
                }
            }
        }

        // Step 2: Execute project-init commands
        let mut commands_run = 0;
        for cmd_config in &project_init.commands {
            // Check auth requirements
            let auth_ok = match cmd_config.requires_auth {
                AuthProvider::None => true,
                ref auth => auth_checker(&entry.extension, auth),
            };

            if !auth_ok && cmd_config.conditional {
                debug!(
                    "Skipping {} command (requires auth): {}",
                    name, cmd_config.description
                );
                continue;
            }

            debug!("Running: {}", cmd_config.description);

            let log_dir = dirs_log_dir(&entry.extension_name);
            let result = std::process::Command::new("sh")
                .arg("-c")
                .arg(&cmd_config.command)
                .current_dir(&self.workspace)
                .env("SINDRI_LOG_DIR", &log_dir)
                .output();

            match result {
                Ok(output) if output.status.success() => {
                    commands_run += 1;
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    debug!(
                        "Project-init command failed for {}: {}",
                        name,
                        stderr.trim()
                    );
                }
                Err(e) => {
                    debug!("Failed to run project-init command for {}: {}", name, e);
                }
            }
        }

        // Step 3: Apply conflict rules (if configured)
        let conflicts = if let Some(collision) = &capabilities.collision_handling {
            if collision.enabled {
                match ConflictApplier::apply(
                    &self.workspace,
                    &collision.conflict_rules,
                    name,
                    &self.mode,
                ) {
                    Ok(c) => c,
                    Err(e) => {
                        return ProjectInitResult {
                            extension_name: name.clone(),
                            outcome: InitOutcome::Failed {
                                error: format!("Conflict resolution failed: {}", e),
                            },
                        };
                    }
                }
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        ProjectInitResult {
            extension_name: name.clone(),
            outcome: InitOutcome::Executed {
                commands_run,
                conflicts_resolved: conflicts,
            },
        }
    }

    fn write_collision_log(&self, extension_name: &str, result: &ProjectInitResult) {
        let writer = match ExtensionLogWriter::new_default() {
            Ok(w) => w,
            Err(e) => {
                debug!("Failed to create log writer for collision log: {}", e);
                return;
            }
        };

        let timestamp = Utc::now();
        let (status, log_lines) = match &result.outcome {
            InitOutcome::Executed {
                commands_run,
                conflicts_resolved,
            } => {
                let mut lines = vec![format!("[COMMANDS] {} commands executed", commands_run)];
                for c in conflicts_resolved {
                    lines.push(format!(
                        "[CONFLICT] {} -> {:?}{}",
                        c.path.display(),
                        c.action_taken,
                        if let Some(bp) = &c.backup_path {
                            format!(" (backup: {})", bp.display())
                        } else {
                            String::new()
                        }
                    ));
                }
                ("executed", lines)
            }
            InitOutcome::Skipped { reason } => {
                ("skipped", vec![format!("[SKIP] {}", reason.trim())])
            }
            InitOutcome::Stopped { reason } => {
                ("stopped", vec![format!("[STOP] {}", reason.trim())])
            }
            InitOutcome::Failed { error } => ("failed", vec![format!("[ERROR] {}", error)]),
        };

        if let Err(e) = writer.write_collision_log(extension_name, timestamp, status, &log_lines) {
            debug!(
                "Failed to write collision log for {}: {}",
                extension_name, e
            );
        }
    }
}

/// Get the standard log directory for an extension
fn dirs_log_dir(extension_name: &str) -> PathBuf {
    let home = sindri_core::get_home_dir().unwrap_or_else(|_| PathBuf::from("/tmp"));
    home.join(".sindri").join("logs").join(extension_name)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use sindri_core::types::{
        CommandValidation, Extension, ExtensionCategory, ExtensionMetadata, InstallConfig,
        InstallMethod, ValidateConfig,
    };

    /// Helper to create a minimal ProjectInitEntry for tests
    pub fn make_entry(name: &str, priority: u32) -> ProjectInitEntry {
        let extension = Extension {
            metadata: ExtensionMetadata {
                name: name.to_string(),
                version: "1.0.0".to_string(),
                description: "test".to_string(),
                category: ExtensionCategory::Testing,
                author: None,
                homepage: None,
                dependencies: vec![],
            },
            requirements: None,
            install: InstallConfig {
                method: InstallMethod::Script,
                script: None,
                mise: None,
                apt: None,
                binary: None,
                npm: None,
            },
            validate: ValidateConfig {
                commands: vec![CommandValidation {
                    name: "echo".to_string(),
                    version_flag: "test".to_string(),
                    expected_pattern: None,
                }],
                mise: None,
            },
            configure: None,
            remove: None,
            upgrade: None,
            capabilities: None,
            deprecation: None,
            docs: None,
            bom: None,
        };

        ProjectInitEntry {
            extension_name: name.to_string(),
            extension,
            priority,
        }
    }

    #[test]
    fn test_collision_resolver_creation() {
        let resolver =
            CollisionResolver::new(PathBuf::from("/tmp"), InteractivityMode::NonInteractive);
        assert_eq!(resolver.workspace, PathBuf::from("/tmp"));
        assert_eq!(resolver.mode, InteractivityMode::NonInteractive);
    }

    #[test]
    fn test_resolve_empty_entries() {
        let resolver =
            CollisionResolver::new(PathBuf::from("/tmp"), InteractivityMode::NonInteractive);
        let results = resolver.resolve_and_execute(vec![], &|_, _| false).unwrap();
        assert!(results.is_empty());
    }
}
