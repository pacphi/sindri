//! Extension validation
//!
//! This module provides JSON Schema validation for extensions and additional
//! semantic validation beyond what schemas can express.

use anyhow::Result;
use serde_json::Value;
use sindri_core::schema::SchemaValidator;
use sindri_core::types::Extension;
use std::path::Path;
use tracing::{debug, info};

/// Validate extension YAML files with JSON Schema validation
pub struct ExtensionValidator<'a> {
    schema_validator: &'a SchemaValidator,
}

impl<'a> ExtensionValidator<'a> {
    /// Create a new extension validator
    pub fn new(schema_validator: &'a SchemaValidator) -> Self {
        Self { schema_validator }
    }

    /// Validate an extension YAML file
    ///
    /// This performs three levels of validation:
    /// 1. JSON Schema validation against extension.schema.json
    /// 2. Deserialization into Extension struct
    /// 3. Additional semantic validation
    pub fn validate_file(&self, path: &Path) -> Result<Extension> {
        info!("Validating extension: {:?}", path);

        // Read file
        let content = std::fs::read_to_string(path)?;

        // Validate against schema and return extension
        self.validate_yaml(&content)
    }

    /// Validate extension YAML content
    ///
    /// Validates YAML content against JSON Schema and performs semantic validation
    pub fn validate_yaml(&self, yaml: &str) -> Result<Extension> {
        // Parse YAML to JSON Value for schema validation
        let value: Value = serde_yaml::from_str(yaml)?;

        // Validate against JSON Schema
        self.schema_validator.validate(&value, "extension")?;

        // Parse into Extension struct
        let extension: Extension = serde_yaml::from_str(yaml)?;

        // Additional semantic validation
        self.validate_extension(&extension)?;

        debug!(
            "Extension {} v{} is valid",
            extension.metadata.name, extension.metadata.version
        );

        Ok(extension)
    }

    /// Validate an Extension struct directly
    ///
    /// This performs validation on an already-parsed Extension struct,
    /// useful when the extension was loaded from a trusted source or
    /// when you want to validate modifications.
    pub fn validate_extension_struct(&self, extension: &Extension) -> Result<()> {
        // Serialize to JSON for schema validation
        let value = serde_json::to_value(extension)?;

        // Validate against JSON Schema
        self.schema_validator.validate(&value, "extension")?;

        // Additional semantic validation
        self.validate_extension(extension)?;

        Ok(())
    }

    /// Perform additional validation beyond schema
    ///
    /// This catches validation rules that cannot be expressed in JSON Schema,
    /// such as:
    /// - Version format validation
    /// - Name format validation
    /// - Install method consistency checks
    /// - Dependency validation
    fn validate_extension(&self, extension: &Extension) -> Result<()> {
        // Check version format (basic semver)
        if !Self::is_valid_semver(&extension.metadata.version) {
            anyhow::bail!(
                "Invalid version format: {}. Expected semantic version (e.g., 1.0.0)",
                extension.metadata.version
            );
        }

        // Check name format
        if !Self::is_valid_name(&extension.metadata.name) {
            anyhow::bail!(
                "Invalid extension name: {}. Must be lowercase with hyphens only",
                extension.metadata.name
            );
        }

        // Check install method has required config
        self.validate_install_config(extension)?;

        // Validate dependencies don't include self
        if extension
            .metadata
            .dependencies
            .contains(&extension.metadata.name)
        {
            anyhow::bail!(
                "Extension {} cannot depend on itself",
                extension.metadata.name
            );
        }

        // Validate docs if present
        if let Some(docs) = &extension.docs {
            self.validate_docs(docs)?;
        }

        // Validate BOM if present
        if let Some(bom) = &extension.bom {
            self.validate_bom(bom)?;
        }

        // Validate capabilities if present
        if let Some(capabilities) = &extension.capabilities {
            self.validate_capabilities(capabilities)?;
        }

        Ok(())
    }

    /// Validate install configuration
    ///
    /// Ensures that the install method has the corresponding configuration block
    fn validate_install_config(&self, extension: &Extension) -> Result<()> {
        use sindri_core::types::InstallMethod;

        match extension.install.method {
            InstallMethod::Mise => {
                if extension.install.mise.is_none() {
                    anyhow::bail!("Install method 'mise' requires 'mise' configuration block");
                }
            }
            InstallMethod::Apt => {
                if extension.install.apt.is_none() {
                    anyhow::bail!("Install method 'apt' requires 'apt' configuration block");
                }
            }
            InstallMethod::Binary => {
                if extension.install.binary.is_none() {
                    anyhow::bail!("Install method 'binary' requires 'binary' configuration block");
                }
            }
            InstallMethod::Script => {
                if extension.install.script.is_none() {
                    anyhow::bail!("Install method 'script' requires 'script' configuration block");
                }
            }
            InstallMethod::Npm | InstallMethod::NpmGlobal => {
                if extension.install.npm.is_none() {
                    anyhow::bail!("Install method 'npm' requires 'npm' configuration block");
                }
            }
            InstallMethod::Hybrid => {
                // Hybrid can have any combination, no specific requirements
            }
        }

        Ok(())
    }

