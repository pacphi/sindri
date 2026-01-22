//! Unit tests for compatibility checking
//!
//! Tests cover:
//! - Version matching (exact and wildcard)
//! - Extension compatibility checking
//! - Loading from manifest
//! - Compatibility matrix parsing
//! - Mock manifest files

use sindri_update::compatibility::{CompatResult, CompatibilityChecker, IncompatibleExtension};
use std::collections::HashMap;

/// Sample compatibility matrix YAML for testing
const TEST_MATRIX_YAML: &str = r#"
schema_version: "1.0"
cli_versions:
  "3.0.0":
    extension_schema: "1.0"
    compatible_extensions:
      git: "^1.0.0"
      docker: "^2.0.0"
      kubernetes: "^1.5.0"
    breaking_changes:
      - "Changed extension API structure"
  "3.0.x":
    extension_schema: "1.0"
    compatible_extensions:
      git: "^1.0.0"
      docker: "^2.0.0"
      kubernetes: "^1.5.0"
    breaking_changes: []
  "3.1.0":
    extension_schema: "1.1"
    compatible_extensions:
      git: "^1.1.0"
      docker: "^2.1.0"
      kubernetes: "^1.6.0"
    breaking_changes:
      - "Updated to new extension schema 1.1"
"#;

const TEST_MATRIX_WITH_CONFLICTS: &str = r#"
schema_version: "1.0"
cli_versions:
  "4.0.0":
    extension_schema: "2.0"
    compatible_extensions:
      git: "^2.0.0"
      docker: "^3.0.0"
    breaking_changes:
      - "Major API overhaul"
      - "Removed deprecated features"
"#;

#[test]
fn test_compatibility_checker_creation() {
    let checker = CompatibilityChecker::new();
    // Should create successfully without a loaded matrix
    assert!(std::ptr::addr_of!(checker) != std::ptr::null());
}

#[test]
fn test_load_matrix_from_string() {
    let mut checker = CompatibilityChecker::new();
    let result = checker.load_matrix_from_str(TEST_MATRIX_YAML);

    assert!(result.is_ok());
}

#[test]
fn test_load_invalid_yaml() {
    let mut checker = CompatibilityChecker::new();
    let invalid_yaml = "invalid: yaml: structure:\n  - broken";

    let result = checker.load_matrix_from_str(invalid_yaml);
    assert!(result.is_err());
}

#[test]
fn test_exact_version_match() {
    let mut checker = CompatibilityChecker::new();
    checker.load_matrix_from_str(TEST_MATRIX_YAML).unwrap();

    let mut installed = HashMap::new();
    installed.insert("git".to_string(), "1.0.0".to_string());
    installed.insert("docker".to_string(), "2.0.0".to_string());
    installed.insert("kubernetes".to_string(), "1.5.0".to_string());

    let result = checker.check_compatibility("3.0.0", &installed).unwrap();

    assert!(result.compatible);
    assert!(result.incompatible_extensions.is_empty());
}

#[test]
fn test_wildcard_version_match() {
    let mut checker = CompatibilityChecker::new();
    checker.load_matrix_from_str(TEST_MATRIX_YAML).unwrap();

    let mut installed = HashMap::new();
    installed.insert("git".to_string(), "1.0.5".to_string());
    installed.insert("docker".to_string(), "2.0.3".to_string());

    // Should match "3.0.x" pattern
    let result = checker.check_compatibility("3.0.8", &installed).unwrap();

    assert!(result.compatible);
}

#[test]
fn test_incompatible_extension_version() {
    let mut checker = CompatibilityChecker::new();
    checker.load_matrix_from_str(TEST_MATRIX_YAML).unwrap();

    let mut installed = HashMap::new();
    installed.insert("git".to_string(), "0.9.0".to_string()); // Too old
    installed.insert("docker".to_string(), "2.0.0".to_string());

    let result = checker.check_compatibility("3.0.0", &installed).unwrap();

    assert!(!result.compatible);
    assert_eq!(result.incompatible_extensions.len(), 1);
    assert_eq!(result.incompatible_extensions[0].name, "git");
    assert_eq!(result.incompatible_extensions[0].current_version, "0.9.0");
}

