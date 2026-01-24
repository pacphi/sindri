//! Config command

use anyhow::{anyhow, Result};
use camino::Utf8Path;
use sindri_core::config::{generate_config, SindriConfig};
use sindri_core::schema::SchemaValidator;
use sindri_core::types::Provider;

use crate::cli::{ConfigCommands, ConfigInitArgs, ConfigShowArgs, ConfigValidateArgs};
use crate::output;

pub async fn run(cmd: ConfigCommands) -> Result<()> {
    match cmd {
        ConfigCommands::Init(args) => init(args),
        ConfigCommands::Validate(args) => validate(args),
        ConfigCommands::Show(args) => show(args),
    }
}

fn init(args: ConfigInitArgs) -> Result<()> {
    // Check if file exists
    if args.output.exists() && !args.force {
        return Err(anyhow!(
            "File {} already exists. Use --force to overwrite.",
            args.output
        ));
    }

    // Get project name
    let name = args.name.unwrap_or_else(|| {
        std::env::current_dir()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_else(|| "my-project".to_string())
            .to_lowercase()
            .replace(' ', "-")
    });

    // Parse provider
    let provider: Provider = match args.provider.as_str() {
        "docker" | "docker-compose" => Provider::Docker,
        "fly" => Provider::Fly,
        "devpod" => Provider::Devpod,
        "e2b" => Provider::E2b,
        "kubernetes" | "k8s" => Provider::Kubernetes,
        _ => return Err(anyhow!("Unknown provider: {}", args.provider)),
    };

    // Generate config using template with selected profile
    let content = generate_config(&name, provider, &args.profile)
        .map_err(|e| anyhow!("Failed to generate config: {}", e))?;

    // Write file
    std::fs::write(&args.output, content)?;

    output::success(&format!("Created {}", args.output));
    output::info(&format!("Provider: {}", provider));
    output::info(&format!("Profile: {}", args.profile));

    Ok(())
}

fn validate(args: ConfigValidateArgs) -> Result<()> {
    let spinner = output::spinner("Validating configuration...");

    // Get config path
    let config_path = args.file.map(|p| p.into_std_path_buf());

    // Load and validate
    let validator = SchemaValidator::new()?;

    let config = if let Some(path) = &config_path {
        SindriConfig::load_and_validate(Some(Utf8Path::from_path(path).unwrap()), &validator)?
    } else {
        SindriConfig::load_and_validate(None, &validator)?
    };

    spinner.finish_and_clear();

    output::success(&format!("Configuration is valid: {}", config.config_path));
    output::kv("Name", config.name());
    output::kv("Provider", &config.provider().to_string());

    if let Some(profile) = &config.extensions().profile {
        output::kv("Profile", profile);
    }

    if args.check_extensions {
        output::info("Extension validation not yet implemented");
    }

    Ok(())
}

fn show(args: ConfigShowArgs) -> Result<()> {
    let config = SindriConfig::load(None)?;

    if args.json {
        let json = serde_json::to_string_pretty(&config.config)?;
        println!("{}", json);
    } else {
        let yaml = config.to_yaml()?;
        println!("{}", yaml);
    }

    Ok(())
}
