//! Utility functions for provider implementations

use anyhow::{anyhow, Result};
use std::process::Output;
use tracing::{debug, warn};

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

/// Run a command and return output
#[allow(dead_code)] // Reserved for future use
pub fn run_command(cmd: &str, args: &[&str]) -> Result<Output> {
    debug!("Running: {} {}", cmd, args.join(" "));

    let output = std::process::Command::new(cmd).args(args).output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!(
            "Command failed: {} {}\nStderr: {}",
            cmd,
            args.join(" "),
            stderr
        );
    }

    Ok(output)
}

/// Run a command asynchronously
#[allow(dead_code)] // Reserved for future use
pub async fn run_command_async(cmd: &str, args: &[&str]) -> Result<Output> {
    debug!("Running async: {} {}", cmd, args.join(" "));

    let output = tokio::process::Command::new(cmd)
        .args(args)
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!(
            "Command failed: {} {}\nStderr: {}",
            cmd,
            args.join(" "),
            stderr
        );
    }

    Ok(output)
}

/// Parse memory string (e.g., "4GB", "512MB") to bytes
#[allow(dead_code)] // Reserved for future use
pub fn parse_memory(mem: &str) -> Result<u64> {
    let mem = mem.trim().to_uppercase();

    if let Some(gb) = mem.strip_suffix("GB") {
        let value: u64 = gb.parse()?;
        Ok(value * 1024 * 1024 * 1024)
    } else if let Some(mb) = mem.strip_suffix("MB") {
        let value: u64 = mb.parse()?;
        Ok(value * 1024 * 1024)
    } else {
        Err(anyhow!(
            "Invalid memory format: {}. Expected format: NGB or NMB",
            mem
        ))
    }
}

/// Format bytes as human-readable string
#[allow(dead_code)] // Reserved for future use
pub fn format_bytes(bytes: u64) -> String {
    const GB: u64 = 1024 * 1024 * 1024;
    const MB: u64 = 1024 * 1024;
    const KB: u64 = 1024;

    if bytes >= GB {
        format!("{:.1}GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}KB", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
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
/// * `version` - Optional version to fetch (defaults to CLI version)
///
/// # Returns
/// Path to the v3 directory containing the Dockerfile and build context
///
/// # Note
/// This function replaces the old find_dockerfile() which searched for user-provided
/// Dockerfiles. Sindri v3 uses its own official Dockerfile from the GitHub repository,
/// not user-provided ones. A shallow clone is necessary because v3/Dockerfile has
/// dependencies on other files in the v3 directory.
pub async fn fetch_sindri_build_context(
    cache_dir: &std::path::Path,
    version: Option<&str>,
) -> Result<std::path::PathBuf> {
    use tokio::fs;
    use tracing::{debug, info, warn};

    // Determine which version to fetch (default to CLI version)
    let target_version = version.unwrap_or(env!("CARGO_PKG_VERSION"));

    // Determine Git ref (try tag first, fallback to main)
    let git_ref = format!("v{}", target_version);

    info!(
        "Fetching Sindri v3 build context (version: {})",
        target_version
    );

    // Create cache directory if it doesn't exist
    fs::create_dir_all(cache_dir).await?;

    // Repository cache path
    let repo_dir = cache_dir.join(format!("sindri-{}", target_version));
    let v3_dir = repo_dir.join("v3");

    // Return cached v3 directory if it exists and is valid
    if v3_dir.join("Dockerfile").exists() {
        debug!("Using cached Sindri v3 directory at {}", v3_dir.display());
        return Ok(v3_dir);
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
    }

    // Verify v3 directory exists
    if !v3_dir.join("Dockerfile").exists() {
        return Err(anyhow!(
            "Cloned repository doesn't contain v3/Dockerfile at expected location"
        ));
    }

    info!("Sindri v3 build context ready at {}", v3_dir.display());
    Ok(v3_dir)
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

/// Build Sindri binary from cloned source and prepare for Docker build
///
/// Compiles the Sindri binary using cargo and copies it to v3/bin/ directory
/// in the cloned repository. This allows the Dockerfile's builder-local stage
/// to pick it up, avoiding the need for BUILD_FROM_SOURCE=true which would
/// redundantly clone the repository again inside Docker.
///
/// # Arguments
/// * `v3_dir` - Path to the v3 directory in the cloned repository
///
/// # Returns
/// Path to the compiled binary in v3/bin/sindri
pub async fn build_and_prepare_binary(v3_dir: &std::path::Path) -> Result<std::path::PathBuf> {
    use tokio::process::Command;
    use tracing::info;

    info!("Compiling Sindri binary from source...");

    // Build using cargo
    let cargo_status = Command::new("cargo")
        .args(["build", "--release", "--bin", "sindri"])
        .current_dir(v3_dir)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .await?;

    if !cargo_status.success() {
        return Err(anyhow!("Failed to compile Sindri binary from source"));
    }

    // Copy the built binary to v3/bin/ for Dockerfile's builder-local stage
    let built_binary = v3_dir.join("target/release/sindri");
    let bin_dir = v3_dir.join("bin");
    std::fs::create_dir_all(&bin_dir)?;
    let dest_binary = bin_dir.join("sindri");
    std::fs::copy(&built_binary, &dest_binary)?;

    info!("Binary ready at {}", dest_binary.display());
    Ok(dest_binary)
}
