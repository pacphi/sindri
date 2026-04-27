// ADR-003: OCI-only registry distribution
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Canonical name of the first-party Sindri registry.
///
/// Per ADR-008 (Gate 4 — capability trust), only components sourced from this
/// registry may declare a `:shared` `collision_handling.path_prefix`. All other
/// registries must declare prefixes scoped to `{component-name}/...`.
pub const CORE_REGISTRY_NAME: &str = "sindri/core";

/// Sentinel value for the `:shared` collision-handling escape hatch.
/// Only valid when the owning component is sourced from [`CORE_REGISTRY_NAME`].
pub const SHARED_PATH_PREFIX: &str = ":shared";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RegistryIndex {
    pub version: u32,
    pub registry: String,
    pub components: Vec<ComponentEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ComponentEntry {
    pub name: String,
    pub backend: String,
    pub latest: String,
    #[serde(default)]
    pub versions: Vec<String>,
    pub description: String,
    pub kind: ComponentKind,
    pub oci_ref: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ComponentKind {
    Component,
    Collection,
}