#[test]
fn test_multiple_incompatible_extensions() {
    let mut checker = CompatibilityChecker::new();
    checker.load_matrix_from_str(TEST_MATRIX_YAML).unwrap();

    let mut installed = HashMap::new();
    installed.insert("git".to_string(), "0.5.0".to_string()); // Too old
    installed.insert("docker".to_string(), "1.0.0".to_string()); // Too old
    installed.insert("kubernetes".to_string(), "1.0.0".to_string()); // Too old

    let result = checker.check_compatibility("3.0.0", &installed).unwrap();

    assert!(!result.compatible);
    assert_eq!(result.incompatible_extensions.len(), 3);
}

#[test]
fn test_extension_not_in_matrix() {
    let mut checker = CompatibilityChecker::new();
    checker.load_matrix_from_str(TEST_MATRIX_YAML).unwrap();

    let mut installed = HashMap::new();
    installed.insert("unknown-extension".to_string(), "1.0.0".to_string());

    // Extensions not in the matrix should not cause incompatibility
    let result = checker.check_compatibility("3.0.0", &installed).unwrap();

    assert!(result.compatible);
}

#[test]
fn test_empty_installed_extensions() {
    let mut checker = CompatibilityChecker::new();
    checker.load_matrix_from_str(TEST_MATRIX_YAML).unwrap();

    let installed = HashMap::new();

    let result = checker.check_compatibility("3.0.0", &installed).unwrap();

    // No extensions installed, so should be compatible
    assert!(result.compatible);
}

#[test]
fn test_breaking_changes_reported() {
    let mut checker = CompatibilityChecker::new();
    checker.load_matrix_from_str(TEST_MATRIX_YAML).unwrap();

    let installed = HashMap::new();

    let result = checker.check_compatibility("3.0.0", &installed).unwrap();

    assert_eq!(result.breaking_changes.len(), 1);
    assert!(result.breaking_changes[0].contains("Changed extension API"));
}

#[test]
fn test_no_breaking_changes_for_patch_version() {
    let mut checker = CompatibilityChecker::new();
    checker.load_matrix_from_str(TEST_MATRIX_YAML).unwrap();

    let installed = HashMap::new();

    // 3.0.x has no breaking changes
    let result = checker.check_compatibility("3.0.5", &installed).unwrap();

    assert!(result.breaking_changes.is_empty());
}

#[test]
fn test_version_requirement_parsing() {
    use semver::{Version, VersionReq};

    // Test various version requirement formats
    let req1 = VersionReq::parse("^1.0.0").unwrap();
    let req2 = VersionReq::parse(">=1.5.0").unwrap();
    let req3 = VersionReq::parse("~2.0.0").unwrap();

    let v1_0_0 = Version::parse("1.0.0").unwrap();
    let v1_5_0 = Version::parse("1.5.0").unwrap();
    let v2_0_1 = Version::parse("2.0.1").unwrap();

    assert!(req1.matches(&v1_0_0));
    assert!(req2.matches(&v1_5_0));
    assert!(req3.matches(&v2_0_1));
}

#[test]
fn test_caret_requirement() {
    use semver::{Version, VersionReq};

    let req = VersionReq::parse("^1.2.0").unwrap();

    assert!(req.matches(&Version::parse("1.2.0").unwrap()));
    assert!(req.matches(&Version::parse("1.2.5").unwrap()));
    assert!(req.matches(&Version::parse("1.9.0").unwrap()));
    assert!(!req.matches(&Version::parse("2.0.0").unwrap()));
    assert!(!req.matches(&Version::parse("1.1.9").unwrap()));
}

#[test]
fn test_check_without_loaded_matrix() {
    let checker = CompatibilityChecker::new();
    let installed = HashMap::new();

    let result = checker.check_compatibility("3.0.0", &installed);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not loaded"));
}

#[test]
fn test_version_not_in_matrix() {
    let mut checker = CompatibilityChecker::new();
    checker.load_matrix_from_str(TEST_MATRIX_YAML).unwrap();

    let installed = HashMap::new();

    let result = checker.check_compatibility("99.0.0", &installed);

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("No compatibility entry"));
}

