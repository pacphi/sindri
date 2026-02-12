//! Diagnostic reporter module
//!
//! Provides formatting for diagnostic results in multiple output formats:
//! human-readable, JSON, and YAML.

use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};

use crate::checker::{AuthStatus, ToolState, ToolStatus};
use crate::platform::PlatformInfo;
use crate::tool::ToolCategory;
use crate::{DiagnosticResult, OverallStatus};

/// Output format for diagnostic results
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// Human-readable colored output
    #[default]
    Human,
    /// JSON format for machine consumption
    Json,
    /// YAML format for machine consumption
    Yaml,
}

/// Diagnostic result reporter
pub struct DiagnosticReporter {
    verbose: bool,
}

impl DiagnosticReporter {
    /// Create a new reporter
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }

    /// Format the diagnostic result
    pub fn format(&self, result: &DiagnosticResult, format: OutputFormat) -> String {
        match format {
            OutputFormat::Human => self.format_human(result),
            OutputFormat::Json => self.format_json(result),
            OutputFormat::Yaml => self.format_yaml(result),
        }
    }

    /// Format as human-readable output
    fn format_human(&self, result: &DiagnosticResult) -> String {
        let mut output = String::new();

        // Header
        output.push_str(&self.format_header(&result.platform));
        output.push('\n');

        // Group tools by category and display
        output.push_str(&self.format_tools_by_category(&result.tools));

        // Summary
        output.push_str(&self.format_summary(&result.overall_status));

        output
    }

    /// Format header with platform info
    fn format_header(&self, platform: &PlatformInfo) -> String {
        let mut header = String::new();

        header.push_str(&format!("{}\n", "Sindri Doctor".bold()));
        header.push_str(&format!("Platform: {} ({})\n", platform.os, platform.arch));

        if !platform.package_managers.is_empty() {
            let managers: Vec<String> = platform
                .package_managers
                .iter()
                .map(|pm| pm.to_string())
                .collect();
            header.push_str(&format!("Package managers: {}\n", managers.join(", ")));
        }

        header
    }

    /// Format tools grouped by category
    fn format_tools_by_category(&self, tools: &[ToolStatus]) -> String {
        let mut output = String::new();

        // Define category order
        let categories = [
            ToolCategory::Core,
            ToolCategory::ProviderDocker,
            ToolCategory::ProviderFly,
            ToolCategory::ProviderDevpod,
            ToolCategory::ProviderE2B,
            ToolCategory::ProviderKubernetes,
            ToolCategory::ExtensionBackend,
            ToolCategory::Secrets,
            ToolCategory::Optional,
        ];

        for category in categories {
            let category_tools: Vec<&ToolStatus> = tools
                .iter()
                .filter(|t| t.tool.categories.contains(&category))
                .collect();

            if category_tools.is_empty() {
                continue;
            }

            // Category header
            output.push_str(&format!("{}\n", category.display_name().bold().underline()));

            // Tools in category
            for status in category_tools {
                output.push_str(&self.format_tool_status(status));
            }

            output.push('\n');
        }

        output
    }

    /// Format a single tool status
    fn format_tool_status(&self, status: &ToolStatus) -> String {
        let mut output = String::new();

        let (icon, name_style, status_text) = match &status.state {
            ToolState::Available => {
                let auth_text = match &status.auth_status {
                    Some(AuthStatus::Authenticated) => " (authenticated)".green().to_string(),
                    Some(AuthStatus::NotAuthenticated) => {
                        " (not authenticated)".yellow().to_string()
                    }
                    Some(AuthStatus::Unknown) => " (auth unknown)".dimmed().to_string(),
                    None => String::new(),
                };
                let version = status.version.as_deref().unwrap_or("-");
                (
                    "✓".green().to_string(),
                    status.tool.name.green().to_string(),
                    format!(
                        "{} - {}{}",
                        version.dimmed(),
                        status.tool.description.dimmed(),
                        auth_text
                    ),
                )
            }
            ToolState::Missing => (
                "✗".red().to_string(),
                status.tool.name.red().to_string(),
                format!("- {}", status.tool.description.dimmed()),
            ),
            ToolState::VersionTooOld { found, required } => (
                "⚠".yellow().to_string(),
                status.tool.name.yellow().to_string(),
                format!(
                    "{} - {} (required: {}+)",
                    found.dimmed(),
                    "version too old".yellow(),
                    required
                ),
            ),
            ToolState::CheckFailed { error } => (
                "?".yellow().to_string(),
                status.tool.name.yellow().to_string(),
                format!("- {}", error.yellow()),
            ),
        };

        output.push_str(&format!("  {} {} {}\n", icon, name_style, status_text));

        // Show install instructions for missing tools
        if matches!(status.state, ToolState::Missing) {
            if let Some(instruction) = status
                .tool
                .instruction_for_platform(&crate::detect_platform().os)
            {
                output.push_str(&format!(
                    "      Install: {}\n",
                    instruction.command.yellow()
                ));
                if let Some(notes) = instruction.notes {
                    output.push_str(&format!("      Note: {}\n", notes.dimmed()));
                }
            }
        }

        // Show auth instructions for unauthenticated tools
        if matches!(status.auth_status, Some(AuthStatus::NotAuthenticated))
            && status.tool.auth_check.is_some()
        {
            // Provide auth hint based on tool
            let auth_hint = match status.tool.id {
                "flyctl" => Some("flyctl auth login"),
                "gh" => Some("gh auth login"),
                "vault" => Some("vault login"),
                "e2b" => Some("e2b auth login"),
                _ => None,
            };
            if let Some(hint) = auth_hint {
                output.push_str(&format!("      Authenticate: {}\n", hint.yellow()));
            }
        }

        // Verbose: show check duration
        if self.verbose {
            output.push_str(&format!(
                "      {} checked in {:?}\n",
                "→".dimmed(),
                status.check_duration
            ));
        }

        output
    }

    /// Format the summary section
    fn format_summary(&self, status: &OverallStatus) -> String {
        let mut output = String::new();

        output.push_str(&format!("{}\n", "Summary".bold().underline()));

        match status {
            OverallStatus::Ready => {
                output.push_str(&format!(
                    "  {} All tools available, ready to use Sindri!\n",
                    "✓".green()
                ));
            }
            OverallStatus::MissingRequired(n) => {
                output.push_str(&format!("  {} {} required tool(s) missing\n", "✗".red(), n));
                output.push_str("  Install the missing tools above to proceed.\n");
            }
            OverallStatus::MissingOptional(n) => {
                output.push_str(&format!(
                    "  {} Ready to use Sindri ({} optional tool(s) missing)\n",
                    "✓".green(),
                    n
                ));
            }
            OverallStatus::AuthRequired(n) => {
                output.push_str(&format!(
                    "  {} {} tool(s) require authentication\n",
                    "⚠".yellow(),
                    n
                ));
            }
        }

        output
    }

    /// Format as JSON
    fn format_json(&self, result: &DiagnosticResult) -> String {
        let json_result = JsonDiagnosticResult::from(result);
        serde_json::to_string_pretty(&json_result)
            .unwrap_or_else(|e| format!("{{\"error\": \"Failed to serialize: {}\"}}", e))
    }

    /// Format as YAML
    fn format_yaml(&self, result: &DiagnosticResult) -> String {
        let json_result = JsonDiagnosticResult::from(result);
        serde_yaml_ng::to_string(&json_result)
            .unwrap_or_else(|e| format!("error: \"Failed to serialize: {}\"", e))
    }
}

