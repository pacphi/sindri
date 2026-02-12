//! Backup command

use anyhow::Result;
use camino::Utf8PathBuf;
use clap::{Args, ValueEnum};
use sindri_backup::{ArchiveBuilder, ArchiveConfig, SourceInfo};
use sindri_core::config::SindriConfig;

use crate::output;

#[derive(Args, Debug)]
pub struct BackupArgs {
    /// Output location (directory or file path)
    #[arg(short, long, default_value = ".")]
    pub output: String,

    /// Backup profile
    #[arg(short, long, default_value = "standard", value_enum)]
    pub profile: BackupProfile,

    /// Additional exclude patterns (glob)
    #[arg(long)]
    pub exclude: Vec<String>,

    /// Exclude all secrets (overrides profile)
    #[arg(long)]
    pub exclude_secrets: bool,

    /// Encrypt backup with age
    #[arg(long)]
    pub encrypt: bool,

    /// Encryption key file (age identity)
    #[arg(long, requires = "encrypt")]
    pub key_file: Option<Utf8PathBuf>,

    /// Dry-run mode (show what would be backed up)
    #[arg(long)]
    pub dry_run: bool,

    /// Compression level (1-9)
    #[arg(long, default_value = "6")]
    pub compression: u8,

    /// Show all files being backed up
    #[arg(long)]
    pub show_files: bool,

    /// Skip confirmation for large backups
    #[arg(short, long)]
    pub yes: bool,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum BackupProfile {
    /// Minimal backup (only user data, no cache/logs)
    UserData,

    /// Standard backup (includes config, excludes cache/logs/secrets)
    Standard,

    /// Full backup (everything, including secrets)
    Full,
}

impl BackupProfile {
    /// Convert CLI profile enum to the library profile type.
    fn to_lib_profile(&self) -> sindri_backup::BackupProfile {
        match self {
            BackupProfile::UserData => sindri_backup::BackupProfile::UserData,
            BackupProfile::Standard => sindri_backup::BackupProfile::Standard,
            BackupProfile::Full => sindri_backup::BackupProfile::Full,
        }
    }

    fn description(&self) -> &'static str {
        match self {
            BackupProfile::UserData => {
                "Projects, Claude data, git config (smallest, migration-focused)"
            }
            BackupProfile::Standard => "User data + shell/app configs (default, balanced)",
            BackupProfile::Full => "Everything except caches (largest, complete recovery)",
        }
    }
}

