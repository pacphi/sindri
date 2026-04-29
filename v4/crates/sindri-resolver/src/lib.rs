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

use sindri_core::component::{ComponentManifest, InstallConfig};
use sindri_core::lockfile::Lockfile;
use sindri_core::manifest::BomManifest;
use sindri_core::platform::{Capabilities, Platform, TargetProfile};
use sindri_core::policy::InstallPolicy;
use sindri_core::registry::ComponentEntry;
use sindri_core::source_descriptor::SourceDescriptor;
use sindri_registry::source::{ComponentName, RegistrySource};
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
    /// Root of the sindri registry cache (e.g. `~/.sindri/cache/registries/`).
    ///
    /// When set, the resolver looks for per-component `component.yaml` files
    /// under `<cache_root>/<registry_name_safe>/components/<name>/component.yaml`
    /// and populates the lockfile's `platforms` field from them, enabling
    /// Gate 1 to fire on subsequent `--offline` resolves (Wave 6A / ADR-008).
    ///
    /// `None` means no component-manifest lookup; platforms will not be
    /// persisted in the lockfile for this resolution run.
    pub registry_cache_root: Option<PathBuf>,
    /// Strict-OCI admission gate (DDD-08, ADR-028 — Phase 2).
    ///
    /// When `true`, every component recorded in the lockfile MUST be
    /// served by a source whose [`SourceDescriptor`] is `Oci` or
    /// `LocalOci`; any other descriptor (e.g. `LocalPath`, `Git`) causes
    /// the resolver to return
    /// [`ResolverError::SourceNotProductionGrade`].
    ///
    /// Per ADR-028 Q3 the CLI flag overrides
    /// `registry.policy.strict_oci` in `sindri.yaml`; the CLI is
    /// responsible for applying the precedence before calling the
    /// resolver.
    pub strict_oci: bool,
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
            registry_cache_root: None,
            strict_oci: false,
        }
    }
}

/// Try to load a `ComponentManifest` from the local registry cache.
///
/// Looks for `<cache_root>/<registry_name_safe>/components/<name>/component.yaml`
/// where `registry_name_safe` replaces `'/'` with `'_'` for filesystem safety.
/// Returns `None` on any error (missing file, parse failure) -- callers treat
/// absent manifests as "platforms unknown".
fn load_component_manifest_from_cache(
    cache_root: &std::path::Path,
    registry_name: &str,
    component_name: &str,
) -> Option<ComponentManifest> {
    // Sanitise the registry name for filesystem use (replace '/' with '_').
    let safe_registry = registry_name.replace('/', "_");
    let path = cache_root
        .join(&safe_registry)
        .join("components")
        .join(component_name)
        .join("component.yaml");
    let content = std::fs::read_to_string(&path).ok()?;
    serde_yaml::from_str(&content).ok()
}

/// Main resolution pipeline: manifest -> registry -> closure -> gates -> backend -> lockfile.
///
/// Backwards-compatible entry point retained for existing call sites that
/// have not yet adopted the source-modes API (DDD-08, ADR-028 — Phase 1).
/// Forwards to [`resolve_with_sources`] with an empty source slice; the
/// lockfile writer then synthesises an `Oci { ... }` `SourceDescriptor` from
/// the legacy `entry.oci_ref` field for each component.
pub fn resolve(
    opts: &ResolveOptions,
    registry: &HashMap<String, ComponentEntry>,
    policy: &InstallPolicy,
    platform: &Platform,
) -> Result<Lockfile, ResolverError> {
    resolve_with_sources(opts, registry, &[], policy, platform)
}

/// Source-aware resolution pipeline (DDD-08, ADR-028 — Phase 1.3).
///
/// Identical to [`resolve`] but additionally consults `sources` (in declared
/// order, first-match-wins) when picking a [`SourceDescriptor`] to record on
/// each lockfile entry. Sources contribute scope filtering (DDD-08 §"Source
/// scope") only in Phase 1; full per-source byte fetch lands in Phase 2/3.
pub fn resolve_with_sources(
    opts: &ResolveOptions,
    registry: &HashMap<String, ComponentEntry>,
    sources: &[RegistrySource],
    policy: &InstallPolicy,
    platform: &Platform,
) -> Result<Lockfile, ResolverError> {
    if opts.offline {
        return resolve_offline(opts, registry, sources, policy, platform);
    }
    resolve_online(opts, registry, sources, policy, platform)
}

