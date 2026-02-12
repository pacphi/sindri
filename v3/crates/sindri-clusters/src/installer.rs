//! Tool installation helpers
//!
//! This module provides cross-platform installation support for
//! kind and k3d cluster management tools.
//!
//! # Installation Methods
//!
//! - **macOS**: Homebrew (preferred) or binary download
//! - **Linux**: Binary download from GitHub releases or official scripts
//!
//! # Example
//!
//! ```ignore
//! use sindri_clusters::installer;
//!
//! // Install kind
//! installer::install_kind().await?;
//!
//! // Install k3d
//! installer::install_k3d().await?;
//! ```

use crate::platform::{Os, Platform};
use anyhow::{anyhow, Result};
use tokio::process::Command;
use tracing::{debug, info};

/// Default kind version to install (fallback)
const DEFAULT_KIND_VERSION: &str = "v0.25.0";

/// GitHub API URL for kind releases
const KIND_RELEASES_URL: &str = "https://api.github.com/repos/kubernetes-sigs/kind/releases/latest";

/// Kind binary download base URL
const KIND_DOWNLOAD_BASE: &str = "https://kind.sigs.k8s.io/dl";

/// K3d official install script URL
const K3D_INSTALL_SCRIPT: &str = "https://raw.githubusercontent.com/k3d-io/k3d/main/install.sh";

/// Install kind using the best method for the current platform
pub async fn install_kind() -> Result<()> {
    let platform = Platform::detect()?;
    info!("Installing kind for {}", platform);

    match platform.os {
        Os::MacOS => install_kind_macos(&platform).await,
        Os::Debian | Os::Linux => install_kind_linux(&platform).await,
        Os::Windows => Err(anyhow!(
            "Windows installation not supported. Please install kind manually."
        )),
    }
}

/// Install kind on macOS (Homebrew preferred)
async fn install_kind_macos(platform: &Platform) -> Result<()> {
    if platform.has_homebrew() {
        info!("Installing kind via Homebrew...");
        let output = Command::new("brew")
            .args(["install", "kind"])
            .output()
            .await?;

        if output.status.success() {
            info!("kind installed successfully via Homebrew");
            return Ok(());
        }

        // Fall back to binary if Homebrew fails
        debug!("Homebrew installation failed, falling back to binary download");
    }

    install_kind_binary(platform).await
}

/// Install kind on Linux via binary download
async fn install_kind_linux(platform: &Platform) -> Result<()> {
    install_kind_binary(platform).await
}

/// Install kind via direct binary download
async fn install_kind_binary(platform: &Platform) -> Result<()> {
    let version = get_latest_kind_version().await.unwrap_or_else(|_| {
        info!(
            "Could not determine latest kind version, using {}",
            DEFAULT_KIND_VERSION
        );
        DEFAULT_KIND_VERSION.to_string()
    });

    let os = platform.os.download_name();
    let arch = platform.arch.download_name();
    let url = format!("{}/{}/kind-{}-{}", KIND_DOWNLOAD_BASE, version, os, arch);

    info!("Downloading kind {} for {}/{}...", version, os, arch);
    download_and_install_binary(&url, "kind", platform).await
}

/// Install k3d using the best method for the current platform
pub async fn install_k3d() -> Result<()> {
    let platform = Platform::detect()?;
    info!("Installing k3d for {}", platform);

    match platform.os {
        Os::MacOS => install_k3d_macos(&platform).await,
        Os::Debian | Os::Linux => install_k3d_linux(&platform).await,
        Os::Windows => Err(anyhow!(
            "Windows installation not supported. Please install k3d manually."
        )),
    }
}

/// Install k3d on macOS (Homebrew preferred)
async fn install_k3d_macos(platform: &Platform) -> Result<()> {
    if platform.has_homebrew() {
        info!("Installing k3d via Homebrew...");
        let output = Command::new("brew")
            .args(["install", "k3d"])
            .output()
            .await?;

        if output.status.success() {
            info!("k3d installed successfully via Homebrew");
            return Ok(());
        }

        // Fall back to script if Homebrew fails
        debug!("Homebrew installation failed, falling back to install script");
    }

    install_k3d_script().await
}

/// Install k3d on Linux via official script
async fn install_k3d_linux(_platform: &Platform) -> Result<()> {
    install_k3d_script().await
}

