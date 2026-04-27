//! Content-addressed registry cache (ADR-003 §"Registry artifact structure").
//!
//! Layout under `~/.sindri/cache/registries/`:
//!
//! ```text
//! by-digest/sha256/<aa>/<bbcc…>/manifest.json
//!                              index.yaml
//!                              signature.bundle
//! refs/<registry-name>/<encoded-oci-ref>   (file containing the digest)
//! ```
//!
//! - The first 2 hex chars of the digest are sharded as a subdirectory so
//!   directories don't blow up on registries with thousands of components.
//! - `<encoded-oci-ref>` replaces `:` and `/` with `_` to keep the filename
//!   filesystem-safe on Windows.
//!
//! In Wave 3A.1 only the digest+ref API is exercised by tests; the legacy
//! `get_index`/`put_index` API remains as a thin wrapper so existing callers
//! (`RegistryClient::fetch_index`) keep working until Wave 3A.2 swaps them
//! over to the digest path.

use crate::error::RegistryError;
use crate::oci_ref::OciRef;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Kind of blob stored under a digest.
///
/// Each kind maps to a fixed filename inside the digest directory so that
/// manifests, indices, and signatures are isolated from each other even when
/// they happen to share a content digest (e.g. an empty manifest).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlobKind {
    /// OCI image / artifact manifest JSON.
    Manifest,
    /// `index.yaml` payload (registry catalog).
    Index,
    /// cosign signature bundle.
    Signature,
}

impl BlobKind {
    fn filename(self) -> &'static str {
        match self {
            BlobKind::Manifest => "manifest.json",
            BlobKind::Index => "index.yaml",
            BlobKind::Signature => "signature.bundle",
        }
    }
}

/// Content-addressed blob cache at `~/.sindri/cache/registries/` (ADR-003).
pub struct RegistryCache {
    root: PathBuf,
}

impl RegistryCache {
    /// Open the default user cache (`~/.sindri/cache/registries/`).
    pub fn new() -> Result<Self, RegistryError> {
        let home = dirs_next::home_dir()
            .ok_or_else(|| RegistryError::CacheError("Cannot determine home directory".into()))?;
        let root = home.join(".sindri").join("cache").join("registries");
        fs::create_dir_all(&root)?;
        Ok(RegistryCache { root })
    }

    /// Open a cache rooted at an explicit path (test harnesses, alt-roots).
    pub fn with_path(root: PathBuf) -> Result<Self, RegistryError> {
        fs::create_dir_all(&root)?;
        Ok(RegistryCache { root })
    }

    /// Store `content` under its digest. Returns the absolute path written.
    ///
    /// Writes are atomic: a `.tmp` sibling is renamed into place. The same
    /// digest can be written multiple times — the second write is a no-op
    /// from the caller's perspective.
    pub fn put_by_digest(
        &self,
        digest: &str,
        kind: BlobKind,
        content: &[u8],
    ) -> Result<PathBuf, RegistryError> {
        let dir = self.digest_dir(digest)?;
        fs::create_dir_all(&dir)?;
        let target = dir.join(kind.filename());
        let tmp = target.with_extension("tmp");
        fs::write(&tmp, content)?;
        fs::rename(&tmp, &target)?;
        Ok(target)
    }

    /// Fetch a previously-stored blob by digest. `None` if absent.
    pub fn get_by_digest(&self, digest: &str, kind: BlobKind) -> Option<Vec<u8>> {
        let dir = self.digest_dir(digest).ok()?;
        let path = dir.join(kind.filename());
        fs::read(&path).ok()
    }

    /// Link an OCI reference to a digest under the given registry name.
    ///
    /// The link is a small file containing the digest string; we deliberately
    /// avoid filesystem symlinks for Windows portability.
    pub fn link_ref(
        &self,
        registry_name: &str,
        oci_ref: &OciRef,
        digest: &str,
    ) -> Result<(), RegistryError> {
        let path = self.ref_path(registry_name, oci_ref);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, digest.as_bytes())?;
        fs::rename(&tmp, &path)?;
        Ok(())
    }

    /// Resolve an OCI reference for the given registry to its previously
    /// linked digest, if any.
    pub fn lookup_ref(&self, registry_name: &str, oci_ref: &OciRef) -> Option<String> {
        let path = self.ref_path(registry_name, oci_ref);
        fs::read_to_string(&path)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    /// Returns the cached `index.yaml` content for `registry_name` if present
    /// and within `ttl`. Legacy API retained so the existing
    /// [`crate::client::RegistryClient::fetch_index`] HTTP shim keeps working
    /// until Wave 3A.2 routes it through the digest cache.
    pub fn get_index(&self, registry_name: &str, ttl: Duration) -> Option<String> {
        let path = self.legacy_index_path(registry_name);
        if !path.exists() {
            return None;
        }
        let meta = fs::metadata(&path).ok()?;
        let modified = meta.modified().ok()?;
        if SystemTime::now().duration_since(modified).ok()? > ttl {
            return None;
        }
        fs::read_to_string(&path).ok()
    }

    /// Legacy companion to [`Self::get_index`]; same Wave-3A.2 deprecation
    /// notes apply.
    pub fn put_index(&self, registry_name: &str, content: &str) -> Result<(), RegistryError> {
        let path = self.legacy_index_path(registry_name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, content)?;
        fs::rename(&tmp, &path)?;
        Ok(())
    }

    /// Root of the cache (mostly useful in tests).
    pub fn cache_root(&self) -> &Path {
        &self.root
    }

    // -- internals -----------------------------------------------------------

    fn digest_dir(&self, digest: &str) -> Result<PathBuf, RegistryError> {
        let (alg, hex) = digest.split_once(':').ok_or_else(|| {
            RegistryError::CacheError(format!(
                "digest '{}' missing ':' separator (expected '<alg>:<hex>')",
                digest
            ))
        })?;
        if alg.is_empty() || hex.len() < 4 {
            return Err(RegistryError::CacheError(format!(
                "digest '{}' is too short to shard",
                digest
            )));
        }
        let shard = &hex[..2];
        let rest = &hex[2..];
        Ok(self.root.join("by-digest").join(alg).join(shard).join(rest))
    }

    fn ref_path(&self, registry_name: &str, oci_ref: &OciRef) -> PathBuf {
        let encoded = encode_ref(&oci_ref.to_canonical());
        self.root.join("refs").join(registry_name).join(encoded)
    }

    fn legacy_index_path(&self, registry_name: &str) -> PathBuf {
        self.root.join(registry_name).join("index.yaml")
    }
}

