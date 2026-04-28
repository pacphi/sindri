//! Wave 6A.1 — per-component trust override wiremock integration tests.
//!
//! Follow-up to PR #228 (per-component cosign — `oci_wiremock.rs`) and
//! PR #237 (keyless — `keyless_wiremock.rs`). Exercises the
//! [`sindri_registry::CosignVerifier::verify_component_signature_scoped`]
//! path end-to-end against an in-process OCI mock, covering the
//! precedence rules called out in ADR-014's Wave 6A.1 section:
//!
//! - Component A under glob → verifies against override key. Registry
//!   key must FAIL.
//! - Component B not matching any glob → verifies against registry
//!   fallback. Override key must FAIL.
//! - Component matching multiple globs → most-specific wins.

use base64::Engine as _;
use ecdsa::signature::Signer;
use oci_client::client::{ClientConfig, ClientProtocol};
use oci_client::Client as OciClient;
use p256::ecdsa::{Signature, SigningKey, VerifyingKey};
use p256::pkcs8::EncodePublicKey;
use rand_core::OsRng;
use sha2::{Digest, Sha256};
use sindri_core::manifest::TrustOverride;
use sindri_registry::{CosignVerifier, OciRef};
use std::path::PathBuf;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const COSIGN_SIMPLESIGNING: &str = "application/vnd.dev.cosign.simplesigning.v1+json";
const COSIGN_SIG_ANNOTATION: &str = "dev.cosignproject.cosign/signature";

fn sha256_hex(b: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(b);
    hex::encode(h.finalize())
}

fn http_oci_client() -> OciClient {
    OciClient::new(ClientConfig {
        protocol: ClientProtocol::Http,
        ..ClientConfig::default()
    })
}

fn write_pem_key_to(tmp: &TempDir, name: &str) -> (SigningKey, PathBuf) {
    let signing = SigningKey::random(&mut OsRng);
    let verifying = VerifyingKey::from(&signing);
    let pem = verifying
        .to_public_key_pem(p256::pkcs8::LineEnding::LF)
        .unwrap();
    let path = tmp.path().join(format!("{}.pub", name));
    std::fs::write(&path, pem.as_bytes()).unwrap();
    (signing, path)
}

/// Sanitise a registry endpoint (`host:port`) into a filesystem-safe form.
///
/// Required because `:` is a reserved character in Windows paths (drive
/// separator), so a directory named `127.0.0.1:5555` cannot be created on
/// Windows runners. The sanitised string is used both as the trust-dir
/// name AND as the registry-name argument to the verifier — the verifier
/// uses it as a HashMap key, so consistency on both sides is sufficient.
fn safe_registry_name(registry: &str) -> String {
    registry
        .chars()
        .map(|c| if c == ':' || c == '/' { '_' } else { c })
        .collect()
}

/// Stage a registry-level trust key under `~/.sindri/trust/<registry>/`
/// shape that [`CosignVerifier::load_from_trust_dir`] expects. The caller
/// is responsible for passing a filesystem-safe `registry` string —
/// typically derived via [`safe_registry_name`].
fn write_registry_trust_dir(root: &std::path::Path, registry: &str) -> SigningKey {
    let signing = SigningKey::random(&mut OsRng);
    let verifying = VerifyingKey::from(&signing);
    let pem = verifying
        .to_public_key_pem(p256::pkcs8::LineEnding::LF)
        .unwrap();
    let dir = root.join(registry);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("cosign-registry.pub"), pem.as_bytes()).unwrap();
    signing
}

