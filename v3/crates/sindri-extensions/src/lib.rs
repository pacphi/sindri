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
//! - Configure processing (templates and environment variables)
//! - Trait-based extension sources (bundled, downloaded, local-dev)

pub mod bom;
pub mod configure;
pub mod dependency;
pub mod distribution;
pub mod executor;
pub mod manifest;
pub mod profile;
pub mod registry;
pub mod source;
pub mod support_files;
pub mod types;
pub mod validation;
pub mod validator;

pub use bom::BomGenerator;
pub use configure::ConfigureProcessor;
pub use dependency::DependencyResolver;
pub use distribution::{
    CompatibilityMatrix, ExtensionDistributor, ExtensionManifest, ManifestEntry,
};
pub use executor::ExtensionExecutor;
pub use manifest::ManifestManager;
pub use profile::{ProfileInstallResult, ProfileInstaller, ProfileStatus};
pub use registry::ExtensionRegistry;
pub use source::{
    BundledSource, DownloadedSource, ExtensionSourceResolver, LocalDevSource, SourceType,
};
pub use support_files::{SupportFileManager, SupportFileMetadata, SupportFileSource};
pub use validation::{ValidationConfig, DEFAULT_VALIDATION_PATHS, VALIDATION_EXTRA_PATHS_ENV};
pub use validator::ExtensionValidator;
