//! Secret source trait and implementations

pub mod env;
pub mod file;
pub mod vault;

use crate::types::{ResolutionContext, ResolvedSecret};
use anyhow::Result;
use async_trait::async_trait;
use sindri_core::types::SecretConfig;

/// Trait for secret sources
#[async_trait]
pub trait SecretSource: Send + Sync {
    /// Resolve a secret from this source
    ///
    /// Returns Ok(Some(secret)) if resolved successfully
    /// Returns Ok(None) if the secret is not available from this source
    /// Returns Err if there was an error attempting to resolve
    async fn resolve(
        &self,
        definition: &SecretConfig,
        ctx: &ResolutionContext,
    ) -> Result<Option<ResolvedSecret>>;

    /// Validate this source is available (e.g., vault CLI installed)
    fn validate(&self) -> Result<()>;

    /// Source name for error messages
    fn name(&self) -> &'static str;
}

pub use env::EnvSource;
pub use file::FileSource;
pub use vault::VaultSource;
