//! Extension validate command

use anyhow::{anyhow, Context, Result};
use sindri_core::types::ExtensionState;
use sindri_extensions::StatusLedger;

use crate::cli::ExtensionValidateArgs;
use crate::output;
use crate::utils::{get_cache_dir, get_extensions_dir};

/// Validate an extension against JSON schema
///
/// Supports:
/// - Validate by name: `sindri extension validate python`
/// - Validate file: `sindri extension validate --file extension.yaml`
///
/// Performs full validation:
/// 1. Load extension.yaml (from file or registry)
/// 2. Validate against extension.schema.json
/// 3. Check all required fields
/// 4. Validate dependency references
/// 5. Check for conflicts with installed extensions
pub(super) async fn run(args: ExtensionValidateArgs) -> Result<()> {
    use sindri_extensions::{DependencyResolver, ExtensionRegistry, ExtensionValidator};
    use std::collections::HashSet;
    use tracing::debug;

    // Determine if we're validating a file or a registry extension
    let is_file = args.file.is_some() || {
        let path = std::path::Path::new(&args.name);
        path.exists() && path.is_file()
    };

    let validation_target = if let Some(file_path) = &args.file {
        format!("file: {}", file_path)
    } else if is_file {
        format!("file: {}", args.name)
    } else {
        format!("extension: {}", args.name)
    };

    output::info(&format!("Validating {}", validation_target));

    // Initialize schema validator
    let schema_validator = sindri_core::schema::SchemaValidator::new()
        .context("Failed to initialize schema validator")?;
    let extension_validator = ExtensionValidator::new(&schema_validator);

    // Load registry for dependency/conflict validation
    let cache_dir = get_cache_dir()?;
    let spinner = output::spinner("Loading extension registry...");
    let registry = ExtensionRegistry::load_from_github(cache_dir, "main")
        .await
        .context("Failed to load extension registry")?;
    spinner.finish_and_clear();

    // Load the extension to validate
    let extension = if is_file {
        // Validate from file
        let file_path = if let Some(fp) = &args.file {
            fp.as_std_path().to_path_buf()
        } else {
            std::path::PathBuf::from(&args.name)
        };

        debug!("Validating extension file: {:?}", file_path);
        extension_validator
            .validate_file(&file_path)
            .context("Schema and semantic validation failed")?
    } else {
        // Validate from registry - need to fetch extension definition
        let extensions_dir = get_extensions_dir()?;
        let ext_dir = extensions_dir.join(&args.name);

        // Check if extension exists in registry
        if !registry.has_extension(&args.name) {
            return Err(anyhow!(
                "Extension '{}' not found in registry. Use --file to validate a local file.",
                args.name
            ));
        }

        // Try to load from installed location first
        let extension_yaml = if let Ok(ledger) = StatusLedger::load_default() {
            if let Ok(status_map) = ledger.get_all_latest_status() {
                if let Some(status) = status_map
                    .get(&args.name)
                    .filter(|s| s.current_state == ExtensionState::Installed)
                {
                    if let Some(version) = &status.version {
                        let version_dir = ext_dir.join(version);
                        let yaml_path = version_dir.join("extension.yaml");
                        if yaml_path.exists() {
                            Some(yaml_path)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        if let Some(yaml_path) = extension_yaml {
            debug!("Validating installed extension from: {:?}", yaml_path);
            extension_validator
                .validate_file(&yaml_path)
                .context("Schema and semantic validation failed")?
        } else {
            // Extension not installed locally, validate registry entry only
            output::info(&format!(
                "Extension '{}' not installed locally. Validating registry metadata only.",
                args.name
            ));

            // We can still validate dependencies and conflicts from registry entry
            let entry = registry
                .get_entry(&args.name)
                .ok_or_else(|| anyhow!("Extension '{}' not found in registry", args.name))?;

            output::success(&format!(
                "Registry entry for '{}' is valid (category: {}, description: {})",
                args.name, entry.category, entry.description
            ));

            // Validate dependencies exist
            validate_dependencies_from_registry(&args.name, &entry.dependencies, &registry)?;

            // Check for conflicts with installed extensions
            validate_conflicts_from_registry(&args.name, &entry.conflicts)?;

            return Ok(());
        }
    };

    output::success("Schema and semantic validation passed");

    // Validate dependency references exist in registry
    output::info("Checking dependency references...");
    let mut missing_deps = Vec::new();
    for dep in &extension.metadata.dependencies {
        if !registry.has_extension(dep) {
            missing_deps.push(dep.clone());
        }
    }

    if !missing_deps.is_empty() {
        output::error(&format!(
            "Missing dependencies in registry: {}",
            missing_deps.join(", ")
        ));
        return Err(anyhow!(
            "Extension has dependencies not found in registry: {}",
            missing_deps.join(", ")
        ));
    }
    output::success("All dependencies exist in registry");

    // Check for circular dependencies using DependencyResolver
    output::info("Checking for circular dependencies...");

    // Build a temporary registry with this extension for cycle detection
    let mut temp_registry = ExtensionRegistry::new();
    temp_registry
        .extensions
        .insert(extension.metadata.name.clone(), extension.clone());

    // Add dependencies from the main registry entries
    for dep_name in &extension.metadata.dependencies {
        if let Some(entry) = registry.get_entry(dep_name) {
            // Create a minimal extension for dependency checking
            let dep_ext = sindri_core::types::Extension {
                metadata: sindri_core::types::ExtensionMetadata {
                    name: dep_name.clone(),
                    version: "1.0.0".to_string(),
                    description: entry.description.clone(),
                    category: sindri_core::types::ExtensionCategory::Devops,
                    author: None,
                    homepage: None,
                    dependencies: entry.dependencies.clone(),
                },
                requirements: None,
                install: sindri_core::types::InstallConfig {
                    method: sindri_core::types::InstallMethod::Script,
                    mise: None,
                    apt: None,
                    binary: None,
                    npm: None,
                    script: None,
                },
                configure: None,
                validate: sindri_core::types::ValidateConfig {
                    commands: vec![],
                    mise: None,
                },
                remove: None,
                upgrade: None,
                capabilities: None,
                docs: None,
                bom: None,
            };
            temp_registry.extensions.insert(dep_name.clone(), dep_ext);
        }
    }

    let resolver = DependencyResolver::new(&temp_registry);
    match resolver.resolve(&extension.metadata.name) {
        Ok(order) => {
            debug!("Dependency resolution order: {:?}", order);
            output::success("No circular dependencies detected");
        }
        Err(e) => {
            output::error(&format!("Circular dependency error: {}", e));
            return Err(e);
        }
    }

    // Check for conflicts with installed extensions
    output::info("Checking for conflicts with installed extensions...");
    let ledger = StatusLedger::load_default().context("Failed to load status ledger")?;
    let status_map = ledger
        .get_all_latest_status()
        .context("Failed to get extension status")?;
    let installed: HashSet<String> = status_map
        .iter()
        .filter(|(_, s)| s.current_state == ExtensionState::Installed)
        .map(|(name, _)| name.to_string())
        .collect();

    // Get conflicts for this extension from registry
    let extension_conflicts = registry.get_conflicts(&extension.metadata.name);
    let mut active_conflicts = Vec::new();

    for conflict in &extension_conflicts {
        if installed.contains(conflict) {
            active_conflicts.push(conflict.clone());
        }
    }

    // Also check if any installed extension conflicts with this one
    for installed_name in &installed {
        let installed_conflicts = registry.get_conflicts(installed_name);
        if installed_conflicts.contains(&extension.metadata.name)
            && !active_conflicts.contains(installed_name)
        {
            active_conflicts.push(installed_name.to_string());
        }
    }

    if !active_conflicts.is_empty() {
        output::warning(&format!(
            "Conflicts with installed extensions: {}",
            active_conflicts.join(", ")
        ));
        output::warning(
            "Installing this extension may cause issues with the conflicting extensions",
        );
    } else {
        output::success("No conflicts with installed extensions");
    }

    output::success(&format!(
        "Extension '{}' v{} is valid",
        extension.metadata.name, extension.metadata.version
    ));

    Ok(())
}

/// Helper to validate dependencies from registry entry
fn validate_dependencies_from_registry(
    name: &str,
    dependencies: &[String],
    registry: &sindri_extensions::ExtensionRegistry,
) -> Result<()> {
    if dependencies.is_empty() {
        output::success("No dependencies to validate");
        return Ok(());
    }

    output::info("Checking dependency references...");
    let mut missing = Vec::new();

    for dep in dependencies {
        if !registry.has_extension(dep) {
            missing.push(dep.clone());
        }
    }

    if !missing.is_empty() {
        output::error(&format!(
            "Missing dependencies in registry: {}",
            missing.join(", ")
        ));
        return Err(anyhow!(
            "Extension '{}' has dependencies not found in registry: {}",
            name,
            missing.join(", ")
        ));
    }

    output::success("All dependencies exist in registry");
    Ok(())
}

/// Helper to validate conflicts with installed extensions
fn validate_conflicts_from_registry(name: &str, conflicts: &[String]) -> Result<()> {
    use std::collections::HashSet;

    if conflicts.is_empty() {
        output::success("No conflicts defined");
        return Ok(());
    }

    output::info("Checking for conflicts with installed extensions...");

    let ledger = match StatusLedger::load_default() {
        Ok(l) => l,
        Err(_) => {
            output::info("No ledger found, skipping conflict check");
            return Ok(());
        }
    };

    let status_map = match ledger.get_all_latest_status() {
        Ok(m) => m,
        Err(_) => {
            output::info("Failed to get extension status, skipping conflict check");
            return Ok(());
        }
    };

    let installed: HashSet<String> = status_map
        .iter()
        .filter(|(_, status)| status.current_state == ExtensionState::Installed)
        .map(|(n, _)| n.clone())
        .collect();

    let active_conflicts: Vec<_> = conflicts
        .iter()
        .filter(|c| installed.contains(*c))
        .cloned()
        .collect();

    if !active_conflicts.is_empty() {
        output::warning(&format!(
            "Extension '{}' conflicts with installed: {}",
            name,
            active_conflicts.join(", ")
        ));
    } else {
        output::success("No conflicts with installed extensions");
    }

    Ok(())
}
