use crate::cache::RegistryCache;
use crate::error::RegistryError;
use crate::index::RegistryIndex;
use sindri_core::policy::InstallPolicy;
use std::path::Path;
use std::time::Duration;

/// OCI registry client. Fetches `index.yaml` blobs from OCI registries
/// (ADR-003).
///
/// **Wave 3A.1** still uses an HTTP shim for `index.yaml` — the
/// `oci-client` crate is on the dependency tree (so 3A.2 can swap to the
/// real OCI Distribution Spec pipeline) but is not yet called here.
///
/// **Wave 3A.2** will:
///   1. Replace [`Self::fetch_from_source`] with `oci-client` manifest +
///      blob fetches.
///   2. Implement [`Self::verify`] using the loaded [`crate::CosignVerifier`].
///   3. Honour the [`InstallPolicy`] threaded through [`Self::with_policy`].
pub struct RegistryClient {
    cache: RegistryCache,
    ttl: Duration,
    /// Active install policy. Wave 3A.1 only stores this; the policy is not
    /// yet consulted in [`Self::fetch_index`].
    policy: Option<InstallPolicy>,
}

impl RegistryClient {
    /// Construct a client backed by the default user cache.
    pub fn new() -> Result<Self, RegistryError> {
        Ok(RegistryClient {
            cache: RegistryCache::new()?,
            ttl: Duration::from_secs(3600),
            policy: None,
        })
    }

    /// Override the cache TTL (default: 1h).
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Attach an install policy. Stored only — policy enforcement is
    /// deferred to Wave 3A.2 (signed-registry gate, offline gate).
    pub fn with_policy(mut self, policy: InstallPolicy) -> Self {
        self.policy = Some(policy);
        self
    }

    /// Read the policy currently attached to this client (mostly for tests
    /// + diagnostics; Wave 3A.2 will actually consult it).
    pub fn policy(&self) -> Option<&InstallPolicy> {
        self.policy.as_ref()
    }

    /// Verify the cosign signature of the given registry against trusted
    /// keys.
    ///
    /// **Wave 3A.1 stub.** Always returns
    /// [`RegistryError::SignatureRequired`] with a message explicitly naming
    /// Wave 3A.2 as the place that wires up real verification — we never
    /// silently succeed.
    pub async fn verify(&self, registry_name: &str) -> Result<(), RegistryError> {
        Err(RegistryError::SignatureRequired {
            registry: registry_name.to_string(),
            reason: "verify not yet implemented (deferred to Wave 3A.2)".to_string(),
        })
    }

    /// Fetch the registry index, using cache if within TTL.
    ///
    /// TODO(wave-3a.2): real OCI fetch via `oci-client`; until then,
    /// `fetch_from_source` still uses the HTTP shim below.
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

        // HTTP(S) — fetch index.yaml directly.
        // TODO(wave-3a.2): replace with oci-client (manifest + blob).
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
