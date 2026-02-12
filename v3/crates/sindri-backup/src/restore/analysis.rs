//! Backup analysis

use anyhow::Result;
use camino::Utf8Path;
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use tar::Archive;
use tracing::info;

use super::compatibility::VersionCompatibility;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    pub version: String,
    pub backup_type: String,
    pub created_at: String,
    pub created_by: String,
    pub source: SourceInfo,
    pub profile: String,
    pub compression: String,
    pub checksum: ChecksumInfo,
    pub statistics: Statistics,
    pub extensions: Extensions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInfo {
    pub instance_name: String,
    pub provider: String,
    pub hostname: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecksumInfo {
    pub algorithm: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statistics {
    pub files_included: usize,
    pub total_size_bytes: u64,
    pub compressed_size_bytes: u64,
    pub compression_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extensions {
    pub installed: Vec<String>,
    pub versions: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct BackupAnalysis {
    pub manifest: BackupManifest,
    pub file_count: usize,
    pub total_size: u64,
    pub compatibility: VersionCompatibility,
}

pub struct BackupAnalyzer;

impl BackupAnalyzer {
    pub async fn analyze(&self, archive_path: &Utf8Path) -> Result<BackupAnalysis> {
        info!("Analyzing backup archive: {}", archive_path);

        let manifest = self.extract_manifest(archive_path)?;
        let (file_count, total_size) = self.count_files(archive_path)?;
        let compatibility = VersionCompatibility::check(&manifest.version)?;

        Ok(BackupAnalysis {
            manifest,
            file_count,
            total_size,
            compatibility,
        })
    }

    fn extract_manifest(&self, archive_path: &Utf8Path) -> Result<BackupManifest> {
        let file = File::open(archive_path.as_std_path())?;
        let decoder = GzDecoder::new(file);
        let mut archive = Archive::new(decoder);

        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?;

            if path.to_str() == Some(".backup-manifest.json")
                || path.ends_with(".backup-manifest.json")
            {
                let mut content = String::new();
                entry.read_to_string(&mut content)?;
                let manifest: BackupManifest = serde_json::from_str(&content)?;
                return Ok(manifest);
            }
        }

        anyhow::bail!("Backup manifest not found in archive");
    }

    fn count_files(&self, archive_path: &Utf8Path) -> Result<(usize, u64)> {
        let file = File::open(archive_path.as_std_path())?;
        let decoder = GzDecoder::new(file);
        let mut archive = Archive::new(decoder);

        let mut count = 0;
        let mut total_size = 0u64;

        for entry in archive.entries()? {
            let entry = entry?;
            count += 1;
            total_size += entry.size();
        }

        Ok((count, total_size))
    }

    pub fn validate_archive(&self, archive_path: &Utf8Path) -> Result<()> {
        let file = File::open(archive_path.as_std_path())?;
        let decoder = GzDecoder::new(file);
        let mut archive = Archive::new(decoder);

        for entry in archive.entries()? {
            let _entry = entry?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use tar::Builder;
    use tempfile::TempDir;

    /// Helper: create a valid tar.gz archive from a temp directory with files
    fn create_tar_gz_from_dir(source_dir: &std::path::Path, archive_path: &std::path::Path) {
        let file = File::create(archive_path).unwrap();
        let encoder = GzEncoder::new(file, Compression::default());
        let mut builder = Builder::new(encoder);
        builder.append_dir_all(".", source_dir).unwrap();
        builder.finish().unwrap();
    }

    #[test]
    fn test_validate_archive_not_gzip() {
        let temp_dir = TempDir::new().unwrap();
        let bad_file = temp_dir.path().join("not-gzip.tar.gz");
        std::fs::write(&bad_file, b"this is plain text, not gzip").unwrap();

        let utf8_path = Utf8PathBuf::from_path_buf(bad_file).expect("path should be valid UTF-8");
        let analyzer = BackupAnalyzer;
        let result = analyzer.validate_archive(&utf8_path);
        assert!(result.is_err(), "Should fail for non-gzip content");
    }

    #[test]
    fn test_validate_archive_nonexistent_file() {
        let path = Utf8Path::new("/tmp/nonexistent-archive-12345.tar.gz");
        let analyzer = BackupAnalyzer;
        let result = analyzer.validate_archive(path);
        assert!(result.is_err(), "Should fail for missing file");
    }

    #[test]
    fn test_extract_manifest_missing_from_archive() {
        let temp_dir = TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("no-manifest.tar.gz");

        // Create a source directory with a dummy file (no manifest)
        let source_dir = temp_dir.path().join("source");
        std::fs::create_dir_all(&source_dir).unwrap();
        std::fs::write(source_dir.join("dummy.txt"), b"hello").unwrap();

        create_tar_gz_from_dir(&source_dir, &archive_path);

        let utf8_path =
            Utf8PathBuf::from_path_buf(archive_path).expect("path should be valid UTF-8");
        let analyzer = BackupAnalyzer;
        let result = analyzer.extract_manifest(&utf8_path);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("manifest not found"),
            "Expected 'manifest not found' in error, got: {}",
            err
        );
    }

    #[test]
    fn test_extract_manifest_corrupt_json() {
        let temp_dir = TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("corrupt-manifest.tar.gz");

        // Create a source directory with a manifest file containing invalid JSON
        let source_dir = temp_dir.path().join("source");
        std::fs::create_dir_all(&source_dir).unwrap();
        std::fs::write(
            source_dir.join(".backup-manifest.json"),
            b"{ this is not valid json }",
        )
        .unwrap();

        create_tar_gz_from_dir(&source_dir, &archive_path);

        let utf8_path =
            Utf8PathBuf::from_path_buf(archive_path).expect("path should be valid UTF-8");
        let analyzer = BackupAnalyzer;
        let result = analyzer.extract_manifest(&utf8_path);
        assert!(result.is_err(), "Should fail on corrupt manifest JSON");
    }

    #[tokio::test]
    async fn test_analyze_nonexistent_file() {
        let path = Utf8Path::new("/tmp/nonexistent-backup-12345.tar.gz");
        let analyzer = BackupAnalyzer;
        let result = analyzer.analyze(path).await;
        assert!(result.is_err(), "Should fail for missing file");
    }

    #[test]
    fn test_validate_archive_succeeds_with_valid_tar_gz() {
        let temp_dir = TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("valid.tar.gz");

        let source_dir = temp_dir.path().join("source");
        std::fs::create_dir_all(&source_dir).unwrap();
        std::fs::write(source_dir.join("file1.txt"), b"content1").unwrap();
        std::fs::write(source_dir.join("file2.txt"), b"content2").unwrap();

        create_tar_gz_from_dir(&source_dir, &archive_path);

        let utf8_path =
            Utf8PathBuf::from_path_buf(archive_path).expect("path should be valid UTF-8");
        let analyzer = BackupAnalyzer;
        let result = analyzer.validate_archive(&utf8_path);
        assert!(
            result.is_ok(),
            "Valid tar.gz should pass validation, got: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_count_files_returns_correct_count() {
        let temp_dir = TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("counted.tar.gz");

        let source_dir = temp_dir.path().join("source");
        std::fs::create_dir_all(&source_dir).unwrap();
        std::fs::write(source_dir.join("a.txt"), b"aaa").unwrap();
        std::fs::write(source_dir.join("b.txt"), b"bbbbb").unwrap();
        std::fs::write(source_dir.join("c.txt"), b"c").unwrap();

        create_tar_gz_from_dir(&source_dir, &archive_path);

        let utf8_path =
            Utf8PathBuf::from_path_buf(archive_path).expect("path should be valid UTF-8");
        let analyzer = BackupAnalyzer;
        let (count, total_size) = analyzer.count_files(&utf8_path).unwrap();

        // tar includes the directory entry "." plus 3 files = at least 4 entries
        assert!(
            count >= 3,
            "Expected at least 3 entries (3 files), got {}",
            count
        );
        assert!(
            total_size >= 9,
            "Expected total size >= 9 bytes (3+5+1), got {}",
            total_size
        );
    }
}
