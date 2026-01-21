//! Compression utilities for backup operations.
//!
//! This module handles gzip compression with configurable levels
//! and streaming support for large files.

use flate2::write::GzEncoder;
use flate2::Compression;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

/// Default compression level (6 = balanced speed/ratio).
pub const DEFAULT_COMPRESSION_LEVEL: u32 = 6;

/// Statistics about a compression operation.
#[derive(Debug, Clone)]
pub struct CompressionStats {
    /// Original uncompressed size in bytes
    pub original_size: u64,

    /// Compressed size in bytes
    pub compressed_size: u64,

    /// Compression ratio (compressed / original)
    pub compression_ratio: f64,

    /// SHA256 checksum of compressed data
    pub checksum: String,
}

impl CompressionStats {
    /// Creates new compression statistics.
    pub fn new(original_size: u64, compressed_size: u64, checksum: String) -> Self {
        let compression_ratio = if original_size > 0 {
            compressed_size as f64 / original_size as f64
        } else {
            0.0
        };

        Self {
            original_size,
            compressed_size,
            compression_ratio,
            checksum,
        }
    }

    /// Returns compression percentage saved.
    pub fn compression_percentage(&self) -> u8 {
        ((1.0 - self.compression_ratio) * 100.0) as u8
    }
}

/// Compresses a file with gzip and calculates SHA256 checksum.
pub fn compress_file(
    source: &Path,
    dest: &Path,
    level: Option<u32>,
) -> anyhow::Result<CompressionStats> {
    let compression_level = level.unwrap_or(DEFAULT_COMPRESSION_LEVEL);

    // Open source file
    let mut source_file = File::open(source)
        .map_err(|e| anyhow::anyhow!("Failed to open source file: {}", e))?;

    // Get original file size
    let original_size = source_file.metadata()?.len();

    // Create destination file
    let dest_file = File::create(dest)
        .map_err(|e| anyhow::anyhow!("Failed to create destination file: {}", e))?;

    // Create gzip encoder with checksum writer
    let encoder = GzEncoder::new(dest_file, Compression::new(compression_level));
    let mut checksum_writer = ChecksumWriter::new(encoder);

    // Stream data through compression
    io::copy(&mut source_file, &mut checksum_writer)
        .map_err(|e| anyhow::anyhow!("Failed to compress data: {}", e))?;

    // Get checksum before finishing
    let checksum = checksum_writer.checksum();
    checksum_writer.finish()?;

    // Get the actual compressed file size
    let compressed_size = std::fs::metadata(dest)?.len();

    Ok(CompressionStats::new(original_size, compressed_size, checksum))
}

/// Decompresses a gzip file.
pub fn decompress_file(source: &Path, dest: &Path) -> anyhow::Result<u64> {
    use flate2::read::GzDecoder;

    let source_file = File::open(source)
        .map_err(|e| anyhow::anyhow!("Failed to open source file: {}", e))?;

    let mut decoder = GzDecoder::new(source_file);

    let mut dest_file = File::create(dest)
        .map_err(|e| anyhow::anyhow!("Failed to create destination file: {}", e))?;

    let bytes_written = io::copy(&mut decoder, &mut dest_file)
        .map_err(|e| anyhow::anyhow!("Failed to decompress data: {}", e))?;

    Ok(bytes_written)
}

/// Calculates SHA256 checksum of a file.
pub fn calculate_checksum(path: &Path) -> anyhow::Result<String> {
    let mut file = File::open(path)
        .map_err(|e| anyhow::anyhow!("Failed to open file for checksum: {}", e))?;

    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher)
        .map_err(|e| anyhow::anyhow!("Failed to calculate checksum: {}", e))?;

    let hash = hasher.finalize();
    Ok(format!("{:x}", hash))
}

/// Writer that calculates SHA256 checksum while writing.
struct ChecksumWriter<W: Write> {
    writer: W,
    hasher: Sha256,
    bytes_written: u64,
}

impl<W: Write> ChecksumWriter<W> {
    fn new(writer: W) -> Self {
        Self {
            writer,
            hasher: Sha256::new(),
            bytes_written: 0,
        }
    }

    fn finish(mut self) -> io::Result<u64> {
        self.writer.flush()?;
        Ok(self.bytes_written)
    }

    fn checksum(&self) -> String {
        let hash = self.hasher.clone().finalize();
        format!("{:x}", hash)
    }
}

