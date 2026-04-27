use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};
use sindri_core::policy::PolicyPreset;

pub enum PolicyCmd {
    Use {
        preset: String,
    },
    Show,
    AllowLicense {
        spdx: String,
        reason: Option<String>,
    },
}

pub fn run(cmd: PolicyCmd) -> i32 {
    match cmd {
        PolicyCmd::Use { preset } => use_preset(&preset),
        PolicyCmd::Show => show_policy(),
        PolicyCmd::AllowLicense { spdx, reason } => allow_license(&spdx, reason.as_deref()),
    }
}

fn use_preset(preset: &str) -> i32 {
    let p = match preset {
        "default" => PolicyPreset::Default,
        "strict" => PolicyPreset::Strict,
        "offline" => PolicyPreset::Offline,
        other => {
            eprintln!(
                "Unknown preset '{}'. Valid presets: default, strict, offline",
                other
            );
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    match sindri_policy::write_global_preset(&p) {
        Ok(()) => {
            println!("Policy set to '{}' in ~/.sindri/policy.yaml", preset);
            EXIT_SUCCESS
        }
        Err(e) => {
            eprintln!("Failed to write policy: {}", e);
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        }
    }
}

fn show_policy() -> i32 {
    let effective = sindri_policy::load_effective_policy();
    let p = &effective.policy;

    println!("Effective policy:");
    println!("  preset:                  {}", preset_name(&p.preset));
    println!(
        "  allowed_licenses:        {}",
        if p.allowed_licenses.is_empty() {
            "(any)".into()
        } else {
            p.allowed_licenses.join(", ")
        }
    );
    println!(
        "  denied_licenses:         {}",
        if p.denied_licenses.is_empty() {
            "(none)".into()
        } else {
            p.denied_licenses.join(", ")
        }
    );
    println!("  on_unknown_license:      {:?}", p.on_unknown_license);
    println!(
        "  require_signed_registries: {}",
        p.require_signed_registries.unwrap_or(false)
    );
    println!(
        "  require_checksums:       {}",
        p.require_checksums.unwrap_or(false)
    );
    println!("  offline:                 {}", p.offline.unwrap_or(false));
    println!(
        "  audit.require_justification: {}",
        p.audit
            .as_ref()
            .map(|a| a.require_justification)
            .unwrap_or(false)
    );

    if !effective.sources.is_empty() {
        println!("\nSources:");
        for (key, src) in &effective.sources {
            println!("  {} ← {}", key, src.file);
        }
    }

    EXIT_SUCCESS
}

fn allow_license(spdx: &str, reason: Option<&str>) -> i32 {
    let effective = sindri_policy::load_effective_policy();
    let mut policy = effective.policy;

    if !policy.allowed_licenses.contains(&spdx.to_string()) {
        policy.allowed_licenses.push(spdx.to_string());
    }

    let path = sindri_policy::loader::global_policy_path();
    let yaml = match serde_yaml::to_string(&policy) {
        Ok(y) => y,
        Err(e) => {
            eprintln!("Failed to serialize policy: {}", e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::write(&path, yaml) {
        Ok(_) => {
            if let Some(r) = reason {
                println!("Allowed license '{}' (reason: {})", spdx, r);
            } else {
                println!("Allowed license '{}'", spdx);
            }
            EXIT_SUCCESS
        }
        Err(e) => {
            eprintln!("Failed to update policy: {}", e);
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        }
    }
}

fn preset_name(p: &sindri_core::policy::PolicyPreset) -> &'static str {
    match p {
        sindri_core::policy::PolicyPreset::Default => "default",
        sindri_core::policy::PolicyPreset::Strict => "strict",
        sindri_core::policy::PolicyPreset::Offline => "offline",
    }
}
