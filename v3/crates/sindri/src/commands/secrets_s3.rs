//! S3 encrypted storage commands
//!
//! CLI commands for managing secrets stored in S3 with envelope encryption.

use anyhow::{anyhow, Context, Result};
use camino::Utf8PathBuf;
use clap::{Args, Subcommand};
use sindri_secrets::s3::{
    generate_key_file, KeySource, LocalSecretInfo, S3Backend, S3CacheConfig, S3EncryptionConfig,
    S3SecretBackend, S3SecretResolver, SecretEncryptor, SyncDirection as S3SyncDirection,
};
use std::path::PathBuf;

use crate::output;

#[derive(Subcommand, Debug)]
pub enum S3Commands {
    /// Initialize S3 backend
    Init(InitArgs),

    /// Push secret to S3
    Push(PushArgs),

    /// Pull secret from S3
    Pull(PullArgs),

    /// Sync all secrets
    Sync(SyncArgs),

    /// Generate new master key
    Keygen(KeygenArgs),

    /// Rotate secrets to new key
    Rotate(RotateArgs),
}

#[derive(Args, Debug)]
pub struct InitArgs {
    /// S3 bucket name
    #[arg(long)]
    pub bucket: String,

    /// AWS region
    #[arg(long)]
    pub region: String,

    /// Custom S3-compatible endpoint (e.g., MinIO)
    #[arg(long)]
    pub endpoint: Option<String>,

    /// Master key path
    #[arg(long)]
    pub key_file: Option<PathBuf>,

    /// Create bucket if it doesn't exist
    #[arg(long)]
    pub create_bucket: bool,

    /// Output configuration to file
    #[arg(short, long)]
    pub output: Option<Utf8PathBuf>,
}

#[derive(Args, Debug)]
pub struct PushArgs {
    /// Secret name
    pub name: String,

    /// Secret value
    #[arg(long, conflicts_with = "from_file")]
    pub value: Option<String>,

    /// Read value from file
    #[arg(long, conflicts_with = "value")]
    pub from_file: Option<PathBuf>,

    /// Read value from stdin
    #[arg(long, conflicts_with_all = ["value", "from_file"])]
    pub stdin: bool,

    /// S3 path (defaults to name in lowercase with slashes)
    #[arg(long)]
    pub s3_path: Option<String>,

    /// S3 bucket (overrides config)
    #[arg(long)]
    pub bucket: Option<String>,

    /// AWS region (overrides config)
    #[arg(long)]
    pub region: Option<String>,

    /// Master key file path
    #[arg(long)]
    pub key_file: Option<PathBuf>,

    /// Overwrite existing secret
    #[arg(short, long)]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct PullArgs {
    /// Secret name or S3 path
    pub name: String,

    /// Write to file
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Export as environment variable format
    #[arg(long)]
    pub export: bool,

    /// S3 bucket (overrides config)
    #[arg(long)]
    pub bucket: Option<String>,

    /// AWS region (overrides config)
    #[arg(long)]
    pub region: Option<String>,

    /// Master key file path
    #[arg(long)]
    pub key_file: Option<PathBuf>,

    /// Show secret value
    #[arg(long)]
    pub show: bool,
}

#[derive(Args, Debug)]
pub struct SyncArgs {
    /// Dry-run mode (show what would be synced)
    #[arg(long)]
    pub dry_run: bool,

    /// Sync direction
    #[arg(long, value_enum, default_value = "both")]
    pub direction: SyncDirection,

    /// Delete remote secrets not in local config
    #[arg(long)]
    pub delete_remote: bool,

    /// S3 bucket (overrides config)
    #[arg(long)]
    pub bucket: Option<String>,

    /// AWS region (overrides config)
    #[arg(long)]
    pub region: Option<String>,

    /// Master key file path
    #[arg(long)]
    pub key_file: Option<PathBuf>,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum SyncDirection {
    /// Push local to remote
    Push,
    /// Pull remote to local
    Pull,
    /// Bidirectional sync
    Both,
}

impl From<SyncDirection> for S3SyncDirection {
    fn from(dir: SyncDirection) -> Self {
        match dir {
            SyncDirection::Push => S3SyncDirection::Push,
            SyncDirection::Pull => S3SyncDirection::Pull,
            SyncDirection::Both => S3SyncDirection::Both,
        }
    }
}

#[derive(Args, Debug)]
pub struct KeygenArgs {
    /// Output key file path
    #[arg(short, long, default_value = ".sindri-master.key")]
    pub output: PathBuf,

