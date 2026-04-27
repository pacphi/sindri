use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};
use sindri_core::registry::ComponentEntry;
use sindri_discovery::{search, SearchFilters};
use std::collections::HashMap;

pub struct SearchArgs {
    pub query: String,
    pub registry: Option<String>,
    pub backend: Option<String>,
    pub json: bool,
}

pub fn run(args: SearchArgs) -> i32 {
    let registry = load_registry(args.registry.as_deref());
    if registry.is_empty() {
        eprintln!("No registry cache found. Run `sindri registry refresh <name> <url>` first.");
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }

    let entries: Vec<ComponentEntry> = registry.into_values().collect();
    let filters = SearchFilters {
        backend: args.backend,
        category: None,
    };

    let results = search(&args.query, &entries, &filters);

    if results.is_empty() {
        if args.json {
            println!(r#"{{"results":[],"query":"{}"}}"#, args.query);
        } else {
            println!("No components found matching '{}'", args.query);
        }
        return EXIT_SUCCESS;
    }

    if args.json {
        let items: Vec<serde_json::Value> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "name": r.entry.name,
                    "backend": r.entry.backend,
                    "latest": r.entry.latest,
                    "description": r.entry.description,
                    "score": r.score,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({"results": items}))
                .unwrap_or_default()
        );
    } else {
        println!(
            "{:<30} {:<12} {:<12} DESCRIPTION",
            "COMPONENT", "BACKEND", "LATEST"
        );
        println!("{}", "-".repeat(80));
        for r in &results {
            println!(
                "{:<30} {:<12} {:<12} {}",
                r.entry.name,
                r.entry.backend,
                r.entry.latest,
                r.entry.description.chars().take(35).collect::<String>()
            );
        }
    }

    EXIT_SUCCESS
}

fn load_registry(registry_filter: Option<&str>) -> HashMap<String, ComponentEntry> {
    let cache_root = dirs_next::home_dir()
        .unwrap_or_default()
        .join(".sindri")
        .join("cache")
        .join("registries");

    let mut map = HashMap::new();
    let entries = match std::fs::read_dir(&cache_root) {
        Ok(e) => e,
        Err(_) => return map,
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if registry_filter.map(|f| f != name).unwrap_or(false) {
            continue;
        }
        let index_path = entry.path().join("index.yaml");
        if !index_path.exists() {
            continue;
        }
        let content = match std::fs::read_to_string(&index_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let index: sindri_core::registry::RegistryIndex = match serde_yaml::from_str(&content) {
            Ok(i) => i,
            Err(_) => continue,
        };
        for comp in index.components {
            map.insert(format!("{}:{}", comp.backend, comp.name), comp);
        }
    }
    map
}
