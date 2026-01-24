//! Platform detection module
//!
//! Detects the operating system, architecture, Linux distribution (if applicable),
//! and available package managers. This information is used to provide
//! platform-specific installation instructions.

use std::fs;

/// Operating system platform
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Platform {
    /// macOS (Darwin)
    MacOS,
    /// Linux with detected distribution
    Linux(LinuxDistro),
    /// Windows
    Windows,
    /// Unknown/unsupported platform
    Unknown,
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MacOS => write!(f, "macOS"),
            Self::Linux(distro) => write!(f, "Linux/{}", distro),
            Self::Windows => write!(f, "Windows"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Linux distribution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinuxDistro {
    /// Debian-based (Debian, Ubuntu, Mint, Pop!_OS)
    Debian,
    /// Fedora-based (Fedora, RHEL, CentOS, Rocky Linux)
    Fedora,
    /// Arch-based (Arch Linux, Manjaro)
    Arch,
    /// Alpine Linux
    Alpine,
    /// NixOS
    NixOS,
    /// Unknown distribution
    Unknown,
}

impl std::fmt::Display for LinuxDistro {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Debian => write!(f, "Debian"),
            Self::Fedora => write!(f, "Fedora"),
            Self::Arch => write!(f, "Arch"),
            Self::Alpine => write!(f, "Alpine"),
            Self::NixOS => write!(f, "NixOS"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// CPU architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Arch {
    /// x86_64 / AMD64
    X86_64,
    /// ARM64 / AArch64
    Aarch64,
    /// 32-bit ARM
    Arm,
    /// Unknown architecture
    Unknown,
}

impl std::fmt::Display for Arch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::X86_64 => write!(f, "x86_64"),
            Self::Aarch64 => write!(f, "aarch64"),
            Self::Arm => write!(f, "arm"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// Available package managers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PackageManager {
    /// Homebrew (macOS/Linux)
    Homebrew,
    /// APT (Debian/Ubuntu)
    Apt,
    /// DNF (Fedora)
    Dnf,
    /// Yum (RHEL/CentOS)
    Yum,
    /// Pacman (Arch)
    Pacman,
    /// Apk (Alpine)
    Apk,
    /// Nix (NixOS)
    Nix,
    /// Winget (Windows)
    Winget,
    /// Chocolatey (Windows)
    Chocolatey,
    /// Scoop (Windows)
    Scoop,
}

impl std::fmt::Display for PackageManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Homebrew => write!(f, "Homebrew"),
            Self::Apt => write!(f, "APT"),
            Self::Dnf => write!(f, "DNF"),
            Self::Yum => write!(f, "Yum"),
            Self::Pacman => write!(f, "Pacman"),
            Self::Apk => write!(f, "APK"),
            Self::Nix => write!(f, "Nix"),
            Self::Winget => write!(f, "Winget"),
            Self::Chocolatey => write!(f, "Chocolatey"),
            Self::Scoop => write!(f, "Scoop"),
        }
    }
}

/// Comprehensive platform information
#[derive(Debug, Clone)]
pub struct PlatformInfo {
    /// Operating system
    pub os: Platform,
    /// CPU architecture
    pub arch: Arch,
    /// Available package managers
    pub package_managers: Vec<PackageManager>,
}

impl PlatformInfo {
    /// Get the preferred package manager for this platform
    pub fn preferred_package_manager(&self) -> Option<&PackageManager> {
        self.package_managers.first()
    }
}

/// Detect the current platform
pub fn detect_platform() -> PlatformInfo {
    let os = detect_os();
    let arch = detect_arch();
    let package_managers = detect_package_managers(&os);

    PlatformInfo {
        os,
        arch,
        package_managers,
    }
}

/// Detect the operating system
fn detect_os() -> Platform {
    match std::env::consts::OS {
        "macos" => Platform::MacOS,
        "windows" => Platform::Windows,
        "linux" => Platform::Linux(detect_linux_distro()),
        _ => Platform::Unknown,
    }
}

