//! Extension-specific tool checking
//!
//! Checks tools required by installed extensions based on their
//! validate.commands configuration.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{anyhow, Result};
use serde::Deserialize;
use tokio::process::Command;

/// Extension tool requirement from validation config
#[derive(Debug, Clone)]
pub struct ExtensionTool {
    /// Extension name
    pub extension: String,
    /// Tool/command name
    pub tool: String,
    /// Version flag (e.g., "--version")
    pub version_flag: String,
    /// Expected output pattern (regex)
    pub expected_pattern: Option<String>,
}

/// Status of an extension's tool check
#[derive(Debug, Clone)]
pub struct ExtensionToolStatus {
    /// The tool requirement
    pub tool: ExtensionTool,
    /// Whether the tool is available
    pub available: bool,
    /// Version output if available
    pub version: Option<String>,
    /// Error message if check failed
    pub error: Option<String>,
}

/// Result of checking all extension tools
#[derive(Debug)]
pub struct ExtensionCheckResult {
    /// Extensions checked
    pub extensions_checked: Vec<String>,
    /// Tool statuses
    pub tool_statuses: Vec<ExtensionToolStatus>,
    /// Number of missing tools
    pub missing_count: usize,
    /// Number of available tools
    pub available_count: usize,
}

impl ExtensionCheckResult {
    /// Check if all extension tools are available
    pub fn all_available(&self) -> bool {
        self.missing_count == 0
    }
}

/// Extension checker for installed extensions
pub struct ExtensionChecker {
    /// Path to sindri extensions directory
    extensions_dir: PathBuf,
    /// Timeout for tool checks
    timeout: Duration,
}

impl ExtensionChecker {
    /// Create a new extension checker
    pub fn new() -> Self {
        let home = sindri_core::get_home_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            extensions_dir: home.join(".sindri").join("extensions"),
            timeout: Duration::from_secs(5),
        }
    }

    /// Create with custom extensions directory
    pub fn with_dir(extensions_dir: PathBuf) -> Self {
        Self {
            extensions_dir,
            timeout: Duration::from_secs(5),
        }
    }

    /// Check all installed extension tools
    pub async fn check_all(&self) -> Result<ExtensionCheckResult> {
        let tools = self.collect_extension_tools()?;
        let mut tool_statuses = Vec::new();

        for tool in tools {
            let status = self.check_tool(&tool).await;
            tool_statuses.push(status);
        }

        let missing_count = tool_statuses.iter().filter(|s| !s.available).count();
        let available_count = tool_statuses.iter().filter(|s| s.available).count();

        let extensions_checked: Vec<String> = tool_statuses
            .iter()
            .map(|s| s.tool.extension.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        Ok(ExtensionCheckResult {
            extensions_checked,
            tool_statuses,
            missing_count,
            available_count,
        })
    }

    /// Check a specific extension by name
    pub async fn check_extension(&self, name: &str) -> Result<ExtensionCheckResult> {
        let ext_path = self.extensions_dir.join(name).join("extension.yaml");
        if !ext_path.exists() {
            return Err(anyhow!("Extension '{}' not found", name));
        }

        let tools = self.parse_extension_tools(&ext_path, name)?;
        let mut tool_statuses = Vec::new();

        for tool in tools {
            let status = self.check_tool(&tool).await;
            tool_statuses.push(status);
        }

        let missing_count = tool_statuses.iter().filter(|s| !s.available).count();
        let available_count = tool_statuses.iter().filter(|s| s.available).count();

        Ok(ExtensionCheckResult {
            extensions_checked: vec![name.to_string()],
            tool_statuses,
            missing_count,
            available_count,
        })
    }

    /// Collect tool requirements from all installed extensions
    fn collect_extension_tools(&self) -> Result<Vec<ExtensionTool>> {
        let mut tools = Vec::new();

        if !self.extensions_dir.exists() {
            return Ok(tools);
        }

        for entry in std::fs::read_dir(&self.extensions_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let ext_yaml = path.join("extension.yaml");
                if ext_yaml.exists() {
                    let ext_name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown");

                    if let Ok(ext_tools) = self.parse_extension_tools(&ext_yaml, ext_name) {
                        tools.extend(ext_tools);
                    }
                }
            }
        }

        Ok(tools)
    }

    /// Parse tool requirements from an extension.yaml file
    fn parse_extension_tools(&self, path: &Path, ext_name: &str) -> Result<Vec<ExtensionTool>> {
        let content = std::fs::read_to_string(path)?;
        let ext: PartialExtension = serde_yaml_ng::from_str(&content)?;

        let mut tools = Vec::new();

        if let Some(validate) = ext.validate {
            for cmd in validate.commands {
                tools.push(ExtensionTool {
                    extension: ext_name.to_string(),
                    tool: cmd.name,
                    version_flag: cmd.version_flag.unwrap_or_else(|| "--version".to_string()),
                    expected_pattern: cmd.expected_pattern,
                });
            }
        }

        Ok(tools)
    }

    /// Check a single tool
    async fn check_tool(&self, tool: &ExtensionTool) -> ExtensionToolStatus {
        match which::which(&tool.tool) {
            Ok(_) => {
                // Tool exists, try to get version
                let version_result = self.get_version(&tool.tool, &tool.version_flag).await;
                match version_result {
                    Ok(version) => ExtensionToolStatus {
                        tool: tool.clone(),
                        available: true,
                        version: Some(version),
                        error: None,
                    },
                    Err(e) => ExtensionToolStatus {
                        tool: tool.clone(),
                        available: true,
                        version: None,
                        error: Some(format!("Version check failed: {}", e)),
                    },
                }
            }
            Err(_) => ExtensionToolStatus {
                tool: tool.clone(),
                available: false,
                version: None,
                error: Some(format!("Tool '{}' not found in PATH", tool.tool)),
            },
        }
    }

    /// Get version output from a tool
    async fn get_version(&self, cmd: &str, flag: &str) -> Result<String> {
        let output = tokio::time::timeout(self.timeout, Command::new(cmd).arg(flag).output())
            .await
            .map_err(|_| anyhow!("Timeout"))??;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Some tools output version to stderr
        let version = if !stdout.is_empty() {
            stdout.lines().next().unwrap_or("").to_string()
        } else {
            stderr.lines().next().unwrap_or("").to_string()
        };

        Ok(version)
    }
}

