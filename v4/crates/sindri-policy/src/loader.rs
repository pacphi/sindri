use sindri_core::policy::{
    ApiVersion, AuditPolicy, AuthPolicy, CapabilitiesPolicy, InstallPolicy, LicensePolicy,
    NetworkPolicy, PolicyAction, PolicyKind, PolicyPreset, RegistryPolicy, SourcesPolicy,
};
use std::fs;
use std::path::{Path, PathBuf};

/// Source annotation for a policy setting.
#[derive(Debug, Clone)]
pub struct PolicySource {
    pub file: String,
}

/// Loaded and merged effective policy, with source tracking.
#[derive(Debug)]
pub struct EffectivePolicy {
    pub policy: InstallPolicy,
    pub sources: Vec<(String, PolicySource)>,
}

// =============================================================================
// Presets (ADR-008).
// =============================================================================

pub fn preset_default() -> InstallPolicy {
    InstallPolicy {
        api_version: ApiVersion::V4,
        kind: PolicyKind::InstallPolicy,
        preset: PolicyPreset::Default,
        licenses: LicensePolicy {
            allow: Vec::new(),
            deny: Vec::new(),
            on_unknown: Some(PolicyAction::Warn),
        },
        registries: RegistryPolicy {
            require_signed: Some(false),
            trust: Vec::new(),
        },
        sources: SourcesPolicy {
            require_checksums: Some(false),
            require_pinned_versions: Some(false),
            allow_script_backend: Some(PolicyAction::Allow),
            allow_privileged: Some(PolicyAction::Allow),
        },
        network: NetworkPolicy {
            offline: Some(false),
        },
        capabilities: CapabilitiesPolicy::default(),
        audit: AuditPolicy::default(),
        auth: AuthPolicy::default(),
    }
}

pub fn preset_strict() -> InstallPolicy {
    InstallPolicy {
        api_version: ApiVersion::V4,
        kind: PolicyKind::InstallPolicy,
        preset: PolicyPreset::Strict,
        licenses: LicensePolicy {
            allow: vec![
                "MIT".into(),
                "Apache-2.0".into(),
                "BSD-2-Clause".into(),
                "BSD-3-Clause".into(),
                "ISC".into(),
                "MPL-2.0".into(),
            ],
            deny: vec!["GPL-3.0-only".into(), "AGPL-3.0-only".into()],
            on_unknown: Some(PolicyAction::Deny),
        },
        registries: RegistryPolicy {
            require_signed: Some(true),
            trust: Vec::new(),
        },
        sources: SourcesPolicy {
            require_checksums: Some(true),
            require_pinned_versions: Some(true),
            allow_script_backend: Some(PolicyAction::Prompt),
            allow_privileged: Some(PolicyAction::Prompt),
        },
        network: NetworkPolicy {
            offline: Some(false),
        },
        capabilities: CapabilitiesPolicy::default(),
        audit: AuditPolicy {
            require_justification: true,
        },
        auth: AuthPolicy::default(),
    }
}

pub fn preset_offline() -> InstallPolicy {
    InstallPolicy {
        api_version: ApiVersion::V4,
        kind: PolicyKind::InstallPolicy,
        preset: PolicyPreset::Offline,
        licenses: LicensePolicy {
            allow: Vec::new(),
            deny: Vec::new(),
            on_unknown: Some(PolicyAction::Allow),
        },
        registries: RegistryPolicy {
            require_signed: Some(false),
            trust: Vec::new(),
        },
        sources: SourcesPolicy {
            require_checksums: Some(false),
            require_pinned_versions: Some(false),
            allow_script_backend: Some(PolicyAction::Allow),
            allow_privileged: Some(PolicyAction::Allow),
        },
        network: NetworkPolicy {
            offline: Some(true),
        },
        capabilities: CapabilitiesPolicy::default(),
        audit: AuditPolicy::default(),
        auth: AuthPolicy::default(),
    }
}

// =============================================================================
// Load + merge.
// =============================================================================

