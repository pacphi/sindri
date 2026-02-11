//! Upgrade command

use anyhow::{anyhow, Result};
use dialoguer::Confirm;
use indicatif::ProgressBar;
use semver::Version;
use sindri_core::types::ExtensionState;
use sindri_extensions::StatusLedger;
use sindri_update::{CompatibilityChecker, ReleaseManager, VERSION};
use std::collections::HashMap;
use std::env;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

use crate::cli::UpgradeArgs;
use crate::output;

const COMPAT_MATRIX_URL: &str =
    "https://raw.githubusercontent.com/pacphi/sindri/main/compatibility-matrix.yaml";

pub async fn run(args: UpgradeArgs) -> Result<()> {
    let mut manager = ReleaseManager::new();
    if args.prerelease {
        manager = manager.with_prerelease();
    }

    // List versions
    if args.list {
        return list_versions(&manager).await;
    }

    // Check for compatibility
    if let Some(version) = &args.compat {
        return show_compatibility(&manager, version).await;
    }

    // Check for updates
    if args.check {
        return check_for_updates(&manager).await;
    }

    // Perform upgrade
    do_upgrade(&manager, &args).await
}

/// List available versions
async fn list_versions(manager: &ReleaseManager) -> Result<()> {
    output::header("Available versions");
    let releases = manager.list_releases(20).await?;

    for release in releases {
        let tag = &release.tag_name;
        let version = tag.trim_start_matches('v');
        let current = if version == VERSION { " (current)" } else { "" };
        let prerelease = if release.prerelease {
            " [prerelease]"
        } else {
            ""
        };
        let date = release
            .published_at
            .as_ref()
            .map(|d| format!(" - {}", &d[..10]))
            .unwrap_or_default();
        println!("  {}{}{}{}", tag, date, prerelease, current);
    }
    Ok(())
}

/// Check for updates only
async fn check_for_updates(manager: &ReleaseManager) -> Result<()> {
    output::info(&format!("Current version: {}", VERSION));

    let spinner = output::spinner("Checking for updates...");
    let update = manager.check_update(VERSION).await?;
    spinner.finish_and_clear();

    match update {
        Some(release) => {
            output::success(&format!("Update available: {}", release.tag_name));
            if let Some(body) = &release.body {
                // Show first few lines of changelog
                let preview: String = body.lines().take(10).collect::<Vec<_>>().join("\n");
                println!("\nChangelog:\n{}\n", preview);
            }
            output::info("Run 'sindri upgrade' to install the update");
        }
        None => {
            output::success("Already on the latest version");
        }
    }

    Ok(())
}

/// Show compatibility info for a version
async fn show_compatibility(manager: &ReleaseManager, target_version: &str) -> Result<()> {
    output::header(&format!(
        "Compatibility check for version {}",
        target_version
    ));

    // Normalize version (add 'v' prefix if missing)
    let tag = if target_version.starts_with('v') {
        target_version.to_string()
    } else {
        format!("v{}", target_version)
    };

    // Fetch release info
    let spinner = output::spinner("Fetching release information...");
    let release = manager.get_release(&tag).await?;
    spinner.finish_and_clear();

    output::info(&format!(
        "Release: {}",
        release.name.as_ref().unwrap_or(&tag)
    ));
    if let Some(date) = &release.published_at {
        output::kv("Published", &date[..10]);
    }

    // Load compatibility matrix
    let spinner = output::spinner("Checking extension compatibility...");
    let mut checker = CompatibilityChecker::new();

    match checker.load_matrix(COMPAT_MATRIX_URL).await {
        Ok(_) => {
            spinner.finish_and_clear();

            // Load installed extensions from status ledger
            let ledger = StatusLedger::load_default()?;
            let status_map = ledger.get_all_latest_status()?;
            let installed: HashMap<String, String> = status_map
                .iter()
                .filter(|(_, s)| s.current_state == ExtensionState::Installed)
                .filter_map(|(name, s)| s.version.as_ref().map(|v| (name.clone(), v.clone())))
                .collect();

            if installed.is_empty() {
                output::info("No extensions installed - upgrade should be safe");
                return Ok(());
            }

            // Check compatibility
            let normalized = target_version.trim_start_matches('v');
            let result = checker.check_compatibility(normalized, &installed)?;

            if result.compatible {
                output::success("All installed extensions are compatible");
            } else {
                output::warning("Some extensions are incompatible:");
                println!();
                for ext in &result.incompatible_extensions {
                    println!("  {} ({})", ext.name, ext.current_version);
                    println!("    Required: {}", ext.required_range);
                    println!("    Reason: {}", ext.reason);
                    println!();
                }
            }

            if !result.breaking_changes.is_empty() {
                output::header("Breaking changes");
                for change in &result.breaking_changes {
                    println!("  - {}", change);
                }
            }
        }
        Err(e) => {
            spinner.finish_and_clear();
            output::warning(&format!("Could not load compatibility matrix: {}", e));
            output::info("Compatibility checking skipped");
        }
    }

    Ok(())
}

