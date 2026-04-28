// ADR-001: User-authored sindri.yaml BOM as single source of truth
use crate::component::BomEntry;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
