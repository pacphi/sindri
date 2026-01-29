//! Common test utilities for sindri-packer
//!
//! Provides shared test infrastructure for Packer testing including:
//! - Mock cloud API implementations
//! - Template rendering utilities
//! - Build lifecycle test helpers

pub mod assertions;
pub mod mock_cloud;

pub use assertions::*;
pub use mock_cloud::*;
