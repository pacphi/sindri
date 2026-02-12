//! Extension remove command

use anyhow::{anyhow, Context, Result};
use sindri_core::types::ExtensionState;
use sindri_extensions::{EventEnvelope, ExtensionEvent, StatusLedger};
use std::time::Instant;

use crate::cli::ExtensionRemoveArgs;
use crate::output;
use crate::utils::{get_extensions_dir, get_home_dir};

/// Remove an installed extension
///
/// Supports:
/// - Remove with confirmation: `sindri extension remove python`
/// - Force remove: `sindri extension remove python -y`
/// - Force even with dependents: `sindri extension remove python --force`
pub(super) async fn run(args: ExtensionRemoveArgs) -> Result<()> {
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
                serde_yaml_ng::from_str::<sindri_core::types::Extension>(&content)
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
                let ext: sindri_core::types::Extension = serde_yaml_ng::from_str(&content).ok()?;
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
