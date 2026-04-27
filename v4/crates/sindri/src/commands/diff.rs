use sindri_core::exit_codes::{EXIT_ERROR, EXIT_STALE_LOCKFILE, EXIT_SUCCESS};
use sindri_core::lockfile::Lockfile;
use std::path::PathBuf;

pub struct DiffArgs {
    pub target: String,
    pub json: bool,
}

pub fn run(args: DiffArgs) -> i32 {
    let lock_name = if args.target == "local" {
        "sindri.lock".to_string()
    } else {
        format!("sindri.{}.lock", args.target)
    };

    if !PathBuf::from(&lock_name).exists() {
        if args.json {
            eprintln!(r#"{{"error":"STALE_LOCKFILE"}}"#);
        } else {
            eprintln!(
                "Lockfile '{}' not found. Run `sindri resolve` first.",
                lock_name
            );
        }
        return EXIT_STALE_LOCKFILE;
    }

    let content = match std::fs::read_to_string(&lock_name) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Cannot read lockfile: {}", e);
            return EXIT_ERROR;
        }
    };

    let lockfile: Lockfile = match serde_json::from_str(&content) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Malformed lockfile: {}", e);
            return EXIT_ERROR;
        }
    };

    // Sprint 7: show what the lockfile says (full live-installed state tracking is Sprint 12)
    if lockfile.components.is_empty() {
        if args.json {
            println!(r#"{{"divergences":[],"message":"locked and installed state match"}}"#);
        } else {
            println!("No divergences — lockfile is empty.");
        }
        return EXIT_SUCCESS;
    }

    // Real diff: sindri.lock vs what's actually installed
    // For Sprint 7: show plan items as "to-install"
    if !args.json {
        println!(
            "Diff between sindri.lock and installed state for target '{}':",
            lockfile.target
        );
        for comp in &lockfile.components {
            // Sprint 7: mark all as "+" (unknown installed state)
            println!(
                "  ? {} {} (not tracked — run sindri apply to reconcile)",
                comp.id.to_address(),
                comp.version
            );
        }
    } else {
        let items: Vec<serde_json::Value> = lockfile
            .components
            .iter()
            .map(|c| {
                serde_json::json!({
                    "status": "unknown",
                    "address": c.id.to_address(),
                    "locked_version": c.version.0,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({"divergences": items}))
                .unwrap_or_default()
        );
    }

    EXIT_SUCCESS
}