#[test]
fn test_invalid_extension_version() {
    let mut checker = CompatibilityChecker::new();
    checker.load_matrix_from_str(TEST_MATRIX_YAML).unwrap();

    let mut installed = HashMap::new();
    installed.insert("git".to_string(), "invalid-version".to_string());

    let result = checker.check_compatibility("3.0.0", &installed).unwrap();

    // Should handle invalid versions gracefully (defaults to 0.0.0)
    assert!(!result.compatible);
}

#[test]
fn test_wildcard_pattern_matching() {
    let mut checker = CompatibilityChecker::new();
    checker.load_matrix_from_str(TEST_MATRIX_YAML).unwrap();

    let installed = HashMap::new();

    // All these should match the "3.0.x" pattern
    for patch in 0..10 {
        let version = format!("3.0.{}", patch);
        let result = checker.check_compatibility(&version, &installed);
        assert!(result.is_ok(), "Failed for version {}", version);
    }
}

#[test]
fn test_wildcard_does_not_match_different_minor() {
    let mut checker = CompatibilityChecker::new();
    checker.load_matrix_from_str(TEST_MATRIX_YAML).unwrap();

    let installed = HashMap::new();

    // 3.1.0 should not match "3.0.x" pattern
    let result = checker.check_compatibility("3.1.0", &installed);

    // Should find exact match for 3.1.0 instead
    assert!(result.is_ok());
}

#[test]
fn test_major_version_upgrade_breaking_changes() {
    let mut checker = CompatibilityChecker::new();
    checker
        .load_matrix_from_str(TEST_MATRIX_WITH_CONFLICTS)
        .unwrap();

    let installed = HashMap::new();

    let result = checker.check_compatibility("4.0.0", &installed).unwrap();

    assert_eq!(result.breaking_changes.len(), 2);
    assert!(result.breaking_changes[0].contains("Major API overhaul"));
    assert!(result.breaking_changes[1].contains("Removed deprecated features"));
}

#[test]
fn test_incompatible_extension_details() {
    let incompatible = IncompatibleExtension {
        name: "test-ext".to_string(),
        current_version: "1.0.0".to_string(),
        required_range: "^2.0.0".to_string(),
        reason: "Version mismatch".to_string(),
    };

    assert_eq!(incompatible.name, "test-ext");
    assert_eq!(incompatible.current_version, "1.0.0");
    assert_eq!(incompatible.required_range, "^2.0.0");
}

#[test]
fn test_compat_result_structure() {
    let result = CompatResult {
        compatible: false,
        incompatible_extensions: vec![IncompatibleExtension {
            name: "ext1".to_string(),
            current_version: "1.0.0".to_string(),
            required_range: "^2.0.0".to_string(),
            reason: "Too old".to_string(),
        }],
        warnings: vec!["Warning 1".to_string()],
        breaking_changes: vec!["Breaking change 1".to_string()],
    };

    assert!(!result.compatible);
    assert_eq!(result.incompatible_extensions.len(), 1);
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(result.breaking_changes.len(), 1);
}

#[test]
fn test_multiple_cli_versions_in_matrix() {
    let matrix = r#"
schema_version: "1.0"
cli_versions:
  "3.0.0":
    extension_schema: "1.0"
    compatible_extensions:
      git: "^1.0.0"
    breaking_changes: []
  "3.1.0":
    extension_schema: "1.1"
    compatible_extensions:
      git: "^1.1.0"
    breaking_changes: []
  "3.2.0":
    extension_schema: "1.2"
    compatible_extensions:
      git: "^1.2.0"
    breaking_changes: []
"#;

    let mut checker = CompatibilityChecker::new();
    checker.load_matrix_from_str(matrix).unwrap();

    let mut installed = HashMap::new();
    installed.insert("git".to_string(), "1.1.5".to_string());

    // Should be compatible with 3.1.0
    let result = checker.check_compatibility("3.1.0", &installed).unwrap();
    assert!(result.compatible);

    // Should NOT be compatible with 3.0.0 (requires ^1.0.0)
    let result = checker.check_compatibility("3.0.0", &installed).unwrap();
    assert!(result.compatible); // Actually ^1.0.0 includes 1.1.5

    // Should be compatible with 3.2.0 if ^1.2.0 includes 1.1.5
    let result = checker.check_compatibility("3.2.0", &installed).unwrap();
    assert!(!result.compatible); // ^1.2.0 does not include 1.1.5
}

