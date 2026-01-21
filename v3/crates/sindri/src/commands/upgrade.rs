//! Upgrade command

use anyhow::Result;
use sindri_update::{ReleaseManager, VERSION};

use crate::cli::UpgradeArgs;
use crate::output;

pub async fn run(args: UpgradeArgs) -> Result<()> {
    let mut manager = ReleaseManager::new();
    if args.prerelease {
        manager = manager.with_prerelease();
    }

    // List versions
    if args.list {
        output::header("Available versions");
        let releases = manager.list_releases(10).await?;

        for release in releases {
            let tag = &release.tag_name;
            let current = if tag.trim_start_matches('v') == VERSION {
                " (current)"
            } else {
                ""
            };
            let prerelease = if release.prerelease {
                " [prerelease]"
            } else {
                ""
            };
            println!("  {}{}{}", tag, current, prerelease);
        }
        return Ok(());
    }

    // Check for compatibility
    if let Some(version) = &args.compat {
        output::info(&format!(
            "Checking compatibility for version {}...",
            version
        ));
        output::info("Compatibility checking not yet implemented");
        return Ok(());
    }

    // Check for updates
    if args.check || args.version.is_none() {
        output::info(&format!("Current version: {}", VERSION));

        let spinner = output::spinner("Checking for updates...");
        let update = manager.check_update(VERSION).await?;
        spinner.finish_and_clear();

        match update {
            Some(release) => {
                output::success(&format!("Update available: {}", release.tag_name));
                if let Some(body) = &release.body {
                    // Show first few lines of changelog
                    let preview: String = body.lines().take(5).collect::<Vec<_>>().join("\n");
                    println!("\nChangelog:\n{}\n...", preview);
                }

                if !args.check {
                    output::info("Run 'sindri upgrade' to install");
                }
            }
            None => {
                output::success("Already on the latest version");
            }
        }

        if args.check {
            return Ok(());
        }
    }

    // Perform upgrade
    if let Some(target_version) = &args.version {
        output::info(&format!("Upgrading to version {}...", target_version));
    } else {
        output::info("Upgrading to latest version...");
    }

    // TODO: Implement actual upgrade using self_update crate
    output::info("Self-update not yet implemented");
    output::info("Please download the new version manually from:");
    output::info("  https://github.com/pacphi/sindri/releases");

    Ok(())
}
