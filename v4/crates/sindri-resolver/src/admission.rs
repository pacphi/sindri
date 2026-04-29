//! ADR-008 admission gates.
//!
//! There are four admission gates that every component (root or transitive)
//! must pass before it can be written to the lockfile:
//!
//! 1. **Platform eligibility** — the host platform is in the manifest's
//!    `platforms:` list.
//! 2. **Policy eligibility** — license/signing/scope rules from the merged
//!    install policy. The canonical license rule lives in the
//!    [`sindri_policy::check`] module; this crate delegates to it.
//! 3. **Dependency closure** — gates 1, 2, and 4 must hold for every
//!    transitive dependency.
//! 4. **Capability trust** — `collision_handling.path_prefix` must be either
//!    `{component-name}/...` (any registry) or the literal `:shared` sentinel
//!    (only allowed when sourced from [`CORE_REGISTRY_NAME`]).

use crate::error::ResolverError;
use sindri_core::component::ComponentManifest;
use sindri_core::platform::TargetProfile;
use sindri_core::policy::InstallPolicy;
use sindri_core::registry::{ComponentEntry, CORE_REGISTRY_NAME, SHARED_PATH_PREFIX};
use sindri_policy::check::check_license;

/// Bundle of the data each gate may need.
///
/// `entry` is always available (it's the registry-index row). `manifest`
/// is only available once we've fetched the per-component OCI artifact;
/// gates that depend on manifest fields gracefully skip with a logged
/// warning when it is `None`.
#[derive(Debug, Clone, Copy)]
pub struct CandidateRef<'a> {
    /// Registry-index entry (always present).
    pub entry: &'a ComponentEntry,
    /// Full component manifest, if it has been fetched yet.
    pub manifest: Option<&'a ComponentManifest>,
    /// Canonical name of the registry this candidate came from
    /// (e.g. `"sindri/core"`).
    pub registry_name: &'a str,
}

impl<'a> CandidateRef<'a> {
    /// Construct from an entry only — manifest gates will be skipped.
    pub fn from_entry(entry: &'a ComponentEntry, registry_name: &'a str) -> Self {
        CandidateRef {
            entry,
            manifest: None,
            registry_name,
        }
    }

    /// Construct from an entry plus a fetched manifest — all gates run.
    pub fn with_manifest(
        entry: &'a ComponentEntry,
        manifest: &'a ComponentManifest,
        registry_name: &'a str,
    ) -> Self {
        CandidateRef {
            entry,
            manifest: Some(manifest),
            registry_name,
        }
    }
}

/// Outcome of a single gate evaluation.
#[derive(Debug, Clone)]
pub struct AdmissionResult {
    /// `true` for both `Admitted` and `Skipped`; `false` for `Denied`.
    pub allowed: bool,
    /// Machine-readable code (e.g. `OK`, `SKIPPED`, `ADM_LICENSE_DENIED`).
    pub code: String,
    /// Human-readable summary.
    pub message: String,
    /// Optional remediation hint.
    pub suggested_fix: Option<String>,
    /// `true` when the gate had to be skipped because required input was
    /// unavailable (e.g. manifest not yet fetched). Skipped gates are
    /// non-fatal but should be visible in audit reports.
    pub skipped: bool,
}

impl AdmissionResult {
    /// Successful admission.
    pub fn ok() -> Self {
        AdmissionResult {
            allowed: true,
            code: "OK".into(),
            message: "Admitted".into(),
            suggested_fix: None,
            skipped: false,
        }
    }

    /// Hard denial.
    pub fn deny(code: &str, message: &str, fix: Option<&str>) -> Self {
        AdmissionResult {
            allowed: false,
            code: code.into(),
            message: message.into(),
            suggested_fix: fix.map(|s| s.to_string()),
            skipped: false,
        }
    }

    /// Gate could not run because required input was unavailable.
    /// Treated as non-fatal but recorded for the audit report.
    pub fn skipped(code: &str, message: &str) -> Self {
        AdmissionResult {
            allowed: true,
            code: code.into(),
            message: message.into(),
            suggested_fix: None,
            skipped: true,
        }
    }
}

