use sindri_backends::install_component;
use sindri_core::exit_codes::{EXIT_RESOLUTION_CONFLICT, EXIT_STALE_LOCKFILE, EXIT_SUCCESS};
use sindri_targets::LocalTarget;
use std::path::{Path, PathBuf};

pub struct ApplyArgs {
    pub yes: bool,
    pub dry_run: bool,
    pub target: String,
}

/// Synchronous entry point preserved for the CLI dispatch. Internally we
/// spin up a current-thread tokio runtime to drive the now-async backend
/// trait (Wave 2A, ADR-017). Top-level `main` stays sync to avoid touching
/// every other command site.
pub fn run(args: ApplyArgs) -> i32 {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("Failed to start tokio runtime: {}", e);
            return EXIT_RESOLUTION_CONFLICT;
        }
    };
    runtime.block_on(run_async(args))
}

async fn run_async(args: ApplyArgs) -> i32 {
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
            eprintln!("Lockfile is stale — `sindri.yaml` has changed. Run `sindri resolve` first.");
            return EXIT_STALE_LOCKFILE;
        }
    }

    // reason: only `local` is wired through to a real Target in Wave 2A;
    // remote target plugins (SSH/Docker/cloud) land with Wave 3 (ADR-019).
    if args.target != "local" {
        eprintln!(
            "Target '{}' is not yet wired up — only `local` is supported in Wave 2A. \
             Remote target plugins land with Wave 3 (ADR-019).",
            args.target
        );
        return EXIT_RESOLUTION_CONFLICT;
    }
    let target = LocalTarget::new();
    let total = lockfile.components.len();

    if total == 0 {
        println!("Nothing to apply — lockfile is empty.");
        return EXIT_SUCCESS;
    }

    // Show plan
    println!(
        "Plan: {} component(s) to apply on {}:",
        total, lockfile.target
    );
    for comp in &lockfile.components {
        println!(
            "  + {} {} ({})",
            comp.id.to_address(),
            comp.version,
            comp.backend.as_str()
        );
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
        // manifest = None: OCI ComponentManifest fetch is wired in Wave 3.
        // Until then backends fall back to minimal name@version invocations
        // and emit `tracing::debug!` so the gap is observable.
        match install_component(comp, None, &target).await {
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
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(content.as_bytes());
    hex::encode(h.finalize())
}
