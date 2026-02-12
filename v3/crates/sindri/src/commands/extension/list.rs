//! Extension list command

use anyhow::{Context, Result};
use sindri_core::types::ExtensionState;
use sindri_extensions::StatusLedger;
use tabled::{
    settings::{object::Columns, Modify, Style, Width},
    Table, Tabled,
};

use super::common::get_cli_version;
use crate::cli::ExtensionListArgs;
use crate::output;
use crate::utils::{get_extensions_dir, get_home_dir};

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
pub(super) async fn run(args: ExtensionListArgs) -> Result<()> {
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
