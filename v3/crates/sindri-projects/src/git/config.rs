//! Git configuration management

use crate::error::{Error, Result};
use camino::Utf8Path;
use sindri_core::types::GitWorkflowConfig;
use tokio::process::Command;
use tracing::{debug, info};

/// Git configuration scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigScope {
    /// Local repository config (.git/config)
    Local,
    /// Global user config (~/.gitconfig)
    Global,
    /// System-wide config (/etc/gitconfig)
    System,
}

impl ConfigScope {
    fn as_arg(&self) -> &str {
        match self {
            Self::Local => "--local",
            Self::Global => "--global",
            Self::System => "--system",
        }
    }
}

/// Configure git user information
///
/// # Arguments
/// * `path` - Repository path (only used for local scope)
/// * `name` - User name to set
/// * `email` - User email to set
/// * `scope` - Configuration scope (local, global, or system)
///
/// # Returns
/// Ok(()) on success
pub async fn configure_user(
    path: Option<&Utf8Path>,
    name: Option<&str>,
    email: Option<&str>,
    scope: ConfigScope,
) -> Result<()> {
    if name.is_none() && email.is_none() {
        debug!("No git config overrides to apply");
        return Ok(());
    }

    info!("Configuring git user information");

    // Set user name
    if let Some(name) = name {
        set_config_value(path, "user.name", name, scope).await?;
        info!("Git user name set to: {}", name);
    }

    // Set user email
    if let Some(email) = email {
        set_config_value(path, "user.email", email, scope).await?;
        info!("Git user email set to: {}", email);
    }

    Ok(())
}

/// Set a git configuration value
///
/// # Arguments
/// * `path` - Repository path (only used for local scope)
/// * `key` - Configuration key (e.g., "user.name")
/// * `value` - Configuration value
/// * `scope` - Configuration scope
///
/// # Returns
/// Ok(()) on success
pub async fn set_config_value(
    path: Option<&Utf8Path>,
    key: &str,
    value: &str,
    scope: ConfigScope,
) -> Result<()> {
    debug!("Setting git config: {} = {}", key, value);

    let mut cmd = Command::new("git");

    // Set working directory for local config
    if scope == ConfigScope::Local {
        if let Some(p) = path {
            cmd.current_dir(p);
        }
    }

    cmd.args(["config", scope.as_arg(), key, value]);

    let output = cmd.output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::invalid_config(format!(
            "Failed to set {}: {}",
            key, stderr
        )));
    }

    Ok(())
}

