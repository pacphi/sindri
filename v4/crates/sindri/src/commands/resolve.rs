use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};
use sindri_core::manifest::BomManifest;
use sindri_core::platform::Platform;
use sindri_core::policy::InstallPolicy;
use sindri_core::registry::ComponentEntry;
use sindri_resolver::lockfile_writer::is_oci_source;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct ResolveArgs {
    pub manifest: String,
    pub offline: bool,
    pub refresh: bool,
    pub strict: bool,
    pub explain: Option<String>,
    pub target: String,
    pub json: bool,
}

pub fn run(args: ResolveArgs) -> i32 {
    let manifest_path = PathBuf::from(&args.manifest);
    if !manifest_path.exists() {
        if args.json {
            eprintln!(
                r#"{{"error":"FILE_NOT_FOUND","path":"{}","fix":"Create sindri.yaml or run sindri init"}}"#,
                args.manifest
            );
        } else {
            eprintln!("Manifest not found: {}", args.manifest);
            eprintln!("Hint: run `sindri init` to create a sindri.yaml");
        }
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }

    // Determine lockfile path -- per-target (ADR-018)
    let lock_name = if args.target == "local" {
        "sindri.lock".to_string()
    } else {
        format!("sindri.{}.lock", args.target)
    };
    let lockfile_path = manifest_path
        .parent()
        .unwrap_or(Path::new("."))
        .join(&lock_name);

    // Load registry from cache
    let registry = load_registry_from_cache();
    if registry.is_empty() && !args.offline && !args.json {
        eprintln!("Warning: no registry index found. Run `sindri registry refresh` first.");
        eprintln!("Proceeding with empty registry (no components will resolve).");
    }

    // Load policy (defaults for now; Sprint 6 adds full policy loading)
    let mut policy = InstallPolicy {
        preset: sindri_core::policy::PolicyPreset::Default,
        allowed_licenses: Vec::new(),
        denied_licenses: Vec::new(),
        on_unknown_license: None,
        require_signed_registries: None,
        require_checksums: None,
        offline: Some(args.offline),
        audit: None,
    };
    if args.strict {
        policy.preset = sindri_core::policy::PolicyPreset::Strict;
    }

    let platform = Platform::current();
    // Wave 3A.2: when the registry was fetched live via oci-client, its
    // manifest digest is recorded in the content-addressed cache. Surface
    // any one such digest into the lockfile so apply-time integrity checks
    // can prove "this lockfile was resolved against this exact index.yaml
    // snapshot." Per ADR-003 audit-delta, per-component digests are
    // deferred to Wave 5 (SBOM).
    let registry_manifest_digest = sindri_registry::RegistryCache::new()
        .ok()
        .and_then(|c| c.any_digest_for_registry(sindri_core::registry::CORE_REGISTRY_NAME));

    // Wave 5F — D18: extract the per-target kind from the BOM so the
    // resolver can pick a target-appropriate backend chain. We don't
    // hard-fail on a missing target entry — the resolver falls back to the
    // platform default for unknown / undeclared kinds.
    let target_kind = read_target_kind(&manifest_path, &args.target);

    // Wave 5F — D5 (carry-over from PR #228): pre-fetch per-component OCI
    // layer digests so the lockfile carries `component_digest` for OCI-backed
    // components. Components with non-OCI sources (local file, git, http)
    // are skipped by design and serialize `component_digest: None`.
    let component_digests = if args.offline {
        HashMap::new()
    } else {
        prefetch_component_digests(&manifest_path, &registry)
    };

    // Wave 6A: locate the registry cache root so the resolver can load
    // per-component manifests and persist their `platforms` lists in the
    // lockfile. This enables Gate 1 (ADR-008) to fire on subsequent
    // `--offline` resolves without any network calls.
    let registry_cache_root =
        sindri_core::paths::home_dir().map(|h| h.join(".sindri").join("cache").join("registries"));

    let opts = sindri_resolver::ResolveOptions {
        manifest_path: manifest_path.clone(),
        lockfile_path: lockfile_path.clone(),
        target_name: args.target.clone(),
        offline: args.offline,
        strict: args.strict,
        explain: args.explain.clone(),
        registry_manifest_digest,
        target_kind,
        component_digests,
        registry_cache_root,
    };

    match sindri_resolver::resolve(&opts, &registry, &policy, &platform) {
        Ok(lockfile) => {
            if args.json {
                println!(
                    r#"{{"resolved":true,"lockfile":"{}","components":{}}}"#,
                    lockfile_path.display(),
                    lockfile.components.len()
                );
            } else {
                println!(
                    "Resolved {} component(s) -> {}",
                    lockfile.components.len(),
                    lockfile_path.display()
                );
                for c in &lockfile.components {
                    println!(
                        "  {} {} ({})",
                        c.id.to_address(),
                        c.version,
                        c.backend.as_str()
                    );
                }
            }
            EXIT_SUCCESS
        }
        Err(e) => {
            let code = e.exit_code();
            if args.json {
                eprintln!(r#"{{"error":"{}","detail":"{}"}}"#, code, e);
            } else {
                eprintln!("resolve failed: {}", e);
            }
            code
        }
    }
}

/// Read the `kind` of the requested target from the BOM. Returns `None` if
/// the BOM cannot be parsed, the target isn't declared, or the kind field
/// is empty — in all of those cases the resolver falls back to the platform
/// default chain. Wave 5F — D18.
fn read_target_kind(manifest_path: &Path, target_name: &str) -> Option<String> {
    let content = std::fs::read_to_string(manifest_path).ok()?;
    let bom: BomManifest = serde_yaml::from_str(&content).ok()?;
    bom.targets
        .get(target_name)
        .map(|t| t.kind.clone())
        .filter(|k| !k.is_empty())
        // The CLI's `--target local` default is the universal fallback even
        // when the BOM doesn't declare a `targets.local` entry.
        .or_else(|| {
            if target_name == "local" {
                Some("local".to_string())
            } else {
                None
            }
        })
}

/// Pre-fetch the SHA-256 digest of each OCI-backed component's primary
/// layer. Returns a map keyed by component address (e.g. `"mise:nodejs"`).
/// Best-effort: any failure for a single component is logged and skipped —
/// the resolver tolerates missing entries (apply will fail closed under
/// `policy.require_signed_registries=true` only). Wave 5F — D5.
fn prefetch_component_digests(
    manifest_path: &Path,
    registry: &HashMap<String, ComponentEntry>,
) -> HashMap<String, String> {
    let content = match std::fs::read_to_string(manifest_path) {
        Ok(c) => c,
        Err(_) => return HashMap::new(),
    };
    let bom: BomManifest = match serde_yaml::from_str(&content) {
        Ok(b) => b,
        Err(_) => return HashMap::new(),
    };

    // Collect the (address, oci_ref) pairs we want to fetch. Skip non-OCI
    // sources up front to avoid pointless network attempts.
    let mut targets: Vec<(String, String)> = Vec::new();
    for entry in &bom.components {
        if let Some(reg_entry) = registry.get(&entry.address) {
            if is_oci_source(&reg_entry.oci_ref) {
                targets.push((entry.address.clone(), reg_entry.oci_ref.clone()));
            }
        }
    }
    if targets.is_empty() {
        return HashMap::new();
    }

    // Spin up a small runtime — the resolver itself is sync. Failures here
    // are non-fatal: apply tolerates missing component_digest under
    // permissive policy.
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            tracing::debug!("could not start tokio runtime for digest pre-fetch: {}", e);
            return HashMap::new();
        }
    };

    let client = match sindri_registry::RegistryClient::new() {
        Ok(c) => c,
        Err(e) => {
            tracing::debug!(
                "could not construct RegistryClient for digest pre-fetch: {}",
                e
            );
            return HashMap::new();
        }
    };

    let mut out: HashMap<String, String> = HashMap::new();
    runtime.block_on(async {
        for (addr, oci_ref) in &targets {
            match client.fetch_component_layer_digest(oci_ref).await {
                Ok(d) => {
                    tracing::debug!("component_digest({}) = {}", addr, d);
                    out.insert(addr.clone(), d);
                }
                Err(e) => {
                    tracing::debug!(
                        "skipping component_digest for {} ({}): {}",
                        addr,
                        oci_ref,
                        e
                    );
                }
            }
        }
    });
    out
}

fn load_registry_from_cache() -> HashMap<String, ComponentEntry> {
    let cache_root = sindri_core::paths::home_dir()
        .unwrap_or_default()
        .join(".sindri")
        .join("cache")
        .join("registries");

    let mut map: HashMap<String, ComponentEntry> = HashMap::new();

    let entries = match std::fs::read_dir(&cache_root) {
        Ok(e) => e,
        Err(_) => return map,
    };

    for entry in entries.flatten() {
        let index_path = entry.path().join("index.yaml");
        if !index_path.exists() {
            continue;
        }
        let content = match std::fs::read_to_string(&index_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let index: sindri_core::registry::RegistryIndex = match serde_yaml::from_str(&content) {
            Ok(i) => i,
            Err(_) => continue,
        };
        for comp in index.components {
            let addr = format!("{}:{}", comp.backend, comp.name);
            map.insert(addr, comp);
        }
    }

    map
}
