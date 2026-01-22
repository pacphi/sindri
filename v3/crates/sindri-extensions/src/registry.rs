//! Extension registry management
//!
//! Handles loading extension registry and profiles from:
//! 1. Local filesystem (for deployed instances)
//! 2. GitHub repository (with caching)
//! 3. Cached files (~/.sindri/cache/)
//!
//! The registry tracks available extensions and their metadata,
//! while the manifest (separate module) tracks installed versions.

use anyhow::{anyhow, Result};
use sindri_core::types::{
    Extension, ExtensionRegistry as RegistryFile, Profile, ProfilesFile, RegistryEntry,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{debug, info, warn};

/// GitHub repository for extension registry
const GITHUB_REPO: &str = "pacphi/sindri";
const GITHUB_RAW_URL: &str = "https://raw.githubusercontent.com";
const CACHE_TTL: Duration = Duration::from_secs(3600); // 1 hour

/// Extension registry with loaded extensions and profiles
pub struct ExtensionRegistry {
    /// Registry entries
    pub entries: HashMap<String, RegistryEntry>,

    /// Available profiles
    pub profiles: HashMap<String, Profile>,

    /// Loaded extension definitions
    pub extensions: HashMap<String, Extension>,

    /// Cache directory for downloaded registry files
    cache_dir: Option<PathBuf>,
}

impl ExtensionRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            profiles: HashMap::new(),
            extensions: HashMap::new(),
            cache_dir: None,
        }
    }

    /// Create a registry with cache directory
    pub fn with_cache(cache_dir: PathBuf) -> Self {
        Self {
            entries: HashMap::new(),
            profiles: HashMap::new(),
            extensions: HashMap::new(),
            cache_dir: Some(cache_dir),
        }
    }

    /// Load registry from local files
    ///
    /// Used when running inside a deployed Sindri instance where
    /// registry files are baked into the image at /docker/lib/
    pub fn load_local(registry_path: &Path, profiles_path: &Path) -> Result<Self> {
        debug!(
            "Loading registry from local files: {:?}, {:?}",
            registry_path, profiles_path
        );

        let registry_content = std::fs::read_to_string(registry_path)?;
        let registry: RegistryFile = serde_yaml::from_str(&registry_content)?;

        let profiles_content = std::fs::read_to_string(profiles_path)?;
        let profiles: ProfilesFile = serde_yaml::from_str(&profiles_content)?;

        info!(
            "Loaded {} extensions and {} profiles from local files",
            registry.extensions.len(),
            profiles.profiles.len()
        );

        Ok(Self {
            entries: registry.extensions,
            profiles: profiles.profiles,
            extensions: HashMap::new(),
            cache_dir: None,
        })
    }

    /// Load registry from GitHub with caching
    ///
    /// Downloads registry.yaml and profiles.yaml from GitHub,
    /// caching them locally for TTL duration. Falls back to cache
    /// if network is unavailable.
    pub async fn load_from_github(cache_dir: PathBuf, branch: &str) -> Result<Self> {
        debug!("Loading registry from GitHub (branch: {})", branch);

        // Ensure cache directory exists
        std::fs::create_dir_all(&cache_dir)?;

        let registry_cache = cache_dir.join("registry.yaml");
        let profiles_cache = cache_dir.join("profiles.yaml");

        // Check cache validity
        let cache_valid =
            Self::is_cache_valid(&registry_cache) && Self::is_cache_valid(&profiles_cache);

        let (registry_content, profiles_content) = if cache_valid {
            debug!("Using cached registry files");
            (
                std::fs::read_to_string(&registry_cache)?,
                std::fs::read_to_string(&profiles_cache)?,
            )
        } else {
            debug!("Fetching fresh registry from GitHub");
            match Self::fetch_from_github(branch).await {
                Ok((reg, prof)) => {
                    // Cache the downloaded files
                    std::fs::write(&registry_cache, &reg)?;
                    std::fs::write(&profiles_cache, &prof)?;
                    info!("Cached registry files for future use");
                    (reg, prof)
                }
                Err(e) => {
                    warn!("Failed to fetch from GitHub: {}. Trying cache...", e);
                    // Try to use expired cache as fallback
                    if registry_cache.exists() && profiles_cache.exists() {
                        warn!("Using expired cache as fallback");
                        (
                            std::fs::read_to_string(&registry_cache)?,
                            std::fs::read_to_string(&profiles_cache)?,
                        )
                    } else {
                        return Err(anyhow!(
                            "Failed to fetch from GitHub and no cache available: {}",
                            e
                        ));
                    }
                }
            }
        };

        let registry: RegistryFile = serde_yaml::from_str(&registry_content)?;
        let profiles: ProfilesFile = serde_yaml::from_str(&profiles_content)?;

        info!(
            "Loaded {} extensions and {} profiles from GitHub",
            registry.extensions.len(),
            profiles.profiles.len()
        );

        Ok(Self {
            entries: registry.extensions,
            profiles: profiles.profiles,
            extensions: HashMap::new(),
            cache_dir: Some(cache_dir),
        })
    }

    /// Check if cached file is still valid (within TTL)
    fn is_cache_valid(path: &Path) -> bool {
        if let Ok(metadata) = std::fs::metadata(path) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(elapsed) = modified.elapsed() {
                    return elapsed < CACHE_TTL;
                }
            }
        }
        false
    }

    /// Fetch registry files from GitHub
    async fn fetch_from_github(branch: &str) -> Result<(String, String)> {
        let registry_url = format!(
            "{}/{}/{}/v3/registry.yaml",
            GITHUB_RAW_URL, GITHUB_REPO, branch
        );
        let profiles_url = format!(
            "{}/{}/{}/v3/profiles.yaml",
            GITHUB_RAW_URL, GITHUB_REPO, branch
        );

        debug!("Fetching registry from: {}", registry_url);
        debug!("Fetching profiles from: {}", profiles_url);

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        let registry_response = client.get(&registry_url).send().await?;
        if !registry_response.status().is_success() {
            return Err(anyhow!(
                "Failed to fetch registry.yaml: HTTP {}",
                registry_response.status()
            ));
        }
        let registry_content = registry_response.text().await?;

        let profiles_response = client.get(&profiles_url).send().await?;
        if !profiles_response.status().is_success() {
            return Err(anyhow!(
                "Failed to fetch profiles.yaml: HTTP {}",
                profiles_response.status()
            ));
        }
        let profiles_content = profiles_response.text().await?;

        Ok((registry_content, profiles_content))
    }

    /// Invalidate cache and force refresh on next load
    pub fn invalidate_cache(&self) -> Result<()> {
        if let Some(cache_dir) = &self.cache_dir {
            let registry_cache = cache_dir.join("registry.yaml");
            let profiles_cache = cache_dir.join("profiles.yaml");

            if registry_cache.exists() {
                std::fs::remove_file(&registry_cache)?;
            }
            if profiles_cache.exists() {
                std::fs::remove_file(&profiles_cache)?;
            }

            info!("Invalidated registry cache");
        }
        Ok(())
    }

    /// Load an extension definition from file
    pub fn load_extension(&mut self, name: &str, extension_path: &Path) -> Result<()> {
        let content = std::fs::read_to_string(extension_path)?;
        let extension: Extension = serde_yaml::from_str(&content)?;

        if extension.metadata.name != name {
            return Err(anyhow!(
                "Extension name mismatch: file says '{}' but expected '{}'",
                extension.metadata.name,
                name
            ));
        }

        self.extensions.insert(name.to_string(), extension);
        Ok(())
    }

    /// Get extension entry by name
    pub fn get_entry(&self, name: &str) -> Option<&RegistryEntry> {
        self.entries.get(name)
    }

    /// Get loaded extension definition
    pub fn get_extension(&self, name: &str) -> Option<&Extension> {
        self.extensions.get(name)
    }

    /// Get profile by name
    pub fn get_profile(&self, name: &str) -> Option<&Profile> {
        self.profiles.get(name)
    }

    /// List all extension names
    pub fn list_extensions(&self) -> Vec<&str> {
        self.entries.keys().map(|s| s.as_str()).collect()
    }

    /// List all profile names
    pub fn list_profiles(&self) -> Vec<&str> {
        self.profiles.keys().map(|s| s.as_str()).collect()
    }

    /// Get extensions for a profile
    pub fn get_profile_extensions(&self, profile_name: &str) -> Result<Vec<String>> {
        let profile = self
            .profiles
            .get(profile_name)
            .ok_or_else(|| anyhow!("Unknown profile: {}", profile_name))?;

        Ok(profile.extensions.clone())
    }

    /// Check if an extension exists
    pub fn has_extension(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    /// Check if a profile exists
    pub fn has_profile(&self, name: &str) -> bool {
        self.profiles.contains_key(name)
    }

    /// Get dependencies for an extension
    pub fn get_dependencies(&self, name: &str) -> Vec<String> {
        self.entries
            .get(name)
            .map(|e| e.dependencies.clone())
            .unwrap_or_default()
    }

    /// Get conflicts for an extension
    pub fn get_conflicts(&self, name: &str) -> Vec<String> {
        self.entries
            .get(name)
            .map(|e| e.conflicts.clone())
            .unwrap_or_default()
    }

    /// Check if extension is protected
    pub fn is_protected(&self, name: &str) -> bool {
        self.entries.get(name).map(|e| e.protected).unwrap_or(false)
    }

    /// Filter extensions by category
    pub fn list_by_category(&self, category: &str) -> Vec<&str> {
        self.entries
            .iter()
            .filter(|(_, entry)| entry.category == category)
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// Get all categories
    pub fn list_categories(&self) -> Vec<String> {
        let mut categories: Vec<_> = self.entries.values().map(|e| e.category.clone()).collect();
        categories.sort();
        categories.dedup();
        categories
    }

    /// Search extensions by name or description
    pub fn search(&self, query: &str) -> Vec<&str> {
        let query_lower = query.to_lowercase();
        self.entries
            .iter()
            .filter(|(name, entry)| {
                name.to_lowercase().contains(&query_lower)
                    || entry.description.to_lowercase().contains(&query_lower)
            })
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// Get an iterator over loaded extensions
    pub fn extensions(&self) -> impl Iterator<Item = (&String, &Extension)> {
        self.extensions.iter()
    }
}

impl Default for ExtensionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_ttl() {
        // Verify cache TTL is reasonable
        assert_eq!(CACHE_TTL.as_secs(), 3600);
    }

    #[test]
    fn test_github_urls() {
        // Verify URL construction
        let branch = "main";
        let expected_registry = format!(
            "{}/{}/{}/v3/registry.yaml",
            GITHUB_RAW_URL, GITHUB_REPO, branch
        );
        assert!(expected_registry.contains("raw.githubusercontent.com"));
        assert!(expected_registry.contains("/registry.yaml"));
    }
}
