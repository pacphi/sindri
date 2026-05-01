use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};
use sindri_core::well_known::{bom_schema_url, PROJECT_MANIFEST_FILENAME, PROJECT_POLICY_FILENAME};
use std::fs;
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Built-in templates recognized by `sindri init`.
///
/// Single source of truth: both the template-expansion match arm and the
/// unknown-template error message read from this slice (F-CLI-11).
pub const TEMPLATES: &[&str] = &["minimal", "anthropic-dev"];

/// Built-in policy presets.
const POLICY_PRESETS: &[&str] = &["default", "strict", "offline", "none"];

#[derive(Debug, Error)]
pub enum InitError {
    #[error(
        "Unknown template '{requested}'. Available templates: {}",
        available.join(", ")
    )]
    UnknownTemplate {
        requested: String,
        available: Vec<&'static str>,
    },
    #[error(
        "Unknown policy preset '{requested}'. Valid presets: {}",
        available.join(", ")
    )]
    UnknownPreset {
        requested: String,
        available: Vec<&'static str>,
    },
    #[error("sindri.yaml already exists. Use --force to overwrite.")]
    ManifestExists,
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("Failed to serialize: {0}")]
    Serialize(String),
}

pub struct InitArgs {
    pub template: Option<String>,
    pub name: Option<String>,
    pub policy: Option<String>,
    pub non_interactive: bool,
    pub force: bool,
    /// When true, write the policy preset to the global file
    /// (`~/.sindri/policy.yaml`) instead of `./sindri.policy.yaml`.
    /// (F-CLI-09 escape hatch.)
    pub global: bool,
}

pub fn run(args: InitArgs) -> i32 {
    match run_inner(args) {
        Ok(()) => EXIT_SUCCESS,
        Err(e) => {
            eprintln!("{}", e);
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        }
    }
}

fn run_inner(args: InitArgs) -> Result<(), InitError> {
    let manifest_path = Path::new(PROJECT_MANIFEST_FILENAME);
    if manifest_path.exists() && !args.force {
        return Err(InitError::ManifestExists);
    }

    // Decide whether prompts are appropriate.
    let interactive = !args.non_interactive && io::stdin().is_terminal();

    // Resolve project name.
    let default_name = std::env::current_dir()
        .ok()
        .and_then(|d| d.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "my-project".to_string());
    let name = match args.name {
        Some(n) => n,
        None if interactive => prompt_string("Project name", &default_name)?,
        None => default_name,
    };

    // Resolve template (validate even when the flag is given).
    let template = match args.template.as_deref() {
        Some(t) => {
            ensure_known(t, TEMPLATES, |req, av| InitError::UnknownTemplate {
                requested: req,
                available: av,
            })?;
            t.to_string()
        }
        None if interactive => prompt_select("Template", TEMPLATES, 0)?,
        None => "minimal".to_string(),
    };

    // Resolve policy preset (None = leave policy file alone).
    let policy_preset = match args.policy.as_deref() {
        Some(p) => {
            ensure_known(p, POLICY_PRESETS, |req, av| InitError::UnknownPreset {
                requested: req,
                available: av,
            })?;
            Some(p.to_string())
        }
        None if interactive => {
            let chosen = prompt_select("Policy preset", POLICY_PRESETS, 0)?;
            Some(chosen)
        }
        None => None,
    };

    // Write sindri.yaml with the transitional schema pragmas (ADR-013).
    let components = template_components(&template);
    let schema = bom_schema_url();
    let manifest_content = format!(
        r#"# yaml-language-server: $schema={schema}
# @schema {{ "id": "{schema}" }}
name: {name}

registry:
  sources:
    - type: oci
      url: oci://ghcr.io/sindri-dev/registry-core  # Replace with your registry URL
      tag: "2026.04"

components:
{components}

preferences:
  backend_order: {{}}
"#,
        schema = schema,
        name = name,
        components = components,
    );
    fs::write(manifest_path, &manifest_content)?;

    // .gitignore: per ADR-029 + F-CLI-10, lockfiles are committed
    // (Cargo.lock semantics for binary projects). Only ignore .sindri/.
    append_gitignore()?;

    // Policy file: when a preset is selected, write it. `none` is a
    // sentinel meaning "do not create a policy file."
    if let Some(preset) = policy_preset {
        if preset != "none" {
            let policy = parse_preset(&preset)?;
            let target_desc = if args.global {
                sindri_policy::write_global_preset(&policy)
                    .map_err(|e| InitError::Serialize(e.to_string()))?;
                sindri_policy::loader::global_policy_path()
                    .display()
                    .to_string()
            } else {
                let project_path = PathBuf::from(PROJECT_POLICY_FILENAME);
                sindri_policy::write_project_preset(&policy, &project_path)
                    .map_err(|e| InitError::Serialize(e.to_string()))?;
                PROJECT_POLICY_FILENAME.to_string()
            };
            println!("Policy preset '{}' written to {}", preset, target_desc);
        }
    }

    println!("Created sindri.yaml for project '{}'", name);
    println!("Next steps:");
    println!("  sindri registry refresh core <registry-url>");
    println!("  sindri resolve");
    println!("  sindri apply");

    Ok(())
}

