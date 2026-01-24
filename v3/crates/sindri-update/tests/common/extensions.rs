//! Extension HashMap helpers for testing
//!
//! Provides factory functions for creating commonly used extension
//! configurations to eliminate repetitive HashMap setup code.

use std::collections::HashMap;

use super::constants::*;

/// Create a standard set of compatible extensions for version 3.0.0
///
/// Contains:
/// - git: "1.0.0"
/// - docker: "2.0.0"
/// - kubernetes: "1.5.0"
pub fn standard_extensions() -> HashMap<String, String> {
    extensions_from_pairs(&[
        (EXT_GIT, EXT_VERSION_1_0_0),
        (EXT_DOCKER, EXT_VERSION_2_0_0),
        (EXT_KUBERNETES, EXT_VERSION_1_5_0),
    ])
}

/// Create extensions with only git and docker (no kubernetes)
pub fn minimal_extensions() -> HashMap<String, String> {
    extensions_from_pairs(&[
        (EXT_GIT, EXT_VERSION_1_0_0),
        (EXT_DOCKER, EXT_VERSION_2_0_0),
    ])
}

/// Create extensions with patch versions
pub fn patched_extensions() -> HashMap<String, String> {
    extensions_from_pairs(&[(EXT_GIT, "1.0.5"), (EXT_DOCKER, "2.0.3")])
}

/// Create outdated extensions that are incompatible with 3.0.0
///
/// Contains:
/// - git: "0.5.0" (too old)
/// - docker: "1.0.0" (too old)
/// - kubernetes: "1.0.0" (too old)
pub fn outdated_extensions() -> HashMap<String, String> {
    extensions_from_pairs(&[
        (EXT_GIT, "0.5.0"),
        (EXT_DOCKER, "1.0.0"),
        (EXT_KUBERNETES, "1.0.0"),
    ])
}

/// Create extensions with one outdated extension (git too old)
pub fn one_outdated_extension() -> HashMap<String, String> {
    extensions_from_pairs(&[
        (EXT_GIT, "0.9.0"), // Too old
        (EXT_DOCKER, EXT_VERSION_2_0_0),
    ])
}

/// Create extensions for version 3.1.0 compatibility testing
pub fn v3_1_extensions() -> HashMap<String, String> {
    extensions_from_pairs(&[(EXT_GIT, EXT_VERSION_1_1_5)])
}

/// Create extensions for complex version requirements
pub fn complex_extensions() -> HashMap<String, String> {
    extensions_from_pairs(&[("ext1", "1.5.0"), ("ext2", "1.5.8"), ("ext3", "2.1.5")])
}

/// Create extensions with an unknown extension
pub fn unknown_extension() -> HashMap<String, String> {
    extensions_from_pairs(&[("unknown-extension", "1.0.0")])
}

/// Create extensions with an invalid version string
pub fn invalid_version_extension() -> HashMap<String, String> {
    extensions_from_pairs(&[(EXT_GIT, "invalid-version")])
}

/// Create an empty extension HashMap
pub fn empty_extensions() -> HashMap<String, String> {
    HashMap::new()
}

/// Create a HashMap of extensions from an array of (name, version) pairs
///
/// # Example
/// ```ignore
/// let exts = extensions_from_pairs(&[
///     ("git", "1.0.0"),
///     ("docker", "2.0.0"),
/// ]);
/// ```
pub fn extensions_from_pairs(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(name, version)| (name.to_string(), version.to_string()))
        .collect()
}

/// Create a single extension HashMap
pub fn single_extension(name: &str, version: &str) -> HashMap<String, String> {
    extensions_from_pairs(&[(name, version)])
}
