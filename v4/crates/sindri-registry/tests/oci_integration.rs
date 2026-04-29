//! Live OCI integration tests (ADR-003 / ADR-014).
//!
//! These tests actually hit a public OCI registry, so they are gated behind
//! the `live-oci-tests` feature **and** the `#[ignore]` attribute. CI does
//! not run them by default.
//!
//! Run locally with:
//!
//! ```bash
//! cargo test -p sindri-registry --features live-oci-tests \
//!     --test oci_integration -- --ignored
//! ```

#![cfg(feature = "live-oci-tests")]

use oci_client::secrets::RegistryAuth;
use oci_client::Client as OciClient;
use sindri_registry::OciRef;

/// Smoke test: pull a manifest from a small public OCI image and assert we
/// got a non-empty digest back. Validates the end-to-end oci-client wiring
/// against a real registry without depending on a sindri-published artifact
/// (which doesn't exist yet).
#[tokio::test]
#[ignore = "requires network access; run with --features live-oci-tests --ignored"]
async fn pulls_manifest_from_public_registry() {
    // `cgr.dev/chainguard/static` is a tiny public image; manifests for its
    // tags should always be reachable.
    let oci_ref = OciRef::parse("cgr.dev/chainguard/static:latest").unwrap();
    let reference = oci_client::Reference::with_tag(
        oci_ref.registry.clone(),
        oci_ref.repository.clone(),
        match &oci_ref.reference {
            sindri_registry::OciReference::Tag(t) => t.clone(),
            sindri_registry::OciReference::Digest(d) => d.clone(),
        },
    );
    let client = OciClient::default();
    let (_manifest, digest) = client
        .pull_manifest(&reference, &RegistryAuth::Anonymous)
        .await
        .expect("manifest pull should succeed");
    assert!(!digest.is_empty());
    assert!(
        digest.starts_with("sha256:"),
        "expected sha256-prefixed digest, got {}",
        digest
    );
}
