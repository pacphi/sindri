use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};
use sindri_core::registry::ComponentEntry;
use std::collections::HashMap;

pub struct ShowArgs {
    pub address: String,
    pub versions: bool,
    pub json: bool,
}

pub fn run(args: ShowArgs) -> i32 {
    let (backend, name) = match args.address.split_once(':') {
        Some((b, n)) => (b.to_string(), n.to_string()),
        None => {
            eprintln!("Invalid address '{}' — expected backend:name", args.address);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    let registry = load_registry();
    let key = format!("{}:{}", backend, name);
    let entry = match registry.get(&key) {
        Some(e) => e,
        None => {
            eprintln!("Component '{}' not found in registry cache", key);
            eprintln!("Hint: run `sindri registry refresh` then retry");
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "name": entry.name,
                "backend": entry.backend,
                "latest": entry.latest,
                "versions": entry.versions,
                "description": entry.description,
                "license": entry.license,
                "depends_on": entry.depends_on,
                "kind": format!("{:?}", entry.kind),
                "oci_ref": entry.oci_ref,
            }))
            .unwrap_or_default()
        );
        return EXIT_SUCCESS;
    }

    println!("{}:{}", entry.backend, entry.name);
    println!("  Description: {}", entry.description);
    println!(
        "  License:     {}",
        if entry.license.is_empty() {
            "(unspecified)"
        } else {
            &entry.license
        }
    );
    println!("  Latest:      {}", entry.latest);
    if args.versions {
        println!("  Versions:    {}", entry.versions.join(", "));
    }
    if !entry.depends_on.is_empty() {
        println!("  Depends on:  {}", entry.depends_on.join(", "));
    }
    println!("  OCI ref:     {}", entry.oci_ref);

    EXIT_SUCCESS
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
