//! Repository cloning and forking operations

use crate::error::{Error, Result};
use crate::git::config::{configure_user, setup_fork_aliases, ConfigScope};
use crate::git::init::create_branch;
use camino::{Utf8Path, Utf8PathBuf};
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Options for cloning a repository
#[derive(Debug, Clone, Default)]
pub struct CloneOptions {
    /// Shallow clone with specified depth
    pub depth: Option<u32>,
    /// Branch to checkout after clone
    pub branch: Option<String>,
    /// Feature branch to create after clone
    pub feature_branch: Option<String>,
    /// Git user name for local config
    pub git_name: Option<String>,
    /// Git user email for local config
    pub git_email: Option<String>,
}

/// Options for forking a repository
#[derive(Debug, Clone)]
pub struct ForkOptions {
    /// Branch to checkout after fork
    pub branch: Option<String>,
    /// Feature branch to create after fork
    pub feature_branch: Option<String>,
    /// Git user name for local config
    pub git_name: Option<String>,
    /// Git user email for local config
    pub git_email: Option<String>,
    /// Setup fork aliases
    pub setup_aliases: bool,
}

impl Default for ForkOptions {
    fn default() -> Self {
        Self {
            branch: None,
            feature_branch: None,
            git_name: None,
            git_email: None,
            setup_aliases: true,
        }
    }
}

/// Clone a repository
///
/// # Arguments
/// * `url` - Repository URL to clone
/// * `destination` - Destination directory path
/// * `options` - Clone options
///
/// # Returns
/// Path to the cloned repository
///
/// # Errors
/// Returns error if:
/// - Invalid repository URL
/// - Destination already exists
/// - Clone operation fails
pub async fn clone_repository(
    url: &str,
    destination: &Utf8Path,
    options: &CloneOptions,
) -> Result<Utf8PathBuf> {
    info!("Cloning repository: {} -> {}", url, destination);

    // Validate URL
    if !is_valid_repo_url(url) {
        return Err(Error::invalid_repo_url(url));
    }

    // Check if destination already exists
    if destination.exists() {
        return Err(Error::repo_exists(destination.as_str()));
    }

    // Build clone command
    let mut cmd = Command::new("git");
    cmd.arg("clone");

    if let Some(depth) = options.depth {
        cmd.arg("--depth").arg(depth.to_string());
    }

    if let Some(branch) = &options.branch {
        cmd.arg("--branch").arg(branch);
    }

    cmd.arg(url).arg(destination.as_str());

    debug!("Running: git clone");
    let output = cmd.output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::clone_failed(stderr));
    }

    info!("Repository cloned successfully");

    // Apply git config if specified
    if options.git_name.is_some() || options.git_email.is_some() {
        configure_user(
            Some(destination),
            options.git_name.as_deref(),
            options.git_email.as_deref(),
            ConfigScope::Local,
        )
        .await?;
    }

    // Create feature branch if specified
    if let Some(feature_branch) = &options.feature_branch {
        info!("Creating feature branch: {}", feature_branch);
        create_branch(destination, feature_branch, true).await?;
    }

    Ok(destination.to_path_buf())
}

/// Fork a repository using GitHub CLI
///
/// # Arguments
/// * `url` - Repository URL to fork
/// * `destination` - Destination directory path
/// * `options` - Fork options
///
/// # Returns
/// Path to the forked repository
///
/// # Errors
/// Returns error if:
/// - GitHub CLI is not installed
/// - GitHub CLI is not authenticated
/// - Fork operation fails
pub async fn fork_repository(
    url: &str,
    destination: &Utf8Path,
    options: &ForkOptions,
) -> Result<Utf8PathBuf> {
    info!("Forking repository: {} -> {}", url, destination);

    // Check if gh CLI is available
    check_gh_available().await?;

    // Check if gh is authenticated
    check_gh_authenticated().await?;

    // Validate URL
    if !is_valid_repo_url(url) {
        return Err(Error::invalid_repo_url(url));
    }

    // Check if destination already exists
    if destination.exists() {
        return Err(Error::repo_exists(destination.as_str()));
    }

    // Get parent directory for fork operation
    let parent_dir = destination
        .parent()
        .ok_or_else(|| Error::invalid_repo_url("Invalid destination path"))?;

    // Ensure parent directory exists
    tokio::fs::create_dir_all(parent_dir).await?;

    // Fork using gh CLI
    debug!("Running: gh repo fork --clone");
    let output = Command::new("gh")
        .current_dir(parent_dir)
        .args(["repo", "fork", url, "--clone"])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::fork_failed(stderr));
    }

    info!("Repository forked successfully");

    // The fork is cloned to a directory named after the repo
    // We need to find it and potentially rename it
    let repo_name = extract_repo_name(url)?;
    let cloned_path = parent_dir.join(&repo_name);

    // If destination name differs from repo name, rename
    if cloned_path != destination {
        tokio::fs::rename(&cloned_path, destination).await?;
    }

    // gh repo fork already sets up origin and upstream
    // Verify the setup
    verify_fork_remotes(destination).await?;

    // Setup fork aliases if requested
    if options.setup_aliases {
        setup_fork_aliases(destination).await?;
    }

    // Checkout specific branch if requested
    if let Some(branch) = &options.branch {
        checkout_fork_branch(destination, branch).await?;
    }

    // Apply git config if specified
    if options.git_name.is_some() || options.git_email.is_some() {
        configure_user(
            Some(destination),
            options.git_name.as_deref(),
            options.git_email.as_deref(),
            ConfigScope::Local,
        )
        .await?;
    }

    // Create feature branch if specified
    if let Some(feature_branch) = &options.feature_branch {
        info!("Creating feature branch: {}", feature_branch);
        create_branch(destination, feature_branch, true).await?;
    }

    Ok(destination.to_path_buf())
}

