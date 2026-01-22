//! Backup manifest format and metadata.
//!
//! The manifest is stored as the first file in every backup archive,
//! providing metadata about the backup contents, source, and integrity.

use crate::profile::BackupProfile;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Version of the backup manifest format.
pub const MANIFEST_VERSION: &str = "1.0.0";

/// Name of the manifest file in the backup archive.
pub const MANIFEST_FILENAME: &str = ".backup-manifest.json";

/// Complete backup manifest stored in archive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    /// Manifest format version
    pub version: String,

    /// Backup type/profile used
    pub backup_type: String,

    /// When the backup was created
    pub created_at: DateTime<Utc>,

    /// What created this backup (e.g., "sindri-cli v3.0.0")
    pub created_by: String,

    /// Information about the source instance
    pub source: SourceInfo,

    /// Backup profile used
    pub profile: BackupProfile,

    /// Compression algorithm used
    pub compression: String,

    /// Checksum information
    pub checksum: ChecksumInfo,

    /// Backup statistics
    pub statistics: BackupStatistics,

    /// Extension information (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<ExtensionInfo>,
}

/// Information about the backup source instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInfo {
    /// Instance name (from sindri.yaml)
    pub instance_name: String,

    /// Provider (docker, fly, devpod)
    pub provider: String,

    /// Hostname or identifier
    pub hostname: String,
}

/// Checksum information for integrity verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecksumInfo {
    /// Hash algorithm (sha256)
    pub algorithm: String,

    /// Hex-encoded checksum value
    pub value: String,
}

/// Statistics about the backup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupStatistics {
    /// Number of files included in backup
    pub files_included: usize,

    /// Total uncompressed size in bytes
    pub total_size_bytes: u64,

    /// Compressed archive size in bytes
    pub compressed_size_bytes: u64,

    /// Compression ratio (0.0-1.0)
    pub compression_ratio: f64,

    /// Duration of backup operation in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<f64>,
}

/// Information about installed extensions (optional).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionInfo {
    /// List of installed extension names
    pub installed: Vec<String>,

    /// Extension versions (name -> version)
    pub versions: HashMap<String, String>,
}

impl BackupManifest {
    /// Creates a new backup manifest.
    pub fn new(
        profile: BackupProfile,
        source: SourceInfo,
        statistics: BackupStatistics,
        checksum: ChecksumInfo,
    ) -> Self {
        Self {
            version: MANIFEST_VERSION.to_string(),
            backup_type: profile.to_string(),
            created_at: Utc::now(),
            created_by: format!("sindri-cli v{}", env!("CARGO_PKG_VERSION")),
            source,
            profile,
            compression: "gzip".to_string(),
            checksum,
            statistics,
            extensions: None,
        }
    }

    /// Serializes the manifest to JSON.
    pub fn to_json(&self) -> anyhow::Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| anyhow::anyhow!("Failed to serialize manifest: {}", e))
    }

    /// Deserializes a manifest from JSON.
    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        serde_json::from_str(json)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize manifest: {}", e))
    }

    /// Validates that the manifest is compatible with the current version.
    pub fn validate(&self) -> anyhow::Result<()> {
        // Check version compatibility
        if self.version != MANIFEST_VERSION {
            return Err(anyhow::anyhow!(
                "Incompatible manifest version: {} (expected {})",
                self.version,
                MANIFEST_VERSION
            ));
        }

        // Check checksum algorithm
        if self.checksum.algorithm != "sha256" {
            return Err(anyhow::anyhow!(
                "Unsupported checksum algorithm: {}",
                self.checksum.algorithm
            ));
        }

        // Check compression
        if self.compression != "gzip" {
            return Err(anyhow::anyhow!(
                "Unsupported compression: {}",
                self.compression
            ));
        }

        Ok(())
    }
}

impl BackupStatistics {
    /// Creates new backup statistics.
    pub fn new(files_included: usize, total_size_bytes: u64, compressed_size_bytes: u64) -> Self {
        let compression_ratio = if total_size_bytes > 0 {
            compressed_size_bytes as f64 / total_size_bytes as f64
        } else {
            0.0
        };

        Self {
            files_included,
            total_size_bytes,
            compressed_size_bytes,
            compression_ratio,
            duration_seconds: None,
        }
    }