/// Build a complete cosign-style signature manifest + simple-signing
/// layer pair, signed by `signing_key` over `manifest_digest`. Returns
/// the wire bytes for the signature manifest plus the raw layer body.
fn build_signed_artifact(
    signing_key: &SigningKey,
    manifest_digest: &str,
) -> (String, Vec<u8>, String) {
    let payload = serde_json::json!({
        "critical": {
            "identity": { "docker-reference": "ghcr.io/example/repo" },
            "image": { "docker-manifest-digest": manifest_digest },
            "type": "cosign container image signature"
        },
        "optional": null
    });
    let payload_bytes = serde_json::to_vec(&payload).unwrap();
    let layer_digest = format!("sha256:{}", sha256_hex(&payload_bytes));
    let sig: Signature = signing_key.sign(&payload_bytes);
    let sig_b64 = base64::engine::general_purpose::STANDARD.encode(sig.to_der().as_bytes());
    let sig_manifest = serde_json::json!({
        "schemaVersion": 2,
        "mediaType": "application/vnd.oci.image.manifest.v1+json",
        "config": {
            "mediaType": "application/vnd.oci.image.config.v1+json",
            "digest": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
            "size": 0
        },
        "layers": [{
            "mediaType": COSIGN_SIMPLESIGNING,
            "digest": layer_digest,
            "size": payload_bytes.len()
        }],
        "annotations": {
            COSIGN_SIG_ANNOTATION: sig_b64
        }
    });
    (sig_manifest.to_string(), payload_bytes, layer_digest)
}

/// Mount cosign-signature-tag handlers for `manifest_digest` under
/// `repo` on `server`. Returns the expected sig-tag for assertion.
async fn mount_cosign_signature(
    server: &MockServer,
    repo: &str,
    manifest_digest: &str,
    signing_key: &SigningKey,
) -> String {
    let (sig_manifest, layer_bytes, layer_digest) =
        build_signed_artifact(signing_key, manifest_digest);
    // Cosign tag is `sha256-<hex>.sig`.
    let sig_tag = manifest_digest.replace(':', "-") + ".sig";

    Mock::given(method("GET"))
        .and(path(format!("/v2/{}/manifests/{}", repo, sig_tag)))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/vnd.oci.image.manifest.v1+json")
                .set_body_string(sig_manifest),
        )
        .mount(server)
        .await;

    Mock::given(method("GET"))
        .and(path(format!("/v2/{}/blobs/{}", repo, layer_digest)))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(layer_bytes))
        .mount(server)
        .await;

    sig_tag
}

const REPO: &str = "team-foo/svc";

