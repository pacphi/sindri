use std::collections::HashMap;
use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};
use sindri_core::registry::ComponentEntry;
use crate::commands::manifest::{load_manifest, save_manifest, find_entry_index, address_without_version};

pub struct UpgradeArgs {
    pub component: Option<String>,
    pub all: bool,
    pub check: bool,
    pub manifest: String,
}

pub fn run(args: UpgradeArgs) -> i32 {
    let (mut manifest, _) = match load_manifest(&args.manifest) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{}", e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    let registry = load_registry_from_cache();

    if args.check {
        return check_upgrades(&manifest, &registry);
    }

    if args.all {
        return upgrade_all(&mut manifest, &registry, &args.manifest);
    }

    if let Some(addr) = &args.component {
        return upgrade_one(&mut manifest, addr, &registry, &args.manifest);
    }

    eprintln!("Specify a component or use --all");
    EXIT_SCHEMA_OR_RESOLVE_ERROR
}

fn check_upgrades(manifest: &sindri_core::manifest::BomManifest, registry: &HashMap<String, ComponentEntry>) -> i32 {
    let mut upgradeable = 0;
    for comp in &manifest.components {
        let clean = address_without_version(&comp.address);
        if let Some(entry) = registry.get(&clean) {
            let current_ver = comp.address.split('@').nth(1).unwrap_or("latest");
            if entry.latest != current_ver && current_ver != "latest" {
                println!("  {} {} → {} available", clean, current_ver, entry.latest);
                upgradeable += 1;
            }
        }
    }
    if upgradeable == 0 {
        println!("All components are at the latest available version.");
    }
    EXIT_SUCCESS
}

fn upgrade_one(
    manifest: &mut sindri_core::manifest::BomManifest,
    address: &str,
    registry: &HashMap<String, ComponentEntry>,
    manifest_path: &str,
) -> i32 {
    let clean = address_without_version(address);
    let idx = match find_entry_index(manifest, &clean) {
        Some(i) => i,
        None => {
            eprintln!("Component '{}' not in sindri.yaml", clean);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    let entry = match registry.get(&clean) {
        Some(e) => e,
        None => {
            eprintln!("Component '{}' not found in registry cache", clean);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    let old = manifest.components[idx].address.clone();
    manifest.components[idx].address = format!("{}@{}", clean, entry.latest);

    match save_manifest(manifest_path, manifest) {
        Ok(_) => {
            println!("Upgraded {} → {}", old, entry.latest);
            EXIT_SUCCESS
        }
        Err(e) => {
            eprintln!("Failed: {}", e);
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        }
    }
}

fn upgrade_all(
    manifest: &mut sindri_core::manifest::BomManifest,
    registry: &HashMap<String, ComponentEntry>,
    manifest_path: &str,
) -> i32 {
    let mut upgraded = 0;
    for comp in manifest.components.iter_mut() {
        let clean = address_without_version(&comp.address);
        if let Some(entry) = registry.get(&clean) {
            let old = comp.address.clone();
            comp.address = format!("{}@{}", clean, entry.latest);
            if old != comp.address {
                println!("  {} → {}", old, entry.latest);
                upgraded += 1;
            }
        }
    }

    if upgraded == 0 {
        println!("Nothing to upgrade.");
        return EXIT_SUCCESS;
    }

    match save_manifest(manifest_path, manifest) {
        Ok(_) => {
            println!("Upgraded {} component(s). Run `sindri resolve` to update the lockfile.", upgraded);
            EXIT_SUCCESS
        }
        Err(e) => {
            eprintln!("Failed: {}", e);
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        }
    }
}

fn load_registry_from_cache() -> HashMap<String, ComponentEntry> {
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
        let index_path = entry.path().join("index.yaml");
        if !index_path.exists() { continue; }
        let content = match std::fs::read_to_string(&index_path) { Ok(c) => c, Err(_) => continue };
        let index: sindri_core::registry::RegistryIndex = match serde_yaml::from_str(&content) { Ok(i) => i, Err(_) => continue };
        for comp in index.components {
            let addr = format!("{}:{}", comp.backend, comp.name);
            map.insert(addr, comp);
        }
    }
    map
}
