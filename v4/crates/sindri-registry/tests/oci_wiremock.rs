//! Wave 5A — D7: in-process wiremock tests for the OCI fetch path.
//!
//! These tests replace the previous `TODO(wave-3a.3)` markers in
//! `tests/oci_integration.rs`. They run on every `cargo test` invocation
//! (no feature gate, no `#[ignore]`) because they spin up an in-process
//! HTTP mock with [`wiremock`] rather than hitting a live registry.
//!
//! The live `--features live-oci-tests --ignored` integration tests are
//! kept around as smoke tests for the real-world handshake.
//!
//! ## Coverage matrix
//!
//! | Scenario                              | Test                                  |
//! |---------------------------------------|---------------------------------------|
//! | Bearer-token negotiation (401 → /token → retry) | `bearer_token_negotiation`  |
//! | Manifest + layer fetch + digest match | `fetch_index_succeeds_with_valid_layer` |
//! | Layer digest mismatch                 | `digest_mismatch_aborts_fetch`        |
//! | 404 manifest                          | `manifest_404_maps_to_oci_fetch_error`|
//! | 500 manifest                          | `manifest_500_maps_to_oci_fetch_error`|

use oci_client::client::{ClientConfig, ClientProtocol};
use oci_client::Client as OciClient;
use sha2::{Digest, Sha256};
use sindri_registry::source::{
    OciSource, OciSourceConfig, Source, SourceContext, SourceDescriptor,
};
use sindri_registry::{RegistryCache, RegistryClient};
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use wiremock::matchers::{header, method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

const REPO: &str = "sindri-dev/registry-core";
const TAG: &str = "1.0.0";
const SINDRI_INDEX_MEDIA_TYPE: &str = "application/vnd.sindri.registry.index.v1+yaml";

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

fn http_oci_client() -> OciClient {
    OciClient::new(ClientConfig {
        protocol: ClientProtocol::Http,
        ..ClientConfig::default()
    })
}

fn temp_client(oci: OciClient) -> (TempDir, RegistryClient) {
    let tmp = TempDir::new().unwrap();
    let cache = RegistryCache::with_path(tmp.path().to_path_buf()).unwrap();
    let client = RegistryClient::with_cache(cache)
        .with_ttl(Duration::from_secs(3600))
        .with_oci_client(oci);
    (tmp, client)
}

/// Build the OCI image manifest JSON for an `index.yaml` layer of the given
/// content. The manifest descriptor digest is *not* the manifest's own
/// digest — that's computed by oci-client from the response body — but the
/// layer descriptor digest must match the layer body bytes exactly.
fn make_index_manifest(layer_bytes: &[u8]) -> String {
    let layer_digest = format!("sha256:{}", sha256_hex(layer_bytes));
    serde_json::json!({
        "schemaVersion": 2,
        "mediaType": "application/vnd.oci.image.manifest.v1+json",
        "config": {
            "mediaType": "application/vnd.oci.image.config.v1+json",
            "digest": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
            "size": 0
        },
        "layers": [{
            "mediaType": SINDRI_INDEX_MEDIA_TYPE,
            "digest": layer_digest,
            "size": layer_bytes.len()
        }]
    })
    .to_string()
}

fn registry_url_for(server: &MockServer) -> (String, String) {
    let endpoint = server.uri().trim_start_matches("http://").to_string();
    let registry_url = format!("{}/{}:{}", endpoint, REPO, TAG);
    (endpoint, registry_url)
}

#[tokio::test]
async fn fetch_index_succeeds_with_valid_layer() {
    let server = MockServer::start().await;
    let layer = b"version: 1\nregistry: mock\ncomponents: []\n";
    let manifest = make_index_manifest(layer);
    let layer_digest = format!("sha256:{}", sha256_hex(layer));

    Mock::given(method("GET"))
        .and(path(format!("/v2/{}/manifests/{}", REPO, TAG)))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/vnd.oci.image.manifest.v1+json")
                .set_body_string(manifest.clone()),
        )
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path(format!("/v2/{}/blobs/{}", REPO, layer_digest)))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(layer.to_vec()))
        .mount(&server)
        .await;

    let (endpoint, registry_url) = registry_url_for(&server);
    let _ = endpoint;
    let oci = http_oci_client();
    let (_t, client) = temp_client(oci);
    let (index, digest) = client
        .fetch_index("mock", &registry_url)
        .await
        .expect("fetch_index should succeed");
    assert!(digest.is_some(), "fetch should report a manifest digest");
    assert_eq!(index.components.len(), 0);
}

