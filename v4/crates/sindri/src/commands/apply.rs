//! `sindri apply` — execute the lockfile against a target (ADR-024, plan §4.4).
//!
//! Top-level orchestrator. Per ADR-024 the apply pipeline is:
//!
//! ```text
//!   0. CollisionResolver.validate_and_resolve(&closure)  // once, up-front
//!   for each component (in order):
//!       1. pre-install hook
//!       2. install backend
//!       3. configure   (manifest-only)
//!       4. validate    (manifest-only)
//!       5. post-install hook
//!   // After all installs:
//!   6. pre-project-init hooks   (per component)
//!   7. ProjectInitExecutor.run(steps_sorted_by_priority)
//!   8. post-project-init hooks  (per component)
//! ```
//!
//! Steps 1–5 are factored into [`super::apply_lifecycle::install_one`]; this
//! function is the thin shell that loads the lockfile, runs collision
//! validation, drives the loop, and runs the project-init pass.

use crate::commands::apply_lifecycle::{install_one, ApplyError, ApplyOptions};
use sindri_core::component::ComponentManifest;
use sindri_core::exit_codes::{EXIT_RESOLUTION_CONFLICT, EXIT_STALE_LOCKFILE, EXIT_SUCCESS};
use sindri_core::lockfile::ResolvedComponent;
use sindri_core::platform::Platform;
use sindri_extensions::{
    CollisionContext, CollisionResolver, ComponentRef, HookContext, HooksExecutor,
    ProjectInitContext, ProjectInitExecutor,
};
use sindri_targets::{LocalTarget, Target};
use std::path::{Path, PathBuf};

pub struct ApplyArgs {
    pub yes: bool,
    pub dry_run: bool,
    pub target: String,
    /// Skip SBOM auto-emit on success (ADR-007).
    pub no_bom: bool,
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

    let platform = Platform::current();

    // Step 0: CollisionResolver validates the entire closure once.
    //
    // Today the lockfile rarely carries manifests (resolver fetches OCI in
    // Wave 3A), so in the common case this is a no-op. When manifests ARE
    // present, the resolver enforces ADR-008 Gate 4 path-prefix rules and
    // returns a (possibly reordered) `ordered` plus a `skipped` list.
    let manifest_pairs: Vec<(ComponentManifest, &str)> = lockfile
        .components
        .iter()
        .filter_map(|c| c.manifest.clone().map(|m| (m, "sindri/core")))
        .collect();
    let coll_ctx = CollisionContext { target: &target };
    let plan = match CollisionResolver::new()
        .validate_and_resolve(&manifest_pairs, &coll_ctx)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Collision validation failed: {}", e);
            return EXIT_RESOLUTION_CONFLICT;
        }
    };

    // Build a name → manifest map for downstream lookup. (Lockfile entries
    // remain authoritative for ordering — collision plan only filters.)
    let skipped_names: std::collections::HashSet<String> = plan
        .skipped
        .iter()
        .map(|(m, _)| m.metadata.name.clone())
        .collect();
    for (m, why) in &plan.skipped {
        tracing::warn!(
            component = m.metadata.name.as_str(),
            "collision: skipping component — {}",
            why
        );
    }

    let apply_options = ApplyOptions::default();
    let mut failed = 0usize;
    let mut applied: Vec<&ResolvedComponent> = Vec::new();

    for comp in &lockfile.components {
        if skipped_names.contains(&comp.id.name) {
            println!(
                "  - {} {} (skipped by collision plan)",
                comp.id.to_address(),
                comp.version
            );
            continue;
        }

        print!("  Installing {} {}...", comp.id.to_address(), comp.version);
        match install_one(
            comp,
            comp.manifest.as_ref(),
            &target,
            &platform,
            &apply_options,
        )
        .await
        {
            Ok(outcome) => {
                println!(
                    " done (hooks={}, configured={}, validated={})",
                    outcome.hooks_ran, outcome.configured, outcome.validated
                );
                applied.push(comp);
            }
            Err(e) => {
                println!(" FAILED: {}", render_apply_err(&e));
                failed += 1;
            }
        }
    }

    if failed > 0 {
        eprintln!("\n{}/{} component(s) failed", failed, total);
        return EXIT_RESOLUTION_CONFLICT;
    }

    // Project-init pass (steps 6–8). Runs across every component that was
    // successfully installed AND has a manifest with project_init steps.
    if let Err(e) = run_project_init_pass(&applied, &target).await {
        eprintln!("project-init failed: {}", render_apply_err(&e));
        return EXIT_RESOLUTION_CONFLICT;
    }

    println!("\nApplied {} component(s) successfully.", applied.len());

    // ADR-007: auto-emit `sindri.<target>.bom.spdx.json` next to the lockfile
    // after a successful apply. Disabled by `--no-bom`. Failures here are
    // logged but do **not** flip the apply exit code: a successful install
    // followed by a write-permissions error on the SBOM should still be
    // reported as success.
    if !args.no_bom {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        match crate::commands::bom::auto_emit_after_apply(&lockfile, &cwd) {
            Ok(path) => println!("SBOM written to {}", path.display()),
            Err(e) => eprintln!("warning: failed to auto-emit SBOM: {}", e),
        }
    }

    EXIT_SUCCESS
}

/// Run hooks.pre_project_init, then ProjectInitExecutor across the closure,
/// then hooks.post_project_init. Components without a manifest contribute
/// nothing to this pass.
async fn run_project_init_pass(
    applied: &[&ResolvedComponent],
    target: &dyn Target,
) -> Result<(), ApplyError> {
    let hooks_executor = HooksExecutor::new();

    // 6. pre-project-init hooks (per component, in apply order).
    for comp in applied {
        if let Some(m) = comp.manifest.as_ref() {
            if let Some(h) = m.capabilities.hooks.as_ref() {
                let ctx = HookContext {
                    component: &comp.id.name,
                    version: &comp.version.0,
                    target,
                    env: &[],
                    workdir: ".",
                };
                hooks_executor.run_pre_project_init(h, &ctx).await?;
            }
        }
    }

    // 7. ProjectInitExecutor.
    let mut steps: Vec<(ComponentRef, &sindri_core::component::ProjectInitStep)> = Vec::new();
    for comp in applied {
        if let Some(m) = comp.manifest.as_ref() {
            if let Some(list) = m.capabilities.project_init.as_ref() {
                let cref = ComponentRef {
                    component_id: comp.id.clone(),
                    name: comp.id.name.clone(),
                };
                for step in list {
                    steps.push((cref.clone(), step));
                }
            }
        }
    }
    if !steps.is_empty() {
        let workdir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let pi_ctx = ProjectInitContext {
            target,
            workdir: workdir.as_path(),
            env: &[],
        };
        ProjectInitExecutor::new().run(&steps, &pi_ctx).await?;
    }

    // 8. post-project-init hooks (per component, in apply order).
    for comp in applied {
        if let Some(m) = comp.manifest.as_ref() {
            if let Some(h) = m.capabilities.hooks.as_ref() {
                let ctx = HookContext {
                    component: &comp.id.name,
                    version: &comp.version.0,
                    target,
                    env: &[],
                    workdir: ".",
                };
                hooks_executor.run_post_project_init(h, &ctx).await?;
            }
        }
    }

    Ok(())
}

fn render_apply_err(e: &ApplyError) -> String {
    e.to_string()
}

fn compute_hash(content: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(content.as_bytes());
    hex::encode(h.finalize())
}
