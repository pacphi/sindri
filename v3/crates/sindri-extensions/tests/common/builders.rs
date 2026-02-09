//! Extension builders for creating test fixtures
//!
//! Provides fluent builders for constructing Extension objects
//! with various configurations for testing purposes.

#![allow(dead_code)]

use sindri_core::types::{
    CapabilitiesConfig, CommandValidation, Extension, ExtensionCategory, ExtensionMetadata,
    HookConfig, HooksCapability, InstallConfig, InstallMethod, MiseInstallConfig, ScriptConfig,
    ValidateConfig,
};

/// Builder for creating Extension test fixtures
pub struct ExtensionBuilder {
    name: String,
    version: String,
    description: String,
    category: ExtensionCategory,
    dependencies: Vec<String>,
    install_method: InstallMethod,
    script_config: Option<ScriptConfig>,
    mise_config: Option<MiseInstallConfig>,
    validate_commands: Vec<CommandValidation>,
    pre_install_hook: Option<HookConfig>,
    post_install_hook: Option<HookConfig>,
    install_timeout: u32,
}

impl ExtensionBuilder {
    /// Create a new ExtensionBuilder with defaults
    pub fn new() -> Self {
        Self {
            name: "test-extension".to_string(),
            version: "1.0.0".to_string(),
            description: "Test extension".to_string(),
            category: ExtensionCategory::Testing,
            dependencies: Vec::new(),
            install_method: InstallMethod::Script,
            script_config: Some(ScriptConfig {
                path: "scripts/install.sh".to_string(),
                timeout: 60,
                args: Vec::new(),
            }),
            mise_config: None,
            validate_commands: vec![CommandValidation {
                name: "echo".to_string(),
                version_flag: "test".to_string(),
                expected_pattern: None,
            }],
            pre_install_hook: None,
            post_install_hook: None,
            install_timeout: 300,
        }
    }

    /// Create a minimal extension for basic tests
    pub fn minimal() -> Self {
        Self::new()
            .with_name("minimal")
            .with_description("Minimal test extension")
    }

    /// Create a mise-based extension
    pub fn mise() -> Self {
        Self::new()
            .with_name("mise-test")
            .with_category(ExtensionCategory::Languages)
            .with_install_method(InstallMethod::Mise)
            .with_mise_config(MiseInstallConfig {
                config_file: Some("mise.toml".to_string()),
                reshim_after_install: true,
            })
    }

    /// Create an extension with hooks
    pub fn with_hooks_preset() -> Self {
        Self::new()
            .with_name("hooks-test")
            .with_pre_install_hook(HookConfig {
                command: "echo 'pre-install'".to_string(),
                description: Some("Pre-install hook".to_string()),
            })
            .with_post_install_hook(HookConfig {
                command: "echo 'post-install'".to_string(),
                description: Some("Post-install hook".to_string()),
            })
    }

    /// Set extension name
    pub fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    /// Set extension version
    pub fn with_version(mut self, version: &str) -> Self {
        self.version = version.to_string();
        self
    }

    /// Set extension description
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    /// Set extension category
    pub fn with_category(mut self, category: ExtensionCategory) -> Self {
        self.category = category;
        self
    }

    /// Add a dependency
    pub fn with_dependency(mut self, dep: &str) -> Self {
        self.dependencies.push(dep.to_string());
        self
    }