/// Runs the four admission gates from ADR-008.
pub struct AdmissionChecker<'a> {
    pub policy: &'a InstallPolicy,
    pub target: &'a TargetProfile,
}

impl<'a> AdmissionChecker<'a> {
    /// Construct a new checker bound to a policy and target profile.
    pub fn new(policy: &'a InstallPolicy, target: &'a TargetProfile) -> Self {
        AdmissionChecker { policy, target }
    }

    /// Gate 1 — Platform eligibility.
    ///
    /// Returns:
    /// - [`AdmissionResult::ok`] if the manifest's `platforms` list is empty
    ///   (universal) or contains the target platform.
    /// - [`AdmissionResult::deny`] with `ADM_PLATFORM_UNSUPPORTED` if the
    ///   manifest declares platforms and the target is not among them.
    /// - [`AdmissionResult::skipped`] if no manifest has been fetched yet.
    pub fn check_platform(&self, candidate: &CandidateRef<'_>) -> AdmissionResult {
        let Some(manifest) = candidate.manifest else {
            tracing::debug!(
                component = %candidate.entry.name,
                "platform check skipped: manifest not yet fetched"
            );
            return AdmissionResult::skipped(
                "ADM_PLATFORM_SKIPPED",
                "Manifest not yet fetched; platform gate deferred",
            );
        };

        // Empty platforms list = universal/no constraint.
        if manifest.platforms.is_empty() {
            return AdmissionResult::ok();
        }

        if manifest
            .platforms
            .iter()
            .any(|p| p == &self.target.platform)
        {
            AdmissionResult::ok()
        } else {
            let supported: Vec<String> = manifest
                .platforms
                .iter()
                .map(|p| p.triple().to_string())
                .collect();
            AdmissionResult::deny(
                "ADM_PLATFORM_UNSUPPORTED",
                &format!(
                    "Component `{}` does not support target {} (supported: {})",
                    candidate.entry.name,
                    self.target.platform.triple(),
                    supported.join(", "),
                ),
                Some("Choose a different target or a component that supports this platform"),
            )
        }
    }

    /// Gate 2 — Policy eligibility (license + scope).
    ///
    /// Delegates the license decision to [`sindri_policy::check::check_license`]
    /// — there is exactly one canonical implementation of that rule and it
    /// lives in `sindri-policy`. Preserves the `ADM_LICENSE_DENIED` code so
    /// downstream callers don't break.
    pub fn check_policy(&self, candidate: &CandidateRef<'_>) -> AdmissionResult {
        let result = check_license(&candidate.entry.license, self.policy);
        if !result.allowed {
            return AdmissionResult {
                allowed: false,
                code: result.code,
                message: result.message,
                suggested_fix: result.fix,
                skipped: false,
            };
        }
        AdmissionResult::ok()
    }

