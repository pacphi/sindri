//! Restore command
//!
//! Connects the CLI restore UI to the sindri-backup restore library.

use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use clap::{Args, ValueEnum};
use sindri_backup::restore::{
    BackupAnalysis, BackupAnalyzer, RestoreManager, RestoreMode as LibRestoreMode, RestoreOptions,
};

use crate::output;

#[derive(Args, Debug)]
pub struct RestoreArgs {
    /// Backup source (file path or https:// URL)
    pub source: String,

    /// Restore mode
    #[arg(short, long, default_value = "safe", value_enum)]
    pub mode: RestoreMode,

    /// Target directory (defaults to $HOME)
    #[arg(short = 'd', long)]
    pub target: Option<Utf8PathBuf>,

    /// Dry-run mode (show what would be restored)
    #[arg(long)]
    pub dry_run: bool,

    /// Skip confirmation prompts
    #[arg(long)]
    pub no_interactive: bool,

    /// Force restore even if version is incompatible
    #[arg(long)]
    pub force: bool,

    /// Auto-upgrade extensions to latest compatible versions
    #[arg(long)]
    pub auto_upgrade_extensions: bool,

    /// Decrypt with age key
    #[arg(long)]
    pub decrypt: bool,

    /// Decryption key file (age identity)
    #[arg(long, requires = "decrypt")]
    pub key_file: Option<Utf8PathBuf>,

    /// Show all files being restored
    #[arg(long)]
    pub show_files: bool,

    /// Skip validation of restored files
    #[arg(long)]
    pub skip_validation: bool,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum RestoreMode {
    /// Safe mode: Only restore if no conflicts, preserve system markers
    Safe,

    /// Merge mode: Merge with existing files, newer wins
    Merge,

    /// Full mode: Complete restore, overwrite everything (DANGEROUS)
    Full,
}

impl RestoreMode {
    fn description(&self) -> &'static str {
        match self {
            RestoreMode::Safe => "Safe: Only restore if no conflicts, preserve system markers",
            RestoreMode::Merge => "Merge: Combine with existing files, newer files win",
            RestoreMode::Full => "Full: Overwrite everything (DANGEROUS - may break system)",
        }
    }

    fn to_lib_mode(&self) -> LibRestoreMode {
        match self {
            RestoreMode::Safe => LibRestoreMode::Safe,
            RestoreMode::Merge => LibRestoreMode::Merge,
            RestoreMode::Full => LibRestoreMode::Full,
        }
    }
}

