//! Profile-based extension installation
//!
//! Profiles are defined in profiles.yaml and contain a curated set of extensions.
//! This module handles installing all extensions in a profile with proper
//! dependency resolution and progress tracking.

use anyhow::{anyhow, Context, Result};
use tracing::{debug, info, warn};

use crate::dependency::DependencyResolver;
use crate::executor::ExtensionExecutor;
use crate::manifest::ManifestManager;
use crate::registry::ExtensionRegistry;

/// Progress callback type for profile installations
pub type ProgressCallback<'a> = Option<&'a dyn Fn(usize, usize, &str)>;

/// Result of a profile installation
#[derive(Debug)]
pub struct ProfileInstallResult {
    /// Number of successfully installed extensions
    pub installed_count: usize,

    /// Number of failed extensions
    pub failed_count: usize,

    /// Names of failed extensions
    pub failed_extensions: Vec<String>,

    /// Total number of extensions attempted
    pub total_count: usize,
}

impl ProfileInstallResult {
    /// Check if installation was completely successful
    pub fn is_success(&self) -> bool {
        self.failed_count == 0
    }

    /// Check if installation was partial (some succeeded, some failed)
    pub fn is_partial(&self) -> bool {
        self.installed_count > 0 && self.failed_count > 0
    }
}

/// Profile installer for batch extension installation
pub struct ProfileInstaller {
    registry: ExtensionRegistry,
    executor: ExtensionExecutor,
    manifest: ManifestManager,
}

impl ProfileInstaller {
    /// Create a new profile installer
    pub fn new(
        registry: ExtensionRegistry,
        executor: ExtensionExecutor,
        manifest: ManifestManager,
    ) -> Self {
        Self {
            registry,
            executor,
            manifest,
        }
    }

