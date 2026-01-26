//! Validation lifecycle integration tests
//!
//! Tests extension validation including:
//! - Command validation
//! - Pattern matching validation
//! - Mise tool validation
//! - Validation failure handling

mod common;

use common::*;

#[cfg(test)]
mod validate_lifecycle {
    use super::*;

    #[test]
    fn test_simple_command_validation() {
        let ext = test_extensions::with_validation::simple_command();

        assert!(!ext.validate.commands.is_empty());
        let cmd = &ext.validate.commands[0];
        assert_eq!(cmd.name, "echo");
        assert_eq!(cmd.version_flag, "test");
        assert!(cmd.expected_pattern.is_none());
    }

    #[test]
    fn test_pattern_matching_validation() {
        let ext = test_extensions::with_validation::pattern_matching();

        let cmd = &ext.validate.commands[0];
        assert_eq!(cmd.name, "test-cmd");
        assert!(cmd.expected_pattern.is_some());
        assert_eq!(cmd.expected_pattern, Some(r"\d+\.\d+\.\d+".to_string()));
    }

    #[test]
    fn test_multiple_commands_validation() {
        let ext = test_extensions::with_validation::multiple_commands();

        assert_eq!(ext.validate.commands.len(), 2);
        assert_eq!(ext.validate.commands[0].name, "cmd1");
        assert_eq!(ext.validate.commands[1].name, "cmd2");
    }

    #[test]
    fn test_validation_with_mock_executor() {
        let executor = MockExecutor::new();

        // Mock successful command execution
        executor.mock_success("test-cmd", "test-cmd version 1.2.3");

        let result = executor.execute(
            "test-cmd",
            &["--version"],
            None,
            &std::collections::HashMap::new(),
        );

        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("1.2.3"));
    }

    #[test]
    fn test_validation_command_not_found() {
        let executor = MockExecutor::new();

        // Mock command not found
        executor.mock_failure("nonexistent", "command not found", 127);

        let result = executor.execute(
            "nonexistent",
            &["--version"],
            None,
            &std::collections::HashMap::new(),
        );

        assert_eq!(result.exit_code, 127);
        assert!(result.stderr.contains("not found"));
    }

    #[test]
    fn test_validation_pattern_regex() {
        let pattern = r"\d+\.\d+\.\d+";
        let regex = regex::Regex::new(pattern).unwrap();

        assert!(regex.is_match("1.2.3"));
        assert!(regex.is_match("10.20.30"));
        assert!(!regex.is_match("version"));
        assert!(!regex.is_match("1.2"));
    }

    #[test]
    fn test_command_validation_builder() {
        let cmd = CommandValidationBuilder::new("my-cmd")
            .with_version_flag("-V")
            .with_expected_pattern(r"my-cmd v\d+")
            .build();

        assert_eq!(cmd.name, "my-cmd");
        assert_eq!(cmd.version_flag, "-V");
        assert!(cmd.expected_pattern.is_some());
    }

    #[test]
    fn test_mise_validation_config() {
        let yaml = r#"
metadata:
  name: mise-validate-test
  version: "1.0.0"
  description: Test mise validation
  category: languages

install:
  method: mise
  mise:
    configFile: mise.toml

validate:
  mise:
    tools:
      - python@latest
      - node@lts
"#;

        let ext: sindri_core::types::Extension = serde_yaml::from_str(yaml).unwrap();

        assert!(ext.validate.mise.is_some());
        let mise_validate = ext.validate.mise.unwrap();
        assert_eq!(mise_validate.tools.len(), 2);
        assert!(mise_validate.tools.contains(&"python@latest".to_string()));
        assert!(mise_validate.tools.contains(&"node@lts".to_string()));
    }

    #[test]
    fn test_validation_result_assertions() {
        let success_result = LifecycleResult::success("validation passed", 50);
        let failure_result = LifecycleResult::failure("command not found", 10);

        assert_validation_passed(&success_result, "test-ext");
        assert_validation_failed(&failure_result, "test-ext");
    }

    #[test]
    fn test_validation_timeout() {
        let result = LifecycleResult::success("ok", 100);
        assert_within_timeout(&result, 200);
    }

    #[test]
    #[should_panic(expected = "exceeding timeout")]
    fn test_validation_timeout_exceeded() {
        let result = LifecycleResult::success("ok", 500);
        assert_within_timeout(&result, 100);
    }
}
