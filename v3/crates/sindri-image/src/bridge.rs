//! Bridge between sindri-core's `ImageVersionResolver` trait and the concrete
//! `RegistryClient` / `VersionResolver` types in this crate.

use crate::registry::RegistryClient;
use crate::resolver::VersionResolver;
use crate::types::ResolutionStrategy as ImageResolutionStrategy;
use sindri_core::config::ImageVersionResolver;
use sindri_core::types::ResolutionStrategy as CoreResolutionStrategy;
use std::future::Future;
use std::pin::Pin;

/// Concrete [`ImageVersionResolver`] backed by an OCI registry client.
///
/// Construct via [`RegistryImageResolver::new`] or
/// [`RegistryImageResolver::for_registry`].
pub struct RegistryImageResolver {
    resolver: VersionResolver,
}

impl RegistryImageResolver {
    /// Wrap an existing `VersionResolver`.
    pub fn new(resolver: VersionResolver) -> Self {
        Self { resolver }
    }

    /// Convenience: build a resolver for the given `registry` URL,
    /// extracting the host from paths like `"ghcr.io/org/repo"`.
    ///
    /// An optional authentication token (e.g. `GITHUB_TOKEN`) can be supplied.
    pub fn for_registry(registry: &str, token: Option<String>) -> anyhow::Result<Self> {
        let registry_host = if registry.contains("ghcr.io") {
            "ghcr.io"
        } else if registry.contains("docker.io") {
            "docker.io"
        } else {
            registry.split('/').next().unwrap_or("ghcr.io")
        };

        let mut client = RegistryClient::new(registry_host)?;
        if let Some(t) = token {
            client = client.with_token(t);
        }
        Ok(Self {
            resolver: VersionResolver::new(client),
        })
    }
}

/// Map `sindri_core::types::ResolutionStrategy` to the local enum.
fn map_strategy(s: CoreResolutionStrategy) -> ImageResolutionStrategy {
    match s {
        CoreResolutionStrategy::Semver => ImageResolutionStrategy::Semver,
        CoreResolutionStrategy::LatestStable => ImageResolutionStrategy::LatestStable,
        CoreResolutionStrategy::PinToCli => ImageResolutionStrategy::PinToCli,
        CoreResolutionStrategy::Explicit => ImageResolutionStrategy::Explicit,
    }
}

impl ImageVersionResolver for RegistryImageResolver {
    fn resolve<'a>(
        &'a self,
        repository: &'a str,
        strategy: CoreResolutionStrategy,
        constraint: Option<&'a str>,
        cli_version: Option<&'a str>,
        allow_prerelease: bool,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<String>> + Send + 'a>> {
        Box::pin(async move {
            self.resolver
                .resolve_with_strategy(
                    repository,
                    map_strategy(strategy),
                    constraint,
                    cli_version,
                    allow_prerelease,
                )
                .await
        })
    }
}
