// Platform detection and representation
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Os {
    Linux,
    Macos,
    Windows,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Arch {
    X86_64,
    Aarch64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct Platform {
    pub os: Os,
    pub arch: Arch,
}

impl Platform {
    pub fn current() -> Self {
        Platform {
            os: if cfg!(target_os = "macos") {
                Os::Macos
            } else if cfg!(target_os = "windows") {
                Os::Windows
            } else {
                Os::Linux
            },
            arch: if cfg!(target_arch = "aarch64") {
                Arch::Aarch64
            } else {
                Arch::X86_64
            },
        }
    }

    pub fn triple(&self) -> &'static str {
        match (&self.os, &self.arch) {
            (Os::Linux, Arch::X86_64) => "x86_64-unknown-linux-gnu",
            (Os::Linux, Arch::Aarch64) => "aarch64-unknown-linux-gnu",
            (Os::Macos, Arch::X86_64) => "x86_64-apple-darwin",
            (Os::Macos, Arch::Aarch64) => "aarch64-apple-darwin",
            (Os::Windows, Arch::X86_64) => "x86_64-pc-windows-msvc",
            (Os::Windows, Arch::Aarch64) => "aarch64-pc-windows-msvc",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TargetProfile {
    pub platform: Platform,
    pub capabilities: Capabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Capabilities {
    pub system_package_manager: Option<String>,
    pub has_docker: bool,
    pub has_sudo: bool,
    pub shell: Option<String>,
}

impl Default for Capabilities {
    fn default() -> Self {
        Self {
            system_package_manager: None,
            has_docker: false,
            has_sudo: false,
            shell: None,
        }
    }
}
