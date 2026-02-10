//! Extension management commands
//!
//! Implements Phase 4 extension management CLI commands:
//! - install: Install an extension with optional version
//! - list: List available extensions with filtering
//! - validate: Validate extension against schema
//! - status: Show installation status
//! - info: Show detailed extension information
//! - upgrade: Upgrade extension to newer version
//! - remove: Remove an installed extension
//! - versions: Show available versions with compatibility
//! - check: Check for extension updates
//! - rollback: Rollback to previous version

use anyhow::{anyhow, Context, Result};
use camino::Utf8PathBuf;
use semver::Version;
use serde_json;
use tabled::{
    settings::{object::Columns, Modify, Style, Width},
    Table, Tabled,
};

use crate::cli::{
    ExtensionCheckArgs, ExtensionCommands, ExtensionDocsArgs, ExtensionInfoArgs,
    ExtensionInstallArgs, ExtensionListArgs, ExtensionRemoveArgs, ExtensionRollbackArgs,
    ExtensionStatusArgs, ExtensionUpgradeArgs, ExtensionValidateArgs, ExtensionVersionsArgs,
    UpdateSupportFilesArgs,
};
use crate::output;

// Re-export utility functions for use in this module
use crate::utils::{get_cache_dir, get_extensions_dir, get_home_dir};

/// Helper function to get the CLI version
fn get_cli_version() -> Result<semver::Version> {
    semver::Version::parse(env!("CARGO_PKG_VERSION")).context("Failed to parse CLI version")
}

/// Main entry point for extension subcommands
pub async fn run(cmd: ExtensionCommands) -> Result<()> {
    match cmd {
        ExtensionCommands::Install(args) => install(args).await,
        ExtensionCommands::List(args) => list(args).await,
        ExtensionCommands::Validate(args) => validate(args).await,
        ExtensionCommands::Status(args) => status(args).await,
        ExtensionCommands::Info(args) => info(args).await,
        ExtensionCommands::Upgrade(args) => upgrade(args).await,
        ExtensionCommands::Remove(args) => remove(args).await,
        ExtensionCommands::Versions(args) => versions(args).await,
        ExtensionCommands::Check(args) => check(args).await,
        ExtensionCommands::Rollback(args) => rollback(args).await,
        ExtensionCommands::UpdateSupportFiles(args) => update_support_files(args).await,
        ExtensionCommands::Docs(args) => docs(args).await,
    }
}

// ============================================================================
// Install Command
// ============================================================================

/// Install an extension with optional version specification
///
/// Supports three modes:
/// 1. Install by name: `sindri extension install python` or `python@1.1.0`
/// 2. Install from config: `sindri extension install --from-config sindri.yaml`
/// 3. Install from profile: `sindri extension install --profile minimal`
///
/// Options:
/// - Force reinstall: `--force`
/// - Skip dependencies: `--no-deps`
/// - Skip confirmation: `--yes` (for profile mode)
async fn install(args: ExtensionInstallArgs) -> Result<()> {
    match (&args.from_config, &args.profile, &args.name) {
        // Mode 1: From config file
        (Some(config_path), None, None) => {
            install_from_config(config_path.clone(), args.force, args.no_deps, args.yes).await
        }
        // Mode 2: From profile
        (None, Some(profile_name), None) => {
            install_from_profile(profile_name.clone(), args.yes).await
        }
        // Mode 3: By name
        (None, None, Some(name)) => {
            install_by_name(name.clone(), args.version, args.force, args.no_deps).await
        }
        // Error: No source specified
        (None, None, None) => Err(anyhow!(
            "Must specify extension name, --from-config, or --profile"
        )),
        // Defensive: multiple sources (clap should catch this)
        _ => Err(anyhow!("Conflicting options specified")),
    }
}

/// Install a single extension by name
async fn install_by_name(
    name: String,
    version: Option<String>,
    force: bool,
    no_deps: bool,
) -> Result<()> {
    // Parse name@version format if present
    let (name, version) = if name.contains('@') {
        let parts: Vec<&str> = name.split('@').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid format: {}. Use name@version", name));
        }
        (parts[0].to_string(), Some(parts[1].to_string()))
    } else {
        (name, version)
    };

    // Check if name is a known profile (if no version specified)
    // This helps users who might type "sindri extension install minimal" instead of "sindri profile install minimal"
    if version.is_none() {
        // Load profile names dynamically from registry
        let cache_dir = get_cache_dir()?;
        if let Ok(registry) =
            sindri_extensions::ExtensionRegistry::load_from_github(cache_dir, "main").await
        {
            let profile_names = registry.list_profiles();

            if profile_names.contains(&name.as_str()) {
                output::warning(&format!(
                    "'{}' looks like a profile name. Did you mean 'sindri profile install {}'?",
                    name, name
                ));
                output::info("Or use: sindri extension install --profile <profile-name>");
                return Err(anyhow!(
                    "Use 'sindri profile install {}' for profile installation",
                    name
                ));
            }
        }
    }

    output::info(&format!(
        "Installing extension: {}{}",
        name,
        version
            .as_ref()
            .map(|v| format!("@{}", v))
            .unwrap_or_default()
    ));

    if force {
        output::info("Force reinstall enabled");
    }

    if no_deps {
        output::warning("Skipping dependency installation");
    }

    // Get home directory for cache and extensions
    let home = get_home_dir()?;
    let cache_dir = home.join(".sindri").join("cache");
    let extensions_dir = home.join(".sindri").join("extensions");

    // Parse CLI version
    let cli_version =
        semver::Version::parse(env!("CARGO_PKG_VERSION")).context("Failed to parse CLI version")?;

    // Initialize distributor
    let distributor =
        sindri_extensions::ExtensionDistributor::new(cache_dir, extensions_dir, cli_version)?;

    // Create spinner
    let spinner = output::spinner("Installing extension...");

    // Install extension
    match distributor.install(&name, version.as_deref()).await {
        Ok(()) => {
            spinner.finish_and_clear();
            output::success(&format!(
                "Successfully installed {}{}",
                name,
                version
                    .as_ref()
                    .map(|v| format!("@{}", v))
                    .unwrap_or_default()
            ));
            Ok(())
        }
        Err(e) => {
            spinner.finish_and_clear();
            output::error(&format!("Failed to install {}: {}", name, e));
            Err(e)
        }
    }
}

/// Install extensions from a sindri.yaml config file
async fn install_from_config(
    config_path: Utf8PathBuf,
    force: bool,
    no_deps: bool,
    yes: bool,
) -> Result<()> {
    use dialoguer::Confirm;
    use sindri_core::config::SindriConfig;

    output::info(&format!("Loading configuration from: {}", config_path));
    let config = SindriConfig::load(Some(&config_path))?;
    let ext_config = config.extensions();

    // Case 1: Profile specified - install profile first, then additional extensions
    if let Some(profile_name) = &ext_config.profile {
        // Install the profile
        install_from_profile(profile_name.clone(), yes).await?;

        // Install additional extensions on top of profile (if any)
        if let Some(additional) = &ext_config.additional {
            if !additional.is_empty() {
                if !yes {
                    output::info(&format!(
                        "Installing {} additional extension(s) on top of profile '{}':",
                        additional.len(),
                        profile_name
                    ));
                    for ext in additional {
                        println!("  - {}", ext);
                    }
                    let confirmed = Confirm::new()
                        .with_prompt("Continue?")
                        .default(true)
                        .interact()?;
                    if !confirmed {
                        output::info("Cancelled additional installations");
                        return Ok(());
                    }
                }

                // Install each additional extension
                for ext in additional {
                    install_by_name(ext.clone(), None, force, no_deps).await?;
                }
            }
        }

        return Ok(());
    }

    // Case 2: Active list specified (no profile)
    // Validate: additional should ONLY work with profile
    if ext_config.additional.is_some() && !ext_config.additional.as_ref().unwrap().is_empty() {
        return Err(anyhow!(
            "Configuration error: 'extensions.additional' can only be used with 'extensions.profile'. \
             Use 'extensions.active' for explicit extension lists without a profile."
        ));
    }

    // Install from active list
    if let Some(active) = &ext_config.active {
        if active.is_empty() {
            output::warning("No extensions specified in config file");
            return Ok(());
        }

        // Confirmation prompt (unless --yes)
        if !yes {
            output::info(&format!("Installing {} extension(s):", active.len()));
            for ext in active {
                println!("  - {}", ext);
            }
            let confirmed = Confirm::new()
                .with_prompt("Continue?")
                .default(true)
                .interact()?;
            if !confirmed {
                output::info("Cancelled");
                return Ok(());
            }
        }

        // Install each extension
        for ext in active {
            install_by_name(ext.clone(), None, force, no_deps).await?;
        }

        return Ok(());
    }

    // Case 3: No profile, no active list
    output::warning("No extensions specified in config file");
    Ok(())
}

