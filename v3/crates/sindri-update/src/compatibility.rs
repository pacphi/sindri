//! Compatibility matrix checking

use anyhow::{anyhow, Result};
use semver::{Version, VersionReq};
use sindri_core::types::CompatibilityMatrix;
use std::collections::HashMap;

/// Compatibility check result
#[derive(Debug, Clone)]
pub struct CompatResult {
    /// Whether the upgrade is compatible
    pub compatible: bool,

    /// Incompatible extensions
    pub incompatible_extensions: Vec<IncompatibleExtension>,

    /// Warning messages
    pub warnings: Vec<String>,

    /// Breaking changes
    pub breaking_changes: Vec<String>,
}

/// Incompatible extension info
#[derive(Debug, Clone)]
pub struct IncompatibleExtension {
    /// Extension name
    pub name: String,

    /// Current version
    pub current_version: String,

    /// Required version range
    pub required_range: String,

    /// Reason for incompatibility
    pub reason: String,
}

/// Compatibility checker for CLI upgrades
pub struct CompatibilityChecker {
    /// Compatibility matrix
    matrix: Option<CompatibilityMatrix>,
}

impl CompatibilityChecker {
    /// Create a new checker
    pub fn new() -> Self {
        Self { matrix: None }
    }

    /// Load compatibility matrix from URL
    pub async fn load_matrix(&mut self, url: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let response = client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to fetch compatibility matrix"));
        }

        let content = response.text().await?;
        self.matrix = Some(serde_yaml::from_str(&content)?);

        Ok(())
    }

    /// Load compatibility matrix from string
    pub fn load_matrix_from_str(&mut self, content: &str) -> Result<()> {
        self.matrix = Some(serde_yaml::from_str(content)?);
        Ok(())
    }

    /// Check compatibility for an upgrade
    pub fn check_compatibility(
        &self,
        target_version: &str,
        installed_extensions: &HashMap<String, String>,
    ) -> Result<CompatResult> {
        let matrix = self
            .matrix
            .as_ref()
            .ok_or_else(|| anyhow!("Compatibility matrix not loaded"))?;

        let mut incompatible = Vec::new();
        let warnings = Vec::new();
        let mut breaking_changes = Vec::new();

        // Find matching version entry (using semver matching)
        let version_entry = self.find_version_entry(matrix, target_version)?;

        // Check extension compatibility
        for (ext_name, ext_version) in installed_extensions {
            if let Some(required_range) = version_entry.compatible_extensions.get(ext_name) {
                let req = VersionReq::parse(required_range)
                    .map_err(|e| anyhow!("Invalid version range for {}: {}", ext_name, e))?;

                let current = Version::parse(ext_version).unwrap_or_else(|_| Version::new(0, 0, 0));

                if !req.matches(&current) {
                    incompatible.push(IncompatibleExtension {
                        name: ext_name.clone(),
                        current_version: ext_version.clone(),
                        required_range: required_range.clone(),
                        reason: format!(
                            "Version {} does not satisfy {}",
                            ext_version, required_range
                        ),
                    });
                }
            }
        }

        // Collect breaking changes
        breaking_changes.extend(version_entry.breaking_changes.iter().cloned());

        Ok(CompatResult {
            compatible: incompatible.is_empty(),
            incompatible_extensions: incompatible,
            warnings,
            breaking_changes,
        })
    }

    /// Find version entry in matrix (supports wildcards like "3.0.x")
    fn find_version_entry<'a>(
        &self,
        matrix: &'a CompatibilityMatrix,
        target_version: &str,
    ) -> Result<&'a sindri_core::types::CliVersionCompat> {
        let target = Version::parse(target_version)?;

        // Try exact match first
        if let Some(entry) = matrix.cli_versions.get(target_version) {
            return Ok(entry);
        }

        // Try wildcard matches (e.g., "3.0.x")
        for (pattern, entry) in &matrix.cli_versions {
            if pattern.ends_with(".x") {
                let prefix = &pattern[..pattern.len() - 2];
                let parts: Vec<&str> = prefix.split('.').collect();

                if parts.len() == 2 {
                    if let (Ok(major), Ok(minor)) =
                        (parts[0].parse::<u64>(), parts[1].parse::<u64>())
                    {
                        if target.major == major && target.minor == minor {
                            return Ok(entry);
                        }
                    }
                }
            }
        }

        Err(anyhow!(
            "No compatibility entry found for version {}",
            target_version
        ))
    }
}

impl Default for CompatibilityChecker {
    fn default() -> Self {
        Self::new()
    }
}
