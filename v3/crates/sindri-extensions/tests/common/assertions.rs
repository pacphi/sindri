//! Assertion helpers for extension lifecycle testing
//!
//! Provides specialized assertion functions for validating
//! extension installation, validation, and removal operations.

#![allow(dead_code)]

use sindri_core::types::Extension;
use std::path::Path;

/// Result of a lifecycle operation for assertions
#[derive(Debug, Clone)]
pub struct LifecycleResult {
    pub success: bool,
    pub duration_ms: u64,
    pub output: String,
    pub error: Option<String>,
}

impl LifecycleResult {
    pub fn success(output: &str, duration_ms: u64) -> Self {
        Self {
            success: true,
            duration_ms,
            output: output.to_string(),
            error: None,
        }
    }

    pub fn failure(error: &str, duration_ms: u64) -> Self {
        Self {
            success: false,
            duration_ms,
            output: String::new(),
            error: Some(error.to_string()),
        }
    }
}

/// Assert that an extension installed successfully
pub fn assert_install_success(result: &LifecycleResult, ext_name: &str) {
    assert!(
        result.success,
        "Extension '{}' installation failed: {:?}",
        ext_name, result.error
    );
}

/// Assert that an extension installation failed
pub fn assert_install_failure(result: &LifecycleResult, ext_name: &str) {
    assert!(
        !result.success,
        "Extension '{}' installation unexpectedly succeeded",
        ext_name
    );
}

/// Assert that validation passed
pub fn assert_validation_passed(result: &LifecycleResult, ext_name: &str) {
    assert!(
        result.success,
        "Extension '{}' validation failed: {:?}",
        ext_name, result.error
    );
}

/// Assert that validation failed
pub fn assert_validation_failed(result: &LifecycleResult, ext_name: &str) {
    assert!(
        !result.success,
        "Extension '{}' validation unexpectedly passed",
        ext_name
    );
}

/// Assert that a hook was executed
pub fn assert_hook_executed(
    tracker: &super::mocks::MockHookTracker,
    ext_name: &str,
    hook_type: &str,
) {
    assert!(
        tracker.was_executed(ext_name, hook_type),
        "Hook '{}:{}' was not executed",
        ext_name,
        hook_type
    );
}

/// Assert that a hook was not executed
pub fn assert_hook_not_executed(
    tracker: &super::mocks::MockHookTracker,
    ext_name: &str,
    hook_type: &str,
) {
    assert!(
        !tracker.was_executed(ext_name, hook_type),
        "Hook '{}:{}' was unexpectedly executed",
        ext_name,
        hook_type
    );
}

/// Assert hook execution order
pub fn assert_hook_order(tracker: &super::mocks::MockHookTracker, expected_order: &[(&str, &str)]) {
    let actual = tracker.get_executed();
    let expected: Vec<(String, String)> = expected_order
        .iter()
        .map(|(e, h)| (e.to_string(), h.to_string()))
        .collect();

    assert_eq!(
        actual, expected,
        "Hook execution order mismatch.\nExpected: {:?}\nActual: {:?}",
        expected, actual
    );
}

/// Assert that a command was executed
pub fn assert_command_executed(executor: &super::mocks::MockExecutor, command: &str) {
    assert!(
        executor.was_invoked(command),
        "Command '{}' was not executed",
        command
    );
}

/// Assert command execution count
pub fn assert_command_count(executor: &super::mocks::MockExecutor, command: &str, count: usize) {
    let actual = executor.invocation_count(command);
    assert_eq!(
        actual, count,
        "Expected '{}' to be executed {} times, but was executed {} times",
        command, count, actual
    );
}

/// Assert that a file exists
pub fn assert_file_exists(path: impl AsRef<Path>) {
    let path = path.as_ref();
    assert!(path.exists(), "File does not exist: {:?}", path);
}

/// Assert that a file does not exist
pub fn assert_file_not_exists(path: impl AsRef<Path>) {
    let path = path.as_ref();
    assert!(!path.exists(), "File unexpectedly exists: {:?}", path);
}

/// Assert that a directory exists
pub fn assert_directory_exists(path: impl AsRef<Path>) {
    let path = path.as_ref();
    assert!(path.is_dir(), "Directory does not exist: {:?}", path);
}

/// Assert file contains content
pub fn assert_file_contains(path: impl AsRef<Path>, content: &str) {
    let path = path.as_ref();
    let file_content = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read file {:?}: {}", path, e));
    assert!(
        file_content.contains(content),
        "File {:?} does not contain expected content.\nExpected: {}\nActual: {}",
        path,
        content,
        file_content
    );
}