/// Get a git configuration value
///
/// # Arguments
/// * `path` - Repository path (only used for local scope)
/// * `key` - Configuration key (e.g., "user.name")
/// * `scope` - Optional configuration scope
///
/// # Returns
/// The configuration value if set, None otherwise
pub async fn get_config_value(
    path: Option<&Utf8Path>,
    key: &str,
    scope: Option<ConfigScope>,
) -> Result<Option<String>> {
    debug!("Getting git config: {}", key);

    let mut cmd = Command::new("git");

    // Set working directory for local lookups
    if let Some(p) = path {
        cmd.current_dir(p);
    }

    cmd.arg("config");

    if let Some(scope) = scope {
        cmd.arg(scope.as_arg());
    }

    cmd.arg(key);

    let output = cmd.output().await?;

    if !output.status.success() {
        // Config value not set
        return Ok(None);
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();

    Ok(Some(value))
}

/// Check if a branch exists in the repository
///
/// # Arguments
/// * `path` - Repository path
/// * `branch_name` - Name of the branch to check
///
/// # Returns
/// true if the branch exists, false otherwise
pub async fn branch_exists(path: &Utf8Path, branch_name: &str) -> Result<bool> {
    let output = Command::new("git")
        .current_dir(path)
        .args(["branch", "--list", branch_name])
        .output()
        .await?;

    if !output.status.success() {
        return Ok(false);
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    Ok(!output_str.trim().is_empty())
}

/// Detect the main branch name in a repository
///
/// Checks for common main branch names (main, master) and returns the first one found.
/// Falls back to the default branch from config if none are found.
///
/// # Arguments
/// * `path` - Repository path
/// * `git_config` - Git workflow configuration containing main branch names to check
///
/// # Returns
/// The detected main branch name, or the default from config
pub async fn detect_main_branch(path: &Utf8Path, git_config: &GitWorkflowConfig) -> Result<String> {
    for branch_name in &git_config.main_branch_names {
        if branch_exists(path, branch_name).await? {
            debug!("Detected main branch: {}", branch_name);
            return Ok(branch_name.clone());
        }
    }
    debug!(
        "No main branch found, using default: {}",
        git_config.default_branch
    );
    Ok(git_config.default_branch.clone())
}

/// Setup git aliases for fork management
///
/// # Arguments
/// * `path` - Repository path
///
/// # Returns
/// Ok(()) on success
pub async fn setup_fork_aliases(path: &Utf8Path) -> Result<()> {
    let git_config = GitWorkflowConfig::default();
    setup_fork_aliases_with_config(path, &git_config).await
}

/// Setup git aliases for fork management with configurable values
///
/// # Arguments
/// * `path` - Repository path
/// * `git_config` - Git workflow configuration for branch and remote names
///
/// # Returns
/// Ok(()) on success
pub async fn setup_fork_aliases_with_config(
    path: &Utf8Path,
    git_config: &GitWorkflowConfig,
) -> Result<()> {
    info!("Setting up fork management aliases");

    let default_branch = &git_config.default_branch;
    let upstream_remote = &git_config.upstream_remote;
    let origin_remote = &git_config.origin_remote;

    // Sync with upstream
    set_config_value(
        Some(path),
        "alias.sync-upstream",
        &format!(
            "!git fetch {} && git checkout {} && git merge {}/{}",
            upstream_remote, default_branch, upstream_remote, default_branch
        ),
        ConfigScope::Local,
    )
    .await?;

    // Push to fork's origin
    set_config_value(
        Some(path),
        "alias.push-fork",
        &format!("push {} HEAD", origin_remote),
        ConfigScope::Local,
    )
    .await?;

    // Update all branches from upstream
    set_config_value(
        Some(path),
        "alias.update-from-upstream",
        &format!(
            "!git fetch {} && git rebase {}/{}",
            upstream_remote, upstream_remote, default_branch
        ),
        ConfigScope::Local,
    )
    .await?;

    // Create PR-ready branch
    set_config_value(
        Some(path),
        "alias.pr-branch",
        &format!(
            "!f() {{ git checkout -b \"$1\" {}/{}; }}; f",
            upstream_remote, default_branch
        ),
        ConfigScope::Local,
    )
    .await?;

    // Show fork status
    set_config_value(
        Some(path),
        "alias.fork-status",
        "!echo \"=== Remotes ===\" && git remote -v && echo && echo \"=== Branch Tracking ===\" && git branch -vv",
        ConfigScope::Local,
    )
    .await?;

    info!("Fork aliases configured successfully");
    Ok(())
}

/// Get current branch name
///
/// # Arguments
/// * `path` - Repository path
///
/// # Returns
/// Current branch name
pub async fn get_current_branch(path: &Utf8Path) -> Result<String> {
    let output = Command::new("git")
        .current_dir(path)
        .args(["branch", "--show-current"])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::git_operation(format!(
            "Failed to get current branch: {}",
            stderr
        )));
    }

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();

    Ok(branch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::init::{init_repository, InitOptions};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_configure_user() {
        let temp_dir = TempDir::new().unwrap();
        let path = Utf8Path::from_path(temp_dir.path()).unwrap();

        // Initialize repo first
        let options = InitOptions::default();
        let git_config = GitWorkflowConfig::default();
        init_repository(path, &options, &git_config).await.unwrap();

        // Configure user
        let result = configure_user(
            Some(path),
            Some("Test User"),
            Some("test@example.com"),
            ConfigScope::Local,
        )
        .await;

        assert!(result.is_ok());

        // Verify configuration
        let name = get_config_value(Some(path), "user.name", Some(ConfigScope::Local))
            .await
            .unwrap();
        assert_eq!(name, Some("Test User".to_string()));

        let email = get_config_value(Some(path), "user.email", Some(ConfigScope::Local))
            .await
            .unwrap();
        assert_eq!(email, Some("test@example.com".to_string()));
    }

    #[tokio::test]
    async fn test_setup_fork_aliases() {
        let temp_dir = TempDir::new().unwrap();
        let path = Utf8Path::from_path(temp_dir.path()).unwrap();

        // Initialize repo first
        let options = InitOptions::default();
        let git_config = GitWorkflowConfig::default();
        init_repository(path, &options, &git_config).await.unwrap();

        // Setup aliases
        let result = setup_fork_aliases(path).await;
        assert!(result.is_ok());

        // Verify one of the aliases exists
        let alias = get_config_value(Some(path), "alias.sync-upstream", Some(ConfigScope::Local))
            .await
            .unwrap();
        assert!(alias.is_some());
        // Verify it uses the default branch name
        assert!(alias.unwrap().contains("main"));
    }

    #[tokio::test]
    async fn test_setup_fork_aliases_with_custom_config() {
        let temp_dir = TempDir::new().unwrap();
        let path = Utf8Path::from_path(temp_dir.path()).unwrap();

        // Initialize repo first
        let options = InitOptions::default();
        let git_config = GitWorkflowConfig::default();
        init_repository(path, &options, &git_config).await.unwrap();

        // Custom config
        let custom_config = GitWorkflowConfig {
            default_branch: "develop".to_string(),
            upstream_remote: "source".to_string(),
            origin_remote: "fork".to_string(),
            ..Default::default()
        };

        // Setup aliases with custom config
        let result = setup_fork_aliases_with_config(path, &custom_config).await;
        assert!(result.is_ok());

        // Verify alias uses custom branch name
        let alias = get_config_value(Some(path), "alias.sync-upstream", Some(ConfigScope::Local))
            .await
            .unwrap();
        assert!(alias.is_some());
        let alias_value = alias.unwrap();
        assert!(alias_value.contains("develop"));
        assert!(alias_value.contains("source"));

        // Verify push-fork alias uses custom origin
        let push_alias = get_config_value(Some(path), "alias.push-fork", Some(ConfigScope::Local))
            .await
            .unwrap();
        assert!(push_alias.is_some());
        assert!(push_alias.unwrap().contains("fork"));
    }

    #[tokio::test]
    async fn test_get_current_branch() {
        let temp_dir = TempDir::new().unwrap();
        let path = Utf8Path::from_path(temp_dir.path()).unwrap();

        // Initialize repo first
        let options = InitOptions::default();
        let git_config = GitWorkflowConfig::default();
        init_repository(path, &options, &git_config).await.unwrap();

        // Get current branch (should be "main" from default options)
        let branch = get_current_branch(path).await.unwrap();
        assert_eq!(branch, "main");
    }

    #[tokio::test]
    async fn test_branch_exists() {
        let temp_dir = TempDir::new().unwrap();
        let path = Utf8Path::from_path(temp_dir.path()).unwrap();

        // Initialize repo first
        let options = InitOptions::default();
        let git_config = GitWorkflowConfig::default();
        init_repository(path, &options, &git_config).await.unwrap();

        // Main branch should exist
        let exists = branch_exists(path, "main").await.unwrap();
        assert!(exists);

        // Non-existent branch should not exist
        let exists = branch_exists(path, "nonexistent").await.unwrap();
        assert!(!exists);
    }

    #[tokio::test]
    async fn test_detect_main_branch() {
        let temp_dir = TempDir::new().unwrap();
        let path = Utf8Path::from_path(temp_dir.path()).unwrap();

        // Initialize repo first with "main" branch
        let options = InitOptions::default();
        let git_config = GitWorkflowConfig::default();
        init_repository(path, &options, &git_config).await.unwrap();

        // Should detect "main" as the main branch
        let detected = detect_main_branch(path, &git_config).await.unwrap();
        assert_eq!(detected, "main");
    }

    #[tokio::test]
    async fn test_detect_main_branch_fallback() {
        let temp_dir = TempDir::new().unwrap();
        let path = Utf8Path::from_path(temp_dir.path()).unwrap();

        // Initialize repo first with custom branch
        let options = InitOptions {
            default_branch: Some("develop".to_string()),
            ..Default::default()
        };
        let git_config = GitWorkflowConfig::default();
        init_repository(path, &options, &git_config).await.unwrap();

        // Neither "main" nor "master" exists, should fall back to default
        let detected = detect_main_branch(path, &git_config).await.unwrap();
        assert_eq!(detected, "main"); // Falls back to config default
    }
}