/// Install extensions from a profile (delegates to profile::install)
async fn install_from_profile(profile_name: String, yes: bool) -> Result<()> {
    use crate::cli::ProfileInstallArgs;
    use crate::commands::profile;

    let profile_args = ProfileInstallArgs {
        profile: profile_name,
        yes,
        continue_on_error: true,
    };
    profile::install(profile_args).await
}

// ============================================================================
// List Command
// ============================================================================

/// Extract software components from extension BOM
fn extract_software_list(extension: &sindri_core::types::Extension) -> String {
    if let Some(bom) = &extension.bom {
        let software_items: Vec<String> = bom
            .tools
            .iter()
            .map(|tool| {
                if let Some(version) = &tool.version {
                    format!("{} ({})", tool.name, version)
                } else {
                    tool.name.clone()
                }
            })
            .collect();

        if software_items.is_empty() {
            "-".to_string()
        } else {
            software_items.join(", ")
        }
    } else {
        "-".to_string()
    }
}

/// Fetch extensions in parallel with rate limiting
/// Returns a map of extension name -> (software_list, version)
async fn fetch_extensions_parallel(
    source_resolver: sindri_extensions::ExtensionSourceResolver,
    extension_names: Vec<String>,
    max_concurrent: usize,
) -> std::collections::HashMap<String, (String, Option<String>)> {
    use futures::stream::{FuturesUnordered, StreamExt};
    use std::sync::Arc;
    use tokio::sync::Semaphore;

    let mut results = std::collections::HashMap::new();
    let semaphore = Arc::new(Semaphore::new(max_concurrent));
    let source_resolver = Arc::new(source_resolver);

    // Create futures for all extensions
    let mut futures = FuturesUnordered::new();

    for name in extension_names {
        let sem = semaphore.clone();
        let resolver = source_resolver.clone();
        let name_clone = name.clone();

        futures.push(async move {
            // Acquire semaphore permit for rate limiting
            let _permit = sem.acquire().await.ok();

            // Try to get extension (downloads if not available locally)
            let result = match resolver.get_extension(&name_clone).await {
                Ok(extension) => {
                    let software = extract_software_list(&extension);
                    let version = Some(extension.metadata.version.clone());
                    (software, version)
                }
                Err(_) => {
                    // If fetch fails, return dash for software
                    ("-".to_string(), None)
                }
            };

            (name_clone, result)
        });
    }

    // Collect all results
    while let Some((name, data)) = futures.next().await {
        results.insert(name, data);
    }

    results
}

/// Row for available (not installed) extensions
#[derive(Tabled, serde::Serialize, serde::Deserialize)]
struct AvailableExtensionRow {
    name: String,
    category: String,
    #[tabled(rename = "available version")]
    available_version: String,
    #[tabled(rename = "software packaged")]
    software_packaged: String,
    description: String,
}

/// Row for installed extensions
#[derive(Tabled, serde::Serialize, serde::Deserialize)]
struct InstalledExtensionRow {
    name: String,
    category: String,
    #[tabled(rename = "installed version")]
    installed_version: String,
    #[tabled(rename = "installed software")]
    installed_software: String,
    description: String,
    #[tabled(rename = "install date")]
    install_date: String,
}

/// Row for all extensions (unified view with status)
#[derive(Tabled, serde::Serialize, serde::Deserialize)]
struct AllExtensionsRow {
    name: String,
    category: String,
    version: String,
    software: String,
    status: String,
    #[tabled(rename = "install date")]
    install_date: String,
    description: String,
}