/// Check if GitHub CLI is available
async fn check_gh_available() -> Result<()> {
    let result = Command::new("gh").arg("--version").output().await;

    match result {
        Ok(output) if output.status.success() => Ok(()),
        _ => Err(Error::GhNotFound),
    }
}

/// Check if GitHub CLI is authenticated
async fn check_gh_authenticated() -> Result<()> {
    let output = Command::new("gh").args(["auth", "status"]).output().await?;

    if !output.status.success() {
        return Err(Error::GhNotAuthenticated);
    }

    Ok(())
}

/// Verify fork remotes are properly configured
async fn verify_fork_remotes(path: &Utf8Path) -> Result<()> {
    use crate::git::remote::get_remote_url;

    let upstream_url = get_remote_url(path, "upstream").await?;

    if let Some(url) = upstream_url {
        info!("Fork configured with upstream: {}", url);
    } else {
        warn!("Upstream remote not configured. Fork may not have been set up correctly.");
    }

    Ok(())
}

/// Checkout a branch in a forked repository
async fn checkout_fork_branch(path: &Utf8Path, branch: &str) -> Result<()> {
    use crate::git::init::checkout_branch;
    use crate::git::remote::fetch_remote;

    info!("Checking out branch: {}", branch);

    // Try to checkout locally first
    let result = checkout_branch(path, branch).await;

    if result.is_err() {
        // Branch not found locally, try to fetch from upstream
        warn!(
            "Branch {} not found locally, trying to fetch from upstream",
            branch
        );

        // Fetch from upstream
        if let Err(e) = fetch_remote(path, "upstream", Some(branch)).await {
            debug!("Failed to fetch from upstream: {}", e);
        }

        // Create local branch tracking upstream
        let output = Command::new("git")
            .current_dir(path)
            .args(["checkout", "-b", branch, &format!("upstream/{}", branch)])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::git_operation(format!(
                "Could not checkout branch {}: {}",
                branch, stderr
            )));
        }
    }

    info!("Checked out branch: {}", branch);
    Ok(())
}

/// Validate if a string is a valid repository URL
fn is_valid_repo_url(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("git@") || url.starts_with("http://")
}

/// Extract repository name from URL
///
/// # Examples
/// - https://github.com/user/repo.git -> repo
/// - git@github.com:user/repo.git -> repo
fn extract_repo_name(url: &str) -> Result<String> {
    let url = url.trim_end_matches('/');
    let name = url
        .rsplit('/')
        .next()
        .ok_or_else(|| Error::invalid_repo_url(url))?
        .trim_end_matches(".git");

    if name.is_empty() {
        return Err(Error::invalid_repo_url(url));
    }

    Ok(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_repo_url() {
        assert!(is_valid_repo_url("https://github.com/user/repo.git"));
        assert!(is_valid_repo_url("git@github.com:user/repo.git"));
        assert!(is_valid_repo_url("http://example.com/repo.git"));
        assert!(!is_valid_repo_url("invalid-url"));
        assert!(!is_valid_repo_url(""));
    }

    #[test]
    fn test_extract_repo_name() {
        assert_eq!(
            extract_repo_name("https://github.com/user/repo.git").unwrap(),
            "repo"
        );
        assert_eq!(
            extract_repo_name("git@github.com:user/my-project.git").unwrap(),
            "my-project"
        );
        assert_eq!(
            extract_repo_name("https://github.com/user/repo").unwrap(),
            "repo"
        );
    }

    #[tokio::test]
    async fn test_clone_options_default() {
        let options = CloneOptions::default();
        assert!(options.depth.is_none());
        assert!(options.branch.is_none());
        assert!(options.feature_branch.is_none());
    }

    #[tokio::test]
    async fn test_fork_options_default() {
        let options = ForkOptions::default();
        assert!(options.setup_aliases);
        assert!(options.branch.is_none());
    }
}