/// Perform the upgrade
async fn do_upgrade(manager: &ReleaseManager, args: &UpgradeArgs) -> Result<()> {
    // Determine target version
    let current = Version::parse(VERSION)?;

    let (target_version, release) = if let Some(ver) = &args.target_version {
        let normalized = ver.trim_start_matches('v');
        let target = Version::parse(normalized)?;
        let tag = format!("v{}", normalized);
        let release = manager.get_release(&tag).await?;
        (target, release)
    } else {
        // Get latest version
        let spinner = output::spinner("Checking for updates...");
        let update = manager.check_update(VERSION).await?;
        spinner.finish_and_clear();

        match update {
            Some(release) => {
                let version_str = release.tag_name.trim_start_matches('v');
                let target = Version::parse(version_str)?;
                (target, release)
            }
            None => {
                output::success("Already on the latest version");
                return Ok(());
            }
        }
    };

    // Version comparison
    if target_version == current {
        output::success(&format!("Already at version {}", current));
        return Ok(());
    }

    if target_version < current && !args.allow_downgrade {
        return Err(anyhow!(
            "Target version {} is older than current version {}.\nUse --allow-downgrade to proceed.",
            target_version,
            current
        ));
    }

    // Load installed extensions from status ledger
    let ledger = StatusLedger::load_default()?;
    let status_map = ledger.get_all_latest_status()?;
    let installed: HashMap<String, String> = status_map
        .iter()
        .filter(|(_, s)| s.current_state == ExtensionState::Installed)
        .filter_map(|(name, s)| s.version.as_ref().map(|v| (name.clone(), v.clone())))
        .collect();

    // Check extension compatibility
    if !installed.is_empty() && !args.force {
        let spinner = output::spinner("Checking extension compatibility...");
        let mut checker = CompatibilityChecker::new();

        let compat_result = match checker.load_matrix(COMPAT_MATRIX_URL).await {
            Ok(_) => {
                let result =
                    checker.check_compatibility(&target_version.to_string(), &installed)?;
                spinner.finish_and_clear();
                Some(result)
            }
            Err(e) => {
                spinner.finish_and_clear();
                output::warning(&format!("Could not load compatibility matrix: {}", e));
                output::info("Skipping compatibility check");
                None
            }
        };

        if let Some(result) = compat_result {
            if !result.compatible {
                output::error("Extension compatibility check failed!");
                println!();
                println!("Incompatible extensions:");
                for ext in &result.incompatible_extensions {
                    println!("  {} ({})", ext.name, ext.current_version);
                    println!("    Required: {}", ext.required_range);
                    println!("    Reason: {}", ext.reason);
                }
                println!();

                if !result.breaking_changes.is_empty() {
                    output::warning("Breaking changes in this version:");
                    for change in &result.breaking_changes {
                        println!("  - {}", change);
                    }
                    println!();
                }

                output::info("Use --force to override this check");
                return Err(anyhow!("Upgrade blocked due to incompatible extensions"));
            }

            if !result.breaking_changes.is_empty() {
                output::warning("This upgrade includes breaking changes:");
                for change in &result.breaking_changes {
                    println!("  - {}", change);
                }
                println!();
            }
        }
    }

    // Show upgrade plan
    output::header("Upgrade plan");
    println!("  Current version: {}", current);
    println!("  Target version:  {}", target_version);
    if target_version < current {
        output::warning("This is a downgrade");
    }
    println!();

    // Show changelog preview
    if let Some(body) = &release.body {
        output::header("Changelog");
        let preview: String = body.lines().take(15).collect::<Vec<_>>().join("\n");
        println!("{}", preview);
        if body.lines().count() > 15 {
            println!("...\n");
        } else {
            println!();
        }
    }

    // Prompt for confirmation unless --yes
    if !args.yes {
        let proceed = Confirm::new()
            .with_prompt("Proceed with upgrade?")
            .default(false)
            .interact()?;

        if !proceed {
            output::info("Upgrade cancelled");
            return Ok(());
        }
    }

    // Get platform asset
    let asset = manager
        .get_platform_asset(&release)
        .ok_or_else(|| anyhow!("No binary available for your platform"))?;

    output::info(&format!("Downloading {}", asset.name));

    // Download binary
    let pb = ProgressBar::new(asset.size);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} {msg}")?
            .progress_chars("#>-"),
    );
    pb.set_message("Downloading...");

    let client = reqwest::Client::new();
    let response = client.get(&asset.browser_download_url).send().await?;

    if !response.status().is_success() {
        return Err(anyhow!("Failed to download binary: {}", response.status()));
    }

    let bytes = response.bytes().await?;
    pb.finish_with_message("Downloaded");

    // Determine current binary path
    let current_exe = env::current_exe()?;
    let backup_path = current_exe.with_extension("backup");
    let temp_path = current_exe.with_extension("new");

    // Extract binary from tarball if needed
    let binary_data = if asset.name.ends_with(".tar.gz") {
        output::info("Extracting binary...");
        extract_binary_from_tarball(&bytes)?
    } else {
        bytes.to_vec()
    };

    // Write new binary to temp location
    fs::write(&temp_path, &binary_data)?;

    // Set executable permissions
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(&temp_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&temp_path, perms)?;
    }

    // Verify new binary works
    output::info("Verifying new binary...");
    let output = Command::new(&temp_path).arg("--version").output()?;

    if !output.status.success() {
        fs::remove_file(&temp_path)?;
        return Err(anyhow!("New binary failed verification"));
    }

    let version_output = String::from_utf8_lossy(&output.stdout);
    if !version_output.contains(&target_version.to_string()) {
        fs::remove_file(&temp_path)?;
        return Err(anyhow!(
            "Version mismatch: expected {}, got {}",
            target_version,
            version_output.trim()
        ));
    }

    // Backup current binary
    output::info("Backing up current binary...");
    if backup_path.exists() {
        fs::remove_file(&backup_path)?;
    }
    fs::rename(&current_exe, &backup_path)?;

    // Replace with new binary
    output::info("Installing new binary...");
    if let Err(e) = fs::rename(&temp_path, &current_exe) {
        // Rollback on failure
        output::error("Installation failed, rolling back...");
        fs::rename(&backup_path, &current_exe)?;
        return Err(anyhow!("Failed to install new binary: {}", e));
    }

    output::success(&format!(
        "Successfully upgraded to sindri {}",
        target_version
    ));
    output::info("Backup saved at:");
    println!("  {}", backup_path.display());

    Ok(())
}

/// Extract binary from tarball
fn extract_binary_from_tarball(data: &[u8]) -> Result<Vec<u8>> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    let decoder = GzDecoder::new(data);
    let mut archive = Archive::new(decoder);

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;

        // Look for the sindri binary
        if path.file_name().and_then(|n| n.to_str()) == Some("sindri") {
            let mut buffer = Vec::new();
            std::io::copy(&mut entry, &mut buffer)?;
            return Ok(buffer);
        }
    }

    Err(anyhow!("Binary not found in tarball"))
}
