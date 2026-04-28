/// Full doctor implementation (Sprint 12)
use sindri_core::auth::AuthBindingStatus;
use sindri_core::exit_codes::{EXIT_POLICY_DENIED, EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};
use sindri_targets::traits::Target;
use sindri_targets::LocalTarget;

pub struct DoctorArgs {
    pub target: Option<String>,
    pub fix: bool,
    pub components: bool,
    /// Phase 5 (ADR-027 §Phase 5): focused doctor view that runs Gate 5
    /// against the current manifest+target set without any apply
    /// side-effects, and prints remediation hints inline.
    pub auth: bool,
    /// Emit machine-readable JSON instead of a human report (auth view).
    pub json: bool,
    /// Manifest path for `--auth`. Defaults to `sindri.yaml`.
    pub manifest: String,
}

pub fn run(args: DoctorArgs) -> i32 {
    if args.auth {
        return run_auth_doctor(&args);
    }
    let target_name = args.target.as_deref().unwrap_or("local");
    println!("sindri doctor — target: {}", target_name);
    println!();

    let mut any_failed = false;

    // 1. Target prerequisites
    let checks = LocalTarget::new().check_prerequisites();
    println!("Target prerequisites:");
    for check in &checks {
        if check.passed {
            println!("  [OK]   {}", check.name);
        } else {
            println!("  [FAIL] {}", check.name);
            if let Some(fix) = &check.fix {
                println!("         Fix: {}", fix);
            }
            any_failed = true;
        }
    }

    // 2. Shell configuration
    println!("\nShell configuration:");
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "(not set)".to_string());
    println!("  [OK]   SHELL = {}", shell);

    // 3. Registry access
    println!("\nRegistry cache:");
    let cache_dir = dirs_next::home_dir()
        .unwrap_or_default()
        .join(".sindri")
        .join("cache")
        .join("registries");
    if cache_dir.exists() {
        let count = std::fs::read_dir(&cache_dir)
            .map(|d| d.count())
            .unwrap_or(0);
        if count > 0 {
            println!("  [OK]   {} registry/registries cached", count);
        } else {
            println!("  [WARN] No registry cache — run `sindri registry refresh`");
        }
    } else {
        println!("  [WARN] Registry cache directory not found");
        println!("         Fix: run `sindri registry refresh <name> <url>`");
    }

    // 4. Policy validity
    println!("\nPolicy:");
    let effective = sindri_policy::load_effective_policy();
    let preset = match &effective.policy.preset {
        sindri_core::policy::PolicyPreset::Default => "default",
        sindri_core::policy::PolicyPreset::Strict => "strict",
        sindri_core::policy::PolicyPreset::Offline => "offline",
    };
    println!("  [OK]   Active preset: {}", preset);

    // 5. Mise availability
    println!("\nBackend availability:");
    for (backend, binary) in &[
        ("mise", "mise"),
        ("npm", "npm"),
        ("brew", "brew"),
        ("docker", "docker"),
        ("git", "git"),
    ] {
        if which(binary).is_some() {
            println!("  [OK]   {} ({})", backend, binary);
        } else {
            println!("  [SKIP] {} ({}) — not installed", backend, binary);
        }
    }

    if any_failed {
        println!("\nDoctor found issues. Apply the fixes above and re-run.");
        EXIT_SCHEMA_OR_RESOLVE_ERROR
    } else {
        println!("\nAll checks passed.");
        EXIT_SUCCESS
    }
}

// =============================================================================
// `doctor --auth` — Phase 5, ADR-027 §Phase 5
// =============================================================================

/// Focused doctor view: runs Gate 5 against the current manifest +
/// target set without apply side effects, prints remediation hints
/// inline. Reuses `sindri_policy::check_gate5` from PR #251.
fn run_auth_doctor(args: &DoctorArgs) -> i32 {
    let target_name = args.target.as_deref().unwrap_or("local");
    let lockfile_path = if target_name == "local" {
        std::path::PathBuf::from("sindri.lock")
    } else {
        std::path::PathBuf::from(format!("sindri.{}.lock", target_name))
    };

    if !lockfile_path.exists() {
        if args.json {
            println!(
                r#"{{"ok":false,"error":"LOCKFILE_NOT_FOUND","path":"{}"}}"#,
                lockfile_path.display()
            );
        } else {
            eprintln!(
                "doctor --auth: no lockfile at '{}'. Run `sindri resolve` first.",
                lockfile_path.display()
            );
        }
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }

    let content = match std::fs::read_to_string(&lockfile_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Cannot read lockfile: {}", e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };
    let lockfile: sindri_core::lockfile::Lockfile = match serde_json::from_str(&content) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Malformed lockfile: {}", e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    let effective = sindri_policy::load_effective_policy().policy;
    let gate5 = sindri_policy::check_gate5(&lockfile.auth_bindings, &effective.auth);

    let resolved = lockfile
        .auth_bindings
        .iter()
        .filter(|b| b.status == AuthBindingStatus::Bound)
        .count();
    let deferred = lockfile
        .auth_bindings
        .iter()
        .filter(|b| b.status == AuthBindingStatus::Deferred)
        .count();
    let failed = lockfile
        .auth_bindings
        .iter()
        .filter(|b| b.status == AuthBindingStatus::Failed)
        .count();

    if args.json {
        let payload = serde_json::json!({
            "ok": gate5.allowed,
            "target": target_name,
            "lockfile": lockfile_path.display().to_string(),
            "auth_bindings": {
                "resolved": resolved,
                "deferred": deferred,
                "failed": failed,
                "total": lockfile.auth_bindings.len(),
            },
            "gate5": {
                "allowed": gate5.allowed,
                "code": gate5.code,
                "message": gate5.message,
                "fix": gate5.fix,
            },
        });
        match serde_json::to_string_pretty(&payload) {
            Ok(s) => println!("{}", s),
            Err(_) => println!("{{\"ok\":{}}}", gate5.allowed),
        }
    } else {
        println!("sindri doctor --auth — target: {}", target_name);
        println!();
        println!(
            "auth bindings: {} resolved, {} deferred, {} failed",
            resolved, deferred, failed
        );
        if gate5.allowed {
            println!("[OK]   Gate 5 (auth-resolvable) — all bindings admissible.");
        } else {
            println!("[FAIL] Gate 5 (auth-resolvable) — {}", gate5.code);
            println!("       {}", gate5.message);
            if let Some(fix) = &gate5.fix {
                println!("       fix: {}", fix);
            }
            println!();
            println!("Remediation:");
            println!(
                "  1. `sindri auth show --target {}` to see why bindings failed.",
                target_name
            );
            println!(
                "  2. `sindri target auth {} --bind <req-id>` to bind a rejected candidate.",
                target_name
            );
            println!("  3. Adjust `policy.auth.*` if the violation is intentional (see v4/docs/policy.md).");
        }
    }

    let _ = args.manifest.is_empty(); // keep field used
    if gate5.allowed {
        EXIT_SUCCESS
    } else {
        EXIT_POLICY_DENIED
    }
}

fn which(name: &str) -> Option<std::path::PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).find_map(|d| {
            let c = d.join(name);
            if c.is_file() {
                Some(c)
            } else {
                None
            }
        })
    })
}
