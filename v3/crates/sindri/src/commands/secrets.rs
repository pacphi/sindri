//! Secrets management commands

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use sindri_core::config::SindriConfig;
use sindri_secrets::{ResolutionContext, SecretResolver};
use std::path::PathBuf;

use super::secrets_s3;
use crate::output;

#[derive(Subcommand, Debug)]
pub enum SecretsCommands {
    /// Validate all secrets are resolvable
    Validate(ValidateArgs),

    /// List configured secrets
    List(ListArgs),

    /// Test Vault connection
    TestVault(TestVaultArgs),

    /// Encode file to base64
    EncodeFile(EncodeFileArgs),

    /// S3 encrypted storage commands
    #[command(subcommand)]
    S3(secrets_s3::S3Commands),
}

#[derive(Args, Debug)]
pub struct ValidateArgs {
    /// Show secret values (WARNING: insecure)
    #[arg(long)]
    pub show_values: bool,
}

#[derive(Args, Debug)]
pub struct ListArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Filter by source type
    #[arg(short, long)]
    pub source: Option<String>,
}

#[derive(Args, Debug)]
pub struct TestVaultArgs {
    /// Vault address (overrides config/env)
    #[arg(long)]
    pub address: Option<String>,

    /// Vault token (overrides env)
    #[arg(long)]
    pub token: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct EncodeFileArgs {
    /// File to encode
    pub file: PathBuf,

    /// Output to file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Add newline at end
    #[arg(long, default_value = "true")]
    pub newline: bool,
}

pub async fn run(cmd: SecretsCommands) -> Result<()> {
    match cmd {
        SecretsCommands::Validate(args) => validate(args).await,
        SecretsCommands::List(args) => list(args).await,
        SecretsCommands::TestVault(args) => test_vault(args).await,
        SecretsCommands::EncodeFile(args) => encode_file(args),
        SecretsCommands::S3(cmd) => secrets_s3::run(cmd).await,
    }
}

async fn validate(args: ValidateArgs) -> Result<()> {
    let spinner = output::spinner("Loading configuration...");
    let config = SindriConfig::load(None)?;
    spinner.finish_and_clear();

    let secrets = config.secrets();
    if secrets.is_empty() {
        output::info("No secrets configured");
        return Ok(());
    }

    output::header(&format!("Validating {} secrets", secrets.len()));

    let spinner = output::spinner("Resolving secrets...");
    // Create resolution context from config directory or current directory
    let config_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".sindri");
    let context = ResolutionContext::new(config_dir);
    let resolver = SecretResolver::new(context);
    let results = resolver.resolve_all(secrets).await;
    spinner.finish_and_clear();

    match results {
        Ok(resolved) => {
            output::success(&format!(
                "All {} secrets resolved successfully",
                resolved.len()
            ));
            println!();

            for (name, secret) in resolved {
                let value_display = if args.show_values {
                    secret.value.as_string().unwrap_or("[binary]").to_string()
                } else {
                    "*".repeat(secret.value.len().min(8))
                };

                println!(
                    "  {} {} = {} ({})",
                    style_source(&secret.metadata.source_type.to_string()),
                    console::style(&name).cyan(),
                    console::style(value_display).dim(),
                    console::style(format!("{} bytes", secret.value.len())).dim()
                );
            }

            if !args.show_values {
                println!();
                output::info("Use --show-values to display actual values (WARNING: insecure)");
            }

            Ok(())
        }
        Err(e) => {
            output::error("Secret validation failed");
            Err(e)
        }
    }
}

