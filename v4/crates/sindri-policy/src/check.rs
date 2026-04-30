use crate::admission_codes::{
    ADM_CHECKSUM_MISSING, ADM_LICENSE_DENIED, ADM_LICENSE_UNKNOWN, ADM_PRIVILEGED_DENIED,
    ADM_SCRIPT_DENIED, ADM_VERSION_NOT_PINNED,
};
use sindri_core::policy::{InstallPolicy, PolicyAction, PolicyPreset};
use sindri_core::registry::ComponentEntry;

/// Result of a policy check.
#[derive(Debug)]
pub struct PolicyCheckResult {
    pub allowed: bool,
    pub code: String,
    pub message: String,
    pub fix: Option<String>,
}

impl PolicyCheckResult {
    pub fn ok() -> Self {
        PolicyCheckResult {
            allowed: true,
            code: "OK".into(),
            message: "Allowed".into(),
            fix: None,
        }
    }

    pub fn deny(code: &str, msg: &str, fix: Option<&str>) -> Self {
        PolicyCheckResult {
            allowed: false,
            code: code.into(),
            message: msg.into(),
            fix: fix.map(|s| s.to_string()),
        }
    }
}

// =============================================================================
// License checks.
// =============================================================================

/// Check a component's license against the current policy.
pub fn check_license(license: &str, policy: &InstallPolicy) -> PolicyCheckResult {
    // Explicit deny list always wins.
    if policy.licenses.deny.iter().any(|d| d == license) {
        return PolicyCheckResult::deny(
            ADM_LICENSE_DENIED,
            &format!("License `{}` is explicitly denied by policy", license),
            Some("Remove this license from policy.licenses.deny or use `sindri policy allow-license`"),
        );
    }

    // Strict preset: only allow-listed licenses pass.
    if matches!(policy.preset, PolicyPreset::Strict)
        && !policy.licenses.allow.is_empty()
        && !policy.licenses.allow.iter().any(|a| a == license)
        && !license.trim().is_empty()
    {
        return PolicyCheckResult::deny(
            ADM_LICENSE_DENIED,
            &format!("License `{}` not in strict-mode allow list", license),
            Some("Run `sindri policy allow-license <spdx>` to add it"),
        );
    }

    // Unknown license handling.
    if license.trim().is_empty() {
        match policy.unknown_license_action() {
            PolicyAction::Deny => {
                return PolicyCheckResult::deny(
                    ADM_LICENSE_UNKNOWN,
                    "No license specified; denied by policy",
                    Some("Add `license:` to the component manifest"),
                );
            }
            PolicyAction::Warn | PolicyAction::Allow | PolicyAction::Prompt => {
                // non-blocking
            }
        }
    }

    PolicyCheckResult::ok()
}

/// Check all components in a closure — returns first failure or Ok.
pub fn check_closure(entries: &[&ComponentEntry], policy: &InstallPolicy) -> PolicyCheckResult {
    for entry in entries {
        let result = check_license(&entry.license, policy);
        if !result.allowed {
            return result;
        }
    }
    PolicyCheckResult::ok()
}

// =============================================================================
// Source-quality checks (sources.* knobs).
// =============================================================================

/// Reject if `policy.sources.requirePinnedVersions: true` and the requested
/// version is not an exact pin (`@version`).
pub fn check_pinned_version(
    component_address: &str,
    version_spec: Option<&str>,
    policy: &InstallPolicy,
) -> PolicyCheckResult {
    if !policy.requires_pinned_versions() {
        return PolicyCheckResult::ok();
    }
    let pin = version_spec.unwrap_or("").trim();
    let is_exact = !pin.is_empty()
        && pin != "latest"
        && pin != "*"
        && !pin.starts_with('^')
        && !pin.starts_with('~')
        && !pin.starts_with('>')
        && !pin.starts_with('<');
    if is_exact {
        PolicyCheckResult::ok()
    } else {
        PolicyCheckResult::deny(
            ADM_VERSION_NOT_PINNED,
            &format!(
                "Component `{}` requires an exact version pin (policy.sources.requirePinnedVersions=true)",
                component_address
            ),
            Some("Pin the component with `sindri pin <address> <version>`"),
        )
    }
}

/// Apply the configured action for the `script` backend.
/// Returns `Ok` for `Allow`, `Warn`, `Prompt` (advisory in CI) and a deny for `Deny`.
pub fn check_script_backend(
    component_address: &str,
    backend: &str,
    policy: &InstallPolicy,
) -> PolicyCheckResult {
    if backend != "script" {
        return PolicyCheckResult::ok();
    }
    match policy.script_backend_action() {
        PolicyAction::Allow | PolicyAction::Warn | PolicyAction::Prompt => PolicyCheckResult::ok(),
        PolicyAction::Deny => PolicyCheckResult::deny(
            ADM_SCRIPT_DENIED,
            &format!(
                "Component `{}` uses the `script` backend; policy.sources.allowScriptBackend=deny",
                component_address
            ),
            Some("Switch the component to a typed backend or relax the policy"),
        ),
    }
}

