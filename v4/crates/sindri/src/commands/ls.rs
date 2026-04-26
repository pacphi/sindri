use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};

pub struct LsArgs {
    pub registry: Option<String>,
    pub backend: Option<String>,
    pub installed: bool,
    pub outdated: bool,
    pub json: bool,
    pub refresh: bool,
}

pub fn run(args: LsArgs) -> i32 {
    let cache_dir = dirs_next::home_dir()
        .map(|h| h.join(".sindri").join("cache").join("registries"))
        .unwrap_or_else(|| std::path::PathBuf::from(".sindri/cache/registries"));

    if !cache_dir.exists() {
        if args.json {
            println!(r#"{{"components":[],"hint":"Run sindri registry refresh <name> <url> first"}}"#);
        } else {
            eprintln!("No registry cache found. Run `sindri registry refresh <name> <url>` first.");
        }
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }

    let mut found = false;
    let entries = match std::fs::read_dir(&cache_dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Cannot read cache: {}", e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    for entry in entries.flatten() {
        let index_path = entry.path().join("index.yaml");
        if !index_path.exists() {
            continue;
        }
        let content = match std::fs::read_to_string(&index_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let registry_name = entry.file_name().to_string_lossy().to_string();
        // Filter by --registry flag
        if let Some(ref r) = args.registry {
            if &registry_name != r {
                continue;
            }
        }

        found = true;

        // Parse index as generic YAML value to avoid coupling here
        let index: serde_json::Value = match serde_yaml::from_str(&content) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Malformed index for {}: {}", registry_name, e);
                continue;
            }
        };

        let components = match index.get("components").and_then(|c| c.as_array()) {
            Some(c) => c,
            None => continue,
        };

        if args.json {
            println!(
                "{}",
                serde_json::to_string(&serde_json::json!({
                    "registry": registry_name,
                    "components": components,
                }))
                .unwrap_or_default()
            );
        } else {
            println!("\nRegistry: {}", registry_name);
            println!("{:<30} {:<12} {:<12} KIND", "COMPONENT", "BACKEND", "LATEST");
            println!("{}", "-".repeat(70));
            for comp in components {
                let name = comp.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                let backend = comp.get("backend").and_then(|v| v.as_str()).unwrap_or("?");
                let latest = comp.get("latest").and_then(|v| v.as_str()).unwrap_or("?");
                let kind = comp.get("kind").and_then(|v| v.as_str()).unwrap_or("component");

                if args.backend.as_deref().map(|b| b == backend).unwrap_or(true) {
                    println!("{:<30} {:<12} {:<12} {}", name, backend, latest, kind);
                }
            }
        }
    }

    if !found {
        if !args.json {
            eprintln!("No registry index found. Run `sindri registry refresh <name> <url>` first.");
        }
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }

    EXIT_SUCCESS
}