/// List available extensions with optional filtering
///
/// Supports:
/// - List available (not installed): `sindri extension list`
/// - Filter by category: `sindri extension list --category language`
/// - Show installed only: `sindri extension list --installed`
/// - Show all extensions: `sindri extension list --all`
/// - JSON output: `sindri extension list --json`
async fn list(args: ExtensionListArgs) -> Result<()> {
    output::info(&format!(
        "Listing {}extensions{}",
        if args.all {
            "all "
        } else if args.installed {
            "installed "
        } else {
            "available "
        },
        args.category
            .as_ref()
            .map(|c| format!(" (category: {})", c))
            .unwrap_or_default()
    ));

    // Get home directory for cache and manifest
    let home = get_home_dir()?;
    let cache_dir = home.join(".sindri").join("cache");
    let manifest_path = home.join(".sindri").join("manifest.yaml");

    // Load registry from GitHub with caching
    let registry =
        sindri_extensions::ExtensionRegistry::load_from_github(cache_dir.clone(), "main")
            .await
            .context("Failed to load extension registry")?;

    // Load manifest to get installed versions
    let manifest = sindri_extensions::ManifestManager::new(manifest_path)
        .context("Failed to load manifest")?;

    // Initialize source resolver for loading extension definitions
    let source_resolver = sindri_extensions::ExtensionSourceResolver::from_env()
        .context("Failed to initialize extension source resolver")?;

    if args.all {
        // Show all extensions (both installed and available)
        let mut all_extensions: Vec<AllExtensionsRow> = Vec::new();

        // Initialize distributor for version resolution
        let extensions_dir = get_extensions_dir()?;
        let cli_version = get_cli_version()?;
        let distributor = sindri_extensions::ExtensionDistributor::new(
            cache_dir.clone(),
            extensions_dir,
            cli_version,
        )
        .context("Failed to initialize extension distributor")?;

        // Fetch compatibility matrix
        let matrix = distributor
            .get_compatibility_matrix()
            .await
            .context("Failed to fetch compatibility matrix")?;

        // Collect extension names (applying category filter)
        let extension_names: Vec<String> = registry
            .entries
            .iter()
            .filter(|(_, entry)| {
                args.category
                    .as_ref()
                    .map(|c| &entry.category == c)
                    .unwrap_or(true)
            })
            .map(|(name, _)| name.clone())
            .collect();

        // Fetch all extensions in parallel (max 10 concurrent downloads)
        let extension_data = fetch_extensions_parallel(source_resolver, extension_names, 10).await;

        for (name, entry) in registry.entries.iter() {
            // Filter by category if specified
            if let Some(category) = &args.category {
                if &entry.category != category {
                    continue;
                }
            }

            // Get software list and version from parallel fetch
            let (software_list, local_version) = extension_data
                .get(name)
                .cloned()
                .unwrap_or_else(|| ("-".to_string(), None));

            // Check if installed
            if let Some(installed_ext) = manifest.get_installed(name) {
                // Extension is installed
                all_extensions.push(AllExtensionsRow {
                    name: name.clone(),
                    category: entry.category.clone(),
                    version: installed_ext.version.clone(),
                    software: software_list,
                    status: "installed".to_string(),
                    install_date: installed_ext.status_datetime.format("%Y-%m-%d").to_string(),
                    description: entry.description.clone(),
                });
            } else {
                // Extension is available (not installed)
                // Get the available version
                let version = if let Some(v) = local_version {
                    v
                } else {
                    match distributor.get_compatible_range(&matrix, name) {
                        Ok(version_req) => {
                            match distributor.find_latest_compatible(name, &version_req).await {
                                Ok(ver) => ver.to_string(),
                                Err(_) => "latest".to_string(),
                            }
                        }
                        Err(_) => "latest".to_string(),
                    }
                };

                all_extensions.push(AllExtensionsRow {
                    name: name.clone(),
                    category: entry.category.clone(),
                    version,
                    software: software_list,
                    status: "available".to_string(),
                    install_date: "-".to_string(),
                    description: entry.description.clone(),
                });
            }
        }

        // Sort by status (installed first) then category then name
        all_extensions.sort_by(|a, b| {
            b.status
                .cmp(&a.status)
                .then(a.category.cmp(&b.category))
                .then(a.name.cmp(&b.name))
        });

        if args.json {
            let json = serde_json::to_string_pretty(&all_extensions)
                .context("Failed to serialize extensions to JSON")?;
            println!("{}", json);
        } else if all_extensions.is_empty() {
            output::warn("No extensions found matching criteria");
        } else {
            let mut table = Table::new(all_extensions);
            table.with(Style::sharp());

            // Set column-specific widths with wrapping
            table
                .with(Modify::new(Columns::new(3..4)).with(Width::wrap(50).keep_words(true))) // software
                .with(Modify::new(Columns::new(6..7)).with(Width::wrap(50).keep_words(true))); // description

            println!("{}", table);
        }
    } else if args.installed {
        // Show installed extensions only
        let mut installed_extensions: Vec<InstalledExtensionRow> = Vec::new();

        // Collect installed extension names (applying category filter)
        let installed_names: Vec<String> = registry
            .entries
            .iter()
            .filter(|(name, entry)| {
                manifest.is_installed(name)
                    && args
                        .category
                        .as_ref()
                        .map(|c| &entry.category == c)
                        .unwrap_or(true)
            })
            .map(|(name, _)| name.clone())
            .collect();

        // Fetch all installed extensions in parallel (max 10 concurrent)
        let extension_data = fetch_extensions_parallel(source_resolver, installed_names, 10).await;

        for (name, entry) in registry.entries.iter() {
            // Filter by category if specified
            if let Some(category) = &args.category {
                if &entry.category != category {
                    continue;
                }
            }

            // Get installed info if available
            if let Some(installed_ext) = manifest.get_installed(name) {
                // Get software list from parallel fetch
                let software_list = extension_data
                    .get(name)
                    .map(|(software, _)| software.clone())
                    .unwrap_or_else(|| "-".to_string());

                installed_extensions.push(InstalledExtensionRow {
                    name: name.clone(),
                    category: entry.category.clone(),
                    installed_version: installed_ext.version.clone(),
                    installed_software: software_list,
                    description: entry.description.clone(),
                    install_date: installed_ext.status_datetime.format("%Y-%m-%d").to_string(),
                });
            }
        }

        // Sort by category then name
        installed_extensions.sort_by(|a, b| a.category.cmp(&b.category).then(a.name.cmp(&b.name)));

        if args.json {
            let json = serde_json::to_string_pretty(&installed_extensions)
                .context("Failed to serialize extensions to JSON")?;
            println!("{}", json);
        } else if installed_extensions.is_empty() {
            output::warn("No installed extensions found matching criteria");
        } else {
            let mut table = Table::new(installed_extensions);
            table.with(Style::sharp());

            // Set column-specific widths with wrapping
            table
                .with(Modify::new(Columns::new(3..4)).with(Width::wrap(50).keep_words(true))) // installed software
                .with(Modify::new(Columns::new(4..5)).with(Width::wrap(50).keep_words(true))); // description

            println!("{}", table);
        }
    } else {
        // Show available (not installed) extensions only
        let mut available_extensions: Vec<AvailableExtensionRow> = Vec::new();

        // Initialize distributor to get compatibility info
        let extensions_dir = get_extensions_dir()?;
        let cli_version = get_cli_version()?;
        let distributor = sindri_extensions::ExtensionDistributor::new(
            cache_dir.clone(),
            extensions_dir,
            cli_version,
        )
        .context("Failed to initialize extension distributor")?;

        // Fetch compatibility matrix
        let matrix = distributor
            .get_compatibility_matrix()
            .await
            .context("Failed to fetch compatibility matrix")?;

        // Collect available (not installed) extension names (applying category filter)
        let available_names: Vec<String> = registry
            .entries
            .iter()
            .filter(|(name, entry)| {
                !manifest.is_installed(name)
                    && args
                        .category
                        .as_ref()
                        .map(|c| &entry.category == c)
                        .unwrap_or(true)
            })
            .map(|(name, _)| name.clone())
            .collect();

        // Fetch all available extensions in parallel (max 10 concurrent)
        let extension_data = fetch_extensions_parallel(source_resolver, available_names, 10).await;

        for (name, entry) in registry.entries.iter() {
            // Filter by category if specified
            if let Some(category) = &args.category {
                if &entry.category != category {
                    continue;
                }
            }

            // Only include if NOT installed
            if !manifest.is_installed(name) {
                // Get software list and version from parallel fetch
                let (software_list, local_version) = extension_data
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| ("-".to_string(), None));

                // Get the available version:
                // 1. If loaded locally, use the extension's metadata.version
                // 2. Otherwise, try to resolve from GitHub via compatibility matrix
                let available_version = if let Some(version) = local_version {
                    version
                } else {
                    match distributor.get_compatible_range(&matrix, name) {
                        Ok(version_req) => {
                            match distributor.find_latest_compatible(name, &version_req).await {
                                Ok(version) => version.to_string(),
                                Err(_) => "latest".to_string(),
                            }
                        }
                        Err(_) => "latest".to_string(),
                    }
                };

                available_extensions.push(AvailableExtensionRow {
                    name: name.clone(),
                    category: entry.category.clone(),
                    available_version,
                    software_packaged: software_list,
                    description: entry.description.clone(),
                });
            }
        }

        // Sort by category then name
        available_extensions.sort_by(|a, b| a.category.cmp(&b.category).then(a.name.cmp(&b.name)));

        if args.json {
            let json = serde_json::to_string_pretty(&available_extensions)
                .context("Failed to serialize extensions to JSON")?;
            println!("{}", json);
        } else if available_extensions.is_empty() {
            output::warn("No available extensions found matching criteria");
        } else {
            let mut table = Table::new(available_extensions);
            table.with(Style::sharp());

            // Set column-specific widths with wrapping
            table
                .with(Modify::new(Columns::new(3..4)).with(Width::wrap(50).keep_words(true))) // software packaged
                .with(Modify::new(Columns::new(4..5)).with(Width::wrap(50).keep_words(true))); // description

            println!("{}", table);
        }
    }

    Ok(())
}

// ============================================================================
// Validate Command
// ============================================================================

