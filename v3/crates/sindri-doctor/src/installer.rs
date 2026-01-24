//! Tool installer module
//!
//! Provides automated installation of missing tools using the appropriate
//! package manager for the current platform. Supports dry-run mode and
//! confirmation prompts.

use std::process::Stdio;

use anyhow::{anyhow, Context, Result};
use dialoguer::Confirm;
use tokio::process::Command;

use crate::checker::ToolStatus;
use crate::platform::{detect_platform, PlatformInfo};
use crate::tool::{InstallInstruction, ToolDefinition};

/// Result of an installation attempt
#[derive(Debug, Clone)]
pub enum InstallResult {
    /// Installation succeeded
    Success,
    /// Installation was skipped (dry run or user declined)
    Skipped,
    /// Dry run - shows what would be done
    DryRun,
    /// Installation failed
    Failed {
        /// Error message
        error: String,
    },
}

/// Tool installer that manages automated installation
pub struct ToolInstaller {
    /// Platform information
    platform: PlatformInfo,
    /// Whether to run in dry-run mode
    dry_run: bool,
    /// Whether to ask for confirmation before each install
    confirm: bool,
}

impl ToolInstaller {
    /// Create a new installer
    pub fn new(dry_run: bool, confirm: bool) -> Self {
        Self {
            platform: detect_platform(),
            dry_run,
            confirm,
        }
    }

    /// Install a single tool
    pub async fn install(&self, tool: &ToolDefinition) -> Result<InstallResult> {
        // Get the best installation instruction for this platform
        let instruction = self.select_instruction(tool).ok_or_else(|| {
            anyhow!(
                "No installation instructions for {} on this platform",
                tool.name
            )
        })?;

        // Show what will be installed
        println!("Tool: {} - {}", tool.name, tool.description);
        println!("Command: {}", instruction.command);
        if let Some(notes) = instruction.notes {
            println!("Note: {}", notes);
        }

        // Dry run mode
        if self.dry_run {
            println!("[dry-run] Would execute: {}", instruction.command);
            return Ok(InstallResult::DryRun);
        }

        // Confirmation prompt
        if self.confirm {
            let proceed = Confirm::new()
                .with_prompt(format!("Install {}?", tool.name))
                .default(true)
                .interact()
                .unwrap_or(false);

            if !proceed {
                println!("Skipped {}", tool.name);
                return Ok(InstallResult::Skipped);
            }
        }

        // Execute installation
        let result = self.execute_install(instruction).await;

        match &result {
            Ok(_) => {
                // Verify installation
                if which::which(tool.command).is_ok() {
                    println!("Successfully installed {}", tool.name);
                    Ok(InstallResult::Success)
                } else {
                    Ok(InstallResult::Failed {
                        error: "Tool not found in PATH after installation".to_string(),
                    })
                }
            }
            Err(e) => Ok(InstallResult::Failed {
                error: e.to_string(),
            }),
        }
    }

    /// Install multiple tools
    pub async fn install_all(&self, statuses: &[ToolStatus]) -> Vec<(String, InstallResult)> {
        let mut results = Vec::new();

        for status in statuses {
            if matches!(status.state, crate::checker::ToolState::Missing) {
                let result = self.install(status.tool).await;
                let result = match result {
                    Ok(r) => r,
                    Err(e) => InstallResult::Failed {
                        error: e.to_string(),
                    },
                };
                results.push((status.tool.name.to_string(), result));
            }
        }

        results
    }