/// Pick the [`SourceDescriptor`] that the lockfile should record for a
/// component named `name`. Walks `sources` in declared order, returning the
/// descriptor of the first source whose `scope` matches (or has no scope).
/// Falls back to reconstructing an `Oci { ... }` descriptor from the legacy
/// `entry.oci_ref` field when no source matches — preserving Phase-0
/// behaviour for callers that pass an empty slice.
fn pick_source_descriptor(
    sources: &[RegistrySource],
    entry: &ComponentEntry,
) -> Option<SourceDescriptor> {
    let cname = ComponentName::from(entry.name.as_str());
    for src in sources {
        if src.scope_matches(&cname) {
            return Some(src.dispatch_lockfile_descriptor());
        }
    }
    sindri_registry::source::oci_descriptor_from_legacy_ref(&entry.oci_ref)
}

/// Online resolution pipeline.
///
/// Expands the dependency closure, loads per-component manifests from the
/// local registry cache (to run Gate 1 with real platform data and persist
/// platforms in the lockfile for future offline resolves), runs all
/// admission gates, then writes the lockfile.
fn resolve_online(
    opts: &ResolveOptions,
    registry: &HashMap<String, ComponentEntry>,
    sources: &[RegistrySource],
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

    // 3. Run admission gates.
    //
    // For each component, try to load its `component.yaml` from the local
    // registry cache and build a `CandidateRef::with_manifest` so that Gate 1
    // (platform eligibility, ADR-008) fires with real data.  When no cached
    // manifest is available, fall back to `CandidateRef::from_entry` which
    // records `ADM_PLATFORM_SKIPPED` in the audit trail but does not fail.
    let target_profile = TargetProfile {
        platform: platform.clone(),
        capabilities: Capabilities::default(),
    };
    let checker = admission::AdmissionChecker::new(policy, &target_profile);
    let registry_name = sindri_core::registry::CORE_REGISTRY_NAME;

    // Load per-component manifests from cache (if cache root is configured).
    let component_manifests: HashMap<String, ComponentManifest> = opts
        .registry_cache_root
        .as_deref()
        .map(|root| {
            closure_nodes
                .iter()
                .filter_map(|n| {
                    let m = load_component_manifest_from_cache(root, registry_name, &n.entry.name)?;
                    Some((n.entry.name.clone(), m))
                })
                .collect()
        })
        .unwrap_or_default();

    let candidates: Vec<admission::CandidateRef<'_>> = closure_nodes
        .iter()
        .map(|n| match component_manifests.get(&n.entry.name) {
            Some(m) => admission::CandidateRef::with_manifest(&n.entry, m, registry_name),
            None => admission::CandidateRef::from_entry(&n.entry, registry_name),
        })
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
        // Persist the component's platforms so offline Gate 1 can fire later.
        let platforms = component_manifests
            .get(&node.entry.name)
            .map(|m| m.platforms.clone());
        let source = pick_source_descriptor(sources, &node.entry);
        let resolved = lockfile_writer::resolved_from_entry(
            &node.entry,
            chosen,
            &address,
            opts.registry_manifest_digest.as_deref(),
            component_digest.as_deref(),
            platforms,
            source,
        );
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

    // 6. Strict-OCI admission gate (DDD-08, ADR-028 — Phase 2).
    apply_strict_oci_gate(opts, &lockfile)?;

    // 7. Write lockfile
    lockfile_writer::write_lockfile(&opts.lockfile_path, &lockfile)?;

    Ok(lockfile)
}

