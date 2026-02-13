//! Extension upgrade command

use anyhow::{anyhow, Context, Result};
use semver::Version;
use sindri_core::types::ExtensionState;
use sindri_extensions::{EventEnvelope, ExtensionEvent, StatusLedger};
use std::time::Instant;

use super::common::get_cli_version;
use crate::cli::ExtensionUpgradeArgs;
use crate::output;
use crate::utils::{get_cache_dir, get_extensions_dir};

/// Upgrade an extension to a newer version
///
/// Supports:
/// - Upgrade to latest: `sindri extension upgrade python`
/// - Upgrade to specific: `sindri extension upgrade python --target-version 1.2.0`
/// - Skip confirmation: `sindri extension upgrade python -y`
pub(super) async fn run(args: ExtensionUpgradeArgs) -> Result<()> {
    use dialoguer::Confirm;
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
        // Install specific version (map to discard resolved version string)
        distributor
            .install(&args.name, Some(&target.to_string()))
            .await
            .map(|_| ())
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