/// Apply the configured action for components that require elevation.
/// Components signal this via the `requiresElevation` field on their manifest
/// (Phase 1 deserialises the field on `Component`; default false means no-op).
pub fn check_privileged(
    component_address: &str,
    requires_elevation: bool,
    policy: &InstallPolicy,
) -> PolicyCheckResult {
    if !requires_elevation {
        return PolicyCheckResult::ok();
    }
    match policy.privileged_action() {
        PolicyAction::Allow | PolicyAction::Warn | PolicyAction::Prompt => PolicyCheckResult::ok(),
        PolicyAction::Deny => PolicyCheckResult::deny(
            ADM_PRIVILEGED_DENIED,
            &format!(
                "Component `{}` requires elevation; policy.sources.allowPrivileged=deny",
                component_address
            ),
            Some("Run on a target where elevation is acceptable, or relax the policy"),
        ),
    }
}

/// Reject if `policy.sources.requireChecksums: true` and the entry lacks any
/// checksum metadata. Phase 1 checks the `checksums` blob on the entry's
/// install config (binary backend only); other backends are unaffected.
pub fn check_checksums(
    component_address: &str,
    has_checksums: bool,
    policy: &InstallPolicy,
) -> PolicyCheckResult {
    if !policy.requires_checksums() {
        return PolicyCheckResult::ok();
    }
    if has_checksums {
        PolicyCheckResult::ok()
    } else {
        PolicyCheckResult::deny(
            ADM_CHECKSUM_MISSING,
            &format!(
                "Component `{}` has no asset checksums; policy.sources.requireChecksums=true",
                component_address
            ),
            Some("Run `sindri registry fetch-checksums <path>` and republish"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::{preset_default, preset_strict};

    #[test]
    fn strict_denies_gpl() {
        let policy = preset_strict();
        let result = check_license("GPL-3.0-only", &policy);
        assert!(!result.allowed);
        assert_eq!(result.code, ADM_LICENSE_DENIED);
    }

    #[test]
    fn strict_allows_mit() {
        let policy = preset_strict();
        let result = check_license("MIT", &policy);
        assert!(result.allowed);
    }

    #[test]
    fn default_allows_gpl() {
        let policy = preset_default();
        let result = check_license("GPL-3.0-only", &policy);
        assert!(result.allowed);
    }

    #[test]
    fn strict_denies_unknown_license_via_default_action() {
        // Strict preset's on_unknown is Deny by preset construction.
        let policy = preset_strict();
        let result = check_license("", &policy);
        assert!(!result.allowed);
        assert_eq!(result.code, ADM_LICENSE_UNKNOWN);
    }

    #[test]
    fn pinned_version_required_blocks_latest() {
        let policy = preset_strict();
        let r = check_pinned_version("npm:foo", Some("latest"), &policy);
        assert!(!r.allowed);
        assert_eq!(r.code, ADM_VERSION_NOT_PINNED);
    }

    #[test]
    fn pinned_version_required_blocks_caret() {
        let policy = preset_strict();
        let r = check_pinned_version("npm:foo", Some("^1.2.3"), &policy);
        assert!(!r.allowed);
    }

    #[test]
    fn pinned_version_required_admits_exact() {
        let policy = preset_strict();
        let r = check_pinned_version("npm:foo", Some("1.2.3"), &policy);
        assert!(r.allowed);
    }

    #[test]
    fn pinned_version_default_preset_allows_latest() {
        let policy = preset_default();
        let r = check_pinned_version("npm:foo", Some("latest"), &policy);
        assert!(r.allowed);
    }

    #[test]
    fn script_backend_strict_prompt_admits() {
        let policy = preset_strict();
        let r = check_script_backend("script:foo", "script", &policy);
        assert!(r.allowed);
    }

    #[test]
    fn script_backend_deny_blocks() {
        let mut policy = preset_strict();
        policy.sources.allow_script_backend = Some(PolicyAction::Deny);
        let r = check_script_backend("script:foo", "script", &policy);
        assert!(!r.allowed);
        assert_eq!(r.code, ADM_SCRIPT_DENIED);
    }

    #[test]
    fn script_backend_check_ignores_other_backends() {
        let mut policy = preset_strict();
        policy.sources.allow_script_backend = Some(PolicyAction::Deny);
        let r = check_script_backend("npm:foo", "npm", &policy);
        assert!(r.allowed);
    }

    #[test]
    fn privileged_default_admits() {
        let policy = preset_default();
        let r = check_privileged("apt:docker", true, &policy);
        assert!(r.allowed);
    }

    #[test]
    fn privileged_deny_blocks() {
        let mut policy = preset_default();
        policy.sources.allow_privileged = Some(PolicyAction::Deny);
        let r = check_privileged("apt:docker", true, &policy);
        assert!(!r.allowed);
        assert_eq!(r.code, ADM_PRIVILEGED_DENIED);
    }

    #[test]
    fn privileged_non_elevating_components_pass() {
        let mut policy = preset_default();
        policy.sources.allow_privileged = Some(PolicyAction::Deny);
        let r = check_privileged("npm:foo", false, &policy);
        assert!(r.allowed);
    }

    #[test]
    fn checksums_required_strict_blocks_missing() {
        let policy = preset_strict();
        let r = check_checksums("binary:gh", false, &policy);
        assert!(!r.allowed);
        assert_eq!(r.code, ADM_CHECKSUM_MISSING);
    }

    #[test]
    fn checksums_required_strict_admits_present() {
        let policy = preset_strict();
        let r = check_checksums("binary:gh", true, &policy);
        assert!(r.allowed);
    }
}
