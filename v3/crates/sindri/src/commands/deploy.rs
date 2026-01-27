//! Deploy command

use anyhow::Result;
use sindri_core::config::SindriConfig;
use sindri_core::types::DeployOptions;
use sindri_image::ImageVerifier;
use sindri_providers::create_provider;

use crate::cli::DeployArgs;
use crate::output;

pub async fn run(args: DeployArgs) -> Result<()> {
    // Load config
    let config = SindriConfig::load(None)?;

    output::header(&format!("Deploying sindri to {}", config.provider()));

    // Check if building from source (via flag or YAML config)
    let build_from_source = args.from_source
        || config
            .inner()
            .deployment
            .build_from_source
            .as_ref()
            .map(|b| b.enabled)
            .unwrap_or(false);

    // Resolve image or prepare for source build
    let resolved_image = if build_from_source {
        // Building from source - skip image resolution
        output::info("Building from Sindri repository source");
        if let Some(build_config) = &config.inner().deployment.build_from_source {
            if let Some(ref_name) = &build_config.git_ref {
                output::info(&format!("Using git ref: {}", ref_name));
            }
        }
        output::info("");
        None
    } else {
        // Use pre-built image
        match config.resolve_image().await {
            Ok(image) => {
                output::info(&format!("Using image: {}", image));
                output::info("");

                // Verify image if configured and not skipped
                if !args.skip_image_verification && should_verify_image(&config, &args) {
                    verify_image(&image, &config).await?;
                    output::info("");
                }

                Some(image)
            }
            Err(_) => {
                return Err(anyhow::anyhow!(
                    "No image configured. Please specify:\n\
                    1. deployment.image or deployment.image_config in sindri.yaml, OR\n\
                    2. Use --from-source flag to build from Sindri repository"
                ));
            }
        }
    };

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

    // Display deployment info
    if let Some(image) = &resolved_image {
        output::info(&format!("Deploying with image: {}", image));
    } else {
        output::info("Building from source...");
    }
    output::info("");

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

/// Check if image verification should be performed
fn should_verify_image(config: &SindriConfig, args: &DeployArgs) -> bool {
    if args.skip_image_verification {
        return false;
    }

    // Check if image_config exists and has verification enabled
    if let Some(image_config) = &config.inner().deployment.image_config {
        image_config.verify_signature || image_config.verify_provenance
    } else {
        false
    }
}

/// Verify image signature and provenance
async fn verify_image(image: &str, config: &SindriConfig) -> Result<()> {
    // Check if cosign is available
    if !ImageVerifier::is_available() {
        output::warning("cosign not installed - skipping image verification");
        output::info("Install cosign from: https://docs.sigstore.dev/cosign/installation/");
        return Ok(());
    }

    let verifier = ImageVerifier::new()?;
    let image_config = config.inner().deployment.image_config.as_ref().unwrap();

    // Get certificate identity and OIDC issuer from config or use defaults
    let cert_identity = image_config
        .certificate_identity
        .as_deref()
        .or(Some("https://github.com/pacphi/sindri"));

    let cert_oidc_issuer = image_config
        .certificate_oidc_issuer
        .as_deref()
        .or(Some("https://token.actions.githubusercontent.com"));

    // Verify signature
    if image_config.verify_signature {
        let spinner = output::spinner("Verifying image signature...");
        match verifier
            .verify_signature(image, cert_identity, cert_oidc_issuer)
            .await
        {
            Ok(result) if result.verified => {
                spinner.finish_and_clear();
                output::success("✓ Signature verified");
            }
            Ok(result) => {
                spinner.finish_and_clear();
                output::error("✗ Signature verification failed");
                for error in &result.errors {
                    output::info(&format!("  {}", error));
                }
                return Err(anyhow::anyhow!(
                    "Image signature verification failed. Use --skip-image-verification to bypass."
                ));
            }
            Err(e) => {
                spinner.finish_and_clear();
                output::warning(&format!("✗ Signature verification error: {}", e));
            }
        }
    }

    // Verify provenance
    if image_config.verify_provenance {
        let spinner = output::spinner("Verifying SLSA provenance...");
        match verifier
            .verify_provenance(image, cert_identity, cert_oidc_issuer)
            .await
        {
            Ok(result) if result.verified => {
                spinner.finish_and_clear();
                let level = result.slsa_level.as_deref().unwrap_or("Unknown");
                output::success(&format!("✓ Provenance verified ({})", level));
            }
            Ok(result) => {
                spinner.finish_and_clear();
                output::warning("⚠ Provenance verification failed");
                for error in &result.errors {
                    output::info(&format!("  {}", error));
                }
                // Provenance failure is a warning, not an error
            }
            Err(e) => {
                spinner.finish_and_clear();
                output::warning(&format!("⚠ Provenance verification error: {}", e));
            }
        }
    }

    Ok(())
}
