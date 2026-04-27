use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};
use sindri_core::platform::Platform;
use sindri_core::policy::InstallPolicy;
use sindri_core::registry::ComponentEntry;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct ResolveArgs {
    pub manifest: String,
    pub offline: bool,
    pub refresh: bool,
    pub strict: bool,
    pub explain: Option<String>,
    pub target: String,
    pub json: bool,
}

pub fn run(args: ResolveArgs) -> i32 {
    let manifest_path = PathBuf::from(&args.manifest);
    if !manifest_path.exists() {
        if args.json {
            eprintln!(
                r#"{{"error":"FILE_NOT_FOUND","path":"{}","fix":"Create sindri.yaml or run sindri init"}}"#,
                args.manifest
            );
        } else {
            eprintln!("Manifest not found: {}", args.manifest);
            eprintln!("Hint: run `sindri init` to create a sindri.yaml");
        }
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }

    // Determine lockfile path — per-target (ADR-018)
    let lock_name = if args.target == "local" {
        "sindri.lock".to_string()
    } else {
        format!("sindri.{}.lock", args.target)
    };
    let lockfile_path = manifest_path
        .parent()
        .unwrap_or(Path::new("."))
        .join(&lock_name);

    // Load registry from cache
    let registry = load_registry_from_cache();
    if registry.is_empty() && !args.offline && !args.json {
        eprintln!("Warning: no registry index found. Run `sindri registry refresh` first.");
        eprintln!("Proceeding with empty registry (no components will resolve).");
    }

    // Load policy (defaults for now; Sprint 6 adds full policy loading)
    let mut policy = InstallPolicy {
        preset: sindri_core::policy::PolicyPreset::Default,
        allowed_licenses: Vec::new(),
        denied_licenses: Vec::new(),
        on_unknown_license: None,
        require_signed_registries: None,
        require_checksums: None,
        offline: Some(args.offline),
        audit: None,
    };
    if args.strict {
        policy.preset = sindri_core::policy::PolicyPreset::Strict;
    }

    let platform = Platform::current();
    let opts = sindri_resolver::ResolveOptions {
        manifest_path: manifest_path.clone(),
        lockfile_path: lockfile_path.clone(),
        target_name: args.target.clone(),
        offline: args.offline,
        strict: args.strict,
        explain: args.explain.clone(),
    };

    match sindri_resolver::resolve(&opts, &registry, &policy, &platform) {
        Ok(lockfile) => {
            if args.json {
                println!(
                    r#"{{"resolved":true,"lockfile":"{}","components":{}}}"#,
                    lockfile_path.display(),
                    lockfile.components.len()
                );
            } else {
                println!(
                    "Resolved {} component(s) → {}",
                    lockfile.components.len(),
                    lockfile_path.display()
                );
                for c in &lockfile.components {
                    println!(
                        "  {} {} ({})",
                        c.id.to_address(),
                        c.version,
                        c.backend.as_str()
                    );
                }
            }
            EXIT_SUCCESS
        }
        Err(e) => {
            let code = e.exit_code();
            if args.json {
                eprintln!(r#"{{"error":"{}","detail":"{}"}}"#, code, e);
            } else {
                eprintln!("resolve failed: {}", e);
            }
            code
        }
    }
}

fn load_registry_from_cache() -> HashMap<String, ComponentEntry> {
    let cache_root = dirs_next::home_dir()
        .unwrap_or_default()
        .join(".sindri")
        .join("cache")
        .join("registries");

    let mut map: HashMap<String, ComponentEntry> = HashMap::new();

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
            let addr = format!("{}:{}", comp.backend, comp.name);
            map.insert(addr, comp);
        }
    }

    map
}
