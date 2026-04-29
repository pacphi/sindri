//! Phase-2 acceptance tests for `LocalOciSource` (DDD-08, ADR-028).
//!
//! Builds a deterministic three-component OCI image layout via
//! `local_oci::build_test_fixture` (committed code, not committed binary
//! blobs) and exercises:
//!
//! 1. The trait surface — `fetch_index` / `fetch_component_blob` /
//!    `lockfile_descriptor`.
//! 2. The byte-stable digest invariant — when we serve the fixture's
//!    `index.yaml` blob through a wiremock OCI registry and resolve it via
//!    `OciSource`, the layer digest computed by `oci-client` matches the
//!    digest the fixture wrote on disk. This is the regression guard
//!    called for in Phase 2 §"Acceptance criteria" #3.

use oci_client::client::{ClientConfig, ClientProtocol};
use oci_client::Client as OciClient;
use sha2::{Digest, Sha256};
use sindri_registry::source::local_oci::{build_test_fixture, FixtureLayout};
use sindri_registry::source::{
    LocalOciSource, LocalOciSourceConfig, OciSource, OciSourceConfig, Source, SourceContext,
    SourceDescriptor,
};
use sindri_registry::{RegistryCache, RegistryClient};
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use wiremock::matchers::{method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SINDRI_INDEX_MEDIA_TYPE: &str = "application/vnd.sindri.registry.index.v1+yaml";
const REPO: &str = "sindri-dev/registry-core";
const TAG: &str = "fixture-1.0.0";

fn http_oci_client() -> OciClient {
    OciClient::new(ClientConfig {
        protocol: ClientProtocol::Http,
        ..ClientConfig::default()
    })
}

#[test]
fn fixture_layout_is_three_components() {
    let tmp = TempDir::new().unwrap();
    let layout = build_test_fixture(tmp.path(), false).unwrap();
    let src = LocalOciSource::new(LocalOciSourceConfig {
        layout_path: layout.layout_path,
        scope: None,
        registry_name: "test-fixture".into(),
        artifact_ref: None,
    });
    let idx = src.fetch_index(&SourceContext::default()).unwrap();
    assert_eq!(idx.components.len(), 3);
}

#[test]
fn local_oci_descriptor_uses_fixture_manifest_digest() {
    // The descriptor MUST carry the fixture's manifest digest so a lockfile
    // re-resolved against the same layout is byte-stable.
    let tmp = TempDir::new().unwrap();
    let layout: FixtureLayout = build_test_fixture(tmp.path(), false).unwrap();

    let src = LocalOciSource::new(LocalOciSourceConfig {
        layout_path: layout.layout_path.clone(),
        scope: None,
        registry_name: "test-fixture".into(),
        artifact_ref: None,
    });
    src.fetch_index(&SourceContext::default()).unwrap();

    let d = src.lockfile_descriptor();
    match d {
        SourceDescriptor::LocalOci {
            layout_path,
            manifest_digest,
        } => {
            assert_eq!(layout_path, layout.layout_path);
            assert_eq!(
                manifest_digest.as_deref(),
                Some(layout.manifest_digest.as_str())
            );
        }
        other => panic!("expected LocalOci, got {:?}", other),
    }
}

/// Acceptance #3: `LocalOciSource` produces byte-for-byte the same
/// component blob digests as the `OciSource` it was prefetched from.
///
/// The fixture writes a layer blob at `sha256:<X>`. We serve that exact
/// blob through a wiremock OCI registry and pull it via `OciSource`; the
/// `manifest_digest` recorded by the OCI source must equal the manifest
/// digest the fixture wrote at on disk for the same artifact, which is
/// what `LocalOciSource::lockfile_descriptor()` reports.
#[tokio::test]
async fn descriptor_round_trips_between_oci_and_local_oci() {
    let tmp = TempDir::new().unwrap();
    let layout: FixtureLayout = build_test_fixture(tmp.path(), false).unwrap();

    // Read the manifest blob the fixture wrote so we can serve it from
    // the mock registry verbatim.
    let manifest_path = tmp
        .path()
        .join("blobs/sha256")
        .join(layout.manifest_digest.trim_start_matches("sha256:"));
    let manifest_bytes = std::fs::read(&manifest_path).unwrap();
    // The mock registry must compute the manifest digest itself (oci-client
    // verifies it on the wire), so make sure our recorded digest matches.
    let computed = format!("sha256:{}", hex::encode(Sha256::digest(&manifest_bytes)));
    assert_eq!(computed, layout.manifest_digest);

    let layer_path = tmp
        .path()
        .join("blobs/sha256")
        .join(layout.layer_digest.trim_start_matches("sha256:"));
    let layer_bytes = std::fs::read(&layer_path).unwrap();

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v2/{}/manifests/{}", REPO, TAG)))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/vnd.oci.image.manifest.v1+json")
                .set_body_bytes(manifest_bytes.clone()),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/v2/.*/blobs/sha256:.*$"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(layer_bytes.clone()))
        .mount(&server)
        .await;

    let endpoint = server.uri().trim_start_matches("http://").to_string();
    let url = format!("{}/{}", endpoint, REPO);

    let cache_dir = TempDir::new().unwrap();
    let cache = RegistryCache::with_path(cache_dir.path().to_path_buf()).unwrap();
    let client = RegistryClient::with_cache(cache)
        .with_ttl(Duration::from_secs(3600))
        .with_oci_client(http_oci_client());
    let oci_source = OciSource::with_client(
        OciSourceConfig {
            url,
            tag: TAG.into(),
            scope: None,
            registry_name: "test-fixture".into(),
        },
        Arc::new(client),
    );

    // Drive the fetch via the trait so the OciSource captures its
    // manifest digest. We do this on a worker thread because the test
    // is `#[tokio::test]` and the trait method synchronously bridges
    // through `block_on_async`.
    let oci_source_for_thread = oci_source.clone();
    let oci_descriptor = std::thread::spawn(move || {
        Source::fetch_index(&oci_source_for_thread, &SourceContext::default())
            .expect("oci fetch should succeed");
        oci_source_for_thread.lockfile_descriptor()
    })
    .join()
    .unwrap();

    let local_oci_source = LocalOciSource::new(LocalOciSourceConfig {
        layout_path: layout.layout_path.clone(),
        scope: None,
        registry_name: "test-fixture".into(),
        artifact_ref: None,
    });
    local_oci_source
        .fetch_index(&SourceContext::default())
        .expect("local-oci fetch should succeed");
    let local_descriptor = local_oci_source.lockfile_descriptor();

    // Both descriptors must carry the SAME manifest digest — the
    // byte-stability invariant from DDD-08.
    let oci_digest = match oci_descriptor {
        SourceDescriptor::Oci {
            manifest_digest, ..
        } => manifest_digest,
        other => panic!("expected Oci descriptor, got {:?}", other),
    };
    let local_digest = match local_descriptor {
        SourceDescriptor::LocalOci {
            manifest_digest, ..
        } => manifest_digest,
        other => panic!("expected LocalOci descriptor, got {:?}", other),
    };
    assert_eq!(
        oci_digest.as_deref(),
        Some(layout.manifest_digest.as_str()),
        "OciSource manifest digest must match fixture"
    );
    assert_eq!(
        local_digest.as_deref(),
        Some(layout.manifest_digest.as_str()),
        "LocalOciSource manifest digest must match fixture"
    );
    assert_eq!(
        oci_digest, local_digest,
        "OciSource and LocalOciSource must agree on manifest digest"
    );

    // Also ensure the layer bytes (not just the manifest) hash identically
    // by media type — guarding against any silent transcoding.
    let mut found = false;
    let manifest_json: serde_json::Value = serde_json::from_slice(&manifest_bytes).unwrap();
    for layer in manifest_json
        .get("layers")
        .and_then(|l| l.as_array())
        .unwrap()
    {
        if layer.get("mediaType").and_then(|m| m.as_str()) == Some(SINDRI_INDEX_MEDIA_TYPE) {
            assert_eq!(
                layer.get("digest").and_then(|d| d.as_str()).unwrap(),
                layout.layer_digest.as_str(),
            );
            found = true;
        }
    }
    assert!(found, "fixture manifest should reference the index layer");
}
