//! Start command

use anyhow::Result;
use camino::Utf8Path;
use sindri_core::config::SindriConfig;
use sindri_providers::create_provider;

use crate::cli::StartArgs;
use crate::output;

pub async fn run(args: StartArgs, config_path: Option<&Utf8Path>) -> Result<()> {
    let _ = args;

    // Load config
    let config = SindriConfig::load(config_path)?;

    output::header(&format!("Starting {}", config.name()));

    // Create provider
    let provider = create_provider(config.provider())?;

    // Check prerequisites
    let prereqs = provider.check_prerequisites()?;
    if !prereqs.satisfied {
        output::error("Missing prerequisites:");
        for p in &prereqs.missing {
            output::kv(&p.name, &p.description);
            if let Some(hint) = &p.install_hint {
                output::info(&format!("  Install: {}", hint));
            }
        }
        output::info("");
        output::info(&format!(
            "Run 'sindri doctor --provider {}' for detailed installation instructions",
            config.provider()
        ));
        return Err(anyhow::anyhow!("Prerequisites not satisfied"));
    }

    // Start
    let spinner = output::spinner("Starting deployment...");
    provider.start(&config).await?;
    spinner.finish_and_clear();

    output::success("Deployment started");

    Ok(())
}
