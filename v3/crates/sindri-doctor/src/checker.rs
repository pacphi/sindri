//! Parallel tool checking module
//!
//! Provides async parallel checking of tool availability, versions,
//! and authentication status. Uses tokio for concurrent execution.

use std::sync::LazyLock;
use std::time::{Duration, Instant};

use futures::future::join_all;
use regex::Regex;
use tokio::process::Command;

/// Pre-compiled regex for extracting version numbers from command output
static VERSION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"v?(\d+\.\d+(?:\.\d+)?)").expect("version regex is valid"));

use crate::tool::{AuthCheck, AuthSuccessIndicator, ToolDefinition};

/// Tool checker that runs availability and version checks
pub struct ToolChecker {
    /// Timeout for each check operation
    timeout: Duration,
}

impl ToolChecker {
    /// Create a new tool checker with default timeout
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(5),
        }
    }

    /// Create a new tool checker with custom timeout
    pub fn with_timeout(timeout: Duration) -> Self {
        Self { timeout }
    }

    /// Get the configured timeout
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Check all tools in parallel
    pub async fn check_all(&self, tools: &[&'static ToolDefinition]) -> Vec<ToolStatus> {
        let futures: Vec<_> = tools.iter().map(|tool| self.check_tool(tool)).collect();

        join_all(futures).await
    }

    /// Check a single tool
    async fn check_tool(&self, tool: &'static ToolDefinition) -> ToolStatus {
        let start = Instant::now();

        // Check if tool exists in PATH
        let exists = which::which(tool.command).is_ok();

        if !exists {
            return ToolStatus {
                tool,
                state: ToolState::Missing,
                version: None,
                auth_status: None,
                check_duration: start.elapsed(),
            };
        }

        // Get version
        let version = self.get_version(tool).await;

        // Check minimum version
        let state = match (&version, tool.min_version) {
            (Some(v), Some(min)) => {
                if self.version_satisfies(v, min) {
                    ToolState::Available
                } else {
                    ToolState::VersionTooOld {
                        found: v.clone(),
                        required: min.to_string(),
                    }
                }
            }
            _ => ToolState::Available,
        };

        // Check authentication if applicable
        let auth_status = if let Some(auth_check) = &tool.auth_check {
            Some(self.check_auth(auth_check).await)
        } else {
            None
        };

        ToolStatus {
            tool,
            state,
            version,
            auth_status,
            check_duration: start.elapsed(),
        }
    }

    /// Get the version of a tool
    async fn get_version(&self, tool: &ToolDefinition) -> Option<String> {
        // Build command - handle multi-word version flags
        let parts: Vec<&str> = tool.version_flag.split_whitespace().collect();

        let result = tokio::time::timeout(self.timeout, async {
            let mut cmd = Command::new(tool.command);
            for part in &parts {
                cmd.arg(part);
            }
            cmd.output().await
        })
        .await;

        match result {
            Ok(Ok(output)) => {
                // Try stdout first, then stderr
                let text = if output.stdout.is_empty() {
                    String::from_utf8_lossy(&output.stderr)
                } else {
                    String::from_utf8_lossy(&output.stdout)
                };
                Some(self.parse_version(&text))
            }
            _ => None,
        }
    }

    /// Parse version from command output
    fn parse_version(&self, text: &str) -> String {
        // Common version patterns:
        // - "git version 2.39.0"
        // - "Docker version 24.0.6, build ed223bc"
        // - "flyctl v0.1.130"
        // - "v2.43.0"
        // - "2.43.0"
        // - "npm 10.2.5"

        // Try to find version pattern
        VERSION_RE
            .captures(text)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| {
                // Fallback: return first line trimmed
                text.lines().next().unwrap_or("").trim().to_string()
            })
    }

    /// Check if actual version satisfies minimum requirement
    fn version_satisfies(&self, actual: &str, required: &str) -> bool {
        // Try semver comparison
        match (
            semver::Version::parse(actual),
            semver::Version::parse(required),
        ) {
            (Ok(actual_ver), Ok(required_ver)) => actual_ver >= required_ver,
            _ => {
                // If we can't parse, do simple string comparison
                // or assume it's fine to avoid false negatives
                true
            }
        }
    }

    /// Check authentication status
    async fn check_auth(&self, auth_check: &AuthCheck) -> AuthStatus {
        let result = tokio::time::timeout(self.timeout, async {
            Command::new(auth_check.command)
                .args(auth_check.args)
                .output()
                .await
        })
        .await;

        match result {
            Ok(Ok(output)) => match &auth_check.success_indicator {
                AuthSuccessIndicator::ExitCode(expected) => {
                    if output.status.code() == Some(*expected) {
                        AuthStatus::Authenticated
                    } else {
                        AuthStatus::NotAuthenticated
                    }
                }
                AuthSuccessIndicator::StdoutContains(pattern) => {
                    if String::from_utf8_lossy(&output.stdout).contains(pattern) {
                        AuthStatus::Authenticated
                    } else {
                        AuthStatus::NotAuthenticated
                    }
                }
                AuthSuccessIndicator::StderrNotContains(pattern) => {
                    if !String::from_utf8_lossy(&output.stderr).contains(pattern) {
                        AuthStatus::Authenticated
                    } else {
                        AuthStatus::NotAuthenticated
                    }
                }
            },
            _ => AuthStatus::Unknown,
        }
    }
}

