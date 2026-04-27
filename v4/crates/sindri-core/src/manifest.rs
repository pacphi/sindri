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
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TrustConfig {
    pub signer: String,
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
