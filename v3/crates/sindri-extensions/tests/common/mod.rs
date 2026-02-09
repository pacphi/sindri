//! Common test utilities for sindri-extensions
//!
//! This module provides shared test infrastructure including:
//! - Constants and configuration
//! - Extension builders for creating test fixtures
//! - Mock implementations for testing without side effects
//! - Assertion helpers for lifecycle testing

#![allow(dead_code)]
#![allow(unused_imports)]

pub mod assertions;
pub mod bom_builders;
pub mod builders;
pub mod constants;
pub mod fixtures;
pub mod mocks;
pub mod test_extensions;

pub use assertions::*;
pub use bom_builders::*;
pub use builders::*;
pub use constants::*;
pub use fixtures::*;
pub use mocks::*;
pub use test_extensions::*;
