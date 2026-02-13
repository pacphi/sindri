//! Extension install command

use anyhow::{anyhow, Context, Result};
use sindri_core::types::ExtensionState;
use sindri_extensions::{EventEnvelope, ExtensionEvent, StatusLedger};
use std::time::Instant;

use crate::cli::ExtensionInstallArgs;
use crate::output;
use crate::utils::{get_cache_dir, get_home_dir};

/// Install an extension with optional version specification
///
/// Supports three modes:
/// 1. Install by name: `sindri extension install python` or `python@1.1.0`
/// 2. Install from config: `sindri extension install --from-config sindri.yaml`
/// 3. Install from profile: `sindri extension install --profile minimal`
///
/// Options:
/// - Force reinstall: `--force`
/// - Skip dependencies: `--no-deps`
/// - Skip confirmation: `--yes` (for profile mode)
pub(super) async fn run(args: ExtensionInstallArgs) -> Result<()> {
    match (&args.from_config, &args.profile, &args.name) {
        // Mode 1: From config file
        (Some(config_path), None, None) => {
            install_from_config(config_path.clone(), args.force, args.no_deps, args.yes).await
        }
        // Mode 2: From profile
        (None, Some(profile_name), None) => {
            install_from_profile(profile_name.clone(), args.yes).await
        }
        // Mode 3: By name
        (None, None, Some(name)) => {
            install_by_name(name.clone(), args.version, args.force, args.no_deps).await
        }
        // Error: No source specified
        (None, None, None) => Err(anyhow!(
            "Must specify extension name, --from-config, or --profile"
        )),
        // Defensive: multiple sources (clap should catch this)
        _ => Err(anyhow!("Conflicting options specified")),
    }
}

/// Install a single extension by name
async fn install_by_name(
    name: String,
    version: Option<String>,
    force: bool,
    no_deps: bool,
) -> Result<()> {
    // Parse name@version format if present
    let (name, version) = if name.contains('@') {
        let parts: Vec<&str> = name.split('@').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid format: {}. Use name@version", name));
        }
        (parts[0].to_string(), Some(parts[1].to_string()))
    } else {
        (name, version)
    };

    // Check if name is a known profile (if no version specified)
    // This helps users who might type "sindri extension install minimal" instead of "sindri profile install minimal"
    if version.is_none() {
        // Load profile names dynamically from registry
        let cache_dir = get_cache_dir()?;
        if let Ok(registry) =
            sindri_extensions::ExtensionRegistry::load_from_github(cache_dir, "main").await
        {
            let profile_names = registry.list_profiles();

            if profile_names.contains(&name.as_str()) {
                output::warning(&format!(
                    "'{}' looks like a profile name. Did you mean 'sindri profile install {}'?",
                    name, name
                ));
                output::info("Or use: sindri extension install --profile <profile-name>");
                return Err(anyhow!(
                    "Use 'sindri profile install {}' for profile installation",
                    name
                ));
            }
        }
    }

    output::info(&format!(
        "Installing extension: {}{}",
        name,
        version
            .as_ref()
            .map(|v| format!("@{}", v))
            .unwrap_or_default()
    ));

    if force {
        output::info("Force reinstall enabled");
    }

    if no_deps {
        output::warning("Skipping dependency installation");
    }

    // Get home directory for cache and extensions
    let home = get_home_dir()?;
    let cache_dir = home.join(".sindri").join("cache");
    let extensions_dir = home.join(".sindri").join("extensions");

    // Parse CLI version
    let cli_version =
        semver::Version::parse(env!("CARGO_PKG_VERSION")).context("Failed to parse CLI version")?;

    // Initialize distributor
    let distributor =
        sindri_extensions::ExtensionDistributor::new(cache_dir, extensions_dir, cli_version)?;

    // Initialize ledger for event tracking
    let ledger = StatusLedger::load_default().context("Failed to load status ledger")?;

    // Track start time
    let start_time = Instant::now();

    // Publish InstallStarted event
    let version_str = version.as_deref().unwrap_or("latest").to_string();
    let install_started_event = EventEnvelope::new(
        name.clone(),
        None,
        ExtensionState::Installing,
        ExtensionEvent::InstallStarted {
            extension_name: name.clone(),
            version: version_str.clone(),
            source: "github:pacphi/sindri".to_string(),
            install_method: "Distributor".to_string(),
        },
    );

    if let Err(e) = ledger.append(install_started_event) {
        output::warning(&format!("Failed to publish install started event: {}", e));
    }

    // Create spinner
    let spinner = output::spinner("Installing extension...");

    // Install extension
    let result = distributor.install(&name, version.as_deref()).await;
    let duration_secs = start_time.elapsed().as_secs();

    match result {
        Ok((resolved_version, log_file_path)) => {
            spinner.finish_and_clear();

            // Use the resolved version from the distributor (actual metadata.version)
            // instead of the user-specified version which may be "latest"
            let final_version = resolved_version;

            // Publish InstallCompleted event
            let install_completed_event = EventEnvelope::new(
                name.clone(),
                Some(ExtensionState::Installing),
                ExtensionState::Installed,
                ExtensionEvent::InstallCompleted {
                    extension_name: name.clone(),
                    version: final_version.clone(),
                    duration_secs,
                    components_installed: vec![],
                    log_file: log_file_path,
                },
            );

            if let Err(e) = ledger.append(install_completed_event) {
                output::warning(&format!("Failed to publish install completed event: {}", e));
            }

            output::success(&format!(
                "Successfully installed {}@{}",
                name, final_version
            ));
            Ok(())
        }
        Err(e) => {
            spinner.finish_and_clear();

            // Try to recover the log file path from the most recent log
            let log_file = sindri_extensions::ExtensionLogWriter::new_default()
                .ok()
                .and_then(|w| w.find_latest_log(&name))
                .map(|p| p.to_string_lossy().to_string());

            // Publish InstallFailed event
            let install_failed_event = EventEnvelope::new(
                name.clone(),
                Some(ExtensionState::Installing),
                ExtensionState::Failed,
                ExtensionEvent::InstallFailed {
                    extension_name: name.clone(),
                    version: version_str.clone(),
                    error_message: e.to_string(),
                    retry_count: 0,
                    duration_secs,
                    log_file,
                },
            );

            if let Err(ledger_err) = ledger.append(install_failed_event) {
                output::warning(&format!(
                    "Failed to publish install failed event: {}",
                    ledger_err
                ));
            }

            output::error(&format!("Failed to install {}: {}", name, e));
            Err(e)
        }
    }
}

