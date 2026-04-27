use crate::error::RegistryError;
use crate::index::RegistryIndex;
use sindri_core::component::ComponentManifest;
use std::fs;
use std::path::PathBuf;

/// Local registry loader for development (registry:local:/path protocol, ADR-003)
pub struct LocalRegistry {
    root: PathBuf,
}

impl LocalRegistry {
    pub fn new(path: &str) -> Self {
        LocalRegistry {
            root: PathBuf::from(path),
        }
    }

    pub fn load_index(&self) -> Result<RegistryIndex, RegistryError> {
        let content = fs::read_to_string(self.root.join("index.yaml"))?;
        RegistryIndex::from_yaml(&content).map_err(RegistryError::Yaml)
    }

    pub fn load_component(
        &self,
        backend: &str,
        name: &str,
    ) -> Result<ComponentManifest, RegistryError> {
        let dir = if backend == "collection" {
            self.root.join("collections").join(name)
        } else {
            self.root.join("components").join(name)
        };
        let content = fs::read_to_string(dir.join("component.yaml"))?;
        serde_yaml::from_str(&content).map_err(RegistryError::Yaml)
    }

    pub fn list_components(&self) -> Result<Vec<(String, String)>, RegistryError> {
        let index = self.load_index()?;
        Ok(index
            .components
            .iter()
            .map(|c| (c.backend.clone(), c.name.clone()))
            .collect())
    }
}
