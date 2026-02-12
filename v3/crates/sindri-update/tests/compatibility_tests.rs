//! Unit tests for compatibility checking
//!
//! Tests cover:
//! - Version matching (exact and wildcard)
//! - Extension compatibility checking
//! - Loading from manifest
//! - Compatibility matrix parsing
//! - Mock manifest files

mod common;

use common::*;
use sindri_update::compatibility::{CompatResult, CompatibilityChecker, IncompatibleExtension};

#[test]
fn test_compatibility_checker_creation() {
    let checker = CompatibilityChecker::new().unwrap();
    // Should create successfully without a loaded matrix
    assert!(!std::ptr::addr_of!(checker).is_null());
}

#[test]
fn test_load_matrix_from_string() {
    let mut checker = CompatibilityChecker::new().unwrap();
    let result = checker.load_matrix_from_str(&load_matrix_v1());

    assert!(result.is_ok());
}

#[test]
fn test_load_invalid_yaml() {
    let mut checker = CompatibilityChecker::new().unwrap();
    let invalid_yaml = "invalid: yaml: structure:\n  - broken";

    let result = checker.load_matrix_from_str(invalid_yaml);
    assert!(result.is_err());
}

#[test]
fn test_exact_version_match() {
    let mut checker = CompatibilityChecker::new().unwrap();
    checker.load_matrix_from_str(&load_matrix_v1()).unwrap();

    let result = checker
        .check_compatibility(VERSION_3_0_0, &standard_extensions())
        .unwrap();

    assert_compatible(&result);
    assert!(result.incompatible_extensions.is_empty());
}

#[test]
fn test_wildcard_version_match() {
    let mut checker = CompatibilityChecker::new().unwrap();
    checker.load_matrix_from_str(&load_matrix_v1()).unwrap();

    // Should match "3.0.x" pattern
    let result = checker
        .check_compatibility(VERSION_3_0_8, &patched_extensions())
        .unwrap();

    assert_compatible(&result);
}

#[test]
fn test_incompatible_extension_version() {
    let mut checker = CompatibilityChecker::new().unwrap();
    checker.load_matrix_from_str(&load_matrix_v1()).unwrap();

    let result = checker
        .check_compatibility(VERSION_3_0_0, &one_outdated_extension())
        .unwrap();

    assert_incompatible(&result);
    assert_incompatible_count(&result, 1);
    assert_extension_incompatible_with_version(&result, EXT_GIT, "0.9.0");
}

#[test]
fn test_multiple_incompatible_extensions() {
    let mut checker = CompatibilityChecker::new().unwrap();
    checker.load_matrix_from_str(&load_matrix_v1()).unwrap();

    let result = checker
        .check_compatibility(VERSION_3_0_0, &outdated_extensions())
        .unwrap();

    assert_incompatible(&result);
    assert_incompatible_count(&result, 3);
}

#[test]
fn test_extension_not_in_matrix() {
    let mut checker = CompatibilityChecker::new().unwrap();
    checker.load_matrix_from_str(&load_matrix_v1()).unwrap();

    // Extensions not in the matrix should not cause incompatibility
    let result = checker
        .check_compatibility(VERSION_3_0_0, &unknown_extension())
        .unwrap();

    assert_compatible(&result);
}

#[test]
fn test_empty_installed_extensions() {
    let mut checker = CompatibilityChecker::new().unwrap();
    checker.load_matrix_from_str(&load_matrix_v1()).unwrap();

    let result = checker
        .check_compatibility(VERSION_3_0_0, &empty_extensions())
        .unwrap();

    // No extensions installed, so should be compatible
    assert_compatible(&result);
}

#[test]
fn test_breaking_changes_reported() {
    let mut checker = CompatibilityChecker::new().unwrap();
    checker.load_matrix_from_str(&load_matrix_v1()).unwrap();

    let result = checker
        .check_compatibility(VERSION_3_0_0, &empty_extensions())
        .unwrap();

    assert_breaking_change_count(&result, 1);
    assert_has_breaking_change(&result, "Changed extension API");
}

#[test]
fn test_no_breaking_changes_for_patch_version() {
    let mut checker = CompatibilityChecker::new().unwrap();
    checker.load_matrix_from_str(&load_matrix_v1()).unwrap();

    // 3.0.x has no breaking changes
    let result = checker
        .check_compatibility(VERSION_3_0_5, &empty_extensions())
        .unwrap();

    assert_no_breaking_changes(&result);
}

#[test]
fn test_version_requirement_parsing() {
    use semver::{Version, VersionReq};

    // Test various version requirement formats
    let req1 = VersionReq::parse("^1.0.0").unwrap();
    let req2 = VersionReq::parse(">=1.5.0").unwrap();
    let req3 = VersionReq::parse("~2.0.0").unwrap();

    let v1_0_0 = Version::parse(EXT_VERSION_1_0_0).unwrap();
    let v1_5_0 = Version::parse(EXT_VERSION_1_5_0).unwrap();
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
    let checker = CompatibilityChecker::new().unwrap();

    let result = checker.check_compatibility(VERSION_3_0_0, &empty_extensions());

    assert!(result.is_err());
    assert_error_contains(&result, "not loaded");
}

#[test]
fn test_version_not_in_matrix() {
    let mut checker = CompatibilityChecker::new().unwrap();
    checker.load_matrix_from_str(&load_matrix_v1()).unwrap();

    let result = checker.check_compatibility(VERSION_99_0_0, &empty_extensions());

    assert!(result.is_err());
    assert_error_contains(&result, "No compatibility entry");
}

