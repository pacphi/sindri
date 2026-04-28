#![allow(dead_code)]

pub mod admission;
pub mod backend_choice;
pub mod closure;
pub mod error;
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
    /// Live OCI manifest digest of the registry artifact this resolution
    /// was performed against. Populated by callers that fetched the index
    /// via `RegistryClient::fetch_index` (Wave 3A.2). When `None`, lockfile
    /// entries omit `manifest_digest`. ADR-003 audit-delta tracks moving
    /// this from registry-scoped to per-component when SBOM (Wave 5) lands.
    pub registry_manifest_digest: Option<String>,
    /// Target *kind* (e.g. `local`, `docker`, `k8s`). Drives the per-target
    /// backend preference chain (Wave 5F — D18, ADR-018). Defaults to
    /// `"local"` when unset, preserving pre-5F behaviour.
    #[doc(hidden)]
    pub target_kind: Option<String>,
    /// Pre-fetched per-component OCI layer digests, keyed by component
    /// address (e.g. `mise:nodejs`). Populated by the CLI's resolve command
    /// for OCI-backed components (Wave 5F — D5 carry-over from PR #228).
    /// Components without an entry here serialize `component_digest: None`
    /// (acceptable for local file / git / http tarball sources).
    #[doc(hidden)]
    pub component_digests: HashMap<String, String>,
}

impl Default for ResolveOptions {
    fn default() -> Self {
        Self {
            manifest_path: PathBuf::new(),
            lockfile_path: PathBuf::new(),
            target_name: "local".to_string(),
            offline: false,
            strict: false,
            explain: None,
            registry_manifest_digest: None,
            target_kind: None,
            component_digests: HashMap::new(),
        }
    }
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

    let target_kind = opts.target_kind.as_deref();
    for node in &closure_nodes {
        // Handle explain flag
        if let Some(ref exp) = opts.explain {
            let addr = node.id.to_address();
            if addr.contains(exp) {
                let explanation = backend_choice::explain_choice(&node.entry, platform);
                println!("{}", explanation);
            }
        }

        let chosen =
            backend_choice::choose_backend_for_target(&node.entry, platform, target_kind, None);
        let address = node.id.to_address();
        let component_digest = opts.component_digests.get(&address).cloned();
        let resolved = lockfile_writer::resolved_from_entry(
            &node.entry,
            chosen,
            &address,
            opts.registry_manifest_digest.as_deref(),
            component_digest.as_deref(),
        );
        lockfile.components.push(resolved);
    }

    // 5. Write lockfile
    lockfile_writer::write_lockfile(&opts.lockfile_path, &lockfile)?;

    Ok(lockfile)
}
