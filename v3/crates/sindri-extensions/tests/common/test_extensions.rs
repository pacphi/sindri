//! Pre-defined test extensions for common test scenarios
//!
//! Provides ready-to-use extension configurations for testing
//! different installation methods, lifecycle phases, and edge cases.

#![allow(dead_code)]

use super::builders::ExtensionBuilder;
use sindri_core::types::{
    CommandValidation, Extension, ExtensionCategory, HookConfig, InstallMethod, MiseInstallConfig,
    ScriptConfig,
};

/// Create a minimal test extension (script-based)
pub fn minimal_extension() -> Extension {
    ExtensionBuilder::minimal().build()
}

/// Create a mise-based test extension
pub fn mise_extension() -> Extension {
    ExtensionBuilder::mise().build()
}

/// Create an extension with pre and post install hooks
pub fn hooks_extension() -> Extension {
    ExtensionBuilder::with_hooks_preset().build()
}

/// Create an extension with dependencies
pub fn deps_extension(dependencies: Vec<&str>) -> Extension {
    let mut builder = ExtensionBuilder::new().with_name("deps-test");

    for dep in dependencies {
        builder = builder.with_dependency(dep);
    }

    builder.build()
}

/// Create an extension that will fail installation
pub fn failing_extension() -> Extension {
    ExtensionBuilder::new()
        .with_name("failing-ext")
        .with_script_config(ScriptConfig {
            path: "scripts/fail.sh".to_string(),
            timeout: 5,
            args: Vec::new(),
        })
        .build()
}

/// Create an extension with a long timeout (for timeout testing)
pub fn slow_extension(timeout_secs: u32) -> Extension {
    ExtensionBuilder::new()
        .with_name("slow-ext")
        .with_install_timeout(timeout_secs)
        .with_script_config(ScriptConfig {
            path: "scripts/slow.sh".to_string(),
            timeout: timeout_secs,
            args: Vec::new(),
        })
        .build()
}

/// Create an extension for each install method
pub mod by_method {
    use super::*;

    pub fn script_extension() -> Extension {
        ExtensionBuilder::new()
            .with_name("script-test")
            .with_install_method(InstallMethod::Script)
            .build()
    }

    pub fn mise_extension() -> Extension {
        ExtensionBuilder::new()
            .with_name("mise-test")
            .with_install_method(InstallMethod::Mise)
            .with_mise_config(MiseInstallConfig {
                config_file: Some("mise.toml".to_string()),
                reshim_after_install: true,
            })
            .build()
    }

    pub fn binary_extension() -> Extension {
        ExtensionBuilder::new()
            .with_name("binary-test")
            .with_install_method(InstallMethod::Binary)
            .build()
    }

    pub fn npm_extension() -> Extension {
        ExtensionBuilder::new()
            .with_name("npm-test")
            .with_install_method(InstallMethod::Npm)
            .build()
    }

    pub fn hybrid_extension() -> Extension {
        ExtensionBuilder::new()
            .with_name("hybrid-test")
            .with_install_method(InstallMethod::Hybrid)
            .build()
    }

    pub fn apt_extension() -> Extension {
        ExtensionBuilder::new()
            .with_name("apt-test")
            .with_install_method(InstallMethod::Apt)
            .build()
    }
}

/// Create extensions for each category
pub mod by_category {
    use super::*;

    pub fn language_extension() -> Extension {
        ExtensionBuilder::new()
            .with_name("lang-test")
            .with_category(ExtensionCategory::Languages)
            .build()
    }

    pub fn ai_dev_extension() -> Extension {
        ExtensionBuilder::new()
            .with_name("ai-dev-test")
            .with_category(ExtensionCategory::AiDev)
            .build()
    }

    pub fn devops_extension() -> Extension {
        ExtensionBuilder::new()
            .with_name("devops-test")
            .with_category(ExtensionCategory::Devops)
            .build()
    }

    pub fn testing_extension() -> Extension {
        ExtensionBuilder::new()
            .with_name("testing-test")
            .with_category(ExtensionCategory::Testing)
            .build()
    }
}

/// Create extensions with specific hook configurations
pub mod with_hooks {
    use super::*;

    pub fn pre_install_only() -> Extension {
        ExtensionBuilder::new()
            .with_name("pre-hook-test")
            .with_pre_install_hook(HookConfig {
                command: "echo 'pre-install'".to_string(),
                description: Some("Pre-install hook".to_string()),
            })
            .build()
    }

    pub fn post_install_only() -> Extension {
        ExtensionBuilder::new()
            .with_name("post-hook-test")
            .with_post_install_hook(HookConfig {
                command: "echo 'post-install'".to_string(),
                description: Some("Post-install hook".to_string()),
            })
            .build()
    }

    pub fn both_hooks() -> Extension {
        ExtensionBuilder::with_hooks_preset().build()
    }

    pub fn failing_pre_hook() -> Extension {
        ExtensionBuilder::new()
            .with_name("fail-pre-hook")
            .with_pre_install_hook(HookConfig {
                command: "exit 1".to_string(),
                description: Some("Hook that fails".to_string()),
            })
            .build()
    }
}

