// ADR-008: Install policy as first-class subsystem
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PolicyPreset {
    #[default]
    Default,
    Strict,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct InstallPolicy {
    #[serde(default)]
    pub preset: PolicyPreset,
    #[serde(default)]
    pub allowed_licenses: Vec<String>,
    #[serde(default)]
    pub denied_licenses: Vec<String>,
    pub on_unknown_license: Option<PolicyAction>,
    pub require_signed_registries: Option<bool>,
    pub require_checksums: Option<bool>,
    pub offline: Option<bool>,
    pub audit: Option<AuditConfig>,
    /// Auth-aware admission knobs (Gate 5; ADR-027 §5).
    ///
    /// Additive; existing policy files without an `auth:` block deserialize
    /// as [`AuthPolicy::default()`] which is the **strict default-deny**
    /// posture user-approved for Phase 2B:
    /// - `on_unresolved_required: deny`
    /// - `allow_upstream_credentials: false`
    /// - `allow_prompt_in_ci: false`
    #[serde(default)]
    pub auth: AuthPolicy,
}

/// Auth-aware admission policy (ADR-027 §5).
///
/// All three knobs default to the **deny** stance. Operators must opt in
/// explicitly to relax any of them, and each opt-in is documented with a
/// security caveat in `v4/docs/policy.md`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct AuthPolicy {
    /// What to do when a non-`optional` requirement has no bound source
    /// in the lockfile. Default: `deny` (Gate 5 fails-fast at admission).
    /// Setting `warn` makes the gate purely advisory; `prompt` is reserved
    /// for Phase 5 interactive resolution.
    #[serde(default = "default_on_unresolved_required")]
    pub on_unresolved_required: PolicyAction,

    /// If `false` (default), bindings whose `AuthSource` is
    /// `FromUpstreamCredentials` are denied at Gate 5. Forces operators
    /// to mint dedicated child-workload credentials rather than reusing
    /// the target's session token.
    #[serde(default)]
    pub allow_upstream_credentials: bool,

    /// If `false` (default), bindings whose `AuthSource` is `Prompt` are
    /// denied at Gate 5 in non-interactive runs (no TTY OR `CI=1` /
    /// `SINDRI_CI=1` set).
    #[serde(default)]
    pub allow_prompt_in_ci: bool,
}

fn default_on_unresolved_required() -> PolicyAction {
    PolicyAction::Deny
}

impl Default for AuthPolicy {
    fn default() -> Self {
        AuthPolicy {
            on_unresolved_required: PolicyAction::Deny,
            allow_upstream_credentials: false,
            allow_prompt_in_ci: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum PolicyAction {
    Allow,
    Warn,
    Prompt,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AuditConfig {
    pub require_justification: bool,
}