    /// Install all extensions in a profile
    ///
    /// This method:
    /// 1. Validates the profile exists
    /// 2. Gets the list of extensions from the profile
    /// 3. Resolves all dependencies for all extensions
    /// 4. Installs protected base extensions first
    /// 5. Installs remaining extensions in dependency order
    /// 6. Continues on failures (doesn't stop entire installation)
    /// 7. Returns result with success/failure counts
    pub async fn install_profile(
        &mut self,
        profile_name: &str,
        progress_callback: ProgressCallback<'_>,
    ) -> Result<ProfileInstallResult> {
        info!("Installing profile: {}", profile_name);

        // Step 1: Validate profile exists and get extensions list
        let profile_extensions = {
            let profile = self
                .registry
                .get_profile(profile_name)
                .ok_or_else(|| anyhow!("Unknown profile: {}", profile_name))?;

            if profile.extensions.is_empty() {
                return Err(anyhow!(
                    "Profile '{}' has no extensions defined",
                    profile_name
                ));
            }

            debug!(
                "Profile '{}' contains {} extensions",
                profile_name,
                profile.extensions.len()
            );

            // Clone the extensions list to avoid borrowing issues
            profile.extensions.clone()
        };

        // Step 2: Load extension definitions for dependency resolution
        self.load_extension_definitions(&profile_extensions).await?;

        // Step 3: Resolve dependencies for all extensions
        let resolver = DependencyResolver::new(&self.registry);
        let mut all_extensions = Vec::new();

        for ext_name in &profile_extensions {
            let resolved = resolver
                .resolve(ext_name)
                .context(format!("Failed to resolve dependencies for {}", ext_name))?;
            for resolved_ext in resolved {
                if !all_extensions.contains(&resolved_ext) {
                    all_extensions.push(resolved_ext);
                }
            }
        }

        debug!(
            "Total extensions to install (with deps): {}",
            all_extensions.len()
        );

        // Step 4: Separate protected and regular extensions
        let (protected_exts, regular_exts): (Vec<_>, Vec<_>) = all_extensions
            .into_iter()
            .partition(|name| self.registry.is_protected(name));

        // Track results
        let mut installed_count = 0;
        let mut failed_count = 0;
        let mut failed_extensions = Vec::new();

        let total_count = protected_exts.len() + regular_exts.len();
        let mut current = 0;

        // Step 5: Install protected base extensions first
        for ext_name in &protected_exts {
            current += 1;

            // Skip if already installed (unless force)
            if self.manifest.is_installed(ext_name) {
                debug!("Base extension {} already installed, skipping", ext_name);
                installed_count += 1;
                if let Some(callback) = progress_callback {
                    callback(current, total_count, ext_name);
                }
                continue;
            }

            info!(
                "Installing base extension: {} ({}/{})",
                ext_name, current, total_count
            );
            if let Some(callback) = progress_callback {
                callback(current, total_count, ext_name);
            }

            match self.install_single_extension(ext_name, true).await {
                Ok(_) => {
                    installed_count += 1;
                    info!("Base extension {} installed successfully", ext_name);
                }
                Err(e) => {
                    failed_count += 1;
                    failed_extensions.push(ext_name.clone());
                    warn!("Base extension {} failed: {} (continuing...)", ext_name, e);
                }
            }
        }

        // Step 6: Install regular extensions in dependency order
        for ext_name in &regular_exts {
            current += 1;

            // Skip if already installed
            if self.manifest.is_installed(ext_name) {
                debug!("Extension {} already installed, skipping", ext_name);
                installed_count += 1;
                if let Some(callback) = progress_callback {
                    callback(current, total_count, ext_name);
                }
                continue;
            }

            info!(
                "Installing extension: {} ({}/{})",
                ext_name, current, total_count
            );
            if let Some(callback) = progress_callback {
                callback(current, total_count, ext_name);
            }

            match self.install_single_extension(ext_name, false).await {
                Ok(_) => {
                    installed_count += 1;
                    info!("Extension {} installed successfully", ext_name);
                }
                Err(e) => {
                    failed_count += 1;
                    failed_extensions.push(ext_name.clone());
                    warn!("Extension {} failed: {} (continuing...)", ext_name, e);
                }
            }
        }

        // Step 7: Return result
        let result = ProfileInstallResult {
            installed_count,
            failed_count,
            failed_extensions,
            total_count,
        };

        if result.is_success() {
            info!(
                "Profile '{}' installed successfully ({} extensions)",
                profile_name, installed_count
            );
        } else if result.is_partial() {
            warn!(
                "Profile '{}' partially installed: {} succeeded, {} failed",
                profile_name, installed_count, failed_count
            );
        } else {
            warn!(
                "Profile '{}' installation failed: all {} extensions failed",
                profile_name, failed_count
            );
        }

        Ok(result)
    }

    /// Reinstall all extensions in a profile
    ///
    /// This removes and reinstalls all extensions in the profile,
    /// even if they're already installed.
    pub async fn reinstall_profile(
        &mut self,
        profile_name: &str,
        progress_callback: ProgressCallback<'_>,
    ) -> Result<ProfileInstallResult> {
        info!("Reinstalling profile: {}", profile_name);

        // Step 1: Validate profile exists and get extensions list
        let profile_extensions = {
            let profile = self
                .registry
                .get_profile(profile_name)
                .ok_or_else(|| anyhow!("Unknown profile: {}", profile_name))?;

            if profile.extensions.is_empty() {
                return Err(anyhow!(
                    "Profile '{}' has no extensions defined",
                    profile_name
                ));
            }

            // Clone the extensions list to avoid borrowing issues
            profile.extensions.clone()
        };

        // Step 2: Load extension definitions for dependency resolution
        self.load_extension_definitions(&profile_extensions).await?;

        // Step 3: Resolve dependencies
        let resolver = DependencyResolver::new(&self.registry);
        let mut all_extensions = Vec::new();

        for ext_name in &profile_extensions {
            let resolved = resolver
                .resolve(ext_name)
                .context(format!("Failed to resolve dependencies for {}", ext_name))?;
            for resolved_ext in resolved {
                if !all_extensions.contains(&resolved_ext) {
                    all_extensions.push(resolved_ext);
                }
            }
        }

        // Step 4: Remove all extensions first (in reverse order to handle dependencies)
        info!("Removing existing extensions...");
        for ext_name in all_extensions.iter().rev() {
            if self.manifest.is_installed(ext_name) {
                debug!("Removing extension: {}", ext_name);

                // Execute removal process if extension defines removal configuration
                if let Some(extension) = self.registry.get_extension(ext_name) {
                    if extension.remove.is_some() {
                        // Log that full removal is being performed
                        info!("Extension {} defines removal configuration", ext_name);
                        // Note: Full removal execution would go here, but for reinstall
                        // we only need to clear the manifest. The reinstall will overwrite
                        // any files/configs. If full cleanup is needed in the future,
                        // implement execute_removal() similar to install().
                    }
                }

                // Mark as uninstalled in manifest
                if let Err(e) = self.manifest.mark_uninstalled(ext_name) {
                    warn!("Failed to mark {} as uninstalled: {}", ext_name, e);
                }

                // Note: Pre-remove/post-remove hooks are not currently part of the
                // extension schema. The schema only defines: pre-install, post-install,
                // pre-project-init, and post-project-init hooks.
                // If removal hooks are needed in the future, they should be added to:
                // 1. schemas/extension.schema.json (hooks section)
                // 2. sindri-core/src/types/extension_types.rs (HooksCapability struct)
                // 3. This module (execute_hook calls for pre_remove/post_remove)
            }
        }

        // Step 5: Install all extensions (reuse install_profile logic)
        // For simplicity, we'll just call install_profile since we've cleared the manifest
        self.install_profile(profile_name, progress_callback).await
    }

