// Platform detection and representation
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
        // Test hook (Wave 4A): when `SINDRI_TEST_PLATFORM_OVERRIDE` is set
        // (e.g. `linux-x86_64`, `macos-aarch64`), parse it and short-circuit
        // platform detection. This is consumed by the integration-test
        // harness in `v4/tests/integration` to drive admission gates without
        // having to cross-compile or virtualise. The variable is only ever
        // read by `Platform::current` and is intentionally undocumented in
        // user-facing CLI help.
        if let Ok(raw) = std::env::var("SINDRI_TEST_PLATFORM_OVERRIDE") {
            if let Some(p) = Self::parse_override(&raw) {
                return p;
            }
        }
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

    /// Parse a `<os>-<arch>` token (e.g. `linux-x86_64`, `macos-aarch64`).
    ///
    /// Returns `None` if either component is unrecognised. Used by
    /// [`Platform::current`] when the `SINDRI_TEST_PLATFORM_OVERRIDE`
    /// environment variable is set.
    fn parse_override(raw: &str) -> Option<Self> {
        let (os_s, arch_s) = raw.trim().split_once('-')?;
        let os = match os_s {
            "linux" => Os::Linux,
            "macos" => Os::Macos,
            "windows" => Os::Windows,
            _ => return None,
        };
        let arch = match arch_s {
            "x86_64" => Arch::X86_64,
            "aarch64" => Arch::Aarch64,
            _ => return None,
        };
        Some(Platform { os, arch })
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct Capabilities {
    pub system_package_manager: Option<String>,
    pub has_docker: bool,
    pub has_sudo: bool,
    pub shell: Option<String>,
}
