//! Assertion helpers for compatibility testing
//!
//! Provides semantic assertion functions that make test code more readable
//! and provide better error messages on failure.

use sindri_update::compatibility::CompatResult;

/// Assert that a compatibility result indicates compatibility
///
/// Fails with a descriptive message if incompatible.
pub fn assert_compatible(result: &CompatResult) {
    assert!(
        result.compatible,
        "Expected compatible result but got incompatible. \
         Incompatible extensions: {:?}",
        result.incompatible_extensions
    );
}

/// Assert that a compatibility result indicates incompatibility
///
/// Fails with a descriptive message if compatible.
pub fn assert_incompatible(result: &CompatResult) {
    assert!(
        !result.compatible,
        "Expected incompatible result but got compatible"
    );
}

/// Assert that a compatibility result has a specific number of incompatible extensions
pub fn assert_incompatible_count(result: &CompatResult, expected: usize) {
    assert_eq!(
        result.incompatible_extensions.len(),
        expected,
        "Expected {} incompatible extensions but got {}. Extensions: {:?}",
        expected,
        result.incompatible_extensions.len(),
        result.incompatible_extensions
    );
}

/// Assert that a specific extension is in the incompatible list
pub fn assert_extension_incompatible(result: &CompatResult, name: &str) {
    assert!(
        result
            .incompatible_extensions
            .iter()
            .any(|e| e.name == name),
        "Expected extension '{}' to be incompatible, but it wasn't found in: {:?}",
        name,
        result.incompatible_extensions
    );
}

/// Assert that a specific extension is incompatible with expected version info
pub fn assert_extension_incompatible_with_version(
    result: &CompatResult,
    name: &str,
    current_version: &str,
) {
    let ext = result
        .incompatible_extensions
        .iter()
        .find(|e| e.name == name);

    assert!(
        ext.is_some(),
        "Expected extension '{}' to be incompatible, but it wasn't found",
        name
    );

    let ext = ext.unwrap();
    assert_eq!(
        ext.current_version, current_version,
        "Expected extension '{}' to have version '{}' but got '{}'",
        name, current_version, ext.current_version
    );
}

/// Assert that the result has no breaking changes
pub fn assert_no_breaking_changes(result: &CompatResult) {
    assert!(
        result.breaking_changes.is_empty(),
        "Expected no breaking changes but found: {:?}",
        result.breaking_changes
    );
}

/// Assert that the result has a specific number of breaking changes
pub fn assert_breaking_change_count(result: &CompatResult, expected: usize) {
    assert_eq!(
        result.breaking_changes.len(),
        expected,
        "Expected {} breaking changes but got {}. Changes: {:?}",
        expected,
        result.breaking_changes.len(),
        result.breaking_changes
    );
}

/// Assert that a breaking change message contains the expected pattern
pub fn assert_has_breaking_change(result: &CompatResult, pattern: &str) {
    assert!(
        result
            .breaking_changes
            .iter()
            .any(|change| change.contains(pattern)),
        "Expected breaking change containing '{}' but found: {:?}",
        pattern,
        result.breaking_changes
    );
}

/// Assert that a warning message contains the expected pattern
pub fn assert_has_warning(result: &CompatResult, pattern: &str) {
    assert!(
        result.warnings.iter().any(|w| w.contains(pattern)),
        "Expected warning containing '{}' but found: {:?}",
        pattern,
        result.warnings
    );
}

/// Assert that an error message contains the expected pattern
pub fn assert_error_contains<T: std::fmt::Debug, E: std::fmt::Display>(
    result: &Result<T, E>,
    pattern: &str,
) {
    assert!(result.is_err(), "Expected error but got Ok");
    let error_msg = result.as_ref().unwrap_err().to_string();
    assert!(
        error_msg.contains(pattern),
        "Expected error containing '{}' but got: {}",
        pattern,
        error_msg
    );
}
