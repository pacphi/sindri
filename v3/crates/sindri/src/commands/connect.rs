//! Connect command

use anyhow::Result;
use camino::Utf8Path;
use sindri_core::config::SindriConfig;
use sindri_providers::create_provider;

use crate::cli::ConnectArgs;
use crate::output;

pub async fn run(_args: ConnectArgs, config_path: Option<&Utf8Path>) -> Result<()> {
    // Load config
    let config = SindriConfig::load(config_path)?;

    output::info(&format!("Connecting to {}...", config.name()));

    // Create provider
    let provider = create_provider(config.provider())?;

    // Check prerequisites
    let prereqs = provider.check_prerequisites()?;
    if !prereqs.satisfied {
        output::error("Missing prerequisites:");
        for p in &prereqs.missing {
            output::kv(&p.name, &p.description);
        }
        output::info("");
        output::info(&format!(
            "Run 'sindri doctor --provider {}' for detailed installation instructions",
            config.provider()
        ));
        return Err(anyhow::anyhow!("Prerequisites not satisfied"));
    }

    // Connect
    provider.connect(&config).await?;

    Ok(())
}
