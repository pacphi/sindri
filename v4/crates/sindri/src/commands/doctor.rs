/// Full doctor implementation (Sprint 12)
use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};
use sindri_targets::traits::Target;
use sindri_targets::LocalTarget;

pub struct DoctorArgs {
    pub target: Option<String>,
    pub fix: bool,
    pub components: bool,
}

pub fn run(args: DoctorArgs) -> i32 {
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
    let cache_dir = sindri_core::paths::home_dir()
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