    /// Set dependencies
    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.dependencies = deps;
        self
    }

    /// Set install method
    pub fn with_install_method(mut self, method: InstallMethod) -> Self {
        self.install_method = method;
        self
    }

    /// Set script config
    pub fn with_script_config(mut self, config: ScriptConfig) -> Self {
        self.script_config = Some(config);
        self
    }

    /// Set mise config
    pub fn with_mise_config(mut self, config: MiseInstallConfig) -> Self {
        self.mise_config = Some(config);
        self
    }

    /// Add a validation command
    pub fn with_validation_command(mut self, cmd: CommandValidation) -> Self {
        self.validate_commands.push(cmd);
        self
    }

    /// Set validation commands
    pub fn with_validation_commands(mut self, commands: Vec<CommandValidation>) -> Self {
        self.validate_commands = commands;
        self
    }

    /// Set pre-install hook
    pub fn with_pre_install_hook(mut self, hook: HookConfig) -> Self {
        self.pre_install_hook = Some(hook);
        self
    }

    /// Set post-install hook
    pub fn with_post_install_hook(mut self, hook: HookConfig) -> Self {
        self.post_install_hook = Some(hook);
        self
    }

    /// Set install timeout
    pub fn with_install_timeout(mut self, timeout: u32) -> Self {
        self.install_timeout = timeout;
        self
    }

    /// Build the Extension
    pub fn build(self) -> Extension {
        let metadata = ExtensionMetadata {
            name: self.name,
            version: self.version,
            description: self.description,
            category: self.category,
            author: None,
            homepage: None,
            dependencies: self.dependencies,
        };

        let install = InstallConfig {
            method: self.install_method,
            script: self.script_config,
            mise: self.mise_config,
            apt: None,
            binary: None,
            npm: None,
        };

        let validate = ValidateConfig {
            commands: self.validate_commands,
            mise: None,
        };

        // Build capabilities with hooks if configured
        let capabilities = if self.pre_install_hook.is_some() || self.post_install_hook.is_some() {
            Some(CapabilitiesConfig {
                hooks: Some(HooksCapability {
                    pre_install: self.pre_install_hook,
                    post_install: self.post_install_hook,
                    pre_project_init: None,
                    post_project_init: None,
                }),
                project_init: None,
                auth: None,
                mcp: None,
                project_context: None,
                features: None,
                collision_handling: None,
            })
        } else {
            None
        };

        Extension {
            metadata,
            requirements: None,
            install,
            validate,
            configure: None,
            remove: None,
            upgrade: None,
            capabilities,
            docs: None,
            bom: None,
        }
    }
}

/// Builder for CommandValidation
pub struct CommandValidationBuilder {
    name: String,
    version_flag: String,
    expected_pattern: Option<String>,
}

impl CommandValidationBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            version_flag: "--version".to_string(),
            expected_pattern: None,
        }
    }

    pub fn with_version_flag(mut self, flag: &str) -> Self {
        self.version_flag = flag.to_string();
        self
    }

    pub fn with_expected_pattern(mut self, pattern: &str) -> Self {
        self.expected_pattern = Some(pattern.to_string());
        self
    }

    pub fn build(self) -> CommandValidation {
        CommandValidation {
            name: self.name,
            version_flag: self.version_flag,
            expected_pattern: self.expected_pattern,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_builder_default() {
        let ext = ExtensionBuilder::new().build();
        assert_eq!(ext.metadata.name, "test-extension");
        assert_eq!(ext.metadata.version, "1.0.0");
        assert_eq!(ext.install.method, InstallMethod::Script);
    }

    #[test]
    fn test_extension_builder_minimal() {
        let ext = ExtensionBuilder::minimal().build();
        assert_eq!(ext.metadata.name, "minimal");
        assert_eq!(ext.metadata.description, "Minimal test extension");
    }

    #[test]
    fn test_extension_builder_mise() {
        let ext = ExtensionBuilder::mise().build();
        assert_eq!(ext.metadata.name, "mise-test");
        assert_eq!(ext.install.method, InstallMethod::Mise);
        assert!(ext.install.mise.is_some());
    }

    #[test]
    fn test_extension_builder_with_hooks() {
        let ext = ExtensionBuilder::with_hooks_preset().build();
        assert!(ext.capabilities.is_some());
        let caps = ext.capabilities.unwrap();
        assert!(caps.hooks.is_some());
        let hooks = caps.hooks.unwrap();
        assert!(hooks.pre_install.is_some());
        assert!(hooks.post_install.is_some());
    }

    #[test]
    fn test_extension_builder_with_dependencies() {
        let ext = ExtensionBuilder::new()
            .with_dependency("dep1")
            .with_dependency("dep2")
            .build();
        let deps = &ext.metadata.dependencies;
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&"dep1".to_string()));
        assert!(deps.contains(&"dep2".to_string()));
    }

    #[test]
    fn test_command_validation_builder() {
        let cmd = CommandValidationBuilder::new("test-cmd")
            .with_version_flag("--version")
            .with_expected_pattern(r"\d+\.\d+\.\d+")
            .build();
        assert_eq!(cmd.name, "test-cmd");
        assert_eq!(cmd.version_flag, "--version");
        assert!(cmd.expected_pattern.is_some());
    }
}
