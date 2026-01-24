//! Sindri Doctor - Tool Dependency Management System
//!
//! This crate provides comprehensive tool dependency checking and diagnostic
//! capabilities for the Sindri CLI. It helps users identify missing or
//! misconfigured tools required for different Sindri operations.
//!
//! # Features
//!
//! - **Platform Detection**: Identifies OS, architecture, and available package managers
//! - **Tool Registry**: Static registry of all known tools with installation instructions
//! - **Parallel Checking**: Concurrent tool availability and version checking
//! - **Multi-Format Output**: Human-readable, JSON, and YAML output formats
//! - **Category Filtering**: Check tools by provider or command scope
//!
//! # Example
//!
//! ```rust,no_run
//! use sindri_doctor::{Doctor, DoctorOptions, OutputFormat};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let doctor = Doctor::new();
//!     let options = DoctorOptions::default();
//!     let result = doctor.run(&options).await?;
//!
//!     println!("{}", result.format(OutputFormat::Human));
//!     Ok(())
//! }
//! ```

mod checker;
mod extension;
mod installer;
mod platform;
mod registry;
mod reporter;
mod tool;

pub use checker::{AuthStatus, ToolChecker, ToolState, ToolStatus};
pub use extension::{ExtensionCheckResult, ExtensionChecker, ExtensionTool, ExtensionToolStatus};
pub use installer::{InstallResult, InstallSummary, ToolInstaller};
pub use platform::{detect_platform, Arch, LinuxDistro, PackageManager, Platform, PlatformInfo};
pub use registry::{ToolRegistry, TOOL_REGISTRY};
pub use reporter::{DiagnosticReporter, OutputFormat};
pub use tool::{AuthCheck, AuthSuccessIndicator, InstallInstruction, ToolCategory, ToolDefinition};

use anyhow::Result;

/// Main entry point for the doctor diagnostic system
pub struct Doctor {
    checker: ToolChecker,
}

impl Doctor {
    /// Create a new Doctor instance
    pub fn new() -> Self {
        Self {
            checker: ToolChecker::new(),
        }
    }

    /// Run the diagnostic check with the given options
    pub async fn run(&self, options: &DoctorOptions) -> Result<DiagnosticResult> {
        let platform = detect_platform();

        // Get tools to check based on options
        let tools = self.get_tools_to_check(options);

        // Run parallel checks
        let tool_statuses = self.checker.check_all(&tools).await;

        // Calculate overall status
        let overall_status = self.calculate_overall_status(&tool_statuses, options);

        Ok(DiagnosticResult {
            platform,
            tools: tool_statuses,
            overall_status,
            categories_checked: self.get_categories(options),
        })
    }

    /// Get the list of tools to check based on options
    fn get_tools_to_check(&self, options: &DoctorOptions) -> Vec<&'static ToolDefinition> {
        if options.all {
            return ToolRegistry::all().iter().collect();
        }

        let mut tools: Vec<&'static ToolDefinition> = Vec::new();

        // Always include core tools
        tools.extend(ToolRegistry::by_category(ToolCategory::Core));

        // Add provider-specific tools
        if let Some(provider) = &options.provider {
            tools.extend(ToolRegistry::by_provider(provider));
        }

        // Add command-specific tools
        if let Some(command) = &options.command {
            tools.extend(ToolRegistry::by_command(command));
        }

        // If no specific filters, add commonly used tools
        if options.provider.is_none() && options.command.is_none() {
            // Add extension backends by default
            tools.extend(ToolRegistry::by_category(ToolCategory::ExtensionBackend));
            // Add optional tools
            tools.extend(ToolRegistry::by_category(ToolCategory::Optional));
        }

        // Deduplicate
        tools.sort_by_key(|t| t.id);
        tools.dedup_by_key(|t| t.id);

