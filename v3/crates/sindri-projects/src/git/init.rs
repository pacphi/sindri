//! Repository initialization operations

use crate::error::{Error, Result};
use camino::Utf8Path;
use sindri_core::types::GitWorkflowConfig;
use tokio::process::Command;
use tracing::{debug, info};

/// Options for initializing a git repository
#[derive(Debug, Clone)]
pub struct InitOptions {
    /// Default branch name (e.g., "main") - overrides git_config if set
    pub default_branch: Option<String>,
    /// Initial commit message - overrides git_config if set
    pub initial_commit_message: Option<String>,
    /// Whether to create an initial commit
    pub create_initial_commit: bool,
}

impl Default for InitOptions {
    fn default() -> Self {
        Self {
            default_branch: None,
            initial_commit_message: None,
            create_initial_commit: true,
        }
    }
}

impl InitOptions {
    /// Create options with explicit defaults (for backward compatibility)
    pub fn with_defaults() -> Self {
        let git_config = GitWorkflowConfig::default();
        Self {
            default_branch: Some(git_config.default_branch),
            initial_commit_message: Some(git_config.initial_commit_message),
            create_initial_commit: true,
        }
    }

    /// Get the effective default branch, using git_config as fallback
    pub fn effective_branch(&self, git_config: &GitWorkflowConfig) -> String {
        self.default_branch
            .clone()
            .unwrap_or_else(|| git_config.default_branch.clone())
    }

    /// Get the effective initial commit message, using git_config as fallback
    pub fn effective_commit_message(&self, git_config: &GitWorkflowConfig) -> String {
        self.initial_commit_message
            .clone()
            .unwrap_or_else(|| git_config.initial_commit_message.clone())
    }
}

/// Initialize a new git repository
///
/// # Arguments
/// * `path` - Directory to initialize as a git repository
/// * `options` - Initialization options
/// * `git_config` - Git workflow configuration for defaults
///
/// # Returns
/// Ok(()) on success
///
/// # Errors
/// Returns error if:
/// - Directory doesn't exist
/// - Git is not installed
/// - Git init fails
/// - Initial commit creation fails
pub async fn init_repository(
    path: &Utf8Path,
    options: &InitOptions,
    git_config: &GitWorkflowConfig,
) -> Result<()> {
    info!("Initializing git repository at: {}", path);

    // Check if git is available
    check_git_available().await?;

    // Check if directory exists
    if !path.exists() {
        return Err(Error::RepoNotFound {
            path: path.to_string(),
        });
    }

    // Check if already a git repository
    if path.join(".git").exists() {
        debug!("Git repository already exists at: {}", path);
        return Ok(());
    }

    // Initialize repository
    let mut cmd = Command::new("git");
    cmd.current_dir(path).arg("init");

    // Use effective branch from options or git_config
    let branch = options.effective_branch(git_config);
    cmd.arg("--initial-branch").arg(&branch);

    debug!("Running: git init with branch {}", branch);
    let output = cmd.output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::git_operation(format!("git init failed: {}", stderr)));
    }

    info!("Repository initialized successfully");

    // Create initial commit if requested
    if options.create_initial_commit {
        let commit_message = options.effective_commit_message(git_config);
        create_initial_commit(path, &commit_message).await?;
    }

    Ok(())
}

/// Create an initial commit with .gitignore
async fn create_initial_commit(path: &Utf8Path, message: &str) -> Result<()> {
    debug!("Creating initial commit");

    // Create a basic .gitignore if it doesn't exist
    let gitignore_path = path.join(".gitignore");
    if !gitignore_path.exists() {
        let default_gitignore = include_str!("../templates/default.gitignore");
        tokio::fs::write(&gitignore_path, default_gitignore).await?;
    }

    // Add .gitignore
    let output = Command::new("git")
        .current_dir(path)
        .args(["add", ".gitignore"])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::git_operation(format!("git add failed: {}", stderr)));
    }

    // Create commit
    let output = Command::new("git")
        .current_dir(path)
        .args(["commit", "-m", message])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::git_operation(format!(
            "git commit failed: {}",
            stderr
        )));
    }

    info!("Initial commit created");
    Ok(())
}

/// Check if git is available in PATH
async fn check_git_available() -> Result<()> {
    let output = Command::new("git")
        .arg("--version")
        .output()
        .await
        .map_err(|_| Error::GitNotFound)?;

    if !output.status.success() {
        return Err(Error::GitNotFound);
    }

    Ok(())
}

