//! Profile management commands
//!
//! Implements profile-based batch installation of extensions:
//! - list: List available profiles with descriptions
//! - install: Install all extensions in a profile with dependency resolution
//! - reinstall: Reinstall all extensions in a profile
//! - info: Show detailed information about a profile
//! - status: Check which extensions in a profile are installed

use anyhow::{anyhow, Context, Result};
use console::style;
use dialoguer::Confirm;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use sindri_extensions::{
    DependencyResolver, ExtensionExecutor, ExtensionRegistry, ProfileInstaller, StatusLedger,
};
use std::path::PathBuf;
use tabled::{settings::Style, Table, Tabled};

use crate::cli::{
    ProfileCommands, ProfileInfoArgs, ProfileInstallArgs, ProfileListArgs, ProfileReinstallArgs,
    ProfileStatusArgs,
};
use crate::output;
use crate::utils::{get_cache_dir, get_extensions_dir, get_home_dir};

/// Main entry point for profile subcommands
pub async fn run(cmd: ProfileCommands) -> Result<()> {
    match cmd {
        ProfileCommands::List(args) => list(args).await,
        ProfileCommands::Install(args) => install(args).await,
        ProfileCommands::Reinstall(args) => reinstall(args).await,
        ProfileCommands::Info(args) => info(args).await,
        ProfileCommands::Status(args) => status(args).await,
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get workspace directory (current directory)
fn get_workspace_dir() -> Result<PathBuf> {
    std::env::current_dir().context("Failed to get current directory")
}

/// Load registry from GitHub with local cache
async fn load_registry() -> Result<ExtensionRegistry> {
    let cache_dir = get_cache_dir()?;
    let spinner = output::spinner("Loading extension registry from GitHub...");

    let registry = ExtensionRegistry::load_from_github(cache_dir, "main")
        .await
        .context("Failed to load extension registry")?;

    spinner.finish_and_clear();
    Ok(registry)
}

/// Create profile installer with all dependencies
async fn create_profile_installer() -> Result<ProfileInstaller> {
    let registry = load_registry().await?;
    let ledger = StatusLedger::load_default().context("Failed to load status ledger")?;

    let executor =
        ExtensionExecutor::new(get_extensions_dir()?, get_workspace_dir()?, get_home_dir()?);

    Ok(ProfileInstaller::new(registry, executor, ledger))
}

// ============================================================================
// List Command
// ============================================================================

#[derive(Tabled)]
struct ProfileRow {
    name: String,
    description: String,
    extensions: usize,
}

#[derive(Serialize, Deserialize)]
struct ProfileListJson {
    name: String,
    description: String,
    extensions: usize,
}

/// List all available profiles
async fn list(args: ProfileListArgs) -> Result<()> {
    let registry = load_registry().await?;

    // Get all profiles from registry
    let profile_names = registry.list_profiles();

    if profile_names.is_empty() {
        output::warning("No profiles found in registry");
        return Ok(());
    }

    if args.json {
        let mut json_profiles = Vec::new();

        for name in profile_names {
            if let Some(profile) = registry.get_profile(name) {
                json_profiles.push(ProfileListJson {
                    name: name.to_string(),
                    description: profile.description.clone(),
                    extensions: profile.extensions.len(),
                });
            }
        }

        println!("{}", serde_json::to_string_pretty(&json_profiles)?);
    } else {
        let mut profiles = Vec::new();

        for name in profile_names {
            if let Some(profile) = registry.get_profile(name) {
                profiles.push(ProfileRow {
                    name: name.to_string(),
                    description: profile.description.clone(),
                    extensions: profile.extensions.len(),
                });
            }
        }

        let mut table = Table::new(profiles);
        table.with(Style::sharp());
        println!("{}", table);
    }

    Ok(())
}

// ============================================================================
// Install Command
// ============================================================================

/// Install all extensions in a profile
pub async fn install(args: ProfileInstallArgs) -> Result<()> {
    output::header(&format!("Installing profile: {}", args.profile));

    // Load components
    let mut installer = create_profile_installer().await?;

    // Get profile extensions to show user what will be installed
    let extensions = installer
        .get_profile_extensions(&args.profile)
        .context("Failed to get profile extensions")?;

    if extensions.is_empty() {
        return Err(anyhow!(
            "Profile '{}' has no extensions defined",
            args.profile
        ));
    }

    // Show confirmation if not --yes
    if !args.yes {
        output::info(&format!(
            "This will install {} extensions from profile '{}'",
            extensions.len(),
            args.profile
        ));

        let confirmed = Confirm::new()
            .with_prompt("Continue with installation?")
            .default(false)
            .interact()?;

        if !confirmed {
            output::info("Installation cancelled");
            return Ok(());
        }
    }

    // Create progress bar
    let pb = ProgressBar::new(extensions.len() as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}",
        )
        .unwrap()
        .progress_chars("#>-"),
    );

    // Install profile with progress callback
    let result = installer
        .install_profile(
            &args.profile,
            Some(&|current, total, ext_name| {
                pb.set_position(current as u64);
                pb.set_message(format!("Installing {}", ext_name));
                if current == total {
                    pb.finish_with_message("Installation complete");
                }
            }),
        )
        .await
        .context("Profile installation failed")?;

    // Display results
    println!();
    if result.is_success() {
        output::success(&format!(
            "Profile '{}' installed successfully ({}/{} extensions)",
            args.profile, result.installed_count, result.total_count
        ));
    } else if result.is_partial() {
        output::warning(&format!(
            "Profile '{}' partially installed: {}/{} succeeded, {} failed",
            args.profile, result.installed_count, result.total_count, result.failed_count
        ));

        if !result.failed_extensions.is_empty() {
            println!();
            output::error(&format!(
                "{} extension(s) failed to install:",
                result.failed_count
            ));
            println!();

            for failed in &result.failed_extensions {
                println!("  {} {}", style("✗").red(), style(&failed.name).bold());
                println!("    Phase:  {}", failed.phase);
                if let Some(ref source) = failed.source {
                    println!("    Source: {}", source);
                }
                // Trim error message if too long
                let error_msg = if failed.error.len() > 300 {
                    format!("{}...", &failed.error[..297])
                } else {
                    failed.error.clone()
                };
                println!("    Error:  {}", error_msg);
                println!();
            }

            output::info("Tip: Run with RUST_LOG=debug for detailed logs");
        }
    } else {
        output::error(&format!(
            "Profile '{}' installation failed: all {} extensions failed",
            args.profile, result.failed_count
        ));

        if !result.failed_extensions.is_empty() {
            println!();
            for failed in &result.failed_extensions {
                println!(
                    "  {} {} ({})",
                    style("✗").red(),
                    style(&failed.name).bold(),
                    failed.phase
                );
                println!("    {}", failed.error);
                println!();
            }
        }
    }

    Ok(())
}

