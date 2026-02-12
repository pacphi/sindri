//! Image management commands

use crate::cli::{
    ImageCommands, ImageCurrentArgs, ImageInspectArgs, ImageListArgs, ImageVerifyArgs,
    ImageVersionsArgs,
};
use anyhow::{anyhow, Context, Result};
use sindri_core::SindriConfig;
use sindri_image::{
    CachedImageMetadata, ImageReference, ImageVerifier, RegistryClient, RegistryImageResolver,
    VersionResolver,
};
use tracing::{debug, info, warn};

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
    let repository = args
        .repository
        .clone()
        .unwrap_or_else(|| "pacphi/sindri".to_string());

    // Get GitHub token from environment (optional)
    let github_token = std::env::var("GITHUB_TOKEN").ok();

    // Strategy: Try live fetch first if token available, fall back to cache
    let tags = if let Some(token) = github_token {
        info!("Fetching latest images from registry: {}", args.registry);
        match fetch_live_tags(&args.registry, &repository, &token).await {
            Ok(tags) => tags,
            Err(e) => {
                // Check if it's an authentication error
                if e.to_string().contains("401") || e.to_string().contains("Unauthorized") {
                    return Err(anyhow!(
                        "Authentication failed (401 Unauthorized).\n\n\
                         Your GITHUB_TOKEN may be invalid or expired.\n\
                         Please generate a new token at: https://github.com/settings/tokens\n\
                         Required scope: read:packages\n\n\
                         Then set it: export GITHUB_TOKEN=ghp_your_token_here"
                    ));
                }

                // For other errors, warn and try cache
                warn!("Failed to fetch from registry: {}", e);
                warn!("Falling back to cached image data...");
                load_cached_tags()?
            }
        }
    } else {
        info!("No GITHUB_TOKEN found, using cached image data");
        load_cached_tags()?
    };

    // Check if we got any tags
    if tags.is_empty() {
        return Err(anyhow!(
            "No image data available.\n\n\
             This can happen because:\n\
             1. The CLI was built without cached image metadata\n\
             2. No GITHUB_TOKEN is set for live fetching\n\n\
             To fix:\n\
             - Set GITHUB_TOKEN: export GITHUB_TOKEN=ghp_your_token_here\n\
             - Or update to the latest CLI version with cached metadata\n\n\
             Generate token at: https://github.com/settings/tokens (requires 'read:packages' scope)"
        ));
    }

    // Apply filters
    let filtered_tags = apply_filters(tags, &args);

    // Output
    display_tags(&filtered_tags, &args, &repository)?;

    Ok(())
}

/// Fetch tags from registry (live)
async fn fetch_live_tags(registry: &str, repository: &str, token: &str) -> Result<Vec<String>> {
    let registry_client = RegistryClient::new(registry)?.with_token(token);

    let tags = registry_client
        .list_tags(repository)
        .await
        .context("Failed to list tags from registry")?;

    debug!("Fetched {} tags from live registry", tags.len());
    Ok(tags)
}

/// Load tags from embedded cache
fn load_cached_tags() -> Result<Vec<String>> {
    // Load embedded cache
    const EMBEDDED_CACHE: &str = include_str!(concat!(env!("OUT_DIR"), "/image_metadata.json"));

    let cache: CachedImageMetadata =
        serde_json::from_str(EMBEDDED_CACHE).context("Failed to parse embedded image cache")?;

    // Check if cache is stale
    if cache.is_stale() {
        let age = cache.age_days();
        warn!(
            "⚠️  Cached image data is {} days old (cache TTL: {} days)",
            age,
            CachedImageMetadata::TTL_DAYS
        );
        warn!("   Consider updating the CLI or setting GITHUB_TOKEN for latest data");
        println!();
    }

    // Extract just the tag names
    let tags: Vec<String> = cache.tags.iter().map(|t| t.tag.clone()).collect();

    debug!(
        "Loaded {} tags from cache (generated: {})",
        tags.len(),
        cache.generated_at
    );
    Ok(tags)
}

/// Apply filters to tag list
fn apply_filters(mut tags: Vec<String>, args: &ImageListArgs) -> Vec<String> {
    // Filter by pattern if provided
    if let Some(filter) = &args.filter {
        if let Ok(regex) = regex::Regex::new(filter) {
            tags.retain(|tag| regex.is_match(tag));
            debug!("After regex filter: {} tags", tags.len());
        }
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
        debug!("After prerelease filter: {} tags", tags.len());
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

    tags
}

/// Display tags to user
fn display_tags(tags: &[String], args: &ImageListArgs, repository: &str) -> Result<()> {
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
    let mut registry_client = RegistryClient::new(&img_ref.registry)?;
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

    let mut registry_client = RegistryClient::new("ghcr.io")?;
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

    // Build a resolver for registry-based version resolution
    let resolver = config
        .inner()
        .deployment
        .image_config
        .as_ref()
        .and_then(|ic| {
            let token = std::env::var("GITHUB_TOKEN").ok();
            RegistryImageResolver::for_registry(&ic.registry, token).ok()
        });

    // Resolve image
    let image = config
        .resolve_image(
            resolver
                .as_ref()
                .map(|r| r as &dyn sindri_core::config::ImageVersionResolver),
        )
        .await?;

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
