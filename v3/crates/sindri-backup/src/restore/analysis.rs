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
