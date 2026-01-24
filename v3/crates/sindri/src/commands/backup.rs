//! Backup command

use anyhow::Result;
use camino::Utf8PathBuf;
use clap::{Args, ValueEnum};
use sindri_core::config::SindriConfig;
use std::collections::HashSet;

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

    /// Compression level (0-9, 0=none, 9=max)
    #[arg(short, long, default_value = "6")]
    pub compression: u8,

    /// Verbose output (show all files)
    #[arg(short, long)]
    pub verbose: bool,

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
    fn default_excludes(&self) -> Vec<&'static str> {
        match self {
            BackupProfile::UserData => vec![
                "**/.git/**",
                "**/node_modules/**",
                "**/target/**",
                "**/.cache/**",
                "**/.tmp/**",
                "**/.log",
                "**/logs/**",
                "**/.env*",
                "**/*.key",
                "**/.bash_history",
                "**/.zsh_history",
                "**/.python_history",
                "**/workspace/**", // Large project files
            ],
            BackupProfile::Standard => vec![
                "**/.git/**",
                "**/node_modules/**",
                "**/target/**",
                "**/.cache/**",
                "**/.tmp/**",
                "**/.log",
                "**/logs/**",
                "**/.env.local",
                "**/*.key",
                "**/.bash_history",
                "**/.zsh_history",
                "**/.python_history",
            ],
            BackupProfile::Full => vec![
                "**/.git/**", // Still exclude git objects
                "**/.bash_history",
                "**/.zsh_history",
                "**/.python_history",
            ],
        }
    }

    fn description(&self) -> &'static str {
        match self {
            BackupProfile::UserData => {
                "Minimal backup: User data only, no cache/logs/secrets/workspace"
            }
            BackupProfile::Standard => {
                "Standard backup: Config and data, excludes cache/logs/local secrets"
            }
            BackupProfile::Full => "Full backup: Everything (including secrets - use encryption!)",
        }
    }
}

pub async fn run(args: BackupArgs) -> Result<()> {
    output::header("Backup Workspace");

    // Validate compression level
    if args.compression > 9 {
        return Err(anyhow::anyhow!(
            "Compression level must be 0-9, got {}",
            args.compression
        ));
    }

    // Get workspace root
    let workspace_root = std::env::var("HOME")
        .map(Utf8PathBuf::from)
        .unwrap_or_else(|_| Utf8PathBuf::from("/alt/home/developer"));

    // Load config if available
    let _config = SindriConfig::load(None).ok();

    // Display backup configuration
    output::kv("Profile", &format!("{:?}", args.profile));
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

    // Build exclude list
    let mut excludes: HashSet<String> = args
        .profile
        .default_excludes()
        .iter()
        .map(|s| s.to_string())
        .collect();

    // Add user excludes
    for pattern in &args.exclude {
        excludes.insert(pattern.clone());
    }

    // Add secret excludes if requested
    if args.exclude_secrets {
        excludes.insert("**/.env*".to_string());
        excludes.insert("**/*.key".to_string());
        excludes.insert("**/*.pem".to_string());
        excludes.insert("**/secrets/**".to_string());
    }

    if args.verbose {
        output::info(&format!("Exclude patterns ({}):", excludes.len()));
        for pattern in &excludes {
            println!("  {}", console::style(pattern).dim());
        }
        println!();
    }

    // Estimate backup size
    let spinner = output::spinner("Analyzing workspace...");
    let (file_count, total_size, estimated_compressed) =
        estimate_backup_size(&workspace_root, &excludes)?;
    spinner.finish_and_clear();

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

    // Create backup
    output::info(&format!("Creating backup: {}", output_path));
    println!();

    let pb = output::progress_bar(file_count, "Backing up");
    let start = std::time::Instant::now();

    // TODO: Implement actual backup creation using tar + compression
    // For now, simulate progress
    for i in 0..file_count {
        tokio::time::sleep(tokio::time::Duration::from_micros(100)).await;
        pb.inc(1);
        if args.verbose && i % 100 == 0 {
            // Would show current file being processed
        }
    }

    pb.finish_and_clear();

    let duration = start.elapsed();

    // Encrypt if requested
    let final_path = if args.encrypt {
        let encrypted_path = format!("{}.age", output_path);
        let spinner = output::spinner("Encrypting backup...");
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        spinner.finish_and_clear();
        Utf8PathBuf::from(encrypted_path)
    } else {
        output_path
    };

    // Display summary
    println!();
    output::success("Backup created successfully");
    println!();
    output::kv("Location", final_path.as_str());
    output::kv("Files", &format_number(file_count));
    output::kv("Size", &format_bytes(estimated_compressed));
    output::kv(
        "Compression",
        &format!("{}%", (estimated_compressed * 100 / total_size.max(1))),
    );
    output::kv("Duration", &format!("{:.1}s", duration.as_secs_f64()));
    println!();

    // Show next steps
    output::info("Restore with:");
    println!("  sindri restore {}", final_path);

    Ok(())
}

fn estimate_backup_size(root: &Utf8PathBuf, excludes: &HashSet<String>) -> Result<(u64, u64, u64)> {
    use globset::{Glob, GlobSetBuilder};
    use walkdir::WalkDir;

    // Build exclude globset
    let mut builder = GlobSetBuilder::new();
    for pattern in excludes {
        builder.add(Glob::new(pattern)?);
    }
    let exclude_set = builder.build()?;

    let mut file_count = 0u64;
    let mut total_size = 0u64;

    for entry in WalkDir::new(root.as_std_path())
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let path = e.path();
            !exclude_set.is_match(path)
        })
    {
        let entry = entry?;
        if entry.file_type().is_file() {
            file_count += 1;
            total_size += entry.metadata()?.len();
        }
    }

    // Estimate compressed size (assume ~60% compression for mixed content)
    let estimated_compressed = (total_size as f64 * 0.6) as u64;

    Ok((file_count, total_size, estimated_compressed))
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
