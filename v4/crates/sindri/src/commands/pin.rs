use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};
use crate::commands::manifest::{load_manifest, save_manifest, find_entry_index};

pub struct PinArgs {
    pub address: String,
    pub version: String,
    pub manifest: String,
}

pub struct UnpinArgs {
    pub address: String,
    pub manifest: String,
}

pub fn run_pin(args: PinArgs) -> i32 {
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

    // Set exact version via @version suffix in address
    let clean = crate::commands::manifest::address_without_version(&manifest.components[idx].address);
    manifest.components[idx].address = format!("{}@{}", clean, args.version);

    match save_manifest(&args.manifest, &manifest) {
        Ok(_) => {
            println!("Pinned {} to {}", args.address, args.version);
            EXIT_SUCCESS
        }
        Err(e) => {
            eprintln!("Failed: {}", e);
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        }
    }
}

pub fn run_unpin(args: UnpinArgs) -> i32 {
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

    // Remove @version suffix
    let clean = crate::commands::manifest::address_without_version(&manifest.components[idx].address);
    manifest.components[idx].address = clean.clone();

    match save_manifest(&args.manifest, &manifest) {
        Ok(_) => {
            println!("Unpinned {} (now tracks latest)", clean);
            EXIT_SUCCESS
        }
        Err(e) => {
            eprintln!("Failed: {}", e);
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        }
    }
}