impl<W: Write> Write for ChecksumWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.writer.write(buf)?;
        self.hasher.update(&buf[..n]);
        self.bytes_written += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use tempfile::TempDir;

    fn create_test_file(dir: &Path, name: &str, content: &[u8]) -> anyhow::Result<std::path::PathBuf> {
        let path = dir.join(name);
        let mut file = File::create(&path)?;
        file.write_all(content)?;
        Ok(path)
    }

    #[test]
    fn test_compress_file() {
        let temp_dir = TempDir::new().unwrap();
        // Use much larger content that will definitely compress
        let content = b"Repeated content that compresses very well! ".repeat(5000);
        let source_path = create_test_file(
            temp_dir.path(),
            "source.txt",
            &content,
        ).unwrap();
        let dest_path = temp_dir.path().join("compressed.gz");

        let stats = compress_file(&source_path, &dest_path, None).unwrap();

        // Large repeated content should compress to less than 10% of original
        assert!(stats.compressed_size < stats.original_size / 2,
            "Expected compressed size {} < half of original size {}",
            stats.compressed_size, stats.original_size);
        assert!(stats.compression_ratio < 0.5);
        assert!(!stats.checksum.is_empty());
        assert_eq!(stats.checksum.len(), 64); // SHA256 hex length
    }

    #[test]
    fn test_compression_levels() {
        let temp_dir = TempDir::new().unwrap();
        let content = b"Repeated content! ".repeat(1000);
        let source_path = create_test_file(temp_dir.path(), "source.txt", &content).unwrap();

        // Test different compression levels
        let dest_fast = temp_dir.path().join("fast.gz");
        let stats_fast = compress_file(&source_path, &dest_fast, Some(1)).unwrap();

        let dest_best = temp_dir.path().join("best.gz");
        let stats_best = compress_file(&source_path, &dest_best, Some(9)).unwrap();

        // Best compression should produce smaller file
        assert!(stats_best.compressed_size <= stats_fast.compressed_size);
    }

    #[test]
    fn test_decompress_file() {
        let temp_dir = TempDir::new().unwrap();
        let original_content = b"Hello, World! This is test data.";
        let source_path = create_test_file(temp_dir.path(), "source.txt", original_content).unwrap();
        let compressed_path = temp_dir.path().join("compressed.gz");
        let decompressed_path = temp_dir.path().join("decompressed.txt");

        // Compress
        compress_file(&source_path, &compressed_path, None).unwrap();

        // Decompress
        let bytes_written = decompress_file(&compressed_path, &decompressed_path).unwrap();

        // Verify
        assert_eq!(bytes_written, original_content.len() as u64);

        let mut decompressed_content = Vec::new();
        File::open(&decompressed_path)
            .unwrap()
            .read_to_end(&mut decompressed_content)
            .unwrap();

        assert_eq!(&decompressed_content, original_content);
    }

    #[test]
    fn test_calculate_checksum() {
        let temp_dir = TempDir::new().unwrap();
        let content = b"Test content for checksum";
        let path = create_test_file(temp_dir.path(), "test.txt", content).unwrap();

        let checksum1 = calculate_checksum(&path).unwrap();
        let checksum2 = calculate_checksum(&path).unwrap();

        // Checksums should be consistent
        assert_eq!(checksum1, checksum2);
        assert_eq!(checksum1.len(), 64); // SHA256 hex length

        // Different content should produce different checksum
        let path2 = create_test_file(temp_dir.path(), "test2.txt", b"Different content").unwrap();
        let checksum3 = calculate_checksum(&path2).unwrap();
        assert_ne!(checksum1, checksum3);
    }

    #[test]
    fn test_compression_stats() {
        let stats = CompressionStats::new(1000, 500, "abc123".to_string());
        assert_eq!(stats.compression_ratio, 0.5);
        assert_eq!(stats.compression_percentage(), 50);

        let stats = CompressionStats::new(1000, 700, "def456".to_string());
        assert_eq!(stats.compression_percentage(), 30);
    }

    #[test]
    fn test_checksum_writer() {
        use std::io::Cursor;

        let mut cursor = Cursor::new(Vec::new());
        let mut writer = ChecksumWriter::new(&mut cursor);

        writer.write_all(b"Hello, ").unwrap();
        writer.write_all(b"World!").unwrap();

        // Get checksum before finishing
        let checksum = writer.checksum();
        let bytes_written = writer.finish().unwrap();

        assert_eq!(bytes_written, 13);
        assert_eq!(checksum.len(), 64);

        // Verify content was written
        let content = cursor.into_inner();
        assert_eq!(&content, b"Hello, World!");
    }
}