/// Offline resolution path (ADR-008 Gate 1, Wave 6A).
///
/// # Design decision (Option 2)
///
/// Three options were evaluated for running Gate 1 in `--offline` mode:
///
/// 1. **Always fetch manifest** -- make a one-shot OCI network call even in
///    offline mode (contradicts the offline contract).
/// 2. **Persist platforms in the lockfile** -- the online resolve records the
///    `platforms` array from each component's `component.yaml` into the
///    lockfile; the offline path reads those values back.  Selected.
/// 3. **Document the gap** -- gate 1 is simply skipped for offline resolves.
///
/// Option 2 keeps `--offline` strictly offline while giving Gate 1 real data
/// on every subsequent re-resolve.  The lockfile schema extension is additive
/// (`platforms: Option<Vec<Platform>>` with `#[serde(default)]`) so existing
/// lockfiles without the field continue to deserialize.
///
/// # Behaviour when no lockfile exists
///
/// If no lockfile is present (first resolve is `--offline`), the path falls
/// back to the BOM manifest + registry cache -- identical to a fresh online
/// resolve, but without writing the lockfile.  Components without a
/// cached `platforms` entry fall back to `CandidateRef::from_entry` (Gate 1
/// skipped non-fatally with `ADM_PLATFORM_SKIPPED`).
///
/// # Behaviour when lockfile exists with `platforms`
///
/// Platforms from the lockfile take precedence over anything in the registry
/// cache (the lockfile was written by the previous authoritative online
/// resolve). Gate 1 runs with real data; an unsupported platform produces
/// `ResolverError::AdmissionDenied { code: "ADM_PLATFORM_UNSUPPORTED" }`.
fn resolve_offline(
    opts: &ResolveOptions,
    registry: &HashMap<String, ComponentEntry>,
    sources: &[RegistrySource],
    policy: &InstallPolicy,
    platform: &Platform,
) -> Result<Lockfile, ResolverError> {
    // Attempt to read an existing lockfile to extract cached platform data.
    // If absent, proceed without it (components get from_entry / SKIPPED).
    let existing_lock: Option<Lockfile> = if opts.lockfile_path.exists() {
        lockfile_writer::read_lockfile(&opts.lockfile_path).ok()
    } else {
        None
    };

    // Build a name->platforms map from the existing lockfile.
    let locked_platforms: HashMap<String, Vec<sindri_core::platform::Platform>> = existing_lock
        .as_ref()
        .map(|lf| {
            lf.components
                .iter()
                .filter_map(|rc| {
                    rc.platforms
                        .as_ref()
                        .map(|p| (rc.id.name.clone(), p.clone()))
                })
                .collect()
        })
        .unwrap_or_default();

    // 1. Load manifest
    let bom_content = std::fs::read_to_string(&opts.manifest_path)?;
    let bom: BomManifest = serde_yaml::from_str(&bom_content)
        .map_err(|e| ResolverError::Serialization(e.to_string()))?;

    // 2. Expand dependency closure
    let root_addrs: Vec<String> = bom.components.iter().map(|c| c.address.clone()).collect();
    let closure_nodes = closure::expand_closure(&root_addrs, registry)?;

    // 3. Run admission gates using platform data from the lockfile.
    let target_profile = TargetProfile {
        platform: platform.clone(),
        capabilities: Capabilities::default(),
    };
    let checker = admission::AdmissionChecker::new(policy, &target_profile);
    let registry_name = sindri_core::registry::CORE_REGISTRY_NAME;

    // Build synthetic manifests from the lockfile's cached platform data.
    let synthetic_manifests: HashMap<String, ComponentManifest> = closure_nodes
        .iter()
        .filter_map(|n| {
            locked_platforms.get(&n.entry.name).map(|platforms| {
                let m = build_platform_manifest(&n.entry.name, n.entry.latest.clone(), platforms);
                (n.entry.name.clone(), m)
            })
        })
        .collect();

    let candidates: Vec<admission::CandidateRef<'_>> = closure_nodes
        .iter()
        .map(|n| match synthetic_manifests.get(&n.entry.name) {
            Some(m) => admission::CandidateRef::with_manifest(&n.entry, m, registry_name),
            None => admission::CandidateRef::from_entry(&n.entry, registry_name),
        })
        .collect();
    checker.admit_all(&candidates)?;

    // 4. Choose backends and build lockfile.
    //
    // For each component, carry over the `platforms` data from the existing
    // lockfile (or from the synthetic manifest if constructed above).
    let bom_hash = lockfile_writer::compute_bom_hash(&bom_content);
    let mut lockfile = Lockfile::new(bom_hash, opts.target_name.clone());

    let target_kind = opts.target_kind.as_deref();
    for node in &closure_nodes {
        if let Some(ref exp) = opts.explain {
            let addr = node.id.to_address();
            if addr.contains(exp) {
                let explanation = backend_choice::explain_choice(&node.entry, platform);
                println!("{}", explanation);
            }
        }

        let chosen =
            backend_choice::choose_backend_for_target(&node.entry, platform, target_kind, None);
        // Carry through the platforms from the locked data.
        let platforms = locked_platforms.get(&node.entry.name).cloned();
        let address = node.id.to_address();
        let component_digest = opts.component_digests.get(&address).cloned();
        let source = pick_source_descriptor(sources, &node.entry);
        let resolved = lockfile_writer::resolved_from_entry(
            &node.entry,
            chosen,
            &address,
            opts.registry_manifest_digest.as_deref(),
            component_digest.as_deref(),
            platforms,
            source,
        );
        lockfile.components.push(resolved);
    }

    // 5. Strict-OCI admission gate (DDD-08, ADR-028 — Phase 2).
    apply_strict_oci_gate(opts, &lockfile)?;

    // 6. Write lockfile (preserving platforms for the next offline resolve).
    lockfile_writer::write_lockfile(&opts.lockfile_path, &lockfile)?;

    Ok(lockfile)
}