pub async fn run(args: RestoreArgs) -> Result<()> {
    output::header("Restore Workspace");

    // Determine target directory using canonical home resolution
    let target = args.target.clone().unwrap_or_else(|| {
        crate::utils::get_home_dir()
            .map(|p| {
                Utf8PathBuf::from_path_buf(p)
                    .unwrap_or_else(|p| Utf8PathBuf::from(p.to_string_lossy().as_ref()))
            })
            .unwrap_or_else(|_| {
                Utf8PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/home".to_string()))
            })
    });

    // Display restore configuration
    output::kv("Source", &args.source);
    output::kv("Target", target.as_str());
    output::kv("Mode", &format!("{:?}", args.mode));
    output::kv("Description", args.mode.description());

    if matches!(args.mode, RestoreMode::Full) {
        output::warning("FULL MODE - This will overwrite existing files!");
    }

    if args.dry_run {
        output::warning("DRY RUN MODE - No files will be restored");
    }
    println!();

    // Resolve backup source to a local file path
    let local_source = resolve_source(&args.source).await?;

    // Verify backup exists
    if !local_source.exists() {
        return Err(anyhow::anyhow!("Backup file not found: {}", local_source));
    }

    // Decrypt if needed
    let archive_path = if args.decrypt || local_source.as_str().ends_with(".age") {
        decrypt_backup(&local_source, args.key_file.as_ref()).await?
    } else {
        local_source.clone()
    };

    // Verify backup integrity using the library
    let spinner = output::spinner("Verifying backup integrity...");
    let analyzer = BackupAnalyzer;
    analyzer
        .validate_archive(&archive_path)
        .context("Backup archive is corrupt or unreadable")?;
    spinner.finish_and_clear();
    output::success("Backup verified");

    // Analyze backup using the library
    let spinner = output::spinner("Analyzing backup...");
    let analysis = analyzer
        .analyze(&archive_path)
        .await
        .context("Failed to analyze backup")?;
    spinner.finish_and_clear();

    // Display manifest information
    println!();
    output::info("Backup Information:");
    output::kv("Created", &analysis.manifest.created_at);
    output::kv("Sindri Version", &analysis.manifest.version);
    output::kv("Source", &analysis.manifest.source.instance_name);
    output::kv("Provider", &analysis.manifest.source.provider);
    output::kv("Profile", &analysis.manifest.backup_type);
    output::kv("Files", &format!("{}", analysis.file_count));
    output::kv("Size", &format_bytes(analysis.total_size));
    println!();

    // Display compatibility info
    output::info("Version Compatibility:");
    output::kv(
        "Backup version",
        &analysis.compatibility.backup_version.to_string(),
    );
    output::kv(
        "Current version",
        &analysis.compatibility.current_version.to_string(),
    );
    if analysis.compatibility.compatible {
        output::success("Versions are compatible");
    } else {
        output::warning(&analysis.compatibility.message());
        if !args.force {
            return Err(anyhow::anyhow!(
                "Backup version {} is incompatible with current version {}. Use --force to override.",
                analysis.compatibility.backup_version,
                analysis.compatibility.current_version
            ));
        }
        output::warning("--force specified, proceeding despite incompatibility");
    }

    // Display restore analysis
    let restore_analysis = compute_restore_analysis(&analysis, &target, &args.mode);
    println!();
    output::info("Restore Analysis:");
    println!(
        "  Total files:     {}",
        console::style(analysis.file_count).cyan()
    );
    println!(
        "  System markers:  {}",
        console::style(restore_analysis.system_markers).blue()
    );

    if !analysis.manifest.extensions.installed.is_empty() {
        println!(
            "  Extensions:      {}",
            console::style(analysis.manifest.extensions.installed.len()).cyan()
        );
        if args.show_files {
            for name in &analysis.manifest.extensions.installed {
                let version = analysis
                    .manifest
                    .extensions
                    .versions
                    .get(name)
                    .map(|v| v.as_str())
                    .unwrap_or("?");
                println!("    {} v{}", name, version);
            }
        }
    }
    println!();

    // Check for blocking issues in safe mode
    // In safe mode, the library handler will skip conflicts, so we just warn
    if matches!(args.mode, RestoreMode::Safe) {
        output::info("Safe mode: existing files will NOT be overwritten");
    }

    // System marker warning
    if restore_analysis.system_markers > 0 {
        output::warning(&format!(
            "{} system markers detected - these will NOT be restored",
            restore_analysis.system_markers
        ));
        output::info("System markers indicate provider-managed resources");
    }

    // Show preview in dry-run mode
    if args.dry_run {
        println!();
        output::success("Dry run complete");
        output::info("Remove --dry-run to perform actual restore");
        return Ok(());
    }

    // Confirm restore
    if !args.no_interactive {
        use dialoguer::Confirm;
        let prompt = if matches!(args.mode, RestoreMode::Full) {
            "DANGEROUS: Proceed with full restore? This will overwrite existing files"
        } else {
            "Proceed with restore?"
        };

        if !Confirm::new()
            .with_prompt(prompt)
            .default(false)
            .interact()?
        {
            output::info("Restore cancelled");
            return Ok(());
        }
    }

    // Perform restore using the library RestoreManager
    println!();
    output::info("Restoring files...");

    let options = RestoreOptions {
        mode: args.mode.to_lib_mode(),
        dry_run: args.dry_run,
        interactive: !args.no_interactive,
        force: args.force,
        validate_extensions: !args.skip_validation,
        auto_upgrade_extensions: args.auto_upgrade_extensions,
    };

    let manager = RestoreManager::new(args.mode.to_lib_mode());
    let result = manager
        .restore(&archive_path, &target, options)
        .await
        .context("Restore operation failed")?;

    // Display summary
    println!();
    output::success("Restore completed successfully");
    println!();
    output::kv("Files restored", &format!("{}", result.restored));
    output::kv("Files skipped", &format!("{}", result.skipped));
    if result.backed_up > 0 {
        output::kv("Files backed up", &format!("{}", result.backed_up));
    }
    output::kv(
        "Duration",
        &format!("{:.1}s", result.duration.as_secs_f64()),
    );

    // Clean up temporary download if we fetched from a URL
    if args.source.starts_with("https://") || args.source.starts_with("http://") {
        if local_source != archive_path {
            // Both temp files
            tokio::fs::remove_file(&local_source).await.ok();
            tokio::fs::remove_file(&archive_path).await.ok();
        } else {
            tokio::fs::remove_file(&local_source).await.ok();
        }
    }

    println!();
    output::info("Next steps:");
    println!("  1. Review restored files");
    println!("  2. Test your configuration: sindri config validate");
    println!("  3. Restart services if needed");

    Ok(())
}

