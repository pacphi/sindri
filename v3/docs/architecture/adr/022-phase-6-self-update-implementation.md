# ADR 022: Phase 6 Self-Update Implementation

**Status**: Accepted
**Date**: 2026-01-22
**Deciders**: Core Team
**Related**: [ADR-001: Rust Migration Workspace Architecture](001-rust-migration-workspace-architecture.md), [ADR-010: GitHub Extension Distribution](010-github-extension-distribution.md), [ADR-011: Multi-Method Extension Installation](011-multi-method-extension-installation.md), [ADR-012: Registry Manifest Dual-State Architecture](012-registry-manifest-dual-state-architecture.md), [ADR-021: Bifurcated CI/CD Pipeline](021-bifurcated-ci-cd-v2-v3.md), [Rust Migration Plan](../../planning/rust-cli-migration-v3.md#7-upgrade-system)

## Context

The Sindri CLI v3 Rust migration introduces versioned binary distribution via GitHub releases. Unlike v2's manual bash script updates, v3 requires a robust self-update mechanism that addresses several critical challenges:

### Current Manual Update Process Problems

In v2 (bash-based CLI), users faced:
1. **Manual Download**: Users must manually download new scripts from GitHub
2. **No Version Discovery**: No way to know if updates are available
3. **No Rollback**: Failed updates leave broken state
4. **Extension Compatibility Unknown**: No visibility into whether installed extensions work with new CLI versions
5. **Multi-File Updates**: Updating requires replacing multiple scripts across different directories
6. **No Progress Indication**: Large downloads provide no feedback
7. **Security Risks**: No checksum verification, potential for incomplete downloads

### Requirements from Phase 6 Specification

The [Rust Migration Plan Phase 6](../../planning/rust-cli-migration-v3.md#phase-6-self-update-weeks-18-19) defines these requirements:

**Commands**:
- `sindri upgrade` - Perform upgrade to latest version
- `sindri upgrade --check` - Check for updates without installing
- `sindri upgrade --list` - List available versions
- `sindri upgrade --version <v>` - Upgrade to specific version
- `sindri upgrade --compat <v>` - Show extension compatibility for version
- `sindri upgrade --allow-downgrade` - Allow downgrading to older versions
- `sindri upgrade --yes` - Skip confirmation prompts
- `sindri upgrade --stable` - Show only stable releases (exclude pre-releases)

**Core Features**:
1. **GitHub Releases Integration**: Fetch releases via GitHub API
2. **Binary Download**: Platform-specific asset selection and download
3. **Checksum Verification**: SHA256 verification for security
4. **Extension Compatibility**: Block upgrades if extensions incompatible
5. **Automatic Rollback**: Restore previous binary on failure
6. **Progress Indication**: Show download progress to users

### User Experience Goals

1. **Single Command Upgrade**: `sindri upgrade` should "just work"
2. **Safe by Default**: Never leave CLI in broken state
3. **Transparent**: Show what's changing, why, and impact on extensions
4. **Respectful**: No automatic background updates, user controls when to check
5. **Informative**: Show changelog preview before upgrading
6. **Fast**: Cache update checks, resumable downloads

## Decision

We implement the `sindri-update` crate with six key architectural decisions.

### a) Auto-Rollback Strategy

**Decision**: Implement automatic rollback with timestamped backup and verification.

**Architecture**:
```rust
// crates/sindri-update/src/rollback.rs

use std::fs;
use std::path::{Path, PathBuf};
use chrono::Utc;

pub struct RollbackManager {
    binary_path: PathBuf,
}

impl RollbackManager {
    /// Backup current binary with timestamp
    pub fn backup_current(&self) -> Result<PathBuf> {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let backup_path = self.binary_path
            .with_extension(format!("backup_{}", timestamp));

        fs::copy(&self.binary_path, &backup_path)?;

        // Verify backup
        let original_size = fs::metadata(&self.binary_path)?.len();
        let backup_size = fs::metadata(&backup_path)?.len();

        if original_size != backup_size {
            fs::remove_file(&backup_path)?;
            bail!("Backup verification failed: size mismatch");
        }

        Ok(backup_path)
    }

    /// Test new binary by running --version
    pub async fn verify_binary(&self, path: &Path) -> Result<()> {
        // Make executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(path, perms)?;
        }

        // Test with --version flag
        let output = tokio::process::Command::new(path)
            .arg("--version")
            .timeout(Duration::from_secs(5))
            .output()
            .await?;

        if !output.status.success() {
            bail!("Binary verification failed: --version returned non-zero exit code");
        }

        // Parse version string to ensure valid format
        let version_str = String::from_utf8(output.stdout)?;
        if !version_str.contains("sindri") {
            bail!("Binary verification failed: invalid version output");
        }

        Ok(())
    }

    /// Perform atomic rollback to backup
    pub fn rollback(&self, backup_path: &Path) -> Result<()> {
        // Use atomic rename on Unix, copy+delete on Windows
        #[cfg(unix)]
        {
            std::fs::rename(backup_path, &self.binary_path)?;
        }

        #[cfg(windows)]
        {
            std::fs::copy(backup_path, &self.binary_path)?;
            std::fs::remove_file(backup_path)?;
        }

        Ok(())
    }

    /// Clean up old backups (keep last 3)
    pub fn cleanup_old_backups(&self) -> Result<()> {
        let parent = self.binary_path.parent()
            .ok_or_else(|| anyhow!("Cannot determine parent directory"))?;

        let mut backups: Vec<_> = fs::read_dir(parent)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("sindri.backup_")
            })
            .collect();

        // Sort by modified time (newest first)
        backups.sort_by_key(|e| {
            e.metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        });
        backups.reverse();

        // Remove all but last 3
        for backup in backups.iter().skip(3) {
            let _ = fs::remove_file(backup.path());
        }

        Ok(())
    }
}
```

**Update Workflow**:
```rust
pub async fn update_to(&self, target: &Version) -> Result<()> {
    let rollback_mgr = RollbackManager::new()?;

    // 1. Backup current binary
    let backup_path = rollback_mgr.backup_current()
        .context("Failed to backup current binary")?;

    // 2. Download new binary
    let temp_binary = self.download_binary(target).await?;

    // 3. Verify checksum
    self.verify_checksum(&temp_binary, target).await?;

    // 4. Test new binary
    if let Err(e) = rollback_mgr.verify_binary(&temp_binary).await {
        eprintln!("New binary verification failed: {}", e);
        eprintln!("Rolling back to previous version...");
        rollback_mgr.rollback(&backup_path)?;
        bail!("Update failed and rolled back");
    }

    // 5. Replace current binary atomically
    self.replace_binary(&temp_binary)?;

    // 6. Verify installation
    if let Err(e) = rollback_mgr.verify_binary(&self.binary_path).await {
        eprintln!("Installation verification failed: {}", e);
        eprintln!("Rolling back to previous version...");
        rollback_mgr.rollback(&backup_path)?;
        bail!("Installation failed and rolled back");
    }

    // 7. Cleanup old backups (keep last 3)
    rollback_mgr.cleanup_old_backups()?;

    println!("Successfully updated to sindri {}", target);
    println!("Previous version backed up to: {}", backup_path.display());

    Ok(())
}
```

**Reasoning**: Safety and reliability are paramount for self-update. Users must never be left with a broken CLI. The timestamped backup approach provides:
- **Safety**: Always have working binary to fall back to
- **Transparency**: Users know where backup is located
- **Recoverability**: Multiple backups allow recovery from various failure scenarios
- **Verification**: Testing `--version` ensures binary is executable and valid

### b) Extension Compatibility Blocking

**Decision**: Block upgrades if installed extensions are incompatible with target CLI version, require `--force` to override.

**Architecture**:
```rust
// crates/sindri-update/src/compatibility.rs

use semver::{Version, VersionReq};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct CompatibilityMatrix {
    /// Maps CLI version to extension compatibility requirements
    pub cli_versions: HashMap<String, CliVersionCompat>,
}

#[derive(Debug, Deserialize)]
pub struct CliVersionCompat {
    /// Minimum extension schema version supported
    pub min_extension_schema: String,
    /// Maximum extension schema version supported
    pub max_extension_schema: String,
    /// Breaking changes in this CLI version
    pub breaking_changes: Vec<String>,
    /// Features deprecated (but still work)
    pub deprecated_features: Vec<String>,
    /// Features removed (no longer work)
    pub removed_features: Vec<String>,
}

#[derive(Debug)]
pub struct CompatibilityResult {
    pub is_fully_compatible: bool,
    pub incompatible_extensions: Vec<IncompatibleExtension>,
    pub warnings: Vec<String>,
}

#[derive(Debug)]
pub struct IncompatibleExtension {
    pub name: String,
    pub current_schema: String,
    pub required_range: String,
    pub reason: String,
}

impl SindriUpdater {
    /// Check if installed extensions are compatible with target version
    pub async fn check_extension_compatibility(
        &self,
        target: &Version,
    ) -> Result<CompatibilityResult> {
        // Fetch compatibility matrix from GitHub release assets
        let matrix = self.fetch_compatibility_matrix(target).await?;

        // Load installed extensions from manifest
        let manifest_path = dirs::home_dir()
            .ok_or_else(|| anyhow!("Cannot determine home directory"))?
            .join(".sindri")
            .join("manifest.yaml");

        let installed = if manifest_path.exists() {
            self.load_manifest(&manifest_path)?
        } else {
            Vec::new()
        };

        // Get compatibility requirements for target version
        let target_compat = matrix.cli_versions
            .get(&target.to_string())
            .ok_or_else(|| anyhow!("No compatibility info for version {}", target))?;

        let min = Version::parse(&target_compat.min_extension_schema)?;
        let max = Version::parse(&target_compat.max_extension_schema)?;
        let range = VersionReq::parse(&format!(">={}, <={}", min, max))?;

        // Check each installed extension
        let mut incompatible = Vec::new();
        for ext in installed {
            let schema_ver = Version::parse(&ext.schema_version)?;

            if !range.matches(&schema_ver) {
                incompatible.push(IncompatibleExtension {
                    name: ext.name.clone(),
                    current_schema: ext.schema_version.clone(),
                    required_range: format!("{} - {}", min, max),
                    reason: if schema_ver < min {
                        "Extension schema too old (CLI removed support)".to_string()
                    } else {
                        "Extension schema too new (CLI doesn't support yet)".to_string()
                    },
                });
            }
        }

        let mut warnings = Vec::new();

        // Add warnings for deprecated features
        if !target_compat.deprecated_features.is_empty() {
            warnings.push(format!(
                "Deprecated features (still work, but will be removed in future): {}",
                target_compat.deprecated_features.join(", ")
            ));
        }

        // Add warnings for removed features
        if !target_compat.removed_features.is_empty() {
            warnings.push(format!(
                "Removed features (no longer work): {}",
                target_compat.removed_features.join(", ")
            ));
        }

        Ok(CompatibilityResult {
            is_fully_compatible: incompatible.is_empty(),
            incompatible_extensions: incompatible,
            warnings,
        })
    }

    /// Fetch compatibility matrix from GitHub release assets
    async fn fetch_compatibility_matrix(
        &self,
        version: &Version,
    ) -> Result<CompatibilityMatrix> {
        let url = format!(
            "https://github.com/{}/{}/releases/download/v{}/compatibility-matrix.yaml",
            REPO_OWNER, REPO_NAME, version
        );

        let response = reqwest::get(&url).await?;

        if !response.status().is_success() {
            bail!("Failed to fetch compatibility matrix: HTTP {}", response.status());
        }

        let content = response.text().await?;
        let matrix: CompatibilityMatrix = serde_yaml::from_str(&content)
            .context("Failed to parse compatibility matrix")?;

        Ok(matrix)
    }
}
```

**Upgrade Command with Blocking**:
```rust
// crates/sindri/src/commands/upgrade.rs

async fn do_upgrade(updater: &SindriUpdater, args: &UpgradeArgs) -> Result<()> {
    let current = updater.current_version();

    let target = match &args.version {
        Some(v) => semver::Version::parse(v)?,
        None => updater.get_latest_version(args.stable).await?,
    };

    // Check if already at target version
    if target == current {
        println!("Already at version {}", current);
        return Ok(());
    }

    // Prevent downgrade unless explicitly allowed
    if target < current && !args.allow_downgrade {
        bail!(
            "Target version {} is older than current {}.\n\
             Use --allow-downgrade to proceed.",
            target, current
        );
    }

    // Check extension compatibility
    let compat = updater.check_extension_compatibility(&target).await?;

    if !compat.is_fully_compatible {
        eprintln!("\nWARNING: Incompatible extensions detected:\n");

        for ext in &compat.incompatible_extensions {
            eprintln!("  - {} (schema {}) - {}",
                     ext.name, ext.current_schema, ext.reason);
            eprintln!("    Required schema range: {}", ext.required_range);
        }

        eprintln!("\nThese extensions may not work with CLI version {}.", target);
        eprintln!("Consider updating extensions before upgrading CLI.\n");

        // Block upgrade unless --force or --yes provided
        if !args.yes && !args.force {
            let proceed = dialoguer::Confirm::new()
                .with_prompt("Continue with upgrade despite incompatibility?")
                .default(false)
                .interact()?;

            if !proceed {
                println!("Upgrade cancelled.");
                return Ok(());
            }
        }
    }

    // Show warnings
    if !compat.warnings.is_empty() {
        println!("\nNotes:");
        for warning in &compat.warnings {
            println!("  - {}", warning);
        }
        println!();
    }

    // Show upgrade plan
    println!("Upgrading sindri: {} -> {}", current, target);

    // Perform update
    let pb = indicatif::ProgressBar::new_spinner();
    pb.set_message("Downloading...");
    pb.enable_steady_tick(Duration::from_millis(100));

    updater.update_to(&target).await?;

    pb.finish_with_message(format!("Upgraded to sindri {}", target));

    Ok(())
}
```

**Reasoning**: Preventing broken state is critical. If an extension requires CLI features that no longer exist, it will fail at runtime. Better to block the upgrade and inform the user. The `--force` flag provides an escape hatch for advanced users who understand the risks.

### c) Update Check Strategy

**Decision**: No automatic background checks. Only check when user runs command. Cache results for 1 hour.

**Architecture**:
```rust
// crates/sindri-update/src/cache.rs

use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateCache {
    pub latest_version: String,
    pub checked_at: SystemTime,
    pub releases: Vec<CachedRelease>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CachedRelease {
    pub version: String,
    pub date: String,
    pub prerelease: bool,
}

impl UpdateCache {
    const CACHE_DURATION: Duration = Duration::from_secs(3600); // 1 hour

    fn cache_path() -> Result<PathBuf> {
        let path = dirs::cache_dir()
            .ok_or_else(|| anyhow!("Cannot determine cache directory"))?
            .join("sindri")
            .join("update-cache.json");

        Ok(path)
    }

    pub fn load() -> Result<Option<Self>> {
        let path = Self::cache_path()?;

        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path)?;
        let cache: UpdateCache = serde_json::from_str(&content)?;

        // Check if cache is expired
        let age = SystemTime::now()
            .duration_since(cache.checked_at)
            .unwrap_or(Duration::MAX);

        if age > Self::CACHE_DURATION {
            return Ok(None); // Expired
        }

        Ok(Some(cache))
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::cache_path()?;

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;

        Ok(())
    }
}

impl SindriUpdater {
    pub async fn get_latest_version(&self, stable_only: bool) -> Result<Version> {
        // Try cache first
        if let Some(cache) = UpdateCache::load()? {
            let version = Version::parse(&cache.latest_version)?;
            return Ok(version);
        }

        // Cache miss or expired - fetch from GitHub
        let releases = self.fetch_releases_from_github(10).await?;

        let latest = releases.iter()
            .filter(|r| !stable_only || !r.prerelease)
            .map(|r| &r.version)
            .max()
            .ok_or_else(|| anyhow!("No releases found"))?
            .clone();

        // Update cache
        let cache = UpdateCache {
            latest_version: latest.to_string(),
            checked_at: SystemTime::now(),
            releases: releases.iter()
                .map(|r| CachedRelease {
                    version: r.version.to_string(),
                    date: r.date.clone(),
                    prerelease: r.prerelease,
                })
                .collect(),
        };

        let _ = cache.save(); // Ignore errors

        Ok(latest)
    }
}
```

**Reasoning**:
- **Privacy**: No telemetry or background network calls. User controls when to check.
- **Performance**: Reduces GitHub API calls, stays under rate limits.
- **User Experience**: Fast response for repeat checks within 1 hour window.
- **Offline Friendly**: Cache allows showing last known releases when offline.

### d) Default Upgrade Behavior

**Decision**: Check and prompt before installing. Show changelog preview. Require confirmation unless `--yes`.

**Architecture**:
```rust
// crates/sindri/src/commands/upgrade.rs

pub async fn run(args: UpgradeArgs) -> Result<()> {
    let updater = SindriUpdater::new()?;

    if args.list {
        return list_versions(&updater, args.stable).await;
    }

    if args.check {
        return check_for_updates(&updater).await;
    }

    if let Some(ver) = &args.compatibility {
        return show_compatibility(&updater, ver).await;
    }

    // Default: perform upgrade with prompts
    do_upgrade(&updater, &args).await
}

async fn check_for_updates(updater: &SindriUpdater) -> Result<()> {
    let current = updater.current_version();

    let spinner = indicatif::ProgressBar::new_spinner();
    spinner.set_message("Checking for updates...");
    spinner.enable_steady_tick(Duration::from_millis(100));

    let latest = updater.get_latest_version(true).await?;

    spinner.finish_and_clear();

    if latest > current {
        println!("Update available: {} -> {}", current, latest);
        println!("\nRun `sindri upgrade` to install.");

        // Show changelog preview
        if let Ok(changelog) = updater.fetch_changelog(&latest).await {
            println!("\nChangelog preview:");
            println!("{}", truncate_changelog(&changelog, 10));
        }
    } else {
        println!("Already at latest version: {}", current);
    }

    Ok(())
}

async fn do_upgrade(updater: &SindriUpdater, args: &UpgradeArgs) -> Result<()> {
    let current = updater.current_version();

    let target = match &args.version {
        Some(v) => semver::Version::parse(v)?,
        None => updater.get_latest_version(args.stable).await?,
    };

    if target == current {
        println!("Already at version {}", current);
        return Ok(());
    }

    // Show what will change
    println!("Sindri upgrade plan:");
    println!("  Current version: {}", current);
    println!("  Target version:  {}", target);
    println!();

    // Show changelog
    if let Ok(changelog) = updater.fetch_changelog(&target).await {
        println!("Changelog:");
        println!("{}", indent_text(&changelog, "  "));
        println!();
    }

    // Extension compatibility check
    let compat = updater.check_extension_compatibility(&target).await?;

    if !compat.is_fully_compatible {
        // Show compatibility warnings (see section b above)
        // ...
    }

    // Prompt for confirmation unless --yes
    if !args.yes {
        let proceed = dialoguer::Confirm::new()
            .with_prompt("Proceed with upgrade?")
            .default(true)
            .interact()?;

        if !proceed {
            println!("Upgrade cancelled.");
            return Ok(());
        }
    }

    // Perform update
    updater.update_to(&target).await?;

    println!("\nSuccessfully upgraded to sindri {}", target);

    Ok(())
}

fn truncate_changelog(text: &str, max_lines: usize) -> String {
    let lines: Vec<_> = text.lines().take(max_lines).collect();
    let mut result = lines.join("\n");

    if text.lines().count() > max_lines {
        result.push_str("\n... (see full changelog in release notes)");
    }

    result
}

fn indent_text(text: &str, indent: &str) -> String {
    text.lines()
        .map(|line| format!("{}{}", indent, line))
        .collect::<Vec<_>>()
        .join("\n")
}
```

**Reasoning**: Users should understand what they're upgrading to and why. The changelog preview gives context. Requiring confirmation prevents accidental upgrades. The `--yes` flag enables automation for scripts/CI.

### e) Binary Download

**Decision**: Platform-specific asset selection, SHA256 checksum verification, resumable downloads, progress bars with `indicatif`.

**Architecture**:
```rust
// crates/sindri-update/src/download.rs

use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use sha2::{Sha256, Digest};
use std::fs::File;
use std::io::Write;

impl SindriUpdater {
    /// Download binary for target version and current platform
    pub async fn download_binary(&self, version: &Version) -> Result<PathBuf> {
        // Detect platform
        let platform = self.detect_platform()?;

        // Fetch release assets
        let assets = self.fetch_release_assets(version).await?;

        // Find matching asset for platform
        let asset = assets.iter()
            .find(|a| a.name.contains(&platform))
            .ok_or_else(|| anyhow!(
                "No binary found for platform {} in release {}",
                platform, version
            ))?;

        // Download binary
        let temp_file = self.download_asset_with_progress(&asset.url).await?;

        // Verify checksum
        if let Some(checksum_url) = self.find_checksum_url(&assets, &asset.name) {
            self.verify_download(&temp_file, checksum_url).await?;
        }

        Ok(temp_file)
    }

    fn detect_platform(&self) -> Result<String> {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;

        let platform = match (os, arch) {
            ("linux", "x86_64") => "x86_64-unknown-linux-musl",
            ("linux", "aarch64") => "aarch64-unknown-linux-musl",
            ("macos", "x86_64") => "x86_64-apple-darwin",
            ("macos", "aarch64") => "aarch64-apple-darwin",
            ("windows", "x86_64") => "x86_64-pc-windows-msvc",
            _ => bail!("Unsupported platform: {}-{}", os, arch),
        };

        Ok(platform.to_string())
    }

    async fn download_asset_with_progress(&self, url: &str) -> Result<PathBuf> {
        let client = Client::new();
        let response = client.get(url).send().await?;

        let total_size = response.content_length()
            .ok_or_else(|| anyhow!("Missing Content-Length header"))?;

        // Create progress bar
        let pb = ProgressBar::new(total_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")?
                .progress_chars("#>-")
        );
        pb.set_message("Downloading sindri binary");

        // Download to temporary file
        let temp_path = tempfile::NamedTempFile::new()?.into_temp_path();
        let mut file = File::create(&temp_path)?;

        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk)?;

            downloaded += chunk.len() as u64;
            pb.set_position(downloaded);
        }

        pb.finish_with_message("Download complete");

        Ok(temp_path.to_path_buf())
    }

    async fn verify_download(
        &self,
        binary_path: &Path,
        checksum_url: &str,
    ) -> Result<()> {
        // Download checksum file
        let response = reqwest::get(checksum_url).await?;
        let checksum_content = response.text().await?;

        // Parse checksum (format: "sha256  filename")
        let expected_checksum = checksum_content
            .lines()
            .find(|line| line.contains("sindri"))
            .and_then(|line| line.split_whitespace().next())
            .ok_or_else(|| anyhow!("Failed to parse checksum file"))?;

        // Calculate actual checksum
        let mut file = File::open(binary_path)?;
        let mut hasher = Sha256::new();
        std::io::copy(&mut file, &mut hasher)?;
        let result = hasher.finalize();
        let actual_checksum = format!("{:x}", result);

        // Compare
        if actual_checksum != expected_checksum.to_lowercase() {
            bail!(
                "Checksum verification failed!\n\
                 Expected: {}\n\
                 Actual:   {}",
                expected_checksum,
                actual_checksum
            );
        }

        println!("Checksum verified: {}", expected_checksum);

        Ok(())
    }

    fn find_checksum_url(&self, assets: &[Asset], binary_name: &str) -> Option<String> {
        // Look for corresponding .sha256 file
        let checksum_name = format!("{}.sha256", binary_name);

        assets.iter()
            .find(|a| a.name == checksum_name)
            .map(|a| a.url.clone())
    }
}
```

**Reasoning**:
- **Security**: SHA256 verification prevents corrupted or tampered binaries.
- **User Experience**: Progress bars show download status, reducing uncertainty.
- **Platform Support**: Auto-detection ensures correct binary for user's system.
- **Reliability**: Resumable downloads handle network interruptions gracefully (via `reqwest` built-in retry).

### f) Compatibility Matrix

**Decision**: YAML file in repository root, maps CLI versions to extension version ranges, includes breaking changes, fetched from GitHub releases.

**Architecture**:

**File Format** (`compatibility-matrix.yaml` in repo root):
```yaml
# Sindri CLI Extension Compatibility Matrix
# This file tracks which extension schema versions work with each CLI version

cli_versions:
  "3.0.0":
    min_extension_schema: "1.0.0"
    max_extension_schema: "1.2.0"
    breaking_changes:
      - "Removed support for legacy install.method=shell (use script instead)"
      - "Changed secrets.provider.vault.path to secrets.provider.vault.mount_path"
    deprecated_features:
      - "providers.docker.legacy_networking (use providers.docker.network instead)"
    removed_features: []

  "3.1.0":
    min_extension_schema: "1.1.0"
    max_extension_schema: "1.3.0"
    breaking_changes:
      - "Removed providers.docker.legacy_networking"
    deprecated_features:
      - "install.apt.sudo_required (sudo auto-detected)"
    removed_features:
      - "providers.docker.legacy_networking"

  "3.2.0":
    min_extension_schema: "1.2.0"
    max_extension_schema: "2.0.0"
    breaking_changes:
      - "Extension schema v2.0.0 support added"
      - "Dropped schema v1.0.0 support (minimum now 1.2.0)"
    deprecated_features: []
    removed_features:
      - "install.apt.sudo_required"
      - "Extension schema v1.0.x and v1.1.x"
```

**CI Integration** (`.github/workflows/release.yml`):
```yaml
- name: Upload Compatibility Matrix
  run: |
    gh release upload "v${{ env.VERSION }}" \
      compatibility-matrix.yaml \
      --clobber
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

**Usage in Upgrade Command**:
```rust
// Already shown in section b, repeated here for completeness

impl SindriUpdater {
    async fn fetch_compatibility_matrix(
        &self,
        version: &Version,
    ) -> Result<CompatibilityMatrix> {
        let url = format!(
            "https://github.com/{}/{}/releases/download/v{}/compatibility-matrix.yaml",
            REPO_OWNER, REPO_NAME, version
        );

        let response = reqwest::get(&url).await?;

        if !response.status().is_success() {
            bail!("Failed to fetch compatibility matrix: HTTP {}", response.status());
        }

        let content = response.text().await?;
        let matrix: CompatibilityMatrix = serde_yaml::from_str(&content)
            .context("Failed to parse compatibility matrix")?;

        Ok(matrix)
    }
}
```

**Reasoning**: Centralized compatibility tracking prevents broken states. YAML format is human-readable and easy to maintain. Including it in releases ensures version-specific compatibility data is always available. Breaking changes documentation helps users understand upgrade impact.

## Consequences

### Positive

1. **Single Command Upgrade**: `sindri upgrade` provides seamless update experience
2. **Safe Rollback**: Automatic backup and verification prevents broken CLI state
3. **Extension Protection**: Compatibility checks prevent upgrades that break extensions
4. **Security**: SHA256 verification ensures binary integrity
5. **User Control**: No background updates, user decides when to check/upgrade
6. **Transparency**: Changelog preview and compatibility warnings keep users informed
7. **Fast**: 1-hour cache reduces GitHub API calls, stays under rate limits
8. **Cross-Platform**: Works on Linux, macOS, Windows with platform-specific binaries
9. **Progress Indication**: Progress bars improve UX for large downloads
10. **Downgrade Support**: `--allow-downgrade` enables rollback to older versions
11. **Automation Ready**: `--yes` flag enables scripted upgrades

### Negative

1. **Requires GitHub Connectivity**: Cannot upgrade without internet access
2. **Binary Size Grows**: Each backup adds ~12MB of disk space (mitigated by cleanup)
3. **Complexity**: 6 architectural decisions = ~1200 lines of code across multiple modules
4. **Compatibility Matrix Maintenance**: Must update YAML file with each release
5. **GitHub Rate Limits**: Heavy API usage could hit rate limits (mitigated by cache)
6. **Verification Time**: SHA256 checksum adds 1-2 seconds to upgrade time
7. **Dependency on `self_update` Crate**: Third-party dependency for core functionality

### Neutral

1. **Uses `self_update` Crate**: Established library (0.39) vs custom implementation
2. **1-Hour Cache Duration**: Trade-off between freshness and API calls
3. **3 Backup Limit**: Balance between safety and disk space
4. **Compatibility Matrix in Release Assets**: Must remember to include in CI

## Alternatives Considered

### 1. Manual Download Only

**Description**: No self-update command, users manually download from GitHub releases page.

**Pros**:
- No code complexity
- No dependency on `self_update` crate
- No GitHub API rate limit concerns

**Cons**:
- Poor user experience (requires multiple manual steps)
- No compatibility checking
- No rollback mechanism
- No progress indication
- Easy to install wrong platform binary

**Rejected**: Unacceptable UX regression from bash v2. Self-update is table stakes for modern CLI tools.

### 2. Automatic Background Updates

**Description**: CLI automatically checks for updates on every invocation, downloads in background.

**Pros**:
- Users always have latest version
- No manual intervention required
- Good for security patches

**Cons**:
- Surprising behavior (binary changes without user action)
- Privacy concerns (telemetry, background network calls)
- Can break workflows mid-session
- Requires background process management
- Complicates rollback (which version to roll back to?)

**Rejected**: Too aggressive. Violates principle of user control. Sindri is a developer tool, not a consumer app.

### 3. Package Manager Only Distribution

**Description**: Rely on homebrew, apt, chocolatey for updates. No self-update command.

**Pros**:
- Leverages existing package infrastructure
- Handles platform detection automatically
- Users familiar with package manager workflow

**Cons**:
- Platform-specific (homebrew macOS-only, apt Linux-only)
- Slower release cycle (waiting for package maintainers)
- No control over upgrade process
- Cannot show extension compatibility
- Not all users have package managers

**Rejected**: Package managers are complementary, not primary distribution. Need self-update for cross-platform support.

### 4. Always Upgrade Extensions

**Description**: When upgrading CLI, automatically upgrade all installed extensions to latest compatible versions.

**Pros**:
- Ensures compatibility
- Simplifies user workflow
- Prevents incompatibility errors

**Cons**:
- Extensions might fail to upgrade (network errors, no compatible version)
- Slow (must download all extensions)
- Surprising (extensions change without user request)
- What if extension upgrade fails? Roll back CLI too?

**Rejected**: Too risky. Better to warn user and let them decide. Extension upgrades should be separate command.

### 5. In-Place Binary Replacement (No Backup)

**Description**: Replace binary directly without creating backup.

**Pros**:
- Simpler implementation
- Faster (no backup copy time)
- Less disk space

**Cons**:
- No rollback on failure
- Risky (corrupted download leaves broken CLI)
- No recovery mechanism

**Rejected**: Too dangerous. Safety is paramount for self-update.

### 6. Custom Update Protocol (Not GitHub Releases)

**Description**: Host binaries on custom server with custom manifest format.

**Pros**:
- Full control over distribution
- Can add custom metadata
- Not subject to GitHub rate limits

**Cons**:
- Infrastructure cost (hosting, bandwidth)
- Complexity (custom server, custom protocol)
- Security (must sign binaries, manage keys)
- Less transparent (GitHub releases are public, auditable)

**Rejected**: GitHub releases are free, reliable, and transparent. No need for custom infrastructure.

## Compliance

- ✅ Implements all required commands from Phase 6 spec
- ✅ Supports all required flags (`--check`, `--list`, `--version`, `--compat`, `--allow-downgrade`, `--yes`, `--stable`)
- ✅ GitHub releases integration via `octocrab` and `self_update` crates
- ✅ Extension compatibility blocking from `~/.sindri/manifest.yaml`
- ✅ Automatic rollback with timestamped backups
- ✅ SHA256 checksum verification
- ✅ Progress indication with `indicatif`
- ✅ 1-hour update cache
- ✅ Compatibility matrix YAML format
- ✅ Platform-specific binary selection

## Notes

### Platform Support Matrix

| Platform | Triple | Binary Format | Self-Update Support |
|----------|--------|---------------|---------------------|
| Linux x86_64 | x86_64-unknown-linux-musl | ELF | ✅ Full |
| Linux ARM64 | aarch64-unknown-linux-musl | ELF | ✅ Full |
| macOS x86_64 | x86_64-apple-darwin | Mach-O | ✅ Full |
| macOS ARM64 | aarch64-apple-darwin | Mach-O | ✅ Full |
| Windows x86_64 | x86_64-pc-windows-msvc | PE32+ | ✅ Full |

All platforms support full self-update functionality including download, verification, and rollback.

### GitHub Rate Limit Considerations

GitHub API rate limits:
- **Unauthenticated**: 60 requests/hour per IP
- **Authenticated**: 5000 requests/hour per user

Our update check strategy stays well under limits:
- Checking for updates: 1 request (cached for 1 hour)
- Listing versions: 1 request (cached for 1 hour)
- Downloading binary: Direct asset download (no API)
- Fetching compatibility matrix: Direct asset download (no API)

**Worst case**: 2 API calls per hour = 48 calls per day (well under 60/hour limit).

**Mitigation**: If users provide `GITHUB_TOKEN` environment variable, use authenticated requests for 5000/hour limit.

### Security Implications of Self-Update

**Threats**:
1. **Man-in-the-Middle (MITM)**: Attacker intercepts download, serves malicious binary
2. **Compromised Release**: Attacker gains access to GitHub repo, publishes malicious release
3. **Checksum Bypass**: Attacker modifies both binary and checksum file

**Mitigations**:
1. **HTTPS Only**: All downloads use HTTPS, preventing MITM
2. **SHA256 Verification**: Checksum prevents corrupted/tampered binaries
3. **Binary Verification**: Test `--version` ensures binary is executable and valid
4. **Rollback**: Backup enables recovery from bad updates
5. **User Confirmation**: Prompt before replacing binary (unless `--yes`)
6. **Trusted Source**: GitHub releases with repository owner verification

**Future Enhancements**:
- **GPG Signature Verification**: Sign releases with GPG key
- **Code Signing**: Sign macOS/Windows binaries with developer certificate
- **Reproducible Builds**: Enable users to verify binary matches source

### Compatibility Matrix Updates

**Process**:
1. When planning breaking change, update `compatibility-matrix.yaml` in PR
2. Document breaking change in changelog
3. CI automatically uploads matrix to release assets
4. Users running `sindri upgrade --compat <version>` see impact before upgrading

**Example Breaking Change**:
```yaml
# PR that removes legacy_networking feature

cli_versions:
  "3.1.0":
    min_extension_schema: "1.1.0"
    max_extension_schema: "1.3.0"
    breaking_changes:
      - "Removed providers.docker.legacy_networking"
    deprecated_features: []
    removed_features:
      - "providers.docker.legacy_networking"
```

**Extension Authors** can test compatibility:
```bash
sindri upgrade --compat 3.1.0

# Output:
# CLI version 3.1.0 compatibility check:
#
# Installed extensions:
#   ✅ claude-code (schema 1.2.0) - compatible
#   ⚠️  legacy-docker (schema 1.0.0) - incompatible (schema too old)
#      Required: 1.1.0 - 1.3.0, Current: 1.0.0
#
# Breaking changes in 3.1.0:
#   - Removed providers.docker.legacy_networking
#
# Action required: Update legacy-docker extension to schema 1.1.0+
```

## Related Decisions

- [ADR-001: Rust Migration Workspace Architecture](001-rust-migration-workspace-architecture.md) - Defines `sindri-update` crate
- [ADR-010: GitHub Extension Distribution](010-github-extension-distribution.md) - Release workflow pattern
- [ADR-011: Multi-Method Extension Installation](011-multi-method-extension-installation.md) - Extension installation complexity
- [ADR-012: Registry Manifest Dual-State Architecture](012-registry-manifest-dual-state-architecture.md) - Source of installed extension data
- [ADR-021: Bifurcated CI/CD Pipeline](021-bifurcated-ci-cd-v2-v3.md) - Release workflow for v3 binaries
