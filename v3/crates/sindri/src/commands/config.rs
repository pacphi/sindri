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

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;
    use tempfile::TempDir;

    fn make_init_args(
        output: Utf8PathBuf,
        provider: &str,
        force: bool,
        name: Option<String>,
    ) -> ConfigInitArgs {
        ConfigInitArgs {
            name,
            provider: provider.to_string(),
            profile: "minimal".to_string(),
            output,
            force,
        }
    }

    #[test]
    fn test_init_errors_when_file_exists_without_force() {
        let tmp = TempDir::new().unwrap();
        let file_path = Utf8PathBuf::from_path_buf(tmp.path().join("sindri.yaml")).unwrap();
        std::fs::write(&file_path, "existing content").unwrap();

        let args = make_init_args(file_path.clone(), "docker", false, Some("test".to_string()));
        let result = init(args);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("already exists"),
            "error should mention file already exists, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_init_succeeds_with_force_when_file_exists() {
        let tmp = TempDir::new().unwrap();
        let file_path = Utf8PathBuf::from_path_buf(tmp.path().join("sindri.yaml")).unwrap();
        std::fs::write(&file_path, "existing content").unwrap();

        let args = make_init_args(
            file_path.clone(),
            "docker",
            true,
            Some("test-force".to_string()),
        );
        let result = init(args);

        assert!(
            result.is_ok(),
            "init with --force should succeed: {:?}",
            result.err()
        );
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert!(
            content.contains("test-force"),
            "file should contain the project name"
        );
    }

    #[test]
    fn test_init_valid_provider_docker() {
        let tmp = TempDir::new().unwrap();
        let file_path = Utf8PathBuf::from_path_buf(tmp.path().join("sindri.yaml")).unwrap();

        let args = make_init_args(
            file_path.clone(),
            "docker",
            false,
            Some("my-proj".to_string()),
        );
        let result = init(args);
        assert!(
            result.is_ok(),
            "docker provider should be valid: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_init_valid_provider_fly() {
        let tmp = TempDir::new().unwrap();
        let file_path = Utf8PathBuf::from_path_buf(tmp.path().join("sindri.yaml")).unwrap();

        let args = make_init_args(file_path, "fly", false, Some("fly-proj".to_string()));
        let result = init(args);
        assert!(
            result.is_ok(),
            "fly provider should be valid: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_init_valid_provider_kubernetes_alias() {
        let tmp = TempDir::new().unwrap();
        let file_path = Utf8PathBuf::from_path_buf(tmp.path().join("sindri.yaml")).unwrap();

        let args = make_init_args(file_path, "k8s", false, Some("k8s-proj".to_string()));
        let result = init(args);
        assert!(
            result.is_ok(),
            "k8s alias should be valid: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_init_invalid_provider() {
        let tmp = TempDir::new().unwrap();
        let file_path = Utf8PathBuf::from_path_buf(tmp.path().join("sindri.yaml")).unwrap();

        let args = make_init_args(
            file_path,
            "invalid-provider",
            false,
            Some("test".to_string()),
        );
        let result = init(args);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Unknown provider"),
            "error should mention unknown provider, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_init_creates_valid_yaml_file() {
        let tmp = TempDir::new().unwrap();
        let file_path = Utf8PathBuf::from_path_buf(tmp.path().join("sindri.yaml")).unwrap();

        let args = make_init_args(
            file_path.clone(),
            "docker",
            false,
            Some("yaml-test".to_string()),
        );
        init(args).unwrap();

        let content = std::fs::read_to_string(&file_path).unwrap();
        // The generated config should be valid YAML
        let parsed: Result<serde_yaml_ng::Value, _> = serde_yaml_ng::from_str(&content);
        assert!(parsed.is_ok(), "generated config should be valid YAML");
    }
}