#[tokio::test]
async fn bearer_token_negotiation() {
    // First request returns 401 with WWW-Authenticate: Bearer realm=…
    // pointing at /token. The oci-client library is expected to call /token,
    // collect the bearer, then retry the manifest GET with Authorization.
    let server = MockServer::start().await;
    let layer = b"version: 1\nregistry: bearer-mock\ncomponents: []\n";
    let manifest = make_index_manifest(layer);
    let layer_digest = format!("sha256:{}", sha256_hex(layer));
    let realm = format!("{}/token", server.uri());

    // oci-client's auth flow starts with `GET /v2/` to discover the bearer
    // challenge — it does NOT key off a 401 on the manifest URL itself.
    // See `oci-client::Client::_auth` (the `version request` step).
    Mock::given(method("GET"))
        .and(path("/v2/"))
        .respond_with(
            ResponseTemplate::new(401).insert_header(
                "WWW-Authenticate",
                format!(
                    "Bearer realm=\"{}\",service=\"mock\",scope=\"repository:{}:pull\"",
                    realm, REPO
                )
                .as_str(),
            ),
        )
        .mount(&server)
        .await;

    // Token endpoint.
    Mock::given(method("GET"))
        .and(path("/token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"{"token":"deadbeef","access_token":"deadbeef"}"#),
        )
        .mount(&server)
        .await;

    // Authenticated retry of the manifest fetch — oci-client sends
    // Authorization: Bearer deadbeef.
    Mock::given(method("GET"))
        .and(path(format!("/v2/{}/manifests/{}", REPO, TAG)))
        .and(header("Authorization", "Bearer deadbeef"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/vnd.oci.image.manifest.v1+json")
                .set_body_string(manifest.clone()),
        )
        .mount(&server)
        .await;

    // Layer blob (also needs the bearer).
    Mock::given(method("GET"))
        .and(path(format!("/v2/{}/blobs/{}", REPO, layer_digest)))
        .and(header("Authorization", "Bearer deadbeef"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(layer.to_vec()))
        .mount(&server)
        .await;

    let (endpoint, registry_url) = registry_url_for(&server);
    let _ = endpoint;
    let oci = http_oci_client();
    let (_t, client) = temp_client(oci);
    let res = client.fetch_index("bearer-mock", &registry_url).await;
    assert!(
        res.is_ok(),
        "expected bearer-token flow to succeed, got: {:?}",
        res.err()
    );
}

#[tokio::test]
async fn digest_mismatch_aborts_fetch() {
    // Manifest claims a layer digest that does NOT match the bytes the
    // registry actually serves. oci-client should reject the blob during
    // streaming digest verification.
    let server = MockServer::start().await;
    let real_layer = b"version: 1\ncomponents: []\n";
    let lying_digest = format!("sha256:{}", "f".repeat(64));
    let manifest = serde_json::json!({
        "schemaVersion": 2,
        "mediaType": "application/vnd.oci.image.manifest.v1+json",
        "config": {
            "mediaType": "application/vnd.oci.image.config.v1+json",
            "digest": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
            "size": 0
        },
        "layers": [{
            "mediaType": SINDRI_INDEX_MEDIA_TYPE,
            "digest": lying_digest,
            "size": real_layer.len()
        }]
    })
    .to_string();

    Mock::given(method("GET"))
        .and(path(format!("/v2/{}/manifests/{}", REPO, TAG)))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/vnd.oci.image.manifest.v1+json")
                .set_body_string(manifest),
        )
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path_regex(r"^/v2/.*/blobs/sha256:.*$"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(real_layer.to_vec()))
        .mount(&server)
        .await;

    let (endpoint, registry_url) = registry_url_for(&server);
    let _ = endpoint;
    let oci = http_oci_client();
    let (_t, client) = temp_client(oci);
    let err = client
        .fetch_index("mock", &registry_url)
        .await
        .expect_err("digest mismatch must fail closed");
    let msg = format!("{}", err);
    assert!(
        msg.to_ascii_lowercase().contains("digest")
            || msg.to_ascii_lowercase().contains("integrity")
            || msg.to_ascii_lowercase().contains("blob"),
        "expected a digest/integrity error, got: {}",
        msg
    );
}

#[tokio::test]
async fn fetch_index_succeeds_with_targz_layer() {
    // D6 integration: a registry serving its index.yaml inside a tar+gzip
    // layer must be unwrapped, path-traversal-checked, and digest-verified.
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    let server = MockServer::start().await;
    let index_yaml = b"version: 1\nregistry: targz\ncomponents: []\n";

    let mut tar_buf: Vec<u8> = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut tar_buf);
        let mut header = tar::Header::new_gnu();
        header.set_size(index_yaml.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, "index.yaml", &index_yaml[..])
            .unwrap();
        builder.finish().unwrap();
    }
    let mut gz = GzEncoder::new(Vec::new(), Compression::default());
    gz.write_all(&tar_buf).unwrap();
    let layer = gz.finish().unwrap();
    let layer_digest = format!("sha256:{}", sha256_hex(&layer));

    let manifest = serde_json::json!({
        "schemaVersion": 2,
        "mediaType": "application/vnd.oci.image.manifest.v1+json",
        "config": {
            "mediaType": "application/vnd.oci.image.config.v1+json",
            "digest": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
            "size": 0
        },
        "layers": [{
            "mediaType": "application/vnd.oci.image.layer.v1.tar+gzip",
            "digest": layer_digest,
            "size": layer.len()
        }]
    })
    .to_string();

    Mock::given(method("GET"))
        .and(path(format!("/v2/{}/manifests/{}", REPO, TAG)))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/vnd.oci.image.manifest.v1+json")
                .set_body_string(manifest),
        )
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path_regex(r"^/v2/.*/blobs/sha256:.*$"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(layer.clone()))
        .mount(&server)
        .await;

    let (endpoint, registry_url) = registry_url_for(&server);
    let _ = endpoint;
    let oci = http_oci_client();
    let (_t, client) = temp_client(oci);
    let (index, _digest) = client
        .fetch_index("targz-mock", &registry_url)
        .await
        .expect("tar+gzip layer extraction should succeed");
    assert_eq!(index.components.len(), 0);
}

