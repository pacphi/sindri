use crate::cache::RegistryCache;
use crate::error::RegistryError;
use crate::index::RegistryIndex;
use std::path::Path;
use std::time::Duration;

/// OCI registry client. Fetches index.yaml blobs from OCI registries (ADR-003).
///
/// Sprint 2 uses HTTP fetch for index.yaml; full OCI Distribution Spec blob pipeline
/// (manifest → blob layer → extract) is Sprint 6 hardening.
pub struct RegistryClient {
    cache: RegistryCache,
    ttl: Duration,
}

impl RegistryClient {
    pub fn new() -> Result<Self, RegistryError> {
        Ok(RegistryClient {
            cache: RegistryCache::new()?,
            ttl: Duration::from_secs(3600),
        })
    }

    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Fetch the registry index, using cache if within TTL.
    pub async fn fetch_index(
        &self,
        registry_name: &str,
        registry_url: &str,
    ) -> Result<RegistryIndex, RegistryError> {
        if let Some(cached) = self.cache.get_index(registry_name, self.ttl) {
            tracing::debug!("Using cached index for {}", registry_name);
            return RegistryIndex::from_yaml(&cached).map_err(RegistryError::Yaml);
        }
        let content = self.fetch_from_source(registry_url).await?;
        self.cache.put_index(registry_name, &content)?;
        RegistryIndex::from_yaml(&content).map_err(RegistryError::Yaml)
    }

    /// Force-refresh the registry index, bypassing cache.
    pub async fn refresh_index(
        &self,
        registry_name: &str,
        registry_url: &str,
    ) -> Result<RegistryIndex, RegistryError> {
        let content = self.fetch_from_source(registry_url).await?;
        self.cache.put_index(registry_name, &content)?;
        RegistryIndex::from_yaml(&content).map_err(RegistryError::Yaml)
    }

    async fn fetch_from_source(&self, registry_url: &str) -> Result<String, RegistryError> {
        // registry:local: protocol — read directly from filesystem
        if let Some(path) = registry_url.strip_prefix("registry:local:") {
            let index_path = Path::new(path).join("index.yaml");
            return std::fs::read_to_string(&index_path).map_err(RegistryError::Io);
        }

        // HTTP(S) — fetch index.yaml directly
        // Full OCI Distribution Spec (manifest + blob) is Sprint 6
        let index_url = format!("{}/index.yaml", registry_url.trim_end_matches('/'));
        tracing::info!("Fetching registry index from {}", index_url);

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| RegistryError::Unreachable(e.to_string()))?;

        let resp = client
            .get(&index_url)
            .send()
            .await
            .map_err(|e| RegistryError::Unreachable(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(RegistryError::Unreachable(format!(
                "HTTP {} fetching {}",
                resp.status(),
                index_url
            )));
        }

        resp.text()
            .await
            .map_err(|e| RegistryError::Unreachable(e.to_string()))
    }
}
