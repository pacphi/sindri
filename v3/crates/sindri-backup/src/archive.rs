//! Archive creation and extraction for backups.
//!
//! This module handles tar archive creation with gzip compression,
//! manifest generation, and file filtering.

use crate::compression::{calculate_checksum, DEFAULT_COMPRESSION_LEVEL};
use crate::filters::ExclusionConfig;
use crate::manifest::{
    BackupManifest, BackupStatistics, ChecksumInfo, SourceInfo, MANIFEST_FILENAME,
};
use crate::profile::BackupProfile;
use crate::progress::BackupProgress;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tar::Builder as TarBuilder;
use walkdir::WalkDir;

/// Result of a backup operation.
#[derive(Debug, Clone)]
pub struct BackupResult {
    /// Path to the created archive
    pub archive_path: PathBuf,

    /// Size of the archive in bytes
    pub size_bytes: u64,

    /// Number of files included
    pub file_count: usize,

    /// Backup profile used
    pub profile: BackupProfile,

    /// Manifest with full metadata
    pub manifest: BackupManifest,

    /// Duration of the operation in seconds
    pub duration_seconds: f64,
}

/// Configuration for archive creation.
#[derive(Debug, Clone)]
pub struct ArchiveConfig {
    /// Backup profile to use
    pub profile: BackupProfile,

    /// Exclusion configuration
    pub exclusions: ExclusionConfig,

    /// Source information
    pub source: SourceInfo,

    /// Compression level (1-9)
    pub compression_level: u32,

    /// Whether to show progress
    pub show_progress: bool,
}

impl ArchiveConfig {
    /// Creates a new archive configuration.
    pub fn new(profile: BackupProfile, source: SourceInfo) -> anyhow::Result<Self> {
        Ok(Self {
            profile,
            exclusions: ExclusionConfig::new(vec![])?,
            source,
            compression_level: DEFAULT_COMPRESSION_LEVEL,
            show_progress: true,
        })
    }

    /// Adds additional exclusion patterns.
    pub fn with_exclusions(mut self, patterns: Vec<String>) -> anyhow::Result<Self> {
        self.exclusions = ExclusionConfig::new(patterns)?;
        Ok(self)
    }

    /// Sets the compression level.
    pub fn with_compression_level(mut self, level: u32) -> Self {
        self.compression_level = level.clamp(1, 9);
        self
    }

    /// Sets whether to show progress.
    pub fn with_progress(mut self, show_progress: bool) -> Self {
        self.show_progress = show_progress;
        self
    }
}

/// Archive builder for creating backups.
pub struct ArchiveBuilder {
    config: ArchiveConfig,
}

impl ArchiveBuilder {
    /// Creates a new archive builder.
    pub fn new(config: ArchiveConfig) -> Self {
        Self { config }
    }

    /// Creates a backup archive from the source directory.
    pub async fn create(
        &self,
        source_dir: &Path,
        output_path: &Path,
    ) -> anyhow::Result<BackupResult> {
        let start_time = Instant::now();

        // Initialize progress
        let mut progress = if self.config.show_progress {
            let mut p = BackupProgress::new();
            p.start_scan("Scanning files...");
            Some(p)
        } else {
            None
        };

        // Scan directory to find files to include
        let files_to_backup = self.scan_directory(source_dir, &mut progress)?;

        if let Some(ref progress) = progress {
            progress.finish_scan(&format!("Found {} files to backup", files_to_backup.len()));
        }

        // Create temporary uncompressed tar
        let temp_dir = tempfile::tempdir()?;
        let temp_tar = temp_dir.path().join("backup.tar");

        // Create tar archive
        let file_count =
            self.create_tar_archive(&temp_tar, source_dir, &files_to_backup, &mut progress)?;

        // Calculate uncompressed size
        let uncompressed_size = std::fs::metadata(&temp_tar)?.len();

        // Compress the tar archive
        if let Some(ref mut progress) = progress {
            progress.start_scan("Compressing archive...");
        }

        self.compress_archive(&temp_tar, output_path)?;

        if let Some(ref progress) = progress {
            progress.finish_scan("Compression complete");
        }

        // Calculate compressed size and checksum
        let compressed_size = std::fs::metadata(output_path)?.len();
        let checksum = calculate_checksum(output_path)?;

        // Create manifest
        let duration = start_time.elapsed().as_secs_f64();
        let statistics = BackupStatistics::new(file_count, uncompressed_size, compressed_size)
            .with_duration(duration);

        let checksum_info = ChecksumInfo {
            algorithm: "sha256".to_string(),
            value: checksum,
        };

        let manifest = BackupManifest::new(
            self.config.profile,
            self.config.source.clone(),
            statistics.clone(),
            checksum_info,
        );

        if let Some(ref progress) = progress {
            progress.finish_all();
        }

        Ok(BackupResult {
            archive_path: output_path.to_path_buf(),
            size_bytes: compressed_size,
            file_count,
            profile: self.config.profile,
            manifest,
            duration_seconds: duration,
        })
    }