/// Component A — under `team-foo/*` glob → must verify against the
/// override key. Registry-level key must NOT be used.
#[tokio::test]
async fn override_key_verifies_matching_component() {
    let server = MockServer::start().await;
    let registry_endpoint = server.uri().trim_start_matches("http://").to_string();
    let safe_registry = safe_registry_name(&registry_endpoint);

    // Set up a trust dir with the registry-level key (which we expect NOT to be consulted).
    let trust_dir = TempDir::new().unwrap();
    let _registry_key_signing = write_registry_trust_dir(trust_dir.path(), &safe_registry);

    // Override key — separate from the registry-level key.
    let override_dir = TempDir::new().unwrap();
    let (override_signing, override_path) = write_pem_key_to(&override_dir, "override-key");

    // Sign the artifact with the OVERRIDE key only.
    let manifest_digest = format!("sha256:{}", "a".repeat(64));
    let _sig_tag = mount_cosign_signature(&server, REPO, &manifest_digest, &override_signing).await;
    // Mount /v2/ probe to keep oci-client happy.
    Mock::given(method("GET"))
        .and(path("/v2/"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    // Build verifier + override list.
    let verifier = CosignVerifier::load_from_trust_dir(trust_dir.path()).unwrap();
    let overrides = vec![TrustOverride {
        component_glob: "team-foo/*".to_string(),
        keys: Some(vec![override_path]),
        identity: None,
    }];

    let oci_url = format!("{}/{}:1.0.0", registry_endpoint, REPO);
    let oci_ref = OciRef::parse(&oci_url).unwrap();
    let oci = http_oci_client();

    let key_id = verifier
        .verify_component_signature_scoped(
            &oci,
            &safe_registry,
            "team-foo/svc",
            &oci_ref,
            &manifest_digest,
            &overrides,
            true, // strict policy
        )
        .await
        .expect("override key must verify");
    // The matched key id is the override key's id. We don't know the
    // exact value (random) but can assert it's an 8-char hex and not
    // the `<unsigned>` sentinel.
    assert_eq!(key_id.len(), 8);
    assert_ne!(key_id, "<unsigned>");
}

/// Component B — does NOT match any glob → must fall back to
/// registry-level trust. Override key must NOT verify the signature.
#[tokio::test]
async fn registry_fallback_for_unmatched_component() {
    let server = MockServer::start().await;
    let registry_endpoint = server.uri().trim_start_matches("http://").to_string();
    let safe_registry = safe_registry_name(&registry_endpoint);

    let trust_dir = TempDir::new().unwrap();
    let registry_key_signing = write_registry_trust_dir(trust_dir.path(), &safe_registry);

    let override_dir = TempDir::new().unwrap();
    let (_override_signing, override_path) = write_pem_key_to(&override_dir, "override-key");

    // Sign with the REGISTRY key (the override doesn't apply to this component).
    let manifest_digest = format!("sha256:{}", "b".repeat(64));
    let _ = mount_cosign_signature(&server, REPO, &manifest_digest, &registry_key_signing).await;
    Mock::given(method("GET"))
        .and(path("/v2/"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let verifier = CosignVerifier::load_from_trust_dir(trust_dir.path()).unwrap();
    let overrides = vec![TrustOverride {
        // Override scoped to a totally different component prefix.
        component_glob: "team-bar/*".to_string(),
        keys: Some(vec![override_path]),
        identity: None,
    }];

    let oci_url = format!("{}/{}:1.0.0", registry_endpoint, REPO);
    let oci_ref = OciRef::parse(&oci_url).unwrap();
    let oci = http_oci_client();

    // Component address `team-foo/svc` doesn't match `team-bar/*` →
    // fall back to per-registry trust (which holds the registry key).
    let key_id = verifier
        .verify_component_signature_scoped(
            &oci,
            &safe_registry,
            "team-foo/svc",
            &oci_ref,
            &manifest_digest,
            &overrides,
            true,
        )
        .await
        .expect("registry-level fallback must verify a signature signed by the registry key");
    assert_eq!(key_id.len(), 8);
    assert_ne!(key_id, "<unsigned>");
}

/// Override key must FAIL when artifact was signed by the registry key
/// AND a matching override exists — override-takes-precedence means we
/// MUST NOT fall back to the registry key when an override applies.
#[tokio::test]
async fn override_takes_precedence_rejects_registry_signed_artifact() {
    let server = MockServer::start().await;
    let registry_endpoint = server.uri().trim_start_matches("http://").to_string();
    let safe_registry = safe_registry_name(&registry_endpoint);

    let trust_dir = TempDir::new().unwrap();
    let registry_key_signing = write_registry_trust_dir(trust_dir.path(), &safe_registry);

    let override_dir = TempDir::new().unwrap();
    let (_override_signing, override_path) = write_pem_key_to(&override_dir, "override-key");

    // Sign with the REGISTRY key, but the override matches the component.
    let manifest_digest = format!("sha256:{}", "c".repeat(64));
    let _ = mount_cosign_signature(&server, REPO, &manifest_digest, &registry_key_signing).await;
    Mock::given(method("GET"))
        .and(path("/v2/"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let verifier = CosignVerifier::load_from_trust_dir(trust_dir.path()).unwrap();
    let overrides = vec![TrustOverride {
        component_glob: "team-foo/*".to_string(),
        keys: Some(vec![override_path]),
        identity: None,
    }];

    let oci_url = format!("{}/{}:1.0.0", registry_endpoint, REPO);
    let oci_ref = OciRef::parse(&oci_url).unwrap();
    let oci = http_oci_client();

    let err = verifier
        .verify_component_signature_scoped(
            &oci,
            &safe_registry,
            "team-foo/svc",
            &oci_ref,
            &manifest_digest,
            &overrides,
            true,
        )
        .await
        .expect_err("override-takes-precedence must reject a registry-key signature");
    // SignatureMismatch — the override key list doesn't include the
    // registry key, so verification fails closed.
    let s = format!("{}", err);
    assert!(
        s.to_ascii_lowercase().contains("no trusted key matched")
            || s.to_ascii_lowercase().contains("signature mismatch"),
        "expected SignatureMismatch, got: {}",
        s
    );
}

/// Component matches multiple globs → most-specific (longest literal)
/// wins. Sign with key A keyed to the LESS specific glob; if the
/// most-specific key is selected, the verification fails closed.
#[tokio::test]
async fn most_specific_glob_wins_when_multiple_match() {
    let server = MockServer::start().await;
    let registry_endpoint = server.uri().trim_start_matches("http://").to_string();
    let safe_registry = safe_registry_name(&registry_endpoint);

    let trust_dir = TempDir::new().unwrap();
    let _registry_key_signing = write_registry_trust_dir(trust_dir.path(), &safe_registry);

    let override_dir = TempDir::new().unwrap();
    // Two override keys — broad and specific — keyed to the same component.
    let (broad_signing, broad_path) = write_pem_key_to(&override_dir, "broad-key");
    let (specific_signing, specific_path) = write_pem_key_to(&override_dir, "specific-key");

    // Sign with the SPECIFIC key — that's the one we expect to be selected.
    let manifest_digest = format!("sha256:{}", "d".repeat(64));
    let _ = mount_cosign_signature(&server, REPO, &manifest_digest, &specific_signing).await;
    Mock::given(method("GET"))
        .and(path("/v2/"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let verifier = CosignVerifier::load_from_trust_dir(trust_dir.path()).unwrap();
    let overrides = vec![
        TrustOverride {
            component_glob: "team-foo/*".to_string(), // broad
            keys: Some(vec![broad_path]),
            identity: None,
        },
        TrustOverride {
            component_glob: "team-foo/svc".to_string(), // specific (literal)
            keys: Some(vec![specific_path]),
            identity: None,
        },
    ];

    let oci_url = format!("{}/{}:1.0.0", registry_endpoint, REPO);
    let oci_ref = OciRef::parse(&oci_url).unwrap();
    let oci = http_oci_client();

    let key_id = verifier
        .verify_component_signature_scoped(
            &oci,
            &safe_registry,
            "team-foo/svc",
            &oci_ref,
            &manifest_digest,
            &overrides,
            true,
        )
        .await
        .expect("most-specific override (specific key) must verify a specific-signed artifact");
    assert_eq!(key_id.len(), 8);
    assert_ne!(key_id, "<unsigned>");
    // Cross-check: signing with the broad key and selecting the
    // specific override must fail closed.
    let _ = broad_signing;
}

/// Reverse of the above — sign with the broad key, but the verifier
/// selects the most-specific override. Result must be SignatureMismatch.
#[tokio::test]
async fn most_specific_glob_rejects_signature_from_less_specific_key() {
    let server = MockServer::start().await;
    let registry_endpoint = server.uri().trim_start_matches("http://").to_string();
    let safe_registry = safe_registry_name(&registry_endpoint);

    let trust_dir = TempDir::new().unwrap();
    let _registry_key_signing = write_registry_trust_dir(trust_dir.path(), &safe_registry);

    let override_dir = TempDir::new().unwrap();
    let (broad_signing, broad_path) = write_pem_key_to(&override_dir, "broad-key");
    let (_specific_signing, specific_path) = write_pem_key_to(&override_dir, "specific-key");

    // Sign with the BROAD key — but the most-specific override is keyed
    // to the SPECIFIC key, so verification must fail.
    let manifest_digest = format!("sha256:{}", "e".repeat(64));
    let _ = mount_cosign_signature(&server, REPO, &manifest_digest, &broad_signing).await;
    Mock::given(method("GET"))
        .and(path("/v2/"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let verifier = CosignVerifier::load_from_trust_dir(trust_dir.path()).unwrap();
    let overrides = vec![
        TrustOverride {
            component_glob: "team-foo/*".to_string(),
            keys: Some(vec![broad_path]),
            identity: None,
        },
        TrustOverride {
            component_glob: "team-foo/svc".to_string(),
            keys: Some(vec![specific_path]),
            identity: None,
        },
    ];

    let oci_url = format!("{}/{}:1.0.0", registry_endpoint, REPO);
    let oci_ref = OciRef::parse(&oci_url).unwrap();
    let oci = http_oci_client();

    let err = verifier
        .verify_component_signature_scoped(
            &oci,
            &safe_registry,
            "team-foo/svc",
            &oci_ref,
            &manifest_digest,
            &overrides,
            true,
        )
        .await
        .expect_err(
            "most-specific override must reject signatures from a less-specific override key",
        );
    let s = format!("{}", err).to_ascii_lowercase();
    assert!(
        s.contains("no trusted key matched") || s.contains("signature mismatch"),
        "expected SignatureMismatch, got: {}",
        s
    );
}
