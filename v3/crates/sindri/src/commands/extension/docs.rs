//! Extension docs command

use anyhow::{anyhow, Context, Result};
use sindri_extensions::StatusLedger;

use crate::cli::ExtensionDocsArgs;
use crate::utils::get_extensions_dir;

/// Generate documentation for an extension
///
/// Loads the extension.yaml and renders documentation to stdout using the
/// embedded Tera template.
///
/// Usage: `sindri extension docs golang`
pub(super) async fn run(args: ExtensionDocsArgs) -> Result<()> {
    // Try to find the extension.yaml file
    let extensions_dir = get_extensions_dir()?;

    // Try multiple locations: installed (versioned), installed (flat), source tree
    let extension_yaml_path = {
        let mut found = None;

        // 1. Flat structure (development/bundled mode)
        let flat_path = extensions_dir.join(&args.name).join("extension.yaml");
        if flat_path.exists() {
            found = Some(flat_path);
        }

        // 2. Try source tree locations (development mode)
        if found.is_none() {
            let source_paths = vec![
                std::path::PathBuf::from("extensions")
                    .join(&args.name)
                    .join("extension.yaml"),
                std::path::PathBuf::from("v3/extensions")
                    .join(&args.name)
                    .join("extension.yaml"),
                std::path::PathBuf::from("../extensions")
                    .join(&args.name)
                    .join("extension.yaml"),
            ];

            for path in source_paths {
                if path.exists() {
                    found = Some(path);
                    break;
                }
            }
        }

        // 3. Versioned structure (downloaded mode) - check status ledger
        if found.is_none() {
            if let Ok(ledger) = StatusLedger::load_default() {
                if let Ok(status_map) = ledger.get_all_latest_status() {
                    if let Some(version) =
                        status_map.get(&args.name).and_then(|s| s.version.clone())
                    {
                        let versioned_path = extensions_dir
                            .join(&args.name)
                            .join(version)
                            .join("extension.yaml");
                        if versioned_path.exists() {
                            found = Some(versioned_path);
                        }
                    }
                }
            }
        }

        found.ok_or_else(|| {
            anyhow!(
                "Extension '{}' not found. Checked installed and source tree locations.",
                args.name
            )
        })?
    };

    // Load and parse the extension
    let content = std::fs::read_to_string(&extension_yaml_path)
        .with_context(|| format!("Failed to read {}", extension_yaml_path.display()))?;

    let extension: sindri_core::types::Extension = serde_yaml_ng::from_str(&content)
        .with_context(|| format!("Failed to parse {}", extension_yaml_path.display()))?;

    // Render documentation
    let doc = sindri_core::templates::render_extension_doc(&extension)
        .context("Failed to render extension documentation")?;

    print!("{}", doc);

    Ok(())
}
