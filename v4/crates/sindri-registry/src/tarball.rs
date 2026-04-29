//! Safe OCI layer extraction (Wave 5A — D6).
//!
//! Implements the deferred tar/tar+gzip layer-extraction support that
//! [`crate::client::RegistryClient`] previously rejected with
//! [`crate::error::RegistryError::UnsupportedMediaType`]. Two layer media
//! types are accepted:
//!
//! - `application/vnd.oci.image.layer.v1.tar+gzip` — gzip stream over tar
//! - `application/vnd.oci.image.layer.v1.tar`      — uncompressed tar
//!
//! ## Security model
//!
//! Layer extraction is a classic source of path-traversal CVEs. Every entry
//! that is extracted must satisfy ALL of the following:
//!
//! 1. The entry path is **relative** (no leading `/`, no Windows drive prefix).
//! 2. The entry path contains **no `..` components** at any nesting level.
//! 3. After joining with the destination root, the canonical path is still
//!    contained within the destination root.
//!
//! Any entry that fails these checks aborts the entire extraction with
//! [`TarballError::UnsafePath`] — fail closed, leaving partial output behind
//! is acceptable for an aborted download but the operation as a whole is
//! reported as failed to the caller.
//!
//! ## Digest verification
//!
//! Per ADR-003 §"content addressing", every byte of the layer that goes into
//! the extractor is also fed into a streaming SHA-256 hasher. The final
//! digest is compared against the descriptor digest the caller obtained from
//! the OCI manifest. A mismatch raises [`TarballError::DigestMismatch`]; the
//! extracted output (if any) MUST be treated as untrusted and discarded by
//! the caller. The compute-and-compare is O(n) over the layer bytes and adds
//! no extra I/O round trips.

use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

/// Errors raised by [`extract_layer`].
#[derive(Debug, thiserror::Error)]
pub enum TarballError {
    #[error("tar I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("tar entry path '{path}' is not safe: {reason}")]
    UnsafePath { path: String, reason: String },
    #[error("layer digest mismatch — expected {expected}, computed sha256:{actual}")]
    DigestMismatch { expected: String, actual: String },
    #[error("malformed layer descriptor digest '{0}' (expected sha256:<hex>)")]
    BadDescriptorDigest(String),
}

/// Compression of an OCI layer blob.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerCompression {
    /// `application/vnd.oci.image.layer.v1.tar`
    None,
    /// `application/vnd.oci.image.layer.v1.tar+gzip`
    Gzip,
}

impl LayerCompression {
    /// Resolve a layer media-type to a [`LayerCompression`]. Returns `None`
    /// for media types this module cannot handle.
    pub fn from_media_type(media_type: &str) -> Option<Self> {
        match media_type {
            "application/vnd.oci.image.layer.v1.tar+gzip" => Some(LayerCompression::Gzip),
            "application/vnd.oci.image.layer.v1.tar" => Some(LayerCompression::None),
            _ => None,
        }
    }
}

/// Streaming reader that hashes every byte it reads into a SHA-256 digest.
///
/// Wraps an arbitrary [`Read`] — used to fold "verify the layer digest" into
/// the same pass that feeds the gzip/tar decoder, so we never have to buffer
/// the entire layer in memory.
struct HashingReader<R: Read> {
    inner: R,
    hasher: Sha256,
}

impl<R: Read> HashingReader<R> {
    fn new(inner: R) -> Self {
        HashingReader {
            inner,
            hasher: Sha256::new(),
        }
    }

    fn finish(self) -> [u8; 32] {
        self.hasher.finalize().into()
    }
}

impl<R: Read> Read for HashingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        if n > 0 {
            self.hasher.update(&buf[..n]);
        }
        Ok(n)
    }
}

