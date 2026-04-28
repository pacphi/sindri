#![allow(dead_code)]

pub mod admission;
pub mod auth_binding;
pub mod backend_choice;
pub mod closure;
pub mod error;
pub mod ledger;
pub mod lockfile_writer;
pub mod version;

pub use error::ResolverError;

use sindri_core::lockfile::Lockfile;
use sindri_core::manifest::BomManifest;
use sindri_core::platform::{Capabilities, Platform, TargetProfile};
use sindri_core::policy::InstallPolicy;
use sindri_core::registry::ComponentEntry;
use std::collections::HashMap;
use std::path::PathBuf;

/// Top-level resolver options
pub struct ResolveOptions {
    pub manifest_path: PathBuf,
    pub lockfile_path: PathBuf,
    pub target_name: String,
    pub offline: bool,
    pub strict: bool,
    pub explain: Option<String>,
}

/// Main resolution pipeline: manifest → registry → closure → gates → backend → lockfile
pub fn resolve(
    opts: &ResolveOptions,
    registry: &HashMap<String, ComponentEntry>,
    policy: &InstallPolicy,
    platform: &Platform,
) -> Result<Lockfile, ResolverError> {
    // 1. Load manifest
    let bom_content = std::fs::read_to_string(&opts.manifest_path)?;
    let bom: BomManifest = serde_yaml::from_str(&bom_content)
        .map_err(|e| ResolverError::Serialization(e.to_string()))?;

    // 2. Expand dependency closure
    let root_addrs: Vec<String> = bom.components.iter().map(|c| c.address.clone()).collect();

    let closure_nodes = closure::expand_closure(&root_addrs, registry)?;

    // 3. Run admission gates
    let target_profile = TargetProfile {
        platform: platform.clone(),
        capabilities: Capabilities::default(),
    };
    let checker = admission::AdmissionChecker::new(policy, &target_profile);
    // NOTE: until per-component OCI manifest fetch lands (Wave 2 territory),
    // only the registry-index entry is available. Gates 1 (platform) and 4
    // (capability trust) record a `Skipped` admission result in that case
    // rather than silently passing — see ADR-008.
    let registry_name = sindri_core::registry::CORE_REGISTRY_NAME;
    let candidates: Vec<admission::CandidateRef<'_>> = closure_nodes
        .iter()
        .map(|n| admission::CandidateRef::from_entry(&n.entry, registry_name))
        .collect();
    checker.admit_all(&candidates)?;

    // 4. Choose backends and build lockfile
    let bom_hash = lockfile_writer::compute_bom_hash(&bom_content);
    let mut lockfile = Lockfile::new(bom_hash, opts.target_name.clone());

    for node in &closure_nodes {
        // Handle explain flag
        if let Some(ref exp) = opts.explain {
            let addr = node.id.to_address();
            if addr.contains(exp) {
                let explanation = backend_choice::explain_choice(&node.entry, platform);
                println!("{}", explanation);
            }
        }

        let chosen = backend_choice::choose_backend(&node.entry, platform, None);
        let resolved =
            lockfile_writer::resolved_from_entry(&node.entry, chosen, &node.id.to_address());
        lockfile.components.push(resolved);
    }

    // 5. Auth-binding pass (ADR-027 §3, observability-only — Phase 1 of the
    //    auth-aware implementation plan). Bindings are derived from any
    //    ComponentManifests already attached to the resolved components
    //    (today: only those loaded by callers that pre-populate the field;
    //    full OCI-fetch integration arrives in a later wave). When no
    //    manifests carry auth requirements, this pass produces zero
    //    bindings and is a no-op.
    let target_caps = collect_target_capabilities(&bom, &opts.target_name);
    let comp_inputs = build_component_auth_inputs(&lockfile);
    if !comp_inputs.is_empty() {
        let inputs: Vec<auth_binding::ComponentAuthInput<'_>> = comp_inputs
            .iter()
            .map(|(addr, auth)| auth_binding::ComponentAuthInput {
                address: addr.clone(),
                auth,
            })
            .collect();
        let targets = vec![auth_binding::TargetAuthInput {
            target_id: opts.target_name.clone(),
            capabilities: target_caps,
        }];
        let pass = auth_binding::bind_all(&inputs, &targets);
        ledger::emit_pass_events(&inputs, &targets, &pass);
        lockfile.auth_bindings = pass.bindings;
    }

    // 6. Write lockfile
    lockfile_writer::write_lockfile(&opts.lockfile_path, &lockfile)?;

    Ok(lockfile)
}

/// Stitch `Target::auth_capabilities()` (Phase 4) and
/// `TargetConfig.provides:` (Phase 1) into the candidate list the binding
/// algorithm consumes. Built-in targets currently advertise no intrinsic
/// capabilities (Phase 4 fills these in), so today this returns the
/// per-manifest `provides:` overrides only.
fn collect_target_capabilities(
    bom: &BomManifest,
    target_name: &str,
) -> Vec<sindri_core::auth::AuthCapability> {
    bom.targets
        .get(target_name)
        .map(|tc| tc.provides.clone())
        .unwrap_or_default()
}

/// Walk the resolved component list and pair each component's address
/// with its declared [`AuthRequirements`]. Components without an attached
/// manifest (the common case until OCI fetch lands) contribute nothing.
fn build_component_auth_inputs(
    lockfile: &Lockfile,
) -> Vec<(String, sindri_core::auth::AuthRequirements)> {
    let mut out = Vec::new();
    for c in &lockfile.components {
        if let Some(m) = &c.manifest {
            if !m.auth.is_empty() {
                out.push((c.id.to_address(), m.auth.clone()));
            }
        }
    }
    out
}