fn ensure_known<F>(value: &str, allowed: &[&'static str], err: F) -> Result<(), InitError>
where
    F: FnOnce(String, Vec<&'static str>) -> InitError,
{
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(err(value.to_string(), allowed.to_vec()))
    }
}

fn template_components(template: &str) -> String {
    match template {
        "anthropic-dev" => {
            "  - address: \"mise:nodejs\"\n  - address: \"mise:python\"\n  - address: \"binary:gh\"\n  - address: \"npm:claude-code\""
                .to_string()
        }
        // "minimal" and any future single-component templates.
        _ => "  - address: \"mise:nodejs\"".to_string(),
    }
}

fn append_gitignore() -> Result<(), InitError> {
    let gitignore = Path::new(".gitignore");
    let entry = "\n# Sindri state\n.sindri/\n";
    if gitignore.exists() {
        let content = fs::read_to_string(gitignore).unwrap_or_default();
        if content.contains(".sindri/") {
            return Ok(());
        }
        let mut f = fs::OpenOptions::new().append(true).open(gitignore)?;
        f.write_all(entry.as_bytes())?;
    } else {
        fs::write(gitignore, format!("{}\n", entry.trim()))?;
    }
    Ok(())
}

fn parse_preset(s: &str) -> Result<sindri_core::policy::PolicyPreset, InitError> {
    match s {
        "default" => Ok(sindri_core::policy::PolicyPreset::Default),
        "strict" => Ok(sindri_core::policy::PolicyPreset::Strict),
        "offline" => Ok(sindri_core::policy::PolicyPreset::Offline),
        // "none" is filtered upstream; any other value is caught earlier
        // by ensure_known. Defensive: still surface a clear error.
        other => Err(InitError::UnknownPreset {
            requested: other.to_string(),
            available: POLICY_PRESETS.to_vec(),
        }),
    }
}

// =============================================================================
// Stdin-backed prompt helpers (F-CLI-08).
//
// Hand-rolled rather than pulling in `dialoguer`: only three prompt sites,
// and a numbered-list selector keeps the dependency surface flat. If we
// add many more interactive flows later, revisit and adopt a TTY library.
// =============================================================================

fn prompt_string(label: &str, default: &str) -> Result<String, InitError> {
    print!("{} [{}]: ", label, default);
    io::stdout().flush()?;
    let mut line = String::new();
    let n = io::stdin().lock().read_line(&mut line)?;
    if n == 0 {
        // EOF on stdin: caller fell off a pipe mid-flow. Use the default.
        return Ok(default.to_string());
    }
    let trimmed = line.trim();
    if trimmed.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

fn prompt_select(
    label: &str,
    options: &[&'static str],
    default_index: usize,
) -> Result<String, InitError> {
    println!("{}:", label);
    for (i, opt) in options.iter().enumerate() {
        let marker = if i == default_index { "*" } else { " " };
        println!("  {} {}) {}", marker, i + 1, opt);
    }
    print!("Selection [{}]: ", default_index + 1);
    io::stdout().flush()?;
    let mut line = String::new();
    let n = io::stdin().lock().read_line(&mut line)?;
    if n == 0 {
        return Ok(options[default_index].to_string());
    }
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(options[default_index].to_string());
    }
    // Accept either the option name or its 1-based index.
    if let Ok(idx) = trimmed.parse::<usize>() {
        if idx >= 1 && idx <= options.len() {
            return Ok(options[idx - 1].to_string());
        }
    }
    if options.contains(&trimmed) {
        return Ok(trimmed.to_string());
    }
    // Out-of-range / unknown: don't loop indefinitely in a non-test
    // environment; surface as the default with a notice on stderr.
    eprintln!(
        "warning: '{}' is not a valid choice; using default '{}'.",
        trimmed, options[default_index]
    );
    Ok(options[default_index].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_template_lists_available() {
        let err = ensure_known("bogus", TEMPLATES, |req, av| InitError::UnknownTemplate {
            requested: req,
            available: av,
        })
        .expect_err("should fail");
        let msg = format!("{}", err);
        assert!(msg.contains("bogus"));
        assert!(msg.contains("minimal"));
        assert!(msg.contains("anthropic-dev"));
    }

    #[test]
    fn unknown_preset_lists_available() {
        let err = ensure_known("paranoid", POLICY_PRESETS, |req, av| {
            InitError::UnknownPreset {
                requested: req,
                available: av,
            }
        })
        .expect_err("should fail");
        let msg = format!("{}", err);
        assert!(msg.contains("paranoid"));
        assert!(msg.contains("default"));
        assert!(msg.contains("strict"));
        assert!(msg.contains("offline"));
        assert!(msg.contains("none"));
    }

    #[test]
    fn known_template_passes() {
        for t in TEMPLATES {
            ensure_known(t, TEMPLATES, |req, av| InitError::UnknownTemplate {
                requested: req,
                available: av,
            })
            .expect("known template");
        }
    }

    #[test]
    fn parse_preset_accepts_all_named_presets() {
        assert!(parse_preset("default").is_ok());
        assert!(parse_preset("strict").is_ok());
        assert!(parse_preset("offline").is_ok());
        assert!(parse_preset("nope").is_err());
    }

    #[test]
    fn template_components_minimal_has_one_entry() {
        let body = template_components("minimal");
        assert!(body.contains("mise:nodejs"));
        assert_eq!(body.lines().count(), 1);
    }

    #[test]
    fn template_components_anthropic_has_four_entries() {
        let body = template_components("anthropic-dev");
        assert!(body.contains("mise:nodejs"));
        assert!(body.contains("mise:python"));
        assert!(body.contains("binary:gh"));
        assert!(body.contains("npm:claude-code"));
    }
}
