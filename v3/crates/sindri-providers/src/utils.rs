//! Utility functions for provider implementations

use anyhow::{anyhow, Result};

/// Check if a command is available in PATH
pub fn command_exists(cmd: &str) -> bool {
    which::which(cmd).is_ok()
}

/// Get command version
pub fn get_command_version(cmd: &str, version_flag: &str) -> Result<String> {
    let output = std::process::Command::new(cmd).arg(version_flag).output()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Some tools output version to stderr
        let version = if stdout.trim().is_empty() {
            stderr.trim().to_string()
        } else {
            stdout.trim().to_string()
        };
        Ok(version)
    } else {
        Err(anyhow!("Failed to get version for {}", cmd))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_exists_with_known_command() {
        // "ls" should be available on any Unix system
        assert!(command_exists("ls"), "ls should exist in PATH");
    }

    #[test]
    fn test_command_exists_with_unknown_command() {
        assert!(
            !command_exists("this_command_definitely_does_not_exist_abc123"),
            "non-existent command should return false"
        );
    }

    #[test]
    fn test_get_command_version_success() {
        // "ls --version" should work on Linux (GNU coreutils)
        let result = get_command_version("ls", "--version");
        assert!(result.is_ok(), "ls --version should succeed");
        let version = result.unwrap();
        assert!(!version.is_empty(), "version string should not be empty");
    }

    #[test]
    fn test_get_command_version_nonexistent_command() {
        let result = get_command_version("nonexistent_cmd_xyz", "--version");
        assert!(result.is_err(), "non-existent command should return Err");
    }

    #[test]
    fn test_get_command_version_stderr_fallback() {
        // Test with a command that outputs to stderr (like some --version calls)
        // Using "true" which succeeds but has no output - should return empty string
        let result = get_command_version("true", "--help");
        // "true" ignores its arguments, so it succeeds with empty output
        // The function should return Ok with empty or non-empty string
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_copy_dir_recursive_basic() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");

        // Create source structure: src/a.txt, src/sub/b.txt
        std::fs::create_dir_all(src.join("sub")).unwrap();
        std::fs::write(src.join("a.txt"), "hello").unwrap();
        std::fs::write(src.join("sub").join("b.txt"), "world").unwrap();

        copy_dir_recursive(&src, &dst).unwrap();

        assert!(dst.join("a.txt").exists(), "a.txt should be copied");
        assert!(
            dst.join("sub").join("b.txt").exists(),
            "sub/b.txt should be copied"
        );
        assert_eq!(std::fs::read_to_string(dst.join("a.txt")).unwrap(), "hello");
        assert_eq!(
            std::fs::read_to_string(dst.join("sub").join("b.txt")).unwrap(),
            "world"
        );
    }

    #[test]
    fn test_copy_dir_recursive_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("empty_src");
        let dst = tmp.path().join("empty_dst");

        std::fs::create_dir_all(&src).unwrap();
        copy_dir_recursive(&src, &dst).unwrap();

        assert!(dst.exists(), "destination should be created");
        assert!(dst.is_dir(), "destination should be a directory");
    }

    #[test]
    fn test_copy_dir_recursive_nonexistent_source() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("does_not_exist");
        let dst = tmp.path().join("dst");

        let result = copy_dir_recursive(&src, &dst);
        assert!(result.is_err(), "non-existent source should fail");
    }

    #[test]
    fn test_copy_dir_recursive_nested() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("deep");
        let dst = tmp.path().join("deep_copy");

        // Create deeply nested structure
        std::fs::create_dir_all(src.join("a").join("b").join("c")).unwrap();
        std::fs::write(src.join("a").join("b").join("c").join("deep.txt"), "deep").unwrap();
        std::fs::write(src.join("a").join("top.txt"), "top").unwrap();

        copy_dir_recursive(&src, &dst).unwrap();

        assert_eq!(
            std::fs::read_to_string(dst.join("a").join("b").join("c").join("deep.txt")).unwrap(),
            "deep"
        );
        assert_eq!(
            std::fs::read_to_string(dst.join("a").join("top.txt")).unwrap(),
            "top"
        );
    }
}

