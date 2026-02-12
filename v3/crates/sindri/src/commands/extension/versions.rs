//! Extension versions command

use anyhow::{anyhow, Context, Result};
use sindri_extensions::StatusLedger;
use tabled::{settings::Style, Table, Tabled};

use super::common::get_cli_version;
use crate::cli::ExtensionVersionsArgs;
use crate::output;
use crate::utils::{get_cache_dir, get_extensions_dir};

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
pub(super) async fn run(args: ExtensionVersionsArgs) -> Result<()> {
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
