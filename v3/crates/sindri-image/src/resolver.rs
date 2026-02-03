use crate::registry::RegistryClient;
use crate::types::ResolutionStrategy;
use anyhow::{anyhow, Context, Result};
use semver::{Version, VersionReq};
use tracing::{debug, trace};

/// Resolves container image versions based on constraints
pub struct VersionResolver {
    registry_client: RegistryClient,
}

impl VersionResolver {
    /// Create a new version resolver
    pub fn new(registry_client: RegistryClient) -> Self {
        Self { registry_client }
    }

    /// Resolve a version constraint to a specific tag
    ///
    /// # Arguments
    /// * `repository` - Repository name (e.g., "pacphi/sindri")
    /// * `constraint` - Semver constraint (e.g., "^3.0.0", "~3.1.0", ">=3.0.0")
    /// * `allow_prerelease` - Whether to include prerelease versions
    ///
    /// # Returns
    /// The highest matching version tag
    pub async fn resolve_version(
        &self,
        repository: &str,
        constraint: &str,
        allow_prerelease: bool,
    ) -> Result<String> {
        debug!(
            "Resolving version for {}: constraint={}, allow_prerelease={}",
            repository, constraint, allow_prerelease
        );

        // Get all tags from registry
        let tags = self
            .registry_client
            .list_tags(repository)
            .await
            .with_context(|| format!("Failed to list tags for repository '{}'", repository))?;

        trace!("Found {} tags total", tags.len());

        // Parse constraint
        let version_req = VersionReq::parse(constraint)
            .with_context(|| format!("Invalid version constraint: {}", constraint))?;

        // Parse and filter tags by semver
        let mut matching_versions: Vec<(Version, String)> = tags
            .iter()
            .filter_map(|tag| {
                // Try to parse as semver (strip 'v' prefix if present)
                let version_str = tag.strip_prefix('v').unwrap_or(tag);
                match Version::parse(version_str) {
                    Ok(version) => {
                        // Check if version matches constraint
                        if version_req.matches(&version) {
                            // Filter prereleases if not allowed
                            if !allow_prerelease && !version.pre.is_empty() {
                                trace!("Skipping prerelease: {}", tag);
                                None
                            } else {
                                Some((version, tag.clone()))
                            }
                        } else {
                            None
                        }
                    }
                    Err(_) => {
                        trace!("Skipping non-semver tag: {}", tag);
                        None
                    }
                }
            })
            .collect();

        if matching_versions.is_empty() {
            return Err(anyhow!(
                "No matching versions found for constraint '{}' (allow_prerelease={})",
                constraint,
                allow_prerelease
            ));
        }

        // Sort by version (highest first)
        matching_versions.sort_by(|(a, _), (b, _)| b.cmp(a));

        let (highest_version, highest_tag) = &matching_versions[0];
        debug!("Resolved to: {} ({})", highest_tag, highest_version);

        Ok(highest_tag.clone())
    }

    /// Get the latest stable version (no prerelease)
    pub async fn get_latest_stable(&self, repository: &str) -> Result<String> {
        debug!("Finding latest stable version for {}", repository);

        let tags = self
            .registry_client
            .list_tags(repository)
            .await
            .context("Failed to list tags")?;

        let mut stable_versions: Vec<(Version, String)> = tags
            .iter()
            .filter_map(|tag| {
                let version_str = tag.strip_prefix('v').unwrap_or(tag);
                match Version::parse(version_str) {
                    Ok(version) if version.pre.is_empty() => Some((version, tag.clone())),
                    _ => None,
                }
            })
            .collect();

        if stable_versions.is_empty() {
            return Err(anyhow!("No stable versions found for {}", repository));
        }

        stable_versions.sort_by(|(a, _), (b, _)| b.cmp(a));

        let (_, highest_tag) = &stable_versions[0];
        debug!("Latest stable: {}", highest_tag);

        Ok(highest_tag.clone())
    }

    /// Resolve version based on strategy
    pub async fn resolve_with_strategy(
        &self,
        repository: &str,
        strategy: ResolutionStrategy,
        constraint: Option<&str>,
        cli_version: Option<&str>,
        allow_prerelease: bool,
    ) -> Result<String> {
        match strategy {
            ResolutionStrategy::Semver => {
                let constraint = constraint
                    .ok_or_else(|| anyhow!("Semver strategy requires a version constraint"))?;
                self.resolve_version(repository, constraint, allow_prerelease)
                    .await
            }
            ResolutionStrategy::LatestStable => self.get_latest_stable(repository).await,
            ResolutionStrategy::PinToCli => {
                let cli_version =
                    cli_version.ok_or_else(|| anyhow!("PinToCli strategy requires CLI version"))?;
                // Verify this version exists
                let tags = self.registry_client.list_tags(repository).await?;
                let normalized = if cli_version.starts_with('v') {
                    cli_version.to_string()
                } else {
                    format!("v{}", cli_version)
                };
                if tags.contains(&normalized) {
                    Ok(normalized)
                } else {
                    Err(anyhow!("Version {} not found in registry", normalized))
                }
            }
            ResolutionStrategy::Explicit => {
                // For explicit, the constraint IS the tag
                let tag = constraint.ok_or_else(|| anyhow!("Explicit strategy requires a tag"))?;
                Ok(tag.to_string())
            }
        }
    }

    /// Find all versions matching a constraint
    pub async fn find_matching_versions(
        &self,
        repository: &str,
        constraint: &str,
        allow_prerelease: bool,
    ) -> Result<Vec<String>> {
        let tags = self.registry_client.list_tags(repository).await?;

        let version_req = VersionReq::parse(constraint)?;

        let mut matching: Vec<(Version, String)> = tags
            .iter()
            .filter_map(|tag| {
                let version_str = tag.strip_prefix('v').unwrap_or(tag);
                match Version::parse(version_str) {
                    Ok(version) if version_req.matches(&version) => {
                        if !allow_prerelease && !version.pre.is_empty() {
                            None
                        } else {
                            Some((version, tag.clone()))
                        }
                    }
                    _ => None,
                }
            })
            .collect();

        matching.sort_by(|(a, _), (b, _)| b.cmp(a));

        Ok(matching.into_iter().map(|(_, tag)| tag).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        let versions = ["v3.0.0", "v3.1.0", "v3.1.1-alpha.1", "v2.9.0"];
        let constraint = VersionReq::parse("^3.0.0").unwrap();

        let matching: Vec<_> = versions
            .iter()
            .filter_map(|v| {
                let version_str = v.strip_prefix('v').unwrap_or(v);
                Version::parse(version_str).ok()
            })
            .filter(|v| constraint.matches(v))
            .collect();

        // ^3.0.0 matches 3.0.0 and 3.1.0, but not prereleases (3.1.1-alpha.1)
        // This is semver behavior: prereleases only match if explicitly specified
        assert_eq!(matching.len(), 2); // 3.0.0, 3.1.0
    }

    #[test]
    fn test_prerelease_filtering() {
        let version = Version::parse("3.1.0-alpha.1").unwrap();
        assert!(!version.pre.is_empty());

        let stable_version = Version::parse("3.1.0").unwrap();
        assert!(stable_version.pre.is_empty());
    }

    #[test]
    fn test_version_comparison() {
        let v1 = Version::parse("3.0.0").unwrap();
        let v2 = Version::parse("3.1.0").unwrap();
        let v3 = Version::parse("3.1.1").unwrap();

        assert!(v2 > v1);
        assert!(v3 > v2);
        assert!(v3 > v1);
    }
}
