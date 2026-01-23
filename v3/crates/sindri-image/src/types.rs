use serde::{Deserialize, Serialize};
use std::fmt;

/// Container image reference with registry, repository, and tag/digest
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageReference {
    /// Registry hostname (e.g., "ghcr.io", "docker.io")
    pub registry: String,
    /// Repository path (e.g., "pacphi/sindri")
    pub repository: String,
    /// Tag (e.g., "v3.0.0") - mutually exclusive with digest
    pub tag: Option<String>,
    /// Digest (e.g., "sha256:abc123...") - mutually exclusive with tag
    pub digest: Option<String>,
}

impl ImageReference {
    /// Parse an image reference string like "ghcr.io/pacphi/sindri:v3.0.0"
    pub fn parse(s: &str) -> anyhow::Result<Self> {
        // Split by @ for digest references
        let (image_part, digest) = if let Some(idx) = s.find('@') {
            let (before, after) = s.split_at(idx);
            (before, Some(after[1..].to_string()))
        } else {
            (s, None)
        };

        // Split by : for tag references (only if no digest)
        let (registry_repo, tag) = if digest.is_none() {
            if let Some(idx) = image_part.rfind(':') {
                // Make sure this : is not part of a port number
                let before = &image_part[..idx];
                let after = &image_part[idx + 1..];

                // If there's a / after the last :, it's part of registry (like localhost:5000/image)
                if !before.contains('/') || after.contains('/') {
                    (image_part, Some("latest".to_string()))
                } else {
                    (before, Some(after.to_string()))
                }
            } else {
                (image_part, Some("latest".to_string()))
            }
        } else {
            (image_part, None)
        };

        // Split registry and repository
        let parts: Vec<&str> = registry_repo.splitn(2, '/').collect();
        let (registry, repository) = if parts.len() == 2 {
            (parts[0].to_string(), parts[1].to_string())
        } else {
            // Default to docker.io if no registry specified
            ("docker.io".to_string(), registry_repo.to_string())
        };

        Ok(Self {
            registry,
            repository,
            tag,
            digest,
        })
    }
}

impl fmt::Display for ImageReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let base = format!("{}/{}", self.registry, self.repository);
        if let Some(digest) = &self.digest {
            write!(f, "{}@{}", base, digest)
        } else if let Some(tag) = &self.tag {
            write!(f, "{}:{}", base, tag)
        } else {
            write!(f, "{}:latest", base)
        }
    }
}

/// Container image metadata from registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInfo {
    /// Image digest
    pub digest: String,
    /// Image tags
    pub tags: Vec<String>,
    /// Image size in bytes
    pub size: Option<u64>,
    /// Creation timestamp
    pub created: Option<String>,
    /// Image labels
    pub labels: std::collections::HashMap<String, String>,
    /// Platform information (OS/arch)
    pub platforms: Vec<Platform>,
}

/// Platform information for multi-arch images
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Platform {
    pub os: String,
    pub architecture: String,
    pub variant: Option<String>,
}

/// Image manifest from OCI registry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageManifest {
    pub schema_version: i32,
    pub media_type: String,
    pub config: ManifestConfig,
    pub layers: Vec<ManifestLayer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestConfig {
    pub media_type: String,
    pub size: u64,
    pub digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestLayer {
    pub media_type: String,
    pub size: u64,
    pub digest: String,
}

/// Image signature verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureVerification {
    pub verified: bool,
    pub signatures: Vec<SignatureInfo>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureInfo {
    pub issuer: String,
    pub subject: String,
    pub valid_from: String,
    pub valid_until: String,
}

/// Image provenance verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceVerification {
    pub verified: bool,
    pub slsa_level: Option<String>,
    pub builder_id: Option<String>,
    pub source_repo: Option<String>,
    pub errors: Vec<String>,
}

/// Software Bill of Materials (SBOM)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sbom {
    pub format: String, // "spdx-json", "cyclonedx-json", etc.
    pub version: String,
    pub packages: Vec<SbomPackage>,
    pub raw_data: String, // Full SBOM content
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SbomPackage {
    pub name: String,
    pub version: Option<String>,
    pub supplier: Option<String>,
    pub license: Option<String>,
}

/// Resolution strategy for finding image versions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ResolutionStrategy {
    /// Use semantic versioning constraints
    #[default]
    Semver,
    /// Use the latest stable version
    LatestStable,
    /// Pin to CLI version
    PinToCli,
    /// Use explicit tag/digest
    Explicit,
}

/// Pull policy for container images
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub enum PullPolicy {
    /// Always pull the image
    Always,
    /// Only pull if not present locally
    #[default]
    IfNotPresent,
    /// Never pull, use local only
    Never,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_image_reference() {
        let cases = vec![
            (
                "ghcr.io/pacphi/sindri:v3.0.0",
                ("ghcr.io", "pacphi/sindri", Some("v3.0.0"), None),
            ),
            (
                "ghcr.io/pacphi/sindri@sha256:abc123",
                ("ghcr.io", "pacphi/sindri", None, Some("sha256:abc123")),
            ),
            (
                "ghcr.io/pacphi/sindri",
                ("ghcr.io", "pacphi/sindri", Some("latest"), None),
            ),
            (
                "sindri:v3.0.0",
                ("docker.io", "sindri:v3.0.0", Some("latest"), None),
            ),
        ];

        for (input, (registry, repo, tag, digest)) in cases {
            let result = ImageReference::parse(input);
            assert!(result.is_ok(), "Failed to parse: {}", input);

            let img = result.unwrap();
            assert_eq!(img.registry, registry);
            assert!(
                img.repository.contains(repo.split(':').next().unwrap()),
                "Repository mismatch for {}",
                input
            );
            if let Some(expected_tag) = tag {
                assert_eq!(
                    img.tag.as_deref(),
                    Some(expected_tag),
                    "Tag mismatch for {}",
                    input
                );
            }
            if let Some(expected_digest) = digest {
                assert_eq!(
                    img.digest.as_deref(),
                    Some(expected_digest),
                    "Digest mismatch for {}",
                    input
                );
            }
        }
    }

    #[test]
    fn test_image_reference_display() {
        let img = ImageReference {
            registry: "ghcr.io".to_string(),
            repository: "pacphi/sindri".to_string(),
            tag: Some("v3.0.0".to_string()),
            digest: None,
        };
        assert_eq!(format!("{}", img), "ghcr.io/pacphi/sindri:v3.0.0");

        let img_with_digest = ImageReference {
            registry: "ghcr.io".to_string(),
            repository: "pacphi/sindri".to_string(),
            tag: None,
            digest: Some("sha256:abc123".to_string()),
        };
        assert_eq!(
            format!("{}", img_with_digest),
            "ghcr.io/pacphi/sindri@sha256:abc123"
        );
    }
}