    /// Sets the backup duration.
    pub fn with_duration(mut self, duration_seconds: f64) -> Self {
        self.duration_seconds = Some(duration_seconds);
        self
    }

    /// Returns a human-readable size string.
    pub fn human_readable_size(&self) -> String {
        human_bytes(self.compressed_size_bytes)
    }

    /// Returns the compression ratio as a percentage.
    pub fn compression_percentage(&self) -> u8 {
        ((1.0 - self.compression_ratio) * 100.0) as u8
    }
}

/// Formats bytes as a human-readable string.
fn human_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_source_info() -> SourceInfo {
        SourceInfo {
            instance_name: "test-instance".to_string(),
            provider: "docker".to_string(),
            hostname: "localhost".to_string(),
        }
    }

    fn sample_checksum() -> ChecksumInfo {
        ChecksumInfo {
            algorithm: "sha256".to_string(),
            value: "abc123def456".to_string(),
        }
    }

    fn sample_statistics() -> BackupStatistics {
        BackupStatistics::new(100, 10_000_000, 5_000_000)
    }

    #[test]
    fn test_manifest_creation() {
        let manifest = BackupManifest::new(
            BackupProfile::Standard,
            sample_source_info(),
            sample_statistics(),
            sample_checksum(),
        );

        assert_eq!(manifest.version, MANIFEST_VERSION);
        assert_eq!(manifest.backup_type, "standard");
        assert_eq!(manifest.compression, "gzip");
        assert_eq!(manifest.profile, BackupProfile::Standard);
    }

    #[test]
    fn test_manifest_serialization() {
        let manifest = BackupManifest::new(
            BackupProfile::UserData,
            sample_source_info(),
            sample_statistics(),
            sample_checksum(),
        );

        let json = manifest.to_json().unwrap();
        assert!(json.contains("version"));
        assert!(json.contains("backup_type"));
        assert!(json.contains("user-data"));

        let deserialized = BackupManifest::from_json(&json).unwrap();
        assert_eq!(deserialized.version, manifest.version);
        assert_eq!(deserialized.profile, manifest.profile);
    }

    #[test]
    fn test_manifest_validation() {
        let mut manifest = BackupManifest::new(
            BackupProfile::Full,
            sample_source_info(),
            sample_statistics(),
            sample_checksum(),
        );

        assert!(manifest.validate().is_ok());

        // Invalid version
        manifest.version = "2.0.0".to_string();
        assert!(manifest.validate().is_err());

        manifest.version = MANIFEST_VERSION.to_string();
        assert!(manifest.validate().is_ok());

        // Invalid checksum algorithm
        manifest.checksum.algorithm = "md5".to_string();
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn test_statistics_compression_ratio() {
        let stats = BackupStatistics::new(100, 10_000_000, 5_000_000);
        assert_eq!(stats.compression_ratio, 0.5);
        assert_eq!(stats.compression_percentage(), 50);

        let stats = BackupStatistics::new(100, 10_000_000, 7_000_000);
        assert_eq!(stats.compression_percentage(), 30);
    }

    #[test]
    fn test_statistics_with_duration() {
        let stats = BackupStatistics::new(100, 10_000_000, 5_000_000).with_duration(45.5);
        assert_eq!(stats.duration_seconds, Some(45.5));
    }

    #[test]
    fn test_human_bytes() {
        assert_eq!(human_bytes(512), "512 B");
        assert_eq!(human_bytes(1024), "1.00 KB");
        assert_eq!(human_bytes(1_048_576), "1.00 MB");
        assert_eq!(human_bytes(5_242_880), "5.00 MB");
        assert_eq!(human_bytes(1_073_741_824), "1.00 GB");
    }

    #[test]
    fn test_manifest_with_extensions() {
        let mut manifest = BackupManifest::new(
            BackupProfile::Standard,
            sample_source_info(),
            sample_statistics(),
            sample_checksum(),
        );

        let mut versions = HashMap::new();
        versions.insert("nodejs".to_string(), "1.2.0".to_string());
        versions.insert("python".to_string(), "3.1.0".to_string());

        manifest.extensions = Some(ExtensionInfo {
            installed: vec!["nodejs".to_string(), "python".to_string()],
            versions,
        });

        let json = manifest.to_json().unwrap();
        assert!(json.contains("extensions"));
        assert!(json.contains("nodejs"));
    }
}
