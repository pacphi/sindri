use sindri_core::exit_codes::{EXIT_STALE_LOCKFILE, EXIT_SUCCESS};
use sindri_core::lockfile::Lockfile;
use std::path::PathBuf;

pub struct PlanArgs {
    pub target: String,
    pub json: bool,
}

pub fn run(args: PlanArgs) -> i32 {
    let lock_name = if args.target == "local" {
        "sindri.lock".to_string()
    } else {
        format!("sindri.{}.lock", args.target)
    };
    let lockfile_path = PathBuf::from(&lock_name);

    if !lockfile_path.exists() {
        if args.json {
            eprintln!(r#"{{"error":"STALE_LOCKFILE","fix":"Run sindri resolve"}}"#);
        } else {
            eprintln!(
                "Lockfile '{}' not found. Run `sindri resolve` first.",
                lock_name
            );
        }
        return EXIT_STALE_LOCKFILE;
    }

    let content = match std::fs::read_to_string(&lockfile_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Cannot read lockfile: {}", e);
            return EXIT_STALE_LOCKFILE;
        }
    };

    let lockfile: Lockfile = match serde_json::from_str(&content) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Malformed lockfile: {}", e);
            return EXIT_STALE_LOCKFILE;
        }
    };

    // Compare lockfile against installed state
    // Sprint 7: diff is based on the lockfile only; full live-check in Sprint 12
    if args.json {
        let items: Vec<serde_json::Value> = lockfile
            .components
            .iter()
            .map(|c| {
                serde_json::json!({
                    "action": "install",
                    "address": c.id.to_address(),
                    "version": c.version.0,
                    "backend": c.backend.as_str(),
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "target": lockfile.target,
                "plan": items,
            }))
            .unwrap_or_default()
        );
    } else {
        println!(
            "Plan for target '{}' ({} component(s)):",
            lockfile.target,
            lockfile.components.len()
        );
        for comp in &lockfile.components {
            println!(
                "  + {} {} ({})",
                comp.id.to_address(),
                comp.version,
                comp.backend.as_str()
            );
        }
    }

    EXIT_SUCCESS
}