/// Assert extension metadata
pub fn assert_extension_metadata(ext: &Extension, name: &str, version: &str) {
    assert_eq!(
        ext.metadata.name, name,
        "Extension name mismatch: expected '{}', got '{}'",
        name, ext.metadata.name
    );
    assert_eq!(
        ext.metadata.version, version,
        "Extension version mismatch: expected '{}', got '{}'",
        version, ext.metadata.version
    );
}

/// Assert extension has dependencies
pub fn assert_has_dependencies(ext: &Extension, expected_deps: &[&str]) {
    let deps = &ext.metadata.dependencies;
    assert!(!deps.is_empty(), "Extension has no dependencies");

    for expected in expected_deps {
        assert!(
            deps.contains(&expected.to_string()),
            "Missing dependency '{}' in {:?}",
            expected,
            deps
        );
    }
}

/// Assert extension has no dependencies
pub fn assert_no_dependencies(ext: &Extension) {
    assert!(
        ext.metadata.dependencies.is_empty(),
        "Extension unexpectedly has dependencies: {:?}",
        ext.metadata.dependencies
    );
}

/// Assert extension has hooks configured
pub fn assert_has_hooks(ext: &Extension) {
    assert!(ext.capabilities.is_some(), "Extension has no capabilities");
    let caps = ext.capabilities.as_ref().unwrap();
    assert!(caps.hooks.is_some(), "Extension has no hooks configured");
}

/// Assert extension has pre-install hook
pub fn assert_has_pre_install_hook(ext: &Extension) {
    assert_has_hooks(ext);
    let hooks = ext.capabilities.as_ref().unwrap().hooks.as_ref().unwrap();
    assert!(
        hooks.pre_install.is_some(),
        "Extension has no pre-install hook"
    );
}

/// Assert extension has post-install hook
pub fn assert_has_post_install_hook(ext: &Extension) {
    assert_has_hooks(ext);
    let hooks = ext.capabilities.as_ref().unwrap().hooks.as_ref().unwrap();
    assert!(
        hooks.post_install.is_some(),
        "Extension has no post-install hook"
    );
}

/// Assert operation completed within timeout
pub fn assert_within_timeout(result: &LifecycleResult, max_ms: u64) {
    assert!(
        result.duration_ms <= max_ms,
        "Operation took {}ms, exceeding timeout of {}ms",
        result.duration_ms,
        max_ms
    );
}

/// Assert operation exceeded timeout (for timeout testing)
pub fn assert_exceeded_timeout(result: &LifecycleResult, min_ms: u64) {
    assert!(
        result.duration_ms >= min_ms,
        "Operation completed in {}ms, but should have exceeded {}ms",
        result.duration_ms,
        min_ms
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::builders::ExtensionBuilder;
    use crate::common::mocks::{MockExecutor, MockHookTracker};

    #[test]
    fn test_assert_install_success() {
        let result = LifecycleResult::success("installed", 100);
        assert_install_success(&result, "test-ext");
    }

    #[test]
    #[should_panic(expected = "installation failed")]
    fn test_assert_install_success_panics_on_failure() {
        let result = LifecycleResult::failure("error", 100);
        assert_install_success(&result, "test-ext");
    }

    #[test]
    fn test_assert_hook_executed() {
        let tracker = MockHookTracker::new();
        tracker.record("ext1", "pre-install");
        assert_hook_executed(&tracker, "ext1", "pre-install");
    }

    #[test]
    fn test_assert_command_executed() {
        let executor = MockExecutor::new();
        executor.mock_success("test-cmd", "output");
        executor.execute("test-cmd", &[], None, &std::collections::HashMap::new());
        assert_command_executed(&executor, "test-cmd");
    }

    #[test]
    fn test_assert_extension_metadata() {
        let ext = ExtensionBuilder::new()
            .with_name("test")
            .with_version("2.0.0")
            .build();
        assert_extension_metadata(&ext, "test", "2.0.0");
    }

    #[test]
    fn test_assert_has_dependencies() {
        let ext = ExtensionBuilder::new()
            .with_dependency("dep1")
            .with_dependency("dep2")
            .build();
        assert_has_dependencies(&ext, &["dep1", "dep2"]);
    }

    #[test]
    fn test_assert_has_hooks() {
        let ext = ExtensionBuilder::with_hooks_preset().build();
        assert_has_hooks(&ext);
        assert_has_pre_install_hook(&ext);
        assert_has_post_install_hook(&ext);
    }

    #[test]
    fn test_assert_within_timeout() {
        let result = LifecycleResult::success("ok", 500);
        assert_within_timeout(&result, 1000);
    }
}