impl Default for DiagnosticReporter {
    fn default() -> Self {
        Self::new(false)
    }
}

/// JSON-serializable diagnostic result
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonDiagnosticResult {
    pub platform: JsonPlatformInfo,
    pub tools: Vec<JsonToolStatus>,
    pub overall_status: String,
    pub missing_required_count: usize,
    pub missing_optional_count: usize,
    pub auth_required_count: usize,
}

impl From<&DiagnosticResult> for JsonDiagnosticResult {
    fn from(result: &DiagnosticResult) -> Self {
        let mut missing_required = 0;
        let mut missing_optional = 0;
        let mut auth_required = 0;

        for status in &result.tools {
            match &status.state {
                ToolState::Missing => {
                    if status.tool.optional {
                        missing_optional += 1;
                    } else {
                        missing_required += 1;
                    }
                }
                ToolState::Available => {
                    if let Some(AuthStatus::NotAuthenticated) = &status.auth_status {
                        if status.tool.auth_check.is_some() {
                            auth_required += 1;
                        }
                    }
                }
                _ => {}
            }
        }

        Self {
            platform: JsonPlatformInfo::from(&result.platform),
            tools: result.tools.iter().map(JsonToolStatus::from).collect(),
            overall_status: match &result.overall_status {
                OverallStatus::Ready => "ready".to_string(),
                OverallStatus::MissingRequired(_) => "missing_required".to_string(),
                OverallStatus::MissingOptional(_) => "missing_optional".to_string(),
                OverallStatus::AuthRequired(_) => "auth_required".to_string(),
            },
            missing_required_count: missing_required,
            missing_optional_count: missing_optional,
            auth_required_count: auth_required,
        }
    }
}

/// JSON-serializable platform info
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonPlatformInfo {
    pub os: String,
    pub arch: String,
    pub package_managers: Vec<String>,
}

impl From<&PlatformInfo> for JsonPlatformInfo {
    fn from(platform: &PlatformInfo) -> Self {
        Self {
            os: platform.os.to_string(),
            arch: platform.arch.to_string(),
            package_managers: platform
                .package_managers
                .iter()
                .map(|pm| pm.to_string())
                .collect(),
        }
    }
}