    /// Validate Bill of Materials
    fn validate_bom(&self, bom: &sindri_core::types::BomConfig) -> Result<()> {
        // Ensure tool names are unique
        let mut seen_names = std::collections::HashSet::new();

        for tool in &bom.tools {
            if !seen_names.insert(&tool.name) {
                anyhow::bail!("Duplicate tool in BOM: {}", tool.name);
            }

            // Validate version format if not "dynamic"
            if let Some(version) = &tool.version {
                if version != "dynamic" && !Self::is_valid_version_or_pattern(version) {
                    anyhow::bail!("Invalid version format for tool {}: {}", tool.name, version);
                }
            }
        }

        Ok(())
    }

    /// Validate capabilities configuration
    fn validate_capabilities(
        &self,
        capabilities: &sindri_core::types::CapabilitiesConfig,
    ) -> Result<()> {
        // Validate project-init if enabled
        if let Some(project_init) = &capabilities.project_init {
            if project_init.enabled {
                if project_init.commands.is_empty() {
                    anyhow::bail!("project-init capability is enabled but has no commands defined");
                }

                // Validate state markers if present
                if !project_init.state_markers.is_empty() {
                    let mut seen_paths = std::collections::HashSet::new();
                    for marker in &project_init.state_markers {
                        if !seen_paths.insert(&marker.path) {
                            anyhow::bail!("Duplicate state marker path: {}", marker.path);
                        }
                    }
                }
            }
        }

        // Validate auth if present
        // Note: auth.methods is optional; if not specified, both api-key and cli-auth are accepted
        if let Some(auth) = &capabilities.auth {
            // No validation needed - methods can be empty to allow both authentication types
            let _ = auth; // Suppress unused variable warning
        }

        // Validate MCP if enabled
        if let Some(mcp) = &capabilities.mcp {
            if mcp.enabled && mcp.server.is_none() {
                anyhow::bail!("MCP capability is enabled but has no server configuration");
            }
        }

        Ok(())
    }

    /// Validate documentation metadata
    fn validate_docs(&self, docs: &sindri_core::types::DocsConfig) -> Result<()> {
        // Validate last-updated date format if present
        if let Some(date) = &docs.last_updated {
            if !Self::is_valid_date(date) {
                anyhow::bail!(
                    "Invalid docs.last-updated date format: {}. Expected YYYY-MM-DD",
                    date
                );
            }
        }

        // Validate usage sections have at least one example
        for section in &docs.usage {
            if section.examples.is_empty() {
                anyhow::bail!("Usage section '{}' has no examples", section.section);
            }
        }

        Ok(())
    }

    /// Check if string is a valid ISO date (YYYY-MM-DD)
    fn is_valid_date(date: &str) -> bool {
        let parts: Vec<&str> = date.split('-').collect();
        if parts.len() != 3 {
            return false;
        }
        let year = parts[0].parse::<u32>().ok();
        let month = parts[1].parse::<u32>().ok();
        let day = parts[2].parse::<u32>().ok();
        matches!((year, month, day), (Some(y), Some(m), Some(d)) if y >= 2020 && (1..=12).contains(&m) && (1..=31).contains(&d))
    }

    /// Check if string is valid semver (basic check: X.Y.Z)
    fn is_valid_semver(version: &str) -> bool {
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() != 3 {
            return false;
        }
        parts.iter().all(|p| p.parse::<u32>().is_ok())
    }

    /// Check if version is valid semver or pattern like ">=1.0.0"
    fn is_valid_version_or_pattern(version: &str) -> bool {
        // Allow special version keywords
        if matches!(version, "latest" | "dynamic" | "remote") {
            return true;
        }

        // Allow version patterns like "9.x", "1.2.x"
        if version.ends_with(".x") {
            let base = version.trim_end_matches(".x");
            let parts: Vec<&str> = base.split('.').collect();
            return parts.iter().all(|p| p.parse::<u32>().is_ok());
        }

        // Allow pre-release versions (e.g., "3.0.0-alpha", "1.2.3-beta.1")
        if version.contains('-') {
            let base_version = version.split('-').next().unwrap();
            return Self::is_valid_semver(base_version);
        }

        // Strip version operators
        let version_clean = version
            .trim_start_matches(">=")
            .trim_start_matches("<=")
            .trim_start_matches('>')
            .trim_start_matches('<')
            .trim_start_matches('=')
            .trim_start_matches('^')
            .trim_start_matches('~');

        Self::is_valid_semver(version_clean)
    }

