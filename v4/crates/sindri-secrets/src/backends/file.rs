//! [`FileBackend`] — encrypted-at-rest secret store.
//!
//! ## Storage format
//!
//! The file at `~/.sindri/secrets.enc` contains:
//!
//! ```text
//! [12-byte nonce][ciphertext]
//! ```
//!
//! The ciphertext is a `serde_json`-serialised `HashMap<String, SecretValue>`,
//! encrypted with ChaCha20-Poly1305.  The 256-bit key is derived from a
//! passphrase using HKDF-SHA256 with a static info label `sindri-secrets-v1`.
//!
//! ## Key derivation
//!
//! ```text
//! IKM  = passphrase bytes (UTF-8)
//! salt = SHA-256( "sindri-file-backend-salt" || file-path )
//! info = b"sindri-secrets-v1"
//! OKM  = HKDF-SHA256(IKM, salt, info)[0..32]
//! ```
//!
//! The salt mixes in the file path so the same passphrase does not produce
//! the same key for different files.
//!
//! ## Thread safety
//!
//! `FileBackend` uses a `tokio::sync::Mutex` to serialise concurrent
//! read-modify-write cycles; this is adequate for a CLI that runs one
//! operation at a time but also safe if the same instance is shared.

use crate::{SecretStore, SecretValue, SecretsError};
use async_trait::async_trait;
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use hkdf::Hkdf;
use rand::RngCore;
use sha2::Sha256;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::Mutex;

/// Encrypted file-backed secret store.
///
/// Construct via [`FileBackend::default_path`] (uses `~/.sindri/secrets.enc`)
/// or via [`FileBackend::with_path_and_passphrase`] for tests.
#[derive(Clone)]
pub struct FileBackend {
    path: PathBuf,
    /// ChaCha20-Poly1305 cipher initialised from the derived key.
    cipher: ChaCha20Poly1305,
    /// Serialise all read-modify-write cycles.
    lock: Arc<Mutex<()>>,
}

impl std::fmt::Debug for FileBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileBackend")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl FileBackend {
    /// Construct with a specific file path and passphrase.  Suitable for
    /// both production use (passphrase from OS keyring / env) and tests.
    pub fn with_path_and_passphrase(path: impl Into<PathBuf>, passphrase: &str) -> Self {
        let path = path.into();
        let key = derive_key(passphrase, &path);
        let cipher = ChaCha20Poly1305::new_from_slice(&key)
            .expect("chacha20poly1305 key is always 32 bytes");
        Self {
            path,
            cipher,
            lock: Arc::new(Mutex::new(())),
        }
    }

    /// Construct using the default path `~/.sindri/secrets.enc` and a
    /// passphrase sourced from `SINDRI_SECRETS_PASSPHRASE` env var (falls
    /// back to an empty string, which is only safe in development / CI).
    pub fn default_path() -> Option<Self> {
        let path = sindri_core::paths::sindri_subpath(&["secrets.enc"])?;
        let passphrase = std::env::var("SINDRI_SECRETS_PASSPHRASE").unwrap_or_default();
        Some(Self::with_path_and_passphrase(path, &passphrase))
    }

    // ── private helpers ───────────────────────────────────────────────────

    fn load(&self) -> Result<HashMap<String, SecretValue>, SecretsError> {
        if !self.path.exists() {
            return Ok(HashMap::new());
        }
        let raw = std::fs::read(&self.path)?;
        if raw.len() < 12 {
            return Err(SecretsError::Crypto(
                "secrets file too short to contain a nonce".into(),
            ));
        }
        let (nonce_bytes, ciphertext) = raw.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| SecretsError::Crypto(format!("decryption failed: {}", e)))?;
        serde_json::from_slice(&plaintext)
            .map_err(|e| SecretsError::Serde(format!("cannot parse secrets store: {}", e)))
    }

    fn save(&self, map: &HashMap<String, SecretValue>) -> Result<(), SecretsError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let plaintext = serde_json::to_vec(map)
            .map_err(|e| SecretsError::Serde(format!("cannot serialise secrets store: {}", e)))?;
        let mut nonce_bytes = [0u8; 12];
        rand::rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext.as_slice())
            .map_err(|e| SecretsError::Crypto(format!("encryption failed: {}", e)))?;
        let mut blob = Vec::with_capacity(12 + ciphertext.len());
        blob.extend_from_slice(&nonce_bytes);
        blob.extend_from_slice(&ciphertext);
        // Atomic write via temp file + rename.
        let tmp = self.path.with_extension("enc.tmp");
        std::fs::write(&tmp, &blob)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