    /// Scans the directory and returns list of files to backup.
    fn scan_directory(
        &self,
        source_dir: &Path,
        progress: &mut Option<BackupProgress>,
    ) -> anyhow::Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        // Get include paths for the profile
        let include_paths = match self.config.profile.includes() {
            Some(paths) => paths,
            None => vec![PathBuf::from(".")], // Full backup includes everything
        };

        for include_path in include_paths {
            let full_path = source_dir.join(&include_path);
            if !full_path.exists() {
                tracing::debug!("Skipping non-existent path: {:?}", full_path);
                continue;
            }

            for entry in WalkDir::new(&full_path)
                .follow_links(false)
                .into_iter()
                .filter_entry(|e| {
                    let rel_path = e.path().strip_prefix(source_dir).unwrap_or(e.path());
                    !self.config.exclusions.should_exclude(rel_path)
                })
            {
                let entry =
                    entry.map_err(|e| anyhow::anyhow!("Failed to walk directory: {}", e))?;

                if entry.file_type().is_file() {
                    let rel_path = entry
                        .path()
                        .strip_prefix(source_dir)
                        .map_err(|e| anyhow::anyhow!("Failed to compute relative path: {}", e))?;

                    // Check if this should be excluded
                    if !self.config.exclusions.should_exclude(rel_path) {
                        files.push(rel_path.to_path_buf());

                        if let Some(ref progress) = progress {
                            if files.len() % 100 == 0 {
                                progress.update_scan(&format!("Found {} files...", files.len()));
                            }
                        }
                    }
                }
            }
        }

