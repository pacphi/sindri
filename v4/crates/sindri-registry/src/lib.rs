#![allow(dead_code)]

//! Sindri v4 registry crate.
//!
//! Wave 3A.1 introduces the OCI + cosign foundation (ADR-003, ADR-014):
//!
//! - [`oci_ref`]: pure value parser for OCI references.
//! - [`cache`]: content-addressed blob cache.
//! - [`signing`]: cosign trust-key loader (verification deferred to 3A.2).
//!
//! Live OCI fetch (`oci-client` API calls) and cosign signature verification
//! land in Wave 3A.2; see `v4/docs/review/2026-04-27-implementation-audit-delta.md`.

pub mod cache;
pub mod client;
pub mod error;
pub mod index;
pub mod keyless;
pub mod lint;
pub mod oci_ref;
pub mod signing;
pub mod source;
pub mod tarball;
pub mod trust_scope;

pub use cache::{BlobKind, RegistryCache};
pub use client::RegistryClient;
pub use error::RegistryError;
pub use index::RegistryIndex;
pub use keyless::{
    EnvelopeKind, KeylessIdentity, KeylessTrustRoot, KeylessVerifier, SignatureEnvelope,
    VerificationMode,
};
pub use oci_ref::{OciRef, OciReference};
pub use signing::{CosignVerifier, TrustedKey};
pub use source::{
    ComponentBlob, ComponentId as SourceComponentId, ComponentName, GitSource, GitSourceRuntime,
    LocalOciSource, LocalOciSourceConfig, LocalPathSource, OciSource, OciSourceConfig,
    RegistrySource, Source, SourceContext, SourceDescriptor, SourceError,
};
pub use trust_scope::{glob_match, select_override};
