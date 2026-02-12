//! Git remote management operations

use crate::error::{Error, Result};
use camino::Utf8Path;
use sindri_core::types::GitWorkflowConfig;
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Add a remote to a repository
///
/// # Arguments
/// * `path` - Repository path
/// * `name` - Remote name (e.g., "origin", "upstream")
/// * `url` - Remote URL
///
/// # Returns
/// Ok(()) on success
///
/// # Errors
/// Returns error if remote already exists or git command fails
pub async fn add_remote(path: &Utf8Path, name: &str, url: &str) -> Result<()> {
    info!("Adding remote '{}': {}", name, url);

    // Check if remote already exists
    if remote_exists(path, name).await? {
        return Err(Error::remote_exists(name));
    }

    let output = Command::new("git")
        .current_dir(path)
        .args(["remote", "add", name, url])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::git_operation(format!(
            "Failed to add remote '{}': {}",
            name, stderr
        )));
    }

    info!("Remote '{}' added successfully", name);
    Ok(())
}

/// Remove a remote from a repository
///
/// # Arguments
/// * `path` - Repository path
/// * `name` - Remote name to remove
///
/// # Returns
/// Ok(()) on success
pub async fn remove_remote(path: &Utf8Path, name: &str) -> Result<()> {
    info!("Removing remote: {}", name);

    // Check if remote exists
    if !remote_exists(path, name).await? {
        return Err(Error::remote_not_found(name));
    }

    let output = Command::new("git")
        .current_dir(path)
        .args(["remote", "remove", name])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::git_operation(format!(
            "Failed to remove remote '{}': {}",
            name, stderr
        )));
    }

    info!("Remote '{}' removed successfully", name);
    Ok(())
}

/// Get the URL of a remote
///
/// # Arguments
/// * `path` - Repository path
/// * `name` - Remote name
///
/// # Returns
/// Remote URL if found
pub async fn get_remote_url(path: &Utf8Path, name: &str) -> Result<Option<String>> {
    debug!("Getting URL for remote: {}", name);

    let output = Command::new("git")
        .current_dir(path)
        .args(["remote", "get-url", name])
        .output()
        .await?;

    if !output.status.success() {
        // Remote doesn't exist
        return Ok(None);
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();

    Ok(Some(url))
}

/// Check if a remote exists
///
/// # Arguments
/// * `path` - Repository path
/// * `name` - Remote name to check
///
/// # Returns
/// true if remote exists, false otherwise
pub async fn remote_exists(path: &Utf8Path, name: &str) -> Result<bool> {
    let url = get_remote_url(path, name).await?;
    Ok(url.is_some())
}

/// List all remotes in a repository
///
/// # Arguments
/// * `path` - Repository path
///
/// # Returns
/// Vector of (name, url) tuples
pub async fn list_remotes(path: &Utf8Path) -> Result<Vec<(String, String)>> {
    debug!("Listing remotes");

    let output = Command::new("git")
        .current_dir(path)
        .args(["remote", "-v"])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::git_operation(format!(
            "Failed to list remotes: {}",
            stderr
        )));
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut remotes = Vec::new();

    for line in output_str.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let name = parts[0].to_string();
            let url = parts[1].to_string();
            // Only add fetch URLs (avoid duplicates)
            if parts.len() < 3 || parts[2] == "(fetch)" {
                remotes.push((name, url));
            }
        }
    }

    Ok(remotes)
}

/// Setup fork remotes (origin and upstream)
///
/// # Arguments
/// * `path` - Repository path
/// * `fork_url` - URL of the forked repository (becomes 'origin')
/// * `upstream_url` - URL of the upstream repository
///
/// # Returns
/// Ok(()) on success
///
/// # Errors
/// Returns error if remotes already exist or setup fails
pub async fn setup_fork_remotes(path: &Utf8Path, fork_url: &str, upstream_url: &str) -> Result<()> {
    let git_config = GitWorkflowConfig::default();
    setup_fork_remotes_with_config(path, fork_url, upstream_url, &git_config).await
}

/// Setup fork remotes with configurable remote names
///
/// # Arguments
/// * `path` - Repository path
/// * `fork_url` - URL of the forked repository (becomes origin)
/// * `upstream_url` - URL of the upstream repository
/// * `git_config` - Git workflow configuration for remote names
///
/// # Returns
/// Ok(()) on success
///
/// # Errors
/// Returns error if remotes already exist or setup fails
pub async fn setup_fork_remotes_with_config(
    path: &Utf8Path,
    fork_url: &str,
    upstream_url: &str,
    git_config: &GitWorkflowConfig,
) -> Result<()> {
    info!("Setting up fork remotes");

    let upstream_remote = &git_config.upstream_remote;
    let origin_remote = &git_config.origin_remote;

    // Check if upstream exists, if not add it
    if !remote_exists(path, upstream_remote).await? {
        add_remote(path, upstream_remote, upstream_url).await?;
        info!(
            "Upstream remote '{}' configured: {}",
            upstream_remote, upstream_url
        );
    } else {
        let existing_url = get_remote_url(path, upstream_remote).await?.unwrap();
        if existing_url != upstream_url {
            warn!(
                "Upstream remote '{}' exists with different URL: {}",
                upstream_remote, existing_url
            );
        }
    }

    // Verify origin is set to fork
    if let Some(origin_url) = get_remote_url(path, origin_remote).await? {
        if origin_url != fork_url {
            info!("Updating {} to fork URL", origin_remote);
            // Update origin URL
            let output = Command::new("git")
                .current_dir(path)
                .args(["remote", "set-url", origin_remote, fork_url])
                .output()
                .await?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(Error::git_operation(format!(
                    "Failed to update {} URL: {}",
                    origin_remote, stderr
                )));
            }
        }
    } else {
        // Origin doesn't exist, add it
        add_remote(path, origin_remote, fork_url).await?;
    }

    info!("Fork remotes configured successfully");
    Ok(())
}