/// Validate that `entry_path` is safe to join with `dest_root`.
///
/// Pure function; no filesystem access. Exposed so unit tests can exercise
/// the traversal-detection rules without spinning up a real tarball.
pub fn validate_entry_path(entry_path: &Path) -> Result<PathBuf, TarballError> {
    if entry_path.has_root() {
        return Err(TarballError::UnsafePath {
            path: entry_path.display().to_string(),
            reason: "absolute paths are not allowed in tar entries".into(),
        });
    }
    let mut clean = PathBuf::new();
    for comp in entry_path.components() {
        match comp {
            Component::Normal(part) => clean.push(part),
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(TarballError::UnsafePath {
                    path: entry_path.display().to_string(),
                    reason: "parent-directory components ('..') are not allowed".into(),
                });
            }
            Component::Prefix(_) | Component::RootDir => {
                return Err(TarballError::UnsafePath {
                    path: entry_path.display().to_string(),
                    reason: "drive-letter or root-directory components are not allowed".into(),
                });
            }
        }
    }
    Ok(clean)
}

/// Extract an OCI layer blob into `dest_root`.
///
/// `expected_digest` MUST be the descriptor digest as `sha256:<hex>` — the
/// extractor recomputes the SHA-256 over the wire bytes and aborts with
/// [`TarballError::DigestMismatch`] if they disagree.
///
/// Path-traversal protection (see module docs) is non-negotiable.
///
/// On success, returns the list of extracted relative paths (useful for
/// telemetry, BOM, and tests).
pub fn extract_layer(
    blob: &[u8],
    compression: LayerCompression,
    expected_digest: &str,
    dest_root: &Path,
) -> Result<Vec<PathBuf>, TarballError> {
    let (alg, expected_hex) = expected_digest
        .split_once(':')
        .ok_or_else(|| TarballError::BadDescriptorDigest(expected_digest.to_string()))?;
    if alg != "sha256" || expected_hex.is_empty() {
        return Err(TarballError::BadDescriptorDigest(
            expected_digest.to_string(),
        ));
    }

    std::fs::create_dir_all(dest_root)?;

    // Drive a single streaming pass:
    //   raw bytes → HashingReader → (optional GzDecoder) → tar::Archive
    let cursor = std::io::Cursor::new(blob);
    let hashing = HashingReader::new(cursor);

    // We need both the tar reader (for entries) AND the hasher (after tar
    // is fully consumed, to compare digests). We branch on compression to
    // keep the type tractable for the borrow checker — both arms share the
    // same body via a closure.
    let extracted = match compression {
        LayerCompression::Gzip => {
            // GzDecoder wraps the hashing reader so the hash covers the
            // *compressed* layer bytes (which is what the OCI descriptor
            // digest pins, per the OCI spec).
            let gz = GzDecoder::new(hashing);
            let mut archive = tar::Archive::new(gz);
            let extracted = extract_archive_entries(&mut archive, dest_root)?;
            // Recover the gzip stream from the archive, then drain any
            // remaining gzip-trailer + tar-trailer bytes so the hasher
            // observes the full layer payload.
            let mut gz = archive.into_inner();
            std::io::copy(&mut gz, &mut std::io::sink())?;
            let hashing = gz.into_inner();
            verify_digest(hashing.finish(), expected_hex)?;
            extracted
        }
        LayerCompression::None => {
            let mut archive = tar::Archive::new(hashing);
            let extracted = extract_archive_entries(&mut archive, dest_root)?;
            // Drain trailing zero blocks so the streaming SHA-256 covers
            // every byte of the blob, not just the entry headers/bodies.
            let mut hashing = archive.into_inner();
            std::io::copy(&mut hashing, &mut std::io::sink())?;
            verify_digest(hashing.finish(), expected_hex)?;
            extracted
        }
    };

    Ok(extracted)
}

fn verify_digest(actual: [u8; 32], expected_hex: &str) -> Result<(), TarballError> {
    let actual_hex = hex::encode(actual);
    if !actual_hex.eq_ignore_ascii_case(expected_hex) {
        return Err(TarballError::DigestMismatch {
            expected: format!("sha256:{}", expected_hex),
            actual: actual_hex,
        });
    }
    Ok(())
}

