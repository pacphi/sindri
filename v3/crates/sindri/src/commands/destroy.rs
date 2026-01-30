//! Destroy command

use anyhow::Result;
use camino::Utf8Path;
use dialoguer::Confirm;
use sindri_core::config::SindriConfig;
use sindri_providers::create_provider;

use crate::cli::DestroyArgs;
use crate::output;

pub async fn run(args: DestroyArgs, config_path: Option<&Utf8Path>) -> Result<()> {
    // Load config
    let config = SindriConfig::load(config_path)?;

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