// ============================================================================
// Reinstall Command
// ============================================================================

/// Reinstall all extensions in a profile
async fn reinstall(args: ProfileReinstallArgs) -> Result<()> {
    output::header(&format!("Reinstalling profile: {}", args.profile));

    // Load components
    let mut installer = create_profile_installer().await?;

    // Get profile extensions
    let extensions = installer
        .get_profile_extensions(&args.profile)
        .context("Failed to get profile extensions")?;

    if extensions.is_empty() {
        return Err(anyhow!(
            "Profile '{}' has no extensions defined",
            args.profile
        ));
    }

    // Show confirmation if not --yes
    if !args.yes {
        output::warning(&format!(
            "This will remove and reinstall {} extensions from profile '{}'",
            extensions.len(),
            args.profile
        ));

        let confirmed = Confirm::new()
            .with_prompt("Continue with reinstallation?")
            .default(false)
            .interact()?;

        if !confirmed {
            output::info("Reinstallation cancelled");
            return Ok(());
        }
    }

    // Create progress bars for removal and installation
    let pb = ProgressBar::new((extensions.len() * 2) as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}",
        )
        .unwrap()
        .progress_chars("#>-"),
    );

    // Reinstall profile with progress callback
    let result = installer
        .reinstall_profile(
            &args.profile,
            Some(&|current, total, ext_name| {
                pb.set_position(current as u64);
                pb.set_message(format!("Processing {}", ext_name));
                if current == total {
                    pb.finish_with_message("Reinstallation complete");
                }
            }),
        )
        .await
        .context("Profile reinstallation failed")?;

    // Display results
    println!();
    if result.is_success() {
        output::success(&format!(
            "Profile '{}' reinstalled successfully ({}/{} extensions)",
            args.profile, result.installed_count, result.total_count
        ));
    } else {
        output::warning(&format!(
            "Profile '{}' reinstallation completed with issues: {}/{} succeeded",
            args.profile, result.installed_count, result.total_count
        ));
    }

    Ok(())
}

// ============================================================================
// Info Command
// ============================================================================

