//! Runtime configuration types for operational parameters
//!
//! These types define configuration that controls runtime behavior like
//! network timeouts, retry policies, backup strategies, etc.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete runtime configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RuntimeConfig {
    /// Network and HTTP configuration
    #[serde(default)]
    pub network: NetworkConfig,

    /// Retry policy configurations
    #[serde(default)]
    pub retry_policies: RetryPoliciesConfig,

    /// GitHub repository settings
    #[serde(default)]
    pub github: GitHubConfig,

    /// Backup and restore policies
    #[serde(default)]
    pub backup: BackupConfig,

    /// Git workflow defaults
    #[serde(default)]
    pub git_workflow: GitWorkflowConfig,

    /// Display and output settings
    #[serde(default)]
    pub display: DisplayConfig,
}

/// Network and HTTP configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct NetworkConfig {
    /// HTTP timeout in seconds
    #[serde(default = "default_http_timeout")]
    pub http_timeout_secs: u64,

    /// Download timeout in seconds
    #[serde(default = "default_download_timeout")]
    pub download_timeout_secs: u64,

    /// Deploy operation timeout in seconds
    #[serde(default = "default_deploy_timeout")]
    pub deploy_timeout_secs: u64,

    /// Download chunk size in bytes
    #[serde(default = "default_chunk_size")]
    pub download_chunk_size: usize,

    /// Mise tool installation timeout in seconds
    #[serde(default = "default_mise_timeout")]
    pub mise_timeout_secs: u64,

    /// User agent string for HTTP requests
    #[serde(default = "default_user_agent")]
    pub user_agent: String,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            http_timeout_secs: default_http_timeout(),
            download_timeout_secs: default_download_timeout(),
            deploy_timeout_secs: default_deploy_timeout(),
            download_chunk_size: default_chunk_size(),
            mise_timeout_secs: default_mise_timeout(),
            user_agent: default_user_agent(),
        }
    }
}

fn default_http_timeout() -> u64 {
    300 // 5 minutes
}
fn default_download_timeout() -> u64 {
    300 // 5 minutes
}
fn default_deploy_timeout() -> u64 {
    600 // 10 minutes
}
fn default_chunk_size() -> usize {
    1024 * 1024 // 1 MB
}
fn default_mise_timeout() -> u64 {
    300 // 5 minutes
}
fn default_user_agent() -> String {
    format!(
        "sindri/{} ({}; {})",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}

/// Retry policy configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RetryPoliciesConfig {
    /// Default retry policy
    #[serde(default)]
    pub default: RetryPolicy,

    /// Per-operation retry policies
    #[serde(default)]
    pub operations: HashMap<String, RetryPolicy>,
}

impl Default for RetryPoliciesConfig {
    fn default() -> Self {
        let mut operations = HashMap::new();

        // Download operations use exponential backoff
        operations.insert(
            "download".to_string(),
            RetryPolicy {
                max_attempts: 3,
                strategy: RetryStrategy::ExponentialBackoff,
                backoff_multiplier: 2.0,
                initial_delay_ms: 1000,
                max_delay_ms: 30000,
            },
        );

        Self {
            default: RetryPolicy::default(),
            operations,
        }
    }
}

/// Retry policy for an operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,

    /// Retry strategy
    #[serde(default)]
    pub strategy: RetryStrategy,

    /// Backoff multiplier for exponential strategies
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,

    /// Initial delay in milliseconds
    #[serde(default = "default_initial_delay")]
    pub initial_delay_ms: u64,

    /// Maximum delay in milliseconds
    #[serde(default = "default_max_delay")]
    pub max_delay_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: default_max_attempts(),
            strategy: RetryStrategy::default(),
            backoff_multiplier: default_backoff_multiplier(),
            initial_delay_ms: default_initial_delay(),
            max_delay_ms: default_max_delay(),
        }
    }
}

fn default_max_attempts() -> u32 {
    3
}
fn default_backoff_multiplier() -> f64 {
    2.0
}
fn default_initial_delay() -> u64 {
    1000
}
fn default_max_delay() -> u64 {
    30000
}

/// Retry strategy
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum RetryStrategy {
    /// No retry
    None,

    /// Fixed delay between retries
    FixedDelay,

    /// Exponential backoff (default)
    #[default]
    ExponentialBackoff,

    /// Linear backoff
    LinearBackoff,
}

/// GitHub repository configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GitHubConfig {
    /// Repository owner
    #[serde(default = "default_repo_owner")]
    pub repo_owner: String,

    /// Repository name
    #[serde(default = "default_repo_name")]
    pub repo_name: String,

    /// Base URL for GitHub API
    #[serde(default = "default_github_api_url")]
    pub api_url: String,

    /// Base URL for raw content
    #[serde(default = "default_github_raw_url")]
    pub raw_url: String,
}

impl Default for GitHubConfig {
    fn default() -> Self {
        Self {
            repo_owner: default_repo_owner(),
            repo_name: default_repo_name(),
            api_url: default_github_api_url(),
            raw_url: default_github_raw_url(),
        }
    }
}

fn default_repo_owner() -> String {
    "pacphi".to_string()
}
fn default_repo_name() -> String {
    "sindri".to_string()
}
fn default_github_api_url() -> String {
    "https://api.github.com".to_string()
}
fn default_github_raw_url() -> String {
    "https://raw.githubusercontent.com".to_string()
}

/// Backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BackupConfig {
    /// Maximum number of backups to keep
    #[serde(default = "default_max_backups")]
    pub max_backups: usize,

    /// Backup file extension
    #[serde(default = "default_backup_extension")]
    pub backup_extension: String,

    /// Include timestamp in backup filename
    #[serde(default = "default_timestamp_enabled")]
    pub include_timestamp: bool,

    /// Timestamp format (using chrono format strings)
    #[serde(default = "default_timestamp_format")]
    pub timestamp_format: String,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            max_backups: default_max_backups(),
            backup_extension: default_backup_extension(),
            include_timestamp: default_timestamp_enabled(),
            timestamp_format: default_timestamp_format(),
        }
    }
}

fn default_max_backups() -> usize {
    2
}
fn default_backup_extension() -> String {
    ".bak".to_string()
}
fn default_timestamp_enabled() -> bool {
    true
}
fn default_timestamp_format() -> String {
    "%Y%m%d_%H%M%S".to_string()
}

/// Git workflow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GitWorkflowConfig {
    /// Default branch name
    #[serde(default = "default_git_branch")]
    pub default_branch: String,

    /// Default initial commit message
    #[serde(default = "default_initial_commit_message")]
    pub initial_commit_message: String,

    /// Default remote name for origin
    #[serde(default = "default_origin_remote")]
    pub origin_remote: String,

    /// Default remote name for upstream
    #[serde(default = "default_upstream_remote")]
    pub upstream_remote: String,

    /// Common main branch names to check
    #[serde(default = "default_main_branch_names")]
    pub main_branch_names: Vec<String>,
}

impl Default for GitWorkflowConfig {
    fn default() -> Self {
        Self {
            default_branch: default_git_branch(),
            initial_commit_message: default_initial_commit_message(),
            origin_remote: default_origin_remote(),
            upstream_remote: default_upstream_remote(),
            main_branch_names: default_main_branch_names(),
        }
    }
}

fn default_git_branch() -> String {
    "main".to_string()
}
fn default_initial_commit_message() -> String {
    "chore: initial commit".to_string()
}
fn default_origin_remote() -> String {
    "origin".to_string()
}
fn default_upstream_remote() -> String {
    "upstream".to_string()
}
fn default_main_branch_names() -> Vec<String> {
    vec!["main".to_string(), "master".to_string()]
}

/// Display and output configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DisplayConfig {
    /// Number of lines to show in output previews
    #[serde(default = "default_preview_lines")]
    pub preview_lines: usize,

    /// Number of lines to show before match in context
    #[serde(default = "default_context_before")]
    pub context_lines_before: usize,

    /// Number of lines to show after match in context
    #[serde(default = "default_context_after")]
    pub context_lines_after: usize,

    /// Enable colored output
    #[serde(default = "default_color_enabled")]
    pub color_enabled: bool,

    /// Enable verbose output by default
    #[serde(default = "default_verbose")]
    pub verbose: bool,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            preview_lines: default_preview_lines(),
            context_lines_before: default_context_before(),
            context_lines_after: default_context_after(),
            color_enabled: default_color_enabled(),
            verbose: default_verbose(),
        }
    }
}

fn default_preview_lines() -> usize {
    10
}
fn default_context_before() -> usize {
    2
}
fn default_context_after() -> usize {
    2
}
fn default_color_enabled() -> bool {
    true
}
fn default_verbose() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_config_defaults() {
        let config = RuntimeConfig::default();
        assert_eq!(config.network.http_timeout_secs, 300);
        assert_eq!(config.network.download_chunk_size, 1024 * 1024);
        assert_eq!(config.network.mise_timeout_secs, 300);
        assert_eq!(config.github.repo_owner, "pacphi");
        assert_eq!(config.github.repo_name, "sindri");
        assert_eq!(config.backup.max_backups, 2);
        assert_eq!(config.git_workflow.default_branch, "main");
    }

    #[test]
    fn test_network_config_serialization() {
        let config = NetworkConfig::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        let deserialized: NetworkConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(config.http_timeout_secs, deserialized.http_timeout_secs);
    }

    #[test]
    fn test_retry_policy_defaults() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 3);
        assert!(matches!(policy.strategy, RetryStrategy::ExponentialBackoff));
        assert_eq!(policy.backoff_multiplier, 2.0);
    }

    #[test]
    fn test_retry_policies_config() {
        let config = RetryPoliciesConfig::default();
        assert!(config.operations.contains_key("download"));
        let download_policy = &config.operations["download"];
        assert_eq!(download_policy.max_attempts, 3);
    }

    #[test]
    fn test_backup_config_serialization() {
        let config = BackupConfig {
            max_backups: 5,
            backup_extension: ".backup".to_string(),
            include_timestamp: false,
            timestamp_format: "%Y%m%d".to_string(),
        };
        let yaml = serde_yaml::to_string(&config).unwrap();
        assert!(yaml.contains("max-backups: 5"));
        assert!(yaml.contains("backup-extension: .backup"));
    }

    #[test]
    fn test_git_workflow_config() {
        let config = GitWorkflowConfig::default();
        assert_eq!(config.default_branch, "main");
        assert_eq!(config.origin_remote, "origin");
        assert_eq!(config.upstream_remote, "upstream");
        assert!(config.main_branch_names.contains(&"main".to_string()));
        assert!(config.main_branch_names.contains(&"master".to_string()));
    }
}