/// Validate an extension against JSON schema
///
/// Supports:
/// - Validate by name: `sindri extension validate python`
/// - Validate file: `sindri extension validate --file extension.yaml`
///
/// Performs full validation:
/// 1. Load extension.yaml (from file or registry)
/// 2. Validate against extension.schema.json
/// 3. Check all required fields
/// 4. Validate dependency references
/// 5. Check for conflicts with installed extensions
async fn validate(args: ExtensionValidateArgs) -> Result<()> {
    use sindri_extensions::{
        DependencyResolver, ExtensionRegistry, ExtensionValidator, ManifestManager,
    };
    use std::collections::HashSet;
    use tracing::debug;

    // Determine if we're validating a file or a registry extension
    let is_file = args.file.is_some() || {
        let path = std::path::Path::new(&args.name);
        path.exists() && path.is_file()
    };

    let validation_target = if let Some(file_path) = &args.file {
        format!("file: {}", file_path)
    } else if is_file {
        format!("file: {}", args.name)
    } else {
        format!("extension: {}", args.name)
    };

    output::info(&format!("Validating {}", validation_target));

    // Initialize schema validator
    let schema_validator = sindri_core::schema::SchemaValidator::new()
        .context("Failed to initialize schema validator")?;
    let extension_validator = ExtensionValidator::new(&schema_validator);

    // Load registry for dependency/conflict validation
    let cache_dir = get_cache_dir()?;
    let spinner = output::spinner("Loading extension registry...");
    let registry = ExtensionRegistry::load_from_github(cache_dir, "main")
        .await
        .context("Failed to load extension registry")?;
    spinner.finish_and_clear();

    // Load the extension to validate
    let extension = if is_file {
        // Validate from file
        let file_path = if let Some(fp) = &args.file {
            fp.as_std_path().to_path_buf()
        } else {
            std::path::PathBuf::from(&args.name)
        };

        debug!("Validating extension file: {:?}", file_path);
        extension_validator
            .validate_file(&file_path)
            .context("Schema and semantic validation failed")?
    } else {
        // Validate from registry - need to fetch extension definition
        let extensions_dir = get_extensions_dir()?;
        let ext_dir = extensions_dir.join(&args.name);

        // Check if extension exists in registry
        if !registry.has_extension(&args.name) {
            return Err(anyhow!(
                "Extension '{}' not found in registry. Use --file to validate a local file.",
                args.name
            ));
        }

        // Try to load from installed location first
        let extension_yaml = if let Ok(manifest) = ManifestManager::load_default() {
            if let Some(installed) = manifest.get_installed(&args.name) {
                let version_dir = ext_dir.join(&installed.version);
                let yaml_path = version_dir.join("extension.yaml");
                if yaml_path.exists() {
                    Some(yaml_path)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        if let Some(yaml_path) = extension_yaml {
            debug!("Validating installed extension from: {:?}", yaml_path);
            extension_validator
                .validate_file(&yaml_path)
                .context("Schema and semantic validation failed")?
        } else {
            // Extension not installed locally, validate registry entry only
            output::info(&format!(
                "Extension '{}' not installed locally. Validating registry metadata only.",
                args.name
            ));

            // We can still validate dependencies and conflicts from registry entry
            let entry = registry
                .get_entry(&args.name)
                .ok_or_else(|| anyhow!("Extension '{}' not found in registry", args.name))?;

            output::success(&format!(
                "Registry entry for '{}' is valid (category: {}, description: {})",
                args.name, entry.category, entry.description
            ));

            // Validate dependencies exist
            validate_dependencies_from_registry(&args.name, &entry.dependencies, &registry)?;

            // Check for conflicts with installed extensions
            validate_conflicts_from_registry(&args.name, &entry.conflicts)?;

            return Ok(());
        }
    };

    output::success("Schema and semantic validation passed");

    // Validate dependency references exist in registry
    output::info("Checking dependency references...");
    let mut missing_deps = Vec::new();
    for dep in &extension.metadata.dependencies {
        if !registry.has_extension(dep) {
            missing_deps.push(dep.clone());
        }
    }

    if !missing_deps.is_empty() {
        output::error(&format!(
            "Missing dependencies in registry: {}",
            missing_deps.join(", ")
        ));
        return Err(anyhow!(
            "Extension has dependencies not found in registry: {}",
            missing_deps.join(", ")
        ));
    }
    output::success("All dependencies exist in registry");

    // Check for circular dependencies using DependencyResolver
    output::info("Checking for circular dependencies...");

    // Build a temporary registry with this extension for cycle detection
    let mut temp_registry = ExtensionRegistry::new();
    temp_registry
        .extensions
        .insert(extension.metadata.name.clone(), extension.clone());

    // Add dependencies from the main registry entries
    for dep_name in &extension.metadata.dependencies {
        if let Some(entry) = registry.get_entry(dep_name) {
            // Create a minimal extension for dependency checking
            let dep_ext = sindri_core::types::Extension {
                metadata: sindri_core::types::ExtensionMetadata {
                    name: dep_name.clone(),
                    version: "1.0.0".to_string(),
                    description: entry.description.clone(),
                    category: sindri_core::types::ExtensionCategory::Devops,
                    author: None,
                    homepage: None,
                    dependencies: entry.dependencies.clone(),
                },
                requirements: None,
                install: sindri_core::types::InstallConfig {
                    method: sindri_core::types::InstallMethod::Script,
                    mise: None,
                    apt: None,
                    binary: None,
                    npm: None,
                    script: None,
                },
                configure: None,
                validate: sindri_core::types::ValidateConfig {
                    commands: vec![],
                    mise: None,
                },
                remove: None,
                upgrade: None,
                capabilities: None,
                docs: None,
                bom: None,
            };
            temp_registry.extensions.insert(dep_name.clone(), dep_ext);
        }
    }

    let resolver = DependencyResolver::new(&temp_registry);
    match resolver.resolve(&extension.metadata.name) {
        Ok(order) => {
            debug!("Dependency resolution order: {:?}", order);
            output::success("No circular dependencies detected");
        }
        Err(e) => {
            output::error(&format!("Circular dependency error: {}", e));
            return Err(e);
        }
    }

    // Check for conflicts with installed extensions
    output::info("Checking for conflicts with installed extensions...");
    let manifest = ManifestManager::load_default().context("Failed to load manifest")?;
    let installed: HashSet<String> = manifest
        .list_installed()
        .iter()
        .map(|(name, _)| name.to_string())
        .collect();

    // Get conflicts for this extension from registry
    let extension_conflicts = registry.get_conflicts(&extension.metadata.name);
    let mut active_conflicts = Vec::new();

    for conflict in &extension_conflicts {
        if installed.contains(conflict) {
            active_conflicts.push(conflict.clone());
        }
    }

    // Also check if any installed extension conflicts with this one
    for (installed_name, _) in manifest.list_installed() {
        let installed_conflicts = registry.get_conflicts(installed_name);
        if installed_conflicts.contains(&extension.metadata.name)
            && !active_conflicts.contains(&installed_name.to_string())
        {
            active_conflicts.push(installed_name.to_string());
        }
    }

    if !active_conflicts.is_empty() {
        output::warning(&format!(
            "Conflicts with installed extensions: {}",
            active_conflicts.join(", ")
        ));
        output::warning(
            "Installing this extension may cause issues with the conflicting extensions",
        );
    } else {
        output::success("No conflicts with installed extensions");
    }

    output::success(&format!(
        "Extension '{}' v{} is valid",
        extension.metadata.name, extension.metadata.version
    ));

    Ok(())
}

/// Helper to validate dependencies from registry entry
fn validate_dependencies_from_registry(
    name: &str,
    dependencies: &[String],
    registry: &sindri_extensions::ExtensionRegistry,
) -> Result<()> {
    use crate::output;

    if dependencies.is_empty() {
        output::success("No dependencies to validate");
        return Ok(());
    }

    output::info("Checking dependency references...");
    let mut missing = Vec::new();

    for dep in dependencies {
        if !registry.has_extension(dep) {
            missing.push(dep.clone());
        }
    }

    if !missing.is_empty() {
        output::error(&format!(
            "Missing dependencies in registry: {}",
            missing.join(", ")
        ));
        return Err(anyhow!(
            "Extension '{}' has dependencies not found in registry: {}",
            name,
            missing.join(", ")
        ));
    }

    output::success("All dependencies exist in registry");
    Ok(())
}

/// Helper to validate conflicts with installed extensions
fn validate_conflicts_from_registry(name: &str, conflicts: &[String]) -> Result<()> {
    use crate::output;
    use sindri_extensions::ManifestManager;
    use std::collections::HashSet;

    if conflicts.is_empty() {
        output::success("No conflicts defined");
        return Ok(());
    }

    output::info("Checking for conflicts with installed extensions...");

    let manifest = match ManifestManager::load_default() {
        Ok(m) => m,
        Err(_) => {
            output::info("No manifest found, skipping conflict check");
            return Ok(());
        }
    };

    let installed: HashSet<String> = manifest
        .list_installed()
        .iter()
        .map(|(n, _)| n.to_string())
        .collect();

    let active_conflicts: Vec<_> = conflicts
        .iter()
        .filter(|c| installed.contains(*c))
        .cloned()
        .collect();

    if !active_conflicts.is_empty() {
        output::warning(&format!(
            "Extension '{}' conflicts with installed: {}",
            name,
            active_conflicts.join(", ")
        ));
    } else {
        output::success("No conflicts with installed extensions");
    }

    Ok(())
}

// ============================================================================
// Status Command
// ============================================================================

#[derive(Tabled, serde::Serialize, serde::Deserialize)]
struct StatusRow {
    name: String,
    version: String,
    status: String,
    #[tabled(rename = "status date/time")]
    status_datetime: String,
}

/// Show installation status for extensions
///
/// Supports:
/// - Show all: `sindri extension status`
/// - Show specific: `sindri extension status python`
/// - JSON output: `sindri extension status --json`
async fn status(args: ExtensionStatusArgs) -> Result<()> {
    use sindri_core::types::ExtensionState;
    use sindri_extensions::{verify_extension_installed, ManifestManager};

    if let Some(name) = &args.name {
        output::info(&format!("Checking status of extension: {}", name));
    } else {
        output::info("Checking status of all installed extensions");
    }

    // Load manifest from ~/.sindri/state/manifest.yaml
    let manifest =
        ManifestManager::load_default().context("Failed to load installation manifest")?;

    let entries = manifest.list_all();

    // Filter by name if specified
    let entries: Vec<_> = if let Some(filter_name) = &args.name {
        entries
            .into_iter()
            .filter(|(name, _)| name == filter_name)
            .collect()
    } else {
        entries
    };

    if entries.is_empty() {
        if let Some(name) = &args.name {
            output::warning(&format!("Extension '{name}' is not installed"));
        } else {
            output::info("No extensions installed yet");
            output::info("Install extensions with: sindri extension install <name>");
        }
        return Ok(());
    }

    // Convert to status rows, verifying installed state
    let extensions_dir = get_extensions_dir()?;
    let mut statuses: Vec<StatusRow> = Vec::new();

    for (name, ext) in entries {
        // Determine actual status based on state and verification
        let status_str = match ext.state {
            // Non-installed states: show as-is
            ExtensionState::Failed => "failed".to_string(),
            ExtensionState::Installing => "installing".to_string(),
            ExtensionState::Outdated => "outdated".to_string(),
            ExtensionState::Removing => "removing".to_string(),

            // Installed state: verify it's actually installed
            ExtensionState::Installed => {
                let version_dir = extensions_dir.join(name).join(&ext.version);
                let yaml_path = version_dir.join("extension.yaml");

                if !yaml_path.exists() {
                    // No extension.yaml = no trace of installation
                    "not installed".to_string()
                } else {
                    // extension.yaml exists, check if software is present
                    match std::fs::read_to_string(&yaml_path) {
                        Ok(content) => {
                            match serde_yaml::from_str::<sindri_core::types::Extension>(&content) {
                                Ok(extension) => {
                                    let is_verified = verify_extension_installed(&extension).await;
                                    if is_verified {
                                        "installed".to_string()
                                    } else {
                                        // Installation attempted but software missing/broken
                                        "failed".to_string()
                                    }
                                }
                                Err(_) => {
                                    // Can't parse extension.yaml - corrupted
                                    "failed".to_string()
                                }
                            }
                        }
                        Err(_) => {
                            // Can't read extension.yaml
                            "failed".to_string()
                        }
                    }
                }
            }
        };

        // Only show timestamp for statuses that represent actual states
        // "not installed" means the entry exists in manifest but files are missing
        let status_datetime_str = if status_str == "not installed" {
            String::new()
        } else {
            ext.status_datetime.format("%Y-%m-%d %H:%M").to_string()
        };

        statuses.push(StatusRow {
            name: name.to_string(),
            version: ext.version.clone(),
            status: status_str,
            status_datetime: status_datetime_str,
        });
    }

    if args.json {
        let json = serde_json::to_string_pretty(&statuses)
            .context("Failed to serialize status to JSON")?;
        println!("{}", json);
    } else {
        let mut table = Table::new(statuses);
        table.with(Style::sharp());
        println!("{}", table);
    }

    Ok(())
}

// ============================================================================
// Info Command
// ============================================================================

/// Show detailed information about an extension
///
/// Displays:
/// - Name, version, category
/// - Description
/// - Dependencies
/// - Installation method
/// - Source repository
/// - Installed version and timestamp
async fn info(args: ExtensionInfoArgs) -> Result<()> {
    use sindri_core::types::ExtensionState;
    use sindri_extensions::{verify_extension_installed, ExtensionRegistry, ManifestManager};

    // Load registry and manifest
    let cache_dir = get_cache_dir()?;
    let registry = ExtensionRegistry::load_from_github(cache_dir, "main")
        .await
        .context("Failed to load extension registry")?;

    let manifest = ManifestManager::load_default().context("Failed to load manifest")?;

    // Look up extension in registry
    let entry = registry
        .get_entry(&args.name)
        .ok_or_else(|| anyhow!("Extension '{}' not found in registry", args.name))?;

    // Get installed info from manifest
    let mut installed = manifest.get_installed(&args.name);

    // If manifest says installed, verify it's actually installed
    if let Some(ext) = &installed {
        if ext.state == ExtensionState::Installed {
            // Load extension definition to verify
            let extensions_dir = get_extensions_dir()?;
            let version_dir = extensions_dir.join(&args.name).join(&ext.version);
            let yaml_path = version_dir.join("extension.yaml");

            if yaml_path.exists() {
                let content =
                    std::fs::read_to_string(&yaml_path).context("Failed to read extension.yaml")?;
                if let Ok(extension) =
                    serde_yaml::from_str::<sindri_core::types::Extension>(&content)
                {
                    let is_verified = verify_extension_installed(&extension).await;
                    if !is_verified {
                        // Extension not actually installed, treat as not installed
                        installed = None;
                    }
                } else {
                    // Can't parse extension.yaml, treat as not installed
                    installed = None;
                }
            } else {
                // Extension definition missing, treat as not installed
                installed = None;
            }
        }
    }

    if args.json {
        // JSON output
        let json_output = serde_json::json!({
            "name": args.name,
            "category": entry.category,
            "description": entry.description,
            "dependencies": entry.dependencies,
            "conflicts": entry.conflicts,
            "protected": entry.protected,
            "installed": installed.map(|ext| serde_json::json!({
                "version": ext.version,
                "status_datetime": ext.status_datetime.to_rfc3339(),
                "source": ext.source,
                "state": format!("{:?}", ext.state),
            }))
        });
        println!("{}", serde_json::to_string_pretty(&json_output)?);
    } else {
        // Human-readable output
        output::header(&format!("Extension: {}", args.name));
        println!();

        output::kv("Category", &entry.category);
        output::kv("Description", &entry.description);

        if !entry.dependencies.is_empty() {
            output::kv("Dependencies", &entry.dependencies.join(", "));
        }

        if !entry.conflicts.is_empty() {
            output::kv("Conflicts", &entry.conflicts.join(", "));
        }

        if entry.protected {
            output::kv("Protected", "yes");
        }

        println!();

        if let Some(ext) = installed {
            output::header("Installation Status");
            println!();
            output::kv("Status", &format!("{:?}", ext.state));
            output::kv("Installed Version", &ext.version);
            output::kv(
                "Status Date/Time",
                &ext.status_datetime
                    .format("%Y-%m-%d %H:%M:%S UTC")
                    .to_string(),
            );
            output::kv("Source", &ext.source);

            // Note: Version comparison would require loading the full Extension definition
            // which includes the version field. Registry entries only contain metadata.
            // This is a placeholder for future implementation when extension versions are available.
        } else {
            output::header("Installation Status");
            println!();
            output::info("Not installed");
            println!("  Run 'sindri extension install {}' to install", args.name);
        }
    }

    Ok(())
}

// ============================================================================
// Upgrade Command
// ============================================================================

/// Upgrade an extension to a newer version
///
/// Supports:
/// - Upgrade to latest: `sindri extension upgrade python`
/// - Upgrade to specific: `sindri extension upgrade python --version 1.2.0`
/// - Skip confirmation: `sindri extension upgrade python -y`
async fn upgrade(args: ExtensionUpgradeArgs) -> Result<()> {
    use dialoguer::Confirm;
    use semver::Version;
    use sindri_extensions::{ExtensionDistributor, ManifestManager};

    output::info(&format!("Upgrading extension: {}", args.name));

    // 1. Initialize ExtensionDistributor
    let cache_dir = get_cache_dir()?;
    let extensions_dir = get_extensions_dir()?;
    let cli_version = get_cli_version()?;

    let distributor = ExtensionDistributor::new(cache_dir, extensions_dir, cli_version)
        .context("Failed to initialize extension distributor")?;

    // 2. Check current installed version from manifest
    let manifest = ManifestManager::load_default().context("Failed to load manifest")?;

    let current_version = manifest
        .get_version(&args.name)
        .ok_or_else(|| anyhow!("Extension '{}' is not installed", args.name))?;

    let current = Version::parse(current_version)
        .context(format!("Invalid current version: {}", current_version))?;

    // 3. Determine target version
    let target = if let Some(version_spec) = &args.version {
        // Use specified version
        output::info(&format!("Target version: {}", version_spec));
        Version::parse(version_spec).context(format!("Invalid version: {}", version_spec))?
    } else {
        // Find latest compatible version
        output::info("Finding latest compatible version...");

        // Get compatibility matrix to find latest
        let matrix = distributor
            .get_compatibility_matrix()
            .await
            .context("Failed to fetch compatibility matrix")?;

        let version_req = distributor
            .get_compatible_range(&matrix, &args.name)
            .context("Failed to determine compatible version range")?;

        distributor
            .find_latest_compatible(&args.name, &version_req)
            .await
            .context("Failed to find latest compatible version")?
    };

    // 4. Check if upgrade is needed
    if current >= target {
        output::success(&format!(
            "{} is already at version {} (target: {})",
            args.name, current, target
        ));
        return Ok(());
    }

    // 5. Show upgrade plan
    println!();
    output::header("Upgrade Plan");
    println!();
    output::kv("Extension", &args.name);
    output::kv("Current Version", &current.to_string());
    output::kv("Target Version", &target.to_string());
    println!();

    // 6. Prompt for confirmation (unless -y)
    if !args.yes {
        let confirmed = Confirm::new()
            .with_prompt("Proceed with upgrade?")
            .default(true)
            .interact()
            .context("Failed to get user confirmation")?;

        if !confirmed {
            output::info("Upgrade cancelled");
            return Ok(());
        }
    }

    // 7. Call distributor.upgrade()
    let spinner = output::spinner(&format!("Upgrading {} to {}", args.name, target));

    let result = if args.version.is_some() {
        // Install specific version
        distributor
            .install(&args.name, Some(&target.to_string()))
            .await
    } else {
        // Use upgrade method for latest
        distributor.upgrade(&args.name).await
    };

    spinner.finish_and_clear();

    match result {
        Ok(_) => {
            output::success(&format!(
                "Successfully upgraded {} from {} to {}",
                args.name, current, target
            ));
        }
        Err(e) => {
            output::error(&format!("Upgrade failed: {}", e));
            return Err(e);
        }
    }

    Ok(())
}

// ============================================================================
// Remove Command
// ============================================================================

/// Remove an installed extension
///
/// Supports:
/// - Remove with confirmation: `sindri extension remove python`
/// - Force remove: `sindri extension remove python -y`
/// - Force even with dependents: `sindri extension remove python --force`
async fn remove(args: ExtensionRemoveArgs) -> Result<()> {
    use dialoguer::Confirm;
    use sindri_extensions::ManifestManager;
    use std::collections::HashSet;

    output::info(&format!("Removing extension: {}", args.name));

    // 1. Load ManifestManager
    let mut manifest = ManifestManager::load_default().context("Failed to load manifest")?;

    // 2. Check if extension is installed
    if !manifest.is_installed(&args.name) {
        return Err(anyhow!("Extension '{}' is not installed", args.name));
    }

    let installed_ext = manifest
        .get_installed(&args.name)
        .ok_or_else(|| anyhow!("Extension '{}' not found in manifest", args.name))?;

    let extensions_dir = get_extensions_dir()?;
    let ext_version_dir = extensions_dir.join(&args.name).join(&installed_ext.version);

    let extension = if ext_version_dir.exists() {
        let ext_yaml = ext_version_dir.join("extension.yaml");
        if ext_yaml.exists() {
            let content =
                std::fs::read_to_string(&ext_yaml).context("Failed to read extension.yaml")?;
            Some(
                serde_yaml::from_str::<sindri_core::types::Extension>(&content)
                    .context("Failed to parse extension.yaml")?,
            )
        } else {
            None
        }
    } else {
        None
    };

    // 3. Check if other extensions depend on it (unless --force)
    if !args.force {
        let installed: HashSet<String> = manifest
            .list_installed()
            .iter()
            .map(|(name, _)| name.to_string())
            .collect();

        let dependents: Vec<String> = installed
            .iter()
            .filter(|&name| name != &args.name)
            .filter_map(|name| {
                let version = manifest.get_version(name)?;
                let ext_yaml = extensions_dir
                    .join(name)
                    .join(version)
                    .join("extension.yaml");
                if !ext_yaml.exists() {
                    return None;
                }
                let content = std::fs::read_to_string(&ext_yaml).ok()?;
                let ext: sindri_core::types::Extension = serde_yaml::from_str(&content).ok()?;
                if ext.metadata.dependencies.contains(&args.name) {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();

        if !dependents.is_empty() {
            output::warn(&format!(
                "The following extensions depend on '{}': {}",
                args.name,
                dependents.join(", ")
            ));
            return Err(anyhow!(
                "Cannot remove '{}' because other extensions depend on it. Use --force to remove anyway.",
                args.name
            ));
        }
    } else {
        output::warn("Force removal enabled (ignoring dependencies)");
    }

    // 4. Show what will be removed and prompt for confirmation (unless -y)
    output::info(&format!(
        "This will remove {} version {}",
        args.name, installed_ext.version
    ));

    if !args.yes {
        let confirmed = Confirm::new()
            .with_prompt(format!("Are you sure you want to remove '{}'?", args.name))
            .default(false)
            .interact()?;

        if !confirmed {
            output::info("Cancelled");
            return Ok(());
        }
    }

    // 5. Mark as "removing" in manifest
    manifest
        .mark_removing(&args.name)
        .context("Failed to mark extension as removing")?;

    // 6. Execute removal operations
    if let Some(ext) = &extension {
        if let Some(remove_config) = &ext.remove {
            output::info("Executing removal operations...");

            // Remove mise configuration
            if let Some(mise_remove) = &remove_config.mise {
                output::info("Removing mise configuration...");
                let home = get_home_dir()?;

                if mise_remove.remove_config {
                    let config_file = home
                        .join(".config/mise/conf.d")
                        .join(format!("{}.toml", args.name));

                    if config_file.exists() {
                        tokio::fs::remove_file(&config_file)
                            .await
                            .context("Failed to remove mise config")?;
                    }
                }

                if !mise_remove.tools.is_empty() {
                    output::info("Uninstalling mise tools...");
                    for tool in &mise_remove.tools {
                        let _ = tokio::process::Command::new("mise")
                            .arg("uninstall")
                            .arg(tool)
                            .output()
                            .await;
                    }
                }
            }

            // Remove apt packages
            if let Some(apt_remove) = &remove_config.apt {
                if !apt_remove.packages.is_empty() {
                    output::info("Removing apt packages...");
                    let needs_sudo = tokio::process::Command::new("whoami")
                        .output()
                        .await
                        .ok()
                        .and_then(|o| String::from_utf8(o.stdout).ok())
                        .map(|u| u.trim() != "root")
                        .unwrap_or(true);

                    let mut cmd = if needs_sudo {
                        let mut c = tokio::process::Command::new("sudo");
                        c.arg("apt-get");
                        c
                    } else {
                        tokio::process::Command::new("apt-get")
                    };

                    cmd.arg("remove")
                        .arg("-y")
                        .arg("-qq")
                        .args(&apt_remove.packages);

                    let _ = cmd.output().await;
                }
            }

            // Execute removal script
            if let Some(script_remove) = &remove_config.script {
                if let Some(script_path_str) = &script_remove.path {
                    output::info("Running removal script...");
                    let script_path = ext_version_dir.join(script_path_str);

                    if script_path.exists() {
                        let result = tokio::process::Command::new("bash")
                            .arg(&script_path)
                            .current_dir(&ext_version_dir)
                            .output()
                            .await
                            .context("Failed to execute removal script")?;

                        if !result.status.success() {
                            output::warn("Removal script failed, continuing anyway...");
                        }
                    }
                }
            }

            // Remove specified paths
            for path in &remove_config.paths {
                output::info(&format!("Removing path: {}", path));

                let expanded_path = if path.starts_with("~/") {
                    if let Ok(home) = get_home_dir() {
                        home.join(path.trim_start_matches("~/"))
                    } else {
                        std::path::PathBuf::from(path)
                    }
                } else {
                    std::path::PathBuf::from(path)
                };

                if expanded_path.exists() {
                    if expanded_path.is_dir() {
                        let _ = tokio::fs::remove_dir_all(&expanded_path).await;
                    } else {
                        let _ = tokio::fs::remove_file(&expanded_path).await;
                    }
                }
            }
        }
    }

    // 7. Remove from manifest
    manifest
        .remove(&args.name)
        .context("Failed to remove extension from manifest")?;

    // 8. Remove extension directory
    if ext_version_dir.exists() {
        tokio::fs::remove_dir_all(&ext_version_dir)
            .await
            .context("Failed to remove extension directory")?;
    }

    output::success(&format!("Successfully removed extension: {}", args.name));

    Ok(())
}

// ============================================================================
// Versions Command
// ============================================================================

#[derive(Tabled)]
struct VersionRow {
    version: String,
    compatible: String,
    status: String,
    released: String,
}

/// Show available versions for an extension
///
/// Displays version compatibility with current CLI version
/// and indicates which version is currently installed
///
/// Lists all available versions from GitHub releases, showing:
/// - Version number
/// - Compatibility with current CLI version
/// - Installation status (installed, latest, update available)
/// - Release date
async fn versions(args: ExtensionVersionsArgs) -> Result<()> {
    use semver::VersionReq;
    use sindri_extensions::{ExtensionDistributor, ExtensionRegistry, ManifestManager};

    output::info(&format!("Fetching versions for extension: {}", args.name));

    // 1. Initialize ExtensionDistributor
    let cache_dir = get_cache_dir()?;
    let extensions_dir = get_extensions_dir()?;
    let cli_version = get_cli_version()?;

    let distributor =
        ExtensionDistributor::new(cache_dir.clone(), extensions_dir, cli_version.clone())
            .context("Failed to initialize extension distributor")?;

    // 2. Verify extension exists in registry
    let spinner = output::spinner("Loading extension registry...");
    let registry = ExtensionRegistry::load_from_github(cache_dir, "main")
        .await
        .context("Failed to load extension registry")?;
    spinner.finish_and_clear();

    if !registry.has_extension(&args.name) {
        return Err(anyhow!("Extension '{}' not found in registry", args.name));
    }

    // 3. Fetch compatibility matrix and get compatible version range
    let spinner = output::spinner("Fetching compatibility information...");
    let matrix = distributor
        .get_compatibility_matrix()
        .await
        .context("Failed to fetch compatibility matrix")?;
    spinner.finish_and_clear();

    let compatible_range: Option<VersionReq> = {
        let cli_pattern = format!("{}.{}.x", cli_version.major, cli_version.minor);
        matrix
            .cli_versions
            .get(&cli_pattern)
            .and_then(|compat| compat.compatible_extensions.get(&args.name))
            .and_then(|range_str| VersionReq::parse(range_str).ok())
    };

    // 4. Get installed version from manifest
    let manifest = ManifestManager::load_default().context("Failed to load manifest")?;
    let installed_version = manifest.get_version(&args.name);

    // 5. Fetch all available versions from GitHub releases
    let spinner = output::spinner("Fetching available versions from GitHub...");
    let available_versions = distributor
        .list_available_versions(&args.name, compatible_range.as_ref())
        .await
        .context("Failed to fetch available versions")?;
    spinner.finish_and_clear();

    if available_versions.is_empty() {
        output::warn(&format!(
            "No versions found for extension '{}' in GitHub releases",
            args.name
        ));
        output::info("The extension may not have any published releases yet.");
        return Ok(());
    }

    // 6. Build version rows
    let mut version_rows: Vec<VersionRow> = Vec::new();
    let latest_version = available_versions.first().map(|(v, _, _)| v.to_string());

    for (version, released_at, is_compatible) in &available_versions {
        let version_str = version.to_string();
        let is_installed = installed_version.map(|v| v == version_str).unwrap_or(false);
        let is_latest = latest_version
            .as_ref()
            .map(|l| l == &version_str)
            .unwrap_or(false);

        let status = match (is_installed, is_latest) {
            (true, true) => "installed (latest)".to_string(),
            (true, false) => "installed".to_string(),
            (false, true) => "latest".to_string(),
            (false, false) => "-".to_string(),
        };

        version_rows.push(VersionRow {
            version: version_str,
            compatible: if *is_compatible {
                "yes".to_string()
            } else {
                "no".to_string()
            },
            status,
            released: released_at.format("%Y-%m-%d").to_string(),
        });
    }

    // 7. Output results
    if args.json {
        let json_output = serde_json::json!({
            "extension": args.name,
            "cli_version": cli_version.to_string(),
            "compatible_range": compatible_range.as_ref().map(|r| r.to_string()),
            "installed_version": installed_version,
            "latest_version": latest_version,
            "versions": version_rows.iter().map(|v| {
                serde_json::json!({
                    "version": v.version,
                    "compatible": v.compatible == "yes",
                    "status": v.status,
                    "released": v.released
                })
            }).collect::<Vec<_>>()
        });
        println!("{}", serde_json::to_string_pretty(&json_output)?);
    } else {
        println!();
        output::header(&format!("Available Versions: {}", args.name));
        println!();

        let mut table = Table::new(&version_rows);
        table.with(Style::sharp());
        println!("{}", table);

        println!();
        if let Some(range) = &compatible_range {
            output::info(&format!("Compatible range: {}", range));
        } else {
            output::warning("No compatibility information found for current CLI version");
        }
        output::info(&format!("Current CLI version: {}", cli_version));

        // Show upgrade hint if installed version is not the latest
        if let (Some(installed), Some(latest)) = (installed_version, &latest_version) {
            if installed != latest {
                let compatible_with_latest = version_rows
                    .iter()
                    .find(|v| &v.version == latest)
                    .map(|v| v.compatible == "yes")
                    .unwrap_or(false);

                if compatible_with_latest {
                    println!();
                    output::info(&format!("Upgrade available: {} -> {}", installed, latest));
                    output::info(&format!(
                        "Run 'sindri extension upgrade {}' to upgrade",
                        args.name
                    ));
                }
            }
        }
    }

    Ok(())
}

// ============================================================================
// Check Command
// ============================================================================

#[derive(Tabled, serde::Serialize)]
struct UpdateRow {
    name: String,
    current: String,
    available: String,
    status: String,
}

/// Check for available extension updates
///
/// Supports:
/// - Check all: `sindri extension check`
/// - Check specific: `sindri extension check python nodejs`
/// - JSON output: `sindri extension check --json`
async fn check(args: ExtensionCheckArgs) -> Result<()> {
    use sindri_extensions::{ExtensionDistributor, ExtensionRegistry, ManifestManager};

    if args.extensions.is_empty() {
        output::info("Checking for updates to all installed extensions");
    } else {
        output::info(&format!(
            "Checking for updates to: {}",
            args.extensions.join(", ")
        ));
    }

    // 1. Load manifest to get installed extensions
    let manifest = ManifestManager::load_default().context("Failed to load manifest")?;

    // 2. Get installed extensions (filter by specified names if provided)
    let installed: Vec<_> = if args.extensions.is_empty() {
        manifest.list_installed()
    } else {
        manifest
            .list_installed()
            .into_iter()
            .filter(|(name, _)| args.extensions.contains(&name.to_string()))
            .collect()
    };

    if installed.is_empty() {
        output::warning("No extensions installed");
        return Ok(());
    }

    // 3. Initialize ExtensionDistributor and Registry
    let cache_dir = get_cache_dir()?;
    let extensions_dir = get_extensions_dir()?;
    let cli_version = get_cli_version()?;

    let distributor = ExtensionDistributor::new(cache_dir.clone(), extensions_dir, cli_version)
        .context("Failed to initialize extension distributor")?;

    // Load registry to get latest versions
    let spinner = output::spinner("Fetching extension registry...");
    let registry = ExtensionRegistry::load_from_github(cache_dir, "main")
        .await
        .context("Failed to load extension registry")?;
    spinner.finish_and_clear();

    // 4. Check each extension for updates
    let mut updates = Vec::new();
    let spinner = output::spinner("Checking for updates...");

    // Get compatibility matrix for version checks
    let matrix = distributor
        .get_compatibility_matrix()
        .await
        .context("Failed to fetch compatibility matrix")?;

    for (name, ext) in installed {
        let current_version = Version::parse(&ext.version).context(format!(
            "Invalid version in manifest for {}: {}",
            name, ext.version
        ))?;

        // Check if extension exists in registry
        if registry.get_entry(name).is_none() {
            updates.push(UpdateRow {
                name: name.to_string(),
                current: current_version.to_string(),
                available: "-".to_string(),
                status: "not found in registry".to_string(),
            });
            continue;
        }

        // Get latest compatible version using distributor
        match distributor.get_compatible_range(&matrix, name) {
            Ok(version_req) => match distributor.find_latest_compatible(name, &version_req).await {
                Ok(latest_version) => {
                    let status = if latest_version > current_version {
                        "update available"
                    } else {
                        "up to date"
                    };

                    updates.push(UpdateRow {
                        name: name.to_string(),
                        current: current_version.to_string(),
                        available: latest_version.to_string(),
                        status: status.to_string(),
                    });
                }
                Err(e) => {
                    updates.push(UpdateRow {
                        name: name.to_string(),
                        current: current_version.to_string(),
                        available: "-".to_string(),
                        status: format!("error: {}", e),
                    });
                }
            },
            Err(e) => {
                updates.push(UpdateRow {
                    name: name.to_string(),
                    current: current_version.to_string(),
                    available: "-".to_string(),
                    status: format!("error: {}", e),
                });
            }
        }
    }

    spinner.finish_and_clear();

    // 5. Show results
    if args.json {
        let json_output =
            serde_json::to_string_pretty(&updates).context("Failed to serialize to JSON")?;
        println!("{}", json_output);
    } else {
        let mut table = Table::new(&updates);
        table.with(Style::sharp());
        println!("\n{}", table);

        let available_count = updates
            .iter()
            .filter(|u| u.status == "update available")
            .count();
        if available_count > 0 {
            output::info(&format!("{} update(s) available", available_count));
        } else {
            output::success("All extensions are up to date");
        }
    }

    Ok(())
}

// ============================================================================
// Rollback Command
// ============================================================================

/// Rollback an extension to a previous version
///
/// Restores the extension to its previous installed version
/// from the manifest history.
///
/// The rollback process:
/// 1. Checks if a previous version exists in the manifest history
/// 2. Confirms the rollback with the user (unless --yes is provided)
/// 3. Installs the previous version
/// 4. Updates the manifest to track the rollback
///
/// This follows the pattern from ADR-010 (GitHub Distribution) which specifies:
/// - Get current installed version from manifest
/// - Get previous version from version history
/// - Uninstall current, install previous
async fn rollback(args: ExtensionRollbackArgs) -> Result<()> {
    use dialoguer::Confirm;
    use semver::Version;
    use sindri_extensions::{ExtensionDistributor, ExtensionManifest};

    output::info(&format!("Rolling back extension: {}", args.name));

    // 1. Initialize ExtensionDistributor
    let cache_dir = get_cache_dir()?;
    let extensions_dir = get_extensions_dir()?;
    let cli_version = get_cli_version()?;

    let distributor = ExtensionDistributor::new(
        cache_dir.clone(),
        extensions_dir.clone(),
        cli_version.clone(),
    )
    .context("Failed to initialize extension distributor")?;

    // 2. Load manifest to get current version and version history
    let manifest_path = extensions_dir.parent().unwrap().join("manifest.yaml");

    if !manifest_path.exists() {
        return Err(anyhow!(
            "No manifest found. Extension '{}' may not be installed.",
            args.name
        ));
    }

    let manifest_content = tokio::fs::read_to_string(&manifest_path)
        .await
        .context("Failed to read manifest")?;

    let manifest: ExtensionManifest =
        serde_yaml::from_str(&manifest_content).context("Failed to parse manifest")?;

    let ext_entry = manifest.extensions.get(&args.name).ok_or_else(|| {
        anyhow!(
            "Extension '{}' is not installed. Cannot rollback.",
            args.name
        )
    })?;

    let current = Version::parse(&ext_entry.version)
        .context(format!("Invalid current version: {}", ext_entry.version))?;

    // 3. Check for previous version in history
    let previous_version_str = ext_entry.previous_versions.first().ok_or_else(|| {
        anyhow!(
            "No previous version available for '{}'. Rollback requires version history.",
            args.name
        )
    })?;

    let previous = Version::parse(previous_version_str).context(format!(
        "Invalid previous version: {}",
        previous_version_str
    ))?;

    // 4. Show rollback plan and confirm
    println!();
    output::header("Rollback Plan");
    println!();
    output::kv("Extension", &args.name);
    output::kv("Current Version", &current.to_string());
    output::kv("Rollback To", &previous.to_string());

    // Show version history if available
    if ext_entry.previous_versions.len() > 1 {
        println!();
        output::info("Version history:");
        for (i, v) in ext_entry.previous_versions.iter().enumerate() {
            if i == 0 {
                println!("  {} (rollback target)", v);
            } else {
                println!("  {}", v);
            }
        }
    }
    println!();

    // 5. Confirm with user (unless --yes)
    if !args.yes {
        let confirmed = Confirm::new()
            .with_prompt(format!(
                "Rollback {} from {} to {}?",
                args.name, current, previous
            ))
            .default(false)
            .interact()
            .context("Failed to get user confirmation")?;

        if !confirmed {
            output::info("Rollback cancelled");
            return Ok(());
        }
    }

    // 6. Perform rollback using distributor
    let spinner = output::spinner(&format!(
        "Rolling back {} from {} to {}...",
        args.name, current, previous
    ));

    // The distributor.rollback method handles:
    // - Finding the previous version in the extensions directory
    // - Updating the manifest to point to the previous version
    match distributor.rollback(&args.name).await {
        Ok(()) => {
            spinner.finish_and_clear();
            output::success(&format!(
                "Successfully rolled back {} from {} to {}",
                args.name, current, previous
            ));

            // Show hint about re-upgrading
            println!();
            output::info(&format!(
                "To upgrade back to {}, run: sindri extension upgrade {}",
                current, args.name
            ));
        }
        Err(e) => {
            spinner.finish_and_clear();

            // Check if the error is due to missing version directory
            let prev_version_dir = extensions_dir.join(&args.name).join(previous.to_string());
            if !prev_version_dir.exists() {
                output::error(&format!(
                    "Previous version {} is not available locally",
                    previous
                ));
                output::info(&format!(
                    "Try installing it explicitly: sindri extension install {}@{}",
                    args.name, previous
                ));
                return Err(anyhow!(
                    "Rollback failed: version {} not found locally. Use 'sindri extension install {}@{}' instead.",
                    previous, args.name, previous
                ));
            }

            output::error(&format!("Rollback failed: {}", e));
            return Err(e);
        }
    }

    Ok(())
}

// ============================================================================
// Update Support Files Command
// ============================================================================

/// Update support files (common.sh, compatibility-matrix.yaml, extension-source.yaml)
///
/// Fetches version-matched support files from GitHub or copies from bundled files.
/// Supports three modes:
/// 1. Normal: Check version, update if needed
/// 2. Force: Always update regardless of version
/// 3. Bundled: Use image-bundled files (offline mode)
///
/// # Arguments
/// * `args` - Command arguments (force, bundled, quiet)
///
/// # Examples
/// ```bash
/// # Update if version changed
/// sindri extension update-support-files
///
/// # Force update from GitHub
/// sindri extension update-support-files --force
///
/// # Use bundled files (offline)
/// sindri extension update-support-files --bundled
///
/// # Silent mode (for scripts)
/// sindri extension update-support-files --quiet
/// ```
async fn update_support_files(args: UpdateSupportFilesArgs) -> Result<()> {
    use sindri_extensions::SupportFileManager;

    if !args.quiet {
        output::info("Updating Sindri support files...");
    }

    // Initialize the support file manager
    let manager = SupportFileManager::new().context("Failed to initialize support file manager")?;

    // Execute update based on mode
    let result = if args.bundled {
        // Mode: Use bundled files (offline)
        if !args.quiet {
            output::info("Using bundled support files (offline mode)");
        }
        manager.update_from_bundled().await
    } else {
        // Mode: Fetch from GitHub (with version check unless --force)
        if !args.quiet && args.force {
            output::info("Force updating from GitHub");
        }

        match manager.update_all(args.force).await {
            Ok(true) => {
                // Files were updated
                if !args.quiet {
                    if let Ok(Some(metadata)) = manager.get_metadata().await {
                        output::success(&format!(
                            "Support files updated to version {} from {:?}",
                            metadata.cli_version, metadata.source
                        ));
                    } else {
                        output::success("Support files updated successfully");
                    }
                }
                Ok(())
            }
            Ok(false) => {
                // Files already up-to-date
                if !args.quiet {
                    output::success("Support files already up-to-date");
                }
                Ok(())
            }
            Err(e) => {
                // GitHub fetch failed, try bundled fallback
                if !args.quiet {
                    output::warning(&format!("GitHub fetch failed: {}", e));
                    output::info("Falling back to bundled support files...");
                }

                manager.update_from_bundled().await.map(|_| {
                    if !args.quiet {
                        output::success("Support files updated from bundled sources");
                    }
                })
            }
        }
    };

    // Handle final result
    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            if !args.quiet {
                output::error(&format!("Failed to update support files: {}", e));
            }
            Err(e)
        }
    }
}

// ============================================================================
// Docs Command
// ============================================================================

/// Generate documentation for an extension
///
/// Loads the extension.yaml and renders documentation to stdout using the
/// embedded Tera template.
///
/// Usage: `sindri extension docs golang`
async fn docs(args: ExtensionDocsArgs) -> Result<()> {
    // Try to find the extension.yaml file
    let extensions_dir = get_extensions_dir()?;

    // Try multiple locations: installed (versioned), installed (flat), source tree
    let extension_yaml_path = {
        let mut found = None;

        // 1. Flat structure (development/bundled mode)
        let flat_path = extensions_dir.join(&args.name).join("extension.yaml");
        if flat_path.exists() {
            found = Some(flat_path);
        }

        // 2. Try source tree locations (development mode)
        if found.is_none() {
            let source_paths = vec![
                std::path::PathBuf::from("extensions")
                    .join(&args.name)
                    .join("extension.yaml"),
                std::path::PathBuf::from("v3/extensions")
                    .join(&args.name)
                    .join("extension.yaml"),
                std::path::PathBuf::from("../extensions")
                    .join(&args.name)
                    .join("extension.yaml"),
            ];

            for path in source_paths {
                if path.exists() {
                    found = Some(path);
                    break;
                }
            }
        }

        // 3. Versioned structure (downloaded mode) - check manifest
        if found.is_none() {
            if let Ok(manifest) = sindri_extensions::ManifestManager::load_default() {
                if let Some(version) = manifest.get_version(&args.name) {
                    let versioned_path = extensions_dir
                        .join(&args.name)
                        .join(version)
                        .join("extension.yaml");
                    if versioned_path.exists() {
                        found = Some(versioned_path);
                    }
                }
            }
        }

        found.ok_or_else(|| {
            anyhow!(
                "Extension '{}' not found. Checked installed and source tree locations.",
                args.name
            )
        })?
    };

    // Load and parse the extension
    let content = std::fs::read_to_string(&extension_yaml_path)
        .with_context(|| format!("Failed to read {}", extension_yaml_path.display()))?;

    let extension: sindri_core::types::Extension = serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse {}", extension_yaml_path.display()))?;

    // Render documentation
    let doc = sindri_core::templates::render_extension_doc(&extension)
        .context("Failed to render extension documentation")?;

    print!("{}", doc);

    Ok(())
}

// ============================================================================
// Helper Functions
// ============================================================================
// Note: Helper functions are defined at the top of the file (lines 27-43)
