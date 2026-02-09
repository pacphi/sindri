//! Bill of Materials (BOM) management commands

use anyhow::{anyhow, Context, Result};
use console::style;
use std::collections::HashMap;
use tabled::{settings::Style as TableStyle, Table, Tabled};

use crate::cli::{BomCommands, BomExportArgs, BomGenerateArgs, BomListArgs, BomShowArgs};
use crate::utils::{get_cache_dir, get_extensions_dir, get_manifest_path};

use sindri_core::types::{Extension, InstallManifest};
use sindri_extensions::bom::{BillOfMaterials, BomFormat, BomGenerator, ComponentType};
use sindri_extensions::manifest::ManifestManager;
use sindri_extensions::registry::ExtensionRegistry;

/// Main entry point for BOM commands
pub async fn run(cmd: BomCommands) -> Result<()> {
    match cmd {
        BomCommands::Generate(args) => generate(args).await,
        BomCommands::Show(args) => show(args).await,
        BomCommands::List(args) => list(args).await,
        BomCommands::Export(args) => export(args).await,
    }
}

/// Generate BOM from installed extensions
async fn generate(args: BomGenerateArgs) -> Result<()> {
    // Load manifest
    let manifest_mgr =
        ManifestManager::new(get_manifest_path()?).context("Failed to load manifest")?;

    if manifest_mgr.extensions().is_empty() {
        println!("No extensions installed");
        return Ok(());
    }

    // Load registry
    let mut registry = load_registry().await?;

    // Load extension definitions for installed extensions
    let _loaded_count = load_extension_definitions(&mut registry, manifest_mgr.extensions())?;

    // Generate BOM
    let cli_version = env!("CARGO_PKG_VERSION").to_string();
    let generator = BomGenerator::new(cli_version, "default".to_string());
    let mut bom = generator.generate_from_manifest(manifest_mgr.manifest(), &registry)?;

    // Optionally detect versions
    if args.detect_versions {
        detect_and_update_versions(&mut bom, &registry, manifest_mgr.manifest()).await?;
    }

    if args.json {
        // Output as JSON
        let json = serde_json::to_string_pretty(&bom)?;
        println!("{}", json);
    } else {
        // Display human-readable summary
        display_bom_summary(&bom);
    }

    Ok(())
}

