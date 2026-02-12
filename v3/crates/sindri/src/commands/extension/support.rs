//! Extension update-support-files command

use anyhow::{Context, Result};

use crate::cli::UpdateSupportFilesArgs;
use crate::output;

/// Update support files (common.sh, compatibility-matrix.yaml, extension-source.yaml)
///
/// Fetches version-matched support files from GitHub or copies from bundled files.
/// Supports three modes:
/// 1. Normal: Check version, update if needed
/// 2. Force: Always update regardless of version
/// 3. Bundled: Use image-bundled files (offline mode)
pub(super) async fn run(args: UpdateSupportFilesArgs) -> Result<()> {
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
