//! Build script for Sindri CLI
//!
//! This script runs at compile time to:
//! 1. Generate build metadata (timestamp, git info)
//! 2. Fetch and cache recent image metadata from GHCR (if GITHUB_TOKEN available)

use std::env;
use std::fs;
use std::path::Path;

#[tokio::main]
async fn main() {
    // Set build timestamp
    let now = chrono::Utc::now();
    println!("cargo:rustc-env=BUILD_DATE={}", now.format("%Y-%m-%d"));
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", now.to_rfc3339());

    // Try to get git info
    if let Ok(output) = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
    {
        if output.status.success() {
            let sha = String::from_utf8_lossy(&output.stdout);
            println!("cargo:rustc-env=GIT_SHA={}", sha.trim());
        }
    }

    // Rerun if git changes
    println!("cargo:rerun-if-changed=.git/HEAD");

    // Cache image metadata
    if let Err(e) = cache_image_metadata().await {
        eprintln!("⚠️  Warning: Failed to setup image cache: {}", e);
        // Don't fail the build, just warn
    }
}

async fn cache_image_metadata() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = env::var("OUT_DIR")?;
    let dest_path = Path::new(&out_dir).join("image_metadata.json");

    // Check for GITHUB_TOKEN
    match env::var("GITHUB_TOKEN") {
        Ok(token) if !token.is_empty() => {
            eprintln!("📦 Fetching image metadata from GHCR...");

            match fetch_and_cache_metadata(&token).await {
                Ok(metadata_json) => {
                    fs::write(&dest_path, metadata_json)?;
                    eprintln!("✅ Cached image metadata successfully");
                }
                Err(e) => {
                    eprintln!("⚠️  Failed to fetch image metadata: {}", e);
                    eprintln!("   Creating empty cache (runtime will require GITHUB_TOKEN)");
                    write_empty_cache(&dest_path)?;
                }
            }
        }
        _ => {
            eprintln!("ℹ️  No GITHUB_TOKEN found, creating empty image cache");
            eprintln!("   Users will need to set GITHUB_TOKEN at runtime");
            write_empty_cache(&dest_path)?;
        }
    }

    println!("cargo:rerun-if-env-changed=GITHUB_TOKEN");
    Ok(())
}

async fn fetch_and_cache_metadata(token: &str) -> Result<String, Box<dyn std::error::Error>> {
    use sindri_image::{CachedImageMetadata, CachedTagInfo, RegistryClient};

    const REGISTRY: &str = "ghcr.io";
    const REPOSITORY: &str = "pacphi/sindri";
    const MAX_VERSIONS: usize = 5;

    // Create registry client with authentication
    let registry_client = RegistryClient::new(REGISTRY)?.with_token(token);

    // Fetch all tags
    let mut tags = registry_client
        .list_tags(REPOSITORY)
        .await
        .map_err(|e| format!("Failed to list tags: {}", e))?;

    // Sort by semver (newest first) and take first N
    // Note: comparison must maintain total order - semver tags come first, then non-semver alphabetically
    tags.sort_by(|a, b| {
        let ver_a = a.strip_prefix('v').unwrap_or(a);
        let ver_b = b.strip_prefix('v').unwrap_or(b);

        match (semver::Version::parse(ver_a), semver::Version::parse(ver_b)) {
            (Ok(va), Ok(vb)) => vb.cmp(&va), // Both semver: sort descending
            (Ok(_), Err(_)) => std::cmp::Ordering::Less, // Semver comes first
            (Err(_), Ok(_)) => std::cmp::Ordering::Greater, // Non-semver comes after
            (Err(_), Err(_)) => b.cmp(a),    // Both non-semver: sort descending alphabetically
        }
    });

    let recent_tags: Vec<_> = tags.into_iter().take(MAX_VERSIONS).collect();

    // Fetch manifest for each tag to get digest and created date
    let mut cached_tags = Vec::new();
    for tag in recent_tags {
        match registry_client.get_manifest(REPOSITORY, &tag).await {
            Ok(manifest) => {
                cached_tags.push(CachedTagInfo {
                    tag: tag.clone(),
                    digest: manifest.config.digest.clone(),
                    created: chrono::Utc::now().to_rfc3339(), // Simplified: use current time
                });
            }
            Err(e) => {
                eprintln!("   Warning: Failed to fetch manifest for {}: {}", tag, e);
            }
        }
    }

    // Create cached metadata
    let cache = CachedImageMetadata {
        generated_at: chrono::Utc::now().to_rfc3339(),
        registry: REGISTRY.to_string(),
        repository: REPOSITORY.to_string(),
        tags: cached_tags,
    };

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&cache)?;
    Ok(json)
}

fn write_empty_cache(dest_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use sindri_image::CachedImageMetadata;

    let empty_cache = CachedImageMetadata {
        generated_at: chrono::Utc::now().to_rfc3339(),
        registry: "ghcr.io".to_string(),
        repository: "pacphi/sindri".to_string(),
        tags: vec![],
    };

    let json = serde_json::to_string_pretty(&empty_cache)?;
    fs::write(dest_path, json)?;
    Ok(())
}
