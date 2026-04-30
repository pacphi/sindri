// ADR-008: Install policy as first-class subsystem.
//
// v4 nested policy schema. External YAML keys are camelCase end-to-end;
// internal Rust field names stay snake_case via `#[serde(rename_all)]`.
// Every struct sets `deny_unknown_fields` so typos in user policy files
// surface as deserialization errors instead of being silently ignored.
//
// `apiVersion` and `kind` are validated against the canonical strings
// `sindri.dev/v4` and `InstallPolicy` (Q2 — Phase 1 reconciliation).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Canonical `apiVersion` string for v4 install-policy documents.
pub const POLICY_API_VERSION: &str = "sindri.dev/v4";

/// Canonical `kind` string for v4 install-policy documents.
pub const POLICY_KIND: &str = "InstallPolicy";

// =============================================================================
// `apiVersion` and `kind` enums (validated; single accepted value each).
// =============================================================================

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum ApiVersion {
    #[default]
    #[serde(rename = "sindri.dev/v4")]
    V4,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum PolicyKind {
    #[default]
    #[serde(rename = "InstallPolicy")]
    InstallPolicy,
}

// =============================================================================
// Preset and action enums (lowercase external).
// =============================================================================

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PolicyPreset {
    #[default]
    Default,
    Strict,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PolicyAction {
    Allow,
    Warn,
    Prompt,
    Deny,
}

// =============================================================================
// `InstallPolicy` — top-level aggregate.
// =============================================================================

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallPolicy {
    /// `sindri.dev/v4` — validated.
    #[serde(default)]
    pub api_version: ApiVersion,

    /// `InstallPolicy` — validated.
    #[serde(default)]
    pub kind: PolicyKind,

    /// One of `default`, `strict`, `offline`.
    #[serde(default)]
    pub preset: PolicyPreset,

    /// License allow/deny lists and unknown-license action.
    #[serde(default)]
    pub licenses: LicensePolicy,

    /// Registry signing requirements and trust list.
    #[serde(default)]
    pub registries: RegistryPolicy,

    /// Source-quality knobs (checksums, version pinning, script/privileged backends).
    #[serde(default)]
    pub sources: SourcesPolicy,

    /// Network-access knobs.
    #[serde(default)]
    pub network: NetworkPolicy,

    /// Capability-trust admission inputs (Gate 4; ADR-008).
    /// Phase 1 deserialises into typed data; Gate 4 enforcement lands in Phase 2.
    #[serde(default)]
    pub capabilities: CapabilitiesPolicy,

    /// Audit-trail enforcement.
    /// Phase 1 deserialises into typed data; the `--reason` requirement is wired in Phase 5.
    #[serde(default)]
    pub audit: AuditPolicy,

    /// Auth-aware admission knobs (Gate 5; ADR-027 §5).
    ///
    /// Existing policy files without an `auth:` block deserialise as
    /// [`AuthPolicy::default()`] which is the strict default-deny posture:
    /// - `onUnresolvedRequired: deny`
    /// - `allowUpstreamCredentials: false`
    /// - `allowPromptInCi: false`
    #[serde(default)]
    pub auth: AuthPolicy,
}

// =============================================================================
// Sub-policies.
// =============================================================================

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LicensePolicy {
    /// Strict-preset allow list. Empty = unrestricted (default preset only).
    #[serde(default)]
    pub allow: Vec<String>,

    /// Explicit deny list. Always wins.
    #[serde(default)]
    pub deny: Vec<String>,

    /// Action when a component declares an empty `license:` field.
    /// Defaults to `warn` for the default preset and `deny` for strict
    /// (set by the preset constructors in `sindri-policy::loader`).
    #[serde(default)]
    pub on_unknown: Option<PolicyAction>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RegistryPolicy {
    /// Require a valid cosign signature on every registry index fetched.
    /// `None` is treated as `false` (permissive).
    #[serde(default)]
    pub require_signed: Option<bool>,

    /// Allow-list of registry aliases that may be `sindri registry refresh`-ed.
    /// Empty list = no restriction (any alias is allowed).
    #[serde(default)]
    pub trust: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourcesPolicy {
    /// Reject components whose registry entry omits checksums for binary assets.
    #[serde(default)]
    pub require_checksums: Option<bool>,

    /// Reject components whose manifest entry has no exact `@version` pin.
    #[serde(default)]
    pub require_pinned_versions: Option<bool>,

    /// Action for components installed via the `script` backend.
    #[serde(default)]
    pub allow_script_backend: Option<PolicyAction>,

    /// Action for components that declare `requiresElevation: true`.
    #[serde(default)]
    pub allow_privileged: Option<PolicyAction>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NetworkPolicy {
    /// Disable all network access (cache-only mode).
    #[serde(default)]
    pub offline: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapabilitiesPolicy {
    /// Per-capability trust lists (Gate 4 input).
    #[serde(default)]
    pub trust_sources: TrustSources,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrustSources {
    /// Registries trusted to declare `capabilities.collisionHandling`.
    #[serde(default)]
    pub collision_handling: TrustList,

    /// Registries trusted to run project-init steps post-install.
    #[serde(default)]
    pub project_init: TrustList,

    /// Registries trusted to register MCP servers.
    #[serde(default)]
    pub mcp_registration: TrustList,

    /// Registries trusted to edit shell-RC files.
    #[serde(default)]
    pub shell_rc_edits: TrustList,
}

/// Either an explicit allow-list of registry aliases or the wildcard `"*"`
/// (string) meaning "any registry."
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(untagged)]
pub enum TrustList {
    /// String literal `"*"` — trust any registry.
    Wildcard(WildcardLiteral),
    /// Explicit allow-list. Empty list = trust no third-party registry.
    Allowed(Vec<String>),
}

impl Default for TrustList {
    fn default() -> Self {
        TrustList::Allowed(Vec::new())
    }
}

impl TrustList {
    /// Does this trust list admit the given registry alias?
    pub fn admits(&self, registry: &str) -> bool {
        match self {
            TrustList::Wildcard(_) => true,
            TrustList::Allowed(list) => list.iter().any(|r| r == registry),
        }
    }
}

/// Single-value enum so `serde_yaml` can validate the string `"*"` exactly.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum WildcardLiteral {
    #[serde(rename = "*")]
    Any,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuditPolicy {
    /// When `true`, override commands (e.g. `sindri resolve --allow <id>=<reason>`)
    /// require a non-empty reason string. Phase 5 wires the `--reason` enforcement.
    #[serde(default)]
    pub require_justification: bool,
}

/// Auth-aware admission policy (Gate 5, ADR-027 §5).
///
/// All three knobs default to the **deny** stance. Operators must opt in
/// explicitly to relax any of them.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuthPolicy {
    /// What to do when a non-`optional` requirement has no bound source
    /// in the lockfile. Default: `deny`.
    #[serde(default = "default_on_unresolved_required")]
    pub on_unresolved_required: PolicyAction,

    /// If `false` (default), bindings whose `AuthSource` is
    /// `FromUpstreamCredentials` are denied at Gate 5.
    #[serde(default)]
    pub allow_upstream_credentials: bool,

    /// If `false` (default), bindings whose `AuthSource` is `Prompt` are
    /// denied at Gate 5 in non-interactive runs.
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

// =============================================================================
// Convenience accessors used by the resolver / admission gates.
//
// These are thin wrappers that fall back to safe defaults so call sites
// don't have to repeat `unwrap_or(false)` across the codebase.
// =============================================================================

impl InstallPolicy {
    /// `true` when registries.requireSigned is set to `true`.
    pub fn requires_signed_registries(&self) -> bool {
        self.registries.require_signed.unwrap_or(false)
    }

    /// `true` when sources.requireChecksums is set to `true`.
    pub fn requires_checksums(&self) -> bool {
        self.sources.require_checksums.unwrap_or(false)
    }

    /// `true` when sources.requirePinnedVersions is set to `true`.
    pub fn requires_pinned_versions(&self) -> bool {
        self.sources.require_pinned_versions.unwrap_or(false)
    }

    /// `true` when network.offline is set to `true`.
    pub fn is_offline(&self) -> bool {
        self.network.offline.unwrap_or(false)
    }

    /// Action for the `script` backend (default: `allow`).
    pub fn script_backend_action(&self) -> PolicyAction {
        self.sources
            .allow_script_backend
            .clone()
            .unwrap_or(PolicyAction::Allow)
    }

    /// Action for components that require elevation (default: `allow`).
    pub fn privileged_action(&self) -> PolicyAction {
        self.sources
            .allow_privileged
            .clone()
            .unwrap_or(PolicyAction::Allow)
    }

    /// Action for components with empty `license:` (default depends on preset).
    pub fn unknown_license_action(&self) -> PolicyAction {
        self.licenses
            .on_unknown
            .clone()
            .unwrap_or(match self.preset {
                PolicyPreset::Strict => PolicyAction::Deny,
                _ => PolicyAction::Warn,
            })
    }

    /// Is the given registry alias permitted by `registries.trust`?
    /// An empty trust list means "no restriction" (returns true).
    pub fn registry_trusted(&self, alias: &str) -> bool {
        if self.registries.trust.is_empty() {
            true
        } else {
            self.registries.trust.iter().any(|r| r == alias)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_minimal_document() {
        let yaml = r#"
apiVersion: sindri.dev/v4
kind: InstallPolicy
preset: default
"#;
        let p: InstallPolicy = serde_yaml::from_str(yaml).expect("minimal document parses");
        assert_eq!(p.preset, PolicyPreset::Default);
    }

    #[test]
    fn deserializes_full_nested_document() {
        let yaml = r#"
apiVersion: sindri.dev/v4
kind: InstallPolicy
preset: strict
licenses:
  allow: [MIT, Apache-2.0]
  deny: [GPL-3.0-only]
  onUnknown: deny
registries:
  requireSigned: true
  trust:
    - sindri/core
    - acme/internal
sources:
  requireChecksums: true
  requirePinnedVersions: true
  allowScriptBackend: prompt
  allowPrivileged: deny
network:
  offline: false
capabilities:
  trustSources:
    collisionHandling:
      - sindri/core
    projectInit:
      - sindri/core
    mcpRegistration: "*"
    shellRcEdits:
      - sindri/core
audit:
  requireJustification: true
auth:
  onUnresolvedRequired: deny
  allowUpstreamCredentials: false
  allowPromptInCi: false
"#;
        let p: InstallPolicy = serde_yaml::from_str(yaml).expect("full document parses");
        assert_eq!(p.preset, PolicyPreset::Strict);
        assert_eq!(p.licenses.allow.len(), 2);
        assert_eq!(p.licenses.deny.len(), 1);
        assert_eq!(p.licenses.on_unknown, Some(PolicyAction::Deny));
        assert_eq!(p.registries.require_signed, Some(true));
        assert_eq!(p.registries.trust.len(), 2);
        assert!(p.requires_pinned_versions());
        assert_eq!(p.script_backend_action(), PolicyAction::Prompt);
        assert_eq!(p.privileged_action(), PolicyAction::Deny);
        assert!(matches!(
            p.capabilities.trust_sources.mcp_registration,
            TrustList::Wildcard(_)
        ));
        assert!(p.audit.require_justification);
    }

    #[test]
    fn rejects_wrong_api_version() {
        let yaml = r#"
apiVersion: sindri.dev/v3
kind: InstallPolicy
"#;
        let r: Result<InstallPolicy, _> = serde_yaml::from_str(yaml);
        assert!(r.is_err(), "expected wrong-apiVersion to fail");
    }

    #[test]
    fn rejects_wrong_kind() {
        let yaml = r#"
apiVersion: sindri.dev/v4
kind: SomeOtherKind
"#;
        let r: Result<InstallPolicy, _> = serde_yaml::from_str(yaml);
        assert!(r.is_err(), "expected wrong-kind to fail");
    }

    #[test]
    fn rejects_unknown_field() {
        let yaml = r#"
apiVersion: sindri.dev/v4
kind: InstallPolicy
licenses:
  allow: [MIT]
  unknownField: oops
"#;
        let r: Result<InstallPolicy, _> = serde_yaml::from_str(yaml);
        assert!(r.is_err(), "expected unknown-field to fail");
    }

    #[test]
    fn trust_list_wildcard_admits_everything() {
        let t = TrustList::Wildcard(WildcardLiteral::Any);
        assert!(t.admits("anything"));
        assert!(t.admits("sindri/core"));
    }

    #[test]
    fn trust_list_allowed_admits_only_listed() {
        let t = TrustList::Allowed(vec!["sindri/core".into(), "acme".into()]);
        assert!(t.admits("sindri/core"));
        assert!(t.admits("acme"));
        assert!(!t.admits("untrusted"));
    }

    #[test]
    fn empty_trust_list_for_registries_means_unrestricted() {
        let p = InstallPolicy::default();
        assert!(p.registry_trusted("anything"));
    }

    #[test]
    fn nonempty_trust_list_for_registries_is_restrictive() {
        let p = InstallPolicy {
            registries: RegistryPolicy {
                trust: vec!["sindri/core".into()],
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(p.registry_trusted("sindri/core"));
        assert!(!p.registry_trusted("acme"));
    }

    #[test]
    fn unknown_license_action_default_is_warn() {
        let p = InstallPolicy::default();
        assert_eq!(p.unknown_license_action(), PolicyAction::Warn);
    }

    #[test]
    fn unknown_license_action_strict_is_deny() {
        let p = InstallPolicy {
            preset: PolicyPreset::Strict,
            ..Default::default()
        };
        assert_eq!(p.unknown_license_action(), PolicyAction::Deny);
    }
}