/// Create a new branch and optionally check it out
///
/// # Arguments
/// * `path` - Repository path
/// * `branch_name` - Name of the branch to create
/// * `checkout` - Whether to checkout the branch after creation
///
/// # Returns
/// Ok(()) on success
pub async fn create_branch(path: &Utf8Path, branch_name: &str, checkout: bool) -> Result<()> {
    info!("Creating branch: {}", branch_name);

    // Validate branch name (basic validation)
    if branch_name.is_empty() || branch_name.contains("..") || branch_name.starts_with('-') {
        return Err(Error::invalid_branch(branch_name));
    }

    if checkout {
        // Create and checkout in one command
        let output = Command::new("git")
            .current_dir(path)
            .args(["checkout", "-b", branch_name])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("already exists") {
                return Err(Error::branch_exists(branch_name));
            }
            return Err(Error::git_operation(format!(
                "git checkout -b failed: {}",
                stderr
            )));
        }
    } else {
        // Just create the branch
        let output = Command::new("git")
            .current_dir(path)
            .args(["branch", branch_name])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("already exists") {
                return Err(Error::branch_exists(branch_name));
            }
            return Err(Error::git_operation(format!(
                "git branch failed: {}",
                stderr
            )));
        }
    }

    info!("Branch '{}' created successfully", branch_name);
    Ok(())
}

/// Checkout an existing branch
///
/// # Arguments
/// * `path` - Repository path
/// * `branch_name` - Name of the branch to checkout
///
/// # Returns
/// Ok(()) on success
pub async fn checkout_branch(path: &Utf8Path, branch_name: &str) -> Result<()> {
    info!("Checking out branch: {}", branch_name);

    let output = Command::new("git")
        .current_dir(path)
        .args(["checkout", branch_name])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::git_operation(format!(
            "git checkout failed: {}",
            stderr
        )));
    }

    info!("Checked out branch: {}", branch_name);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_init_repository() {
        let temp_dir = TempDir::new().unwrap();
        let path = Utf8Path::from_path(temp_dir.path()).unwrap();

        let options = InitOptions::default();
        let git_config = GitWorkflowConfig::default();
        let result = init_repository(path, &options, &git_config).await;

        assert!(result.is_ok());
        assert!(path.join(".git").exists());
        assert!(path.join(".gitignore").exists());
    }

    #[tokio::test]
    async fn test_init_repository_with_custom_branch() {
        let temp_dir = TempDir::new().unwrap();
        let path = Utf8Path::from_path(temp_dir.path()).unwrap();

        let options = InitOptions {
            default_branch: Some("develop".to_string()),
            ..Default::default()
        };
        let git_config = GitWorkflowConfig::default();
        let result = init_repository(path, &options, &git_config).await;

        assert!(result.is_ok());
        assert!(path.join(".git").exists());
    }

    #[tokio::test]
    async fn test_init_repository_uses_config_defaults() {
        let temp_dir = TempDir::new().unwrap();
        let path = Utf8Path::from_path(temp_dir.path()).unwrap();

        let options = InitOptions::default();
        let mut git_config = GitWorkflowConfig::default();
        git_config.default_branch = "trunk".to_string();
        git_config.initial_commit_message = "Initial setup".to_string();

        let result = init_repository(path, &options, &git_config).await;

        assert!(result.is_ok());
        assert!(path.join(".git").exists());
    }

    #[tokio::test]
    async fn test_create_branch() {
        let temp_dir = TempDir::new().unwrap();
        let path = Utf8Path::from_path(temp_dir.path()).unwrap();

        // Initialize repo first
        let options = InitOptions::default();
        let git_config = GitWorkflowConfig::default();
        init_repository(path, &options, &git_config).await.unwrap();

        // Create a feature branch
        let result = create_branch(path, "feature/test", false).await;
        assert!(result.is_ok());

        // Try to create the same branch again should fail
        let result = create_branch(path, "feature/test", false).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invalid_branch_name() {
        let temp_dir = TempDir::new().unwrap();
        let path = Utf8Path::from_path(temp_dir.path()).unwrap();

        let result = create_branch(path, "", false).await;
        assert!(result.is_err());

        let result = create_branch(path, "branch..invalid", false).await;
        assert!(result.is_err());
    }
}