/// JSON-serializable tool status
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonToolStatus {
    pub id: String,
    pub name: String,
    pub state: String,
    pub version: Option<String>,
    pub required: bool,
    pub auth_status: Option<String>,
    pub install_command: Option<String>,
    pub check_duration_ms: u64,
}

impl From<&ToolStatus> for JsonToolStatus {
    fn from(status: &ToolStatus) -> Self {
        let platform = crate::detect_platform();
        let install_command = if matches!(status.state, ToolState::Missing) {
            status
                .tool
                .instruction_for_platform(&platform.os)
                .map(|i| i.command.to_string())
        } else {
            None
        };

        Self {
            id: status.tool.id.to_string(),
            name: status.tool.name.to_string(),
            state: match &status.state {
                ToolState::Available => "available".to_string(),
                ToolState::Missing => "missing".to_string(),
                ToolState::VersionTooOld { found, required } => {
                    format!("version_too_old:{}->{}", found, required)
                }
                ToolState::CheckFailed { error } => format!("check_failed:{}", error),
            },
            version: status.version.clone(),
            required: !status.tool.optional,
            auth_status: status.auth_status.as_ref().map(|s| match s {
                AuthStatus::Authenticated => "authenticated".to_string(),
                AuthStatus::NotAuthenticated => "not_authenticated".to_string(),
                AuthStatus::Unknown => "unknown".to_string(),
            }),
            install_command,
            check_duration_ms: status.check_duration.as_millis() as u64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{Arch, LinuxDistro, PackageManager, Platform};
    use crate::tool::ToolDefinition;
    use std::time::Duration;

    fn create_test_platform() -> PlatformInfo {
        PlatformInfo {
            os: Platform::Linux(LinuxDistro::Debian),
            arch: Arch::X86_64,
            package_managers: vec![PackageManager::Apt],
        }
    }

    fn create_test_tool() -> &'static ToolDefinition {
        crate::registry::ToolRegistry::get("git").unwrap()
    }

    #[test]
    fn test_format_header() {
        let reporter = DiagnosticReporter::new(false);
        let platform = create_test_platform();
        let header = reporter.format_header(&platform);

        assert!(header.contains("Sindri Doctor"));
        assert!(header.contains("Linux/Debian"));
        assert!(header.contains("x86_64"));
        assert!(header.contains("APT"));
    }

    #[test]
    fn test_format_summary_ready() {
        let reporter = DiagnosticReporter::new(false);
        let summary = reporter.format_summary(&OverallStatus::Ready);

        assert!(summary.contains("Summary"));
        assert!(summary.contains("ready to use Sindri"));
    }

    #[test]
    fn test_format_summary_missing() {
        let reporter = DiagnosticReporter::new(false);
        let summary = reporter.format_summary(&OverallStatus::MissingRequired(3));

        assert!(summary.contains("3 required tool(s) missing"));
    }

    #[test]
    fn test_json_format() {
        let result = DiagnosticResult {
            platform: create_test_platform(),
            tools: vec![ToolStatus {
                tool: create_test_tool(),
                state: ToolState::Available,
                version: Some("2.43.0".to_string()),
                auth_status: None,
                check_duration: Duration::from_millis(100),
            }],
            overall_status: OverallStatus::Ready,
            categories_checked: vec![ToolCategory::Core],
        };

        let reporter = DiagnosticReporter::new(false);
        let json = reporter.format_json(&result);

        // Should be valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["overall_status"], "ready");
        assert!(parsed["tools"].is_array());
    }

    #[test]
    fn test_yaml_format() {
        let result = DiagnosticResult {
            platform: create_test_platform(),
            tools: vec![ToolStatus {
                tool: create_test_tool(),
                state: ToolState::Available,
                version: Some("2.43.0".to_string()),
                auth_status: None,
                check_duration: Duration::from_millis(100),
            }],
            overall_status: OverallStatus::Ready,
            categories_checked: vec![ToolCategory::Core],
        };

        let reporter = DiagnosticReporter::new(false);
        let yaml = reporter.format_yaml(&result);

        // Should contain YAML-formatted content
        assert!(yaml.contains("overall_status: ready"));
        assert!(yaml.contains("tools:"));
    }

    #[test]
    fn test_verbose_output() {
        let reporter = DiagnosticReporter::new(true);
        let status = ToolStatus {
            tool: create_test_tool(),
            state: ToolState::Available,
            version: Some("2.43.0".to_string()),
            auth_status: None,
            check_duration: Duration::from_millis(100),
        };

        let output = reporter.format_tool_status(&status);

        // Verbose output should include timing
        assert!(output.contains("100"));
    }
}
