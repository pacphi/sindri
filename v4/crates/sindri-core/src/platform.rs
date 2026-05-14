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

#[cfg(test)]
mod tests {
    use super::*;
    // Shared sindri-core test mutex — serialises every test in this binary
    // that mutates the process env. See `crate::paths::ENV_LOCK` docs.
    use crate::paths::ENV_LOCK;

    // ---------------------------------------------------------------------------
    // Platform::triple — all 6 combinations
    // ---------------------------------------------------------------------------

    #[test]
    fn triple_linux_x86_64() {
        let p = Platform {
            os: Os::Linux,
            arch: Arch::X86_64,
        };
        assert_eq!(p.triple(), "x86_64-unknown-linux-gnu");
    }

    #[test]
    fn triple_linux_aarch64() {
        let p = Platform {
            os: Os::Linux,
            arch: Arch::Aarch64,
        };
        assert_eq!(p.triple(), "aarch64-unknown-linux-gnu");
    }

    #[test]
    fn triple_macos_x86_64() {
        let p = Platform {
            os: Os::Macos,
            arch: Arch::X86_64,
        };
        assert_eq!(p.triple(), "x86_64-apple-darwin");
    }

    #[test]
    fn triple_macos_aarch64() {
        let p = Platform {
            os: Os::Macos,
            arch: Arch::Aarch64,
        };
        assert_eq!(p.triple(), "aarch64-apple-darwin");
    }

    #[test]
    fn triple_windows_x86_64() {
        let p = Platform {
            os: Os::Windows,
            arch: Arch::X86_64,
        };
        assert_eq!(p.triple(), "x86_64-pc-windows-msvc");
    }

    #[test]
    fn triple_windows_aarch64() {
        let p = Platform {
            os: Os::Windows,
            arch: Arch::Aarch64,
        };
        assert_eq!(p.triple(), "aarch64-pc-windows-msvc");
    }

    // ---------------------------------------------------------------------------
    // parse_override — valid inputs
    // ---------------------------------------------------------------------------

    #[test]
    fn parse_override_linux_x86_64() {
        let p = Platform::parse_override("linux-x86_64").unwrap();
        assert_eq!(p.os, Os::Linux);
        assert_eq!(p.arch, Arch::X86_64);
    }

    #[test]
    fn parse_override_macos_aarch64() {
        let p = Platform::parse_override("macos-aarch64").unwrap();
        assert_eq!(p.os, Os::Macos);
        assert_eq!(p.arch, Arch::Aarch64);
    }

    #[test]
    fn parse_override_windows_x86_64() {
        let p = Platform::parse_override("windows-x86_64").unwrap();
        assert_eq!(p.os, Os::Windows);
        assert_eq!(p.arch, Arch::X86_64);
    }

    #[test]
    fn parse_override_leading_trailing_whitespace() {
        let p = Platform::parse_override("  linux-x86_64  ").unwrap();
        assert_eq!(p.os, Os::Linux);
    }

    // ---------------------------------------------------------------------------
    // parse_override — invalid inputs
    // ---------------------------------------------------------------------------

    #[test]
    fn parse_override_unknown_os_returns_none() {
        assert!(Platform::parse_override("freebsd-x86_64").is_none());
    }

    #[test]
    fn parse_override_unknown_arch_returns_none() {
        assert!(Platform::parse_override("linux-riscv64").is_none());
    }

    #[test]
    fn parse_override_missing_dash_returns_none() {
        assert!(Platform::parse_override("linuxx86_64").is_none());
    }

    #[test]
    fn parse_override_empty_string_returns_none() {
        assert!(Platform::parse_override("").is_none());
    }

    // ---------------------------------------------------------------------------
    // Platform::current — env-var hook (SINDRI_TEST_PLATFORM_OVERRIDE)
    // ---------------------------------------------------------------------------

    #[test]
    fn current_env_override_linux_x86_64() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // SAFETY: ENV_LOCK serialises env mutations across the test binary.
        unsafe { std::env::set_var("SINDRI_TEST_PLATFORM_OVERRIDE", "linux-x86_64") };
        let p = Platform::current();
        unsafe { std::env::remove_var("SINDRI_TEST_PLATFORM_OVERRIDE") };
        assert_eq!(p.os, Os::Linux);
        assert_eq!(p.arch, Arch::X86_64);
    }

    #[test]
    fn current_env_override_macos_aarch64() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe { std::env::set_var("SINDRI_TEST_PLATFORM_OVERRIDE", "macos-aarch64") };
        let p = Platform::current();
        unsafe { std::env::remove_var("SINDRI_TEST_PLATFORM_OVERRIDE") };
        assert_eq!(p.os, Os::Macos);
        assert_eq!(p.arch, Arch::Aarch64);
    }

    #[test]
    fn current_without_override_does_not_panic() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // Remove the var if a previous test left it set.
        unsafe { std::env::remove_var("SINDRI_TEST_PLATFORM_OVERRIDE") };
        let _p = Platform::current(); // must not panic
    }

    // ---------------------------------------------------------------------------
    // Equality and hashing (derive-based, sanity check)
    // ---------------------------------------------------------------------------

    #[test]
    fn platform_equality() {
        let a = Platform {
            os: Os::Linux,
            arch: Arch::X86_64,
        };
        let b = Platform {
            os: Os::Linux,
            arch: Arch::X86_64,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn platform_inequality_different_arch() {
        let a = Platform {
            os: Os::Linux,
            arch: Arch::X86_64,
        };
        let b = Platform {
            os: Os::Linux,
            arch: Arch::Aarch64,
        };
        assert_ne!(a, b);
    }
}