/// Encode an OCI reference into a filename-safe form (`:` and `/` → `_`).
fn encode_ref(canonical: &str) -> String {
    canonical
        .chars()
        .map(|c| match c {
            '/' | ':' | '@' => '_',
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oci_ref::OciRef;
    use tempfile::TempDir;

    fn fixture_digest(b: u8) -> String {
        format!(
            "sha256:{}",
            std::iter::repeat_n(b, 32).fold(String::new(), |mut a, n| {
                a.push_str(&format!("{:02x}", n));
                a
            })
        )
    }

    #[test]
    fn put_and_get_by_digest_round_trip() {
        let tmp = TempDir::new().unwrap();
        let cache = RegistryCache::with_path(tmp.path().to_path_buf()).unwrap();
        let digest = fixture_digest(0xab);
        let content = b"manifest body";
        let path = cache
            .put_by_digest(&digest, BlobKind::Manifest, content)
            .unwrap();
        assert!(path.ends_with("manifest.json"));
        // Sharded under by-digest/sha256/ab/<rest>/manifest.json.
        // Verify via Path::components so the assertion is correct on Windows
        // (which uses '\\' as the path separator) as well as POSIX.
        let segments: Vec<String> = path
            .components()
            .filter_map(|c| match c {
                std::path::Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
                _ => None,
            })
            .collect();
        let by_digest_idx = segments
            .iter()
            .position(|s| s == "by-digest")
            .expect("path should contain a 'by-digest' segment");
        assert_eq!(
            segments.get(by_digest_idx + 1).map(String::as_str),
            Some("sha256")
        );
        assert_eq!(
            segments.get(by_digest_idx + 2).map(String::as_str),
            Some("ab")
        );
        let read = cache.get_by_digest(&digest, BlobKind::Manifest).unwrap();
        assert_eq!(read, content);
    }

    #[test]
    fn link_ref_and_lookup_ref() {
        let tmp = TempDir::new().unwrap();
        let cache = RegistryCache::with_path(tmp.path().to_path_buf()).unwrap();
        let oci = OciRef::parse("ghcr.io/sindri-dev/registry-core:2026.04").unwrap();
        let digest = fixture_digest(0x12);
        cache.link_ref("sindri/core", &oci, &digest).unwrap();
        assert_eq!(cache.lookup_ref("sindri/core", &oci), Some(digest));
        // Different registry name → distinct namespace, no leak.
        assert_eq!(cache.lookup_ref("other", &oci), None);
    }

    #[test]
    fn blob_kinds_are_isolated() {
        let tmp = TempDir::new().unwrap();
        let cache = RegistryCache::with_path(tmp.path().to_path_buf()).unwrap();
        let digest = fixture_digest(0x77);
        cache
            .put_by_digest(&digest, BlobKind::Manifest, b"manifest")
            .unwrap();
        cache
            .put_by_digest(&digest, BlobKind::Signature, b"signature")
            .unwrap();
        assert_eq!(
            cache.get_by_digest(&digest, BlobKind::Manifest).unwrap(),
            b"manifest"
        );
        assert_eq!(
            cache.get_by_digest(&digest, BlobKind::Signature).unwrap(),
            b"signature"
        );
        assert!(cache.get_by_digest(&digest, BlobKind::Index).is_none());
    }

    #[test]
    fn missing_entry_returns_none() {
        let tmp = TempDir::new().unwrap();
        let cache = RegistryCache::with_path(tmp.path().to_path_buf()).unwrap();
        let digest = fixture_digest(0x01);
        assert!(cache.get_by_digest(&digest, BlobKind::Manifest).is_none());
        let oci = OciRef::parse("ghcr.io/foo/bar:1.0").unwrap();
        assert!(cache.lookup_ref("nope", &oci).is_none());
    }

    #[test]
    fn persists_across_cache_reopen() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().to_path_buf();
        let digest = fixture_digest(0x42);
        let oci = OciRef::parse("ghcr.io/foo/bar:1.0").unwrap();
        {
            let cache = RegistryCache::with_path(path.clone()).unwrap();
            cache
                .put_by_digest(&digest, BlobKind::Index, b"yaml: content")
                .unwrap();
            cache.link_ref("acme", &oci, &digest).unwrap();
        }
        let cache = RegistryCache::with_path(path).unwrap();
        assert_eq!(
            cache.get_by_digest(&digest, BlobKind::Index).unwrap(),
            b"yaml: content"
        );
        assert_eq!(cache.lookup_ref("acme", &oci), Some(digest));
    }

    #[test]
    fn rejects_malformed_digest() {
        let tmp = TempDir::new().unwrap();
        let cache = RegistryCache::with_path(tmp.path().to_path_buf()).unwrap();
        let err = cache
            .put_by_digest("not-a-digest", BlobKind::Manifest, b"x")
            .unwrap_err();
        assert!(matches!(err, RegistryError::CacheError(_)));
    }
}