/// Install k3d via official installation script
async fn install_k3d_script() -> Result<()> {
    info!("Installing k3d via official script...");

    // Download and execute the install script
    let output = Command::new("bash")
        .args(["-c", &format!("curl -s {} | bash", K3D_INSTALL_SCRIPT)])
        .output()
        .await?;

    if output.status.success() {
        info!("k3d installed successfully via install script");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow!("Failed to install k3d: {}", stderr))
    }
}

/// Download a binary and install it to the system
async fn download_and_install_binary(url: &str, name: &str, platform: &Platform) -> Result<()> {
    let temp_path = std::env::temp_dir().join(name);
    let install_path = format!("{}/{}", platform.install_dir(), name);

    debug!("Downloading from: {}", url);
    debug!("Temp path: {:?}", temp_path);
    debug!("Install path: {}", install_path);

    // Download the binary
    let response = reqwest::get(url).await?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Failed to download {}: HTTP {}",
            name,
            response.status()
        ));
    }

    let bytes = response.bytes().await?;
    std::fs::write(&temp_path, &bytes)?;

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temp_path, std::fs::Permissions::from_mode(0o755))?;
    }

    // Move to install directory
    if platform.needs_sudo() {
        info!("Installation requires sudo access...");
        let output = Command::new("sudo")
            .args(["mv", &temp_path.to_string_lossy(), &install_path])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to install {}: {}", name, stderr));
        }
    } else {
        std::fs::rename(&temp_path, &install_path)?;
    }

    info!("{} installed to {}", name, install_path);
    Ok(())
}

/// Get the latest kind version from GitHub API
async fn get_latest_kind_version() -> Result<String> {
    get_latest_github_release_version(KIND_RELEASES_URL).await
}

/// Get the latest release version from a GitHub API URL
async fn get_latest_github_release_version(api_url: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let response = client
        .get(api_url)
        .header(
            "User-Agent",
            format!("sindri-clusters/{}", env!("CARGO_PKG_VERSION")),
        )
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Failed to fetch latest version: HTTP {}",
            response.status()
        ));
    }

    let release: serde_json::Value = response.json().await?;
    let tag_name = release["tag_name"]
        .as_str()
        .ok_or_else(|| anyhow!("No tag_name in release response"))?;

    Ok(tag_name.to_string())
}

/// Get installation instructions for kind
pub fn kind_install_instructions(platform: &Platform) -> Vec<String> {
    let mut instructions = Vec::new();

    match platform.os {
        Os::MacOS => {
            instructions.push("Homebrew: brew install kind".to_string());
            instructions.push(format!(
                "Direct download: curl -Lo ./kind https://kind.sigs.k8s.io/dl/latest/kind-darwin-{}",
                platform.arch
            ));
        }
        Os::Debian | Os::Linux => {
            instructions.push(format!(
                "Direct download: curl -Lo ./kind https://kind.sigs.k8s.io/dl/latest/kind-linux-{}",
                platform.arch
            ));
            instructions.push("Go: go install sigs.k8s.io/kind@latest".to_string());
        }
        Os::Windows => {
            instructions.push("choco install kind".to_string());
            instructions.push(
                "See: https://kind.sigs.k8s.io/docs/user/quick-start/#installation".to_string(),
            );
        }
    }

    instructions
}

/// Get installation instructions for k3d
pub fn k3d_install_instructions(platform: &Platform) -> Vec<String> {
    let mut instructions = Vec::new();

    match platform.os {
        Os::MacOS => {
            instructions.push("Homebrew: brew install k3d".to_string());
            instructions.push("Script: curl -s https://raw.githubusercontent.com/k3d-io/k3d/main/install.sh | bash".to_string());
        }
        Os::Debian | Os::Linux => {
            instructions.push("Script: curl -s https://raw.githubusercontent.com/k3d-io/k3d/main/install.sh | bash".to_string());
            instructions.push("See: https://k3d.io/#installation".to_string());
        }
        Os::Windows => {
            instructions.push("choco install k3d".to_string());
            instructions.push("See: https://k3d.io/#installation".to_string());
        }
    }

    instructions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::Arch;

    #[test]
    fn test_kind_install_instructions() {
        let platform = Platform {
            os: Os::MacOS,
            arch: Arch::Arm64,
        };
        let instructions = kind_install_instructions(&platform);
        assert!(!instructions.is_empty());
        assert!(instructions[0].contains("brew"));
    }

    #[test]
    fn test_k3d_install_instructions() {
        let platform = Platform {
            os: Os::Linux,
            arch: Arch::Amd64,
        };
        let instructions = k3d_install_instructions(&platform);
        assert!(!instructions.is_empty());
        assert!(instructions[0].contains("curl"));
    }
}