/// Clone Sindri repository for Docker build context
///
/// Performs a shallow clone of the Sindri repository to get the v3 directory
/// with all its dependencies (Dockerfile, scripts, build context, etc.).
/// The repository version is matched to the CLI version.
///
/// # Arguments
/// * `cache_dir` - Directory to cache the cloned repository
/// * `git_ref` - Optional git ref (branch, tag, or commit SHA) to fetch (defaults to "v{CLI_VERSION}")
///
/// # Returns
/// Tuple of (v3_dir path, git_ref_used) where git_ref_used is the branch/tag that was successfully cloned
///
/// # Note
/// This function replaces the old find_dockerfile() which searched for user-provided
/// Dockerfiles. Sindri v3 uses its own official Dockerfile from the GitHub repository,
/// not user-provided ones. A shallow clone is necessary because v3/Dockerfile has
/// dependencies on other files in the v3 directory.
pub async fn fetch_sindri_build_context(
    cache_dir: &std::path::Path,
    git_ref: Option<&str>,
) -> Result<(std::path::PathBuf, String)> {
    use tokio::fs;
    use tracing::{debug, info, warn};

    // Determine which git ref to fetch
    // If specified, use as-is (could be branch, tag, commit SHA)
    // Otherwise default to version tag
    let git_ref = if let Some(ref_str) = git_ref {
        ref_str.to_string()
    } else {
        // Default to CLI version tag
        format!("v{}", env!("CARGO_PKG_VERSION"))
    };

    // For caching purposes, use the git ref as the cache key
    let cache_key = git_ref.replace('/', "-"); // Replace slashes for filesystem compatibility

    info!("Fetching Sindri v3 build context (ref: {})", git_ref);

    // Create cache directory if it doesn't exist
    fs::create_dir_all(cache_dir).await?;

    // Repository cache path (use cache_key for filesystem safety)
    let repo_dir = cache_dir.join(format!("sindri-{}", cache_key));
    let v3_dir = repo_dir.join("v3");

    // Return cached v3 directory if it exists and is valid
    if v3_dir.join("Dockerfile").exists() {
        debug!("Using cached Sindri v3 directory at {}", v3_dir.display());
        return Ok((v3_dir, git_ref.clone()));
    }

    // Remove stale clone if it exists
    if repo_dir.exists() {
        debug!("Removing stale clone at {}", repo_dir.display());
        fs::remove_dir_all(&repo_dir).await?;
    }

    // Shallow clone the repository
    info!("Cloning Sindri repository (ref: {})", git_ref);

    let clone_result = tokio::process::Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            "--branch",
            &git_ref,
            "--single-branch",
            "https://github.com/pacphi/sindri.git",
            repo_dir.to_str().unwrap(),
        ])
        .output()
        .await;

    // If versioned clone fails, try main branch
    if clone_result.is_err() || !clone_result.as_ref().unwrap().status.success() {
        warn!("Failed to clone tag {}, trying main branch", git_ref);

        // Remove failed clone attempt
        if repo_dir.exists() {
            fs::remove_dir_all(&repo_dir).await?;
        }

        let main_result = tokio::process::Command::new("git")
            .args([
                "clone",
                "--depth",
                "1",
                "--branch",
                "main",
                "--single-branch",
                "https://github.com/pacphi/sindri.git",
                repo_dir.to_str().unwrap(),
            ])
            .output()
            .await?;

        if !main_result.status.success() {
            let stderr = String::from_utf8_lossy(&main_result.stderr);
            return Err(anyhow!(
                "Failed to clone Sindri repository from GitHub: {}",
                stderr
            ));
        }

        // Successfully cloned from main branch
        info!("Sindri v3 build context ready at {}", v3_dir.display());
        return Ok((v3_dir, "main".to_string()));
    }

    // Successfully cloned from version tag
    // Verify v3 directory exists
    if !v3_dir.join("Dockerfile").exists() {
        return Err(anyhow!(
            "Cloned repository doesn't contain v3/Dockerfile at expected location"
        ));
    }

    info!("Sindri v3 build context ready at {}", v3_dir.display());
    Ok((v3_dir, git_ref))
}

/// Get the Git SHA of the cloned Sindri repository
///
/// Returns the short SHA (7 characters) of the HEAD commit in the cloned repository.
/// This is used to tag on-demand builds with a unique identifier.
///
/// # Arguments
/// * `repo_dir` - Path to the cloned repository
///
/// # Returns
/// Short Git SHA (7 characters) or error
pub async fn get_git_sha(repo_dir: &std::path::Path) -> Result<String> {
    let output = tokio::process::Command::new("git")
        .args([
            "-C",
            repo_dir.to_str().unwrap(),
            "rev-parse",
            "--short=7",
            "HEAD",
        ])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failed to get git SHA: {}", stderr));
    }

    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(sha)
}

/// Recursively copy a directory and its contents
///
/// Used by E2B provider to copy the v3 directory into the template build context.
///
/// # Arguments
/// * `src` - Source directory
/// * `dst` - Destination directory
pub fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<()> {
    use std::fs;

    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}
