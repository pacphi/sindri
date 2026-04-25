use std::fs;
use std::path::{Path, PathBuf};
use sindri_core::policy::{AuditConfig, InstallPolicy, PolicyAction, PolicyPreset};

/// Source annotation for a policy setting
#[derive(Debug, Clone)]
pub struct PolicySource {
    pub file: String,
}

/// Loaded and merged effective policy, with source tracking
#[derive(Debug)]
pub struct EffectivePolicy {
    pub policy: InstallPolicy,
    pub sources: Vec<(String, PolicySource)>,
}

/// Policy presets (ADR-008)
pub fn preset_default() -> InstallPolicy {
    InstallPolicy {
        preset: PolicyPreset::Default,
        allowed_licenses: Vec::new(),
        denied_licenses: Vec::new(),
        on_unknown_license: Some(PolicyAction::Warn),
        require_signed_registries: Some(false),
        require_checksums: Some(false),
        offline: Some(false),
        audit: None,
    }
}

pub fn preset_strict() -> InstallPolicy {
    InstallPolicy {
        preset: PolicyPreset::Strict,
        allowed_licenses: vec![
            "MIT".into(), "Apache-2.0".into(), "BSD-2-Clause".into(),
            "BSD-3-Clause".into(), "ISC".into(), "MPL-2.0".into(),
        ],
        denied_licenses: vec!["GPL-3.0-only".into(), "AGPL-3.0-only".into()],
        on_unknown_license: Some(PolicyAction::Deny),
        require_signed_registries: Some(true),
        require_checksums: Some(true),
        offline: Some(false),
        audit: Some(AuditConfig { require_justification: true }),
    }
}

pub fn preset_offline() -> InstallPolicy {
    InstallPolicy {
        preset: PolicyPreset::Offline,
        allowed_licenses: Vec::new(),
        denied_licenses: Vec::new(),
        on_unknown_license: Some(PolicyAction::Allow),
        require_signed_registries: Some(false),
        require_checksums: Some(false),
        offline: Some(true),
        audit: None,
    }
}

/// Load policy from disk, merging global + project policy files.
/// Precedence: project (./sindri.policy.yaml) overrides global (~/.sindri/policy.yaml)
pub fn load_effective_policy() -> EffectivePolicy {
    let mut policy = preset_default();
    let mut sources = Vec::new();

    // Global policy
    let global_path = global_policy_path();
    if global_path.exists() {
        if let Ok(content) = fs::read_to_string(&global_path) {
            if let Ok(global) = serde_yaml::from_str::<InstallPolicy>(&content) {
                merge_policy(&mut policy, &global);
                sources.push((
                    format!("preset: {}", preset_name(&policy.preset)),
                    PolicySource { file: global_path.to_string_lossy().to_string() },
                ));
            }
        }
    }

    // Project policy (overrides global)
    let project_path = Path::new("sindri.policy.yaml");
    if project_path.exists() {
        if let Ok(content) = fs::read_to_string(project_path) {
            if let Ok(project) = serde_yaml::from_str::<InstallPolicy>(&content) {
                merge_policy(&mut policy, &project);
                sources.push((
                    "project policy".to_string(),
                    PolicySource { file: "sindri.policy.yaml".to_string() },
                ));
            }
        }
    }

    EffectivePolicy { policy, sources }
}

/// Write a preset to the global policy file
pub fn write_global_preset(preset: &PolicyPreset) -> Result<(), std::io::Error> {
    let policy = match preset {
        PolicyPreset::Default => preset_default(),
        PolicyPreset::Strict => preset_strict(),
        PolicyPreset::Offline => preset_offline(),
    };
    let path = global_policy_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let yaml = serde_yaml::to_string(&policy).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
    })?;
    fs::write(&path, yaml)
}

fn merge_policy(base: &mut InstallPolicy, overlay: &InstallPolicy) {
    // Only non-default fields from overlay override base
    base.preset = overlay.preset.clone();
    if !overlay.allowed_licenses.is_empty() {
        base.allowed_licenses = overlay.allowed_licenses.clone();
    }
    if !overlay.denied_licenses.is_empty() {
        base.denied_licenses = overlay.denied_licenses.clone();
    }
    if overlay.on_unknown_license.is_some() {
        base.on_unknown_license = overlay.on_unknown_license.clone();
    }
    if overlay.require_signed_registries.is_some() {
        base.require_signed_registries = overlay.require_signed_registries;
    }
    if overlay.require_checksums.is_some() {
        base.require_checksums = overlay.require_checksums;
    }
    if overlay.offline.is_some() {
        base.offline = overlay.offline;
    }
    if overlay.audit.is_some() {
        base.audit = overlay.audit.clone();
    }
}

pub fn global_policy_path() -> PathBuf {
    dirs_next::home_dir()
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
        assert!(policy.denied_licenses.contains(&"GPL-3.0-only".to_string()));
    }

    #[test]
    fn offline_preset_sets_offline_flag() {
        let policy = preset_offline();
        assert_eq!(policy.offline, Some(true));
    }

    #[test]
    fn default_preset_allows_unknown() {
        let policy = preset_default();
        assert!(matches!(policy.on_unknown_license, Some(PolicyAction::Warn)));
    }
}