impl Default for ToolChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Status of a tool after checking
#[derive(Debug, Clone)]
pub struct ToolStatus {
    /// The tool definition
    pub tool: &'static ToolDefinition,
    /// Current state of the tool
    pub state: ToolState,
    /// Detected version (if available)
    pub version: Option<String>,
    /// Authentication status (if applicable)
    pub auth_status: Option<AuthStatus>,
    /// How long the check took
    pub check_duration: Duration,
}

/// State of a tool
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolState {
    /// Tool is available and meets requirements
    Available,
    /// Tool is not installed
    Missing,
    /// Tool version is too old
    VersionTooOld {
        /// The version found
        found: String,
        /// The minimum required version
        required: String,
    },
    /// Tool check failed
    CheckFailed {
        /// Error message
        error: String,
    },
}

/// Authentication status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthStatus {
    /// Tool is authenticated
    Authenticated,
    /// Tool is not authenticated
    NotAuthenticated,
    /// Authentication status unknown
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::ToolRegistry;

    #[test]
    fn test_checker_creation() {
        let checker = ToolChecker::new();
        assert_eq!(checker.timeout(), Duration::from_secs(5));
    }

    #[test]
    fn test_checker_with_timeout() {
        let checker = ToolChecker::with_timeout(Duration::from_secs(10));
        assert_eq!(checker.timeout(), Duration::from_secs(10));
    }

    #[test]
    fn test_parse_version_git() {
        let checker = ToolChecker::new();
        assert_eq!(checker.parse_version("git version 2.39.0"), "2.39.0");
    }

    #[test]
    fn test_parse_version_docker() {
        let checker = ToolChecker::new();
        assert_eq!(
            checker.parse_version("Docker version 24.0.6, build ed223bc"),
            "24.0.6"
        );
    }

    #[test]
    fn test_parse_version_fly() {
        let checker = ToolChecker::new();
        assert_eq!(checker.parse_version("flyctl v0.1.130"), "0.1.130");
    }

    #[test]
    fn test_parse_version_npm() {
        let checker = ToolChecker::new();
        assert_eq!(checker.parse_version("10.2.5"), "10.2.5");
    }

    #[test]
    fn test_version_satisfies_valid() {
        let checker = ToolChecker::new();
        assert!(checker.version_satisfies("2.43.0", "2.0.0"));
        assert!(checker.version_satisfies("2.0.0", "2.0.0"));
        assert!(!checker.version_satisfies("1.9.0", "2.0.0"));
    }

    #[test]
    fn test_version_satisfies_invalid() {
        let checker = ToolChecker::new();
        // Invalid versions should return true to avoid false negatives
        assert!(checker.version_satisfies("invalid", "2.0.0"));
    }

    #[tokio::test]
    async fn test_check_tool_missing() {
        let checker = ToolChecker::new();

        // We use a registry tool to test since we need a static reference
        // that we know the state of
        let git = ToolRegistry::get("git").unwrap();
        let status = checker.check_tool(git).await;

        // Git might be installed or not depending on the system
        // Just verify we get a valid status
        assert!(matches!(
            status.state,
            ToolState::Available | ToolState::Missing | ToolState::VersionTooOld { .. }
        ));
    }

    #[tokio::test]
    async fn test_check_all_parallel() {
        let checker = ToolChecker::new();
        let tools: Vec<&'static ToolDefinition> = ToolRegistry::all().iter().take(3).collect();

        let start = Instant::now();
        let statuses = checker.check_all(&tools).await;
        let duration = start.elapsed();

        // Should have status for each tool
        assert_eq!(statuses.len(), 3);

        // Parallel execution should be faster than sequential
        // (3 tools with 5s timeout each would be 15s if sequential)
        // This is a sanity check, not a strict assertion
        assert!(duration < Duration::from_secs(10));
    }
}