#[tokio::test]
async fn manifest_404_maps_to_oci_fetch_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/v2/.*/manifests/.*$"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let (endpoint, registry_url) = registry_url_for(&server);
    let _ = endpoint;
    let oci = http_oci_client();
    let (_t, client) = temp_client(oci);
    let err = client
        .fetch_index("mock", &registry_url)
        .await
        .expect_err("404 must surface as an error");
    let msg = format!("{}", err);
    assert!(
        msg.to_ascii_lowercase().contains("404")
            || msg.to_ascii_lowercase().contains("not found")
            || msg.to_ascii_lowercase().contains("oci fetch"),
        "expected a not-found-style error, got: {}",
        msg
    );
}

#[tokio::test]
async fn fetch_component_layer_digest_returns_descriptor_digest() {
    // Wave 5F — D5: the resolver pre-fetches per-component layer digests
    // and writes them into the lockfile. This test exercises the manifest-
    // only fetch path (no blob pull).
    let server = MockServer::start().await;
    let layer = b"opaque-component-payload";
    let layer_digest = format!("sha256:{}", sha256_hex(layer));
    let manifest = serde_json::json!({
        "schemaVersion": 2,
        "mediaType": "application/vnd.oci.image.manifest.v1+json",
        "config": {
            "mediaType": "application/vnd.oci.image.config.v1+json",
            "digest": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
            "size": 0
        },
        "layers": [{
            "mediaType": OCI_TAR_GZIP_MEDIA_TYPE,
            "digest": layer_digest,
            "size": layer.len()
        }]
    })
    .to_string();

    Mock::given(method("GET"))
        .and(path(format!("/v2/{}/manifests/{}", REPO, TAG)))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/vnd.oci.image.manifest.v1+json")
                .set_body_string(manifest),
        )
        .mount(&server)
        .await;

    let (_endpoint, registry_url) = registry_url_for(&server);
    let oci = http_oci_client();
    let (_t, client) = temp_client(oci);
    let returned = client
        .fetch_component_layer_digest(&registry_url)
        .await
        .expect("manifest-only fetch should succeed");
    assert_eq!(returned, layer_digest);
}

#[tokio::test]
async fn fetch_component_layer_digest_404_surfaces_oci_fetch_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/v2/.*/manifests/.*$"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    let (_endpoint, registry_url) = registry_url_for(&server);
    let oci = http_oci_client();
    let (_t, client) = temp_client(oci);
    let err = client
        .fetch_component_layer_digest(&registry_url)
        .await
        .expect_err("404 must surface");
    let msg = format!("{}", err).to_ascii_lowercase();
    assert!(
        msg.contains("404") || msg.contains("not found") || msg.contains("oci fetch"),
        "expected fetch error, got: {}",
        msg
    );
}

const OCI_TAR_GZIP_MEDIA_TYPE: &str = "application/vnd.oci.image.layer.v1.tar+gzip";