        tools
    }

    /// Get categories being checked
    fn get_categories(&self, options: &DoctorOptions) -> Vec<ToolCategory> {
        let mut categories = vec![ToolCategory::Core];

        if let Some(provider) = &options.provider {
            match provider.as_str() {
                "docker" | "docker-compose" => categories.push(ToolCategory::ProviderDocker),
                "fly" => categories.push(ToolCategory::ProviderFly),
                "devpod" => categories.push(ToolCategory::ProviderDevpod),
                "e2b" => categories.push(ToolCategory::ProviderE2B),
                "kubernetes" | "k8s" => categories.push(ToolCategory::ProviderKubernetes),
                _ => {}
            }
        }

        if let Some(command) = &options.command {
            match command.as_str() {
                "extension" => categories.push(ToolCategory::ExtensionBackend),
                "secrets" => categories.push(ToolCategory::Secrets),
                _ => {}
            }
        }

        if options.all || (options.provider.is_none() && options.command.is_none()) {
            categories.push(ToolCategory::ExtensionBackend);
            categories.push(ToolCategory::Optional);
        }

        categories
    }

    /// Calculate overall status from tool statuses
    fn calculate_overall_status(
        &self,
        tools: &[ToolStatus],
        options: &DoctorOptions,
    ) -> OverallStatus {
        let mut missing_required = 0;
        let mut missing_optional = 0;
        let mut auth_required = 0;
        let mut version_issues = 0;

        for status in tools {
            match &status.state {
                ToolState::Missing => {
                    if status.tool.optional {
                        missing_optional += 1;
                    } else {
                        missing_required += 1;
                    }
                }
                ToolState::VersionTooOld { .. } => {
                    version_issues += 1;
                }
                ToolState::Available => {
                    if options.check_auth {
                        if let Some(AuthStatus::NotAuthenticated) = &status.auth_status {
                            if status.tool.auth_check.is_some() {
                                auth_required += 1;
                            }
                        }
                    }
                }
                ToolState::CheckFailed { .. } => {
                    // Treat as missing for now
                    if !status.tool.optional {
                        missing_required += 1;
                    }
                }
            }
        }

        if missing_required > 0 || version_issues > 0 {
            OverallStatus::MissingRequired(missing_required + version_issues)
        } else if auth_required > 0 {
            OverallStatus::AuthRequired(auth_required)
        } else if missing_optional > 0 {
            OverallStatus::MissingOptional(missing_optional)
        } else {
            OverallStatus::Ready
        }
    }
}

impl Default for Doctor {
    fn default() -> Self {
        Self::new()
    }
}

/// Options for running the doctor diagnostic
#[derive(Debug, Clone, Default)]
pub struct DoctorOptions {
    /// Check tools for a specific provider
    pub provider: Option<String>,
    /// Check tools for a specific command
    pub command: Option<String>,
    /// Check all tools regardless of filters
    pub all: bool,
    /// Check authentication status for tools that require it
    pub check_auth: bool,
    /// Enable verbose output with timing information
    pub verbose: bool,
    /// CI mode - affects exit codes
    pub ci: bool,
}

/// Result of running the doctor diagnostic
#[derive(Debug)]
pub struct DiagnosticResult {
    /// Detected platform information
    pub platform: PlatformInfo,
    /// Status of each checked tool
    pub tools: Vec<ToolStatus>,
    /// Overall status summary
    pub overall_status: OverallStatus,
    /// Categories that were checked
    pub categories_checked: Vec<ToolCategory>,
}

impl DiagnosticResult {
    /// Format the result for display
    pub fn format(&self, format: OutputFormat) -> String {
        let reporter = DiagnosticReporter::new(false);
        reporter.format(self, format)
    }

    /// Format with verbose output
    pub fn format_verbose(&self, format: OutputFormat) -> String {
        let reporter = DiagnosticReporter::new(true);
        reporter.format(self, format)
    }

    /// Get the exit code for CI mode
    pub fn exit_code(&self) -> i32 {
        match &self.overall_status {
            OverallStatus::Ready => 0,
            OverallStatus::MissingOptional(_) => 0,
            OverallStatus::MissingRequired(_) => 1,
            OverallStatus::AuthRequired(_) => 3,
        }
    }

    /// Check if all required tools are available
    pub fn is_ready(&self) -> bool {
        matches!(
            self.overall_status,
            OverallStatus::Ready | OverallStatus::MissingOptional(_)
        )
    }
}

/// Overall status of the diagnostic check
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverallStatus {
    /// All required tools available
    Ready,
    /// Some required tools are missing
    MissingRequired(usize),
    /// Only optional tools are missing
    MissingOptional(usize),
    /// Tools require authentication
    AuthRequired(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doctor_creation() {
        let doctor = Doctor::new();
        assert!(doctor.checker.timeout().as_secs() > 0);
    }

    #[test]
    fn test_default_options() {
        let options = DoctorOptions::default();
        assert!(options.provider.is_none());
        assert!(options.command.is_none());
        assert!(!options.all);
        assert!(!options.check_auth);
    }

    #[tokio::test]
    async fn test_get_tools_to_check_all() {
        let doctor = Doctor::new();
        let options = DoctorOptions {
            all: true,
            ..Default::default()
        };
        let tools = doctor.get_tools_to_check(&options);
        assert!(!tools.is_empty());
    }

    #[tokio::test]
    async fn test_get_tools_to_check_provider() {
        let doctor = Doctor::new();
        let options = DoctorOptions {
            provider: Some("docker".to_string()),
            ..Default::default()
        };
        let tools = doctor.get_tools_to_check(&options);
        assert!(tools.iter().any(|t| t.id == "docker"));
    }
}
