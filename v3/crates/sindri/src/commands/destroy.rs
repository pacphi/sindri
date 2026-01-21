//! Destroy command

use anyhow::Result;
use dialoguer::Confirm;
use sindri_core::config::SindriConfig;
use sindri_providers::create_provider;

use crate::cli::DestroyArgs;
use crate::output;

pub async fn run(args: DestroyArgs) -> Result<()> {
    // Load config
    let config = SindriConfig::load(None)?;

    // Confirm destruction
    if !args.force {
        let confirmed = Confirm::new()
            .with_prompt(format!(
                "Are you sure you want to destroy '{}'?",
                config.name()
            ))
            .default(false)
            .interact()?;

        if !confirmed {
            output::info("Cancelled");
            return Ok(());
        }
    }

    output::header(&format!("Destroying {}", config.name()));

    // Create provider
    let provider = create_provider(config.provider())?;

    // Destroy
    let spinner = output::spinner("Destroying resources...");
    provider.destroy(&config, args.force).await?;
    spinner.finish_and_clear();

    output::success("Deployment destroyed");

    if args.volumes {
        output::info("Volumes also removed");
    }

    Ok(())
}
