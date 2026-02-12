//! Extension info command

use anyhow::{anyhow, Context, Result};
use sindri_core::types::ExtensionState;
use sindri_extensions::StatusLedger;

use crate::cli::ExtensionInfoArgs;
use crate::output;
use crate::utils::{get_cache_dir, get_extensions_dir};

/// Show detailed information about an extension
///
/// Displays:
/// - Name, version, category
/// - Description
/// - Dependencies
/// - Installation method
/// - Source repository
/// - Installed version and timestamp
pub(super) async fn run(args: ExtensionInfoArgs) -> Result<()> {
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
