//! Image management commands

use crate::cli::{
    ImageCommands, ImageCurrentArgs, ImageInspectArgs, ImageListArgs, ImageVerifyArgs,
    ImageVersionsArgs,
};
use anyhow::{Context, Result};
use sindri_core::SindriConfig;
use sindri_image::{ImageReference, ImageVerifier, RegistryClient, VersionResolver};
use tracing::{debug, info};

/// Execute image command
pub async fn execute(cmd: ImageCommands) -> Result<()> {
    match cmd {
        ImageCommands::List(args) => list(args).await,
        ImageCommands::Inspect(args) => inspect(args).await,
        ImageCommands::Verify(args) => verify(args).await,
        ImageCommands::Versions(args) => versions(args).await,
        ImageCommands::Current(args) => current(args).await,
    }
}

/// List available images from registry
async fn list(args: ImageListArgs) -> Result<()> {
    info!("Listing images from registry: {}", args.registry);

    let repository = args
        .repository
        .unwrap_or_else(|| "pacphi/sindri".to_string());

    // Get GitHub token from environment (optional)
    let github_token = std::env::var("GITHUB_TOKEN").ok();

    // Create registry client
    let mut registry_client = RegistryClient::new(&args.registry);
    if let Some(token) = github_token {
        registry_client = registry_client.with_token(token);
    }

    // List tags
    let mut tags = registry_client
        .list_tags(&repository)
        .await
        .context("Failed to list tags")?;

    debug!("Found {} tags", tags.len());

    // Filter tags if pattern provided
    if let Some(filter) = &args.filter {
        let regex = regex::Regex::new(filter)?;
        tags.retain(|tag| regex.is_match(tag));
        debug!("After filtering: {} tags", tags.len());
    }

    // Filter prereleases if not included
    if !args.include_prerelease {
        tags.retain(|tag| {
            let version_str = tag.strip_prefix('v').unwrap_or(tag);
            match semver::Version::parse(version_str) {
                Ok(v) => v.pre.is_empty(),
                Err(_) => true, // Keep non-semver tags
            }
        });
        debug!("After removing prereleases: {} tags", tags.len());
    }

    // Sort tags (newest first)
    tags.sort_by(|a, b| {
        let ver_a = a.strip_prefix('v').unwrap_or(a);
        let ver_b = b.strip_prefix('v').unwrap_or(b);

        match (semver::Version::parse(ver_a), semver::Version::parse(ver_b)) {
            (Ok(va), Ok(vb)) => vb.cmp(&va),
            _ => b.cmp(a),
        }
    });

    // Output
    if args.json {
        println!("{}", serde_json::to_string_pretty(&tags)?);
    } else {
        println!("Available images for {}:{}:", args.registry, repository);
        println!();
        for tag in tags.iter().take(20) {
            println!("  {}:{}", repository, tag);
        }
        if tags.len() > 20 {
            println!();
            println!("... and {} more (use --json to see all)", tags.len() - 20);
        }
    }

    Ok(())
}

/// Inspect image details
async fn inspect(args: ImageInspectArgs) -> Result<()> {
    info!("Inspecting image: {}", args.tag);

    let img_ref = ImageReference::parse(&args.tag)?;

    // Get GitHub token from environment (optional)
    let github_token = std::env::var("GITHUB_TOKEN").ok();

    // Create registry client
    let mut registry_client = RegistryClient::new(&img_ref.registry);
    if let Some(token) = github_token {
        registry_client = registry_client.with_token(token);
    }

    // Get image info
    let info = registry_client
        .get_image_info(&img_ref)
        .await
        .context("Failed to get image info")?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&info)?);
        return Ok(());
    }

    // Display info
    println!("Image: {}", img_ref);
    println!();
    println!("Digest: {}", info.digest);
    if let Some(size) = info.size {
        let size_mb = size as f64 / 1_048_576.0;
        println!("Size: {:.2} MB", size_mb);
    }
    if let Some(created) = info.created {
        println!("Created: {}", created);
    }

    if !info.platforms.is_empty() {
        println!();
        println!("Platforms:");
        for platform in &info.platforms {
            let variant = platform
                .variant
                .as_ref()
                .map(|v| format!("/{}", v))
                .unwrap_or_default();
            println!("  - {}/{}{}", platform.os, platform.architecture, variant);
        }
    }

    if !info.labels.is_empty() {
        println!();
        println!("Labels:");
        for (key, value) in &info.labels {
            println!("  {}: {}", key, value);
        }
    }

    // Show SBOM if requested
    if args.sbom {
        println!();
        println!("Fetching SBOM...");

        if !ImageVerifier::is_available() {
            println!("⚠️  cosign not installed - SBOM verification requires cosign");
            println!("   Install from: https://docs.sigstore.dev/cosign/installation/");
            return Ok(());
        }

        let verifier = ImageVerifier::new()?;
        match verifier.fetch_sbom(&img_ref.to_string()).await {
            Ok(sbom) => {
                println!();
                println!("SBOM ({} format, version {})", sbom.format, sbom.version);
                println!("Packages: {}", sbom.packages.len());
                println!();
                println!("Top 10 packages:");
                for pkg in sbom.packages.iter().take(10) {
                    let version = pkg.version.as_deref().unwrap_or("unknown");
                    println!("  - {} ({})", pkg.name, version);
                }
                if sbom.packages.len() > 10 {
                    println!("  ... and {} more", sbom.packages.len() - 10);
                }
            }
            Err(e) => {
                println!("Failed to fetch SBOM: {}", e);
            }
        }
    }

    Ok(())
}