/// Create extensions with validation configurations
pub mod with_validation {
    use super::*;

    pub fn simple_command() -> Extension {
        ExtensionBuilder::new()
            .with_name("simple-validation")
            .with_validation_commands(vec![CommandValidation {
                name: "echo".to_string(),
                version_flag: "test".to_string(),
                expected_pattern: None,
            }])
            .build()
    }

    pub fn pattern_matching() -> Extension {
        ExtensionBuilder::new()
            .with_name("pattern-validation")
            .with_validation_commands(vec![CommandValidation {
                name: "test-cmd".to_string(),
                version_flag: "--version".to_string(),
                expected_pattern: Some(r"\d+\.\d+\.\d+".to_string()),
            }])
            .build()
    }

    pub fn multiple_commands() -> Extension {
        ExtensionBuilder::new()
            .with_name("multi-validation")
            .with_validation_commands(vec![
                CommandValidation {
                    name: "cmd1".to_string(),
                    version_flag: "--version".to_string(),
                    expected_pattern: None,
                },
                CommandValidation {
                    name: "cmd2".to_string(),
                    version_flag: "-v".to_string(),
                    expected_pattern: None,
                },
            ])
            .build()
    }
}

/// Create extensions for dependency testing
pub mod with_dependencies {
    use super::*;

    pub fn single_dependency() -> Extension {
        ExtensionBuilder::new()
            .with_name("single-dep")
            .with_dependency("base-ext")
            .build()
    }

    pub fn multiple_dependencies() -> Extension {
        ExtensionBuilder::new()
            .with_name("multi-dep")
            .with_dependencies(vec![
                "dep1".to_string(),
                "dep2".to_string(),
                "dep3".to_string(),
            ])
            .build()
    }

    pub fn chain_dependency_a() -> Extension {
        ExtensionBuilder::new().with_name("chain-a").build()
    }

    pub fn chain_dependency_b() -> Extension {
        ExtensionBuilder::new()
            .with_name("chain-b")
            .with_dependency("chain-a")
            .build()
    }

    pub fn chain_dependency_c() -> Extension {
        ExtensionBuilder::new()
            .with_name("chain-c")
            .with_dependency("chain-b")
            .build()
    }

    // For circular dependency testing
    pub fn circular_a() -> Extension {
        ExtensionBuilder::new()
            .with_name("circular-a")
            .with_dependency("circular-b")
            .build()
    }

    pub fn circular_b() -> Extension {
        ExtensionBuilder::new()
            .with_name("circular-b")
            .with_dependency("circular-a")
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal_extension() {
        let ext = minimal_extension();
        assert_eq!(ext.metadata.name, "minimal");
        assert_eq!(ext.install.method, InstallMethod::Script);
    }

    #[test]
    fn test_mise_extension() {
        let ext = mise_extension();
        assert_eq!(ext.install.method, InstallMethod::Mise);
        assert!(ext.install.mise.is_some());
    }

    #[test]
    fn test_hooks_extension() {
        let ext = hooks_extension();
        assert!(ext.capabilities.is_some());
        let hooks = ext.capabilities.unwrap().hooks.unwrap();
        assert!(hooks.pre_install.is_some());
        assert!(hooks.post_install.is_some());
    }

    #[test]
    fn test_deps_extension() {
        let ext = deps_extension(vec!["dep1", "dep2"]);
        let deps = &ext.metadata.dependencies;
        assert_eq!(deps.len(), 2);
    }

    #[test]
    fn test_by_method_extensions() {
        assert_eq!(
            by_method::script_extension().install.method,
            InstallMethod::Script
        );
        assert_eq!(
            by_method::mise_extension().install.method,
            InstallMethod::Mise
        );
        assert_eq!(
            by_method::binary_extension().install.method,
            InstallMethod::Binary
        );
        assert_eq!(
            by_method::npm_extension().install.method,
            InstallMethod::Npm
        );
        assert_eq!(
            by_method::hybrid_extension().install.method,
            InstallMethod::Hybrid
        );
    }

    #[test]
    fn test_by_category_extensions() {
        assert_eq!(
            by_category::language_extension().metadata.category,
            ExtensionCategory::Languages
        );
        assert_eq!(
            by_category::ai_dev_extension().metadata.category,
            ExtensionCategory::AiDev
        );
        assert_eq!(
            by_category::devops_extension().metadata.category,
            ExtensionCategory::Devops
        );
    }

    #[test]
    fn test_dependency_chain() {
        let a = with_dependencies::chain_dependency_a();
        let b = with_dependencies::chain_dependency_b();
        let c = with_dependencies::chain_dependency_c();

        assert!(a.metadata.dependencies.is_empty());
        assert!(b.metadata.dependencies.contains(&"chain-a".to_string()));
        assert!(c.metadata.dependencies.contains(&"chain-b".to_string()));
    }
}