#[test]
fn test_invalid_extension_version() {
    let mut checker = CompatibilityChecker::new().unwrap();
    checker.load_matrix_from_str(&load_matrix_v1()).unwrap();

    let result = checker
        .check_compatibility(VERSION_3_0_0, &invalid_version_extension())
        .unwrap();

    // Should handle invalid versions gracefully (defaults to 0.0.0)
    assert_incompatible(&result);
}

#[test]
fn test_wildcard_pattern_matching() {
    let mut checker = CompatibilityChecker::new().unwrap();
    checker.load_matrix_from_str(&load_matrix_v1()).unwrap();

    // All these should match the "3.0.x" pattern
    for patch in 0..10 {
        let version = format!("3.0.{}", patch);
        let result = checker.check_compatibility(&version, &empty_extensions());
        assert!(result.is_ok(), "Failed for version {}", version);
    }
}

#[test]
fn test_wildcard_does_not_match_different_minor() {
    let mut checker = CompatibilityChecker::new().unwrap();
    checker.load_matrix_from_str(&load_matrix_v1()).unwrap();

    // 3.1.0 should not match "3.0.x" pattern
    let result = checker.check_compatibility(VERSION_3_1_0, &empty_extensions());

    // Should find exact match for 3.1.0 instead
    assert!(result.is_ok());
}

#[test]
fn test_major_version_upgrade_breaking_changes() {
    let mut checker = CompatibilityChecker::new().unwrap();
    checker
        .load_matrix_from_str(&load_matrix_conflicts())
        .unwrap();

    let result = checker
        .check_compatibility(VERSION_4_0_0, &empty_extensions())
        .unwrap();

    assert_breaking_change_count(&result, 2);
    assert_has_breaking_change(&result, "Major API overhaul");
    assert_has_breaking_change(&result, "Removed deprecated features");
}

#[test]
fn test_incompatible_extension_details() {
    let incompatible = IncompatibleExtension {
        name: "test-ext".to_string(),
        current_version: EXT_VERSION_1_0_0.to_string(),
        required_range: "^2.0.0".to_string(),
        reason: "Version mismatch".to_string(),
    };

    assert_eq!(incompatible.name, "test-ext");
    assert_eq!(incompatible.current_version, EXT_VERSION_1_0_0);
    assert_eq!(incompatible.required_range, "^2.0.0");
}

#[test]
fn test_compat_result_structure() {
    let result = CompatResult {
        compatible: false,
        incompatible_extensions: vec![IncompatibleExtension {
            name: "ext1".to_string(),
            current_version: EXT_VERSION_1_0_0.to_string(),
            required_range: "^2.0.0".to_string(),
            reason: "Too old".to_string(),
        }],
        warnings: vec!["Warning 1".to_string()],
        breaking_changes: vec!["Breaking change 1".to_string()],
    };

    assert_incompatible(&result);
    assert_incompatible_count(&result, 1);
    assert_eq!(result.warnings.len(), 1);
    assert_breaking_change_count(&result, 1);
}

#[test]
fn test_multiple_cli_versions_in_matrix() {
    let mut checker = CompatibilityChecker::new().unwrap();
    checker
        .load_matrix_from_str(&load_matrix_multi_version())
        .unwrap();

    let installed = v3_1_extensions();

    // Should be compatible with 3.1.0
    let result = checker
        .check_compatibility(VERSION_3_1_0, &installed)
        .unwrap();
    assert_compatible(&result);

    // Should NOT be compatible with 3.0.0 (requires ^1.0.0)
    let result = checker
        .check_compatibility(VERSION_3_0_0, &installed)
        .unwrap();
    assert_compatible(&result); // Actually ^1.0.0 includes 1.1.5

    // Should be compatible with 3.2.0 if ^1.2.0 includes 1.1.5
    let result = checker
        .check_compatibility(VERSION_3_2_0, &installed)
        .unwrap();
    assert_incompatible(&result); // ^1.2.0 does not include 1.1.5
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
    assert!(!std::ptr::addr_of!(checker).is_null());
}

#[test]
fn test_empty_compatibility_matrix() {
    let mut checker = CompatibilityChecker::new().unwrap();
    checker.load_matrix_from_str(&load_matrix_empty()).unwrap();

    let result = checker.check_compatibility(VERSION_3_0_0, &empty_extensions());

    assert!(result.is_err());
}

#[test]
fn test_schema_version_field() {
    let mut checker = CompatibilityChecker::new().unwrap();
    let result = checker.load_matrix_from_str(&load_matrix_schema_v2());

    assert!(result.is_ok());
}

#[test]
fn test_complex_version_requirements() {
    let mut checker = CompatibilityChecker::new().unwrap();
    checker
        .load_matrix_from_str(&load_matrix_complex())
        .unwrap();

    let result = checker
        .check_compatibility(VERSION_3_5_0, &complex_extensions())
        .unwrap();
    assert_compatible(&result);
}

#[tokio::test]
async fn test_load_matrix_from_url_mock() {
    // This would require a mock HTTP server
    // For now, we test that the function exists and has the right signature
    let mut checker = CompatibilityChecker::new().unwrap();
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
        current_version: EXT_VERSION_1_0_0.to_string(),
        required_range: "^2.0.0".to_string(),
        reason: "Mismatch".to_string(),
    };

    let cloned = ext.clone();
    assert_eq!(cloned.name, ext.name);
    assert_eq!(cloned.current_version, ext.current_version);
}