async fn list(args: ListArgs) -> Result<()> {
    let config = SindriConfig::load(None)?;
    let secrets = config.secrets();

    if secrets.is_empty() {
        output::info("No secrets configured");
        return Ok(());
    }

    // Filter by source if specified
    let filtered: Vec<_> = if let Some(ref source_filter) = args.source {
        secrets
            .iter()
            .filter(|s| s.source.to_string().eq_ignore_ascii_case(source_filter))
            .collect()
    } else {
        secrets.iter().collect()
    };

    if args.json {
        let json = serde_json::to_string_pretty(&filtered)?;
        println!("{}", json);
        return Ok(());
    }

    output::header(&format!("Configured Secrets ({} total)", secrets.len()));

    if filtered.is_empty() {
        output::warning(&format!(
            "No secrets match source filter: {}",
            args.source.unwrap_or_default()
        ));
        return Ok(());
    }

    // Group by source
    use std::collections::HashMap;
    let mut by_source: HashMap<String, Vec<_>> = HashMap::new();
    for secret in &filtered {
        by_source
            .entry(secret.source.to_string())
            .or_default()
            .push(secret);
    }

    for (source, secrets) in by_source {
        println!();
        println!(
            "{} {} ({} secrets)",
            style_source(&source),
            console::style(&source).bold(),
            secrets.len()
        );

        for secret in secrets {
            println!(
                "  {} {}",
                console::style("•").dim(),
                console::style(&secret.name).cyan()
            );

            // Show source-specific details based on source type
            match secret.source {
                sindri_core::types::SecretSource::Env => {
                    // For env, the name IS the var name
                    println!(
                        "    {} {}",
                        console::style("var:").dim(),
                        console::style(&secret.name).yellow()
                    );
                    if let Some(ref from_file) = secret.from_file {
                        println!(
                            "    {} {}",
                            console::style("fromFile:").dim(),
                            console::style(from_file).yellow()
                        );
                    }
                }
                sindri_core::types::SecretSource::File => {
                    if let Some(ref path) = secret.path {
                        println!(
                            "    {} {}",
                            console::style("path:").dim(),
                            console::style(path).yellow()
                        );
                    }
                }
                sindri_core::types::SecretSource::Vault => {
                    if let Some(ref path) = secret.vault_path {
                        println!(
                            "    {} {}",
                            console::style("path:").dim(),
                            console::style(path).yellow()
                        );
                    }
                    if let Some(ref key) = secret.vault_key {
                        println!(
                            "    {} {}",
                            console::style("key:").dim(),
                            console::style(key).yellow()
                        );
                    }
                }
                sindri_core::types::SecretSource::S3 => {
                    if let Some(ref s3_path) = secret.s3_path {
                        println!(
                            "    {} {}",
                            console::style("s3Path:").dim(),
                            console::style(s3_path).yellow()
                        );
                    }
                }
            }
        }
    }

    println!();
    output::info("Use 'sindri secrets validate' to test resolution");

    Ok(())
}

async fn test_vault(args: TestVaultArgs) -> Result<()> {
    output::header("Testing Vault Connection");

    // Get Vault configuration
    let vault_addr = if let Some(addr) = args.address {
        addr
    } else if let Ok(addr) = std::env::var("VAULT_ADDR") {
        addr
    } else {
        return Err(anyhow::anyhow!(
            "Vault address not provided. Use --address or set VAULT_ADDR"
        ));
    };

    let vault_token = if let Some(token) = args.token {
        token
    } else if let Ok(token) = std::env::var("VAULT_TOKEN") {
        token
    } else {
        return Err(anyhow::anyhow!(
            "Vault token not provided. Use --token or set VAULT_TOKEN"
        ));
    };

    output::kv("Address", &vault_addr);
    output::kv("Token", &"*".repeat(8));
    println!();

    let spinner = output::spinner("Connecting to Vault...");

    // Use vaultrs directly for health check
    use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};
    let settings = VaultClientSettingsBuilder::default()
        .address(&vault_addr)
        .token(&vault_token)
        .build()
        .context("Failed to build Vault client settings")?;

    let client = VaultClient::new(settings)?;
    spinner.finish_and_clear();

    let spinner = output::spinner("Testing authentication...");

    // Test by reading token info
    match vaultrs::token::lookup(&client, &vault_token).await {
        Ok(_) => {
            spinner.finish_and_clear();
            output::success("Vault connection successful");

            if args.json {
                let result = serde_json::json!({
                    "status": "ok",
                    "address": vault_addr,
                    "authenticated": true,
                });
                println!("{}", serde_json::to_string_pretty(&result)?);
            }

            Ok(())
        }
        Err(e) => {
            spinner.finish_and_clear();
            output::error("Vault connection failed");
            Err(anyhow::anyhow!("Vault authentication failed: {}", e))
        }
    }
}

fn encode_file(args: EncodeFileArgs) -> Result<()> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    let contents = std::fs::read(&args.file)
        .with_context(|| format!("Failed to read file: {:?}", args.file))?;

    let encoded = STANDARD.encode(&contents);

    if let Some(output_path) = args.output {
        let mut output = encoded;
        if args.newline {
            output.push('\n');
        }
        std::fs::write(&output_path, output)
            .with_context(|| format!("Failed to write to: {:?}", output_path))?;

        output::success(&format!(
            "Encoded {} bytes to {}",
            contents.len(),
            output_path.display()
        ));
    } else {
        print!("{}", encoded);
        if args.newline {
            println!();
        }
    }

    Ok(())
}

fn style_source(source: &str) -> console::StyledObject<&'static str> {
    match source.to_lowercase().as_str() {
        "env" => console::style("📦").green(),
        "file" => console::style("📄").blue(),
        "vault" => console::style("🔐").magenta(),
        "s3" => console::style("☁️").cyan(),
        _ => console::style("•").dim(),
    }
}
