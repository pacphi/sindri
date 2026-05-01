//! Integration coverage for the embedded-key lookup path (F-REG-01,
//! Phase 3 of the 2026-04-30 reconciliation plan).
//!
//! The production [`EMBEDDED_KEYS`] slice is empty today (real
//! `sindri-core` cosign signing infrastructure is a prerequisite track),
//! so we cannot exercise it directly. Instead this test wires a
//! synthetic [`EmbeddedKey`] backed by a runtime-generated ECDSA P-256
//! keypair through `CosignVerifier::load_with_embedded`, signs a
//! cosign-style payload, and verifies the round-trip succeeds with no
//! disk-trust population at all.
//!
//! What this proves:
//! - Embedded keys are honored even when `~/.sindri/trust/` is empty.
//! - The active-key filter respects the rotation `valid_until` window.
//! - Embedded-key SPKI parsing follows the same path disk-loaded keys do.
//! - Multiple embedded entries for the same registry alias are merged.

use ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};
use p256::pkcs8::{EncodePublicKey, LineEnding};
use rand_core::OsRng;
use sindri_registry::{CosignVerifier, EmbeddedKey};
use std::path::Path;
use tempfile::TempDir;

/// Build a single embedded-key entry from a generated public key.
///
/// The static lifetime is satisfied by `Box::leak` — only suitable in
/// tests, never in production where keys are `include_bytes!`-ed at
/// compile time.
fn leak_embedded(
    alias: &'static str,
    key_id: &'static str,
    pem: String,
    valid_until: Option<&'static str>,
    generation: u32,
) -> EmbeddedKey {
    let bytes: &'static [u8] = Box::leak(pem.into_bytes().into_boxed_slice());
    EmbeddedKey {
        registry_alias: alias,
        key_id,
        spki_pem: bytes,
        valid_until,
        generation,
    }
}

/// Cosign simple-signing payload — only `critical.image.docker-manifest-digest`
/// is consulted by the verifier; the rest is structural ballast.
fn cosign_payload(manifest_digest: &str) -> Vec<u8> {
    let payload = serde_json::json!({
        "critical": {
            "identity": { "docker-reference": "ghcr.io/example/repo" },
            "image": { "docker-manifest-digest": manifest_digest },
            "type": "cosign container image signature"
        },
        "optional": null
    });
    serde_json::to_vec(&payload).unwrap()
}

#[test]
fn embedded_key_verifies_signature_with_empty_disk_trust() {
    // Generate keypair.
    let signing = SigningKey::random(&mut OsRng);
    let verifying = signing.verifying_key();
    let pem = verifying.to_public_key_pem(LineEnding::LF).unwrap();

    // Build a synthetic embedded set (no expiry → always active).
    let embedded = vec![leak_embedded("sindri-core", "deadbeef", pem, None, 1)];

    // Verifier load — empty disk trust dir, embedded keys carry the trust.
    let empty_dir = TempDir::new().unwrap();
    let verifier =
        CosignVerifier::load_with_embedded(empty_dir.path(), &embedded, "2026-05-01T00:00:00Z")
            .expect("load_with_embedded");

    let registries: Vec<&str> = verifier.trusted_registries().collect();
    assert_eq!(
        registries,
        vec!["sindri-core"],
        "embedded entry should surface as a trusted registry"
    );
    assert_eq!(verifier.keys_for("sindri-core").len(), 1);

    // Round-trip: sign a payload, verify against the embedded key.
    let manifest_digest = format!("sha256:{}", "a".repeat(64));
    let payload = cosign_payload(&manifest_digest);
    let sig: Signature = signing.sign(&payload);
    let sig_bytes = sig.to_der().as_bytes().to_vec();

    let key_id = verifier
        .verify_payload("sindri-core", &payload, &sig_bytes, &manifest_digest, true)
        .expect("embedded key should verify the round-trip signature");
    assert!(!key_id.is_empty());
    assert_ne!(key_id, "<unsigned>");
}

#[test]
fn embedded_key_filtered_when_valid_until_in_past() {
    // Generate keypair.
    let signing = SigningKey::random(&mut OsRng);
    let verifying = signing.verifying_key();
    let pem = verifying.to_public_key_pem(LineEnding::LF).unwrap();

    // Embedded entry expired 2026-01-01.
    let embedded = vec![leak_embedded(
        "sindri-core",
        "expired1",
        pem,
        Some("2026-01-01T00:00:00Z"),
        1,
    )];

    let empty_dir = TempDir::new().unwrap();
    // Clock is 2026-05-01 — past the expiry. Active filter drops the key.
    let verifier =
        CosignVerifier::load_with_embedded(empty_dir.path(), &embedded, "2026-05-01T00:00:00Z")
            .expect("load_with_embedded");

    assert_eq!(
        verifier.keys_for("sindri-core").len(),
        0,
        "expired embedded key must be filtered out"
    );

    // The verifier is otherwise fine — just no trusted keys for this registry.
    assert!(
        verifier.trusted_registries().count() == 0,
        "no trusted registries when only key is expired"
    );
}

#[test]
fn embedded_keys_merge_across_rotation_overlap() {
    // Two generations of the same registry's key — both active.
    let sk_a = SigningKey::random(&mut OsRng);
    let sk_b = SigningKey::random(&mut OsRng);
    let pem_a = sk_a
        .verifying_key()
        .to_public_key_pem(LineEnding::LF)
        .unwrap();
    let pem_b = sk_b
        .verifying_key()
        .to_public_key_pem(LineEnding::LF)
        .unwrap();

    let embedded = vec![
        leak_embedded(
            "sindri-core",
            "gen0001a",
            pem_a,
            Some("2027-01-01T00:00:00Z"),
            1,
        ),
        leak_embedded("sindri-core", "gen0002b", pem_b, None, 2),
    ];

    let empty_dir = TempDir::new().unwrap();
    let verifier =
        CosignVerifier::load_with_embedded(empty_dir.path(), &embedded, "2026-05-01T00:00:00Z")
            .expect("load_with_embedded");

    assert_eq!(
        verifier.keys_for("sindri-core").len(),
        2,
        "both rotation generations must be active during the overlap window"
    );

    // Either key should verify a signature it produced.
    let manifest_digest = format!("sha256:{}", "b".repeat(64));
    let payload = cosign_payload(&manifest_digest);
    for sk in [&sk_a, &sk_b] {
        let sig: Signature = sk.sign(&payload);
        let sig_bytes = sig.to_der().as_bytes().to_vec();
        verifier
            .verify_payload("sindri-core", &payload, &sig_bytes, &manifest_digest, true)
            .expect("rotation overlap: either generation should verify");
    }
}

#[test]
fn empty_embedded_set_falls_back_to_disk() {
    // Sanity guard: when no embedded keys, behavior matches the
    // pre-Phase-3 disk-only loader.
    let empty: &[EmbeddedKey] = &[];
    let empty_dir: &Path = Path::new("/this-path-does-not-exist-and-thats-fine");
    let verifier = CosignVerifier::load_with_embedded(empty_dir, empty, "2026-05-01T00:00:00Z")
        .expect("load_with_embedded");
    assert_eq!(verifier.trusted_registries().count(), 0);
}
