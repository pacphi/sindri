//! Extension types re-exported for sindri-extensions
//!
//! This module re-exports extension types from sindri-core for convenience
//! and adds extension-specific type utilities.

// Re-export all extension types from sindri-core
pub use sindri_core::types::{
    AdvancedFeatures,
    AptInstallConfig,
    AptRemoveConfig,
    AptRepository,
    AptUpgradeConfig,

    AuthCapability,
    AuthFeature,
    AuthMethod,
    AuthProvider,
    AuthValidator,
    BinaryDownload,
    BinaryInstallConfig,
    // Bill of Materials
    BomConfig,
    BomFile,
    BomFileType,
    BomSource,
    BomTool,
    BomToolType,
    // Capabilities
    CapabilitiesConfig,
    Checksum,
    ChecksumAlgorithm,
    CollisionHandlingConfig,
    CollisionScenario,
    CommandValidation,
    // Configuration phase
    ConfigureConfig,
    ConflictActionType,
    ConflictResourceType,
    ConflictRule,
    CoreFeatures,
    DetectionMethod,
    DownloadSource,
    DownloadSourceType,
    EnvironmentConfig,
    EnvironmentScope,

    // Core extension types
    Extension,
    ExtensionCategory,
    ExtensionMetadata,
    ExtensionRequirements,
    FeaturesConfig,
    GpuRequirementType,

    GpuRequirements,
    HookConfig,
    HooksCapability,
    // Installation configuration
    InstallConfig,
    InstallMethod,
    LlmFeatures,
    McpCapability,
    McpFeatures,
    McpServerConfig,
    McpTool,
    MergeFileConfig,
    MergeStrategy,
    MiseInstallConfig,
    MiseRemoveConfig,
    MiseUpgradeConfig,
    MiseValidation,

    NpmInstallConfig,
    OnConflictAction,
    ProjectContextCapability,
    ProjectInitCapability,
    ProjectInitCommand,
    ProjectInitValidation,
    // Removal
    RemoveConfig,
    ScenarioAction,
    ScenarioOption,

    ScriptConfig,

    ScriptRemoveConfig,

    StateMarker,
    StateMarkerType,
    SwarmFeatures,
    TemplateConfig,
    TemplateMode,
    // Upgrade
    UpgradeConfig,
    UpgradeStrategy,
    // Validation
    ValidateConfig,
    VersionDetection,
    VersionMarker,
};

