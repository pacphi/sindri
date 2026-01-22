//! Extension management for Sindri
//!
//! This crate handles:
//! - Extension registry loading
//! - Dependency resolution
//! - Extension installation/removal
//! - Validation
//! - Bill of Materials (BOM) generation
//! - GitHub-based extension distribution
//! - Profile-based batch installation

pub mod bom;
pub mod dependency;
pub mod distribution;
pub mod executor;
pub mod manifest;
pub mod profile;
pub mod registry;
pub mod types;
pub mod validator;

pub use bom::BomGenerator;
pub use dependency::DependencyResolver;
pub use distribution::{
    CompatibilityMatrix, ExtensionDistributor, ExtensionManifest, ManifestEntry,
};
pub use executor::ExtensionExecutor;
pub use manifest::ManifestManager;
pub use profile::{ProfileInstallResult, ProfileInstaller, ProfileStatus};
pub use registry::ExtensionRegistry;
pub use validator::ExtensionValidator;
