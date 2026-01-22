//! Deploy command

use anyhow::Result;
use sindri_core::config::SindriConfig;
use sindri_core::types::DeployOptions;
use sindri_providers::create_provider;

use crate::cli::DeployArgs;
use crate::output;

pub async fn run(args: DeployArgs) -> Result<()> {
    // Load config
    let config = SindriConfig::load(None)?;

    output::header(&format!(
        "Deploying {} to {}",
        config.name(),
        config.provider()
    ));

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

    // Create deploy options
    let opts = DeployOptions {
        force: args.force,
        dry_run: args.dry_run,
        wait: args.wait,
        timeout: Some(args.timeout),
        skip_validation: args.skip_validation,
        verbose: false,
    };

    // Dry run
    if args.dry_run {
        let plan = provider.plan(&config).await?;
        output::info("Dry run - would execute:");
        for action in &plan.actions {
            output::kv(&action.resource, &action.description);
        }
        return Ok(());
    }

    // Deploy
    let spinner = output::spinner("Deploying...");
    let result = provider.deploy(&config, opts).await?;
    spinner.finish_and_clear();

    if result.success {
        output::success("Deployment complete");
        if let Some(conn) = &result.connection {
            if let Some(ssh) = &conn.ssh_command {
                output::kv("SSH", ssh);
            }
            if let Some(http) = &conn.http_url {
                output::kv("HTTP", http);
            }
        }
    } else {
        output::error("Deployment failed");
        for msg in &result.messages {
            output::info(msg);
        }
    }

    for warning in &result.warnings {
        output::warning(warning);
    }

    Ok(())
}
