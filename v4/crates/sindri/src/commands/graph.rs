use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};
use sindri_core::registry::ComponentEntry;
use sindri_discovery::{explain_path, render_explain, render_mermaid, render_tree};
use std::collections::HashMap;

pub struct GraphArgs {
    pub address: String,
    pub format: String, // "text" | "mermaid"
    pub reverse: bool,
}

pub struct ExplainArgs {
    pub component: String,
    pub in_collection: Option<String>,
}

pub fn run_graph(args: GraphArgs) -> i32 {
    let registry = load_registry();

    if !registry.contains_key(&args.address) {
        // Try component without version
        let clean = args.address.split('@').next().unwrap_or(&args.address);
        if !registry.contains_key(clean) {
            eprintln!("Component '{}' not found in registry cache", args.address);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    }

    match args.format.as_str() {
        "mermaid" => {
            let output = render_mermaid(&[args.address.as_str()], &registry);
            println!("{}", output);
        }
        _ => {
            let output = render_tree(&args.address, &registry, args.reverse);
            println!("{}", output);
        }
    }
    EXIT_SUCCESS
}

pub fn run_explain(args: ExplainArgs) -> i32 {
    let registry = load_registry();

    let root = args.in_collection.as_deref().unwrap_or(&args.component);

    match explain_path(&args.component, root, &registry) {
        Some(path) => {
            println!("{}", render_explain(&path));
            EXIT_SUCCESS
        }
        None => {
            eprintln!("No dependency path from '{}' to '{}'", root, args.component);
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        }
    }
}

fn load_registry() -> HashMap<String, ComponentEntry> {
    let cache_root = sindri_core::paths::home_dir()
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
