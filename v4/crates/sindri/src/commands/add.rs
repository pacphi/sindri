use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};
use sindri_core::component::BomEntry;
use crate::commands::manifest::{load_manifest, save_manifest, find_entry_index};

pub struct AddArgs {
    pub address: String,
    pub dry_run: bool,
    pub apply: bool,
    pub manifest: String,
}

pub fn run(args: AddArgs) -> i32 {
    // Parse address — must be `backend:name[@version]`
    if sindri_core::component::ComponentId::parse(&args.address).is_none() {
        eprintln!(
            "Invalid component address '{}'. Expected format: backend:name[@version]",
            args.address
        );
        eprintln!("Example: mise:nodejs@22.0.0  or  npm:claude-code");
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }

    let (mut manifest, _content) = match load_manifest(&args.manifest) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{}", e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    // Check for duplicate
    let clean_addr = crate::commands::manifest::address_without_version(&args.address);
    if let Some(_) = find_entry_index(&manifest, &clean_addr) {
        eprintln!(
            "Component '{}' is already in sindri.yaml",
            clean_addr
        );
        eprintln!("Use `sindri upgrade {}` to change its version.", clean_addr);
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }

    let entry = BomEntry {
        address: args.address.clone(),
        version: None,
        options: Default::default(),
    };

    if args.dry_run {
        println!("Would add: {}", args.address);
        return EXIT_SUCCESS;
    }

    manifest.components.push(entry);

    match save_manifest(&args.manifest, &manifest) {
        Ok(_) => {
            println!("Added {} to sindri.yaml", args.address);
            if args.apply {
                println!("Hint: run `sindri resolve && sindri apply` to install");
            }
            EXIT_SUCCESS
        }
        Err(e) => {
            eprintln!("Failed to save manifest: {}", e);
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        }
    }
}
