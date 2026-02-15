//! Deploy command

use anyhow::Result;
use camino::Utf8Path;
use sindri_core::config::SindriConfig;
use sindri_core::types::DeployOptions;
use sindri_image::{ImageVerifier, RegistryImageResolver};
use sindri_providers::create_provider;

use crate::cli::DeployArgs;
use crate::output;

pub async fn run(args: DeployArgs, config_path: Option<&Utf8Path>) -> Result<()> {
    // Load config
    let config = SindriConfig::load(config_path)?;

    // Perform preflight check for .env files
    check_env_files(&config, args.env_file.as_deref())?;

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
        // Use pre-built image — build a resolver for registry-based version resolution
        let resolver = config
            .inner()
            .deployment
            .image_config
            .as_ref()
            .and_then(|ic| {
                let token = std::env::var("GITHUB_TOKEN").ok();
                RegistryImageResolver::for_registry(&ic.registry, token).ok()
            });
        match config
            .resolve_image(
                resolver
                    .as_ref()
                    .map(|r| r as &dyn sindri_core::config::ImageVersionResolver),
            )
            .await
        {
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
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Failed to resolve image: {}\n\n\
                    Please specify:\n\
                    1. deployment.image or deployment.image_config in sindri.yaml, OR\n\
                    2. Use --from-source flag to build from Sindri repository",
                    e
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
    // No spinner - let provider output flow through for transparency (like v2)
    let result = provider.deploy(&config, opts).await?;

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
    let cert_identity = Some(image_config.cert_identity_or_default());
    let cert_oidc_issuer = Some(image_config.cert_oidc_issuer_or_default());

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

/// Check for .env files and provide informational feedback
fn check_env_files(config: &SindriConfig, custom_env_file: Option<&Utf8Path>) -> Result<()> {
    let config_dir = config
        .config_path
        .parent()
        .unwrap_or_else(|| Utf8Path::new("."));

    // If custom env file is provided, check that path
    if let Some(env_file_path) = custom_env_file {
        let full_path = if env_file_path.is_absolute() {
            env_file_path.to_path_buf()
        } else {
            config_dir.join(env_file_path)
        };

        if full_path.exists() {
            output::info(&format!("Using custom .env file: {}", full_path.as_str()));
        } else {
            output::warning(&format!(
                "Custom .env file not found: {}",
                full_path.as_str()
            ));
            output::info("Secrets will be loaded from environment variables or other sources");
        }
        return Ok(());
    }

    // Check for default .env files in config directory
    let env_local_path = config_dir.join(".env.local");
    let env_path = config_dir.join(".env");

    let mut found_files = Vec::new();

    if env_local_path.exists() {
        found_files.push(".env.local");
    }
    if env_path.exists() {
        found_files.push(".env");
    }

    if !found_files.is_empty() {
        output::info(&format!(
            "Found environment files in {}: {}",
            config_dir.as_str(),
            found_files.join(", ")
        ));
        output::info("Secrets will be resolved with priority: shell env > .env.local > .env");
    } else {
        output::info(&format!(
            "No .env files found in {} (this is OK)",
            config_dir.as_str()
        ));
        output::info(
            "Secrets will be loaded from environment variables, Vault, S3, or other sources",
        );
        output::info("To use .env files, create .env or .env.local in the config directory");
        output::info("Or use --env-file to specify a custom location");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_check_env_files_with_both_files() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf()).unwrap();

        // Create .env and .env.local
        fs::write(config_dir.join(".env"), "KEY=value").unwrap();
        fs::write(config_dir.join(".env.local"), "SECRET=hidden").unwrap();

        // Create a mock config
        let config_path = config_dir.join("sindri.yaml");
        fs::write(&config_path, "version: '3.0'\nname: test\ndeployment:\n  provider: docker\nextensions:\n  profile: minimal").unwrap();

        let config = SindriConfig::load(Some(&config_path)).unwrap();

        // Should not error
        check_env_files(&config, None).expect("check_env_files with .env files should succeed");
    }

    #[test]
    fn test_check_env_files_with_no_files() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf()).unwrap();

        // Create a mock config without .env files
        let config_path = config_dir.join("sindri.yaml");
        fs::write(&config_path, "version: '3.0'\nname: test\ndeployment:\n  provider: docker\nextensions:\n  profile: minimal").unwrap();

        let config = SindriConfig::load(Some(&config_path)).unwrap();

        // Should not error (just informational)
        check_env_files(&config, None).expect("check_env_files with no .env files should succeed");
    }

    #[test]
    fn test_check_env_files_with_custom_path() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf()).unwrap();

        // Create custom .env file
        let custom_env = config_dir.join("custom.env");
        fs::write(&custom_env, "CUSTOM=value").unwrap();

        // Create a mock config
        let config_path = config_dir.join("sindri.yaml");
        fs::write(&config_path, "version: '3.0'\nname: test\ndeployment:\n  provider: docker\nextensions:\n  profile: minimal").unwrap();

        let config = SindriConfig::load(Some(&config_path)).unwrap();

        // Should detect custom file
        check_env_files(&config, Some(&Utf8PathBuf::from("custom.env")))
            .expect("check_env_files with custom .env path should succeed");
    }

    #[test]
    fn test_check_env_files_custom_path_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf()).unwrap();

        // Create a mock config without custom env file
        let config_path = config_dir.join("sindri.yaml");
        fs::write(&config_path, "version: '3.0'\nname: test\ndeployment:\n  provider: docker\nextensions:\n  profile: minimal").unwrap();

        let config = SindriConfig::load(Some(&config_path)).unwrap();

        // Should warn but not error
        check_env_files(&config, Some(&Utf8PathBuf::from("missing.env")))
            .expect("check_env_files with missing custom .env should still succeed");
    }
}