/// Detect the Linux distribution
fn detect_linux_distro() -> LinuxDistro {
    // Try to read /etc/os-release
    if let Ok(content) = fs::read_to_string("/etc/os-release") {
        let content_lower = content.to_lowercase();

        // Check ID and ID_LIKE fields
        if content_lower.contains("id=debian")
            || content_lower.contains("id=ubuntu")
            || content_lower.contains("id=linuxmint")
            || content_lower.contains("id=pop")
            || content_lower.contains("id_like=debian")
            || content_lower.contains("id_like=\"debian")
        {
            return LinuxDistro::Debian;
        }

        if content_lower.contains("id=fedora")
            || content_lower.contains("id=rhel")
            || content_lower.contains("id=centos")
            || content_lower.contains("id=rocky")
            || content_lower.contains("id=almalinux")
            || content_lower.contains("id_like=fedora")
            || content_lower.contains("id_like=\"fedora")
        {
            return LinuxDistro::Fedora;
        }

        if content_lower.contains("id=arch")
            || content_lower.contains("id=manjaro")
            || content_lower.contains("id_like=arch")
        {
            return LinuxDistro::Arch;
        }

        if content_lower.contains("id=alpine") {
            return LinuxDistro::Alpine;
        }

        if content_lower.contains("id=nixos") {
            return LinuxDistro::NixOS;
        }
    }

    // Fallback checks using package manager presence
    if which::which("apt-get").is_ok() {
        return LinuxDistro::Debian;
    }
    if which::which("dnf").is_ok() {
        return LinuxDistro::Fedora;
    }
    if which::which("pacman").is_ok() {
        return LinuxDistro::Arch;
    }
    if which::which("apk").is_ok() {
        return LinuxDistro::Alpine;
    }
    if which::which("nix-env").is_ok() {
        return LinuxDistro::NixOS;
    }

    LinuxDistro::Unknown
}

/// Detect the CPU architecture
fn detect_arch() -> Arch {
    match std::env::consts::ARCH {
        "x86_64" => Arch::X86_64,
        "aarch64" => Arch::Aarch64,
        "arm" => Arch::Arm,
        _ => Arch::Unknown,
    }
}

/// Detect available package managers
fn detect_package_managers(os: &Platform) -> Vec<PackageManager> {
    let mut managers = Vec::new();

    // Check for cross-platform managers first (preferred)
    if which::which("brew").is_ok() {
        managers.push(PackageManager::Homebrew);
    }

    // Platform-specific managers
    match os {
        Platform::Linux(distro) => {
            match distro {
                LinuxDistro::Debian => {
                    if which::which("apt-get").is_ok() {
                        managers.push(PackageManager::Apt);
                    }
                }
                LinuxDistro::Fedora => {
                    if which::which("dnf").is_ok() {
                        managers.push(PackageManager::Dnf);
                    } else if which::which("yum").is_ok() {
                        managers.push(PackageManager::Yum);
                    }
                }
                LinuxDistro::Arch => {
                    if which::which("pacman").is_ok() {
                        managers.push(PackageManager::Pacman);
                    }
                }
                LinuxDistro::Alpine => {
                    if which::which("apk").is_ok() {
                        managers.push(PackageManager::Apk);
                    }
                }
                LinuxDistro::NixOS => {
                    if which::which("nix-env").is_ok() {
                        managers.push(PackageManager::Nix);
                    }
                }
                LinuxDistro::Unknown => {
                    // Try common package managers
                    if which::which("apt-get").is_ok() {
                        managers.push(PackageManager::Apt);
                    }
                    if which::which("dnf").is_ok() {
                        managers.push(PackageManager::Dnf);
                    }
                    if which::which("yum").is_ok() {
                        managers.push(PackageManager::Yum);
                    }
                    if which::which("pacman").is_ok() {
                        managers.push(PackageManager::Pacman);
                    }
                }
            }
        }
        Platform::Windows => {
            if which::which("winget").is_ok() {
                managers.push(PackageManager::Winget);
            }
            if which::which("choco").is_ok() {
                managers.push(PackageManager::Chocolatey);
            }
            if which::which("scoop").is_ok() {
                managers.push(PackageManager::Scoop);
            }
        }
        Platform::MacOS => {
            // Homebrew already checked above
        }
        Platform::Unknown => {}
    }

    managers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_display() {
        assert_eq!(Platform::MacOS.to_string(), "macOS");
        assert_eq!(
            Platform::Linux(LinuxDistro::Debian).to_string(),
            "Linux/Debian"
        );
        assert_eq!(Platform::Windows.to_string(), "Windows");
    }

    #[test]
    fn test_arch_display() {
        assert_eq!(Arch::X86_64.to_string(), "x86_64");
        assert_eq!(Arch::Aarch64.to_string(), "aarch64");
    }

    #[test]
    fn test_package_manager_display() {
        assert_eq!(PackageManager::Homebrew.to_string(), "Homebrew");
        assert_eq!(PackageManager::Apt.to_string(), "APT");
    }

    #[test]
    fn test_detect_platform() {
        let platform = detect_platform();
        // Just verify it runs without panic
        assert!(matches!(
            platform.os,
            Platform::MacOS | Platform::Linux(_) | Platform::Windows | Platform::Unknown
        ));
    }

    #[test]
    fn test_detect_arch() {
        let arch = detect_arch();
        // Should detect something on most systems
        assert!(matches!(
            arch,
            Arch::X86_64 | Arch::Aarch64 | Arch::Arm | Arch::Unknown
        ));
    }
}
