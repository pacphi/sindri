use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};
use sindri_targets::{LocalTarget, DockerTarget, SshTarget, Target};

pub enum TargetCmd {
    Add { name: String, kind: String, opts: Vec<(String, String)> },
    Ls,
    Status { name: String },
    Create { name: String },
    Destroy { name: String },
    Doctor { name: Option<String> },
    Shell { name: String },
}

pub fn run(cmd: TargetCmd) -> i32 {
    match cmd {
        TargetCmd::Add { name, kind, opts } => add_target(&name, &kind, &opts),
        TargetCmd::Ls => list_targets(),
        TargetCmd::Status { name } => status_target(&name),
        TargetCmd::Create { name } => create_target(&name),
        TargetCmd::Destroy { name } => destroy_target(&name),
        TargetCmd::Doctor { name } => doctor(&name),
        TargetCmd::Shell { name } => shell(&name),
    }
}

fn add_target(name: &str, kind: &str, _opts: &[(String, String)]) -> i32 {
    // Sprint 9: write to sindri.yaml targets: section
    // Full implementation requires manifest read/write
    println!("Target '{}' (kind: {}) added — update sindri.yaml targets: section manually for now", name, kind);
    EXIT_SUCCESS
}

fn list_targets() -> i32 {
    // Sprint 9: show local as the always-present default (ADR-023)
    println!("{:<20} {:<10} {}", "NAME", "KIND", "STATUS");
    println!("{}", "-".repeat(50));
    println!("{:<20} {:<10} {}", "local", "local", "ready");
    EXIT_SUCCESS
}

fn status_target(name: &str) -> i32 {
    if name == "local" {
        let t = LocalTarget::new();
        match t.profile() {
            Ok(p) => {
                println!("Target: local");
                println!("  Platform: {}", p.platform.triple());
                println!("  System PM: {}", p.capabilities.system_package_manager.as_deref().unwrap_or("none"));
                println!("  Docker: {}", p.capabilities.has_docker);
            }
            Err(e) => eprintln!("Error: {}", e),
        }
        return EXIT_SUCCESS;
    }
    eprintln!("Target '{}' not found", name);
    EXIT_SCHEMA_OR_RESOLVE_ERROR
}

fn create_target(name: &str) -> i32 {
    // Sprint 9: only docker targets are provisionable here
    let image = "ubuntu:24.04";
    let t = DockerTarget::new(name, image);
    match t.create() {
        Ok(_) => {
            println!("Created Docker target '{}' using image {}", name, image);
            EXIT_SUCCESS
        }
        Err(e) => {
            eprintln!("Failed to create target '{}': {}", name, e);
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        }
    }
}

fn destroy_target(name: &str) -> i32 {
    let image = "ubuntu:24.04";
    let t = DockerTarget::new(name, image);
    match t.destroy() {
        Ok(_) => {
            println!("Destroyed target '{}'", name);
            EXIT_SUCCESS
        }
        Err(e) => {
            eprintln!("Failed: {}", e);
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        }
    }
}

fn doctor(name: &Option<String>) -> i32 {
    let target_name = name.as_deref().unwrap_or("local");
    println!("Running doctor checks for target '{}'...", target_name);

    let checks = if target_name == "local" {
        LocalTarget::new().check_prerequisites()
    } else {
        DockerTarget::new(target_name, "").check_prerequisites()
    };

    let mut any_failed = false;
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

    if any_failed {
        EXIT_SCHEMA_OR_RESOLVE_ERROR
    } else {
        println!("All checks passed.");
        EXIT_SUCCESS
    }
}

fn shell(name: &str) -> i32 {
    if name == "local" {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        let status = std::process::Command::new(&shell)
            .status()
            .unwrap_or_else(|_| std::process::exit(1));
        if status.success() { EXIT_SUCCESS } else { EXIT_SCHEMA_OR_RESOLVE_ERROR }
    } else {
        eprintln!("Interactive shell for target '{}': sprint 10 (cloud targets)", name);
        EXIT_SCHEMA_OR_RESOLVE_ERROR
    }
}
