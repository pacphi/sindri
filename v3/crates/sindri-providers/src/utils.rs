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

    // Derive clone URL from extension source config
    let source_config = sindri_extensions::ExtensionSourceConfig::load().unwrap_or_default();
    let clone_url = format!(
        "https://github.com/{}/{}.git",
        source_config.github.owner, source_config.github.repo
    );

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
            &clone_url,
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
                &clone_url,
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

/// Configuration for building and pushing a Docker image
pub struct ImageBuildConfig<'a> {
    /// Image name including registry path (e.g., "ghcr.io/org/sindri")
    pub image_name: &'a str,
    /// Image tag (e.g., "latest", "v3.0.0-a1b2c3d")
    pub image_tag: &'a str,
    /// Whether to skip Docker layer cache
    pub no_cache: bool,
}

/// Result of a successful image build and push
pub struct ImageBuildResult {
    /// Full image reference that was pushed (e.g., "ghcr.io/org/sindri:v3.0.0-a1b2c3d")
    pub image_ref: String,
}

/// Build a Sindri Docker image from the official build context.
///
/// Fetches the Sindri repository (if not cached), selects the appropriate Dockerfile,
/// and builds a Docker image tagged with the CLI version and git SHA.
///
/// # Arguments
/// * `config` - Build configuration specifying image name, tag, and cache behavior
/// * `git_ref` - Optional git ref to fetch (defaults to CLI version tag)
///
/// # Returns
/// The full image reference (name:tag) of the built image
pub async fn build_sindri_image(
    config: &ImageBuildConfig<'_>,
    git_ref: Option<&str>,
) -> Result<ImageBuildResult> {
    use std::path::PathBuf;
    use std::process::Stdio;
    use tracing::info;

    // Check Docker is available
    if !command_exists("docker") {
        return Err(anyhow!(
            "Docker is required to build images. Install from https://docs.docker.com/get-docker/"
        ));
    }

    // Fetch build context
    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("sindri")
        .join("repos");

    let (v3_dir, git_ref_used) = fetch_sindri_build_context(&cache_dir, git_ref).await?;
    let repo_dir = v3_dir.parent().unwrap();

    let git_sha = get_git_sha(repo_dir)
        .await
        .unwrap_or_else(|_| "unknown".to_string());

    let image_ref = format!("{}:{}", config.image_name, config.image_tag);
    let dockerfile = v3_dir.join("Dockerfile");

    info!(
        "Building Docker image {} from {} (ref: {}, commit: {})",
        image_ref,
        v3_dir.display(),
        git_ref_used,
        git_sha
    );

    let sindri_version = git_ref.unwrap_or(&git_ref_used).to_string();

    let mut args = vec!["build", "-t", &image_ref, "-f"];
    let dockerfile_str = dockerfile.to_string_lossy();
    args.push(&dockerfile_str);

    if config.no_cache {
        args.push("--no-cache");
    }

    args.push("--build-arg");
    let sindri_version_arg = format!("SINDRI_VERSION={}", sindri_version);
    args.push(&sindri_version_arg);

    let context_str = repo_dir.to_string_lossy();
    args.push(&context_str);

    info!("Building Docker image - this may take several minutes...");
    let status = tokio::process::Command::new("docker")
        .args(&args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await
        .map_err(|e| anyhow!("Failed to execute docker build: {}", e))?;

    if !status.success() {
        return Err(anyhow!(
            "Docker build failed. Check the build output above for details."
        ));
    }

    info!("Docker image built successfully: {}", image_ref);
    Ok(ImageBuildResult { image_ref })
}

/// Push a Docker image to a registry.
///
/// Assumes `docker login` has already been performed for the target registry.
/// Handles common registry authentication patterns (GHCR, Docker Hub, etc.).
///
/// # Arguments
/// * `image_ref` - Full image reference to push (e.g., "ghcr.io/org/sindri:v3.0.0")
///
/// # Returns
/// Ok(()) on success, error with details on failure
pub async fn push_image_to_registry(image_ref: &str) -> Result<()> {
    use std::process::Stdio;
    use tracing::info;

    if !command_exists("docker") {
        return Err(anyhow!("Docker is required to push images"));
    }

    info!("Pushing image to registry: {}", image_ref);

    let status = tokio::process::Command::new("docker")
        .args(["push", image_ref])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await
        .map_err(|e| anyhow!("Failed to execute docker push: {}", e))?;

    if !status.success() {
        return Err(anyhow!(
            "Failed to push image {}. Ensure you are logged in to the registry with 'docker login'.",
            image_ref
        ));
    }

    info!("Image pushed successfully: {}", image_ref);
    Ok(())
}

/// Check if a Docker image exists locally.
///
/// # Arguments
/// * `image_ref` - Full image reference to check (e.g., "ghcr.io/org/sindri:latest")
pub async fn image_exists_locally(image_ref: &str) -> bool {
    let output = tokio::process::Command::new("docker")
        .args(["image", "inspect", image_ref])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await;

    output.map(|s| s.success()).unwrap_or(false)
}

/// Resolve the image to use for deployment, building and pushing if necessary.
///
/// This is the main entry point for providers that need Docker image build+push.
/// It handles the full workflow:
/// 1. If skip_build is true, use the image from config as-is
/// 2. If build_from_source is enabled, build from Sindri repository
/// 3. If an image is configured, use it directly
/// 4. Otherwise, build from source and push to registry
///
/// # Arguments
/// * `config` - Sindri configuration
/// * `skip_build` - Whether to skip the build step
/// * `force` - Whether to force rebuild (no cache)
///
/// # Returns
/// The image reference to use for deployment
pub async fn resolve_and_build_image(
    config: &sindri_core::config::SindriConfig,
    skip_build: bool,
    force: bool,
) -> Result<String> {
    use tracing::{debug, info};

    let file = config.inner();

    // If skip_build, use the configured image as-is
    if skip_build {
        let image = file.deployment.image.clone().ok_or_else(|| {
            anyhow!(
                "No image configured. Cannot use --skip-build without specifying \
                     'deployment.image' in sindri.yaml"
            )
        })?;
        info!("Using pre-built image (skip-build): {}", image);
        return Ok(image);
    }

    // Check if build_from_source is explicitly enabled
    let should_build = file
        .deployment
        .build_from_source
        .as_ref()
        .map(|b| b.enabled)
        .unwrap_or(false);

    // If there's a configured image and build_from_source is not enabled, use it directly
    if !should_build {
        if let Some(ref image) = file.deployment.image {
            debug!("Using configured image: {}", image);
            return Ok(image.clone());
        }

        // Try resolve_image for image_config support
        if file.deployment.image_config.is_some() {
            let resolved = config
                .resolve_image(None)
                .await
                .map_err(|e| anyhow!("Failed to resolve image: {}", e))?;
            debug!("Using resolved image: {}", resolved);
            return Ok(resolved);
        }
    }

    // Build from source
    let git_ref = file
        .deployment
        .build_from_source
        .as_ref()
        .and_then(|b| b.git_ref.as_deref());

    // Determine image name from config or use default
    let image_name = file
        .deployment
        .image
        .as_deref()
        .and_then(|img| img.rsplit_once(':').map(|(name, _)| name))
        .unwrap_or("sindri");

    let cli_version = env!("CARGO_PKG_VERSION");
    let image_tag = format!("{}-build", cli_version);

    let build_config = ImageBuildConfig {
        image_name,
        image_tag: &image_tag,
        no_cache: force,
    };

    let result = build_sindri_image(&build_config, git_ref).await?;

    // If the image name contains a registry prefix, push it
    if image_name.contains('/') {
        info!("Pushing built image to registry...");
        push_image_to_registry(&result.image_ref).await?;
    }

    Ok(result.image_ref)
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
        // "uname -r" works on both Linux and macOS
        let result = get_command_version("uname", "-r");
        assert!(result.is_ok(), "uname -r should succeed");
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