/// Build a minimal `ComponentManifest` stub that carries only the `platforms`
/// list.  All other fields are set to safe defaults.  Used by the offline
/// resolve path to synthesise a manifest for Gate 1 from lockfile data.
fn build_platform_manifest(
    name: &str,
    version: String,
    platforms: &[Platform],
) -> ComponentManifest {
    use sindri_core::component::{ComponentCapabilities, ComponentMetadata, Options};
    ComponentManifest {
        metadata: ComponentMetadata {
            name: name.to_string(),
            version,
            description: String::new(),
            license: String::new(),
            tags: vec![],
            homepage: None,
        },
        platforms: platforms.to_vec(),
        install: InstallConfig::default(),
        depends_on: vec![],
        capabilities: ComponentCapabilities::default(),
        options: Options::default(),
        validate: None,
        configure: None,
        remove: None,
        overrides: Default::default(),
        auth: Default::default(),
    }
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

/// Strict-OCI admission gate (DDD-08, ADR-028 — Phase 2).
///
/// Walks the lockfile after resolution and either:
///
/// - Returns [`ResolverError::SourceNotProductionGrade`] if `strict_oci`
///   is set and any component carries a non-production-grade source
///   descriptor (anything other than `Oci` or `LocalOci`).
/// - Emits exactly one `tracing::warn!` summarising the source mix when
///   `strict_oci` is *off* and at least one component used a
///   non-production-grade source — keeps CI logs noisy enough that a
///   user can spot drift, but not so noisy that every component
///   produces its own line.
fn apply_strict_oci_gate(opts: &ResolveOptions, lockfile: &Lockfile) -> Result<(), ResolverError> {
    let mut offenders: Vec<(String, String)> = Vec::new();
    for c in &lockfile.components {
        let kind = match &c.source {
            Some(s) => s.kind(),
            None => "<unknown>",
        };
        if !descriptor_is_production_grade(c.source.as_ref()) {
            offenders.push((c.id.to_address(), kind.to_string()));
        }
    }

    if opts.strict_oci {
        if offenders.is_empty() {
            tracing::info!(
                "strict-OCI gate passed for target '{}' ({} components)",
                opts.target_name,
                lockfile.components.len()
            );
            return Ok(());
        }
        return Err(ResolverError::SourceNotProductionGrade { offenders });
    }

    if !offenders.is_empty() {
        // Loud-but-once warning. Per the plan: a single tracing::warn!,
        // not one per component, so CI logs aren't spammed.
        let summary: Vec<String> = offenders
            .iter()
            .map(|(c, k)| format!("{} (source={})", c, k))
            .collect();
        tracing::warn!(
            "non-strict resolve: {} component(s) produced by non-production-grade sources — \
             enable `--strict-oci` (or `registry.policy.strict_oci: true`) to fail-closed: {}",
            offenders.len(),
            summary.join(", ")
        );
    }

    Ok(())
}

/// `true` when this descriptor is one of the strict-OCI-eligible kinds.
fn descriptor_is_production_grade(source: Option<&SourceDescriptor>) -> bool {
    match source {
        Some(SourceDescriptor::Oci { .. }) | Some(SourceDescriptor::LocalOci { .. }) => true,
        // `LocalPath`, `Git`, and missing descriptors all fail the gate.
        _ => false,
    }
}

#[cfg(test)]
mod strict_oci_tests {
    use super::*;
    use sindri_core::lockfile::{Lockfile, ResolvedComponent};

    fn lockfile_with_descriptors(descriptors: Vec<Option<SourceDescriptor>>) -> Lockfile {
        use sindri_core::component::Backend;
        use sindri_core::component::ComponentId as CoreComponentId;
        let mut lf = Lockfile::new("hash".into(), "local".into());
        for (i, d) in descriptors.into_iter().enumerate() {
            let rc = ResolvedComponent {
                id: CoreComponentId {
                    backend: Backend::Mise,
                    name: format!("comp-{}", i),
                    qualifier: None,
                },
                version: sindri_core::version::Version::new("1.0.0"),
                backend: Backend::Mise,
                oci_digest: None,
                checksums: std::collections::HashMap::new(),
                depends_on: vec![],
                manifest: None,
                manifest_digest: None,
                component_digest: None,
                platforms: None,
                source: d,
            };
            lf.components.push(rc);
        }
        lf
    }

    fn opts_strict(strict: bool) -> ResolveOptions {
        ResolveOptions {
            strict_oci: strict,
            ..ResolveOptions::default()
        }
    }

    #[test]
    fn strict_off_with_non_production_sources_emits_warn_but_passes() {
        let lf = lockfile_with_descriptors(vec![
            Some(SourceDescriptor::LocalPath {
                path: std::path::PathBuf::from("/x"),
            }),
            Some(SourceDescriptor::Oci {
                url: "oci://x".into(),
                tag: "1".into(),
                manifest_digest: None,
            }),
        ]);
        // Should NOT return an error; just warn.
        apply_strict_oci_gate(&opts_strict(false), &lf).expect("non-strict mode is permissive");
    }

    #[test]
    fn strict_on_with_local_path_rejects() {
        let lf = lockfile_with_descriptors(vec![
            Some(SourceDescriptor::LocalPath {
                path: std::path::PathBuf::from("/x"),
            }),
            Some(SourceDescriptor::Oci {
                url: "oci://x".into(),
                tag: "1".into(),
                manifest_digest: None,
            }),
        ]);
        let err = apply_strict_oci_gate(&opts_strict(true), &lf)
            .expect_err("strict mode must reject LocalPath");
        match err {
            ResolverError::SourceNotProductionGrade { offenders } => {
                assert_eq!(offenders.len(), 1);
                assert_eq!(offenders[0].1, "local-path");
            }
            other => panic!("expected SourceNotProductionGrade, got {:?}", other),
        }
    }

    #[test]
    fn strict_on_with_only_oci_and_local_oci_passes() {
        let lf = lockfile_with_descriptors(vec![
            Some(SourceDescriptor::Oci {
                url: "oci://x".into(),
                tag: "1".into(),
                manifest_digest: Some("sha256:aa".into()),
            }),
            Some(SourceDescriptor::LocalOci {
                layout_path: std::path::PathBuf::from("/layout"),
                manifest_digest: Some("sha256:bb".into()),
            }),
        ]);
        apply_strict_oci_gate(&opts_strict(true), &lf)
            .expect("strict mode must accept Oci + LocalOci");
    }

    #[test]
    fn strict_on_with_git_descriptor_rejects() {
        let lf = lockfile_with_descriptors(vec![Some(SourceDescriptor::Git {
            url: "https://x".into(),
            commit_sha: "abcd".into(),
            subdir: None,
        })]);
        let err = apply_strict_oci_gate(&opts_strict(true), &lf)
            .expect_err("strict mode must reject Git");
        match err {
            ResolverError::SourceNotProductionGrade { offenders } => {
                assert_eq!(offenders[0].1, "git");
            }
            other => panic!("expected SourceNotProductionGrade, got {:?}", other),
        }
    }

    #[test]
    fn strict_on_with_missing_descriptor_rejects() {
        let lf = lockfile_with_descriptors(vec![None]);
        let err = apply_strict_oci_gate(&opts_strict(true), &lf)
            .expect_err("strict mode must reject missing descriptors");
        assert!(matches!(
            err,
            ResolverError::SourceNotProductionGrade { .. }
        ));
    }

    #[test]
    fn exit_code_for_source_not_production_grade_is_admission_2() {
        let err = ResolverError::SourceNotProductionGrade {
            offenders: vec![("mise:nodejs".into(), "local-path".into())],
        };
        assert_eq!(err.exit_code(), 2);
    }
}
