//! Cosign trust-key loading (ADR-014 §"Trust model").
//!
//! Wave 3A.1 owns trust-key **loading** only. Verification — fetching the
//! signature manifest, decoding the simple-signing payload, and verifying
//! signature bytes — is deferred to Wave 3A.2 so we don't ship a verifier
//! that quietly accepts everything.
//!
//! ## On-disk layout
//!
//! ```text
//! ~/.sindri/trust/
//!   <registry-name>/
//!     cosign-<short-key-id>.pub
//! ```
//!
//! Each `.pub` file is an ECDSA P-256 public key in PEM (PKCS#8 SPKI) form,
//! the default cosign key format.

use crate::error::RegistryError;
use ecdsa::elliptic_curve::pkcs8::DecodePublicKey;
use p256::ecdsa::VerifyingKey;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// A single trusted cosign key, parsed from PEM.
#[derive(Debug, Clone)]
pub struct TrustedKey {
    /// First 8 hex chars of the SHA-256 of the DER-encoded SPKI bytes.
    /// Stable, human-friendly identifier suitable for filenames + log lines.
    pub key_id: String,
    /// Original PEM (re-export / debug).
    pub spki_pem: String,
    /// The parsed verifying key.
    pub key: VerifyingKey,
}

impl TrustedKey {
    /// Parse a PEM public key, computing its short key id.
    pub fn from_pem(pem: &str) -> Result<Self, RegistryError> {
        let key = VerifyingKey::from_public_key_pem(pem).map_err(|e| {
            RegistryError::TrustKeyParseFailed {
                path: "<pem>".to_string(),
                detail: e.to_string(),
            }
        })?;
        let spki_der = key.to_encoded_point(false);
        let mut hasher = Sha256::new();
        hasher.update(spki_der.as_bytes());
        let digest = hasher.finalize();
        let key_id = hex::encode(&digest[..4]);
        Ok(TrustedKey {
            key_id,
            spki_pem: pem.to_string(),
            key,
        })
    }
}

/// Set of trusted cosign keys, indexed by registry name.
///
/// Built by [`CosignVerifier::load_from_trust_dir`] from
/// `~/.sindri/trust/<registry>/cosign-*.pub`.
#[derive(Debug)]
pub struct CosignVerifier {
    trusted_keys: HashMap<String, Vec<TrustedKey>>,
}

