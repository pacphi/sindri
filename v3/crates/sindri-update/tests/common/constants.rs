//! Shared constants for test infrastructure
//!
//! Provides centralized version strings, platform identifiers, and test data
//! to eliminate duplication across test files.

// Version constants
pub const VERSION_3_0_0: &str = "3.0.0";
pub const VERSION_3_0_5: &str = "3.0.5";
pub const VERSION_3_0_8: &str = "3.0.8";
pub const VERSION_3_1_0: &str = "3.1.0";
pub const VERSION_3_2_0: &str = "3.2.0";
pub const VERSION_3_5_0: &str = "3.5.0";
pub const VERSION_4_0_0: &str = "4.0.0";
pub const VERSION_5_0_0: &str = "5.0.0";
pub const VERSION_99_0_0: &str = "99.0.0";

// Tag constants (with 'v' prefix)
pub const TAG_V3_0_0: &str = "v3.0.0";
pub const TAG_V3_0_0_TEST: &str = "v3.0.0-test";
pub const TAG_V3_1_0: &str = "v3.1.0";

// Platform target triples
pub const PLATFORM_LINUX_X86_64: &str = "x86_64-unknown-linux-musl";
pub const PLATFORM_MACOS_X86_64: &str = "x86_64-apple-darwin";
pub const PLATFORM_MACOS_ARM64: &str = "aarch64-apple-darwin";
pub const PLATFORM_WINDOWS_X86_64: &str = "x86_64-pc-windows-msvc";

// Binary content for testing
pub const FAKE_BINARY_CONTENT: &[u8] = b"fake binary content for testing";
pub const SUCCESS_CONTENT: &[u8] = b"success";
pub const TEST_BINARY_CONTENT: &[u8] = b"test binary";
pub const SHORT_CONTENT: &[u8] = b"short content";

// Common extension versions
pub const EXT_VERSION_1_0_0: &str = "1.0.0";
pub const EXT_VERSION_1_1_0: &str = "1.1.0";
pub const EXT_VERSION_1_1_5: &str = "1.1.5";
pub const EXT_VERSION_1_5_0: &str = "1.5.0";
pub const EXT_VERSION_2_0_0: &str = "2.0.0";

// Extension names
pub const EXT_GIT: &str = "git";
pub const EXT_DOCKER: &str = "docker";
pub const EXT_KUBERNETES: &str = "kubernetes";

// Checksum constants
pub const WRONG_CHECKSUM: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Get the default test platform based on compile-time target
pub fn default_test_platform() -> &'static str {
    if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        PLATFORM_LINUX_X86_64
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "x86_64") {
        PLATFORM_MACOS_X86_64
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        PLATFORM_MACOS_ARM64
    } else {
        PLATFORM_LINUX_X86_64 // default fallback
    }
}