/// Show BOM for specific extension
async fn show(args: BomShowArgs) -> Result<()> {
    // Load manifest
    let manifest_mgr =
        ManifestManager::new(get_manifest_path()?).context("Failed to load manifest")?;

    // Check if extension is installed
    if !manifest_mgr.extensions().contains_key(&args.extension) {
        return Err(anyhow!(
            "Extension '{}' is not installed. Run 'sindri extension install {}' first.",
            args.extension,
            args.extension
        ));
    }

    // Load registry
    let mut registry = load_registry().await?;

    // Load extension definitions
    let _loaded_count = load_extension_definitions(&mut registry, manifest_mgr.extensions())?;

    // Get extension definition
    let ext = registry
        .get_extension(&args.extension)
        .ok_or_else(|| anyhow!("Extension not found in registry: {}", args.extension))?;

    // Extract components
    let cli_version = env!("CARGO_PKG_VERSION").to_string();
    let generator = BomGenerator::new(cli_version, "default".to_string());
    let components = extract_components_from_extension(&generator, ext)?;

    // Get extension entry for metadata
    let entry = registry
        .get_entry(&args.extension)
        .ok_or_else(|| anyhow!("Extension entry not found: {}", args.extension))?;

    if args.json {
        // Output as JSON
        let json = serde_json::json!({
            "name": args.extension,
            "version": manifest_mgr.extensions().get(&args.extension).unwrap().version,
            "category": format!("{:?}", ext.metadata.category),
            "components": components,
            "dependencies": entry.dependencies,
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        // Display human-readable output
        display_extension_bom(&args.extension, ext, &components, &entry.dependencies);
    }

    Ok(())
}

/// List all components
async fn list(args: BomListArgs) -> Result<()> {
    // Load manifest
    let manifest_mgr =
        ManifestManager::new(get_manifest_path()?).context("Failed to load manifest")?;

    if manifest_mgr.extensions().is_empty() {
        println!("No extensions installed");
        return Ok(());
    }

    // Load registry
    let mut registry = load_registry().await?;

    // Load extension definitions
    let _loaded_count = load_extension_definitions(&mut registry, manifest_mgr.extensions())?;

    // Collect all components
    let mut all_components = Vec::new();
    let cli_version = env!("CARGO_PKG_VERSION").to_string();
    let generator = BomGenerator::new(cli_version, "default".to_string());

    for name in manifest_mgr.extensions().keys() {
        // Filter by extension name if specified
        if let Some(ref filter_ext) = args.extension {
            if name != filter_ext {
                continue;
            }
        }

        let ext = registry
            .get_extension(name)
            .ok_or_else(|| anyhow!("Extension not found: {}", name))?;

        let components = extract_components_from_extension(&generator, ext)?;

        for comp in components {
            // Filter by component type if specified
            if let Some(ref filter_type) = args.component_type {
                let comp_type_str = format!("{:?}", comp.component_type).to_lowercase();
                if !comp_type_str.contains(&filter_type.to_lowercase()) {
                    continue;
                }
            }

            all_components.push(ComponentRow {
                extension: name.to_string(),
                name: comp.name.clone(),
                version: comp.version.clone(),
                component_type: format!("{:?}", comp.component_type).to_lowercase(),
                license: comp.license.clone().unwrap_or_else(|| "-".to_string()),
            });
        }
    }

    if all_components.is_empty() {
        println!("No components found matching the filters");
        return Ok(());
    }

    // Sort by extension, then name
    all_components.sort_by(|a, b| a.extension.cmp(&b.extension).then(a.name.cmp(&b.name)));

    if args.json {
        let json = serde_json::to_string_pretty(&all_components)?;
        println!("{}", json);
    } else {
        let table = Table::new(&all_components)
            .with(TableStyle::rounded())
            .to_string();
        println!("{}", table);
        println!(
            "\n{} components found",
            style(all_components.len()).green().bold()
        );
    }

    Ok(())
}

/// Export BOM to file
async fn export(args: BomExportArgs) -> Result<()> {
    // Check if output file exists
    let output_path = std::path::Path::new(args.output.as_str());
    if output_path.exists() && !args.force {
        return Err(anyhow!(
            "Output file '{}' already exists. Use --force to overwrite.",
            args.output
        ));
    }

    // Load manifest
    let manifest_mgr =
        ManifestManager::new(get_manifest_path()?).context("Failed to load manifest")?;

    if manifest_mgr.extensions().is_empty() {
        return Err(anyhow!("No extensions installed. Nothing to export."));
    }

    // Load registry
    let mut registry = load_registry().await?;

    // Load extension definitions for installed extensions
    let _loaded_count = load_extension_definitions(&mut registry, manifest_mgr.extensions())?;

    // Generate BOM
    let cli_version = env!("CARGO_PKG_VERSION").to_string();
    let generator = BomGenerator::new(cli_version, "default".to_string());
    let mut bom = generator.generate_from_manifest(manifest_mgr.manifest(), &registry)?;

    // Optionally detect versions
    if args.detect_versions {
        detect_and_update_versions(&mut bom, &registry, manifest_mgr.manifest()).await?;
    }

    // Parse format
    let format = match args.format.to_lowercase().as_str() {
        "json" => BomFormat::Json,
        "yaml" => BomFormat::Yaml,
        "cyclonedx" => BomFormat::CycloneDx,
        "spdx" => BomFormat::Spdx,
        _ => {
            return Err(anyhow!(
                "Unsupported format '{}'. Supported formats: json, yaml, cyclonedx, spdx",
                args.format
            ));
        }
    };

    // Write BOM to file
    generator.write_bom(&bom, output_path, format)?;

    println!(
        "{} BOM exported to {} in {} format",
        style("✓").green().bold(),
        style(&args.output).cyan(),
        style(&args.format).yellow()
    );
    println!(
        "  {} extensions, {} total components",
        bom.extensions.len(),
        bom.total_components
    );

    Ok(())
}

// Helper functions

/// Load extension registry
async fn load_registry() -> Result<ExtensionRegistry> {
    let cache_dir = get_cache_dir()?;
    ExtensionRegistry::load_from_github(cache_dir, "main")
        .await
        .context("Failed to load extension registry")
}

/// Load extension definitions from disk for installed extensions
///
/// Handles multiple deployment modes:
/// 1. Development mode: v3/extensions/{name}/extension.yaml (flat, source tree)
/// 2. Bundled mode: /opt/sindri/extensions/{name}/extension.yaml (flat, baked into image)
/// 3. Downloaded mode: ~/.sindri/extensions/{name}/{version}/extension.yaml (versioned)
fn load_extension_definitions(
    registry: &mut ExtensionRegistry,
    installed: &HashMap<String, sindri_core::types::InstalledExtension>,
) -> Result<usize> {
    let mut extensions_dir = get_extensions_dir()?;

    // Fallback: If extensions_dir doesn't exist, try source tree (development mode)
    if !extensions_dir.exists() {
        // Try multiple possible source tree locations
        let possible_paths = vec![
            std::path::PathBuf::from("extensions"),    // Running from v3/
            std::path::PathBuf::from("v3/extensions"), // Running from project root
            std::path::PathBuf::from("../extensions"), // Running from v3/crates/sindri
        ];

        let mut found = false;
        for path in possible_paths {
            if path.exists() {
                extensions_dir = path;
                tracing::info!("Using source tree extensions: {:?}", extensions_dir);
                found = true;
                break;
            }
        }

        if !found {
            tracing::warn!(
                "Extensions directory not found: {:?}",
                get_extensions_dir()?
            );
            return Ok(0);
        }
    }

    let mut loaded_count = 0;

    // Detect deployment mode by checking if extensions_dir contains flat or versioned structure
    let is_flat_mode = {
        // Check if any extension exists as a direct subdirectory with extension.yaml
        installed
            .keys()
            .any(|name| extensions_dir.join(name).join("extension.yaml").exists())
    };

    tracing::debug!(
        "Extensions directory: {:?}, flat mode: {}",
        extensions_dir,
        is_flat_mode
    );

    for (name, ext_info) in installed {
        let ext_path = if is_flat_mode {
            // Flat structure (development/bundled mode)
            let path = extensions_dir.join(name).join("extension.yaml");
            if path.exists() {
                Some(path)
            } else {
                None
            }
        } else {
            // Versioned structure (downloaded mode)
            // Try both the installed version and "latest" as fallback
            let paths = vec![
                extensions_dir
                    .join(name)
                    .join(&ext_info.version)
                    .join("extension.yaml"),
                extensions_dir
                    .join(name)
                    .join("latest")
                    .join("extension.yaml"),
            ];

            paths.into_iter().find(|p| p.exists())
        };

        let ext_path = match ext_path {
            Some(p) => p,
            None => {
                tracing::debug!(
                    "Extension definition not found for: {} (version: {})",
                    name,
                    ext_info.version
                );
                continue;
            }
        };

        // Load and parse extension definition
        let content = std::fs::read_to_string(&ext_path)
            .with_context(|| format!("Failed to read extension.yaml for {}", name))?;

        let extension: Extension = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse extension.yaml for {}", name))?;

        // Add to registry
        registry.extensions.insert(name.clone(), extension);
        loaded_count += 1;
        tracing::debug!("Loaded extension definition for: {}", name);
    }

    tracing::info!(
        "Loaded {} of {} extension definitions",
        loaded_count,
        installed.len()
    );
    Ok(loaded_count)
}

/// Extract components from extension
fn extract_components_from_extension(
    _generator: &BomGenerator,
    extension: &Extension,
) -> Result<Vec<sindri_extensions::bom::Component>> {
    // Use reflection to call the private method (workaround)
    // In reality, we'll need to make this public or duplicate the logic
    // For now, extract from BOM config
    let mut components = Vec::new();

    if let Some(bom_config) = &extension.bom {
        for tool in &bom_config.tools {
            let component_type = match &tool.r#type {
                Some(t) => match format!("{:?}", t).to_lowercase().as_str() {
                    s if s.contains("runtime") => ComponentType::Runtime,
                    s if s.contains("library") => ComponentType::Library,
                    s if s.contains("package") => ComponentType::Package,
                    _ => ComponentType::Tool,
                },
                None => ComponentType::Tool,
            };

            let mut metadata = HashMap::new();
            if let Some(purl) = &tool.purl {
                metadata.insert("purl".to_string(), purl.clone());
            }
            if let Some(cpe) = &tool.cpe {
                metadata.insert("cpe".to_string(), cpe.clone());
            }
            metadata.insert("source_method".to_string(), format!("{:?}", tool.source));

            components.push(sindri_extensions::bom::Component {
                name: tool.name.clone(),
                version: tool
                    .version
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
                component_type,
                license: tool.license.clone(),
                source: tool.homepage.clone().or_else(|| tool.download_url.clone()),
                install_path: None,
                metadata,
            });
        }
    }

    Ok(components)
}

/// Detect and update versions by running validation commands
async fn detect_and_update_versions(
    bom: &mut BillOfMaterials,
    _registry: &ExtensionRegistry,
    _manifest: &InstallManifest,
) -> Result<()> {
    println!("{}", style("Detecting versions...").cyan());

    for ext_bom in &mut bom.extensions {
        // Parse mise.toml if it exists (for mise-based extensions)
        if let Some(mise_versions) = parse_mise_toml(&ext_bom.name).await? {
            for component in &mut ext_bom.components {
                if let Some(version) = mise_versions.get(&component.name) {
                    if component.version == "unknown" || component.version == "dynamic" {
                        component.version = version.clone();
                    }
                }
            }
        }
    }

    Ok(())
}

/// Parse mise.toml to extract explicit versions
async fn parse_mise_toml(extension_name: &str) -> Result<Option<HashMap<String, String>>> {
    let extensions_dir = get_extensions_dir()?;
    let mise_path = extensions_dir.join(extension_name).join("mise.toml");

    if !mise_path.exists() {
        return Ok(None);
    }

    let content = tokio::fs::read_to_string(&mise_path).await?;
    let mut versions = HashMap::new();

    // Parse [tools] section
    let mut in_tools_section = false;
    for line in content.lines() {
        let line = line.trim();

        if line == "[tools]" {
            in_tools_section = true;
            continue;
        }

        if line.starts_with('[') && line != "[tools]" {
            in_tools_section = false;
        }

        if in_tools_section && line.contains('=') {
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() == 2 {
                let tool = parts[0].trim().to_string();
                let version = parts[1].trim().trim_matches('"').to_string();
                versions.insert(tool, version);
            }
        }
    }

    Ok(if versions.is_empty() {
        None
    } else {
        Some(versions)
    })
}

/// Display BOM summary
fn display_bom_summary(bom: &BillOfMaterials) {
    println!("\n{}", style("Bill of Materials").bold().underlined());
    println!("Schema Version: {}", bom.schema_version);
    println!("CLI Version: {}", bom.cli_version);
    println!(
        "Generated: {}",
        bom.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
    );
    println!("Config: {}", bom.config_name);
    println!();

    println!(
        "{} {} extensions installed",
        style("✓").green().bold(),
        style(bom.extensions.len()).cyan().bold()
    );
    println!(
        "{} {} total components",
        style("✓").green().bold(),
        style(bom.total_components).cyan().bold()
    );
    println!();

    // Show extensions
    for ext in &bom.extensions {
        println!(
            "  {} {} ({})",
            style("•").cyan(),
            style(&ext.name).bold(),
            style(&ext.version).dim()
        );
        println!("    {} components", ext.components.len());
    }
}

/// Display extension BOM
fn display_extension_bom(
    name: &str,
    ext: &Extension,
    components: &[sindri_extensions::bom::Component],
    dependencies: &[String],
) {
    println!(
        "\n{} {}",
        style("Extension:").bold(),
        style(name).cyan().bold()
    );
    println!("{} {:?}", style("Category:").bold(), ext.metadata.category);
    println!(
        "{} {:?}",
        style("Install Method:").bold(),
        ext.install.method
    );

    if !dependencies.is_empty() {
        println!("\n{}", style("Dependencies:").bold());
        for dep in dependencies {
            println!("  {} {}", style("•").cyan(), dep);
        }
    }

    println!("\n{} ({})", style("Components:").bold(), components.len());

    if !components.is_empty() {
        #[derive(Tabled)]
        struct ComponentDisplay {
            name: String,
            version: String,
            #[tabled(rename = "type")]
            component_type: String,
            license: String,
        }

        let rows: Vec<ComponentDisplay> = components
            .iter()
            .map(|c| ComponentDisplay {
                name: c.name.clone(),
                version: c.version.clone(),
                component_type: format!("{:?}", c.component_type).to_lowercase(),
                license: c.license.clone().unwrap_or_else(|| "-".to_string()),
            })
            .collect();

        let table = Table::new(&rows).with(TableStyle::rounded()).to_string();
        println!("{}", table);
    }
}

/// Component row for list output
#[derive(Tabled, serde::Serialize)]
struct ComponentRow {
    extension: String,
    name: String,
    version: String,
    #[tabled(rename = "type")]
    component_type: String,
    license: String,
}
