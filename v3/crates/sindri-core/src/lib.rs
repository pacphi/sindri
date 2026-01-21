//! # sindri-core
//!
//! Core library for the Sindri CLI providing:
//! - Configuration file parsing (sindri.yaml)
//! - JSON Schema validation
//! - Type definitions for extensions, providers, and deployments

pub mod config;
pub mod error;
pub mod schema;
pub mod types;

pub use config::SindriConfig;
pub use error::{Error, Result};
pub use schema::SchemaValidator;
