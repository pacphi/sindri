//! Profile-based extension installation
//!
//! Profiles are defined in profiles.yaml and contain a curated set of extensions.
//! This module handles installing all extensions in a profile with proper
//! dependency resolution and progress tracking.

use anyhow::{anyhow, Context, Result};
use sindri_core::types::ExtensionState;
use tracing::{debug, info, warn};

use crate::dependency::DependencyResolver;
use crate::events::{EventEnvelope, ExtensionEvent};
use crate::executor::ExtensionExecutor;
use crate::ledger::StatusLedger;
use crate::registry::ExtensionRegistry;
use crate::source::ExtensionSourceResolver;

/// Progress callback type for profile installations
pub type ProgressCallback<'a> = Option<&'a dyn Fn(usize, usize, &str)>;

/// Installation phase where an error occurred
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallPhase {
    /// Error during source resolution (finding extension)
    SourceResolution,
    /// Error during download from GitHub
    Download,
    /// Error during installation execution
    Install,
    /// Error during validation
    Validate,
}

impl std::fmt::Display for InstallPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallPhase::SourceResolution => write!(f, "Source Resolution"),
            InstallPhase::Download => write!(f, "Download"),
            InstallPhase::Install => write!(f, "Install"),
            InstallPhase::Validate => write!(f, "Validate"),
        }
    }
}

/// Information about a failed extension installation
#[derive(Debug, Clone)]
pub struct FailedExtension {
    /// Extension name
    pub name: String,
    /// Error message
    pub error: String,
    /// Phase where the error occurred
    pub phase: InstallPhase,
    /// Source type (bundled, downloaded, local-dev) if known
    pub source: Option<String>,
}

/// Successfully installed extension information
#[derive(Debug, Clone)]
pub struct InstalledExtension {
    /// Extension name
    pub name: String,
    /// Version installed
    pub version: String,
    /// Source type (bundled, downloaded, local-dev)
    pub source: String,
}

/// Result of a profile installation
#[derive(Debug)]
pub struct ProfileInstallResult {
    /// Successfully installed extensions with details
    pub installed_extensions: Vec<InstalledExtension>,

    /// Number of successfully installed extensions
    pub installed_count: usize,

    /// Number of failed extensions
    pub failed_count: usize,

    /// Failed extensions with error details
    pub failed_extensions: Vec<FailedExtension>,

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
    ledger: StatusLedger,
}

impl ProfileInstaller {
    /// Create a new profile installer
    pub fn new(
        registry: ExtensionRegistry,
        executor: ExtensionExecutor,
        ledger: StatusLedger,
    ) -> Self {
        Self {
            registry,
            executor,
            ledger,
        }
    }

    /// Check if an extension is installed via ledger
    fn is_installed(&self, name: &str) -> bool {
        self.ledger
            .get_all_latest_status()
            .ok()
            .and_then(|map| {
                map.get(name)
                    .map(|s| s.current_state == ExtensionState::Installed)
            })
            .unwrap_or(false)
    }

    /// Get version of an installed extension from ledger
    fn get_version(&self, name: &str) -> Option<String> {
        self.ledger
            .get_all_latest_status()
            .ok()
            .and_then(|map| map.get(name).and_then(|s| s.version.clone()))
    }

