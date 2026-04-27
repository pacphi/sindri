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
pub mod lint;
pub mod local;
pub mod oci_ref;
pub mod signing;

pub use cache::{BlobKind, RegistryCache};
pub use client::RegistryClient;
pub use error::RegistryError;
pub use index::RegistryIndex;
pub use local::LocalRegistry;
pub use oci_ref::{OciRef, OciReference};
pub use signing::{CosignVerifier, TrustedKey};
