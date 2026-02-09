//! Extension verification module
//!
//! Verifies that extension software is actually installed on the system
//! by checking if tools, packages, and binaries exist.

use sindri_core::types::{Extension, InstallMethod};
use std::path::Path;
use std::process::Command;
use tracing::{debug, warn};

/// Verify that an extension's software is actually installed
///
/// This checks:
/// - mise tools are installed (for mise method)
/// - apt packages are installed (for apt method)
/// - binaries exist in PATH or destination (for binary method)
/// - npm packages are installed globally (for npm-global method)
/// - validation commands succeed (for all methods)
///
/// Returns true if verification passes, false otherwise
pub async fn verify_extension_installed(extension: &Extension) -> bool {
    debug!(
        "Verifying extension: {} (method: {:?})",
        extension.metadata.name, extension.install.method
    );

    // 1. Check installation method-specific verification
    let method_verified = match extension.install.method {
        InstallMethod::Mise => verify_mise_tools(extension).await,
        InstallMethod::Apt => verify_apt_packages(extension).await,
        InstallMethod::Binary => verify_binaries(extension).await,
        InstallMethod::Npm | InstallMethod::NpmGlobal => verify_npm_packages(extension).await,
        InstallMethod::Script => {
            // For scripts, we can't verify the method itself, rely on validation commands
            true
        }
        InstallMethod::Hybrid => {
            // Hybrid uses multiple methods, verify each that's present
            let mut all_verified = true;

            if extension.install.mise.is_some() {
                all_verified &= verify_mise_tools(extension).await;
            }
            if extension.install.apt.is_some() {
                all_verified &= verify_apt_packages(extension).await;
            }
            if extension.install.binary.is_some() {
                all_verified &= verify_binaries(extension).await;
            }
            if extension.install.npm.is_some() {
                all_verified &= verify_npm_packages(extension).await;
            }

            all_verified
        }
    };

    if !method_verified {
        debug!(
            "Extension {} failed method verification",
            extension.metadata.name
        );
        return false;
    }

    // 2. Run validation commands if defined
    if !extension.validate.commands.is_empty() {
        for cmd_validation in &extension.validate.commands {
            debug!(
                "Running validation command: {} {}",
                cmd_validation.name, cmd_validation.version_flag
            );

            let output = Command::new(&cmd_validation.name)
                .arg(&cmd_validation.version_flag)
                .output();

            match output {
                Ok(output) if output.status.success() => {
                    debug!("Command {} succeeded", cmd_validation.name);
                }
                Ok(output) => {
                    warn!(
                        "Command {} failed with exit code: {:?}",
                        cmd_validation.name,
                        output.status.code()
                    );
                    return false;
                }
                Err(e) => {
                    warn!("Command {} not found: {}", cmd_validation.name, e);
                    return false;
                }
            }
        }
    }

    // 3. Check mise validation if defined
    if let Some(mise_validation) = &extension.validate.mise {
        if !mise_validation.tools.is_empty() {
            let mise_verified = verify_mise_tools_list(&mise_validation.tools).await;
            if !mise_verified {
                debug!(
                    "Extension {} failed mise validation",
                    extension.metadata.name
                );
                return false;
            }
        }
    }

    debug!("Extension {} verification passed", extension.metadata.name);
    true
}

/// Verify mise tools are installed
async fn verify_mise_tools(extension: &Extension) -> bool {
    let Some(mise_config) = &extension.install.mise else {
        return true; // No mise config, nothing to verify
    };

    // Check if mise config file exists
    if let Some(config_file) = &mise_config.config_file {
        let Ok(home) = std::env::var("HOME") else {
            debug!("HOME environment variable not set");
            return false;
        };

        let extensions_dir = Path::new(&home)
            .join(".sindri")
            .join("extensions")
            .join(&extension.metadata.name);

        // Try both versioned and flat structure
        let config_path = extensions_dir
            .join(&extension.metadata.version)
            .join(config_file);
        let config_path_flat = extensions_dir.join(config_file);

        if !config_path.exists() && !config_path_flat.exists() {
            debug!("mise config file not found: {}", config_file);
            return false;
        }

        // Parse the mise.toml to get tool names
        let config_content = if config_path.exists() {
            match tokio::fs::read_to_string(&config_path).await {
                Ok(content) => content,
                Err(e) => {
                    debug!("Failed to read mise config: {}", e);
                    return false;
                }
            }
        } else {
            match tokio::fs::read_to_string(&config_path_flat).await {
                Ok(content) => content,
                Err(e) => {
                    debug!("Failed to read mise config: {}", e);
                    return false;
                }
            }
        };

        // Simple parsing to extract tool names from [tools] section
        let tools = parse_mise_tools(&config_content);
        if tools.is_empty() {
            debug!("No tools found in mise config");
            return true; // Config exists but no tools defined
        }

        return verify_mise_tools_list(&tools).await;
    }

    true
}