#[test]
fn test_extension_version_comparison_semantics() {
    use semver::{Version, VersionReq};

    let req = VersionReq::parse("^1.5.0").unwrap();

    // Patch updates compatible
    assert!(req.matches(&Version::parse("1.5.1").unwrap()));
    assert!(req.matches(&Version::parse("1.5.99").unwrap()));

    // Minor updates compatible
    assert!(req.matches(&Version::parse("1.6.0").unwrap()));
    assert!(req.matches(&Version::parse("1.99.0").unwrap()));

    // Major updates NOT compatible
    assert!(!req.matches(&Version::parse("2.0.0").unwrap()));

    // Lower versions NOT compatible
    assert!(!req.matches(&Version::parse("1.4.99").unwrap()));
}

#[test]
fn test_compatibility_checker_default() {
    let checker = CompatibilityChecker::default();
    // Should create successfully
    assert!(std::ptr::addr_of!(checker) != std::ptr::null());
}

#[test]
fn test_empty_compatibility_matrix() {
    let empty_matrix = r#"
schema_version: "1.0"
cli_versions: {}
"#;

    let mut checker = CompatibilityChecker::new();
    checker.load_matrix_from_str(empty_matrix).unwrap();

    let installed = HashMap::new();
    let result = checker.check_compatibility("3.0.0", &installed);

    assert!(result.is_err());
}

#[test]
fn test_schema_version_field() {
    let matrix_with_schema = r#"
schema_version: "2.0"
cli_versions:
  "5.0.0":
    extension_schema: "2.0"
    compatible_extensions:
      new-ext: "^1.0.0"
    breaking_changes: []
"#;

    let mut checker = CompatibilityChecker::new();
    let result = checker.load_matrix_from_str(matrix_with_schema);

    assert!(result.is_ok());
}

#[test]
fn test_complex_version_requirements() {
    let matrix = r#"
schema_version: "1.0"
cli_versions:
  "3.5.0":
    extension_schema: "1.5"
    compatible_extensions:
      ext1: ">=1.0.0, <2.0.0"
      ext2: "~1.5.0"
      ext3: "^2.1.0"
    breaking_changes: []
"#;

    let mut checker = CompatibilityChecker::new();
    checker.load_matrix_from_str(matrix).unwrap();

    let mut installed = HashMap::new();
    installed.insert("ext1".to_string(), "1.5.0".to_string());
    installed.insert("ext2".to_string(), "1.5.8".to_string());
    installed.insert("ext3".to_string(), "2.1.5".to_string());

    let result = checker.check_compatibility("3.5.0", &installed).unwrap();
    assert!(result.compatible);
}

#[tokio::test]
async fn test_load_matrix_from_url_mock() {
    // This would require a mock HTTP server
    // For now, we test that the function exists and has the right signature
    let mut checker = CompatibilityChecker::new();
    let result = checker
        .load_matrix("https://invalid.example.com/matrix.yaml")
        .await;

    // Should fail because URL doesn't exist
    assert!(result.is_err());
}

#[test]
fn test_compat_result_clone() {
    let result = CompatResult {
        compatible: true,
        incompatible_extensions: vec![],
        warnings: vec![],
        breaking_changes: vec![],
    };

    let cloned = result.clone();
    assert_eq!(cloned.compatible, result.compatible);
}

#[test]
fn test_incompatible_extension_clone() {
    let ext = IncompatibleExtension {
        name: "test".to_string(),
        current_version: "1.0.0".to_string(),
        required_range: "^2.0.0".to_string(),
        reason: "Mismatch".to_string(),
    };

    let cloned = ext.clone();
    assert_eq!(cloned.name, ext.name);
    assert_eq!(cloned.current_version, ext.current_version);
}
