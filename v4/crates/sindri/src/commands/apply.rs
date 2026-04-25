use std::path::{Path, PathBuf};
use sindri_core::exit_codes::{EXIT_RESOLUTION_CONFLICT, EXIT_STALE_LOCKFILE, EXIT_SUCCESS};
use sindri_core::platform::Platform;
use sindri_backends::install_component;

pub struct ApplyArgs {
    pub yes: bool,
    pub dry_run: bool,
    pub target: String,
}

pub fn run(args: ApplyArgs) -> i32 {
    // Determine lockfile path (ADR-018)
    let lock_name = if args.target == "local" {
        "sindri.lock".to_string()
    } else {
        format!("sindri.{}.lock", args.target)
    };
    let lockfile_path = PathBuf::from(&lock_name);

    if !lockfile_path.exists() {
        eprintln!(
            "Lockfile '{}' not found — run `sindri resolve` first",
            lockfile_path.display()
        );
        return EXIT_STALE_LOCKFILE;
    }

    // Load lockfile
    let content = match std::fs::read_to_string(&lockfile_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Cannot read lockfile: {}", e);
            return EXIT_STALE_LOCKFILE;
        }
    };

    let lockfile: sindri_core::lockfile::Lockfile = match serde_json::from_str(&content) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Malformed lockfile: {}", e);
            return EXIT_STALE_LOCKFILE;
        }
    };

    // Check staleness against sindri.yaml
    if Path::new("sindri.yaml").exists() {
        let bom_content = std::fs::read_to_string("sindri.yaml").unwrap_or_default();
        let current_hash = compute_hash(&bom_content);
        if lockfile.is_stale(&current_hash) {
            eprintln!(
                "Lockfile is stale — `sindri.yaml` has changed. Run `sindri resolve` first."
            );
            return EXIT_STALE_LOCKFILE;
        }
    }

    let platform = Platform::current();
    let total = lockfile.components.len();

    if total == 0 {
        println!("Nothing to apply — lockfile is empty.");
        return EXIT_SUCCESS;
    }

    // Show plan
    println!("Plan: {} component(s) to apply on {}:", total, lockfile.target);
    for comp in &lockfile.components {
        println!("  + {} {} ({})", comp.id.to_address(), comp.version, comp.backend.as_str());
    }

    if args.dry_run {
        println!("\nDry run — no changes made.");
        return EXIT_SUCCESS;
    }

    // Prompt unless --yes
    if !args.yes {
        eprint!("\nProceed? [y/N] ");
        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() {
            return EXIT_RESOLUTION_CONFLICT;
        }
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return EXIT_SUCCESS;
        }
    }

    // Install in topological order (already sorted by resolver)
    let mut failed = 0usize;
    for comp in &lockfile.components {
        print!("  Installing {} {}...", comp.id.to_address(), comp.version);
        match install_component(comp, &platform) {
            Ok(()) => println!(" done"),
            Err(e) => {
                println!(" FAILED: {}", e);
                failed += 1;
            }
        }
    }

    if failed > 0 {
        eprintln!("\n{}/{} component(s) failed", failed, total);
        EXIT_RESOLUTION_CONFLICT
    } else {
        println!("\nApplied {} component(s) successfully.", total);
        EXIT_SUCCESS
    }
}

fn compute_hash(content: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut h = Sha256::new();
    h.update(content.as_bytes());
    hex::encode(h.finalize())
}