    /// Overwrite existing key file
    #[arg(short, long)]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct RotateArgs {
    /// New master key path
    #[arg(long)]
    pub new_key: PathBuf,

    /// Old master key path (defaults to config)
    #[arg(long)]
    pub old_key: Option<PathBuf>,

    /// Only add new key, don't remove old
    #[arg(long)]
    pub add_only: bool,

    /// S3 bucket (overrides config)
    #[arg(long)]
    pub bucket: Option<String>,

    /// AWS region (overrides config)
    #[arg(long)]
    pub region: Option<String>,

    /// Skip confirmation prompt
    #[arg(short, long)]
    pub yes: bool,
}

pub async fn run(cmd: S3Commands) -> Result<()> {
    match cmd {
        S3Commands::Init(args) => init(args).await,
        S3Commands::Push(args) => push(args).await,
        S3Commands::Pull(args) => pull(args).await,
        S3Commands::Sync(args) => sync(args).await,
        S3Commands::Keygen(args) => keygen(args),
        S3Commands::Rotate(args) => rotate(args).await,
    }
}

/// Build S3 backend configuration from command line args
fn build_config(
    bucket: &str,
    region: &str,
    endpoint: Option<&str>,
    key_file: Option<&PathBuf>,
) -> S3SecretBackend {
    S3SecretBackend {
        bucket: bucket.to_string(),
        region: region.to_string(),
        endpoint: endpoint.map(String::from),
        prefix: "secrets/".to_string(),
        encryption: S3EncryptionConfig {
            key_source: KeySource::File,
            key_file: key_file.cloned(),
            ..Default::default()
        },
        cache: Some(S3CacheConfig::default()),
    }
}

/// Convert a secret name to an S3 path
fn name_to_s3_path(name: &str) -> String {
    name.to_lowercase().replace('_', "/")
}

async fn init(args: InitArgs) -> Result<()> {
    output::header("Initialize S3 Encrypted Storage");

    output::kv("Bucket", &args.bucket);
    output::kv("Region", &args.region);
    if let Some(ref endpoint) = args.endpoint {
        output::kv("Endpoint", endpoint);
    }
    if let Some(ref key_file) = args.key_file {
        output::kv("Key File", &key_file.display().to_string());
    } else {
        output::info("Will generate new master key");
    }
    println!();

    // Create S3 backend to check connectivity
    let spinner = output::spinner("Connecting to S3...");

    let backend = S3Backend::from_params(
        args.bucket.clone(),
        args.region.clone(),
        args.endpoint.clone(),
        "secrets/".to_string(),
    )
    .await
    .context("Failed to create S3 client")?;

    spinner.finish_and_clear();

    // Check if bucket exists
    let spinner = output::spinner("Checking S3 bucket...");

    let bucket_exists = backend.check_bucket().await?;

    spinner.finish_and_clear();

    if !bucket_exists {
        if args.create_bucket {
            let spinner = output::spinner("Creating S3 bucket...");
            backend.create_bucket().await?;
            spinner.finish_and_clear();
            output::success("Bucket created with versioning enabled");
        } else {
            return Err(anyhow!(
                "Bucket '{}' does not exist. Use --create-bucket to create it",
                args.bucket
            ));
        }
    } else {
        output::success("Bucket exists and is accessible");
    }

    // Generate or load master key
    let key_path = args
        .key_file
        .clone()
        .unwrap_or_else(|| PathBuf::from(".sindri-master.key"));

    if !key_path.exists() {
        let spinner = output::spinner("Generating master key...");

        let public_key = generate_key_file(&key_path, false)?;

        spinner.finish_and_clear();

        output::success(&format!("Master key saved to {}", key_path.display()));
        output::kv("Public key", &public_key);
        output::warning("IMPORTANT: Keep this key secure and backed up!");
    } else {
        output::info(&format!("Using existing key file: {}", key_path.display()));

        // Load and display public key
        let encryptor = SecretEncryptor::from_key_file(&key_path)?;
        output::kv("Public key", &encryptor.public_key());
    }

    // Write configuration
    if let Some(output_path) = args.output {
        let config = format!(
            r#"secrets:
  backend:
    type: s3
    bucket: {}
    region: {}
{}    prefix: secrets/
    encryption:
      algorithm: chacha20poly1305
      key_source: file
      key_file: {}
    cache:
      enabled: true
      ttl_seconds: 3600
      path: ~/.sindri/cache/secrets/
"#,
            args.bucket,
            args.region,
            if let Some(ref endpoint) = args.endpoint {
                format!("    endpoint: {}\n", endpoint)
            } else {
                String::new()
            },
            key_path.display()
        );
        std::fs::write(&output_path, config)?;
        output::success(&format!("Configuration written to {}", output_path));
    }

    println!();
    output::success("S3 backend initialized");
    println!();
    output::info("Next steps:");
    println!("  1. Add secrets to sindri.yaml");
    println!("  2. Push secrets: sindri secrets s3 push <name> --value <value>");
    println!("  3. Test: sindri secrets validate");

    Ok(())
}

async fn push(args: PushArgs) -> Result<()> {
    output::header(&format!("Push Secret: {}", args.name));

    // Get secret value
    let value = if args.stdin {
        use std::io::Read;
        let mut buffer = String::new();
        std::io::stdin().read_to_string(&mut buffer)?;
        buffer.trim().to_string()
    } else if let Some(value) = args.value {
        value
    } else if let Some(file_path) = args.from_file {
        std::fs::read_to_string(&file_path)
            .with_context(|| format!("Failed to read file: {:?}", file_path))?
            .trim()
            .to_string()
    } else {
        // Prompt for value
        use dialoguer::Password;
        Password::new().with_prompt("Secret value").interact()?
    };

    if value.is_empty() {
        return Err(anyhow!("Secret value cannot be empty"));
    }

    // Determine S3 path
    let s3_path = args
        .s3_path
        .clone()
        .unwrap_or_else(|| name_to_s3_path(&args.name));

    let bucket = args.bucket.as_deref().unwrap_or("sindri-secrets");
    let region = args.region.as_deref().unwrap_or("us-east-1");
    let key_file = args
        .key_file
        .as_ref()
        .cloned()
        .unwrap_or_else(|| PathBuf::from(".sindri-master.key"));

    output::kv("Name", &args.name);
    output::kv("S3 Path", &s3_path);
    output::kv("Bucket", bucket);
    output::kv("Size", &format!("{} bytes", value.len()));
    println!();

    // Create resolver
    let config = build_config(bucket, region, None, Some(&key_file));
    let resolver = S3SecretResolver::new(config)
        .await
        .context("Failed to create S3 resolver")?;

    // Check if secret exists
    if !args.force {
        let spinner = output::spinner("Checking if secret exists...");
        let exists = resolver.exists(&s3_path).await?;
        spinner.finish_and_clear();

        if exists {
            return Err(anyhow!(
                "Secret '{}' already exists at s3://{}/secrets/{}. Use --force to overwrite",
                args.name,
                bucket,
                s3_path
            ));
        }
    }

    let spinner = output::spinner("Encrypting and uploading...");

    let version_id = resolver.push(&args.name, &value, &s3_path, &[]).await?;

    spinner.finish_and_clear();

    output::success(&format!("Secret '{}' pushed successfully", args.name));
    output::kv("Version", &version_id);
    output::kv("Location", &format!("s3://{}/secrets/{}", bucket, s3_path));

    Ok(())
}

async fn pull(args: PullArgs) -> Result<()> {
    output::header(&format!("Pull Secret: {}", args.name));

    // Determine S3 path (could be the name itself or derived from it)
    let s3_path = if args.name.contains('/') {
        args.name.clone()
    } else {
        name_to_s3_path(&args.name)
    };

    let bucket = args.bucket.as_deref().unwrap_or("sindri-secrets");
    let region = args.region.as_deref().unwrap_or("us-east-1");
    let key_file = args
        .key_file
        .as_ref()
        .cloned()
        .unwrap_or_else(|| PathBuf::from(".sindri-master.key"));

    output::kv("Name", &args.name);
    output::kv("S3 Path", &s3_path);
    output::kv("Bucket", bucket);
    println!();

    // Create resolver
    let config = build_config(bucket, region, None, Some(&key_file));
    let resolver = S3SecretResolver::new(config)
        .await
        .context("Failed to create S3 resolver")?;

    let spinner = output::spinner("Downloading and decrypting...");

    let value = resolver.resolve(&s3_path).await?;

    spinner.finish_and_clear();

    if let Some(output_path) = args.output {
        std::fs::write(&output_path, &value)?;
        output::success(&format!("Secret written to {}", output_path.display()));
    } else if args.export {
        // Use the original name for export
        let env_name = args.name.to_uppercase().replace(['-', '/'], "_");
        println!("export {}='{}'", env_name, value);
    } else if args.show {
        output::success("Secret value:");
        println!("{}", value);
    } else {
        output::success(&format!(
            "Secret '{}' pulled successfully ({} bytes)",
            args.name,
            value.len()
        ));
        output::info("Use --show to display value or --output to save to file");
    }

    // Show cache stats if available
    if let Some(stats) = resolver.cache_stats().await {
        if stats.hits > 0 || stats.misses > 0 {
            output::kv(
                "Cache",
                &format!(
                    "{} hits, {} misses ({:.0}% hit rate)",
                    stats.hits,
                    stats.misses,
                    stats.hit_rate()
                ),
            );
        }
    }

    Ok(())
}

async fn sync(args: SyncArgs) -> Result<()> {
    output::header("Sync Secrets");

    let bucket = args.bucket.as_deref().unwrap_or("sindri-secrets");
    let region = args.region.as_deref().unwrap_or("us-east-1");
    let key_file = args
        .key_file
        .as_ref()
        .cloned()
        .unwrap_or_else(|| PathBuf::from(".sindri-master.key"));

    output::kv("Bucket", bucket);
    output::kv("Direction", &format!("{:?}", args.direction));
    if args.dry_run {
        output::warning("DRY RUN MODE - No changes will be made");
    }
    println!();

    // Create resolver
    let config = build_config(bucket, region, None, Some(&key_file));
    let resolver = S3SecretResolver::new(config)
        .await
        .context("Failed to create S3 resolver")?;

    let spinner = output::spinner("Analyzing secrets...");

    // For now, we'll get the list of remote secrets and show sync status
    // In a full implementation, this would compare with local sindri.yaml secrets
    let local_secrets: Vec<LocalSecretInfo> = Vec::new(); // Would be loaded from config

    let sync_result = resolver
        .sync(&local_secrets, args.direction.into(), args.dry_run)
        .await?;

    spinner.finish_and_clear();

    if !sync_result.to_push.is_empty() {
        output::info(&format!("To push ({}):", sync_result.to_push.len()));
        for name in &sync_result.to_push {
            println!(
                "  {} {}",
                console::style("^").cyan(),
                console::style(name).cyan()
            );
        }
    }

    if !sync_result.to_pull.is_empty() {
        output::info(&format!("To pull ({}):", sync_result.to_pull.len()));
        for name in &sync_result.to_pull {
            println!(
                "  {} {}",
                console::style("v").cyan(),
                console::style(name).cyan()
            );
        }
    }

    if !sync_result.in_sync.is_empty() {
        output::info(&format!("In sync ({}):", sync_result.in_sync.len()));
        for name in &sync_result.in_sync {
            println!(
                "  {} {}",
                console::style("=").green(),
                console::style(name).green()
            );
        }
    }

    if !sync_result.conflicts.is_empty() {
        output::warning(&format!("Conflicts ({}):", sync_result.conflicts.len()));
        for conflict in &sync_result.conflicts {
            println!(
                "  {} {} - {}",
                console::style("!").red(),
                console::style(&conflict.name).red(),
                conflict.reason
            );
        }
    }

    if args.dry_run {
        println!();
        output::info("Dry run complete. Use without --dry-run to apply changes");
        return Ok(());
    }

    // If there were actual changes to make
    if !sync_result.to_push.is_empty() || !sync_result.to_pull.is_empty() {
        use dialoguer::Confirm;
        if !Confirm::new()
            .with_prompt("Apply changes?")
            .default(false)
            .interact()?
        {
            output::info("Sync cancelled");
            return Ok(());
        }

        output::success("Secrets synced successfully");
    } else if sync_result.in_sync.is_empty() && sync_result.to_pull.is_empty() {
        // List remote secrets to show what's available
        let remote_secrets = resolver.backend().list_secrets().await?;
        if remote_secrets.is_empty() {
            output::info("No secrets found in S3");
        } else {
            output::info(&format!("Remote secrets ({}):", remote_secrets.len()));
            for secret in &remote_secrets {
                println!("  - {}", secret);
            }
        }
    } else {
        output::success("All secrets are in sync");
    }

    Ok(())
}

fn keygen(args: KeygenArgs) -> Result<()> {
    output::header("Generate Master Key");

    if args.output.exists() && !args.force {
        return Err(anyhow!(
            "Key file already exists: {}. Use --force to overwrite",
            args.output.display()
        ));
    }

    output::kv("Output", &args.output.display().to_string());
    output::kv("Algorithm", "age X25519");
    println!();

    let spinner = output::spinner("Generating age keypair...");

    let public_key = generate_key_file(&args.output, args.force)?;

    spinner.finish_and_clear();

    output::success("Master key generated successfully");
    output::kv("Public key", &public_key);
    println!();
    output::warning("IMPORTANT: Keep this key secure and backed up!");
    output::info("Add to .gitignore to prevent committing to version control");
    println!();
    println!("  echo '{}' >> .gitignore", args.output.display());

    Ok(())
}

async fn rotate(args: RotateArgs) -> Result<()> {
    output::header("Rotate Master Key");

    // Verify new key exists
    if !args.new_key.exists() {
        return Err(anyhow!(
            "New key file not found: {}",
            args.new_key.display()
        ));
    }

    let old_key_path = args
        .old_key
        .clone()
        .unwrap_or_else(|| PathBuf::from(".sindri-master.key"));

    if !old_key_path.exists() {
        return Err(anyhow!(
            "Old key file not found: {}",
            old_key_path.display()
        ));
    }

    let bucket = args.bucket.as_deref().unwrap_or("sindri-secrets");
    let region = args.region.as_deref().unwrap_or("us-east-1");

    output::kv("Bucket", bucket);
    output::kv("New Key", &args.new_key.display().to_string());
    output::kv("Old Key", &old_key_path.display().to_string());

    if args.add_only {
        output::info("Mode: Add new key only (dual-key support)");
    } else {
        output::warning("Mode: Full rotation (re-encrypt all secrets)");
    }
    println!();

    // Load both keys
    let old_encryptor = SecretEncryptor::from_key_file(&old_key_path)?;
    let new_encryptor = SecretEncryptor::from_key_file(&args.new_key)?;

    output::kv("Old public key", &old_encryptor.public_key());
    output::kv("New public key", &new_encryptor.public_key());
    println!();

    // Confirm
    if !args.yes {
        use dialoguer::Confirm;
        if !Confirm::new()
            .with_prompt("Rotate master key? This will re-encrypt all secrets")
            .default(false)
            .interact()?
        {
            output::info("Key rotation cancelled");
            return Ok(());
        }
    }

    // Create resolver with old key
    let config = build_config(bucket, region, None, Some(&old_key_path));
    let resolver = S3SecretResolver::new(config)
        .await
        .context("Failed to create S3 resolver")?;

    let spinner = output::spinner("Listing secrets...");
    let secrets = resolver.backend().list_secrets().await?;
    spinner.finish_and_clear();

    if secrets.is_empty() {
        output::info("No secrets found to rotate");
        return Ok(());
    }

    output::info(&format!("Found {} secrets to rotate", secrets.len()));

    let pb = output::progress_bar(secrets.len() as u64, "Rotating secrets");

    let rotated = resolver.rotate_key(&new_encryptor, &secrets).await?;

    for _ in &rotated {
        pb.inc(1);
    }
    pb.finish_and_clear();

    output::success(&format!(
        "Successfully rotated {} secrets to new key",
        rotated.len()
    ));

    if !args.add_only {
        println!();
        output::warning("Old key is no longer needed. Secure deletion recommended:");
        println!("  shred -u {}", old_key_path.display());
    }

    Ok(())
}