    /// Install a single extension with proper manifest tracking
    async fn install_single_extension(&mut self, name: &str, _is_protected: bool) -> Result<()> {
        // Mark as installing in manifest
        self.manifest
            .mark_installing(name, "installing", "github:pacphi/sindri")
            .context("Failed to update manifest")?;

        // Load extension definition if not already loaded
        if self.registry.get_extension(name).is_none() {
            return Err(anyhow!("Extension {} definition not loaded", name));
        }

        let extension = self
            .registry
            .get_extension(name)
            .ok_or_else(|| anyhow!("Extension {} not found in registry", name))?;

        // Execute installation
        let result = self.executor.install(extension).await;

        match result {
            Ok(_) => {
                // Validate installation
                let validation_result = self.executor.validate_extension(extension).await?;

                if validation_result {
                    // Mark as installed in manifest
                    let _category = self
                        .registry
                        .get_entry(name)
                        .map(|e| e.category.as_str())
                        .unwrap_or("unknown");

                    self.manifest
                        .mark_installed(name, &extension.metadata.version, "github:pacphi/sindri")
                        .context("Failed to update manifest")?;

                    Ok(())
                } else {
                    self.manifest.mark_failed(name)?;
                    Err(anyhow!("Extension {} failed validation", name))
                }
            }
            Err(e) => {
                self.manifest.mark_failed(name)?;
                Err(e)
            }
        }
    }

    /// Load extension definitions from registry
    ///
    /// Loads extension.yaml files for the given extensions. Tries in order:
    /// 1. Already loaded in registry (skip)
    /// 2. Local development path: v3/extensions/<name>/extension.yaml
    /// 3. Downloaded from GitHub releases: ~/.sindri/extensions/<name>/<version>/extension.yaml
    ///
    /// For production use, extensions should be downloaded via ExtensionDistributor first.
    /// For development, extensions can be loaded directly from v3/extensions/.
    async fn load_extension_definitions(&mut self, extension_names: &[String]) -> Result<()> {
        debug!(
            "Loading definitions for {} extensions",
            extension_names.len()
        );

        for name in extension_names {
            // Check that extension exists in registry metadata
            if !self.registry.has_extension(name) {
                return Err(anyhow!(
                    "Extension '{}' not found in registry. Available extensions: {:?}",
                    name,
                    self.registry.list_extensions()
                ));
            }

            // Skip if already loaded
            if self.registry.get_extension(name).is_some() {
                debug!("Extension '{}' already loaded, skipping", name);
                continue;
            }

            // Try loading from local development path (v3/extensions/<name>/extension.yaml)
            let dev_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent() // sindri-extensions -> crates
                .and_then(|p| p.parent()) // crates -> v3
                .map(|p| p.join("extensions").join(name).join("extension.yaml"));

            if let Some(dev_path) = dev_path {
                if dev_path.exists() {
                    debug!(
                        "Loading extension '{}' from development path: {:?}",
                        name, dev_path
                    );
                    self.registry
                        .load_extension(name, &dev_path)
                        .context(format!(
                            "Failed to load extension '{}' from {:?}",
                            name, dev_path
                        ))?;
                    continue;
                }
            }

            // If not in development mode, extension should have been downloaded
            // via ExtensionDistributor to ~/.sindri/extensions/<name>/<version>/extension.yaml
            // For now, return an error if we can't find it
            return Err(anyhow!(
                "Extension '{}' definition not loaded. \
                 For development, place extension.yaml at v3/extensions/{}/extension.yaml. \
                 For production, use ExtensionDistributor to download from GitHub releases.",
                name,
                name
            ));
        }

        Ok(())
    }