#[tokio::test]
async fn manifest_500_maps_to_oci_fetch_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/v2/.*/manifests/.*$"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let (endpoint, registry_url) = registry_url_for(&server);
    let _ = endpoint;
    let oci = http_oci_client();
    let (_t, client) = temp_client(oci);
    let err = client
        .fetch_index("mock", &registry_url)
        .await
        .expect_err("5xx must surface as an error");
    let msg = format!("{}", err);
    assert!(
        msg.to_ascii_lowercase().contains("500")
            || msg.to_ascii_lowercase().contains("server")
            || msg.to_ascii_lowercase().contains("oci fetch"),
        "expected a 5xx-style error, got: {}",
        msg
    );
}

// ----------------------------------------------------------------------
// Phase 2 — DDD-08 / ADR-028 trait surface tests
//
// These mirror two of the direct-client tests above, but drive the work
// through the [`Source`] trait against an [`OciSource`] backed by the same
// wiremock mock registry. The direct-client coverage is preserved
// (`fetch_index_succeeds_with_valid_layer` / `manifest_404_…` stay) so the
// underlying client is still tested without going through the trait.
// ----------------------------------------------------------------------

#[tokio::test]
async fn oci_source_trait_fetch_index_succeeds() {
    let server = MockServer::start().await;
    let layer = b"version: 1\nregistry: oci-source-trait\ncomponents: []\n";
    let manifest = make_index_manifest(layer);
    let layer_digest = format!("sha256:{}", sha256_hex(layer));

    Mock::given(method("GET"))
        .and(path(format!("/v2/{}/manifests/{}", REPO, TAG)))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/vnd.oci.image.manifest.v1+json")
                .set_body_string(manifest.clone()),
        )
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path(format!("/v2/{}/blobs/{}", REPO, layer_digest)))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(layer.to_vec()))
        .mount(&server)
        .await;

    let endpoint = server.uri().trim_start_matches("http://").to_string();
    let url = format!("{}/{}", endpoint, REPO);

    let tmp = TempDir::new().unwrap();
    let cache = RegistryCache::with_path(tmp.path().to_path_buf()).unwrap();
    let client = RegistryClient::with_cache(cache)
        .with_ttl(Duration::from_secs(3600))
        .with_oci_client(http_oci_client());
    let source = OciSource::with_client(
        OciSourceConfig {
            url,
            tag: TAG.into(),
            scope: None,
            registry_name: "oci-source-trait".into(),
        },
        Arc::new(client),
    );

    // Drive through the trait (NOT the client) — this is the migration
    // that the Phase-2 plan calls for.
    let index = Source::fetch_index(&source, &SourceContext::default())
        .expect("trait-driven fetch_index should succeed");
    assert_eq!(index.components.len(), 0);

    // Lockfile descriptor records the manifest digest captured during
    // the fetch, so the lockfile is byte-stable.
    match source.lockfile_descriptor() {
        SourceDescriptor::Oci {
            url: _,
            tag,
            manifest_digest,
        } => {
            assert_eq!(tag, TAG);
            assert!(manifest_digest.is_some(), "expected manifest digest");
            assert!(manifest_digest
                .as_ref()
                .map(|d| d.starts_with("sha256:"))
                .unwrap_or(false));
        }
        other => panic!("expected Oci descriptor, got {:?}", other),
    }

    // A successful fetch flips the verified bit, so a `sindri/core` /
    // explicitly-trusted source can claim strict-OCI eligibility.
    assert!(
        source.is_verified(),
        "successful fetch should mark the source verified"
    );
}

#[tokio::test]
async fn oci_source_trait_404_propagates_as_source_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/v2/.*/manifests/.*$"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let endpoint = server.uri().trim_start_matches("http://").to_string();
    let url = format!("{}/{}", endpoint, REPO);

    let tmp = TempDir::new().unwrap();
    let cache = RegistryCache::with_path(tmp.path().to_path_buf()).unwrap();
    let client = RegistryClient::with_cache(cache)
        .with_ttl(Duration::from_secs(3600))
        .with_oci_client(http_oci_client());
    let source = OciSource::with_client(
        OciSourceConfig {
            url,
            tag: TAG.into(),
            scope: None,
            registry_name: "oci-source-trait-404".into(),
        },
        Arc::new(client),
    );

    let err = Source::fetch_index(&source, &SourceContext::default())
        .expect_err("404 must surface through the trait");
    let msg = format!("{}", err).to_ascii_lowercase();
    assert!(
        msg.contains("404") || msg.contains("not found") || msg.contains("oci fetch"),
        "expected propagated fetch error, got: {}",
        err
    );
    // Failed fetch must NOT flip the verified bit.
    assert!(
        !source.is_verified(),
        "failed fetch must not mark the source verified"
    );
}
