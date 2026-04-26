use std::fs;
use std::path::Path;
use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};

pub struct InitArgs {
    pub template: Option<String>,
    pub name: Option<String>,
    pub policy: Option<String>,
    pub non_interactive: bool,
    pub force: bool,
}

pub fn run(args: InitArgs) -> i32 {
    let manifest_path = Path::new("sindri.yaml");
    if manifest_path.exists() && !args.force {
        eprintln!("sindri.yaml already exists. Use --force to overwrite.");
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }

    let name = args.name.unwrap_or_else(|| {
        std::env::current_dir()
            .ok()
            .and_then(|d| d.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_else(|| "my-project".to_string())
    });

    let components = template_components(args.template.as_deref());
    let policy_preset = args.policy.as_deref().unwrap_or("default");

    // Generate sindri.yaml with ADR-013 YAML-LSP schema pragma
    let manifest_content = format!(
        r#"# yaml-language-server: $schema=https://schemas.sindri.dev/v4/bom.json
# @schema {{ "id": "https://schemas.sindri.dev/v4/bom.json" }}
name: {name}

registries:
  - name: core
    url: registry:local:./registry-core  # Replace with OCI registry URL

components:
{components}

preferences:
  backend_order: {{}}
"#,
        name = name,
        components = components,
    );

    match fs::write(manifest_path, &manifest_content) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Failed to write sindri.yaml: {}", e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    }

    // Write .gitignore entry for .sindri/
    append_gitignore();

    // Write sindri.policy.yaml if non-default
    if policy_preset != "default"
        && sindri_policy::write_global_preset(&parse_preset(policy_preset)).is_ok() {
            println!("Policy set to '{}'", policy_preset);
        }

    println!("Created sindri.yaml for project '{}'", name);
    println!("Next steps:");
    println!("  sindri registry refresh core <registry-url>");
    println!("  sindri resolve");
    println!("  sindri apply");

    EXIT_SUCCESS
}

fn template_components(template: Option<&str>) -> String {
    match template {
        Some("anthropic-dev") => {
            "  - address: \"mise:nodejs\"\n  - address: \"mise:python\"\n  - address: \"binary:gh\"\n  - address: \"npm:claude-code\""
                .to_string()
        }
        Some("minimal") | None => {
            "  - address: \"mise:nodejs\"".to_string()
        }
        Some(t) => {
            format!("  # template '{}' — add components here", t)
        }
    }
}

fn append_gitignore() {
    let gitignore = Path::new(".gitignore");
    let entry = "\n# Sindri state\n.sindri/\nsindri.*.lock\n";
    if gitignore.exists() {
        if let Ok(content) = fs::read_to_string(gitignore) {
            if content.contains(".sindri/") {
                return;
            }
        }
        let _ = fs::OpenOptions::new()
            .append(true)
            .open(gitignore)
            .and_then(|mut f| {
                use std::io::Write;
                f.write_all(entry.as_bytes())
            });
    } else {
        let _ = fs::write(gitignore, format!("{}\n", entry.trim()));
    }
}

fn parse_preset(s: &str) -> sindri_core::policy::PolicyPreset {
    match s {
        "strict" => sindri_core::policy::PolicyPreset::Strict,
        "offline" => sindri_core::policy::PolicyPreset::Offline,
        _ => sindri_core::policy::PolicyPreset::Default,
    }
}
