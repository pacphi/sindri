//! Platform detection utilities
//!
//! This module provides OS and architecture detection for
//! determining the correct installation method for cluster tools.

use anyhow::Result;

/// Supported operating systems
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    /// macOS
    MacOS,
    /// Debian/Ubuntu Linux
    Debian,
    /// Other Linux distributions
    Linux,
    /// Windows (limited support)
    Windows,
}

impl Os {
    /// Detect the current operating system
    pub fn detect() -> Result<Self> {
        #[cfg(target_os = "macos")]
        {
            Ok(Os::MacOS)
        }

        #[cfg(target_os = "linux")]
        {
            // Check for Debian/Ubuntu
            if std::path::Path::new("/etc/debian_version").exists() {
                return Ok(Os::Debian);
            }

            // Check os-release for debian-based
            if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
                let content_lower = content.to_lowercase();
                if content_lower.contains("debian") || content_lower.contains("ubuntu") {
                    return Ok(Os::Debian);
                }
            }

            Ok(Os::Linux)
        }

        #[cfg(target_os = "windows")]
        {
            Ok(Os::Windows)
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            Err(anyhow!("Unsupported operating system"))
        }
    }

    /// Get the OS name for binary downloads (e.g., "darwin", "linux")
    pub fn download_name(&self) -> &'static str {
        match self {
            Os::MacOS => "darwin",
            Os::Debian | Os::Linux => "linux",
            Os::Windows => "windows",
        }
    }

    /// Check if this OS supports Homebrew
    pub fn supports_homebrew(&self) -> bool {
        matches!(self, Os::MacOS | Os::Debian | Os::Linux)
    }
}

impl std::fmt::Display for Os {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Os::MacOS => write!(f, "macOS"),
            Os::Debian => write!(f, "Debian/Ubuntu"),
            Os::Linux => write!(f, "Linux"),
            Os::Windows => write!(f, "Windows"),
        }
    }
}

/// Supported CPU architectures
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    /// x86_64 / AMD64
    Amd64,
    /// ARM64 / AArch64
    Arm64,
}

impl Arch {
    /// Detect the current CPU architecture
    pub fn detect() -> Result<Self> {
        #[cfg(target_arch = "x86_64")]
        {
            Ok(Arch::Amd64)
        }

        #[cfg(target_arch = "aarch64")]
        {
            Ok(Arch::Arm64)
        }

        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            Err(anyhow!(
                "Unsupported CPU architecture: {}",
                std::env::consts::ARCH
            ))
        }
    }

    /// Get the architecture name for binary downloads (e.g., "amd64", "arm64")
    pub fn download_name(&self) -> &'static str {
        match self {
            Arch::Amd64 => "amd64",
            Arch::Arm64 => "arm64",
        }
    }
}

impl std::fmt::Display for Arch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Arch::Amd64 => write!(f, "amd64"),
            Arch::Arm64 => write!(f, "arm64"),
        }
    }
}

/// Platform information combining OS and architecture
#[derive(Debug, Clone)]
pub struct Platform {
    pub os: Os,
    pub arch: Arch,
}

impl Platform {
    /// Detect the current platform
    pub fn detect() -> Result<Self> {
        Ok(Self {
            os: Os::detect()?,
            arch: Arch::detect()?,
        })
    }

    /// Check if Homebrew is available
    pub fn has_homebrew(&self) -> bool {
        if !self.os.supports_homebrew() {
            return false;
        }
        which::which("brew").is_ok()
    }

    /// Get the install directory for binaries
    pub fn install_dir(&self) -> &'static str {
        "/usr/local/bin"
    }

    /// Check if we need sudo for installation
    pub fn needs_sudo(&self) -> bool {
        let install_dir = self.install_dir();
        !std::fs::metadata(install_dir)
            .map(|m| {
                // Check if directory is writable
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mode = m.permissions().mode();
                    // Check if current user can write
                    mode & 0o200 != 0 || mode & 0o020 != 0 || mode & 0o002 != 0
                }
                #[cfg(not(unix))]
                {
                    !m.permissions().readonly()
                }
            })
            .unwrap_or(true)
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.os, self.arch)
    }
}

/// Check if a command exists in PATH
pub fn command_exists(cmd: &str) -> bool {
    which::which(cmd).is_ok()
}

/// Get version of a command
pub fn get_command_version(cmd: &str, version_arg: &str) -> Option<String> {
    let output = std::process::Command::new(cmd)
        .arg(version_arg)
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_os_detect() {
        let os = Os::detect();
        assert!(os.is_ok());
    }

    #[test]
    fn test_arch_detect() {
        let arch = Arch::detect();
        assert!(arch.is_ok());
    }

    #[test]
    fn test_platform_detect() {
        let platform = Platform::detect();
        assert!(platform.is_ok());
    }

    #[test]
    fn test_os_download_name() {
        assert_eq!(Os::MacOS.download_name(), "darwin");
        assert_eq!(Os::Linux.download_name(), "linux");
        assert_eq!(Os::Debian.download_name(), "linux");
    }

    #[test]
    fn test_arch_download_name() {
        assert_eq!(Arch::Amd64.download_name(), "amd64");
        assert_eq!(Arch::Arm64.download_name(), "arm64");
    }
}
