//! Wave 6A — ADR-014 D1 keyless cosign integration tests.
//!
//! These tests cover the parts of [`sindri_registry::keyless`] that are
//! awkward to exercise as pure unit tests because they need:
//!
//! 1. A synthesised Fulcio CA → leaf cert chain with a real SAN URI +
//!    OIDC issuer extension.
//! 2. A wiremock'd Rekor lookup endpoint (for the future detached-signature
//!    online path; today's bundle path is fully offline).
//!
//! Cert synthesis is done with `rcgen` (dev-dep only — never reaches a
//! production build). The signing key shape is P-256 ECDSA so we exercise
//! the same crypto path as production Fulcio.

#![cfg(feature = "keyless")]

use rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, KeyPair, KeyUsagePurpose, SanType};
use sindri_registry::keyless::{
    EnvelopeKind, KeylessIdentity, KeylessTrustRoot, KeylessVerifier, SignatureEnvelope,
    VerificationMode,
};
use sindri_registry::RegistryError;

const FULCIO_OIDC_OID_LEGACY: &str = "1.3.6.1.4.1.57264.1.1";

/// Build a synthetic root CA + leaf cert pair. The leaf carries:
/// - SAN URI = `san_uri`
/// - Custom extension OID `1.3.6.1.4.1.57264.1.1` containing `oidc_issuer`
/// - Validity window = `[not_before, not_after]` (Unix seconds)
///
/// Returns `(leaf_pem, root_pem)`.
fn make_chain(
    san_uri: &str,
    oidc_issuer: &str,
    not_before: time::OffsetDateTime,
    not_after: time::OffsetDateTime,
) -> (String, String) {
    make_chain_with_root_cn(
        san_uri,
        oidc_issuer,
        not_before,
        not_after,
        "Sindri Test Fulcio Root",
    )
}

fn make_chain_with_root_cn(
    san_uri: &str,
    oidc_issuer: &str,
    not_before: time::OffsetDateTime,
    not_after: time::OffsetDateTime,
    root_cn: &str,
) -> (String, String) {
    // 1. Root CA.
    let mut root_params = CertificateParams::new(vec![root_cn.into()]).unwrap();
    root_params
        .distinguished_name
        .push(DnType::CommonName, root_cn);
    root_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    root_params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    let root_kp = KeyPair::generate().unwrap();
    let root_cert = root_params.self_signed(&root_kp).unwrap();

    // 2. Leaf — custom SAN URI + Fulcio OIDC issuer extension.
    let mut leaf_params = CertificateParams::new(vec![]).unwrap();
    leaf_params
        .distinguished_name
        .push(DnType::CommonName, "Sindri Test Cosign Cert");
    leaf_params.subject_alt_names = vec![SanType::URI(san_uri.try_into().unwrap())];
    leaf_params.not_before = not_before;
    leaf_params.not_after = not_after;
    // Fulcio OIDC issuer custom extension — encoded as a raw UTF-8 string
    // inside the extension value. rcgen's CustomExtension takes the raw
    // OCTET STRING contents (no DER wrapper inside).
    let issuer_ext = rcgen::CustomExtension::from_oid_content(
        &parse_oid(FULCIO_OIDC_OID_LEGACY),
        oidc_issuer.as_bytes().to_vec(),
    );
    leaf_params.custom_extensions = vec![issuer_ext];
    let leaf_kp = KeyPair::generate().unwrap();
    let leaf_cert = leaf_params
        .signed_by(&leaf_kp, &root_cert, &root_kp)
        .unwrap();

    (leaf_cert.pem(), root_cert.pem())
}

fn parse_oid(s: &str) -> Vec<u64> {
    s.split('.').map(|p| p.parse::<u64>().unwrap()).collect()
}

fn now_window(
    skew_secs_before: i64,
    skew_secs_after: i64,
) -> (time::OffsetDateTime, time::OffsetDateTime) {
    let now = time::OffsetDateTime::now_utc();
    let nb = now - time::Duration::seconds(skew_secs_before);
    let na = now + time::Duration::seconds(skew_secs_after);
    (nb, na)
}

