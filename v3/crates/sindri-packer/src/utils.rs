//! Utility functions for Packer operations
//!
//! This module provides common utilities used across cloud provider implementations.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Output;
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Check if Packer CLI is installed
pub fn check_packer_installed() -> Result<Option<String>> {
    match which::which("packer") {
        Ok(path) => {
            debug!("Found Packer at: {}", path.display());
            // Get version
            let output = std::process::Command::new("packer")
                .args(["--version"])
                .output()
                .context("Failed to get Packer version")?;

            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .replace("Packer v", "")
                    .to_string();
                Ok(Some(version))
            } else {
                Ok(Some("unknown".to_string()))
            }
        }
        Err(_) => Ok(None),
    }
}

/// Check if a CLI tool is installed and get its version
pub fn check_cli_installed(cli_name: &str, version_args: &[&str]) -> Result<Option<String>> {
    match which::which(cli_name) {
        Ok(path) => {
            debug!("Found {} at: {}", cli_name, path.display());
            let output = std::process::Command::new(cli_name)
                .args(version_args)
                .output()
                .context(format!("Failed to get {} version", cli_name))?;

            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .next()
                    .unwrap_or("unknown")
                    .trim()
                    .to_string();
                Ok(Some(version))
            } else {
                Ok(Some("unknown".to_string()))
            }
        }
        Err(_) => Ok(None),
    }
}

/// Initialize Packer plugins for a template
pub async fn packer_init(template_path: &Path) -> Result<Output> {
    info!("Initializing Packer plugins...");

    let output = Command::new("packer")
        .args(["init", template_path.to_str().unwrap()])
        .output()
        .await
        .context("Failed to run packer init")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("Packer init warnings: {}", stderr);
    }

    Ok(output)
}

/// Validate a Packer template
pub async fn packer_validate(template_path: &Path, syntax_only: bool) -> Result<Output> {
    info!("Validating Packer template...");

    let mut cmd = Command::new("packer");
    cmd.arg("validate");

    if syntax_only {
        cmd.arg("-syntax-only");
    }

    cmd.arg(template_path.to_str().unwrap());

    let output = cmd
        .output()
        .await
        .context("Failed to run packer validate")?;

    Ok(output)
}

/// Format a Packer template
#[allow(dead_code)]
pub async fn packer_fmt(template_path: &Path, check_only: bool) -> Result<Output> {
    let mut cmd = Command::new("packer");
    cmd.arg("fmt");

    if check_only {
        cmd.arg("-check");
    }

    cmd.arg(template_path.to_str().unwrap());

    let output = cmd.output().await.context("Failed to run packer fmt")?;

    Ok(output)
}

/// Ensure directory exists
pub fn ensure_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)
            .context(format!("Failed to create directory: {}", path.display()))?;
    }
    Ok(())
}

/// Write content to file
pub fn write_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    std::fs::write(path, content).context(format!("Failed to write file: {}", path.display()))
}

/// Read file content
#[allow(dead_code)]
pub fn read_file(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).context(format!("Failed to read file: {}", path.display()))
}

/// Get the default output directory for Packer artifacts
pub fn default_output_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("sindri")
        .join("packer")
}

/// Generate a unique build ID
pub fn generate_build_id() -> String {
    uuid::Uuid::new_v4().to_string()[..8].to_string()
}

/// Generate a timestamped image name
#[allow(dead_code)]
pub fn generate_image_name(prefix: &str) -> String {
    let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
    format!("{}-{}", prefix, timestamp)
}

/// Parse AMI ID from Packer output
pub fn parse_ami_id(output: &str) -> Option<String> {
    for line in output.lines() {
        if line.contains("ami-") {
            if let Some(start) = line.find("ami-") {
                let rest = &line[start..];
                let end = rest
                    .find(|c: char| !c.is_alphanumeric() && c != '-')
                    .unwrap_or(rest.len());
                if end > 4 {
                    // "ami-" is 4 chars
                    return Some(rest[..end].to_string());
                }
            }
        }
    }
    None
}

/// Parse Azure image ID from Packer output
pub fn parse_azure_image_id(output: &str) -> Option<String> {
    for line in output.lines() {
        if line.contains("/providers/Microsoft.Compute/images/") {
            // Extract the full resource ID
            if let Some(start) = line.find("/subscriptions/") {
                let rest = &line[start..];
                if let Some(end) = rest.find(|c: char| c.is_whitespace() || c == '"') {
                    return Some(rest[..end].to_string());
                }
                return Some(rest.to_string());
            }
        }
    }
    None
}

/// Parse GCP image name from Packer output
pub fn parse_gcp_image(output: &str) -> Option<String> {
    // Look for "A disk image was created:" or similar pattern
    for line in output.lines() {
        if line.contains("disk image was created") || line.contains("googlecompute: A disk image") {
            // The image name is usually on the next line or in the same line
            if let Some(name_start) = line.rfind(' ') {
                let potential_name = line[name_start..].trim();
                if !potential_name.is_empty() && !potential_name.contains(':') {
                    return Some(potential_name.to_string());
                }
            }
        }
    }
    None
}

/// Parse OCI image OCID from Packer output
pub fn parse_oci_image_id(output: &str) -> Option<String> {
    for line in output.lines() {
        if line.contains("ocid1.image.") {
            if let Some(start) = line.find("ocid1.image.") {
                let rest = &line[start..];
                let end = rest
                    .find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
                    .unwrap_or(rest.len());
                return Some(rest[..end].to_string());
            }
        }
    }
    None
}

/// Parse Alibaba Cloud image ID from Packer output
pub fn parse_alicloud_image_id(output: &str) -> Option<String> {
    for line in output.lines() {
        if line.contains("m-") && (line.contains("Image ID:") || line.contains("alicloud-ecs")) {
            // Alibaba image IDs start with "m-"
            if let Some(start) = line.find("m-") {
                let rest = &line[start..];
                let end = rest
                    .find(|c: char| !c.is_alphanumeric() && c != '-')
                    .unwrap_or(rest.len());
                if end > 2 {
                    return Some(rest[..end].to_string());
                }
            }
        }
    }
    None
}

/// Sanitize a string for use in resource names
pub fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

/// Calculate a hash of the configuration for cache key
pub fn config_hash(config: &impl serde::Serialize) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let json = serde_json::to_string(config).unwrap_or_default();
    let mut hasher = DefaultHasher::new();
    json.hash(&mut hasher);
    format!("{:x}", hasher.finish())[..8].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ami_id() {
        let output = r#"
==> amazon-ebs.sindri: Creating AMI from instance i-1234567890abcdef0
    amazon-ebs.sindri: AMI: ami-0123456789abcdef0
==> amazon-ebs.sindri: Waiting for AMI to become ready...
        "#;
        assert_eq!(
            parse_ami_id(output),
            Some("ami-0123456789abcdef0".to_string())
        );
    }

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("My Dev Environment"), "my-dev-environment");
        assert_eq!(sanitize_name("test_name-123"), "test_name-123");
        assert_eq!(sanitize_name("---test---"), "test");
    }

    #[test]
    fn test_generate_image_name() {
        let name = generate_image_name("sindri-dev");
        assert!(name.starts_with("sindri-dev-"));
        assert!(name.len() > 15); // prefix + dash + timestamp
    }
}
