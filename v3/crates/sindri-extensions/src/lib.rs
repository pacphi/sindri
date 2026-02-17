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
pub mod events;
pub mod executor;
pub mod ledger;
pub mod log_files;
pub mod profile;
pub mod registry;
pub mod source;
pub mod support_files;
pub mod types;
pub mod validation;
pub mod validator;
pub mod verifier;

pub use bom::BomGenerator;
pub use configure::ConfigureProcessor;
pub use dependency::DependencyResolver;
pub use distribution::{
    verify_content_integrity, CompatibilityMatrix, ExtensionDistributor, ExtensionSourceConfig,
};
pub use events::{EventEnvelope, ExtensionEvent};
pub use executor::{ExtensionExecutor, InstallOutput};
pub use ledger::{
    EventFilter, ExtensionStatus, LedgerStats, StatusLedger, DEFAULT_FOLLOW_POLL_SECS,
    DEFAULT_LOG_TAIL_LINES,
};
pub use log_files::ExtensionLogWriter;
pub use profile::{ProfileInstallResult, ProfileInstaller, ProfileStatus};
pub use registry::ExtensionRegistry;
pub use source::{
    BundledSource, DownloadedSource, ExtensionSourceResolver, LocalDevSource, SourceType,
};
pub use support_files::{SupportFileManager, SupportFileMetadata, SupportFileSource};
pub use validation::{ValidationConfig, DEFAULT_VALIDATION_PATHS, VALIDATION_EXTRA_PATHS_ENV};
pub use validator::ExtensionValidator;
pub use verifier::{find_extension_yaml, verify_extension_installed};
