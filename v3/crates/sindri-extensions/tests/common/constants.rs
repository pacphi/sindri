//! Test constants for sindri-extensions tests

#![allow(dead_code)]

use std::time::Duration;

/// Default timeout for test operations
pub const DEFAULT_TEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Extended timeout for slow operations (network, builds)
pub const EXTENDED_TEST_TIMEOUT: Duration = Duration::from_secs(120);

/// Short timeout for quick validation tests
pub const SHORT_TEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Default test extension name
pub const TEST_EXTENSION_NAME: &str = "test-extension";

/// Default test extension version
pub const TEST_EXTENSION_VERSION: &str = "1.0.0";

/// Heavy extensions that require special handling (>4GB memory/disk)
pub const HEAVY_EXTENSIONS: &[&str] = &[
    "cuda",
    "ollama",
    "android-sdk",
    "xcode-cli",
    "pytorch",
    "tensorflow",
];

/// Extensions known to require network access during install
pub const NETWORK_EXTENSIONS: &[&str] = &["docker", "kubernetes", "aws-cli", "gcloud", "azure-cli"];

/// Extensions that use mise for installation
pub const MISE_EXTENSIONS: &[&str] = &[
    "python", "nodejs", "golang", "rust", "ruby", "java", "deno", "bun",
];

/// Extensions that use apt for installation
pub const APT_EXTENSIONS: &[&str] = &["docker", "postgresql", "redis", "nginx"];

/// Extensions that use script installation
pub const SCRIPT_EXTENSIONS: &[&str] = &["claude-code", "cursor", "windsurf", "zed"];

/// Extensions that use binary download
pub const BINARY_EXTENSIONS: &[&str] = &["gh", "ripgrep", "fd", "bat", "exa"];

/// All extension categories
pub const EXTENSION_CATEGORIES: &[&str] = &[
    "ai-agents",
    "ai-dev",
    "claude",
    "cloud",
    "desktop",
    "devops",
    "documentation",
    "languages",
    "mcp",
    "productivity",
    "research",
    "testing",
];

/// Test fixture paths
pub mod fixtures {
    /// Directory containing test extension YAML files
    pub const EXTENSIONS_DIR: &str = "tests/fixtures/extensions";

    /// Directory containing test manifest files
    pub const MANIFESTS_DIR: &str = "tests/fixtures/manifests";

    /// Minimal test extension fixture
    pub const MINIMAL_EXTENSION: &str = "tests/fixtures/extensions/minimal.yaml";

    /// Full-featured test extension fixture
    pub const FULL_EXTENSION: &str = "tests/fixtures/extensions/full.yaml";

    /// Extension with hooks
    pub const HOOKS_EXTENSION: &str = "tests/fixtures/extensions/hooks.yaml";

    /// Extension with dependencies
    pub const DEPS_EXTENSION: &str = "tests/fixtures/extensions/dependencies.yaml";
}

/// Mock data for testing
pub mod mock_data {
    /// Sample extension YAML for minimal extension
    pub const MINIMAL_EXTENSION_YAML: &str = r#"
metadata:
  name: test-minimal
  version: "1.0.0"
  description: Minimal test extension for unit tests
  category: testing

install:
  method: script
  script:
    path: scripts/install.sh
    timeout: 60

validate:
  commands:
    - name: echo
      versionFlag: "test"
"#;

    /// Sample extension YAML with mise installation
    pub const MISE_EXTENSION_YAML: &str = r#"
metadata:
  name: test-mise
  version: "1.0.0"
  description: Mise-based test extension
  category: languages

install:
  method: mise
  mise:
    configFile: mise.toml
    reshim_after_install: true

validate:
  mise:
    tools:
      - test-tool@latest
"#;

    /// Sample extension YAML with hooks
    pub const HOOKS_EXTENSION_YAML: &str = r#"
metadata:
  name: test-hooks
  version: "1.0.0"
  description: Test extension with hooks
  category: testing

install:
  method: script
  script:
    path: scripts/install.sh
    timeout: 60

validate:
  commands:
    - name: echo
      versionFlag: "test"

capabilities:
  hooks:
    pre-install:
      command: "echo 'Pre-install hook executed'"
      description: "Runs before installation"
    post-install:
      command: "echo 'Post-install hook executed'"
      description: "Runs after installation"
"#;

    /// Sample extension YAML with dependencies
    pub const DEPS_EXTENSION_YAML: &str = r#"
metadata:
  name: test-deps
  version: "1.0.0"
  description: Test extension with dependencies
  category: testing
  dependencies:
    - test-minimal

install:
  method: script
  script:
    path: scripts/install.sh
    timeout: 60

validate:
  commands:
    - name: echo
      versionFlag: "test"
"#;

    /// Sample manifest JSON
    pub const MANIFEST_JSON: &str = r#"{
  "extensions": {
    "test-minimal": {
      "version": "1.0.0",
      "installed_at": "2026-01-26T00:00:00Z",
      "method": "script"
    }
  }
}"#;
}
