// ADR-003: OCI-only registry distribution
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

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
