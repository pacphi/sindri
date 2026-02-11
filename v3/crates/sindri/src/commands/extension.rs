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
use sindri_core::types::ExtensionState;
use sindri_extensions::{EventEnvelope, ExtensionEvent, StatusLedger};
use std::time::Instant;
use tabled::{
    settings::{object::Columns, Modify, Style, Width},
    Table, Tabled,
};

use crate::cli::{
    ExtensionCheckArgs, ExtensionCommands, ExtensionDocsArgs, ExtensionInfoArgs,
    ExtensionInstallArgs, ExtensionListArgs, ExtensionRemoveArgs, ExtensionRollbackArgs,
    ExtensionStatusArgs, ExtensionUpgradeArgs, ExtensionValidateArgs, ExtensionVerifyArgs,
    ExtensionVersionsArgs, UpdateSupportFilesArgs,
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
        ExtensionCommands::Verify(args) => verify(args).await,
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

    // Initialize ledger for event tracking
    let ledger = StatusLedger::load_default().context("Failed to load status ledger")?;

    // Track start time
    let start_time = Instant::now();

    // Publish InstallStarted event
    let version_str = version.as_deref().unwrap_or("latest").to_string();
    let install_started_event = EventEnvelope::new(
        name.clone(),
        None,
        ExtensionState::Installing,
        ExtensionEvent::InstallStarted {
            extension_name: name.clone(),
            version: version_str.clone(),
            source: "github:pacphi/sindri".to_string(),
            install_method: "Distributor".to_string(),
        },
    );

    if let Err(e) = ledger.append(install_started_event) {
        output::warning(&format!("Failed to publish install started event: {}", e));
    }

    // Create spinner
    let spinner = output::spinner("Installing extension...");

    // Install extension
    let result = distributor.install(&name, version.as_deref()).await;
    let duration_secs = start_time.elapsed().as_secs();

    match result {
        Ok(()) => {
            spinner.finish_and_clear();

            // Publish InstallCompleted event
            let install_completed_event = EventEnvelope::new(
                name.clone(),
                Some(ExtensionState::Installing),
                ExtensionState::Installed,
                ExtensionEvent::InstallCompleted {
                    extension_name: name.clone(),
                    version: version_str.clone(),
                    duration_secs,
                    components_installed: vec![], // TODO: collect from executor
                },
            );

            if let Err(e) = ledger.append(install_completed_event) {
                output::warning(&format!("Failed to publish install completed event: {}", e));
            }

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

            // Publish InstallFailed event
            let install_failed_event = EventEnvelope::new(
                name.clone(),
                Some(ExtensionState::Installing),
                ExtensionState::Failed,
                ExtensionEvent::InstallFailed {
                    extension_name: name.clone(),
                    version: version_str.clone(),
                    error_message: e.to_string(),
                    retry_count: 0,
                    duration_secs,
                },
            );

            if let Err(ledger_err) = ledger.append(install_failed_event) {
                output::warning(&format!(
                    "Failed to publish install failed event: {}",
                    ledger_err
                ));
            }

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

    // Load registry from GitHub with caching
    let registry =
        sindri_extensions::ExtensionRegistry::load_from_github(cache_dir.clone(), "main")
            .await
            .context("Failed to load extension registry")?;

    // Load ledger to get installed extension status
    let ledger = StatusLedger::load_default().context("Failed to load status ledger")?;
    let status_map = ledger
        .get_all_latest_status()
        .context("Failed to get extension status")?;

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
            if let Some(installed_ext) = status_map
                .get(name)
                .filter(|s| s.current_state == ExtensionState::Installed)
            {
                // Extension is installed
                all_extensions.push(AllExtensionsRow {
                    name: name.clone(),
                    category: entry.category.clone(),
                    version: installed_ext
                        .version
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string()),
                    software: software_list,
                    status: "installed".to_string(),
                    install_date: installed_ext.last_event_time.format("%Y-%m-%d").to_string(),
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
                status_map
                    .get(*name)
                    .map(|s| s.current_state == ExtensionState::Installed)
                    .unwrap_or(false)
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
            if let Some(installed_ext) = status_map
                .get(name)
                .filter(|s| s.current_state == ExtensionState::Installed)
            {
                // Get software list from parallel fetch
                let software_list = extension_data
                    .get(name)
                    .map(|(software, _)| software.clone())
                    .unwrap_or_else(|| "-".to_string());

                installed_extensions.push(InstalledExtensionRow {
                    name: name.clone(),
                    category: entry.category.clone(),
                    installed_version: installed_ext
                        .version
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string()),
                    installed_software: software_list,
                    description: entry.description.clone(),
                    install_date: installed_ext.last_event_time.format("%Y-%m-%d").to_string(),
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
                !status_map
                    .get(*name)
                    .map(|s| s.current_state == ExtensionState::Installed)
                    .unwrap_or(false)
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
            if !status_map
                .get(name)
                .map(|s| s.current_state == ExtensionState::Installed)
                .unwrap_or(false)
            {
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
    use sindri_extensions::{DependencyResolver, ExtensionRegistry, ExtensionValidator};
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
        let extension_yaml = if let Ok(ledger) = StatusLedger::load_default() {
            if let Ok(status_map) = ledger.get_all_latest_status() {
                if let Some(status) = status_map
                    .get(&args.name)
                    .filter(|s| s.current_state == ExtensionState::Installed)
                {
                    if let Some(version) = &status.version {
                        let version_dir = ext_dir.join(version);
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
    let ledger = StatusLedger::load_default().context("Failed to load status ledger")?;
    let status_map = ledger
        .get_all_latest_status()
        .context("Failed to get extension status")?;
    let installed: HashSet<String> = status_map
        .iter()
        .filter(|(_, s)| s.current_state == ExtensionState::Installed)
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
    for installed_name in &installed {
        let installed_conflicts = registry.get_conflicts(installed_name);
        if installed_conflicts.contains(&extension.metadata.name)
            && !active_conflicts.contains(installed_name)
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
    use std::collections::HashSet;

    if conflicts.is_empty() {
        output::success("No conflicts defined");
        return Ok(());
    }

    output::info("Checking for conflicts with installed extensions...");

    let ledger = match StatusLedger::load_default() {
        Ok(l) => l,
        Err(_) => {
            output::info("No ledger found, skipping conflict check");
            return Ok(());
        }
    };

    let status_map = match ledger.get_all_latest_status() {
        Ok(m) => m,
        Err(_) => {
            output::info("Failed to get extension status, skipping conflict check");
            return Ok(());
        }
    };

    let installed: HashSet<String> = status_map
        .iter()
        .filter(|(_, status)| status.current_state == ExtensionState::Installed)
        .map(|(n, _)| n.clone())
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

/// Format an extension event into a human-readable summary string
fn format_event_summary(event: &ExtensionEvent) -> String {
    match event {
        ExtensionEvent::InstallStarted {
            version,
            install_method,
            ..
        } => format!("Install started (v{version}, method: {install_method})"),

        ExtensionEvent::InstallCompleted {
            version,
            duration_secs,
            ..
        } => format!("Install completed (v{version}, {duration_secs}s)"),

        ExtensionEvent::InstallFailed {
            version,
            error_message,
            duration_secs,
            ..
        } => format!("Install failed (v{version}, {duration_secs}s): {error_message}"),

        ExtensionEvent::UpgradeStarted {
            from_version,
            to_version,
            ..
        } => format!("Upgrade started ({from_version} \u{2192} {to_version})"),

        ExtensionEvent::UpgradeCompleted {
            from_version,
            to_version,
            duration_secs,
            ..
        } => format!("Upgrade completed ({from_version} \u{2192} {to_version}, {duration_secs}s)"),

        ExtensionEvent::UpgradeFailed {
            from_version,
            to_version,
            error_message,
            ..
        } => format!("Upgrade failed ({from_version} \u{2192} {to_version}): {error_message}"),

        ExtensionEvent::RemoveStarted { version, .. } => {
            format!("Remove started (v{version})")
        }

        ExtensionEvent::RemoveCompleted {
            version,
            duration_secs,
            ..
        } => format!("Remove completed (v{version}, {duration_secs}s)"),

        ExtensionEvent::RemoveFailed {
            version,
            error_message,
            ..
        } => format!("Remove failed (v{version}): {error_message}"),

        ExtensionEvent::OutdatedDetected {
            current_version,
            latest_version,
            ..
        } => format!("Outdated detected ({current_version} \u{2192} {latest_version})"),

        ExtensionEvent::ValidationSucceeded {
            version,
            validation_type,
            ..
        } => format!("Validation succeeded (v{version}, {validation_type})"),

        ExtensionEvent::ValidationFailed {
            version,
            validation_type,
            error_message,
            ..
        } => format!("Validation failed (v{version}, {validation_type}): {error_message}"),
    }
}

/// Show installation status for extensions
///
/// Supports:
/// - Show all: `sindri extension status`
/// - Show specific: `sindri extension status python`
/// - JSON output: `sindri extension status --json`
/// - Event history: `sindri extension status python --limit 10`
/// - Date filtering: `sindri extension status python --since 2026-02-10T00:00:00Z`
/// - Verification: `sindri extension status --verify` (slower, checks actual installation)
async fn status(args: ExtensionStatusArgs) -> Result<()> {
    if let Some(name) = &args.name {
        output::info(&format!("Checking status of extension: {}", name));
    } else {
        output::info("Checking status of all installed extensions");
    }

    // Load status from ledger
    let ledger = StatusLedger::load_default().context("Failed to load status ledger")?;
    let status_map = ledger
        .get_all_latest_status()
        .context("Failed to get extension status")?;

    // Filter by name if specified
    let entries: Vec<_> = if let Some(filter_name) = &args.name {
        status_map
            .iter()
            .filter(|(name, _)| *name == filter_name)
            .collect()
    } else {
        status_map.iter().collect()
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

    // Convert to status rows
    let mut statuses: Vec<StatusRow> = Vec::new();

    for (name, ext_status) in &entries {
        let version_str = ext_status.version.clone().unwrap_or_default();

        // Determine status string from ledger state
        let status_str = if args.verify && ext_status.current_state == ExtensionState::Installed {
            // --verify flag: run actual verification checks (slower)
            use sindri_extensions::{find_extension_yaml, verify_extension_installed};

            if let Some(yaml_path) = find_extension_yaml(name, &version_str) {
                match std::fs::read_to_string(&yaml_path) {
                    Ok(content) => {
                        match serde_yaml::from_str::<sindri_core::types::Extension>(&content) {
                            Ok(extension) => {
                                if verify_extension_installed(&extension).await {
                                    "installed (verified)".to_string()
                                } else {
                                    "failed (verification)".to_string()
                                }
                            }
                            Err(_) => "failed (parse error)".to_string(),
                        }
                    }
                    Err(_) => "failed (unreadable)".to_string(),
                }
            } else {
                "not installed".to_string()
            }
        } else {
            // Default: trust the ledger state (fast path, no I/O verification)
            match ext_status.current_state {
                ExtensionState::Installed => "installed".to_string(),
                ExtensionState::Failed => "failed".to_string(),
                ExtensionState::Installing => "installing".to_string(),
                ExtensionState::Outdated => "outdated".to_string(),
                ExtensionState::Removing => "removing".to_string(),
            }
        };

        let status_datetime_str = ext_status
            .last_event_time
            .format("%Y-%m-%d %H:%M")
            .to_string();

        statuses.push(StatusRow {
            name: name.to_string(),
            version: version_str,
            status: status_str,
            status_datetime: status_datetime_str,
        });
    }

    if args.json {
        let json = serde_json::to_string_pretty(&statuses)
            .context("Failed to serialize status to JSON")?;
        println!("{}", json);
    } else {
        let mut table = Table::new(&statuses);
        table.with(Style::sharp());
        println!("{}", table);
    }

    // Show event history for a single extension
    if let Some(name) = &args.name {
        // Parse --since filter if provided
        let since_filter = if let Some(since_str) = &args.since {
            let parsed = chrono::DateTime::parse_from_rfc3339(since_str).context(format!(
                "Invalid --since date '{}'. Use ISO 8601 format: 2026-02-10T00:00:00Z",
                since_str
            ))?;
            Some(parsed.with_timezone(&chrono::Utc))
        } else {
            None
        };

        let limit = args.limit.or(Some(20));
        let history = ledger
            .get_extension_history(name, limit)
            .context("Failed to get extension history")?;

        // Apply --since filter
        let history: Vec<_> = if let Some(since) = since_filter {
            history
                .into_iter()
                .filter(|e| e.timestamp >= since)
                .collect()
        } else {
            history
        };

        if history.is_empty() {
            output::info("No event history found");
        } else {
            println!();
            output::header(&format!("Event history for '{name}'"));

            for envelope in &history {
                let ts = envelope.timestamp.format("%Y-%m-%d %H:%M:%S UTC");
                let summary = format_event_summary(&envelope.event);
                println!("  [{ts}] {summary}");
            }

            println!();
            output::info(&format!("{} event(s) shown", history.len()));
        }
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
    use sindri_extensions::{verify_extension_installed, ExtensionRegistry};

    // Load registry and ledger
    let cache_dir = get_cache_dir()?;
    let registry = ExtensionRegistry::load_from_github(cache_dir, "main")
        .await
        .context("Failed to load extension registry")?;

    let ledger = StatusLedger::load_default().context("Failed to load status ledger")?;
    let status_map = ledger
        .get_all_latest_status()
        .context("Failed to get extension status")?;

    // Look up extension in registry
    let entry = registry
        .get_entry(&args.name)
        .ok_or_else(|| anyhow!("Extension '{}' not found in registry", args.name))?;

    // Get installed info from ledger
    let mut installed = status_map
        .get(&args.name)
        .filter(|s| s.current_state == ExtensionState::Installed)
        .cloned();

    // If ledger says installed, verify it's actually installed
    if let Some(ext_status) = &installed {
        if let Some(version) = &ext_status.version {
            // Load extension definition to verify
            let extensions_dir = get_extensions_dir()?;
            let version_dir = extensions_dir.join(&args.name).join(version);
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
        } else {
            // No version info, treat as not installed
            installed = None;
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
            "installed": installed.as_ref().map(|ext_status| serde_json::json!({
                "version": ext_status.version.clone().unwrap_or_default(),
                "status_datetime": ext_status.last_event_time.to_rfc3339(),
                "state": format!("{:?}", ext_status.current_state),
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

        if let Some(ext_status) = installed {
            output::header("Installation Status");
            println!();
            output::kv("Status", &format!("{:?}", ext_status.current_state));
            output::kv("Installed Version", &ext_status.version.unwrap_or_default());
            output::kv(
                "Status Date/Time",
                &ext_status
                    .last_event_time
                    .format("%Y-%m-%d %H:%M:%S UTC")
                    .to_string(),
            );

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
/// - Upgrade to specific: `sindri extension upgrade python --target-version 1.2.0`
/// - Skip confirmation: `sindri extension upgrade python -y`
async fn upgrade(args: ExtensionUpgradeArgs) -> Result<()> {
    use dialoguer::Confirm;
    use semver::Version;
    use sindri_extensions::ExtensionDistributor;

    output::info(&format!("Upgrading extension: {}", args.name));

    // 1. Initialize ExtensionDistributor
    let cache_dir = get_cache_dir()?;
    let extensions_dir = get_extensions_dir()?;
    let cli_version = get_cli_version()?;

    let distributor = ExtensionDistributor::new(cache_dir, extensions_dir, cli_version)
        .context("Failed to initialize extension distributor")?;

    // 2. Check current installed version from ledger
    let ledger = StatusLedger::load_default().context("Failed to load status ledger")?;
    let status_map = ledger
        .get_all_latest_status()
        .context("Failed to get extension status")?;

    let ext_status = status_map
        .get(&args.name)
        .filter(|s| s.current_state == ExtensionState::Installed)
        .ok_or_else(|| anyhow!("Extension '{}' is not installed", args.name))?;

    let current_version = ext_status
        .version
        .as_ref()
        .ok_or_else(|| anyhow!("Extension '{}' has no version information", args.name))?;

    let current = Version::parse(current_version)
        .context(format!("Invalid current version: {}", current_version))?;

    // 3. Determine target version
    let target = if let Some(version_spec) = &args.target_version {
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

    // 7. Initialize ledger for event tracking
    let ledger = StatusLedger::load_default().context("Failed to load status ledger")?;

    // Track start time
    let start_time = Instant::now();

    // Publish UpgradeStarted event
    let upgrade_started_event = EventEnvelope::new(
        args.name.clone(),
        Some(ExtensionState::Installed),
        ExtensionState::Installing,
        ExtensionEvent::UpgradeStarted {
            extension_name: args.name.clone(),
            from_version: current.to_string(),
            to_version: target.to_string(),
        },
    );

    if let Err(e) = ledger.append(upgrade_started_event) {
        output::warning(&format!("Failed to publish upgrade started event: {}", e));
    }

    // 8. Call distributor.upgrade()
    let spinner = output::spinner(&format!("Upgrading {} to {}", args.name, target));

    let result = if args.target_version.is_some() {
        // Install specific version
        distributor
            .install(&args.name, Some(&target.to_string()))
            .await
    } else {
        // Use upgrade method for latest
        distributor.upgrade(&args.name).await
    };

    spinner.finish_and_clear();

    let duration_secs = start_time.elapsed().as_secs();

    match result {
        Ok(_) => {
            // Publish UpgradeCompleted event
            let upgrade_completed_event = EventEnvelope::new(
                args.name.clone(),
                Some(ExtensionState::Installing),
                ExtensionState::Installed,
                ExtensionEvent::UpgradeCompleted {
                    extension_name: args.name.clone(),
                    from_version: current.to_string(),
                    to_version: target.to_string(),
                    duration_secs,
                },
            );

            if let Err(e) = ledger.append(upgrade_completed_event) {
                output::warning(&format!("Failed to publish upgrade completed event: {}", e));
            }

            output::success(&format!(
                "Successfully upgraded {} from {} to {}",
                args.name, current, target
            ));
        }
        Err(e) => {
            // Publish UpgradeFailed event
            let upgrade_failed_event = EventEnvelope::new(
                args.name.clone(),
                Some(ExtensionState::Installing),
                ExtensionState::Failed,
                ExtensionEvent::UpgradeFailed {
                    extension_name: args.name.clone(),
                    from_version: current.to_string(),
                    to_version: target.to_string(),
                    error_message: e.to_string(),
                    duration_secs,
                },
            );

            if let Err(ledger_err) = ledger.append(upgrade_failed_event) {
                output::warning(&format!(
                    "Failed to publish upgrade failed event: {}",
                    ledger_err
                ));
            }

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
    use std::collections::HashSet;

    output::info(&format!("Removing extension: {}", args.name));

    // 1. Load ledger to check installation status
    let ledger = StatusLedger::load_default().context("Failed to load status ledger")?;
    let status_map = ledger
        .get_all_latest_status()
        .context("Failed to get extension status")?;

    // 2. Check if extension is installed
    let ext_status = status_map
        .get(&args.name)
        .filter(|s| s.current_state == ExtensionState::Installed)
        .ok_or_else(|| anyhow!("Extension '{}' is not installed", args.name))?;

    // Get version from ledger status
    let ext_version = ext_status
        .version
        .clone()
        .ok_or_else(|| anyhow!("Extension '{}' has no version information", args.name))?;

    let extensions_dir = get_extensions_dir()?;
    let ext_version_dir = extensions_dir.join(&args.name).join(&ext_version);

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
        let installed: HashSet<String> = status_map
            .iter()
            .filter(|(_, status)| status.current_state == ExtensionState::Installed)
            .map(|(name, _)| name.clone())
            .collect();

        let dependents: Vec<String> = installed
            .iter()
            .filter(|&name| name != &args.name)
            .filter_map(|name| {
                let status = status_map.get(name)?;
                let version = status.version.as_ref()?;
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
        args.name, ext_version
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

    // 5. Track start time
    let start_time = Instant::now();

    // Publish RemoveStarted event
    let remove_started_event = EventEnvelope::new(
        args.name.clone(),
        Some(ExtensionState::Installed),
        ExtensionState::Removing,
        ExtensionEvent::RemoveStarted {
            extension_name: args.name.clone(),
            version: ext_version.clone(),
        },
    );

    if let Err(e) = ledger.append(remove_started_event) {
        output::warning(&format!("Failed to publish remove started event: {}", e));
    }

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

    // 7. Remove extension directory
    if ext_version_dir.exists() {
        if let Err(e) = tokio::fs::remove_dir_all(&ext_version_dir)
            .await
            .context("Failed to remove extension directory")
        {
            let duration_secs = start_time.elapsed().as_secs();

            // Publish RemoveFailed event
            let remove_failed_event = EventEnvelope::new(
                args.name.clone(),
                Some(ExtensionState::Removing),
                ExtensionState::Failed,
                ExtensionEvent::RemoveFailed {
                    extension_name: args.name.clone(),
                    version: ext_version.clone(),
                    error_message: e.to_string(),
                    duration_secs,
                },
            );

            if let Err(ledger_err) = ledger.append(remove_failed_event) {
                output::warning(&format!(
                    "Failed to publish remove failed event: {}",
                    ledger_err
                ));
            }

            return Err(e);
        }
    }

    let duration_secs = start_time.elapsed().as_secs();

    // Publish RemoveCompleted event
    let remove_completed_event = EventEnvelope::new(
        args.name.clone(),
        Some(ExtensionState::Removing),
        ExtensionState::Installed, // Note: Actually removed, but no "Removed" state exists
        ExtensionEvent::RemoveCompleted {
            extension_name: args.name.clone(),
            version: ext_version.clone(),
            duration_secs,
        },
    );

    if let Err(e) = ledger.append(remove_completed_event) {
        output::warning(&format!("Failed to publish remove completed event: {}", e));
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
    use sindri_extensions::{ExtensionDistributor, ExtensionRegistry};

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

    // 4. Get installed version from ledger
    let ledger = StatusLedger::load_default().context("Failed to load status ledger")?;
    let status_map = ledger
        .get_all_latest_status()
        .context("Failed to get extension status")?;
    let installed_version = status_map.get(&args.name).and_then(|s| s.version.clone());

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
        let is_installed = installed_version
            .as_deref()
            .map(|v| v == version_str)
            .unwrap_or(false);
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
        if let (Some(installed), Some(latest)) = (&installed_version, &latest_version) {
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
    use sindri_extensions::{ExtensionDistributor, ExtensionRegistry};

    if args.extensions.is_empty() {
        output::info("Checking for updates to all installed extensions");
    } else {
        output::info(&format!(
            "Checking for updates to: {}",
            args.extensions.join(", ")
        ));
    }

    // 1. Load ledger to get installed extensions
    let ledger = StatusLedger::load_default().context("Failed to load status ledger")?;
    let status_map = ledger
        .get_all_latest_status()
        .context("Failed to get extension status")?;

    // 2. Get installed extensions (filter by specified names if provided)
    let installed: Vec<_> = if args.extensions.is_empty() {
        status_map
            .iter()
            .filter(|(_, s)| s.current_state == ExtensionState::Installed)
            .collect()
    } else {
        status_map
            .iter()
            .filter(|(_, s)| s.current_state == ExtensionState::Installed)
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

    for (name, ext_status) in installed {
        let version_str = ext_status.version.clone().unwrap_or_default();
        let current_version = Version::parse(&version_str).context(format!(
            "Invalid version in ledger for {}: {}",
            name, version_str
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
                        // Publish OutdatedDetected event to ledger
                        let event = EventEnvelope::new(
                            name.to_string(),
                            Some(ExtensionState::Installed),
                            ExtensionState::Outdated,
                            ExtensionEvent::OutdatedDetected {
                                extension_name: name.to_string(),
                                current_version: current_version.to_string(),
                                latest_version: latest_version.to_string(),
                            },
                        );
                        if let Err(e) = ledger.append(event) {
                            output::warning(&format!(
                                "Failed to publish OutdatedDetected event: {}",
                                e
                            ));
                        }
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
/// from the ledger history.
///
/// The rollback process:
/// 1. Checks if a previous version exists in the ledger history
/// 2. Confirms the rollback with the user (unless --yes is provided)
/// 3. Installs the previous version
/// 4. Updates the ledger to track the rollback
///
/// This follows the pattern from ADR-010 (GitHub Distribution) which specifies:
/// - Get current installed version from ledger
/// - Get previous version from version history
/// - Uninstall current, install previous
async fn rollback(args: ExtensionRollbackArgs) -> Result<()> {
    use dialoguer::Confirm;
    use semver::Version;
    use sindri_extensions::ExtensionDistributor;

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

    // 2. Load ledger to get current version and version history
    let ledger = StatusLedger::load_default().context("Failed to load status ledger")?;
    let status_map = ledger
        .get_all_latest_status()
        .context("Failed to get extension status")?;

    let ext_status = status_map.get(&args.name).ok_or_else(|| {
        anyhow!(
            "Extension '{}' is not installed. Cannot rollback.",
            args.name
        )
    })?;

    let current_version_str = ext_status.version.clone().ok_or_else(|| {
        anyhow!(
            "No version information available for '{}'. Cannot rollback.",
            args.name
        )
    })?;

    let current = Version::parse(&current_version_str)
        .context(format!("Invalid current version: {}", current_version_str))?;

    // 3. Check for previous version in history from ledger events
    // Extract version from event payload
    let extract_version = |event: &ExtensionEvent| -> Option<String> {
        match event {
            ExtensionEvent::InstallStarted { version, .. }
            | ExtensionEvent::InstallCompleted { version, .. }
            | ExtensionEvent::InstallFailed { version, .. }
            | ExtensionEvent::RemoveStarted { version, .. }
            | ExtensionEvent::RemoveCompleted { version, .. }
            | ExtensionEvent::RemoveFailed { version, .. }
            | ExtensionEvent::ValidationSucceeded { version, .. }
            | ExtensionEvent::ValidationFailed { version, .. } => Some(version.clone()),
            ExtensionEvent::UpgradeCompleted { to_version, .. }
            | ExtensionEvent::UpgradeFailed { to_version, .. }
            | ExtensionEvent::UpgradeStarted { to_version, .. } => Some(to_version.clone()),
            ExtensionEvent::OutdatedDetected { latest_version, .. } => Some(latest_version.clone()),
        }
    };

    let history = ledger
        .get_extension_history(&args.name, None)
        .context("Failed to get extension history")?;

    // Collect unique versions from history (most recent first), excluding current
    let mut previous_versions: Vec<String> = Vec::new();
    for envelope in history.iter().rev() {
        if let Some(v) = extract_version(&envelope.event) {
            if v != current_version_str && !previous_versions.contains(&v) {
                previous_versions.push(v);
            }
        }
    }

    let previous_version_str = previous_versions.first().ok_or_else(|| {
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
    if previous_versions.len() > 1 {
        println!();
        output::info("Version history:");
        for (i, v) in previous_versions.iter().enumerate() {
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
    // - Performing the rollback installation
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

        // 3. Versioned structure (downloaded mode) - check status ledger
        if found.is_none() {
            if let Ok(ledger) = StatusLedger::load_default() {
                if let Ok(status_map) = ledger.get_all_latest_status() {
                    if let Some(version) =
                        status_map.get(&args.name).and_then(|s| s.version.clone())
                    {
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
// Verify Command
// ============================================================================

/// Verify installed extensions are working correctly
///
/// Checks that installed extensions have valid extension.yaml files and
/// that their required tools/binaries are available on the system.
///
/// Usage:
/// - `sindri extension verify` - verify all installed extensions
/// - `sindri extension verify python` - verify a specific extension
async fn verify(args: ExtensionVerifyArgs) -> Result<()> {
    use sindri_extensions::{find_extension_yaml, verify_extension_installed};

    let ledger = StatusLedger::load_default().context("Failed to load status ledger")?;
    let status_map = ledger
        .get_all_latest_status()
        .context("Failed to get extension status")?;

    // Get extensions to verify
    let to_verify: Vec<_> = if let Some(name) = &args.name {
        let status = status_map
            .get(name)
            .filter(|s| s.current_state == ExtensionState::Installed)
            .ok_or_else(|| anyhow!("Extension '{}' is not installed", name))?;
        vec![(name.clone(), status.clone())]
    } else {
        status_map
            .iter()
            .filter(|(_, s)| s.current_state == ExtensionState::Installed)
            .map(|(n, s)| (n.clone(), s.clone()))
            .collect()
    };

    if to_verify.is_empty() {
        output::info("No installed extensions to verify");
        return Ok(());
    }

    output::info(&format!("Verifying {} extension(s)...", to_verify.len()));

    let mut verified = 0;
    let mut failed = 0;

    for (name, status) in &to_verify {
        let version = status.version.clone().unwrap_or_default();

        if let Some(yaml_path) = find_extension_yaml(name, &version) {
            match std::fs::read_to_string(&yaml_path) {
                Ok(content) => {
                    match serde_yaml::from_str::<sindri_core::types::Extension>(&content) {
                        Ok(extension) => {
                            let is_verified = verify_extension_installed(&extension).await;
                            if is_verified {
                                output::success(&format!("{} {} verified", name, version));

                                // Publish ValidationSucceeded event
                                let event = EventEnvelope::new(
                                    name.clone(),
                                    Some(ExtensionState::Installed),
                                    ExtensionState::Installed,
                                    ExtensionEvent::ValidationSucceeded {
                                        extension_name: name.clone(),
                                        version: version.clone(),
                                        validation_type: "manual".to_string(),
                                    },
                                );
                                if let Err(e) = ledger.append(event) {
                                    output::warning(&format!(
                                        "Failed to publish validation event: {}",
                                        e
                                    ));
                                }

                                verified += 1;
                            } else {
                                output::error(&format!("{} {} verification failed", name, version));

                                // Publish ValidationFailed event
                                let event = EventEnvelope::new(
                                    name.clone(),
                                    Some(ExtensionState::Installed),
                                    ExtensionState::Failed,
                                    ExtensionEvent::ValidationFailed {
                                        extension_name: name.clone(),
                                        version: version.clone(),
                                        validation_type: "manual".to_string(),
                                        error_message: "Verification checks failed".to_string(),
                                    },
                                );
                                if let Err(e) = ledger.append(event) {
                                    output::warning(&format!(
                                        "Failed to publish validation event: {}",
                                        e
                                    ));
                                }

                                failed += 1;
                            }
                        }
                        Err(e) => {
                            output::error(&format!("{}: invalid extension.yaml: {}", name, e));
                            failed += 1;
                        }
                    }
                }
                Err(e) => {
                    output::error(&format!("{}: cannot read extension.yaml: {}", name, e));
                    failed += 1;
                }
            }
        } else {
            output::error(&format!("{}: extension.yaml not found", name));
            failed += 1;
        }
    }

    println!();
    output::info(&format!(
        "Verification complete: {} verified, {} failed",
        verified, failed
    ));

    if failed > 0 {
        Err(anyhow!("{} extension(s) failed verification", failed))
    } else {
        Ok(())
    }
}

// ============================================================================
// Helper Functions
// ============================================================================
// Note: Helper functions are defined at the top of the file (lines 27-43)