        Ok(files)
    }

    /// Creates a tar archive with the specified files.
    fn create_tar_archive(
        &self,
        output_path: &Path,
        source_dir: &Path,
        files: &[PathBuf],
        progress: &mut Option<BackupProgress>,
    ) -> anyhow::Result<usize> {
        let file = File::create(output_path)?;
        let mut tar = TarBuilder::new(file);

        // Add manifest as first file
        let manifest_temp = self.create_temp_manifest(files.len())?;
        tar.append_path_with_name(&manifest_temp, MANIFEST_FILENAME)?;

        // Initialize archive progress
        if let Some(ref mut progress) = progress {
            progress.start_archive(files.len() as u64, "Creating archive...");
        }

        // Add files to archive
        let mut file_count = 1; // Count manifest
        for file_path in files {
            let full_path = source_dir.join(file_path);
            tar.append_path_with_name(&full_path, file_path)?;

            file_count += 1;

            if let Some(ref progress) = progress {
                progress.inc_archive();
            }
        }

        tar.finish()?;

        if let Some(ref progress) = progress {
            progress.finish_archive(&format!("Added {} files to archive", file_count));
        }

        Ok(file_count)
    }

    /// Creates a temporary manifest file (will be replaced with final manifest after compression).
    fn create_temp_manifest(&self, estimated_files: usize) -> anyhow::Result<PathBuf> {
        let temp_manifest = BackupManifest::new(
            self.config.profile,
            self.config.source.clone(),
            BackupStatistics::new(estimated_files, 0, 0),
            ChecksumInfo {
                algorithm: "sha256".to_string(),
                value: "pending".to_string(),
            },
        );

        let temp_dir = tempfile::tempdir()?;
        let manifest_path = temp_dir.path().join(MANIFEST_FILENAME);
        let mut file = File::create(&manifest_path)?;
        file.write_all(temp_manifest.to_json()?.as_bytes())?;

        // Keep the temp dir alive by forgetting it (caller is responsible for cleanup)
        std::mem::forget(temp_dir);

        Ok(manifest_path)
    }

    /// Compresses the tar archive with gzip.
    fn compress_archive(&self, source: &Path, dest: &Path) -> anyhow::Result<()> {
        let source_file = File::open(source)?;
        let dest_file = File::create(dest)?;

        let encoder = GzEncoder::new(dest_file, Compression::new(self.config.compression_level));
        let mut reader = std::io::BufReader::new(source_file);
        let mut writer = std::io::BufWriter::new(encoder);

        std::io::copy(&mut reader, &mut writer)?;
        writer.flush()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_directory() -> anyhow::Result<TempDir> {
        let temp_dir = TempDir::new()?;
        let base = temp_dir.path();

        // Create test structure
        fs::create_dir_all(base.join("workspace/projects"))?;
        fs::create_dir_all(base.join(".claude"))?;
        fs::write(base.join("workspace/projects/test.txt"), "test content")?;
        fs::write(base.join(".gitconfig"), "git config")?;
        fs::write(base.join(".claude/config.json"), "{}")?;

        // Create cache that should be excluded
        fs::create_dir_all(base.join(".cache"))?;
        fs::write(base.join(".cache/data.bin"), "cache")?;

        Ok(temp_dir)
    }

    #[tokio::test]
    async fn test_archive_creation() {
        let source_dir = create_test_directory().unwrap();
        let output_dir = TempDir::new().unwrap();
        let output_path = output_dir.path().join("backup.tar.gz");

        let source_info = SourceInfo {
            instance_name: "test".to_string(),
            provider: "test".to_string(),
            hostname: "localhost".to_string(),
        };

        let config = ArchiveConfig::new(BackupProfile::UserData, source_info)
            .unwrap()
            .with_progress(false);

        let builder = ArchiveBuilder::new(config);
        let result = builder
            .create(source_dir.path(), &output_path)
            .await
            .unwrap();

        assert!(output_path.exists());
        assert!(result.size_bytes > 0);
        assert!(result.file_count > 0);
        assert_eq!(result.profile, BackupProfile::UserData);
    }

    #[tokio::test]
    async fn test_archive_excludes_cache() {
        let source_dir = create_test_directory().unwrap();
        let output_dir = TempDir::new().unwrap();
        let output_path = output_dir.path().join("backup.tar.gz");

        let source_info = SourceInfo {
            instance_name: "test".to_string(),
            provider: "test".to_string(),
            hostname: "localhost".to_string(),
        };

        let config = ArchiveConfig::new(BackupProfile::Standard, source_info)
            .unwrap()
            .with_progress(false);

        let builder = ArchiveBuilder::new(config);
        let _result = builder
            .create(source_dir.path(), &output_path)
            .await
            .unwrap();

        // Extract and verify cache is not included
        let extract_dir = TempDir::new().unwrap();
        let tar_gz = File::open(&output_path).unwrap();
        let tar = flate2::read::GzDecoder::new(tar_gz);
        let mut archive = tar::Archive::new(tar);
        archive.unpack(extract_dir.path()).unwrap();

        assert!(!extract_dir.path().join(".cache").exists());
    }

    #[test]
    fn test_archive_config_builder() {
        let source_info = SourceInfo {
            instance_name: "test".to_string(),
            provider: "test".to_string(),
            hostname: "localhost".to_string(),
        };

        let config = ArchiveConfig::new(BackupProfile::Full, source_info.clone())
            .unwrap()
            .with_compression_level(9)
            .with_progress(false);

        assert_eq!(config.profile, BackupProfile::Full);
        assert_eq!(config.compression_level, 9);
        assert!(!config.show_progress);
    }

    #[test]
    fn test_archive_config_compression_clamping() {
        let source_info = SourceInfo {
            instance_name: "test".to_string(),
            provider: "test".to_string(),
            hostname: "localhost".to_string(),
        };

        let config = ArchiveConfig::new(BackupProfile::Standard, source_info)
            .unwrap()
            .with_compression_level(15);

        assert_eq!(config.compression_level, 9); // Should be clamped to max
    }
}