/// Build a Rekor bundle JSON whose SET signs the canonicalised payload
/// with `signing_key`. Returned bytes plug straight into
/// `SignatureEnvelope::bundle_json`. The integratedTime is configurable
/// so tests can drive the validity-window check.
fn make_rekor_bundle(integrated_time: i64, signing_pem: &str) -> Vec<u8> {
    use base64::Engine as _;
    use ecdsa::signature::Signer;
    use p256::ecdsa::{Signature, SigningKey};
    use p256::pkcs8::DecodePrivateKey;
    let sk = SigningKey::from_pkcs8_pem(signing_pem).unwrap();
    let canonical = serde_json::json!({
        "body": "Ym9keQ==",
        "integratedTime": integrated_time,
        "logIndex": 42_i64,
        "logID": "test-log",
    });
    let canonical_bytes = canonical_json(&canonical);
    let sig: Signature = sk.sign(&canonical_bytes);
    let set_b64 = base64::engine::general_purpose::STANDARD.encode(sig.to_der().as_bytes());
    let bundle = serde_json::json!({
        "SignedEntryTimestamp": set_b64,
        "Payload": {
            "body": "Ym9keQ==",
            "integratedTime": integrated_time,
            "logIndex": 42_i64,
            "logID": "test-log",
        }
    });
    serde_json::to_vec(&bundle).unwrap()
}

fn canonical_json(v: &serde_json::Value) -> Vec<u8> {
    let mut buf = Vec::new();
    canonicalise_into(v, &mut buf);
    buf
}

fn canonicalise_into(v: &serde_json::Value, out: &mut Vec<u8>) {
    match v {
        serde_json::Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push(b'{');
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                let key_escaped = serde_json::to_string(k).expect("string key");
                out.extend_from_slice(key_escaped.as_bytes());
                out.push(b':');
                canonicalise_into(&map[*k], out);
            }
            out.push(b'}');
        }
        serde_json::Value::Array(arr) => {
            out.push(b'[');
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                canonicalise_into(item, out);
            }
            out.push(b']');
        }
        other => {
            let s = serde_json::to_string(other).expect("scalar");
            out.extend_from_slice(s.as_bytes());
        }
    }
}

/// Build a Rekor signing key pair and return its (signing PKCS#8 PEM,
/// pubkey SPKI PEM).
fn make_rekor_keypair() -> (String, String) {
    use ecdsa::elliptic_curve::rand_core::OsRng;
    use p256::ecdsa::SigningKey;
    use p256::pkcs8::EncodePrivateKey;
    use p256::pkcs8::EncodePublicKey;
    let sk = SigningKey::random(&mut OsRng);
    let sk_pem = sk
        .to_pkcs8_pem(p256::pkcs8::LineEnding::LF)
        .unwrap()
        .to_string();
    let pk_pem = sk
        .verifying_key()
        .to_public_key_pem(p256::pkcs8::LineEnding::LF)
        .unwrap();
    (sk_pem, pk_pem)
}

// -- Tests -----------------------------------------------------------------

#[test]
fn keyless_happy_path_with_bundle_envelope() {
    let san =
        "https://github.com/sindri-dev/registry/.github/workflows/publish.yml@refs/heads/main";
    let issuer = "https://token.actions.githubusercontent.com";
    let (nb, na) = now_window(60, 86_400);
    let (leaf_pem, root_pem) = make_chain(san, issuer, nb, na);
    let (rekor_sk_pem, rekor_pk_pem) = make_rekor_keypair();
    let integrated_time = time::OffsetDateTime::now_utc().unix_timestamp();
    let bundle = make_rekor_bundle(integrated_time, &rekor_sk_pem);

    let trust =
        KeylessTrustRoot::from_pem(root_pem.into_bytes(), rekor_pk_pem.into_bytes()).unwrap();
    let v = KeylessVerifier::new(trust);
    let env = SignatureEnvelope {
        signature: Some(b"sig".to_vec()),
        cert_pem: Some(leaf_pem.into_bytes()),
        bundle_json: Some(bundle),
    };
    assert_eq!(env.kind(), EnvelopeKind::Bundle);
    let identity = KeylessIdentity {
        san_uri: san.into(),
        issuer: issuer.into(),
    };
    let matched = v
        .verify("ghcr.io_sindri", &env, &identity, b"payload", "sha256:abc")
        .expect("happy path should verify");
    assert_eq!(matched, san);
}

