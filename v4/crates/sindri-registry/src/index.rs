use serde::{Deserialize, Serialize};
use sindri_core::registry::ComponentEntry;

/// The registry index.yaml format served from OCI registries (ADR-003, ADR-016)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryIndex {
    pub version: u32,
    pub registry: String,
    pub generated_at: Option<String>,
    pub components: Vec<ComponentEntry>,
}

impl RegistryIndex {
    pub fn from_yaml(s: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(s)
    }

    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }

    /// Find a component by backend:name address
    pub fn find(&self, backend: &str, name: &str) -> Option<&ComponentEntry> {
        self.components
            .iter()
            .find(|c| c.backend == backend && c.name == name)
    }

    pub fn find_by_name(&self, name: &str) -> Vec<&ComponentEntry> {
        self.components.iter().filter(|c| c.name == name).collect()
    }
}
