//! Common test infrastructure for sindri-update tests
//!
//! This module provides shared constants, builders, and helper functions
//! to reduce duplication across test files.
//!
//! # Usage
//!
//! In your test file, add:
//! ```ignore
//! mod common;
//! use common::*;
//! ```
//!
//! # Modules
//!
//! - `constants`: Version strings, platform identifiers, test data
//! - `builders`: Fluent builders for Release and ReleaseAsset
//! - `extensions`: HashMap factory functions for extension sets
//! - `mock_server`: Wiremock setup helpers for download tests
//! - `assertions`: Semantic assertion functions for compatibility results
//! - `fixtures`: YAML fixture loading helpers
//! - `updater_helpers`: Helpers for updater/binary testing

// Allow unused code in test infrastructure - these are scaffolded for future tests
#![allow(dead_code)]
#![allow(unused_imports)]

pub mod assertions;
pub mod builders;
pub mod constants;
pub mod extensions;
pub mod fixtures;
pub mod mock_server;
pub mod updater_helpers;

// Re-export all public items for convenience
pub use assertions::*;
pub use builders::*;
pub use constants::*;
pub use extensions::*;
pub use fixtures::*;
pub use mock_server::*;
pub use updater_helpers::*;