    /// List available profiles
    pub fn list_profiles(&self) -> Vec<(&str, &str)> {
        self.registry
            .profiles
            .iter()
            .map(|(name, profile)| (name.as_str(), profile.description.as_str()))
            .collect()
    }

    /// Get extensions in a profile
    pub fn get_profile_extensions(&self, profile_name: &str) -> Result<Vec<String>> {
        self.registry.get_profile_extensions(profile_name)
    }

    /// Check which extensions in a profile are already installed
    pub fn check_profile_status(&self, profile_name: &str) -> Result<ProfileStatus> {
        let extensions = self.registry.get_profile_extensions(profile_name)?;

        let installed: Vec<_> = extensions
            .iter()
            .filter(|name| self.manifest.is_installed(name))
            .cloned()
            .collect();

        let not_installed: Vec<_> = extensions
            .iter()
            .filter(|name| !self.manifest.is_installed(name))
            .cloned()
            .collect();

        Ok(ProfileStatus {
            profile_name: profile_name.to_string(),
            total_extensions: extensions.len(),
            installed_extensions: installed,
            not_installed_extensions: not_installed,
        })
    }
}

/// Status of a profile's extensions
#[derive(Debug)]
pub struct ProfileStatus {
    /// Profile name
    pub profile_name: String,

    /// Total number of extensions in profile
    pub total_extensions: usize,

    /// Extensions that are installed
    pub installed_extensions: Vec<String>,

    /// Extensions that are not installed
    pub not_installed_extensions: Vec<String>,
}

impl ProfileStatus {
    /// Check if profile is fully installed
    pub fn is_fully_installed(&self) -> bool {
        self.not_installed_extensions.is_empty()
    }

    /// Check if profile is partially installed
    pub fn is_partially_installed(&self) -> bool {
        !self.installed_extensions.is_empty() && !self.not_installed_extensions.is_empty()
    }

    /// Get percentage of installed extensions
    pub fn installed_percentage(&self) -> f64 {
        if self.total_extensions == 0 {
            0.0
        } else {
            (self.installed_extensions.len() as f64 / self.total_extensions as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_install_result() {
        let result = ProfileInstallResult {
            installed_count: 5,
            failed_count: 0,
            failed_extensions: vec![],
            total_count: 5,
        };

        assert!(result.is_success());
        assert!(!result.is_partial());

        let partial = ProfileInstallResult {
            installed_count: 3,
            failed_count: 2,
            failed_extensions: vec!["ext1".to_string(), "ext2".to_string()],
            total_count: 5,
        };

        assert!(!partial.is_success());
        assert!(partial.is_partial());
    }

    #[test]
    fn test_profile_status() {
        let status = ProfileStatus {
            profile_name: "minimal".to_string(),
            total_extensions: 5,
            installed_extensions: vec!["python".to_string(), "nodejs".to_string()],
            not_installed_extensions: vec![
                "golang".to_string(),
                "rust".to_string(),
                "docker".to_string(),
            ],
        };

        assert!(!status.is_fully_installed());
        assert!(status.is_partially_installed());
        assert_eq!(status.installed_percentage(), 40.0);
    }
}