pub async fn run(args: BackupArgs) -> Result<()> {
    output::header("Backup Workspace");

    // Validate compression level
    if args.compression == 0 || args.compression > 9 {
        return Err(anyhow::anyhow!(
            "Compression level must be 1-9, got {}",
            args.compression
        ));
    }

    // Get workspace root
    let workspace_root = std::env::var("HOME")
        .map(Utf8PathBuf::from)
        .unwrap_or_else(|_| Utf8PathBuf::from("/alt/home/developer"));

    // Load config if available (used for instance/provider info)
    let config = SindriConfig::load(None).ok();

    // Build source info from config or defaults
    let source_info = build_source_info(&config);

    // Convert CLI profile to library profile
    let lib_profile = args.profile.to_lib_profile();

    // Display backup configuration
    output::kv("Profile", &format!("{}", lib_profile));
    output::kv("Description", args.profile.description());
    output::kv("Source", workspace_root.as_str());
    output::kv("Output", &args.output);
    if args.encrypt {
        output::kv("Encryption", "Enabled (age)");
    }
    if args.dry_run {
        output::warning("DRY RUN MODE - No backup will be created");
    }
    println!();

    // Build additional exclusion patterns
    let mut additional_excludes: Vec<String> = args.exclude.clone();

    // Add profile-specific exclusions
    additional_excludes.extend(lib_profile.excludes().into_iter().map(|s| s.to_string()));

    // Add secret excludes if requested
    if args.exclude_secrets {
        additional_excludes.push("**/.env*".to_string());
        additional_excludes.push("**/*.key".to_string());
        additional_excludes.push("**/*.pem".to_string());
        additional_excludes.push("**/secrets/**".to_string());
    }

    if args.show_files {
        output::info(&format!(
            "Additional exclude patterns ({}):",
            additional_excludes.len()
        ));
        for pattern in &additional_excludes {
            println!("  {}", console::style(pattern).dim());
        }
        println!();
    }

    // Estimate backup size using library's exclusion config
    let spinner = output::spinner("Analyzing workspace...");
    let (file_count, total_size) =
        estimate_backup_size(&workspace_root, lib_profile, &additional_excludes)?;
    spinner.finish_and_clear();

    // Estimate compressed size (~60% of original for mixed content)
    let estimated_compressed = (total_size as f64 * 0.6) as u64;

    // Display estimate
    output::info("Backup Estimate:");
    println!(
        "  Files:              {}",
        console::style(format_number(file_count)).cyan()
    );
    println!(
        "  Size:               {}",
        console::style(format_bytes(total_size)).cyan()
    );
    println!(
        "  Estimated archive:  {}",
        console::style(format_bytes(estimated_compressed)).cyan()
    );
    println!(
        "  Compression ratio:  {}%",
        console::style(estimated_compressed * 100 / total_size.max(1)).cyan()
    );
    println!();

    // Warn about large backups
    if total_size > 10 * 1024 * 1024 * 1024 && !args.yes {
        output::warning("Large backup detected (>10GB)");
    }

    // Warn about secrets in full backup
    if matches!(args.profile, BackupProfile::Full) && !args.encrypt {
        output::warning("Full backup includes secrets without encryption!");
        output::info("Consider using --encrypt to protect sensitive data");
        println!();
    }

    // Confirm if not dry-run
    if !args.dry_run && !args.yes {
        use dialoguer::Confirm;
        if !Confirm::new()
            .with_prompt("Proceed with backup?")
            .default(true)
            .interact()?
        {
            output::info("Backup cancelled");
            return Ok(());
        }
    }

    if args.dry_run {
        output::success("Dry run complete");
        return Ok(());
    }

    // Generate backup filename
    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let backup_name = format!("backup-{}.tar.gz", timestamp);
    let output_path = if args.output == "." || std::path::Path::new(&args.output).is_dir() {
        Utf8PathBuf::from(&args.output).join(&backup_name)
    } else {
        Utf8PathBuf::from(&args.output)
    };

    // Create backup using sindri-backup library
    output::info(&format!("Creating backup: {}", output_path));
    println!();

    let archive_config = ArchiveConfig::new(lib_profile, source_info)?
        .with_exclusions(additional_excludes)?
        .with_compression_level(args.compression as u32)
        .with_progress(!args.show_files); // Use library progress unless user wants file-level output

    let builder = ArchiveBuilder::new(archive_config);
    let result = builder
        .create(workspace_root.as_std_path(), output_path.as_std_path())
        .await?;

    // Encrypt if requested
    let final_path = if args.encrypt {
        encrypt_backup(&output_path, args.key_file.as_ref())?
    } else {
        output_path
    };

    // Display summary
    println!();
    output::success("Backup created successfully");
    println!();
    output::kv("Location", final_path.as_str());
    output::kv("Files", &format_number(result.file_count as u64));
    output::kv("Size", &format_bytes(result.size_bytes));
    if result.manifest.statistics.total_size_bytes > 0 {
        output::kv(
            "Compression",
            &format!("{}%", result.manifest.statistics.compression_percentage()),
        );
    }
    output::kv("Duration", &format!("{:.1}s", result.duration_seconds));
    println!();

    // Show next steps
    output::info("Restore with:");
    println!("  sindri restore {}", final_path);

    Ok(())
}

