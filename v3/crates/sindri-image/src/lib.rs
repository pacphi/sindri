//! Container image management for Sindri CLI
//!
//! This crate provides functionality for:
//! - Querying OCI-compatible container registries (GHCR, Docker Hub, etc.)
//! - Resolving image versions using semantic versioning constraints
//! - Verifying image signatures and provenance using Cosign
//! - Fetching and parsing SBOMs (Software Bill of Materials)
//!
//! # Example
//!
//! ```no_run
//! use sindri_image::{RegistryClient, VersionResolver, ImageReference};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Create a registry client
//!     let client = RegistryClient::new("ghcr.io")
//!         .with_token(std::env::var("GITHUB_TOKEN")?);
//!
//!     // Resolve version
//!     let resolver = VersionResolver::new(client);
//!     let tag = resolver.resolve_version("pacphi/sindri", "^3.0.0", false).await?;
//!
//!     println!("Resolved to: {}", tag);
//!
//!     Ok(())
//! }
//! ```

pub mod registry;
pub mod resolver;
pub mod types;
pub mod verify;

// Re-export main types for convenience
pub use registry::RegistryClient;
pub use resolver::VersionResolver;
pub use types::{
    ImageInfo, ImageManifest, ImageReference, Platform, ProvenanceVerification, PullPolicy,
    ResolutionStrategy, Sbom, SbomPackage, SignatureInfo, SignatureVerification,
};
pub use verify::ImageVerifier;

/// Version of the sindri-image crate
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
        println!("sindri-image version: {}", VERSION);
    }
}