/// Load policy from disk, merging global + project policy files.
/// Precedence: project (./sindri.policy.yaml) overrides global (~/.sindri/policy.yaml).
pub fn load_effective_policy() -> EffectivePolicy {
    let mut policy = preset_default();
    let mut sources = Vec::new();

    // Global policy.
    let global_path = global_policy_path();
    if global_path.exists() {
        if let Ok(content) = fs::read_to_string(&global_path) {
            if let Ok(global) = serde_yaml::from_str::<InstallPolicy>(&content) {
                merge_policy(&mut policy, &global);
                sources.push((
                    format!("preset: {}", preset_name(&policy.preset)),
                    PolicySource {
                        file: global_path.to_string_lossy().to_string(),
                    },
                ));
            }
        }
    }

    // Project policy (overrides global).
    let project_path = Path::new("sindri.policy.yaml");
    if project_path.exists() {
        if let Ok(content) = fs::read_to_string(project_path) {
            if let Ok(project) = serde_yaml::from_str::<InstallPolicy>(&content) {
                merge_policy(&mut policy, &project);
                sources.push((
                    "project policy".to_string(),
                    PolicySource {
                        file: "sindri.policy.yaml".to_string(),
                    },
                ));
            }
        }
    }

    EffectivePolicy { policy, sources }
}

/// Write a preset to the global policy file (`~/.sindri/policy.yaml`).
pub fn write_global_preset(preset: &PolicyPreset) -> Result<(), std::io::Error> {
    let policy = preset_to_policy(preset);
    let path = global_policy_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let yaml = serde_yaml::to_string(&policy).map_err(|e| std::io::Error::other(e.to_string()))?;
    fs::write(&path, yaml)
}

/// Write a preset to a project-scoped policy file (default `./sindri.policy.yaml`).
/// Phase 4 wires this in from `sindri init` and `sindri policy use`.
pub fn write_project_preset(preset: &PolicyPreset, path: &Path) -> Result<(), std::io::Error> {
    let policy = preset_to_policy(preset);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let yaml = serde_yaml::to_string(&policy).map_err(|e| std::io::Error::other(e.to_string()))?;
    fs::write(path, yaml)
}

fn preset_to_policy(preset: &PolicyPreset) -> InstallPolicy {
    match preset {
        PolicyPreset::Default => preset_default(),
        PolicyPreset::Strict => preset_strict(),
        PolicyPreset::Offline => preset_offline(),
    }
}

/// Merge `overlay` onto `base`. Field-wise nested merge: any sub-policy field
/// set on the overlay (i.e. non-default) overrides its base counterpart.
pub fn merge_policy(base: &mut InstallPolicy, overlay: &InstallPolicy) {
    // apiVersion / kind: fixed singletons, no merge.
    base.preset = overlay.preset.clone();

    // Licenses.
    if !overlay.licenses.allow.is_empty() {
        base.licenses.allow = overlay.licenses.allow.clone();
    }
    if !overlay.licenses.deny.is_empty() {
        base.licenses.deny = overlay.licenses.deny.clone();
    }
    if overlay.licenses.on_unknown.is_some() {
        base.licenses.on_unknown = overlay.licenses.on_unknown.clone();
    }

    // Registries.
    if overlay.registries.require_signed.is_some() {
        base.registries.require_signed = overlay.registries.require_signed;
    }
    if !overlay.registries.trust.is_empty() {
        base.registries.trust = overlay.registries.trust.clone();
    }

    // Sources.
    if overlay.sources.require_checksums.is_some() {
        base.sources.require_checksums = overlay.sources.require_checksums;
    }
    if overlay.sources.require_pinned_versions.is_some() {
        base.sources.require_pinned_versions = overlay.sources.require_pinned_versions;
    }
    if overlay.sources.allow_script_backend.is_some() {
        base.sources.allow_script_backend = overlay.sources.allow_script_backend.clone();
    }
    if overlay.sources.allow_privileged.is_some() {
        base.sources.allow_privileged = overlay.sources.allow_privileged.clone();
    }

    // Network.
    if overlay.network.offline.is_some() {
        base.network.offline = overlay.network.offline;
    }

    // Capabilities — Phase 1 ships data-only; Phase 2 wires Gate 4. Replace
    // wholesale on overlay (consistent with merge_policy's "non-default
    // overlay wins" idiom for nested groups).
    let cap_default = sindri_core::policy::CapabilitiesPolicy::default();
    if !is_capabilities_default(&overlay.capabilities, &cap_default) {
        base.capabilities = overlay.capabilities.clone();
    }

    // Audit.
    if overlay.audit.require_justification {
        base.audit.require_justification = true;
    }

    // Auth: overlay always wins. Defaults are documented as strict-deny so
    // accidental omission cannot relax them.
    base.auth = overlay.auth.clone();
}

