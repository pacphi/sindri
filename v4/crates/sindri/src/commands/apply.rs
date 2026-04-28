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
//!
//! # Wave 5H — `--resume` / `--clear-state` (D19)
//!
//! `sindri apply --resume` retries from the failing component instead of
//! restarting the whole apply.  State is persisted to
//! `~/.sindri/apply-state/<bom-hash>.jsonl` (append-only JSONL, one record
//! per state transition).
//!
//! State machine per component:
//!
//! ```text
//! pending → pre_install → installing → configuring → validating
//!         → post_install → pre_project_init → project_init
//!         → post_project_init → completed
//!                                   ↑ failed{stage, error} on any error
//! ```
//!
//! On `--resume`, the store is loaded and components already in `completed`
//! state are skipped.  Components in `failed` or `pending` state are
//! re-attempted.
//!
//! `--clear-state` wipes the state file for the current BOM hash so the user
//! can force a clean-slate apply after fixing config drift.
//!
//! Concurrent-apply protection: an exclusive flock is taken on the state file;
//! if another process already holds it the command exits with
//! [`EXIT_APPLY_IN_PROGRESS`] (code 6, ADR-012).
//!
//! The cosign pre-flight (PR #228) runs at the top of **every** apply,
//! including `--resume` — it is cheap and idempotent.

