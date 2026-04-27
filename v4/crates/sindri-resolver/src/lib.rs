#![allow(dead_code)]

pub mod admission;
pub mod backend_choice;
pub mod closure;
pub mod error;
pub mod lockfile_writer;
pub mod version;

pub use error::ResolverError;

use std::collections::HashMap;
use std::path::PathBuf;
use sindri_core::lockfile::Lockfile;
use sindri_core::manifest::BomManifest;
use sindri_core::platform::{Platform, TargetProfile, Capabilities};
use sindri_core::policy::InstallPolicy;
use sindri_core::registry::ComponentEntry;

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
    let root_addrs: Vec<String> = bom
        .components
        .iter()
        .map(|c| c.address.clone())
        .collect();

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
        let resolved = lockfile_writer::resolved_from_entry(&node.entry, chosen, &node.id.to_address());
        lockfile.components.push(resolved);
    }

    // 5. Write lockfile
    lockfile_writer::write_lockfile(&opts.lockfile_path, &lockfile)?;

    Ok(lockfile)
}