/// Fetch from a remote
///
/// # Arguments
/// * `path` - Repository path
/// * `remote` - Remote name to fetch from
/// * `branch` - Optional specific branch to fetch
///
/// # Returns
/// Ok(()) on success
pub async fn fetch_remote(path: &Utf8Path, remote: &str, branch: Option<&str>) -> Result<()> {
    info!("Fetching from remote: {}", remote);

    let mut cmd = Command::new("git");
    cmd.current_dir(path).args(["fetch", remote]);

    if let Some(b) = branch {
        cmd.arg(b);
    }

    let output = cmd.output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::git_operation(format!(
            "Failed to fetch from '{}': {}",
            remote, stderr
        )));
    }

    info!("Fetch from '{}' completed successfully", remote);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::init::{init_repository, InitOptions};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_add_remove_remote() {
        let temp_dir = TempDir::new().unwrap();
        let path = Utf8Path::from_path(temp_dir.path()).unwrap();

        // Initialize repo first
        let options = InitOptions::default();
        let git_config = GitWorkflowConfig::default();
        init_repository(path, &options, &git_config).await.unwrap();

        // Add a remote
        add_remote(path, "origin", "https://github.com/user/repo.git")
            .await
            .expect("add_remote should succeed");

        // Verify remote exists
        let exists = remote_exists(path, "origin").await.unwrap();
        assert!(exists);

        // Try to add same remote again should fail
        let result = add_remote(path, "origin", "https://github.com/user/repo.git").await;
        assert!(result.is_err());

        // Get remote URL
        let url = get_remote_url(path, "origin").await.unwrap();
        assert_eq!(url, Some("https://github.com/user/repo.git".to_string()));

        // Remove remote
        remove_remote(path, "origin")
            .await
            .expect("remove_remote should succeed");

        // Verify remote is gone
        let exists = remote_exists(path, "origin").await.unwrap();
        assert!(!exists);
    }

    #[tokio::test]
    async fn test_list_remotes() {
        let temp_dir = TempDir::new().unwrap();
        let path = Utf8Path::from_path(temp_dir.path()).unwrap();

        // Initialize repo first
        let options = InitOptions::default();
        let git_config = GitWorkflowConfig::default();
        init_repository(path, &options, &git_config).await.unwrap();

        // Add multiple remotes
        add_remote(path, "origin", "https://github.com/user/repo.git")
            .await
            .unwrap();
        add_remote(path, "upstream", "https://github.com/original/repo.git")
            .await
            .unwrap();

        // List remotes
        let remotes = list_remotes(path).await.unwrap();
        assert_eq!(remotes.len(), 2);
        assert!(remotes.iter().any(|(name, _)| name == "origin"));
        assert!(remotes.iter().any(|(name, _)| name == "upstream"));
    }

    #[tokio::test]
    async fn test_setup_fork_remotes() {
        let temp_dir = TempDir::new().unwrap();
        let path = Utf8Path::from_path(temp_dir.path()).unwrap();

        // Initialize repo first
        let options = InitOptions::default();
        let git_config = GitWorkflowConfig::default();
        init_repository(path, &options, &git_config).await.unwrap();

        // Setup fork remotes
        let result = setup_fork_remotes(
            path,
            "https://github.com/user/repo.git",
            "https://github.com/original/repo.git",
        )
        .await;

        result.expect("setup_fork_remotes should succeed");

        // Verify both remotes exist
        let origin_url = get_remote_url(path, "origin").await.unwrap();
        assert_eq!(
            origin_url,
            Some("https://github.com/user/repo.git".to_string())
        );

        let upstream_url = get_remote_url(path, "upstream").await.unwrap();
        assert_eq!(
            upstream_url,
            Some("https://github.com/original/repo.git".to_string())
        );
    }

    #[tokio::test]
    async fn test_setup_fork_remotes_with_custom_config() {
        let temp_dir = TempDir::new().unwrap();
        let path = Utf8Path::from_path(temp_dir.path()).unwrap();

        // Initialize repo first
        let options = InitOptions::default();
        let git_config = GitWorkflowConfig::default();
        init_repository(path, &options, &git_config).await.unwrap();

        // Custom config with different remote names
        let custom_config = GitWorkflowConfig {
            origin_remote: "fork".to_string(),
            upstream_remote: "source".to_string(),
            ..Default::default()
        };

        // Setup fork remotes with custom names
        let result = setup_fork_remotes_with_config(
            path,
            "https://github.com/user/repo.git",
            "https://github.com/original/repo.git",
            &custom_config,
        )
        .await;

        result.expect("setup_fork_remotes_with_config should succeed");

        // Verify custom-named remotes exist
        let fork_url = get_remote_url(path, "fork").await.unwrap();
        assert_eq!(
            fork_url,
            Some("https://github.com/user/repo.git".to_string())
        );

        let source_url = get_remote_url(path, "source").await.unwrap();
        assert_eq!(
            source_url,
            Some("https://github.com/original/repo.git".to_string())
        );
    }
}