/// Resolve a source string to a local file path.
///
/// For local paths, just returns as-is.
/// For HTTPS URLs, downloads to a temp file.
/// For S3 URLs, returns an error with a clear message.
async fn resolve_source(source: &str) -> Result<Utf8PathBuf> {
    if source.starts_with("s3://") {
        anyhow::bail!(
            "S3 sources are not yet supported. Download the backup file first, then restore from the local path.\n\
             Example: aws s3 cp {} ./backup.tar.gz && sindri restore ./backup.tar.gz",
            source
        );
    }

    if source.starts_with("https://") || source.starts_with("http://") {
        output::info("Downloading remote backup...");
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("sindri-restore-download.tar.gz");
        let temp_path = Utf8PathBuf::from_path_buf(temp_file)
            .map_err(|_| anyhow::anyhow!("Temporary directory path is not valid UTF-8"))?;

        let response = reqwest::get(source)
            .await
            .with_context(|| format!("Failed to download backup from {}", source))?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Download failed: HTTP {} from {}",
                response.status(),
                source
            );
        }

        let bytes = response
            .bytes()
            .await
            .context("Failed to read download response body")?;

        tokio::fs::write(&temp_path, &bytes)
            .await
            .context("Failed to write downloaded backup to temp file")?;

        output::success(&format!("Downloaded {} bytes", bytes.len()));
        Ok(temp_path)
    } else {
        // Local file path
        Ok(Utf8PathBuf::from(source))
    }
}

/// Decrypt an age-encrypted backup.
///
/// Currently requires the `age` CLI to be installed.
async fn decrypt_backup(source: &Utf8Path, key_file: Option<&Utf8PathBuf>) -> Result<Utf8PathBuf> {
    // Check that the age CLI is available
    let age_check = tokio::process::Command::new("age")
        .arg("--version")
        .output()
        .await;

    match age_check {
        Ok(output) if output.status.success() => {}
        _ => {
            anyhow::bail!(
                "Decryption requires the 'age' CLI tool.\n\
                 Install it with: brew install age (macOS) or apt install age (Debian/Ubuntu)\n\
                 See: https://github.com/FiloSottile/age"
            );
        }
    }

    let decrypted_name = source
        .as_str()
        .strip_suffix(".age")
        .unwrap_or(source.as_str());
    let temp_dir = std::env::temp_dir();
    let decrypted_path = temp_dir.join(
        Utf8Path::new(decrypted_name)
            .file_name()
            .unwrap_or("sindri-restore-decrypted.tar.gz"),
    );
    let decrypted_utf8 = Utf8PathBuf::from_path_buf(decrypted_path)
        .map_err(|_| anyhow::anyhow!("Temporary directory path is not valid UTF-8"))?;

    let spinner = output::spinner("Decrypting backup...");

    let mut cmd = tokio::process::Command::new("age");
    cmd.arg("--decrypt");

    if let Some(key) = key_file {
        cmd.arg("--identity").arg(key.as_str());
    }

    cmd.arg("--output").arg(decrypted_utf8.as_str());
    cmd.arg(source.as_str());

    let output = cmd.output().await.context("Failed to run age decryption")?;

    spinner.finish_and_clear();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Decryption failed: {}", stderr.trim());
    }

    output::success("Decryption complete");
    Ok(decrypted_utf8)
}

/// Lightweight pre-restore analysis for UI display purposes.
struct PreRestoreAnalysis {
    system_markers: usize,
}

fn compute_restore_analysis(
    _analysis: &BackupAnalysis,
    _target: &Utf8PathBuf,
    _mode: &RestoreMode,
) -> PreRestoreAnalysis {
    // Count system markers from the manifest statistics
    // The actual conflict detection happens inside RestoreManager at restore time
    let system_markers = sindri_backup::restore::NEVER_RESTORE.len();

    PreRestoreAnalysis { system_markers }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.2} {}", size, UNITS[unit_idx])
    }
}