    /// Gate 3 — Dependency closure.
    ///
    /// Walks every candidate and runs gates 1, 2, and 4 against it. Returns
    /// the first hard denial; skipped gates are tolerated.
    pub fn check_closure(&self, candidates: &[CandidateRef<'_>]) -> AdmissionResult {
        for c in candidates {
            let r = self.check_platform(c);
            if !r.allowed {
                return r;
            }
            let r = self.check_policy(c);
            if !r.allowed {
                return r;
            }
            let r = self.check_capability_trust(c);
            if !r.allowed {
                return r;
            }
        }
        AdmissionResult::ok()
    }

    /// Gate 4 — Capability trust (ADR-008 §Gate 4).
    ///
    /// Rules:
    /// - If the manifest declares `capabilities.collision_handling.path_prefix`,
    ///   the prefix's first path segment must equal the component's metadata
    ///   name (or the registry-entry name when the manifest is absent).
    /// - The literal sentinel `:shared` is only permitted when the candidate
    ///   is sourced from [`CORE_REGISTRY_NAME`] (`"sindri/core"`).
    /// - No `collision_handling` declared → no constraint, returns `ok`.
    ///
    /// Without a manifest the prefix cannot be inspected at all, so the gate
    /// is skipped with a debug log.
    pub fn check_capability_trust(&self, candidate: &CandidateRef<'_>) -> AdmissionResult {
        let Some(manifest) = candidate.manifest else {
            tracing::debug!(
                component = %candidate.entry.name,
                "capability-trust check skipped: manifest not yet fetched"
            );
            return AdmissionResult::skipped(
                "ADM_CAPABILITY_TRUST_SKIPPED",
                "Manifest not yet fetched; capability-trust gate deferred",
            );
        };

        let Some(coll) = manifest.capabilities.collision_handling.as_ref() else {
            // No collision-handling declared, nothing to enforce.
            return AdmissionResult::ok();
        };

        let prefix = coll.path_prefix.trim();
        let component_name = manifest.metadata.name.as_str();

        // The :shared escape hatch: only the core registry can claim it.
        if prefix == SHARED_PATH_PREFIX {
            if candidate.registry_name == CORE_REGISTRY_NAME {
                return AdmissionResult::ok();
            }
            return AdmissionResult::deny(
                "ADM_CAPABILITY_TRUST_VIOLATION",
                &format!(
                    "Component `{}` from registry `{}` declares the `:shared` \
                     collision-handling prefix, which is reserved for `{}`",
                    component_name, candidate.registry_name, CORE_REGISTRY_NAME,
                ),
                Some(
                    "Replace `:shared` with `{component-name}/...` or publish via the core registry",
                ),
            );
        }

        // Otherwise the first path segment must equal the component's name.
        // Strip a leading slash, then take the first segment.
        let normalized = prefix.trim_start_matches('/');
        let first_segment = normalized.split('/').next().unwrap_or("");

        if first_segment == component_name {
            AdmissionResult::ok()
        } else {
            AdmissionResult::deny(
                "ADM_CAPABILITY_TRUST_VIOLATION",
                &format!(
                    "Component `{}` declares collision-handling prefix `{}` whose \
                     first segment must equal the component name `{}`",
                    component_name, prefix, component_name,
                ),
                Some(
                    "Use a path of the form `{component-name}/...` (or publish via the core registry to use `:shared`)",
                ),
            )
        }
    }

    /// Run gates 1–4 against every candidate, failing on the first denial.
    ///
    /// Skipped gates (e.g. when the manifest hasn't been fetched yet) are
    /// non-fatal and do not abort admission.
    pub fn admit_all(&self, candidates: &[CandidateRef<'_>]) -> Result<(), ResolverError> {
        for c in candidates {
            for result in [
                self.check_platform(c),
                self.check_policy(c),
                self.check_capability_trust(c),
            ] {
                if !result.allowed {
                    return Err(ResolverError::AdmissionDenied {
                        code: result.code,
                        message: result.message,
                    });
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sindri_core::component::{
        CollisionHandlingConfig, ComponentCapabilities, ComponentManifest, ComponentMetadata,
        InstallConfig,
    };
    use sindri_core::platform::{Arch, Capabilities, Os, Platform, TargetProfile};
    use sindri_core::policy::{InstallPolicy, PolicyPreset};
    use sindri_core::registry::{ComponentEntry, ComponentKind};

    fn target(os: Os, arch: Arch) -> TargetProfile {
        TargetProfile {
            platform: Platform { os, arch },
            capabilities: Capabilities::default(),
        }
    }

    fn permissive_policy() -> InstallPolicy {
        InstallPolicy {
            preset: PolicyPreset::Default,
            allowed_licenses: vec![],
            denied_licenses: vec![],
            on_unknown_license: None,
            require_signed_registries: None,
            require_checksums: None,
            offline: None,
            audit: None,
            auth: sindri_core::policy::AuthPolicy::default(),
        }
    }

    fn strict_policy() -> InstallPolicy {
        InstallPolicy {
            preset: PolicyPreset::Strict,
            allowed_licenses: vec!["MIT".into(), "Apache-2.0".into()],
            denied_licenses: vec![],
            on_unknown_license: None,
            require_signed_registries: Some(true),
            require_checksums: Some(true),
            offline: None,
            audit: None,
            auth: sindri_core::policy::AuthPolicy::default(),
        }
    }

    fn entry(name: &str, license: &str) -> ComponentEntry {
        ComponentEntry {
            name: name.into(),
            backend: "binary".into(),
            latest: "1.0.0".into(),
            versions: vec!["1.0.0".into()],
            description: "test".into(),
            kind: ComponentKind::Component,
            oci_ref: format!("registry.example.com/{}@sha256:deadbeef", name),
            license: license.into(),
            depends_on: vec![],
        }
    }

    fn manifest(name: &str, platforms: Vec<Platform>, prefix: Option<&str>) -> ComponentManifest {
        ComponentManifest {
            metadata: ComponentMetadata {
                name: name.into(),
                version: "1.0.0".into(),
                description: "test".into(),
                license: "MIT".into(),
                tags: vec![],
                homepage: None,
            },
            platforms,
            install: InstallConfig::default(),
            depends_on: vec![],
            capabilities: ComponentCapabilities {
                collision_handling: prefix.map(|p| CollisionHandlingConfig {
                    path_prefix: p.into(),
                }),
                hooks: None,
                project_init: None,
            },
            options: Default::default(),
            validate: None,
            configure: None,
            remove: None,
            overrides: Default::default(),
            auth: Default::default(),
        }
    }

    // ---- Gate 1 ----

    #[test]
    fn gate1_denies_when_platform_not_supported() {
        let policy = permissive_policy();
        let target = target(Os::Macos, Arch::Aarch64);
        let checker = AdmissionChecker::new(&policy, &target);

        let e = entry("foo", "MIT");
        let m = manifest(
            "foo",
            vec![Platform {
                os: Os::Linux,
                arch: Arch::X86_64,
            }],
            None,
        );
        let cand = CandidateRef::with_manifest(&e, &m, "third/party");

        let r = checker.check_platform(&cand);
        assert!(!r.allowed, "expected deny, got {:?}", r);
        assert_eq!(r.code, "ADM_PLATFORM_UNSUPPORTED");
    }

    #[test]
    fn gate1_allows_when_platform_supported() {
        let policy = permissive_policy();
        let target = target(Os::Linux, Arch::X86_64);
        let checker = AdmissionChecker::new(&policy, &target);

        let e = entry("foo", "MIT");
        let m = manifest(
            "foo",
            vec![Platform {
                os: Os::Linux,
                arch: Arch::X86_64,
            }],
            None,
        );
        let cand = CandidateRef::with_manifest(&e, &m, "third/party");

        let r = checker.check_platform(&cand);
        assert!(r.allowed, "expected ok, got {:?}", r);
        assert!(!r.skipped);
    }

    #[test]
    fn gate1_allows_when_platforms_empty() {
        let policy = permissive_policy();
        let target = target(Os::Linux, Arch::X86_64);
        let checker = AdmissionChecker::new(&policy, &target);

        let e = entry("foo", "MIT");
        let m = manifest("foo", vec![], None);
        let cand = CandidateRef::with_manifest(&e, &m, "third/party");

        assert!(checker.check_platform(&cand).allowed);
    }

    #[test]
    fn gate1_skipped_when_manifest_absent() {
        let policy = permissive_policy();
        let target = target(Os::Linux, Arch::X86_64);
        let checker = AdmissionChecker::new(&policy, &target);

        let e = entry("foo", "MIT");
        let cand = CandidateRef::from_entry(&e, "third/party");

        let r = checker.check_platform(&cand);
        assert!(r.allowed, "skipped should be non-fatal");
        assert!(r.skipped, "expected skipped=true, got {:?}", r);
        assert_eq!(r.code, "ADM_PLATFORM_SKIPPED");
    }

    // ---- Gate 4 ----

    #[test]
    fn gate4_denies_path_prefix_pointing_at_etc() {
        let policy = permissive_policy();
        let target = target(Os::Linux, Arch::X86_64);
        let checker = AdmissionChecker::new(&policy, &target);

        let e = entry("nodejs", "MIT");
        let m = manifest("nodejs", vec![], Some("/etc/foo"));
        let cand = CandidateRef::with_manifest(&e, &m, "third/party");

        let r = checker.check_capability_trust(&cand);
        assert!(!r.allowed, "expected deny, got {:?}", r);
        assert_eq!(r.code, "ADM_CAPABILITY_TRUST_VIOLATION");
    }

    #[test]
    fn gate4_allows_path_prefix_matching_component_name() {
        let policy = permissive_policy();
        let target = target(Os::Linux, Arch::X86_64);
        let checker = AdmissionChecker::new(&policy, &target);

        let e = entry("nodejs", "MIT");
        let m = manifest("nodejs", vec![], Some("nodejs/conf"));
        let cand = CandidateRef::with_manifest(&e, &m, "third/party");

        let r = checker.check_capability_trust(&cand);
        assert!(r.allowed, "expected ok, got {:?}", r);
    }

    #[test]
    fn gate4_allows_shared_only_for_sindri_core() {
        let policy = permissive_policy();
        let target = target(Os::Linux, Arch::X86_64);
        let checker = AdmissionChecker::new(&policy, &target);

        let e = entry("nodejs", "MIT");
        let m = manifest("nodejs", vec![], Some(":shared"));

        // Allowed when sourced from sindri/core.
        let core_cand = CandidateRef::with_manifest(&e, &m, CORE_REGISTRY_NAME);
        assert!(checker.check_capability_trust(&core_cand).allowed);

        // Denied for any other registry.
        let other_cand = CandidateRef::with_manifest(&e, &m, "third/party");
        let r = checker.check_capability_trust(&other_cand);
        assert!(!r.allowed);
        assert_eq!(r.code, "ADM_CAPABILITY_TRUST_VIOLATION");
    }

    #[test]
    fn gate4_skipped_when_manifest_absent() {
        let policy = permissive_policy();
        let target = target(Os::Linux, Arch::X86_64);
        let checker = AdmissionChecker::new(&policy, &target);

        let e = entry("nodejs", "MIT");
        let cand = CandidateRef::from_entry(&e, "third/party");

        let r = checker.check_capability_trust(&cand);
        assert!(r.allowed && r.skipped);
        assert_eq!(r.code, "ADM_CAPABILITY_TRUST_SKIPPED");
    }

    #[test]
    fn gate4_allows_when_no_collision_handling() {
        let policy = permissive_policy();
        let target = target(Os::Linux, Arch::X86_64);
        let checker = AdmissionChecker::new(&policy, &target);

        let e = entry("nodejs", "MIT");
        let m = manifest("nodejs", vec![], None);
        let cand = CandidateRef::with_manifest(&e, &m, "third/party");

        assert!(checker.check_capability_trust(&cand).allowed);
    }

    // ---- Gate 2: license dedup ----

    #[test]
    fn license_dedup_uses_policy_crate() {
        let policy = strict_policy();
        let target = target(Os::Linux, Arch::X86_64);
        let checker = AdmissionChecker::new(&policy, &target);

        // Strict + GPL-3.0 → deny via sindri-policy.
        let e = entry("foo", "GPL-3.0-only");
        let cand = CandidateRef::from_entry(&e, "third/party");
        let r = checker.check_policy(&cand);
        assert!(!r.allowed);
        assert_eq!(r.code, "ADM_LICENSE_DENIED");

        // Behaviour matches sindri-policy for several inputs.
        for license in ["MIT", "Apache-2.0", "GPL-3.0-only", "proprietary"] {
            let e = entry("foo", license);
            let cand = CandidateRef::from_entry(&e, "third/party");
            let admission = checker.check_policy(&cand);
            let policy_result = check_license(license, &policy);
            assert_eq!(
                admission.allowed, policy_result.allowed,
                "license `{}` mismatch between admission and policy crate",
                license,
            );
        }

        // Source check: no in-resolver string-match GPL/AGPL branch remains.
        let admission_src = include_str!("admission.rs");
        assert!(
            !admission_src.contains("contains(\"GPL\")"),
            "duplicate license logic must not be reintroduced into admission.rs",
        );
        assert!(
            !admission_src.contains("contains(\"AGPL\")"),
            "duplicate license logic must not be reintroduced into admission.rs",
        );
    }
}
