//! Extension management commands
//!
//! Implements Phase 4 extension management CLI commands:
//! - install: Install an extension with optional version
//! - list: List available extensions with filtering
//! - validate: Validate extension against schema
//! - status: Show installation status
//! - info: Show detailed extension information
//! - upgrade: Upgrade extension to newer version
//! - remove: Remove an installed extension
//! - versions: Show available versions with compatibility
//! - check: Check for extension updates
//! - rollback: Rollback to previous version

use anyhow::{anyhow, Context, Result};
use semver::Version;
use serde_json;
use tabled::{Table, Tabled};

use crate::cli::{
    ExtensionCheckArgs, ExtensionCommands, ExtensionInfoArgs, ExtensionInstallArgs,
    ExtensionListArgs, ExtensionRemoveArgs, ExtensionRollbackArgs, ExtensionStatusArgs,
    ExtensionUpgradeArgs, ExtensionValidateArgs, ExtensionVersionsArgs,
};
use crate::output;

/// Helper function to get the cache directory
fn get_cache_dir() -> Result<std::path::PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
    Ok(home.join(".sindri").join("cache"))
}

/// Helper function to get the extensions directory
fn get_extensions_dir() -> Result<std::path::PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
    Ok(home.join(".sindri").join("extensions"))
}

/// Helper function to get the CLI version
fn get_cli_version() -> Result<semver::Version> {
    semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .context("Failed to parse CLI version")
}

/// Main entry point for extension subcommands
pub async fn run(cmd: ExtensionCommands) -> Result<()> {
    match cmd {
        ExtensionCommands::Install(args) => install(args).await,
        ExtensionCommands::List(args) => list(args).await,
        ExtensionCommands::Validate(args) => validate(args).await,
        ExtensionCommands::Status(args) => status(args).await,
        ExtensionCommands::Info(args) => info(args).await,
        ExtensionCommands::Upgrade(args) => upgrade(args).await,
        ExtensionCommands::Remove(args) => remove(args).await,
        ExtensionCommands::Versions(args) => versions(args).await,
        ExtensionCommands::Check(args) => check(args).await,
        ExtensionCommands::Rollback(args) => rollback(args).await,
    }
}

// ============================================================================
// Install Command
// ============================================================================