/// Build SourceInfo from loaded config or sensible defaults.
fn build_source_info(config: &Option<SindriConfig>) -> SourceInfo {
    let instance_name = config
        .as_ref()
        .map(|c| c.config.name.clone())
        .unwrap_or_else(|| "sindri".to_string());

    let provider = config
        .as_ref()
        .map(|c| c.config.deployment.provider.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let hostname = std::env::var("HOSTNAME")
        .or_else(|_| std::fs::read_to_string("/etc/hostname").map(|s| s.trim().to_string()))
        .unwrap_or_else(|_| "unknown".to_string());

    SourceInfo {
        instance_name,
        provider,
        hostname,
    }
}

/// Estimate backup size by scanning the workspace with the library's exclusion config.
fn estimate_backup_size(
    root: &Utf8PathBuf,
    profile: sindri_backup::BackupProfile,
    additional_excludes: &[String],
) -> Result<(u64, u64)> {
    use sindri_backup::ExclusionConfig;
    use walkdir::WalkDir;

    let exclusions = ExclusionConfig::new(additional_excludes.to_vec())?;

    // Get include paths for the profile
    let include_paths = match profile.includes() {
        Some(paths) => paths,
        None => vec![std::path::PathBuf::from(".")],
    };

    let mut file_count = 0u64;
    let mut total_size = 0u64;

    for include_path in include_paths {
        let full_path = root.as_std_path().join(&include_path);
        if !full_path.exists() {
            continue;
        }

        for entry in WalkDir::new(&full_path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                let rel_path = e
                    .path()
                    .strip_prefix(root.as_std_path())
                    .unwrap_or(e.path());
                !exclusions.should_exclude(rel_path)
            })
        {
            let entry = entry?;
            if entry.file_type().is_file() {
                let rel_path = entry
                    .path()
                    .strip_prefix(root.as_std_path())
                    .unwrap_or(entry.path());
                if !exclusions.should_exclude(rel_path) {
                    file_count += 1;
                    total_size += entry.metadata().map(|m| m.len()).unwrap_or(0);
                }
            }
        }
    }

    Ok((file_count, total_size))
}

/// Encrypt a backup archive using the `age` CLI tool.
fn encrypt_backup(
    archive_path: &Utf8PathBuf,
    key_file: Option<&Utf8PathBuf>,
) -> Result<Utf8PathBuf> {
    let encrypted_path = Utf8PathBuf::from(format!("{}.age", archive_path));

    // Check if age CLI is available
    let age_cmd = which::which("age").or_else(|_| which::which("rage"));

    match age_cmd {
        Ok(age_bin) => {
            let spinner = output::spinner("Encrypting backup...");

            let mut cmd = std::process::Command::new(&age_bin);
            cmd.arg("-o").arg(encrypted_path.as_str());

            if let Some(key) = key_file {
                cmd.arg("-i").arg(key.as_str());
            } else {
                // Use passphrase-based encryption
                cmd.arg("-p");
            }

            cmd.arg(archive_path.as_str());

            let status = cmd
                .status()
                .map_err(|e| anyhow::anyhow!("Failed to run age encryption: {}", e))?;

            spinner.finish_and_clear();

            if !status.success() {
                // Clean up the unencrypted file is left to user
                return Err(anyhow::anyhow!(
                    "Encryption failed with exit code: {}",
                    status.code().unwrap_or(-1)
                ));
            }

            // Remove the unencrypted archive
            std::fs::remove_file(archive_path.as_std_path()).ok();

            Ok(encrypted_path)
        }
        Err(_) => Err(anyhow::anyhow!(
            "Encryption requires the 'age' CLI tool.\n\
                 Install with one of:\n  \
                 cargo install rage\n  \
                 brew install age\n  \
                 apt install age\n\n\
                 The unencrypted backup has been saved to: {}",
            archive_path
        )),
    }
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

fn format_number(n: u64) -> String {
    n.to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect::<Vec<_>>()
        .join(",")
}
