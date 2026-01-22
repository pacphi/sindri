//! Test fixture loading helpers
//!
//! Provides functions for loading YAML test fixtures from the fixtures/ directory.

use std::path::PathBuf;

/// Get the path to the fixtures directory
fn fixtures_dir() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .join("tests")
        .join("fixtures")
}

/// Load a fixture file as a string
pub fn load_fixture(filename: &str) -> String {
    let path = fixtures_dir().join(filename);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to load fixture '{}': {}", path.display(), e))
}

// Pre-defined fixture loading functions for convenience

/// Load the standard v1 compatibility matrix
pub fn load_matrix_v1() -> String {
    load_fixture("compatibility_matrix_v1.yaml")
}

/// Load the conflicts compatibility matrix (v4.0.0 with breaking changes)
pub fn load_matrix_conflicts() -> String {
    load_fixture("compatibility_matrix_conflicts.yaml")
}

/// Load the complex version requirements matrix
pub fn load_matrix_complex() -> String {
    load_fixture("compatibility_matrix_complex.yaml")
}

/// Load the multi-version compatibility matrix
pub fn load_matrix_multi_version() -> String {
    load_fixture("compatibility_matrix_multi_version.yaml")
}

/// Load the empty compatibility matrix
pub fn load_matrix_empty() -> String {
    load_fixture("compatibility_matrix_empty.yaml")
}

/// Load the schema v2 compatibility matrix
pub fn load_matrix_schema_v2() -> String {
    load_fixture("compatibility_matrix_schema_v2.yaml")
}