// Re-export registry types that extensions use
pub use sindri_core::types::{Profile, RegistryEntry};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_deserialization() {
        let yaml = r#"
metadata:
  name: test-extension
  version: "1.0.0"
  description: Test extension for YAML parsing
  category: utilities

install:
  method: script
  script:
    path: scripts/install.sh
    timeout: 300

validate:
  commands:
    - name: test-cmd
      versionFlag: "--version"
"#;

        let extension: Extension = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(extension.metadata.name, "test-extension");
        assert_eq!(extension.metadata.version, "1.0.0");
        assert_eq!(extension.metadata.category, ExtensionCategory::Utilities);
        assert_eq!(extension.install.method, InstallMethod::Script);
    }

    #[test]
    fn test_extension_category_parsing() {
        let categories = vec![
            ("base", ExtensionCategory::Base),
            ("language", ExtensionCategory::Language),
            ("dev-tools", ExtensionCategory::DevTools),
            ("ai", ExtensionCategory::Ai),
            ("infrastructure", ExtensionCategory::Infrastructure),
            ("utilities", ExtensionCategory::Utilities),
            ("desktop", ExtensionCategory::Desktop),
            ("monitoring", ExtensionCategory::Monitoring),
            ("database", ExtensionCategory::Database),
            ("mobile", ExtensionCategory::Mobile),
            ("agile", ExtensionCategory::Agile),
        ];

        for (yaml_value, expected_category) in categories {
            let yaml = format!(
                r#"
metadata:
  name: test
  version: "1.0.0"
  description: Test
  category: {}

install:
  method: script
  script:
    path: test.sh

validate:
  commands: []
"#,
                yaml_value
            );

            let extension: Extension = serde_yaml::from_str(&yaml).unwrap();
            assert_eq!(extension.metadata.category, expected_category);
        }
    }

    #[test]
    fn test_install_method_parsing() {
        let methods = vec![
            ("mise", InstallMethod::Mise),
            ("apt", InstallMethod::Apt),
            ("binary", InstallMethod::Binary),
            ("npm", InstallMethod::Npm),
            ("npm-global", InstallMethod::NpmGlobal),
            ("script", InstallMethod::Script),
            ("hybrid", InstallMethod::Hybrid),
        ];

        for (yaml_value, expected_method) in methods {
            let yaml = format!(
                r#"
metadata:
  name: test
  version: "1.0.0"
  description: Test
  category: utilities

install:
  method: {}

validate:
  commands: []
"#,
                yaml_value
            );

            let extension: Extension = serde_yaml::from_str(&yaml).unwrap();
            assert_eq!(extension.install.method, expected_method);
        }
    }

    #[test]
    fn test_capabilities_parsing() {
        let yaml = r#"
metadata:
  name: test
  version: "1.0.0"
  description: Test with capabilities
  category: ai

install:
  method: script
  script:
    path: install.sh

validate:
  commands: []

capabilities:
  project-init:
    enabled: true
    priority: 100
    commands:
      - command: "echo 'init'"
        description: "Initialize project"
        requiresAuth: anthropic
        conditional: false
    state-markers:
      - path: ".initialized"
        type: file
  auth:
    provider: anthropic
    required: true
    methods:
      - api-key
      - cli-auth
    envVars:
      - ANTHROPIC_API_KEY
  mcp:
    enabled: true
    server:
      command: node
      args:
        - server.js
"#;

        let extension: Extension = serde_yaml::from_str(yaml).unwrap();
        assert!(extension.capabilities.is_some());

        let caps = extension.capabilities.as_ref().unwrap();

        // Test project-init
        assert!(caps.project_init.is_some());
        let project_init = caps.project_init.as_ref().unwrap();
        assert!(project_init.enabled);
        assert_eq!(project_init.priority, 100);
        assert_eq!(project_init.commands.len(), 1);
        assert_eq!(project_init.state_markers.len(), 1);

        // Test auth
        assert!(caps.auth.is_some());
        let auth = caps.auth.as_ref().unwrap();
        assert_eq!(auth.provider, AuthProvider::Anthropic);
        assert!(auth.required);
        assert_eq!(auth.methods.len(), 2);
        assert!(auth.methods.contains(&AuthMethod::ApiKey));
        assert!(auth.methods.contains(&AuthMethod::CliAuth));

        // Test MCP
        assert!(caps.mcp.is_some());
        let mcp = caps.mcp.as_ref().unwrap();
        assert!(mcp.enabled);
        assert!(mcp.server.is_some());
    }

    #[test]
    fn test_bom_config_parsing() {
        let yaml = r#"
metadata:
  name: test
  version: "1.0.0"
  description: Test with BOM
  category: language

install:
  method: mise
  mise:
    configFile: mise.toml

validate:
  commands:
    - name: python
      versionFlag: "--version"

bom:
  tools:
    - name: python
      version: "3.13.0"
      source: mise
      type: runtime
      license: PSF
      homepage: https://python.org
  files:
    - path: /usr/local/bin/python
      type: binary
"#;

        let extension: Extension = serde_yaml::from_str(yaml).unwrap();
        assert!(extension.bom.is_some());

        let bom = extension.bom.as_ref().unwrap();
        assert_eq!(bom.tools.len(), 1);
        assert_eq!(bom.files.len(), 1);

        let tool = &bom.tools[0];
        assert_eq!(tool.name, "python");
        assert_eq!(tool.version, Some("3.13.0".to_string()));
        assert_eq!(tool.source, BomSource::Mise);
        assert_eq!(tool.r#type, Some(BomToolType::Runtime));
    }
}