    /// Check if name is valid (lowercase, hyphens, numbers)
    fn is_valid_name(name: &str) -> bool {
        if name.is_empty() {
            return false;
        }

        let first_char = name.chars().next().unwrap();
        if !first_char.is_ascii_lowercase() {
            return false;
        }

        name.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_semver() {
        assert!(ExtensionValidator::is_valid_semver("1.0.0"));
        assert!(ExtensionValidator::is_valid_semver("0.1.0"));
        assert!(ExtensionValidator::is_valid_semver("10.20.30"));
        assert!(!ExtensionValidator::is_valid_semver("1.0"));
        assert!(!ExtensionValidator::is_valid_semver("1.0.0-alpha"));
        assert!(!ExtensionValidator::is_valid_semver("v1.0.0"));
    }

    #[test]
    fn test_valid_version_or_pattern() {
        assert!(ExtensionValidator::is_valid_version_or_pattern("1.0.0"));
        assert!(ExtensionValidator::is_valid_version_or_pattern(">=1.0.0"));
        assert!(ExtensionValidator::is_valid_version_or_pattern("^1.2.3"));
        assert!(ExtensionValidator::is_valid_version_or_pattern("~2.0.0"));
        assert!(ExtensionValidator::is_valid_version_or_pattern("latest"));
        assert!(ExtensionValidator::is_valid_version_or_pattern("dynamic"));
        assert!(!ExtensionValidator::is_valid_version_or_pattern("invalid"));
    }

    #[test]
    fn test_valid_name() {
        assert!(ExtensionValidator::is_valid_name("python"));
        assert!(ExtensionValidator::is_valid_name("nodejs-devtools"));
        assert!(ExtensionValidator::is_valid_name("claude-flow-v2"));
        assert!(!ExtensionValidator::is_valid_name("Python"));
        assert!(!ExtensionValidator::is_valid_name("2fast"));
        assert!(!ExtensionValidator::is_valid_name(""));
        assert!(!ExtensionValidator::is_valid_name("under_score"));
    }

    #[test]
    fn test_validator_creation() {
        let schema_validator = SchemaValidator::new().unwrap();
        let _validator = ExtensionValidator::new(&schema_validator);

        // Basic smoke test
        assert!(schema_validator.has_schema("extension"));
    }

    #[test]
    fn test_validate_minimal_extension() {
        let schema_validator = SchemaValidator::new().unwrap();
        let validator = ExtensionValidator::new(&schema_validator);

        let yaml = r#"
metadata:
  name: test-extension
  version: 1.0.0
  description: A test extension for validation
  category: devops

install:
  method: script
  script:
    path: scripts/install.sh

validate:
  commands:
    - name: test-cmd
      versionFlag: "--version"
"#;

        let result = validator.validate_yaml(yaml);
        assert!(result.is_ok(), "Validation failed: {:?}", result.err());
    }

    #[test]
    fn test_validate_invalid_version() {
        let schema_validator = SchemaValidator::new().unwrap();
        let validator = ExtensionValidator::new(&schema_validator);

        let yaml = r#"
metadata:
  name: test-extension
  version: v1.0.0
  description: A test extension for validation
  category: devops

install:
  method: script
  script:
    path: scripts/install.sh

validate:
  commands:
    - name: test-cmd
"#;

        let result = validator.validate_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_invalid_name() {
        let schema_validator = SchemaValidator::new().unwrap();
        let validator = ExtensionValidator::new(&schema_validator);

        let yaml = r#"
metadata:
  name: Invalid_Name
  version: 1.0.0
  description: A test extension for validation
  category: devops

install:
  method: script
  script:
    path: scripts/install.sh

validate:
  commands:
    - name: test-cmd
"#;

        let result = validator.validate_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_missing_install_config() {
        let schema_validator = SchemaValidator::new().unwrap();
        let validator = ExtensionValidator::new(&schema_validator);

        let yaml = r#"
metadata:
  name: test-extension
  version: 1.0.0
  description: A test extension for validation
  category: devops

install:
  method: mise

validate:
  commands:
    - name: test-cmd
"#;

        let result = validator.validate_yaml(yaml);
        // Should fail because mise method requires mise config block
        assert!(result.is_err());
    }
}