#[derive(Serialize, Deserialize)]
struct ProfileInfoJson {
    name: String,
    description: String,
    extensions: Vec<String>,
    total_with_dependencies: usize,
}

/// Show detailed information about a profile
async fn info(args: ProfileInfoArgs) -> Result<()> {
    let registry = load_registry().await?;

    // Get profile
    let profile = registry
        .get_profile(&args.profile)
        .ok_or_else(|| anyhow!("Profile '{}' not found", args.profile))?;

    // Calculate total with dependencies using DependencyResolver
    let resolver = DependencyResolver::new(&registry);
    let mut all_extensions = std::collections::HashSet::new();

    for ext_name in &profile.extensions {
        if let Ok(resolved) = resolver.resolve(ext_name) {
            all_extensions.extend(resolved);
        } else {
            // If resolution fails, just count the extension itself
            all_extensions.insert(ext_name.clone());
        }
    }

    if args.json {
        let info = ProfileInfoJson {
            name: args.profile.clone(),
            description: profile.description.clone(),
            extensions: profile.extensions.clone(),
            total_with_dependencies: all_extensions.len(),
        };
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else {
        println!("Name:         {}", args.profile);
        println!("Description:  {}", profile.description);
        println!("Extensions:   {}", profile.extensions.len());
        println!();
        println!("Extensions in profile:");
        for ext in &profile.extensions {
            println!("  - {}", ext);
        }
        println!();
        println!(
            "Total with dependencies: {} extensions",
            all_extensions.len()
        );
    }

    Ok(())
}

// ============================================================================
// Status Command
// ============================================================================

#[derive(Tabled)]
struct ExtensionStatusRow {
    extension: String,
    version: String,
    status: String,
}

#[derive(Serialize, Deserialize)]
struct ProfileStatusJson {
    profile: String,
    total_extensions: usize,
    installed_count: usize,
    not_installed_count: usize,
    installed_percentage: f64,
    extensions: Vec<ExtensionStatusDetail>,
}

#[derive(Serialize, Deserialize)]
struct ExtensionStatusDetail {
    name: String,
    version: Option<String>,
    installed: bool,
}

/// Check profile installation status
async fn status(args: ProfileStatusArgs) -> Result<()> {
    let installer = create_profile_installer().await?;

    // Get profile status
    let status = installer
        .check_profile_status(&args.profile)
        .context("Failed to check profile status")?;

    let ledger = StatusLedger::load_default()?;
    let status_map = ledger.get_all_latest_status()?;

    if args.json {
        // Build extension details
        let mut extensions = Vec::new();

        for ext_name in &status.installed_extensions {
            let version = status_map.get(ext_name).and_then(|s| s.version.clone());

            extensions.push(ExtensionStatusDetail {
                name: ext_name.clone(),
                version,
                installed: true,
            });
        }

        for ext_name in &status.not_installed_extensions {
            extensions.push(ExtensionStatusDetail {
                name: ext_name.clone(),
                version: None,
                installed: false,
            });
        }

        let json_status = ProfileStatusJson {
            profile: args.profile.clone(),
            total_extensions: status.total_extensions,
            installed_count: status.installed_extensions.len(),
            not_installed_count: status.not_installed_extensions.len(),
            installed_percentage: status.installed_percentage(),
            extensions,
        };

        println!("{}", serde_json::to_string_pretty(&json_status)?);
    } else {
        // Build table rows
        let mut rows = Vec::new();

        for ext_name in &status.installed_extensions {
            let version = status_map
                .get(ext_name)
                .and_then(|s| s.version.clone())
                .unwrap_or_else(|| "-".to_string());

            rows.push(ExtensionStatusRow {
                extension: ext_name.clone(),
                version,
                status: "installed".to_string(),
            });
        }

        for ext_name in &status.not_installed_extensions {
            rows.push(ExtensionStatusRow {
                extension: ext_name.clone(),
                version: "-".to_string(),
                status: "not installed".to_string(),
            });
        }

        let mut table = Table::new(rows);
        table.with(Style::sharp());
        println!("{}", table);
        println!();

        output::info(&format!(
            "Profile '{}': {}/{} extensions installed ({:.1}%)",
            args.profile,
            status.installed_extensions.len(),
            status.total_extensions,
            status.installed_percentage()
        ));

        if status.is_fully_installed() {
            output::success("Profile is fully installed");
        } else {
            output::info(&format!(
                "Run 'sindri profile install {}' to install remaining extensions",
                args.profile
            ));
        }
    }

    Ok(())
}