impl Default for ExtensionChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Partial extension structure for parsing only validate section
#[derive(Debug, Deserialize)]
struct PartialExtension {
    #[serde(default)]
    validate: Option<ValidateConfig>,
}

#[derive(Debug, Deserialize)]
struct ValidateConfig {
    #[serde(default)]
    commands: Vec<CommandValidation>,
}

#[derive(Debug, Deserialize)]
struct CommandValidation {
    name: String,
    #[serde(default, rename = "versionFlag")]
    version_flag: Option<String>,
    #[serde(default, rename = "expectedPattern")]
    expected_pattern: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_extension_checker_creation() {
        let checker = ExtensionChecker::new();
        assert!(checker.timeout.as_secs() > 0);
    }

    #[test]
    fn test_parse_extension_tools() {
        let dir = tempdir().unwrap();
        let ext_dir = dir.path().join("test-ext");
        fs::create_dir_all(&ext_dir).unwrap();

        let ext_yaml = ext_dir.join("extension.yaml");
        fs::write(
            &ext_yaml,
            r#"
metadata:
  name: test-ext
  version: "1.0.0"
  description: Test extension
  category: utilities

install:
  method: script
  script:
    path: install.sh

validate:
  commands:
    - name: git
      versionFlag: "--version"
    - name: curl
"#,
        )
        .unwrap();

        let checker = ExtensionChecker::with_dir(dir.path().to_path_buf());
        let tools = checker
            .parse_extension_tools(&ext_yaml, "test-ext")
            .unwrap();

        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].tool, "git");
        assert_eq!(tools[0].version_flag, "--version");
        assert_eq!(tools[1].tool, "curl");
    }

    #[test]
    fn test_collect_extension_tools_empty_dir() {
        let dir = tempdir().unwrap();
        let checker = ExtensionChecker::with_dir(dir.path().to_path_buf());
        let tools = checker.collect_extension_tools().unwrap();
        assert!(tools.is_empty());
    }

    #[tokio::test]
    async fn test_check_tool_available() {
        let tool = ExtensionTool {
            extension: "test".to_string(),
            tool: "sh".to_string(), // sh should always exist
            version_flag: "--version".to_string(),
            expected_pattern: None,
        };

        let checker = ExtensionChecker::new();
        let status = checker.check_tool(&tool).await;

        assert!(status.available);
    }

    #[tokio::test]
    async fn test_check_tool_missing() {
        let tool = ExtensionTool {
            extension: "test".to_string(),
            tool: "nonexistent-tool-12345".to_string(),
            version_flag: "--version".to_string(),
            expected_pattern: None,
        };

        let checker = ExtensionChecker::new();
        let status = checker.check_tool(&tool).await;

        assert!(!status.available);
        assert!(status.error.is_some());
    }

    #[tokio::test]
    async fn test_check_all_empty() {
        let dir = tempdir().unwrap();
        let checker = ExtensionChecker::with_dir(dir.path().to_path_buf());
        let result = checker.check_all().await.unwrap();

        assert!(result.all_available());
        assert_eq!(result.missing_count, 0);
        assert_eq!(result.available_count, 0);
    }
}
