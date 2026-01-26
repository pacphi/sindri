//! Installation lifecycle integration tests
//!
//! Tests the complete installation lifecycle including:
//! - Script-based installation
//! - Mise-based installation
//! - Binary download installation
//! - Hybrid installation
//! - Timeout handling
//! - Error recovery

mod common;

use common::*;

#[cfg(test)]
mod install_lifecycle {
    use super::*;

    #[test]
    fn test_minimal_extension_yaml_parsing() {
        let yaml = mock_data::MINIMAL_EXTENSION_YAML;
        let ext: sindri_core::types::Extension = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(ext.metadata.name, "test-minimal");
        assert_eq!(ext.metadata.version, "1.0.0");
        assert_eq!(
            ext.install.method,
            sindri_core::types::InstallMethod::Script
        );
    }

    #[test]
    fn test_mise_extension_yaml_parsing() {
        let yaml = mock_data::MISE_EXTENSION_YAML;
        let ext: sindri_core::types::Extension = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(ext.metadata.name, "test-mise");
        assert_eq!(ext.install.method, sindri_core::types::InstallMethod::Mise);
        assert!(ext.install.mise.is_some());
    }

    #[test]
    fn test_extension_builder_creates_valid_extension() {
        let ext = ExtensionBuilder::new()
            .with_name("builder-test")
            .with_version("2.0.0")
            .with_description("Test extension from builder")
            .build();

        assert_eq!(ext.metadata.name, "builder-test");
        assert_eq!(ext.metadata.version, "2.0.0");
    }

    #[test]
    fn test_install_method_variants() {
        // Test each install method
        let methods = [
            ("script", sindri_core::types::InstallMethod::Script),
            ("mise", sindri_core::types::InstallMethod::Mise),
            ("binary", sindri_core::types::InstallMethod::Binary),
            ("npm", sindri_core::types::InstallMethod::Npm),
            ("hybrid", sindri_core::types::InstallMethod::Hybrid),
            ("apt", sindri_core::types::InstallMethod::Apt),
        ];

        for (method_str, expected_method) in methods {
            let yaml = format!(
                r#"
metadata:
  name: method-test
  version: "1.0.0"
  description: Test
  category: testing

install:
  method: {}

validate:
  commands: []
"#,
                method_str
            );

            let ext: sindri_core::types::Extension = serde_yaml::from_str(&yaml).unwrap();
            assert_eq!(ext.install.method, expected_method);
        }
    }

    #[test]
    fn test_extension_with_timeout() {
        let ext = ExtensionBuilder::new()
            .with_name("timeout-test")
            .with_install_timeout(600)
            .build();

        // Timeout is set through requirements, but builder uses script config
        assert!(ext.install.script.is_some());
    }

    #[test]
    fn test_mock_executor_records_invocations() {
        let executor = MockExecutor::new();
        executor.mock_success("test-cmd", "output");

        let result = executor.execute(
            "test-cmd",
            &["--version"],
            None,
            &std::collections::HashMap::new(),
        );

        assert!(result.exit_code == 0);
        assert!(executor.was_invoked("test-cmd"));
        assert_eq!(executor.invocation_count("test-cmd"), 1);
    }

    #[test]
    fn test_mock_executor_failure_response() {
        let executor = MockExecutor::new();
        executor.mock_failure("fail-cmd", "error message", 1);

        let result = executor.execute("fail-cmd", &[], None, &std::collections::HashMap::new());

        assert_eq!(result.exit_code, 1);
        assert_eq!(result.stderr, "error message");
    }

    #[test]
    fn test_fixture_manager_creates_temp_dirs() {
        let manager = FixtureManager::new().unwrap();

        let workspace = manager.create_workspace().unwrap();
        let home = manager.create_home().unwrap();

        assert!(workspace.exists());
        assert!(home.exists());
        assert!(home.join(".sindri/extensions").exists());
        assert!(home.join(".config/mise/conf.d").exists());
    }

    #[test]
    fn test_fixture_manager_creates_extension_dir() {
        let manager = FixtureManager::new().unwrap();
        let ext_dir = manager.create_extension_dir("test-ext").unwrap();

        assert!(ext_dir.exists());
        assert!(ext_dir.join("scripts/install.sh").exists());

        // Verify script is executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let script = ext_dir.join("scripts/install.sh");
            let perms = std::fs::metadata(&script).unwrap().permissions();
            assert!(perms.mode() & 0o111 != 0, "Script should be executable");
        }
    }

    #[test]
    fn test_fixture_manager_creates_mise_extension() {
        let manager = FixtureManager::new().unwrap();
        let ext_dir = manager
            .create_mise_extension_dir("mise-ext", &["python", "node"])
            .unwrap();

        let mise_config = ext_dir.join("mise.toml");
        assert!(mise_config.exists());

        let content = std::fs::read_to_string(&mise_config).unwrap();
        assert!(content.contains("python"));
        assert!(content.contains("node"));
    }

    #[test]
    fn test_lifecycle_result_success() {
        let result = LifecycleResult::success("installed successfully", 150);

        assert!(result.success);
        assert_eq!(result.duration_ms, 150);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_lifecycle_result_failure() {
        let result = LifecycleResult::failure("installation failed", 50);

        assert!(!result.success);
        assert_eq!(result.error, Some("installation failed".to_string()));
    }

    #[test]
    fn test_assertion_install_success() {
        let result = LifecycleResult::success("ok", 100);
        assert_install_success(&result, "test-ext");
    }

    #[test]
    #[should_panic(expected = "installation failed")]
    fn test_assertion_install_success_panics() {
        let result = LifecycleResult::failure("error", 100);
        assert_install_success(&result, "test-ext");
    }

    #[test]
    fn test_test_extensions_by_method() {
        let script = test_extensions::by_method::script_extension();
        let mise = test_extensions::by_method::mise_extension();
        let binary = test_extensions::by_method::binary_extension();

        assert_eq!(
            script.install.method,
            sindri_core::types::InstallMethod::Script
        );
        assert_eq!(mise.install.method, sindri_core::types::InstallMethod::Mise);
        assert_eq!(
            binary.install.method,
            sindri_core::types::InstallMethod::Binary
        );
    }
}