/// Verify image signature and provenance
async fn verify(args: ImageVerifyArgs) -> Result<()> {
    info!("Verifying image: {}", args.tag);

    if !ImageVerifier::is_available() {
        anyhow::bail!("cosign not installed - verification requires cosign\nInstall from: https://docs.sigstore.dev/cosign/installation/");
    }

    let verifier = ImageVerifier::new()?;

    // Default certificate identity and issuer for Sindri images
    let cert_identity = Some("https://github.com/pacphi/sindri");
    let cert_oidc_issuer = Some("https://token.actions.githubusercontent.com");

    // Verify signature
    if !args.no_signature {
        println!("Verifying signature...");
        let sig_result = verifier
            .verify_signature(&args.tag, cert_identity, cert_oidc_issuer)
            .await?;

        if sig_result.verified {
            println!("✅ Signature verified");
            for sig in &sig_result.signatures {
                println!("   Issuer: {}", sig.issuer);
                println!("   Subject: {}", sig.subject);
            }
        } else {
            println!("❌ Signature verification failed");
            for error in &sig_result.errors {
                println!("   Error: {}", error);
            }
            anyhow::bail!("Signature verification failed");
        }
        println!();
    }

    // Verify provenance
    if !args.no_provenance {
        println!("Verifying provenance...");
        let prov_result = verifier
            .verify_provenance(&args.tag, cert_identity, cert_oidc_issuer)
            .await?;

        if prov_result.verified {
            println!("✅ Provenance verified");
            if let Some(level) = &prov_result.slsa_level {
                println!("   SLSA Level: {}", level);
            }
            if let Some(builder) = &prov_result.builder_id {
                println!("   Builder: {}", builder);
            }
            if let Some(source) = &prov_result.source_repo {
                println!("   Source: {}", source);
            }
        } else {
            println!("⚠️  Provenance verification failed");
            for error in &prov_result.errors {
                println!("   Error: {}", error);
            }
        }
    }

    Ok(())
}

/// Show version compatibility matrix
async fn versions(args: ImageVersionsArgs) -> Result<()> {
    let cli_version = args
        .cli_version
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());

    println!("CLI Version: {}", cli_version);
    println!();
    println!("Compatible image versions:");
    println!();

    // Get versions from registry
    let repository = "pacphi/sindri";
    let github_token = std::env::var("GITHUB_TOKEN").ok();

    let mut registry_client = RegistryClient::new("ghcr.io");
    if let Some(token) = github_token {
        registry_client = registry_client.with_token(token);
    }

    let resolver = VersionResolver::new(registry_client);

    // Parse CLI version
    let cli_ver = semver::Version::parse(&cli_version)?;
    let major = cli_ver.major;

    // Find all versions matching the major version
    let constraint = format!("^{}.0.0", major);
    match resolver
        .find_matching_versions(repository, &constraint, false)
        .await
    {
        Ok(versions) => {
            if args.format == "json" {
                println!("{}", serde_json::to_string_pretty(&versions)?);
            } else {
                println!("Semver constraint: {}", constraint);
                println!();
                for version in versions.iter().take(10) {
                    println!("  ✓ ghcr.io/{}:{}", repository, version);
                }
                if versions.len() > 10 {
                    println!("  ... and {} more", versions.len() - 10);
                }
            }
        }
        Err(e) => {
            println!("Failed to fetch versions: {}", e);
        }
    }

    Ok(())
}

/// Show currently deployed image
async fn current(args: ImageCurrentArgs) -> Result<()> {
    // Load config
    let config = SindriConfig::load(None)?;

    // Resolve image
    let image = config.resolve_image().await?;

    if args.json {
        let output = serde_json::json!({
            "image": image,
            "config_path": config.config_path,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("Current image: {}", image);
        println!("Config: {}", config.config_path);
    }

    Ok(())
}
