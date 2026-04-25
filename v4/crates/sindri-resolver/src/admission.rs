use sindri_core::component::Backend;
use sindri_core::platform::TargetProfile;
use sindri_core::policy::{InstallPolicy, PolicyAction, PolicyPreset};
use sindri_core::registry::ComponentEntry;
use crate::error::ResolverError;

/// The four admission gates (ADR-008)
pub struct AdmissionChecker<'a> {
    pub policy: &'a InstallPolicy,
    pub target: &'a TargetProfile,
}

#[derive(Debug)]
pub struct AdmissionResult {
    pub allowed: bool,
    pub code: String,
    pub message: String,
    pub suggested_fix: Option<String>,
}

impl AdmissionResult {
    pub fn ok() -> Self {
        AdmissionResult {
            allowed: true,
            code: "OK".into(),
            message: "Admitted".into(),
            suggested_fix: None,
        }
    }

    pub fn deny(code: &str, message: &str, fix: Option<&str>) -> Self {
        AdmissionResult {
            allowed: false,
            code: code.into(),
            message: message.into(),
            suggested_fix: fix.map(|s| s.to_string()),
        }
    }
}

impl<'a> AdmissionChecker<'a> {
    pub fn new(policy: &'a InstallPolicy, target: &'a TargetProfile) -> Self {
        AdmissionChecker { policy, target }
    }

    /// Gate 1: Does the component support this platform?
    pub fn check_platform(&self, entry: &ComponentEntry) -> AdmissionResult {
        // For Sprint 3, all components with a non-empty depends_on set pass.
        // Full platform matrix check in Sprint 4 when ComponentManifest is fetched.
        AdmissionResult::ok()
    }

    /// Gate 2: Does the component pass the current policy (license, signing, scope)?
    pub fn check_policy(&self, entry: &ComponentEntry) -> AdmissionResult {
        // License check
        let license = &entry.license;
        if !license.is_empty() {
            // Strict preset: deny GPL unless explicitly allowed
            if matches!(self.policy.preset, PolicyPreset::Strict) {
                if (license.contains("GPL") || license.contains("AGPL"))
                    && !self.policy.allowed_licenses.iter().any(|l| l == license)
                {
                    return AdmissionResult::deny(
                        "ADM_LICENSE_DENIED",
                        &format!("License {} not permitted in strict policy", license),
                        Some("Use `sindri policy allow-license` or switch to default preset"),
                    );
                }
            }
            // Explicit denial list
            if self.policy.denied_licenses.iter().any(|l| l == license) {
                return AdmissionResult::deny(
                    "ADM_LICENSE_DENIED",
                    &format!("License {} is explicitly denied", license),
                    Some("Remove this license from the deny list or choose a different component"),
                );
            }
        }

        // Offline policy
        if self.policy.offline.unwrap_or(false) {
            // All components pass in offline mode — downloads prevented elsewhere
        }

        AdmissionResult::ok()
    }

    /// Gate 3: Does the entire closure pass platform + policy?
    pub fn check_closure(&self, entries: &[&ComponentEntry]) -> AdmissionResult {
        for entry in entries {
            let r = self.check_platform(entry);
            if !r.allowed {
                return r;
            }
            let r = self.check_policy(entry);
            if !r.allowed {
                return r;
            }
        }
        AdmissionResult::ok()
    }

    /// Gate 4: Capability trust — collision handling path prefix + registry trust (ADR-008)
    pub fn check_capability_trust(
        &self,
        _entry: &ComponentEntry,
        _registry_name: &str,
    ) -> AdmissionResult {
        // Full implementation in Sprint 6 (Policy subsystem)
        AdmissionResult::ok()
    }

    pub fn admit_all(&self, entries: &[&ComponentEntry]) -> Result<(), ResolverError> {
        for entry in entries {
            let result = self.check_policy(entry);
            if !result.allowed {
                return Err(ResolverError::AdmissionDenied {
                    code: result.code,
                    message: result.message,
                });
            }
        }
        Ok(())
    }
}
