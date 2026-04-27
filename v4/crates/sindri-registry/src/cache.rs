use crate::error::RegistryError;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Content-addressed blob cache at ~/.sindri/cache/registries/ (ADR-003)
pub struct RegistryCache {
    root: PathBuf,
}

impl RegistryCache {
    pub fn new() -> Result<Self, RegistryError> {
        let home = dirs_next::home_dir()
            .ok_or_else(|| RegistryError::CacheError("Cannot determine home directory".into()))?;
        let root = home.join(".sindri").join("cache").join("registries");
        fs::create_dir_all(&root)?;
        Ok(RegistryCache { root })
    }

    pub fn with_path(root: PathBuf) -> Result<Self, RegistryError> {
        fs::create_dir_all(&root)?;
        Ok(RegistryCache { root })
    }

    /// Returns the cached index.yaml content if not stale
    pub fn get_index(&self, registry_name: &str, ttl: Duration) -> Option<String> {
        let path = self.index_path(registry_name);
        if !path.exists() {
            return None;
        }
        let meta = fs::metadata(&path).ok()?;
        let modified = meta.modified().ok()?;
        if SystemTime::now().duration_since(modified).ok()? > ttl {
            return None;
        }
        fs::read_to_string(&path).ok()
    }

    pub fn put_index(&self, registry_name: &str, content: &str) -> Result<(), RegistryError> {
        let path = self.index_path(registry_name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        // Write atomically: temp file + rename
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, content)?;
        fs::rename(&tmp, &path)?;
        Ok(())
    }

    pub fn index_path(&self, registry_name: &str) -> PathBuf {
        self.root.join(registry_name).join("index.yaml")
    }

    pub fn cache_root(&self) -> &Path {
        &self.root
    }
}