#[test]
fn keyless_rejects_san_mismatch() {
    let san =
        "https://github.com/sindri-dev/registry/.github/workflows/publish.yml@refs/heads/main";
    let issuer = "https://token.actions.githubusercontent.com";
    let (nb, na) = now_window(60, 86_400);
    let (leaf_pem, root_pem) = make_chain(san, issuer, nb, na);
    let (rekor_sk_pem, rekor_pk_pem) = make_rekor_keypair();
    let bundle = make_rekor_bundle(
        time::OffsetDateTime::now_utc().unix_timestamp(),
        &rekor_sk_pem,
    );

    let trust =
        KeylessTrustRoot::from_pem(root_pem.into_bytes(), rekor_pk_pem.into_bytes()).unwrap();
    let v = KeylessVerifier::new(trust);
    let env = SignatureEnvelope {
        signature: Some(b"sig".to_vec()),
        cert_pem: Some(leaf_pem.into_bytes()),
        bundle_json: Some(bundle),
    };
    let wrong_identity = KeylessIdentity {
        san_uri: "https://github.com/attacker/evil/.github/workflows/release.yml@refs/heads/main"
            .into(),
        issuer: issuer.into(),
    };
    let err = v
        .verify(
            "ghcr.io_sindri",
            &env,
            &wrong_identity,
            b"payload",
            "sha256:abc",
        )
        .unwrap_err();
    assert!(
        matches!(err, RegistryError::KeylessIdentityMismatch { .. }),
        "expected SAN mismatch, got {:?}",
        err
    );
}

#[test]
fn keyless_rejects_bad_issuer() {
    let san =
        "https://github.com/sindri-dev/registry/.github/workflows/publish.yml@refs/heads/main";
    let issuer = "https://token.actions.githubusercontent.com";
    let (nb, na) = now_window(60, 86_400);
    let (leaf_pem, root_pem) = make_chain(san, issuer, nb, na);
    let (rekor_sk_pem, rekor_pk_pem) = make_rekor_keypair();
    let bundle = make_rekor_bundle(
        time::OffsetDateTime::now_utc().unix_timestamp(),
        &rekor_sk_pem,
    );

    let trust =
        KeylessTrustRoot::from_pem(root_pem.into_bytes(), rekor_pk_pem.into_bytes()).unwrap();
    let v = KeylessVerifier::new(trust);
    let env = SignatureEnvelope {
        signature: Some(b"sig".to_vec()),
        cert_pem: Some(leaf_pem.into_bytes()),
        bundle_json: Some(bundle),
    };
    let wrong_issuer_identity = KeylessIdentity {
        san_uri: san.into(),
        issuer: "https://gitlab.example.com".into(),
    };
    let err = v
        .verify(
            "ghcr.io_sindri",
            &env,
            &wrong_issuer_identity,
            b"payload",
            "sha256:abc",
        )
        .unwrap_err();
    assert!(matches!(err, RegistryError::KeylessIdentityMismatch { .. }));
}

#[test]
fn keyless_rejects_cert_from_untrusted_ca() {
    let san =
        "https://github.com/sindri-dev/registry/.github/workflows/publish.yml@refs/heads/main";
    let issuer = "https://token.actions.githubusercontent.com";
    let (nb, na) = now_window(60, 86_400);
    // Build a leaf signed by CA-A but ship CA-B (different DN) as the
    // trust bundle. With distinct CN strings the chain validator's
    // issuer-DN check rejects the cert.
    let (leaf_pem, _good_root_pem) = make_chain_with_root_cn(san, issuer, nb, na, "Real Fulcio CA");
    let (_, bad_root_pem) = make_chain_with_root_cn(san, issuer, nb, na, "Some Other CA");
    let (rekor_sk_pem, rekor_pk_pem) = make_rekor_keypair();
    let bundle = make_rekor_bundle(
        time::OffsetDateTime::now_utc().unix_timestamp(),
        &rekor_sk_pem,
    );

    let trust =
        KeylessTrustRoot::from_pem(bad_root_pem.into_bytes(), rekor_pk_pem.into_bytes()).unwrap();
    let v = KeylessVerifier::new(trust);
    let env = SignatureEnvelope {
        signature: Some(b"sig".to_vec()),
        cert_pem: Some(leaf_pem.into_bytes()),
        bundle_json: Some(bundle),
    };
    let id = KeylessIdentity {
        san_uri: san.into(),
        issuer: issuer.into(),
    };
    let err = v
        .verify("ghcr.io_sindri", &env, &id, b"payload", "sha256:abc")
        .unwrap_err();
    assert!(
        matches!(err, RegistryError::FulcioChainInvalid { .. }),
        "expected FulcioChainInvalid, got {:?}",
        err
    );
}