    /// Publish an event to the ledger (non-failing)
    fn publish_event(&self, envelope: EventEnvelope) {
        if let Err(e) = self.ledger.append(envelope) {
            warn!("Failed to publish event to ledger: {}", e);
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

        // Step 2: Load extension definitions for extensions explicitly listed in profile
        // This loads extension.yaml for each extension in the profile (e.g., nodejs, python)
        // so the DependencyResolver can read their metadata and discover dependencies
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

        // Step 3.5: Load extension definitions for ALL extensions including dependencies
        // This ensures dependencies not in the original profile (e.g., mise-config) get loaded.
        // load_extension_definitions() skips already-loaded extensions, so this is safe and idempotent.
        self.load_extension_definitions(&all_extensions).await?;

        // Step 4: Separate protected and regular extensions
        let (protected_exts, regular_exts): (Vec<_>, Vec<_>) = all_extensions
            .into_iter()
            .partition(|name| self.registry.is_protected(name));

        // Track results
        let mut installed_count = 0;
        let mut failed_count = 0;
        let mut failed_extensions = Vec::new();
        let mut installed_extensions = Vec::new();

        let total_count = protected_exts.len() + regular_exts.len();
        let mut current = 0;

        // Create source resolver to track where extensions come from
        let resolver = ExtensionSourceResolver::from_env()
            .context("Failed to create extension source resolver")?;

        // Step 5: Install protected base extensions first
        for ext_name in &protected_exts {
            current += 1;

            // Skip if already installed (unless force)
            if self.is_installed(ext_name) {
                debug!("Base extension {} already installed, skipping", ext_name);
                installed_count += 1;

                // Track as installed with version info
                if let Some(version) = self.get_version(ext_name) {
                    let source = resolver
                        .find_source(ext_name)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "unknown".to_string());

                    installed_extensions.push(InstalledExtension {
                        name: ext_name.clone(),
                        version: version.to_string(),
                        source,
                    });
                }

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

            match self
                .install_single_extension(ext_name, true, &resolver)
                .await
            {
                Ok((version, source)) => {
                    installed_count += 1;
                    info!("Base extension {} installed successfully", ext_name);

                    installed_extensions.push(InstalledExtension {
                        name: ext_name.clone(),
                        version,
                        source,
                    });
                }
                Err(e) => {
                    failed_count += 1;

                    // Determine phase and source from error
                    let (phase, error_msg) = classify_error(&e);
                    let source = resolver.find_source(ext_name).map(|s| s.to_string());

                    failed_extensions.push(FailedExtension {
                        name: ext_name.clone(),
                        error: error_msg,
                        phase,
                        source,
                    });

                    warn!("Base extension {} failed: {} (continuing...)", ext_name, e);
                }
            }
        }

        // Step 6: Install regular extensions in dependency order
        for ext_name in &regular_exts {
            current += 1;

            // Skip if already installed
            if self.is_installed(ext_name) {
                debug!("Extension {} already installed, skipping", ext_name);
                installed_count += 1;

                // Track as installed with version info
                if let Some(version) = self.get_version(ext_name) {
                    let source = resolver
                        .find_source(ext_name)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "unknown".to_string());

                    installed_extensions.push(InstalledExtension {
                        name: ext_name.clone(),
                        version: version.to_string(),
                        source,
                    });
                }

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

            match self
                .install_single_extension(ext_name, false, &resolver)
                .await
            {
                Ok((version, source)) => {
                    installed_count += 1;
                    info!("Extension {} installed successfully", ext_name);

                    installed_extensions.push(InstalledExtension {
                        name: ext_name.clone(),
                        version,
                        source,
                    });
                }
                Err(e) => {
                    failed_count += 1;

                    // Determine phase and source from error
                    let (phase, error_msg) = classify_error(&e);
                    let source = resolver.find_source(ext_name).map(|s| s.to_string());

                    failed_extensions.push(FailedExtension {
                        name: ext_name.clone(),
                        error: error_msg,
                        phase,
                        source,
                    });

                    warn!("Extension {} failed: {} (continuing...)", ext_name, e);
                }
            }
        }

        // Step 7: Return result
        let result = ProfileInstallResult {
            installed_extensions,
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

        // Step 2: Load extension definitions for extensions explicitly listed in profile
        // This loads extension.yaml for each extension in the profile (e.g., nodejs, python)
        // so the DependencyResolver can read their metadata and discover dependencies
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

        // Step 3.5: Load extension definitions for ALL extensions including dependencies
        // This ensures dependencies not in the original profile (e.g., mise-config) get loaded.
        // load_extension_definitions() skips already-loaded extensions, so this is safe and idempotent.
        self.load_extension_definitions(&all_extensions).await?;

        // Step 4: Remove all extensions first (in reverse order to handle dependencies)
        info!("Removing existing extensions...");
        for ext_name in all_extensions.iter().rev() {
            if self.is_installed(ext_name) {
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

                // Publish removal event to ledger
                self.publish_event(EventEnvelope::new(
                    ext_name.clone(),
                    Some(ExtensionState::Installed),
                    ExtensionState::Removing,
                    ExtensionEvent::RemoveCompleted {
                        extension_name: ext_name.clone(),
                        version: self.get_version(ext_name).unwrap_or_default(),
                        duration_secs: 0,
                    },
                ));

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
    ///
    /// Returns (version, source) on success
    async fn install_single_extension(
        &mut self,
        name: &str,
        _is_protected: bool,
        resolver: &ExtensionSourceResolver,
    ) -> Result<(String, String)> {
        // Determine source before installation
        let source_type = resolver
            .find_source(name)
            .map(|s| s.to_string())
            .unwrap_or_else(|| "github".to_string());

        // Publish InstallStarted event
        self.publish_event(EventEnvelope::new(
            name.to_string(),
            None,
            ExtensionState::Installing,
            ExtensionEvent::InstallStarted {
                extension_name: name.to_string(),
                version: "installing".to_string(),
                source: source_type.clone(),
                install_method: "Profile".to_string(),
            },
        ));

        let start_time = std::time::Instant::now();

        // Load extension definition if not already loaded
        if self.registry.get_extension(name).is_none() {
            return Err(anyhow!("Extension {} definition not loaded", name));
        }

        let extension = self
            .registry
            .get_extension(name)
            .ok_or_else(|| anyhow!("Extension {} not found in registry", name))?;

        let version = extension.metadata.version.clone();

        // Execute installation
        let result = self.executor.install(extension).await;
        let duration_secs = start_time.elapsed().as_secs();

        match result {
            Ok(_) => {
                // Validate installation
                let validation_result = self.executor.validate_extension(extension).await?;

                if validation_result {
                    // Publish InstallCompleted event
                    self.publish_event(EventEnvelope::new(
                        name.to_string(),
                        Some(ExtensionState::Installing),
                        ExtensionState::Installed,
                        ExtensionEvent::InstallCompleted {
                            extension_name: name.to_string(),
                            version: version.clone(),
                            duration_secs,
                            components_installed: vec![],
                        },
                    ));

                    Ok((version, source_type))
                } else {
                    // Publish InstallFailed event (validation failure)
                    self.publish_event(EventEnvelope::new(
                        name.to_string(),
                        Some(ExtensionState::Installing),
                        ExtensionState::Failed,
                        ExtensionEvent::InstallFailed {
                            extension_name: name.to_string(),
                            version: version.clone(),
                            error_message: "Validation failed".to_string(),
                            retry_count: 0,
                            duration_secs,
                        },
                    ));
                    Err(anyhow!("Extension {} failed validation", name))
                }
            }
            Err(e) => {
                // Publish InstallFailed event
                self.publish_event(EventEnvelope::new(
                    name.to_string(),
                    Some(ExtensionState::Installing),
                    ExtensionState::Failed,
                    ExtensionEvent::InstallFailed {
                        extension_name: name.to_string(),
                        version: version.clone(),
                        error_message: e.to_string(),
                        retry_count: 0,
                        duration_secs,
                    },
                ));
                Err(e)
            }
        }
    }

    /// Load extension definitions using ExtensionSourceResolver
    ///
    /// Uses the unified source resolver to load extensions from:
    /// 1. Bundled source (/opt/sindri/extensions) - for Docker builds
    /// 2. Local dev source (v3/extensions) - for development
    /// 3. Downloaded source (~/.sindri/extensions) - with GitHub download fallback
    async fn load_extension_definitions(&mut self, extension_names: &[String]) -> Result<()> {
        debug!(
            "Loading definitions for {} extensions",
            extension_names.len()
        );

        // Create source resolver from environment
        let resolver = ExtensionSourceResolver::from_env()
            .context("Failed to create extension source resolver")?;

        if resolver.is_bundled_mode() {
            debug!("Running in bundled mode");
        } else if resolver.is_dev_mode() {
            debug!("Running in development mode");
        } else {
            debug!("Running in production mode (download fallback enabled)");
        }

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

            // Use the resolver to get extension (handles bundled/local/download automatically)
            let extension = resolver
                .get_extension(name)
                .await
                .context(format!("Failed to load extension '{}'", name))?;

            // Register the extension in the registry
            self.registry.register_extension(name, extension);

            if let Some(source_type) = resolver.find_source(name) {
                debug!("Loaded extension '{}' from {} source", name, source_type);
            }
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
            .filter(|name| self.is_installed(name))
            .cloned()
            .collect();

        let not_installed: Vec<_> = extensions
            .iter()
            .filter(|name| !self.is_installed(name))
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

/// Classify an error to determine the installation phase where it occurred
fn classify_error(error: &anyhow::Error) -> (InstallPhase, String) {
    let error_str = error.to_string().to_lowercase();

    let phase = if error_str.contains("not found") || error_str.contains("definition not loaded") {
        InstallPhase::SourceResolution
    } else if error_str.contains("download")
        || error_str.contains("fetch")
        || error_str.contains("github")
    {
        InstallPhase::Download
    } else if error_str.contains("validation") || error_str.contains("validate") {
        InstallPhase::Validate
    } else {
        InstallPhase::Install
    };

    (phase, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_install_result() {
        let result = ProfileInstallResult {
            installed_extensions: vec![
                InstalledExtension {
                    name: "ext1".to_string(),
                    version: "1.0.0".to_string(),
                    source: "bundled".to_string(),
                },
                InstalledExtension {
                    name: "ext2".to_string(),
                    version: "2.0.0".to_string(),
                    source: "downloaded".to_string(),
                },
            ],
            installed_count: 5,
            failed_count: 0,
            failed_extensions: vec![],
            total_count: 5,
        };

        assert!(result.is_success());
        assert!(!result.is_partial());

        let partial = ProfileInstallResult {
            installed_extensions: vec![InstalledExtension {
                name: "ext1".to_string(),
                version: "1.0.0".to_string(),
                source: "bundled".to_string(),
            }],
            installed_count: 3,
            failed_count: 2,
            failed_extensions: vec![
                FailedExtension {
                    name: "ext2".to_string(),
                    error: "Installation failed".to_string(),
                    phase: InstallPhase::Install,
                    source: Some("downloaded".to_string()),
                },
                FailedExtension {
                    name: "ext3".to_string(),
                    error: "Validation failed".to_string(),
                    phase: InstallPhase::Validate,
                    source: Some("downloaded".to_string()),
                },
            ],
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