impl CosignVerifier {
    /// Load all trust keys under `root` (typically `~/.sindri/trust/`).
    ///
    /// - Each immediate subdirectory is treated as a registry name.
    /// - Inside each subdirectory, every file matching `cosign-*.pub` is
    ///   parsed as a P-256 public key.
    /// - A malformed key file aborts the whole load with
    ///   [`RegistryError::TrustKeyParseFailed`] — we fail closed rather than
    ///   silently dropping bad keys.
    /// - A non-existent `root` is treated as an empty trust set.
    pub fn load_from_trust_dir(root: &Path) -> Result<Self, RegistryError> {
        let mut trusted_keys: HashMap<String, Vec<TrustedKey>> = HashMap::new();
        if !root.exists() {
            return Ok(CosignVerifier { trusted_keys });
        }
        for entry in fs::read_dir(root)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let registry_name = match path.file_name().and_then(|s| s.to_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };
            let mut keys: Vec<TrustedKey> = Vec::new();
            for key_entry in fs::read_dir(&path)? {
                let key_entry = key_entry?;
                let key_path = key_entry.path();
                if !key_path.is_file() {
                    continue;
                }
                let name = key_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default();
                if !name.starts_with("cosign-") || !name.ends_with(".pub") {
                    continue;
                }
                let pem = fs::read_to_string(&key_path)?;
                let key = TrustedKey::from_pem(&pem).map_err(|e| match e {
                    RegistryError::TrustKeyParseFailed { detail, .. } => {
                        RegistryError::TrustKeyParseFailed {
                            path: key_path.display().to_string(),
                            detail,
                        }
                    }
                    other => other,
                })?;
                keys.push(key);
            }
            if !keys.is_empty() {
                trusted_keys.insert(registry_name, keys);
            }
        }
        Ok(CosignVerifier { trusted_keys })
    }

    /// Iterator over registry names with at least one trusted key.
    pub fn trusted_registries(&self) -> impl Iterator<Item = &str> {
        self.trusted_keys.keys().map(|s| s.as_str())
    }

    /// Trusted keys for `registry`, or an empty slice.
    pub fn keys_for(&self, registry: &str) -> &[TrustedKey] {
        self.trusted_keys
            .get(registry)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::ecdsa::SigningKey;
    use p256::pkcs8::EncodePublicKey;
    use rand_core::OsRng;
    use std::fs;
    use tempfile::TempDir;

    fn write_test_key(dir: &Path, registry: &str, key_idx: usize) -> String {
        let registry_dir = dir.join(registry);
        fs::create_dir_all(&registry_dir).unwrap();
        let signing = SigningKey::random(&mut OsRng);
        let verifying = VerifyingKey::from(&signing);
        let pem = verifying
            .to_public_key_pem(p256::pkcs8::LineEnding::LF)
            .unwrap();
        let path = registry_dir.join(format!("cosign-test-{}.pub", key_idx));
        fs::write(&path, &pem).unwrap();
        pem
    }

    #[test]
    fn loads_fixture_key() {
        let tmp = TempDir::new().unwrap();
        write_test_key(tmp.path(), "ghcr.io_sindri", 0);
        let verifier = CosignVerifier::load_from_trust_dir(tmp.path()).unwrap();
        let registries: Vec<&str> = verifier.trusted_registries().collect();
        assert_eq!(registries, vec!["ghcr.io_sindri"]);
        let keys = verifier.keys_for("ghcr.io_sindri");
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].key_id.len(), 8);
        assert!(keys[0].spki_pem.contains("BEGIN PUBLIC KEY"));
    }

    #[test]
    fn rejects_malformed_pem() {
        let tmp = TempDir::new().unwrap();
        let registry_dir = tmp.path().join("acme");
        fs::create_dir_all(&registry_dir).unwrap();
        fs::write(
            registry_dir.join("cosign-bad.pub"),
            "-----BEGIN PUBLIC KEY-----\nnot-pem\n-----END PUBLIC KEY-----\n",
        )
        .unwrap();
        let err = CosignVerifier::load_from_trust_dir(tmp.path()).unwrap_err();
        assert!(
            matches!(err, RegistryError::TrustKeyParseFailed { ref path, .. } if path.contains("cosign-bad.pub"))
        );
    }

    #[test]
    fn supports_multiple_keys_per_registry() {
        let tmp = TempDir::new().unwrap();
        write_test_key(tmp.path(), "acme", 0);
        write_test_key(tmp.path(), "acme", 1);
        let verifier = CosignVerifier::load_from_trust_dir(tmp.path()).unwrap();
        let keys = verifier.keys_for("acme");
        assert_eq!(keys.len(), 2);
        assert_ne!(keys[0].key_id, keys[1].key_id);
    }

    #[test]
    fn empty_trust_dir_yields_empty_verifier() {
        let tmp = TempDir::new().unwrap();
        let verifier = CosignVerifier::load_from_trust_dir(tmp.path()).unwrap();
        assert_eq!(verifier.trusted_registries().count(), 0);
    }

    #[test]
    fn nonexistent_root_yields_empty_verifier() {
        let tmp = TempDir::new().unwrap();
        let missing = tmp.path().join("does-not-exist");
        let verifier = CosignVerifier::load_from_trust_dir(&missing).unwrap();
        assert_eq!(verifier.trusted_registries().count(), 0);
    }

    #[test]
    fn ignores_non_cosign_files() {
        let tmp = TempDir::new().unwrap();
        write_test_key(tmp.path(), "acme", 0);
        let registry_dir = tmp.path().join("acme");
        fs::write(registry_dir.join("README.md"), "not a key").unwrap();
        fs::write(registry_dir.join("other.pub"), "not loaded").unwrap();
        let verifier = CosignVerifier::load_from_trust_dir(tmp.path()).unwrap();
        assert_eq!(verifier.keys_for("acme").len(), 1);
    }
}
