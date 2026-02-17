//! Tool definition types
//!
//! This module defines the types used to describe external tools that Sindri
//! depends on. Each tool has metadata including detection commands, version
//! requirements, installation instructions, and authentication checks.

use crate::platform::{PackageManager, Platform};

/// Definition of an external tool that Sindri may depend on
#[derive(Debug, Clone)]
pub struct ToolDefinition {
    /// Unique identifier (e.g., "docker", "flyctl")
    pub id: &'static str,

    /// Human-readable name (e.g., "Docker", "Fly CLI")
    pub name: &'static str,

    /// Description of what the tool does
    pub description: &'static str,

    /// Command to check existence (the executable name)
    pub command: &'static str,

    /// Flag to get version (e.g., "--version", "version")
    pub version_flag: &'static str,

    /// Minimum required version (semver format)
    pub min_version: Option<&'static str>,

    /// Categories this tool belongs to
    pub categories: &'static [ToolCategory],

    /// Authentication check (if applicable)
    pub auth_check: Option<AuthCheck>,

    /// Installation instructions per platform
    pub install_instructions: &'static [InstallInstruction],

    /// Official documentation URL
    pub docs_url: &'static str,

    /// Whether this tool is optional (nice-to-have vs required)
    pub optional: bool,
}

impl ToolDefinition {
    /// Get the installation instruction for a specific platform
    pub fn instruction_for_platform(&self, platform: &Platform) -> Option<&InstallInstruction> {
        // First try exact match
        self.install_instructions
            .iter()
            .find(|i| i.platform == *platform)
            .or_else(|| {
                // Then try generic Linux match
                if let Platform::Linux(_) = platform {
                    self.install_instructions
                        .iter()
                        .find(|i| matches!(i.platform, Platform::Linux(_)))
                } else {
                    None
                }
            })
            .or_else(|| self.install_instructions.first())
    }
}

/// Categories for grouping tools
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolCategory {
    /// Core tools required for all operations (git)
    Core,
    /// Docker provider tools
    ProviderDocker,
    /// Fly.io provider tools
    ProviderFly,
    /// DevPod provider tools
    ProviderDevpod,
    /// E2B provider tools
    ProviderE2B,
    /// Kubernetes provider tools
    ProviderKubernetes,
    /// RunPod provider tools
    ProviderRunpod,
    /// Northflank provider tools
    ProviderNorthflank,
    /// Local Kubernetes cluster tools (kind, k3d)
    KubernetesClusters,
    /// Packer/VM image building tools
    ProviderPacker,
    /// Infrastructure-as-code tools (terraform, pulumi, ansible)
    Infrastructure,
    /// Cloud provider CLIs (aws, az, gcloud)
    CloudCLI,
    /// Extension installation backends
    ExtensionBackend,
    /// Secret management tools
    Secrets,
    /// Optional enhancement tools
    Optional,
}

impl ToolCategory {
    /// Get the display name for this category
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Core => "Core Tools",
            Self::ProviderDocker => "Docker Provider",
            Self::ProviderFly => "Fly.io Provider",
            Self::ProviderDevpod => "DevPod Provider",
            Self::ProviderE2B => "E2B Provider",
            Self::ProviderKubernetes => "Kubernetes Provider",
            Self::ProviderRunpod => "RunPod Provider",
            Self::ProviderNorthflank => "Northflank Provider",
            Self::KubernetesClusters => "Local Kubernetes Clusters",
            Self::ProviderPacker => "Packer/VM Images",
            Self::Infrastructure => "Infrastructure-as-Code",
            Self::CloudCLI => "Cloud Provider CLIs",
            Self::ExtensionBackend => "Extension Backends",
            Self::Secrets => "Secret Management",
            Self::Optional => "Optional Tools",
        }
    }
}

/// Authentication check configuration
#[derive(Debug, Clone)]
pub struct AuthCheck {
    /// Command to run for auth check
    pub command: &'static str,
    /// Arguments for the auth check command
    pub args: &'static [&'static str],
    /// How to determine if authentication succeeded
    pub success_indicator: AuthSuccessIndicator,
}

/// How to determine if an authentication check succeeded
#[derive(Debug, Clone)]
pub enum AuthSuccessIndicator {
    /// Check if exit code matches
    ExitCode(i32),
    /// Check if stdout contains a string
    StdoutContains(&'static str),
    /// Check if stderr does NOT contain a string
    StderrNotContains(&'static str),
}

/// Installation instruction for a specific platform
#[derive(Debug, Clone)]
pub struct InstallInstruction {
    /// Platform this instruction is for
    pub platform: Platform,
    /// Package manager (if using one)
    pub package_manager: Option<PackageManager>,
    /// The installation command or instruction text
    pub command: &'static str,
    /// Additional notes (e.g., post-install steps)
    pub notes: Option<&'static str>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::LinuxDistro;

    #[test]
    fn test_tool_category_display_name() {
        assert_eq!(ToolCategory::Core.display_name(), "Core Tools");
        assert_eq!(
            ToolCategory::ProviderDocker.display_name(),
            "Docker Provider"
        );
    }

    #[test]
    fn test_instruction_for_platform() {
        let tool = ToolDefinition {
            id: "test",
            name: "Test Tool",
            description: "A test tool",
            command: "test",
            version_flag: "--version",
            min_version: None,
            categories: &[ToolCategory::Core],
            auth_check: None,
            install_instructions: &[
                InstallInstruction {
                    platform: Platform::MacOS,
                    package_manager: Some(PackageManager::Homebrew),
                    command: "brew install test",
                    notes: None,
                },
                InstallInstruction {
                    platform: Platform::Linux(LinuxDistro::Debian),
                    package_manager: Some(PackageManager::Apt),
                    command: "sudo apt install test",
                    notes: None,
                },
            ],
            docs_url: "https://example.com",
            optional: false,
        };

        // Test exact match
        let macos_inst = tool.instruction_for_platform(&Platform::MacOS);
        assert!(macos_inst.is_some());
        assert_eq!(macos_inst.unwrap().command, "brew install test");

        // Test Linux match
        let debian_inst = tool.instruction_for_platform(&Platform::Linux(LinuxDistro::Debian));
        assert!(debian_inst.is_some());
        assert_eq!(debian_inst.unwrap().command, "sudo apt install test");
    }
}
