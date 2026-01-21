//! Provider trait definitions

use anyhow::Result;
use async_trait::async_trait;
use sindri_core::config::SindriConfig;
use sindri_core::types::{
    DeployOptions, DeployResult, DeploymentPlan, DeploymentStatus, PrerequisiteStatus,
};

/// Provider trait for deployment backends
#[async_trait]
pub trait Provider: Send + Sync {
    /// Get the provider name
    fn name(&self) -> &'static str;

    /// Check if all prerequisites are met
    fn check_prerequisites(&self) -> Result<PrerequisiteStatus>;

    /// Deploy the environment
    async fn deploy(&self, config: &SindriConfig, opts: DeployOptions) -> Result<DeployResult>;

    /// Connect to a deployed environment
    async fn connect(&self, config: &SindriConfig) -> Result<()>;

    /// Get deployment status
    async fn status(&self, config: &SindriConfig) -> Result<DeploymentStatus>;

    /// Destroy the deployment
    async fn destroy(&self, config: &SindriConfig, force: bool) -> Result<()>;

    /// Generate a deployment plan (dry-run)
    async fn plan(&self, config: &SindriConfig) -> Result<DeploymentPlan>;

    /// Start a stopped/suspended deployment
    async fn start(&self, config: &SindriConfig) -> Result<()>;

    /// Stop a running deployment
    async fn stop(&self, config: &SindriConfig) -> Result<()>;

    /// Check if GPU is supported
    fn supports_gpu(&self) -> bool {
        false
    }

    /// Check if auto-suspend is supported
    fn supports_auto_suspend(&self) -> bool {
        false
    }
}

/// Factory for creating providers
pub trait ProviderFactory {
    /// Create a new provider instance
    fn create() -> Box<dyn Provider>;
}
