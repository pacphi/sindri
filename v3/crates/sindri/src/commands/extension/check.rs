//! Extension check command

use anyhow::{Context, Result};
use semver::Version;
use sindri_core::types::ExtensionState;
use sindri_extensions::{EventEnvelope, ExtensionEvent, StatusLedger};
use tabled::{settings::Style, Table, Tabled};

use super::common::get_cli_version;
use crate::cli::ExtensionCheckArgs;
use crate::output;
use crate::utils::{get_cache_dir, get_extensions_dir};

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
pub(super) async fn run(args: ExtensionCheckArgs) -> Result<()> {
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