/// Install an extension with optional version specification
///
/// Supports:
/// - Install latest compatible version: `sindri extension install python`
/// - Install specific version: `sindri extension install python@1.1.0` or `--version 1.1.0`
/// - Force reinstall: `--force`
/// - Skip dependencies: `--no-deps`
async fn install(args: ExtensionInstallArgs) -> Result<()> {
    // Parse name@version format if present
    let (name, version) = if args.name.contains('@') {
        let parts: Vec<&str> = args.name.split('@').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid format: {}. Use name@version", args.name));
        }
        (parts[0].to_string(), Some(parts[1].to_string()))
    } else {
        (args.name.clone(), args.version.clone())
    };

    // Check if name is a known profile (if no version specified)
    // This helps users who might type "sindri extension install minimal" instead of "sindri profile install minimal"
    if version.is_none() {
        // Load profile names dynamically from registry
        let cache_dir = get_cache_dir()?;
        if let Ok(registry) = sindri_extensions::ExtensionRegistry::load_from_github(cache_dir, "main").await {
            let profile_names = registry.list_profiles();

            if profile_names.contains(&name.as_str()) {
                output::warning(&format!(
                    "'{}' looks like a profile name. Did you mean 'sindri profile install {}'?",
                    name, name
                ));
                output::info("To install an extension with this name, use: sindri extension install <name> --version <version>");
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

    if args.force {
        output::info("Force reinstall enabled");
    }

    if args.no_deps {
        output::warning("Skipping dependency installation");
    }

    // Get home directory for cache and extensions
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
    let cache_dir = home.join(".sindri").join("cache");
    let extensions_dir = home.join(".sindri").join("extensions");

    // Parse CLI version
    let cli_version =
        semver::Version::parse(env!("CARGO_PKG_VERSION")).context("Failed to parse CLI version")?;

    // Initialize distributor
    let distributor =
        sindri_extensions::ExtensionDistributor::new(cache_dir, extensions_dir, cli_version)?;

    // Create spinner
    let spinner = output::spinner("Installing extension...");

    // Install extension
    match distributor.install(&name, version.as_deref()).await {
        Ok(()) => {
            spinner.finish_and_clear();
            output::success(&format!(
                "Successfully installed {}{}",
                name,
                version
                    .as_ref()
                    .map(|v| format!("@{}", v))
                    .unwrap_or_default()
            ));
            Ok(())
        }
        Err(e) => {
            spinner.finish_and_clear();
            output::error(&format!("Failed to install {}: {}", name, e));
            Err(e)
        }
    }
}

// ============================================================================
// List Command
// ============================================================================

#[derive(Tabled, serde::Serialize, serde::Deserialize)]
struct ExtensionRow {
    name: String,
    category: String,
    version: String,
    installed: String,
    description: String,
}

/// List available extensions with optional filtering
///
/// Supports:
/// - List all: `sindri extension list`
/// - Filter by category: `sindri extension list --category language`
/// - Show installed only: `sindri extension list --installed`
/// - JSON output: `sindri extension list --json`
async fn list(args: ExtensionListArgs) -> Result<()> {
    output::info(&format!(
        "Listing extensions{}{}",
        args.category
            .as_ref()
            .map(|c| format!(" (category: {})", c))
            .unwrap_or_default(),
        if args.installed {
            " (installed only)"
        } else {
            ""
        }
    ));

    // Get home directory for cache and manifest
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
    let cache_dir = home.join(".sindri").join("cache");
    let manifest_path = home.join(".sindri").join("manifest.yaml");

    // Load registry from GitHub with caching
    let registry = sindri_extensions::ExtensionRegistry::load_from_github(cache_dir, "main")
        .await
        .context("Failed to load extension registry")?;

    // Load manifest to get installed versions
    let manifest = sindri_extensions::ManifestManager::new(manifest_path)
        .context("Failed to load manifest")?;

    // Build extension rows
    let mut extensions: Vec<ExtensionRow> = Vec::new();

    for (name, entry) in registry.entries.iter() {
        // Filter by category if specified
        if let Some(category) = &args.category {
            if &entry.category != category {
                continue;
            }
        }

        // Get installed version if available
        let installed_version = manifest
            .get_version(name)
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".to_string());

        // Filter by installed if requested
        if args.installed && installed_version == "-" {
            continue;
        }

        extensions.push(ExtensionRow {
            name: name.clone(),
            category: entry.category.clone(),
            version: "latest".to_string(),
            installed: installed_version,
            description: entry.description.clone(),
        });
    }

    // Sort by category then name
    extensions.sort_by(|a, b| a.category.cmp(&b.category).then(a.name.cmp(&b.name)));

    if args.json {
        let json = serde_json::to_string_pretty(&extensions)
            .context("Failed to serialize extensions to JSON")?;
        println!("{}", json);
    } else {
        if extensions.is_empty() {
            output::warn("No extensions found matching criteria");
        } else {
            let table = Table::new(extensions);
            println!("{}", table);
        }
    }

    Ok(())
}

// ============================================================================
// Validate Command
// ============================================================================

/// Validate an extension against JSON schema
///
/// Supports:
/// - Validate by name: `sindri extension validate python`
/// - Validate file: `sindri extension validate --file extension.yaml`
async fn validate(args: ExtensionValidateArgs) -> Result<()> {
    // Check if --file is specified or if name is a path
    let validation_target = if let Some(file_path) = &args.file {
        format!("file: {}", file_path)
    } else {
        let path = std::path::Path::new(&args.name);
        if path.exists() && path.is_file() {
            format!("file: {}", args.name)
        } else {
            format!("extension: {}", args.name)
        }
    };

    output::info(&format!("Validating {}", validation_target));

    // TODO: Implement full validation
    // 1. Load extension.yaml (from file or registry)
    // 2. Validate against extension.schema.json
    // 3. Check all required fields
    // 4. Validate dependency references
    // 5. Check for conflicts with installed extensions

    let validator = sindri_core::schema::SchemaValidator::new()?;

    let path = if let Some(file_path) = &args.file {
        file_path.as_std_path()
    } else {
        std::path::Path::new(&args.name)
    };

    if path.exists() {
        validator.validate_file(path, "extension")?;
        output::success(&format!("Extension {} is valid", args.name));
    } else {
        output::warning("Extension path not found, would validate from registry");
        // TODO: Load from registry and validate
    }

    Ok(())
}

// ============================================================================
// Status Command
// ============================================================================

#[derive(Tabled)]
struct StatusRow {
    name: String,
    version: String,
    status: String,
    installed_at: String,
}

/// Show installation status for extensions
///
/// Supports:
/// - Show all: `sindri extension status`
/// - Show specific: `sindri extension status python`
/// - JSON output: `sindri extension status --json`
async fn status(args: ExtensionStatusArgs) -> Result<()> {
    if let Some(name) = &args.name {
        output::info(&format!("Checking status of extension: {}", name));
    } else {
        output::info("Checking status of all installed extensions");
    }

    // TODO: Load manifest and check actual status
    // 1. Load ~/.sindri/manifest.yaml
    // 2. For each extension, check:
    //    - Installed version
    //    - Installation timestamp
    //    - Validation status (run validation commands)
    //    - Available updates
    // 3. Format output

    let statuses = vec![
        StatusRow {
            name: "python".to_string(),
            version: "1.1.0".to_string(),
            status: "installed".to_string(),
            installed_at: "2025-01-20".to_string(),
        },
        StatusRow {
            name: "nodejs".to_string(),
            version: "2.0.0".to_string(),
            status: "installed".to_string(),
            installed_at: "2025-01-19".to_string(),
        },
    ];

    if args.json {
        // TODO: Proper JSON serialization
        println!("[]");
    } else {
        let table = Table::new(statuses);
        println!("{}", table);
    }

    Ok(())
}

// ============================================================================
// Info Command
// ============================================================================

/// Show detailed information about an extension
///
/// Displays:
/// - Name, version, category
/// - Description
/// - Dependencies
/// - Installation method
/// - Source repository
/// - Installed version and timestamp
async fn info(args: ExtensionInfoArgs) -> Result<()> {
    use sindri_extensions::{ExtensionRegistry, ManifestManager};

    // Load registry and manifest
    let cache_dir = get_cache_dir()?;
    let registry = ExtensionRegistry::load_from_github(cache_dir, "main")
        .await
        .context("Failed to load extension registry")?;

    let manifest = ManifestManager::load_default().context("Failed to load manifest")?;

    // Look up extension in registry
    let entry = registry
        .get_entry(&args.name)
        .ok_or_else(|| anyhow!("Extension '{}' not found in registry", args.name))?;

    // Get installed info if available
    let installed = manifest.get_installed(&args.name);

    if args.json {
        // JSON output
        let json_output = serde_json::json!({
            "name": args.name,
            "category": entry.category,
            "description": entry.description,
            "dependencies": entry.dependencies,
            "conflicts": entry.conflicts,
            "protected": entry.protected,
            "installed": installed.map(|ext| serde_json::json!({
                "version": ext.version,
                "installed_at": ext.installed_at.to_rfc3339(),
                "source": ext.source,
                "state": format!("{:?}", ext.state),
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

        if let Some(ext) = installed {
            output::header("Installation Status");
            println!();
            output::kv("Status", &format!("{:?}", ext.state));
            output::kv("Installed Version", &ext.version);
            output::kv(
                "Installed At",
                &ext.installed_at.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            );
            output::kv("Source", &ext.source);

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

// ============================================================================
// Upgrade Command
// ============================================================================

/// Upgrade an extension to a newer version
///
/// Supports:
/// - Upgrade to latest: `sindri extension upgrade python`
/// - Upgrade to specific: `sindri extension upgrade python --version 1.2.0`
/// - Skip confirmation: `sindri extension upgrade python -y`
async fn upgrade(args: ExtensionUpgradeArgs) -> Result<()> {
    use dialoguer::Confirm;
    use semver::Version;
    use sindri_extensions::{ExtensionDistributor, ManifestManager};

    output::info(&format!("Upgrading extension: {}", args.name));

    // 1. Initialize ExtensionDistributor
    let cache_dir = get_cache_dir()?;
    let extensions_dir = get_extensions_dir()?;
    let cli_version = get_cli_version()?;

    let distributor = ExtensionDistributor::new(cache_dir, extensions_dir, cli_version)
        .context("Failed to initialize extension distributor")?;

    // 2. Check current installed version from manifest
    let manifest = ManifestManager::load_default().context("Failed to load manifest")?;

    let current_version = manifest
        .get_version(&args.name)
        .ok_or_else(|| anyhow!("Extension '{}' is not installed", args.name))?;

    let current = Version::parse(current_version)
        .context(format!("Invalid current version: {}", current_version))?;

    // 3. Determine target version
    let target = if let Some(version_spec) = &args.version {
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

    // 7. Call distributor.upgrade()
    let spinner = output::spinner(&format!("Upgrading {} to {}", args.name, target));

    let result = if args.version.is_some() {
        // Install specific version
        distributor
            .install(&args.name, Some(&target.to_string()))
            .await
    } else {
        // Use upgrade method for latest
        distributor.upgrade(&args.name).await
    };

    spinner.finish_and_clear();

    match result {
        Ok(_) => {
            output::success(&format!(
                "Successfully upgraded {} from {} to {}",
                args.name, current, target
            ));
        }
        Err(e) => {
            output::error(&format!("Upgrade failed: {}", e));
            return Err(e);
        }
    }

    Ok(())
}

// ============================================================================
// Remove Command
// ============================================================================

/// Remove an installed extension
///
/// Supports:
/// - Remove with confirmation: `sindri extension remove python`
/// - Force remove: `sindri extension remove python -y`
/// - Force even with dependents: `sindri extension remove python --force`
async fn remove(args: ExtensionRemoveArgs) -> Result<()> {
    use dialoguer::Confirm;
    use sindri_extensions::ManifestManager;
    use std::collections::HashSet;

    output::info(&format!("Removing extension: {}", args.name));

    // 1. Load ManifestManager
    let mut manifest = ManifestManager::load_default()
        .context("Failed to load manifest")?;

    // 2. Check if extension is installed
    if !manifest.is_installed(&args.name) {
        return Err(anyhow!("Extension '{}' is not installed", args.name));
    }

    let installed_ext = manifest.get_installed(&args.name)
        .ok_or_else(|| anyhow!("Extension '{}' not found in manifest", args.name))?;

    let extensions_dir = get_extensions_dir()?;
    let ext_version_dir = extensions_dir
        .join(&args.name)
        .join(&installed_ext.version);

    let extension = if ext_version_dir.exists() {
        let ext_yaml = ext_version_dir.join("extension.yaml");
        if ext_yaml.exists() {
            let content = std::fs::read_to_string(&ext_yaml)
                .context("Failed to read extension.yaml")?;
            Some(serde_yaml::from_str::<sindri_core::types::Extension>(&content)
                .context("Failed to parse extension.yaml")?)
        } else {
            None
        }
    } else {
        None
    };

    // 3. Check if other extensions depend on it (unless --force)
    if !args.force {
        let installed: HashSet<String> = manifest
            .list_installed()
            .iter()
            .map(|(name, _)| name.to_string())
            .collect();

        let dependents: Vec<String> = installed
            .iter()
            .filter(|&name| name != &args.name)
            .filter_map(|name| {
                let version = manifest.get_version(name)?;
                let ext_yaml = extensions_dir.join(name).join(version).join("extension.yaml");
                if !ext_yaml.exists() {
                    return None;
                }
                let content = std::fs::read_to_string(&ext_yaml).ok()?;
                let ext: sindri_core::types::Extension = serde_yaml::from_str(&content).ok()?;
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
        args.name, installed_ext.version
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

    // 5. Mark as "removing" in manifest
    manifest.mark_removing(&args.name)
        .context("Failed to mark extension as removing")?;

    // 6. Execute removal operations
    if let Some(ext) = &extension {
        if let Some(remove_config) = &ext.remove {
            output::info("Executing removal operations...");

            // Remove mise configuration
            if let Some(mise_remove) = &remove_config.mise {
                output::info("Removing mise configuration...");
                let home = dirs::home_dir()
                    .ok_or_else(|| anyhow!("Could not determine home directory"))?;

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
                    if let Some(home) = dirs::home_dir() {
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

    // 7. Remove from manifest
    manifest.remove(&args.name)
        .context("Failed to remove extension from manifest")?;

    // 8. Remove extension directory
    if ext_version_dir.exists() {
        tokio::fs::remove_dir_all(&ext_version_dir)
            .await
            .context("Failed to remove extension directory")?;
    }

    output::success(&format!("Successfully removed extension: {}", args.name));

    Ok(())
}

// ============================================================================
// Versions Command
// ============================================================================

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
async fn versions(args: ExtensionVersionsArgs) -> Result<()> {
    use semver::{Version, VersionReq};
    use sindri_extensions::{ExtensionDistributor, ManifestManager};

    output::info(&format!("Available versions for extension: {}", args.name));

    // 1. Initialize ExtensionDistributor
    let cache_dir = get_cache_dir()?;
    let extensions_dir = get_extensions_dir()?;
    let cli_version = get_cli_version()?;

    let distributor = ExtensionDistributor::new(cache_dir, extensions_dir, cli_version.clone())
        .context("Failed to initialize extension distributor")?;

    // 2. Fetch compatibility matrix
    let matrix = distributor
        .get_compatibility_matrix()
        .await
        .context("Failed to fetch compatibility matrix")?;

    // 3. Get compatible version range for current CLI
    let compatible_range = {
        let cli_pattern = format!("{}.{}.x", cli_version.major, cli_version.minor);
        let compat = matrix.cli_versions.get(&cli_pattern).ok_or_else(|| {
            anyhow!("CLI version {} not found in compatibility matrix", cli_version)
        })?;

        let range_str = compat.compatible_extensions.get(&args.name).ok_or_else(|| {
            anyhow!(
                "Extension '{}' not found in compatibility matrix for CLI {}",
                args.name,
                cli_pattern
            )
        })?;

        VersionReq::parse(range_str).context(format!("Invalid version requirement: {}", range_str))?
    };

    // 4. Get installed version
    let manifest = ManifestManager::load_default()
        .context("Failed to load manifest")?;
    let installed_version = manifest.get_version(&args.name);

    // 5. For now, we'll show a simplified version list
    // TODO: Full implementation would require exposing release listing from distributor
    let latest_compatible = distributor
        .find_latest_compatible(&args.name, &compatible_range)
        .await
        .context("Failed to find latest compatible version")?;

    let mut version_rows = vec![
        VersionRow {
            version: latest_compatible.to_string(),
            compatible: "yes".to_string(),
            status: if installed_version.map(|v| v == &latest_compatible.to_string()).unwrap_or(false) {
                "installed (latest)".to_string()
            } else {
                "latest".to_string()
            },
            released: "available".to_string(),
        }
    ];

    // Add installed version if different
    if let Some(installed_ver) = installed_version {
        if installed_ver != &latest_compatible.to_string() {
            version_rows.push(VersionRow {
                version: installed_ver.to_string(),
                compatible: if compatible_range.matches(&Version::parse(installed_ver)?) {
                    "yes".to_string()
                } else {
                    "no".to_string()
                },
                status: "installed".to_string(),
                released: "-".to_string(),
            });
        }
    }

    // 7. Output results
    if args.json {
        let json_output = serde_json::json!({
            "extension": args.name,
            "cli_version": cli_version.to_string(),
            "compatible_range": compatible_range.to_string(),
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
        if version_rows.is_empty() {
            output::warn(&format!("No versions found for extension '{}'", args.name));
            return Ok(());
        }

        let table = Table::new(version_rows);
        println!("{}", table);

        println!();
        output::info(&format!("Compatible range: {}", compatible_range));
        output::info(&format!("Current CLI version: {}", cli_version));
    }

    Ok(())
}

// ============================================================================
// Check Command
// ============================================================================

#[derive(Tabled, serde::Serialize)]
struct UpdateRow {
    name: String,
    current: String,
    available: String,
    status: String,
}

/// Check for available extension updates
///
/// Supports:
/// - Check all: `sindri extension check`
/// - Check specific: `sindri extension check python nodejs`
/// - JSON output: `sindri extension check --json`
async fn check(args: ExtensionCheckArgs) -> Result<()> {
    use sindri_extensions::{ExtensionDistributor, ExtensionRegistry, ManifestManager};

    if args.extensions.is_empty() {
        output::info("Checking for updates to all installed extensions");
    } else {
        output::info(&format!(
            "Checking for updates to: {}",
            args.extensions.join(", ")
        ));
    }

    // 1. Load manifest to get installed extensions
    let manifest = ManifestManager::load_default().context("Failed to load manifest")?;

    // 2. Get installed extensions (filter by specified names if provided)
    let installed: Vec<_> = if args.extensions.is_empty() {
        manifest.list_installed()
    } else {
        manifest
            .list_installed()
            .into_iter()
            .filter(|(name, _)| args.extensions.contains(&name.to_string()))
            .collect()
    };

    if installed.is_empty() {
        output::warning("No extensions installed");
        return Ok(());
    }

    // 3. Initialize ExtensionDistributor and Registry
    let cache_dir = get_cache_dir()?;
    let extensions_dir = get_extensions_dir()?;
    let cli_version = get_cli_version()?;

    let distributor = ExtensionDistributor::new(cache_dir.clone(), extensions_dir, cli_version)
        .context("Failed to initialize extension distributor")?;

    // Load registry to get latest versions
    let spinner = output::spinner("Fetching extension registry...");
    let registry = ExtensionRegistry::load_from_github(cache_dir, "main")
        .await
        .context("Failed to load extension registry")?;
    spinner.finish_and_clear();

    // 4. Check each extension for updates
    let mut updates = Vec::new();
    let spinner = output::spinner("Checking for updates...");

    // Get compatibility matrix for version checks
    let matrix = distributor
        .get_compatibility_matrix()
        .await
        .context("Failed to fetch compatibility matrix")?;

    for (name, ext) in installed {
        let current_version = Version::parse(&ext.version).context(format!(
            "Invalid version in manifest for {}: {}",
            name, ext.version
        ))?;

        // Check if extension exists in registry
        if registry.get_entry(name).is_none() {
            updates.push(UpdateRow {
                name: name.to_string(),
                current: current_version.to_string(),
                available: "-".to_string(),
                status: "not found in registry".to_string(),
            });
            continue;
        }

        // Get latest compatible version using distributor
        match distributor.get_compatible_range(&matrix, name) {
            Ok(version_req) => {
                match distributor.find_latest_compatible(name, &version_req).await {
                    Ok(latest_version) => {
                        let status = if latest_version > current_version {
                            "update available"
                        } else {
                            "up to date"
                        };

                        updates.push(UpdateRow {
                            name: name.to_string(),
                            current: current_version.to_string(),
                            available: latest_version.to_string(),
                            status: status.to_string(),
                        });
                    }
                    Err(e) => {
                        updates.push(UpdateRow {
                            name: name.to_string(),
                            current: current_version.to_string(),
                            available: "-".to_string(),
                            status: format!("error: {}", e),
                        });
                    }
                }
            }
            Err(e) => {
                updates.push(UpdateRow {
                    name: name.to_string(),
                    current: current_version.to_string(),
                    available: "-".to_string(),
                    status: format!("error: {}", e),
                });
            }
        }
    }

    spinner.finish_and_clear();

    // 5. Show results
    if args.json {
        let json_output =
            serde_json::to_string_pretty(&updates).context("Failed to serialize to JSON")?;
        println!("{}", json_output);
    } else {
        let table = Table::new(&updates).to_string();
        println!("\n{}", table);

        let available_count = updates.iter().filter(|u| u.status == "update available").count();
        if available_count > 0 {
            output::info(&format!("{} update(s) available", available_count));
        } else {
            output::success("All extensions are up to date");
        }
    }

    Ok(())
}

// ============================================================================
// Rollback Command
// ============================================================================

/// Rollback an extension to a previous version
///
/// Restores the extension to its previous installed version
/// from the manifest history
async fn rollback(args: ExtensionRollbackArgs) -> Result<()> {
    output::info(&format!("Rolling back extension: {}", args.name));

    // TODO: Implement rollback functionality
    // 1. Load manifest and get previous version from history
    // 2. Check if previous version exists
    // 3. Confirm with user
    // 4. Install previous version
    // 5. Update manifest

    output::error("Rollback functionality not yet implemented");
    Err(anyhow!("Rollback not yet implemented"))
}

// ============================================================================
// Helper Functions
// ============================================================================
// Note: Helper functions are defined at the top of the file (lines 27-43)
