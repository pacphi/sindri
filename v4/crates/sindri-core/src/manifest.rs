// ADR-001: User-authored sindri.yaml BOM as single source of truth
use crate::component::BomEntry;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BomManifest {
    #[serde(rename = "$schema")]
    pub schema: Option<String>,
    pub name: Option<String>,
    pub components: Vec<BomEntry>,
    #[serde(default)]
    pub registries: Vec<RegistryConfig>,
    #[serde(default)]
    pub targets: HashMap<String, TargetConfig>,
    pub preferences: Option<Preferences>,
    pub r#override: Option<Vec<OverrideEntry>>,
    /// Optional secret references (Sprint 12, Wave 4C).
    ///
    /// Map of secret-id → prefixed `AuthValue` string (`env:FOO`,
    /// `file:~/.token`, `cli:gh`, `plain:…`). Resolved on demand by
    /// `sindri secrets validate`; values are never persisted.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub secrets: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RegistryConfig {
    pub name: String,
    pub url: String,
    pub trust: Option<TrustConfig>,
    /// Wave 6A — ADR-014 D1: cosign verification mode for this registry.
    ///
    /// - `key-based` (default, omit-able): existing flow, loads
    ///   `~/.sindri/trust/<name>/cosign-*.pub`.
    /// - `keyless`: short-lived Fulcio cert + Rekor inclusion proof.
    ///   When set, the registry SHOULD also populate `identity` so the
    ///   verifier can SAN-match.
    ///
    /// Field is `Option<String>` rather than the typed
    /// `sindri_registry::VerificationMode` to keep the core crate free
    /// of a registry-crate dep (avoids a cycle); the registry crate
    /// parses + validates the string at load time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_mode: Option<String>,
    /// The expected SAN URI + OIDC issuer for keyless mode. Required when
    /// `verification_mode == "keyless"`; ignored otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<RegistryIdentity>,
    /// Wave 6A.1 — per-component trust scoping (ADR-014, follow-up to PR #228 + #237).
    ///
    /// Each entry narrows the trust set for components whose canonical
    /// address matches `component_glob`. Most-specific glob wins
    /// (longest-pattern tie-break); when no entry matches, the verifier
    /// falls back to the registry-level `trust` / `identity` fields.
    ///
    /// Fail-closed semantics: under
    /// `policy.require_signed_registries=true` a component that matches
    /// neither an override **nor** registry-level trust is rejected.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trust_overrides: Vec<TrustOverride>,
}

/// Per-component trust scope (Wave 6A.1).
///
/// Lets a single registry publish artifacts signed by multiple teams /
/// keys / Fulcio identities — a common pattern when an organisation
/// shares one OCI registry across product groups.
///
/// Either [`Self::keys`] (key-based override) or [`Self::identity`]
/// (keyless override) should be set; setting both is allowed and means
/// the component can verify under either mode (whichever the cosign
/// signature actually used).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TrustOverride {
    /// Glob pattern matched against the component's canonical address
    /// (e.g. `"mise:nodejs"`, `"team-foo/*"`, `"team-bar/specific@v1"`).
    /// `*` matches any run of characters except `/`; `**` matches any
    /// run including `/`. Most-specific match (longest pattern) wins.
    pub component_glob: String,
    /// Key-based trust override — list of paths to PEM-encoded P-256
    /// public keys. Resolved relative to the manifest file at load time.
    /// Verifier accepts if any key matches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keys: Option<Vec<PathBuf>>,
    /// Keyless trust override — SAN URI + OIDC issuer pair the
    /// Fulcio-issued cert must match exactly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<RegistryIdentity>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TrustConfig {
    pub signer: String,
}

/// Mirror of `sindri_registry::keyless::KeylessIdentity` — duplicated
/// here so `sindri-core` doesn't depend on `sindri-registry` (which
/// would introduce a cycle, since the registry crate already depends on
/// core for `BomEntry` etc.). The registry crate converts via
/// `From<&RegistryIdentity>`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct RegistryIdentity {
    /// The expected SAN URI extension in the Fulcio-issued certificate
    /// (e.g. a GitHub Actions workflow run URL).
    pub san_uri: String,
    /// The expected OIDC issuer URL (e.g.
    /// `https://token.actions.githubusercontent.com`).
    pub issuer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TargetConfig {
    pub kind: String,
    pub infra: Option<serde_json::Value>,
    pub auth: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Preferences {
    pub backend_order: Option<HashMap<String, Vec<String>>>,
    pub default_target: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OverrideEntry {
    pub address: String,
    pub reason: String,
}