/// Install extensions from a sindri.yaml config file
async fn install_from_config(
    config_path: camino::Utf8PathBuf,
    force: bool,
    no_deps: bool,
    yes: bool,
) -> Result<()> {
    use dialoguer::Confirm;
    use sindri_core::config::SindriConfig;

    output::info(&format!("Loading configuration from: {}", config_path));
    let config = SindriConfig::load(Some(&config_path))?;
    let ext_config = config.extensions();

    // Case 1: Profile specified - install profile first, then additional extensions
    if let Some(profile_name) = &ext_config.profile {
        // Install the profile
        install_from_profile(profile_name.clone(), yes).await?;

        // Install additional extensions on top of profile (if any)
        if let Some(additional) = &ext_config.additional {
            if !additional.is_empty() {
                if !yes {
                    output::info(&format!(
                        "Installing {} additional extension(s) on top of profile '{}':",
                        additional.len(),
                        profile_name
                    ));
                    for ext in additional {
                        println!("  - {}", ext);
                    }
                    let confirmed = Confirm::new()
                        .with_prompt("Continue?")
                        .default(true)
                        .interact()?;
                    if !confirmed {
                        output::info("Cancelled additional installations");
                        return Ok(());
                    }
                }

                // Install each additional extension
                for ext in additional {
                    install_by_name(ext.clone(), None, force, no_deps).await?;
                }
            }
        }

        return Ok(());
    }

    // Case 2: Active list specified (no profile)
    // Validate: additional should ONLY work with profile
    if ext_config.additional.is_some() && !ext_config.additional.as_ref().unwrap().is_empty() {
        return Err(anyhow!(
            "Configuration error: 'extensions.additional' can only be used with 'extensions.profile'. \
             Use 'extensions.active' for explicit extension lists without a profile."
        ));
    }

    // Install from active list
    if let Some(active) = &ext_config.active {
        if active.is_empty() {
            output::warning("No extensions specified in config file");
            return Ok(());
        }

        // Confirmation prompt (unless --yes)
        if !yes {
            output::info(&format!("Installing {} extension(s):", active.len()));
            for ext in active {
                println!("  - {}", ext);
            }
            let confirmed = Confirm::new()
                .with_prompt("Continue?")
                .default(true)
                .interact()?;
            if !confirmed {
                output::info("Cancelled");
                return Ok(());
            }
        }

        // Install each extension
        for ext in active {
            install_by_name(ext.clone(), None, force, no_deps).await?;
        }

        return Ok(());
    }

    // Case 3: No profile, no active list
    output::warning("No extensions specified in config file");
    Ok(())
}

/// Install extensions from a profile (delegates to profile::install)
async fn install_from_profile(profile_name: String, yes: bool) -> Result<()> {
    use crate::cli::ProfileInstallArgs;
    use crate::commands::profile;

    let profile_args = ProfileInstallArgs {
        profile: profile_name,
        yes,
        continue_on_error: true,
    };
    profile::install(profile_args).await
}
