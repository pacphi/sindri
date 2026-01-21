//! Connect command

use anyhow::Result;
use sindri_core::config::SindriConfig;
use sindri_providers::create_provider;

use crate::cli::ConnectArgs;
use crate::output;

pub async fn run(_args: ConnectArgs) -> Result<()> {
    // Load config
    let config = SindriConfig::load(None)?;

    output::info(&format!("Connecting to {}...", config.name()));

    // Create provider
    let provider = create_provider(config.provider())?;

    // Connect
    provider.connect(&config).await?;

    Ok(())
}