use crate::commands::apply_lifecycle::{install_one, ApplyError, ApplyOptions};
use sindri_core::apply_state::{
    now_rfc3339, try_lock_state_file, ApplyStateStore, ComponentStage, RecordStatus, StateError,
    StateRecord,
};
use sindri_core::component::ComponentManifest;
use sindri_core::exit_codes::{
    EXIT_APPLY_IN_PROGRESS, EXIT_RESOLUTION_CONFLICT, EXIT_STALE_LOCKFILE, EXIT_SUCCESS,
};
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
    /// Resume from the last failing component instead of restarting (Wave 5H).
    pub resume: bool,
    /// Wipe the apply-state file for the current BOM (Wave 5H).
    pub clear_state: bool,
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

    // ---------------------------------------------------------------------------
    // Wave 5H: state-file management
    // ---------------------------------------------------------------------------

    // Derive the BOM hash from `sindri.yaml` if it exists; fall back to the
    // lockfile content.  This ensures two different BOMs never share state.
    let bom_content_for_hash = if Path::new("sindri.yaml").exists() {
        std::fs::read_to_string("sindri.yaml").unwrap_or_else(|_| content.clone())
    } else {
        content.clone()
    };

    let state_path = match ApplyStateStore::path_for_bom(&bom_content_for_hash) {
        Some(p) => p,
        None => {
            // No home directory — skip state file entirely.
            eprintln!(
                "warning: could not determine home directory; apply-state will not be persisted"
            );
            // Fall back to a temp path so the rest of the code compiles.
            std::env::temp_dir().join("sindri-apply-state-fallback.jsonl")
        }
    };

    // --clear-state: delete the state file and exit (unless combined with --resume).
    if args.clear_state {
        let store = match ApplyStateStore::open(state_path.clone()) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to open apply-state: {}", e);
                return EXIT_RESOLUTION_CONFLICT;
            }
        };
        match store.clear() {
            Ok(()) => {
                println!("Apply-state cleared for this BOM.");
                // If only --clear-state (without --resume), stop here.
                if !args.resume {
                    return EXIT_SUCCESS;
                }
            }
            Err(e) => {
                eprintln!("Failed to clear apply-state: {}", e);
                return EXIT_RESOLUTION_CONFLICT;
            }
        }
    }

    // Open the state store and acquire the exclusive flock.
    let store = match ApplyStateStore::open(state_path.clone()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to open apply-state: {}", e);
            return EXIT_RESOLUTION_CONFLICT;
        }
    };

    let _lock = match try_lock_state_file(&state_path) {
        Ok(l) => l,
        Err(StateError::AlreadyRunning { path }) => {
            eprintln!("error: {}", StateError::AlreadyRunning { path });
            return EXIT_APPLY_IN_PROGRESS;
        }
        Err(e) => {
            eprintln!("Failed to lock apply-state: {}", e);
            return EXIT_RESOLUTION_CONFLICT;
        }
    };

    // Load prior run's state if --resume.
    let prior_summary = if args.resume {
        match store.load_summary() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to load apply-state for --resume: {}", e);
                return EXIT_RESOLUTION_CONFLICT;
            }
        }
    } else {
        sindri_core::apply_state::ApplyStateSummary::default()
    };

    // ---------------------------------------------------------------------------
    // Wave 5A — D5: per-component cosign pre-flight.
    //
    // Runs on EVERY apply (including --resume) — it is cheap and idempotent
    // per the PR #228 contract.
    // ---------------------------------------------------------------------------
    if let Err(e) = preflight_component_signatures(&lockfile).await {
        eprintln!("Component signature verification failed: {}", e);
        return EXIT_RESOLUTION_CONFLICT;
    }

    let target = LocalTarget::new();
    let total = lockfile.components.len();

    if total == 0 {
        println!("Nothing to apply — lockfile is empty.");
        return EXIT_SUCCESS;
    }

    // When resuming, report how many components will be skipped.
    let skip_count = if args.resume {
        lockfile
            .components
            .iter()
            .filter(|c| prior_summary.is_completed(&c.id.name))
            .count()
    } else {
        0
    };

    if args.resume && skip_count > 0 {
        println!(
            "Resuming apply: {} of {} component(s) already completed, {} remaining.",
            skip_count,
            total,
            total - skip_count
        );
    } else {
        println!(
            "Plan: {} component(s) to apply on {}:",
            total, lockfile.target
        );
    }

    for comp in &lockfile.components {
        let marker = if args.resume && prior_summary.is_completed(&comp.id.name) {
            "(already completed)"
        } else {
            ""
        };
        println!(
            "  + {} {} ({}) {}",
            comp.id.to_address(),
            comp.version,
            comp.backend.as_str(),
            marker
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
        let name = &comp.id.name;

        if skipped_names.contains(name) {
            println!(
                "  - {} {} (skipped by collision plan)",
                comp.id.to_address(),
                comp.version
            );
            continue;
        }

        // --resume: skip components that already completed in a prior run.
        if args.resume && prior_summary.is_completed(name) {
            println!(
                "  - {} {} (skipped — already completed)",
                comp.id.to_address(),
                comp.version
            );
            applied.push(comp);
            continue;
        }

        // Record transition: pending → installing
        let _ = store.append(&StateRecord {
            component: name.clone(),
            stage: ComponentStage::Installing,
            status: RecordStatus::InProgress,
            error: None,
            ts: now_rfc3339(),
        });

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
                // Record: completed
                let _ = store.append(&StateRecord {
                    component: name.clone(),
                    stage: ComponentStage::Completed,
                    status: RecordStatus::Completed,
                    error: None,
                    ts: now_rfc3339(),
                });
                applied.push(comp);
            }
            Err(e) => {
                let err_msg = render_apply_err(&e);
                println!(" FAILED: {}", err_msg);
                // Record: failed
                let _ = store.append(&StateRecord {
                    component: name.clone(),
                    stage: ComponentStage::Failed,
                    status: RecordStatus::Failed,
                    error: Some(err_msg),
                    ts: now_rfc3339(),
                });
                failed += 1;
            }
        }
    }

    if failed > 0 {
        eprintln!("\n{}/{} component(s) failed", failed, total);
        eprintln!(
            "hint: fix the issue and run `sindri apply --resume` to retry failed component(s)."
        );
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

/// Wave 5A — D5: per-component cosign signature pre-flight.
///
/// Behaviour matrix (loaded against `sindri.policy.yaml`, defaulting to
/// `require_signed_registries=false`):
///
/// | policy strict | digest present | trust keys | outcome                    |
/// |---------------|----------------|------------|----------------------------|
/// | false         | —              | —          | warn-and-proceed (legacy)  |
/// | true          | None           | —          | error (digest required)    |
/// | true          | Some           | empty      | error (no trusted keys)    |
/// | true          | Some           | non-empty  | verify; pass-or-error      |
/// | false         | Some           | non-empty  | verify; warn-on-fail       |
///
/// Components that have **no** `oci_digest` (resolved from a non-OCI
/// backend like brew/cargo) are skipped: there is no OCI artifact to sign.
///
/// This runs on every apply including `--resume` — it is idempotent per
/// the PR #228 contract.
async fn preflight_component_signatures(
    lockfile: &sindri_core::lockfile::Lockfile,
) -> Result<(), String> {
    let strict = load_install_policy()
        .and_then(|p| p.require_signed_registries)
        .unwrap_or(false);

    // Best-effort trust-store load. A missing `~/.sindri/trust/` is treated
    // as "no keys" — the matrix above describes how that interacts with
    // strict mode.
    let trust_dir = sindri_core::paths::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".sindri")
        .join("trust");
    let verifier =
        sindri_registry::CosignVerifier::load_from_trust_dir(&trust_dir).map_err(|e| {
            format!(
                "failed to load cosign trust keys from {}: {}",
                trust_dir.display(),
                e
            )
        })?;

    for comp in &lockfile.components {
        // Components without an OCI ref were resolved from a registry-less
        // backend (cargo, brew, …) — there is no OCI artifact to sign.
        let oci_str = match comp.oci_digest.as_deref() {
            Some(s) => s,
            None => continue,
        };

        match comp.component_digest.as_deref() {
            None => {
                if strict {
                    return Err(format!(
                        "component '{}' is missing `component_digest` but policy.require_signed_registries=true \
                         (re-resolve to populate the digest, or relax the policy)",
                        comp.id.to_address()
                    ));
                }
                tracing::warn!(
                    component = comp.id.to_address().as_str(),
                    "no component_digest — skipping cosign verification (permissive policy)"
                );
            }
            Some(digest) => {
                let oci_ref = match sindri_registry::OciRef::parse(oci_str) {
                    Ok(r) => r,
                    Err(e) => {
                        return Err(format!(
                            "component '{}' has malformed oci_digest '{}': {}",
                            comp.id.to_address(),
                            oci_str,
                            e
                        ));
                    }
                };
                let registry_name = oci_ref.registry.clone();
                let trusted_keys = verifier.keys_for(&registry_name);
                if trusted_keys.is_empty() {
                    if strict {
                        return Err(format!(
                            "component '{}' references registry '{}' but no trusted cosign keys are loaded",
                            comp.id.to_address(),
                            registry_name
                        ));
                    }
                    tracing::warn!(
                        component = comp.id.to_address().as_str(),
                        registry = registry_name.as_str(),
                        "no trusted cosign keys — skipping per-component verification"
                    );
                    continue;
                }
                // Real OCI fetch + cosign verification. The verifier wraps
                // a default `oci_client::Client` internally — `sindri apply`
                // does not otherwise need a long-lived client.
                match verifier
                    .verify_component_signature_default_client(
                        &registry_name,
                        &oci_ref,
                        digest,
                        strict,
                    )
                    .await
                {
                    Ok(key_id) if key_id != "<unsigned>" => {
                        tracing::info!(
                            component = comp.id.to_address().as_str(),
                            "verified per-component cosign signature against key {}",
                            key_id
                        );
                    }
                    Ok(_) => {
                        // `<unsigned>` only happens with empty trust set +
                        // permissive policy — which we already handled above.
                    }
                    Err(e) => {
                        if strict {
                            return Err(format!(
                                "component '{}' cosign verification failed: {}",
                                comp.id.to_address(),
                                e
                            ));
                        }
                        tracing::warn!(
                            component = comp.id.to_address().as_str(),
                            "cosign verification failed (permissive policy): {}",
                            e
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

fn load_install_policy() -> Option<sindri_core::policy::InstallPolicy> {
    let path = std::path::Path::new("sindri.policy.yaml");
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(path).ok()?;
    serde_yaml::from_str(&content).ok()
}

fn compute_hash(content: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(content.as_bytes());
    hex::encode(h.finalize())
}
