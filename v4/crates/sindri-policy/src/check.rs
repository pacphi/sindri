use sindri_core::policy::{InstallPolicy, PolicyAction, PolicyPreset};
use sindri_core::registry::ComponentEntry;

/// Result of a policy check
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

/// Check a component's license against the current policy
pub fn check_license(license: &str, policy: &InstallPolicy) -> PolicyCheckResult {
    // Explicit deny list always wins
    if policy.denied_licenses.iter().any(|d| d == license) {
        return PolicyCheckResult::deny(
            "ADM_LICENSE_DENIED",
            &format!("License `{}` is explicitly denied by policy", license),
            Some("Remove this license from the deny list or use `sindri policy allow-license`"),
        );
    }

    // Strict preset: only allow-listed licenses pass
    if matches!(policy.preset, PolicyPreset::Strict)
        && !policy.allowed_licenses.is_empty()
        && !policy.allowed_licenses.iter().any(|a| a == license)
    {
        return PolicyCheckResult::deny(
            "ADM_LICENSE_DENIED",
            &format!("License `{}` not in strict-mode allow list", license),
            Some("Run `sindri policy allow-license <spdx>` to add it"),
        );
    }

    // Unknown license handling
    if license.trim().is_empty() {
        let action = policy
            .on_unknown_license
            .as_ref()
            .unwrap_or(&PolicyAction::Warn);
        match action {
            PolicyAction::Deny => {
                return PolicyCheckResult::deny(
                    "ADM_LICENSE_UNKNOWN",
                    "No license specified; denied by policy",
                    Some("Add `license:` to the component manifest"),
                );
            }
            PolicyAction::Warn => {
                // warn is non-blocking — allowed with a warning logged externally
            }
            _ => {}
        }
    }

    PolicyCheckResult::ok()
}

/// Check all components in a closure — returns first failure or Ok
pub fn check_closure(entries: &[&ComponentEntry], policy: &InstallPolicy) -> PolicyCheckResult {
    for entry in entries {
        let result = check_license(&entry.license, policy);
        if !result.allowed {
            return result;
        }
    }
    PolicyCheckResult::ok()
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
        assert_eq!(result.code, "ADM_LICENSE_DENIED");
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
}