    /// Select the best installation instruction for the current platform
    fn select_instruction<'a>(&self, tool: &'a ToolDefinition) -> Option<&'a InstallInstruction> {
        // First, try to find an instruction for an available package manager
        for pm in &self.platform.package_managers {
            if let Some(inst) = tool
                .install_instructions
                .iter()
                .find(|i| i.package_manager.as_ref() == Some(pm))
            {
                return Some(inst);
            }
        }

        // Fall back to platform-specific instruction without package manager
        tool.install_instructions
            .iter()
            .find(|i| i.platform == self.platform.os && i.package_manager.is_none())
            .or_else(|| {
                // Fall back to any instruction for this platform
                tool.instruction_for_platform(&self.platform.os)
            })
    }

    /// Execute an installation command
    async fn execute_install(&self, instruction: &InstallInstruction) -> Result<()> {
        let command = instruction.command;

        // Skip non-executable instructions (like "Download from...")
        if command.starts_with("Download")
            || command.starts_with("Pre-installed")
            || command.starts_with("Included")
        {
            return Err(anyhow!("Manual installation required: {}", command));
        }

        // Handle commands that need sudo
        let (shell, args) = if command.starts_with("sudo ") {
            // Execute via shell to handle sudo
            ("sh", vec!["-c", command])
        } else if command.contains('|') || command.contains('&') || command.contains(';') {
            // Complex command - use shell
            ("sh", vec!["-c", command])
        } else {
            // Simple command - use shell anyway for consistency
            ("sh", vec!["-c", command])
        };

        println!("Running: {}", command);

        let output = Command::new(shell)
            .args(&args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .context("Failed to spawn installation process")?
            .wait()
            .await
            .context("Installation process failed")?;

        if output.success() {
            Ok(())
        } else {
            Err(anyhow!(
                "Installation command failed with exit code: {:?}",
                output.code()
            ))
        }
    }

    /// Get installation summary
    pub fn summarize_results(results: &[(String, InstallResult)]) -> InstallSummary {
        let mut succeeded = 0;
        let mut failed = 0;
        let mut skipped = 0;

        for (_, result) in results {
            match result {
                InstallResult::Success => succeeded += 1,
                InstallResult::Failed { .. } => failed += 1,
                InstallResult::Skipped | InstallResult::DryRun => skipped += 1,
            }
        }

        InstallSummary {
            succeeded,
            failed,
            skipped,
            results: results.to_vec(),
        }
    }
}

/// Summary of installation results
#[derive(Debug)]
pub struct InstallSummary {
    /// Number of successful installations
    pub succeeded: usize,
    /// Number of failed installations
    pub failed: usize,
    /// Number of skipped installations
    pub skipped: usize,
    /// Individual results
    pub results: Vec<(String, InstallResult)>,
}

impl InstallSummary {
    /// Check if all installations succeeded
    pub fn all_succeeded(&self) -> bool {
        self.failed == 0
    }

    /// Display the summary
    pub fn display(&self) {
        println!("\nInstallation Summary:");
        println!("  Succeeded: {}", self.succeeded);
        println!("  Failed: {}", self.failed);
        println!("  Skipped: {}", self.skipped);

        if self.failed > 0 {
            println!("\nFailed installations:");
            for (name, result) in &self.results {
                if let InstallResult::Failed { error } = result {
                    println!("  {}: {}", name, error);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::ToolRegistry;

    #[test]
    fn test_installer_creation() {
        let installer = ToolInstaller::new(true, false);
        assert!(installer.dry_run);
        assert!(!installer.confirm);
    }

    #[test]
    fn test_select_instruction() {
        let installer = ToolInstaller::new(false, false);
        let git = ToolRegistry::get("git").unwrap();
        let instruction = installer.select_instruction(git);
        assert!(instruction.is_some());
    }

    #[test]
    fn test_summarize_results() {
        let results = vec![
            ("git".to_string(), InstallResult::Success),
            (
                "docker".to_string(),
                InstallResult::Failed {
                    error: "Permission denied".to_string(),
                },
            ),
            ("gh".to_string(), InstallResult::Skipped),
        ];

        let summary = ToolInstaller::summarize_results(&results);
        assert_eq!(summary.succeeded, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.skipped, 1);
        assert!(!summary.all_succeeded());
    }

    #[test]
    fn test_install_summary_display() {
        let results = vec![
            ("git".to_string(), InstallResult::Success),
            (
                "docker".to_string(),
                InstallResult::Failed {
                    error: "Permission denied".to_string(),
                },
            ),
        ];

        let summary = ToolInstaller::summarize_results(&results);
        // Just verify it doesn't panic
        summary.display();
    }
}