#[test]
fn keyless_rejects_expired_cert() {
    let san =
        "https://github.com/sindri-dev/registry/.github/workflows/publish.yml@refs/heads/main";
    let issuer = "https://token.actions.githubusercontent.com";
    // Cert validity ends *before* Rekor's integratedTime — caught by the
    // window check.
    let now = time::OffsetDateTime::now_utc();
    let nb = now - time::Duration::seconds(86_400 * 2);
    let na = now - time::Duration::seconds(60); // expired 60s ago
    let (leaf_pem, root_pem) = make_chain(san, issuer, nb, na);
    let (rekor_sk_pem, rekor_pk_pem) = make_rekor_keypair();
    // Rekor says it integrated this entry "now" — outside the cert window.
    let bundle = make_rekor_bundle(now.unix_timestamp(), &rekor_sk_pem);

    let trust =
        KeylessTrustRoot::from_pem(root_pem.into_bytes(), rekor_pk_pem.into_bytes()).unwrap();
    let v = KeylessVerifier::new(trust);
    let env = SignatureEnvelope {
        signature: Some(b"sig".to_vec()),
        cert_pem: Some(leaf_pem.into_bytes()),
        bundle_json: Some(bundle),
    };
    let id = KeylessIdentity {
        san_uri: san.into(),
        issuer: issuer.into(),
    };
    let err = v
        .verify("ghcr.io_sindri", &env, &id, b"payload", "sha256:abc")
        .unwrap_err();
    assert!(
        matches!(err, RegistryError::KeylessCertificateExpired { .. }),
        "expected KeylessCertificateExpired, got {:?}",
        err
    );
}

#[test]
fn keyless_rejects_tampered_inclusion_proof() {
    let san =
        "https://github.com/sindri-dev/registry/.github/workflows/publish.yml@refs/heads/main";
    let issuer = "https://token.actions.githubusercontent.com";
    let (nb, na) = now_window(60, 86_400);
    let (leaf_pem, root_pem) = make_chain(san, issuer, nb, na);
    let (rekor_sk_pem, rekor_pk_pem) = make_rekor_keypair();
    let mut bundle = make_rekor_bundle(
        time::OffsetDateTime::now_utc().unix_timestamp(),
        &rekor_sk_pem,
    );
    // Mutate Payload.integratedTime — invalidates SET signature.
    let mut v: serde_json::Value = serde_json::from_slice(&bundle).unwrap();
    v["Payload"]["integratedTime"] =
        serde_json::json!(time::OffsetDateTime::now_utc().unix_timestamp() + 99_999);
    bundle = serde_json::to_vec(&v).unwrap();

    let trust =
        KeylessTrustRoot::from_pem(root_pem.into_bytes(), rekor_pk_pem.into_bytes()).unwrap();
    let verifier = KeylessVerifier::new(trust);
    let env = SignatureEnvelope {
        signature: Some(b"sig".to_vec()),
        cert_pem: Some(leaf_pem.into_bytes()),
        bundle_json: Some(bundle),
    };
    let id = KeylessIdentity {
        san_uri: san.into(),
        issuer: issuer.into(),
    };
    let err = verifier
        .verify("ghcr.io_sindri", &env, &id, b"payload", "sha256:abc")
        .unwrap_err();
    assert!(
        matches!(err, RegistryError::RekorInclusionProofInvalid { .. }),
        "expected RekorInclusionProofInvalid, got {:?}",
        err
    );
}

#[test]
fn detached_envelope_is_rejected_until_rekor_lookup_is_wired() {
    // Wave 6A only supports bundle-format keyless verification offline;
    // detached signatures (no inline cert + bundle) need an online Rekor
    // lookup which lands in a follow-up. Confirm we fail closed in the
    // meantime rather than silently accepting unsigned material.
    let trust = KeylessTrustRoot::from_pem(b"trust".to_vec(), b"rekor".to_vec()).unwrap();
    let v = KeylessVerifier::new(trust);
    let env = SignatureEnvelope {
        signature: Some(b"sig".to_vec()),
        cert_pem: None,
        bundle_json: None,
    };
    assert_eq!(env.kind(), EnvelopeKind::Detached);
    let id = KeylessIdentity {
        san_uri: "x".into(),
        issuer: "y".into(),
    };
    let err = v
        .verify("acme", &env, &id, b"payload", "sha256:abc")
        .unwrap_err();
    assert!(matches!(
        err,
        RegistryError::KeylessCertificateMissing { .. }
    ));
}

#[test]
fn verification_mode_round_trips_through_serde() {
    // Confirm the policy field deserialises from the kebab-case form
    // we put into `RegistryConfig::verification_mode` strings.
    assert_eq!(
        VerificationMode::parse("key-based").unwrap(),
        VerificationMode::KeyBased
    );
    assert_eq!(
        VerificationMode::parse("keyless").unwrap(),
        VerificationMode::Keyless
    );
}