/// Derive a 32-byte ChaCha20-Poly1305 key from `passphrase` + `path`.
fn derive_key(passphrase: &str, path: &Path) -> [u8; 32] {
    use sha2::Digest;
    // salt = SHA-256("sindri-file-backend-salt" || path-bytes)
    let mut salt_input = b"sindri-file-backend-salt".to_vec();
    salt_input.extend_from_slice(path.to_string_lossy().as_bytes());
    let salt = Sha256::digest(&salt_input);

    let hkdf = Hkdf::<Sha256>::new(Some(&salt), passphrase.as_bytes());
    let mut okm = [0u8; 32];
    hkdf.expand(b"sindri-secrets-v1", &mut okm)
        .expect("HKDF expand with 32 bytes always succeeds for SHA-256");
    okm
}

#[async_trait]
impl SecretStore for FileBackend {
    async fn read(&self, name: &str) -> Result<SecretValue, SecretsError> {
        let _guard = self.lock.lock().await;
        let map = self.load()?;
        map.into_iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v)
            .ok_or_else(|| SecretsError::NotFound {
                name: name.to_string(),
            })
    }

    async fn write(&self, name: &str, value: SecretValue) -> Result<(), SecretsError> {
        let _guard = self.lock.lock().await;
        let mut map = self.load()?;
        map.insert(name.to_string(), value);
        self.save(&map)
    }

    async fn delete(&self, name: &str) -> Result<(), SecretsError> {
        let _guard = self.lock.lock().await;
        let mut map = self.load()?;
        if map.remove(name).is_none() {
            return Err(SecretsError::NotFound {
                name: name.to_string(),
            });
        }
        self.save(&map)
    }

    async fn list(&self) -> Result<Vec<String>, SecretsError> {
        let _guard = self.lock.lock().await;
        let map = self.load()?;
        let mut names: Vec<String> = map.into_keys().collect();
        names.sort();
        Ok(names)
    }
}

// ── tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn backend(dir: &tempfile::TempDir) -> FileBackend {
        FileBackend::with_path_and_passphrase(dir.path().join("secrets.enc"), "testpass")
    }

    #[tokio::test]
    async fn write_then_read_round_trip() {
        let dir = tempdir().unwrap();
        let store = backend(&dir);
        store
            .write("my.secret", SecretValue::from_plaintext("abc"))
            .await
            .unwrap();
        let sv = store.read("my.secret").await.unwrap();
        assert_eq!(sv.expose_str().unwrap(), "abc");
    }

    #[tokio::test]
    async fn read_missing_returns_not_found() {
        let dir = tempdir().unwrap();
        let store = backend(&dir);
        let result = store.read("nonexistent").await;
        assert!(matches!(result, Err(SecretsError::NotFound { .. })));
    }

    #[tokio::test]
    async fn delete_removes_secret() {
        let dir = tempdir().unwrap();
        let store = backend(&dir);
        store
            .write("tok", SecretValue::from_plaintext("hello"))
            .await
            .unwrap();
        store.delete("tok").await.unwrap();
        let result = store.read("tok").await;
        assert!(matches!(result, Err(SecretsError::NotFound { .. })));
    }

    #[tokio::test]
    async fn delete_missing_returns_not_found() {
        let dir = tempdir().unwrap();
        let store = backend(&dir);
        let result = store.delete("ghost").await;
        assert!(matches!(result, Err(SecretsError::NotFound { .. })));
    }

    #[tokio::test]
    async fn list_returns_sorted_names() {
        let dir = tempdir().unwrap();
        let store = backend(&dir);
        store
            .write("b-key", SecretValue::from_plaintext("1"))
            .await
            .unwrap();
        store
            .write("a-key", SecretValue::from_plaintext("2"))
            .await
            .unwrap();
        let names = store.list().await.unwrap();
        assert_eq!(names, vec!["a-key", "b-key"]);
    }

    #[tokio::test]
    async fn wrong_passphrase_fails_decryption() {
        let dir = tempdir().unwrap();
        let store_a = backend(&dir);
        store_a
            .write("secret", SecretValue::from_plaintext("value"))
            .await
            .unwrap();

        // A different passphrase should fail to decrypt.
        let store_b =
            FileBackend::with_path_and_passphrase(dir.path().join("secrets.enc"), "wrongpass");
        let result = store_b.read("secret").await;
        assert!(
            matches!(result, Err(SecretsError::Crypto(_))),
            "unexpected result: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn multiple_secrets_persist() {
        let dir = tempdir().unwrap();
        let store = backend(&dir);
        for i in 0..5u32 {
            store
                .write(
                    &format!("key-{}", i),
                    SecretValue::from_plaintext(&format!("val-{}", i)),
                )
                .await
                .unwrap();
        }
        let names = store.list().await.unwrap();
        assert_eq!(names.len(), 5);
    }

    #[tokio::test]
    async fn truncated_file_returns_crypto_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("secrets.enc");
        // Write fewer than 12 bytes (minimum nonce size).
        std::fs::write(&path, b"short").unwrap();
        let store = FileBackend::with_path_and_passphrase(&path, "pass");
        let result = store.read("anything").await;
        assert!(matches!(result, Err(SecretsError::Crypto(_))));
    }
}