fn extract_archive_entries<R: Read>(
    archive: &mut tar::Archive<R>,
    dest_root: &Path,
) -> Result<Vec<PathBuf>, TarballError> {
    let mut extracted = Vec::new();
    // Refuse symlinks/hardlinks — they are another path-traversal vector
    // and registry layer payloads have no legitimate need for them.
    archive.set_overwrite(true);
    archive.set_preserve_permissions(false);

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        let safe_rel = validate_entry_path(&path)?;
        let entry_type = entry.header().entry_type();
        if entry_type.is_symlink() || entry_type.is_hard_link() {
            return Err(TarballError::UnsafePath {
                path: path.display().to_string(),
                reason: "symlinks and hard links are not allowed in registry layers".into(),
            });
        }

        let target = dest_root.join(&safe_rel);
        // Defence in depth: even though `validate_entry_path` rejects
        // traversal, re-check that the join landed inside `dest_root`.
        // This catches edge cases like a tar entry path that resolves
        // unexpectedly on Windows.
        if !target.starts_with(dest_root) {
            return Err(TarballError::UnsafePath {
                path: path.display().to_string(),
                reason: "resolved path escaped the destination root".into(),
            });
        }

        if entry_type.is_dir() {
            std::fs::create_dir_all(&target)?;
        } else if entry_type.is_file() {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out = std::fs::File::create(&target)?;
            std::io::copy(&mut entry, &mut out)?;
        } else {
            // Unknown entry type — skip silently rather than fail. Standard
            // metadata entries (PAX headers, GNU LongLink) reach this arm
            // and are handled by `tar::Archive::entries` internally.
            continue;
        }
        extracted.push(safe_rel);
    }
    Ok(extracted)
}

/// Extract a single entry from a layer blob into memory.
///
/// Used by callers that only need one logical file out of a tar payload
/// (e.g. `index.yaml` from a registry artifact). Performs the same
/// path-traversal guards and digest verification as [`extract_layer`] but
/// never touches the filesystem.
///
/// Returns the entry's bytes if found, or `Ok(None)` if the layer was
/// well-formed and digest-correct but did not contain `wanted`.
pub fn read_entry_from_layer(
    blob: &[u8],
    compression: LayerCompression,
    expected_digest: &str,
    wanted: &Path,
) -> Result<Option<Vec<u8>>, TarballError> {
    let (alg, expected_hex) = expected_digest
        .split_once(':')
        .ok_or_else(|| TarballError::BadDescriptorDigest(expected_digest.to_string()))?;
    if alg != "sha256" || expected_hex.is_empty() {
        return Err(TarballError::BadDescriptorDigest(
            expected_digest.to_string(),
        ));
    }
    let wanted = validate_entry_path(wanted)?;

    let cursor = std::io::Cursor::new(blob);
    let hashing = HashingReader::new(cursor);

    let result = match compression {
        LayerCompression::Gzip => {
            let gz = GzDecoder::new(hashing);
            let mut archive = tar::Archive::new(gz);
            let entry_bytes = scan_for_entry(&mut archive, &wanted)?;
            let mut gz = archive.into_inner();
            std::io::copy(&mut gz, &mut std::io::sink())?;
            let hashing = gz.into_inner();
            verify_digest(hashing.finish(), expected_hex)?;
            entry_bytes
        }
        LayerCompression::None => {
            let mut archive = tar::Archive::new(hashing);
            let entry_bytes = scan_for_entry(&mut archive, &wanted)?;
            let mut hashing = archive.into_inner();
            std::io::copy(&mut hashing, &mut std::io::sink())?;
            verify_digest(hashing.finish(), expected_hex)?;
            entry_bytes
        }
    };
    Ok(result)
}