/// Parse tool names from mise.toml content
fn parse_mise_tools(content: &str) -> Vec<String> {
    let mut tools = Vec::new();
    let mut in_tools_section = false;

    for line in content.lines() {
        let line = line.trim();

        if line == "[tools]" {
            in_tools_section = true;
            continue;
        }

        if line.starts_with('[') && in_tools_section {
            // New section, stop parsing
            break;
        }

        if in_tools_section && !line.is_empty() && !line.starts_with('#') {
            // Extract tool name (before = or :)
            if let Some(tool_name) = line.split('=').next().or_else(|| line.split(':').next()) {
                tools.push(tool_name.trim().trim_matches('"').to_string());
            }
        }
    }

    tools
}

/// Verify a list of mise tools are installed
async fn verify_mise_tools_list(tools: &[String]) -> bool {
    for tool in tools {
        debug!("Checking if mise tool {} is installed", tool);

        let output = Command::new("mise").arg("list").arg(tool).output();

        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.trim().is_empty() {
                    debug!("mise tool {} not installed", tool);
                    return false;
                }
            }
            Ok(output) => {
                debug!(
                    "mise list {} failed with exit code: {:?}",
                    tool,
                    output.status.code()
                );
                return false;
            }
            Err(e) => {
                debug!("mise command failed: {}", e);
                return false;
            }
        }
    }

    true
}

/// Verify apt packages are installed
async fn verify_apt_packages(extension: &Extension) -> bool {
    let Some(apt_config) = &extension.install.apt else {
        return true; // No apt config, nothing to verify
    };

    if apt_config.packages.is_empty() {
        return true;
    }

    for package in &apt_config.packages {
        debug!("Checking if apt package {} is installed", package);

        let Ok(output) = Command::new("dpkg").arg("-l").arg(package).output() else {
            debug!("dpkg command failed");
            return false;
        };

        if !output.status.success() {
            debug!("Package {} not installed", package);
            return false;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.contains(&format!("ii  {}", package)) {
            debug!("Package {} not fully installed", package);
            return false;
        }
    }

    true
}

/// Verify binaries are installed
async fn verify_binaries(extension: &Extension) -> bool {
    let Some(binary_config) = &extension.install.binary else {
        return true; // No binary config, nothing to verify
    };

    if binary_config.downloads.is_empty() {
        return true;
    }

    for download in &binary_config.downloads {
        debug!("Checking if binary {} exists", download.name);

        // Check if binary exists in destination or PATH
        if let Some(destination) = &download.destination {
            // Expand ~/ in destination
            let dest_path = if destination.starts_with("~/") {
                let Ok(home) = std::env::var("HOME") else {
                    debug!("HOME environment variable not set");
                    return false;
                };
                Path::new(&home).join(destination.trim_start_matches("~/"))
            } else {
                Path::new(destination).to_path_buf()
            };

            if !dest_path.exists() {
                debug!("Binary {} not found at {}", download.name, destination);
                return false;
            }
        } else {
            // Check PATH
            let Ok(output) = Command::new("which").arg(&download.name).output() else {
                debug!("which command failed for {}", download.name);
                return false;
            };

            if !output.status.success() {
                debug!("Binary {} not found in PATH", download.name);
                return false;
            }
        }
    }

    true
}

/// Verify npm packages are installed
async fn verify_npm_packages(extension: &Extension) -> bool {
    let Some(npm_config) = &extension.install.npm else {
        return true; // No npm config, nothing to verify
    };

    debug!(
        "Checking if npm package {} is installed",
        npm_config.package
    );

    // Extract package name (without version)
    let package_name = npm_config
        .package
        .split('@')
        .next()
        .unwrap_or(&npm_config.package);

    // Check global npm packages
    let Ok(output) = Command::new("npm")
        .args(["list", "-g", "--depth=0", package_name])
        .output()
    else {
        debug!("npm command failed");
        return false;
    };

    if !output.status.success() {
        debug!("npm package {} not installed globally", package_name);
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mise_tools() {
        let content = r#"
[tools]
node = "20"
python = "3.11"
golang = "latest"

[env]
FOO = "bar"
"#;

        let tools = parse_mise_tools(content);
        assert_eq!(tools.len(), 3);
        assert!(tools.contains(&"node".to_string()));
        assert!(tools.contains(&"python".to_string()));
        assert!(tools.contains(&"golang".to_string()));
    }

    #[test]
    fn test_parse_mise_tools_empty() {
        let content = r#"
[env]
FOO = "bar"
"#;

        let tools = parse_mise_tools(content);
        assert!(tools.is_empty());
    }
}
