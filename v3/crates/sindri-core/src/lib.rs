//! # sindri-core
//!
//! Core library for the Sindri CLI providing:
//! - Configuration file parsing (sindri.yaml)
//! - JSON Schema validation
//! - Type definitions for extensions, providers, and deployments
//! - Retry execution engine with policy-based configuration

pub mod config;
pub mod error;
pub mod retry;
pub mod schema;
pub mod templates;
pub mod types;
pub mod utils;

pub use config::{ImageVersionResolver, SindriConfig};
pub use error::{Error, Result};
pub use schema::SchemaValidator;
pub use utils::get_home_dir;