fn scan_for_entry<R: Read>(
    archive: &mut tar::Archive<R>,
    wanted: &Path,
) -> Result<Option<Vec<u8>>, TarballError> {
    let mut found: Option<Vec<u8>> = None;
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        let safe_rel = validate_entry_path(&path)?;
        let entry_type = entry.header().entry_type();
        if entry_type.is_symlink() || entry_type.is_hard_link() {
            return Err(TarballError::UnsafePath {
                path: path.display().to_string(),
                reason: "symlinks and hard links are not allowed in registry layers".into(),
            });
        }
        if entry_type.is_file() && safe_rel == wanted && found.is_none() {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            found = Some(buf);
            // Continue draining subsequent entries so the streaming hash
            // sees the entire layer payload (digest-verify integrity).
            continue;
        }
        // Drain the entry body so the underlying reader (and our hasher)
        // observes every byte of the layer.
        std::io::copy(&mut entry, &mut std::io::sink())?;
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;
    use tempfile::TempDir;

    fn make_tar(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::new();
        {
            let mut builder = tar::Builder::new(&mut buf);
            for (name, body) in entries {
                let mut header = tar::Header::new_gnu();
                header.set_size(body.len() as u64);
                header.set_mode(0o644);
                header.set_cksum();
                builder.append_data(&mut header, name, *body).unwrap();
            }
            builder.finish().unwrap();
        }
        buf
    }

    fn gzip(bytes: &[u8]) -> Vec<u8> {
        let mut enc = GzEncoder::new(Vec::new(), Compression::default());
        enc.write_all(bytes).unwrap();
        enc.finish().unwrap()
    }

    fn sha256(bytes: &[u8]) -> String {
        let mut h = Sha256::new();
        h.update(bytes);
        format!("sha256:{}", hex::encode(h.finalize()))
    }

    #[test]
    fn validate_entry_path_accepts_relative() {
        assert_eq!(
            validate_entry_path(Path::new("a/b/c.yaml")).unwrap(),
            PathBuf::from("a/b/c.yaml")
        );
    }

    #[test]
    fn validate_entry_path_rejects_absolute() {
        let err = validate_entry_path(Path::new("/etc/passwd")).unwrap_err();
        assert!(matches!(err, TarballError::UnsafePath { .. }));
    }

    #[test]
    fn validate_entry_path_rejects_dotdot() {
        let err = validate_entry_path(Path::new("a/../../etc/passwd")).unwrap_err();
        assert!(matches!(err, TarballError::UnsafePath { ref reason, .. }
            if reason.contains("parent-directory")));
    }

    #[test]
    fn validate_entry_path_strips_curdir() {
        assert_eq!(
            validate_entry_path(Path::new("./a/./b")).unwrap(),
            PathBuf::from("a/b")
        );
    }

    #[test]
    fn extract_uncompressed_tar_succeeds() {
        let tmp = TempDir::new().unwrap();
        let blob = make_tar(&[
            ("index.yaml", b"version: 1\n" as &[u8]),
            ("nested/file.txt", b"hello"),
        ]);
        let digest = sha256(&blob);
        let extracted = extract_layer(&blob, LayerCompression::None, &digest, tmp.path()).unwrap();
        assert_eq!(extracted.len(), 2);
        let read_index = std::fs::read(tmp.path().join("index.yaml")).unwrap();
        assert_eq!(read_index, b"version: 1\n");
        let read_nested = std::fs::read(tmp.path().join("nested/file.txt")).unwrap();
        assert_eq!(read_nested, b"hello");
    }

    #[test]
    fn extract_gzipped_tar_succeeds() {
        let tmp = TempDir::new().unwrap();
        let tar_bytes = make_tar(&[("only.yaml", b"k: v\n" as &[u8])]);
        let blob = gzip(&tar_bytes);
        let digest = sha256(&blob);
        let extracted = extract_layer(&blob, LayerCompression::Gzip, &digest, tmp.path()).unwrap();
        assert_eq!(extracted, vec![PathBuf::from("only.yaml")]);
        let body = std::fs::read(tmp.path().join("only.yaml")).unwrap();
        assert_eq!(body, b"k: v\n");
    }

    #[test]
    fn extract_aborts_on_digest_mismatch() {
        let tmp = TempDir::new().unwrap();
        let blob = make_tar(&[("a.txt", b"a" as &[u8])]);
        let bogus = format!("sha256:{}", "f".repeat(64));
        let err = extract_layer(&blob, LayerCompression::None, &bogus, tmp.path()).unwrap_err();
        assert!(matches!(err, TarballError::DigestMismatch { .. }));
    }

    #[test]
    fn extract_aborts_on_dotdot_entry() {
        // Hand-craft a 512-byte tar header whose name field is `../escape`.
        // tar::Builder rejects unsafe names on the producer side, so we
        // emit the header bytes directly to exercise the consumer-side
        // guard — the realistic threat is a malicious *publisher*.
        let mut header_bytes = [0u8; 512];
        // Name field (offset 0..100). Writing as bytes — null-terminated.
        let name = b"../escape\0";
        header_bytes[..name.len()].copy_from_slice(name);
        // Mode (offset 100, 8 bytes), uid/gid (108, 116; 8 bytes each),
        // size (124, 12 bytes octal).
        let body = b"oops";
        let size_octal = format!("{:011o}\0", body.len());
        header_bytes[100..108].copy_from_slice(b"0000644\0");
        header_bytes[108..116].copy_from_slice(b"0000000\0");
        header_bytes[116..124].copy_from_slice(b"0000000\0");
        header_bytes[124..136].copy_from_slice(size_octal.as_bytes());
        // Mtime (136, 12 bytes octal). Zeros are fine.
        header_bytes[136..148].copy_from_slice(b"00000000000\0");
        // Type flag (offset 156): '0' = regular file.
        header_bytes[156] = b'0';
        // Magic (257..263) + version (263..265): "ustar\0" + "00"
        header_bytes[257..263].copy_from_slice(b"ustar\0");
        header_bytes[263..265].copy_from_slice(b"00");
        // Compute checksum (offset 148, 8 bytes). Field starts as spaces
        // when checksumming.
        for b in &mut header_bytes[148..156] {
            *b = b' ';
        }
        let cksum: u32 = header_bytes.iter().map(|b| *b as u32).sum();
        let cksum_str = format!("{:06o}\0 ", cksum);
        header_bytes[148..156].copy_from_slice(cksum_str.as_bytes());

        let mut buf: Vec<u8> = Vec::new();
        buf.extend_from_slice(&header_bytes);
        // Body, padded to 512.
        buf.extend_from_slice(body);
        buf.extend(std::iter::repeat_n(0u8, 512 - body.len()));
        // Two 512-byte zero blocks = end-of-archive marker.
        buf.extend(std::iter::repeat_n(0u8, 1024));

        let digest = sha256(&buf);
        let tmp = TempDir::new().unwrap();
        let err = extract_layer(&buf, LayerCompression::None, &digest, tmp.path()).unwrap_err();
        assert!(
            matches!(err, TarballError::UnsafePath { ref reason, .. }
                if reason.contains("parent-directory")),
            "expected UnsafePath, got {:?}",
            err
        );
    }

    #[test]
    fn extract_rejects_symlink_entries() {
        let mut buf: Vec<u8> = Vec::new();
        {
            let mut builder = tar::Builder::new(&mut buf);
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::Symlink);
            header.set_size(0);
            header.set_mode(0o777);
            header
                .set_link_name("/etc/passwd")
                .expect("set link target");
            header.set_cksum();
            builder
                .append_data(&mut header, "link", std::io::empty())
                .unwrap();
            builder.finish().unwrap();
        }
        let digest = sha256(&buf);
        let tmp = TempDir::new().unwrap();
        let err = extract_layer(&buf, LayerCompression::None, &digest, tmp.path()).unwrap_err();
        assert!(matches!(err, TarballError::UnsafePath { ref reason, .. }
            if reason.contains("symlink")));
    }

    #[test]
    fn from_media_type_recognises_both_layer_types() {
        assert_eq!(
            LayerCompression::from_media_type("application/vnd.oci.image.layer.v1.tar+gzip"),
            Some(LayerCompression::Gzip)
        );
        assert_eq!(
            LayerCompression::from_media_type("application/vnd.oci.image.layer.v1.tar"),
            Some(LayerCompression::None)
        );
        assert_eq!(LayerCompression::from_media_type("text/plain"), None);
    }

    #[test]
    fn extract_rejects_bad_descriptor_digest() {
        let tmp = TempDir::new().unwrap();
        let blob = make_tar(&[("a", b"a" as &[u8])]);
        let err =
            extract_layer(&blob, LayerCompression::None, "not-a-digest", tmp.path()).unwrap_err();
        assert!(matches!(err, TarballError::BadDescriptorDigest(_)));
        let err2 =
            extract_layer(&blob, LayerCompression::None, "md5:abcdef", tmp.path()).unwrap_err();
        assert!(matches!(err2, TarballError::BadDescriptorDigest(_)));
    }
}
