use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};
use crate::commands::manifest::{load_manifest, save_manifest, find_entry_index};

pub struct RemoveArgs {
    pub address: String,
    pub manifest: String,
}

pub fn run(args: RemoveArgs) -> i32 {
    let (mut manifest, _) = match load_manifest(&args.manifest) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{}", e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    let idx = match find_entry_index(&manifest, &args.address) {
        Some(i) => i,
        None => {
            eprintln!("Component '{}' not found in sindri.yaml", args.address);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    // Check if other components depend on this one
    let addr_clean = crate::commands::manifest::address_without_version(&args.address);
    let dependents: Vec<String> = manifest.components
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != idx)
        .filter(|(_, c)| {
            // Sprint 7: simple string scan — full graph check in Sprint 8
            c.address.contains(&addr_clean)
        })
        .map(|(_, c)| c.address.clone())
        .collect();

    if !dependents.is_empty() {
        eprintln!(
            "Warning: removing '{}' may break: {}",
            addr_clean,
            dependents.join(", ")
        );
    }

    manifest.components.remove(idx);

    match save_manifest(&args.manifest, &manifest) {
        Ok(_) => {
            println!("Removed {} from sindri.yaml", args.address);
            println!("Run `sindri resolve` to update the lockfile.");
            EXIT_SUCCESS
        }
        Err(e) => {
            eprintln!("Failed to save manifest: {}", e);
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        }
    }
}