fn is_capabilities_default(
    a: &sindri_core::policy::CapabilitiesPolicy,
    default: &sindri_core::policy::CapabilitiesPolicy,
) -> bool {
    use sindri_core::policy::TrustList;
    fn list_eq(a: &TrustList, b: &TrustList) -> bool {
        match (a, b) {
            (TrustList::Allowed(la), TrustList::Allowed(lb)) => la == lb,
            (TrustList::Wildcard(_), TrustList::Wildcard(_)) => true,
            _ => false,
        }
    }
    list_eq(
        &a.trust_sources.collision_handling,
        &default.trust_sources.collision_handling,
    ) && list_eq(
        &a.trust_sources.project_init,
        &default.trust_sources.project_init,
    ) && list_eq(
        &a.trust_sources.mcp_registration,
        &default.trust_sources.mcp_registration,
    ) && list_eq(
        &a.trust_sources.shell_rc_edits,
        &default.trust_sources.shell_rc_edits,
    )
}

pub fn global_policy_path() -> PathBuf {
    sindri_core::paths::home_dir()
        .unwrap_or_default()
        .join(".sindri")
        .join("policy.yaml")
}

fn preset_name(p: &PolicyPreset) -> &'static str {
    match p {
        PolicyPreset::Default => "default",
        PolicyPreset::Strict => "strict",
        PolicyPreset::Offline => "offline",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_preset_denies_gpl() {
        let policy = preset_strict();
        assert!(policy.licenses.deny.contains(&"GPL-3.0-only".to_string()));
    }

    #[test]
    fn strict_preset_requires_signed_registries() {
        let policy = preset_strict();
        assert!(policy.requires_signed_registries());
    }

    #[test]
    fn strict_preset_requires_pinned_versions() {
        let policy = preset_strict();
        assert!(policy.requires_pinned_versions());
    }

    #[test]
    fn offline_preset_sets_offline_flag() {
        let policy = preset_offline();
        assert!(policy.is_offline());
    }

    #[test]
    fn default_preset_warns_on_unknown() {
        let policy = preset_default();
        assert!(matches!(
            policy.licenses.on_unknown,
            Some(PolicyAction::Warn)
        ));
    }

    #[test]
    fn merge_overrides_license_lists() {
        let mut base = preset_default();
        let mut overlay = InstallPolicy::default();
        overlay.licenses.allow = vec!["MIT".into()];
        merge_policy(&mut base, &overlay);
        assert_eq!(base.licenses.allow, vec!["MIT".to_string()]);
    }

    #[test]
    fn merge_overrides_registries_trust() {
        let mut base = preset_default();
        let mut overlay = InstallPolicy::default();
        overlay.registries.trust = vec!["sindri/core".into()];
        merge_policy(&mut base, &overlay);
        assert_eq!(base.registries.trust, vec!["sindri/core".to_string()]);
    }

    #[test]
    fn merge_does_not_clobber_unset_overlay_fields() {
        let mut base = preset_strict();
        let overlay = InstallPolicy::default(); // empty overlay
        merge_policy(&mut base, &overlay);
        // Strict preset's allow list survives an empty overlay.
        assert!(base.licenses.allow.contains(&"MIT".to_string()));
        assert!(base.requires_signed_registries());
    }

    #[test]
    fn write_project_preset_creates_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("sindri.policy.yaml");
        write_project_preset(&PolicyPreset::Strict, &path).expect("write");
        let content = fs::read_to_string(&path).expect("read");
        // Round-trip parse to confirm the file is valid YAML for InstallPolicy.
        let parsed: InstallPolicy = serde_yaml::from_str(&content).expect("parse");
        assert_eq!(parsed.preset, PolicyPreset::Strict);
    }
}
