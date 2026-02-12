//! Extension rollback command

use anyhow::{anyhow, Context, Result};
use semver::Version;
use sindri_extensions::{ExtensionEvent, StatusLedger};

use super::common::get_cli_version;
use crate::cli::ExtensionRollbackArgs;
use crate::output;
use crate::utils::{get_cache_dir, get_extensions_dir};

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
pub(super) async fn run(args: ExtensionRollbackArgs) -> Result<()> {
    use dialoguer::Confirm;
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
            ExtensionEvent::OutdatedDetected { latest_version, .. } => {
                Some(latest_version.clone())
            }
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
