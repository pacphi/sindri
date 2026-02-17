//! JSON Schema validation for Sindri configurations

use crate::error::{Error, Result};
use jsonschema::Validator;
use rust_embed::RustEmbed;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;
use tracing::debug;

/// Embedded schema files
#[derive(RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/../../schemas/"]
#[prefix = ""]
struct EmbeddedSchemas;

/// Schema validator with pre-compiled schemas
#[derive(Debug)]
pub struct SchemaValidator {
    /// Compiled schemas by name
    schemas: HashMap<String, Validator>,
}

/// Global schema validator instance
static VALIDATOR: OnceLock<SchemaValidator> = OnceLock::new();

impl SchemaValidator {
    /// Create a new schema validator with embedded schemas
    pub fn new() -> Result<Self> {
        let mut schemas = HashMap::new();

        // Load embedded schemas
        for file in EmbeddedSchemas::iter() {
            if file.ends_with(".schema.json") {
                let name = file.trim_end_matches(".schema.json").to_string();

                debug!("Loading embedded schema: {}", name);

                if let Some(content) = EmbeddedSchemas::get(&file) {
                    let json_str = std::str::from_utf8(&content.data).map_err(|_| {
                        Error::invalid_config(format!("Invalid UTF-8 in schema: {}", file))
                    })?;

                    let schema_value: Value = serde_json::from_str(json_str)?;

                    let compiled = jsonschema::validator_for(&schema_value).map_err(|e| {
                        Error::invalid_config(format!("Failed to compile schema {}: {}", name, e))
                    })?;

                    schemas.insert(name, compiled);
                }
            }
        }

        // If no embedded schemas found, use fallback schemas
        if schemas.is_empty() {
            debug!("No embedded schemas found, using fallback schemas");
            Self::load_fallback_schemas(&mut schemas)?;
        }

        Ok(Self { schemas })
    }

    /// Load from external schema directory (for development)
    pub fn from_directory(path: &std::path::Path) -> Result<Self> {
        let mut schemas = HashMap::new();

        if path.is_dir() {
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let file_path = entry.path();

                if file_path.extension().is_some_and(|e| e == "json") {
                    if let Some(name) = file_path.file_stem() {
                        let name = name
                            .to_string_lossy()
                            .trim_end_matches(".schema")
                            .to_string();

                        debug!("Loading schema from file: {:?}", file_path);

                        let content = std::fs::read_to_string(&file_path)?;
                        let schema_value: Value = serde_json::from_str(&content)?;

                        let compiled = jsonschema::validator_for(&schema_value).map_err(|e| {
                            Error::invalid_config(format!(
                                "Failed to compile schema {}: {}",
                                name, e
                            ))
                        })?;

                        schemas.insert(name, compiled);
                    }
                }
            }
        }

        if schemas.is_empty() {
            return Err(Error::schema_not_found(format!(
                "No schemas found in {:?}",
                path
            )));
        }

        Ok(Self { schemas })
    }

    /// Get the global validator instance
    pub fn global() -> &'static SchemaValidator {
        VALIDATOR.get_or_init(|| {
            SchemaValidator::new().expect("Failed to initialize global schema validator")
        })
    }

    /// Validate JSON value against a schema
    pub fn validate(&self, value: &Value, schema_name: &str) -> Result<()> {
        let schema = self
            .schemas
            .get(schema_name)
            .ok_or_else(|| Error::schema_not_found(schema_name))?;

        let errors: Vec<String> = schema
            .iter_errors(value)
            .map(|e| {
                let path = e.instance_path().to_string();
                if path.is_empty() {
                    format!("  - {}", e)
                } else {
                    format!("  - {}: {}", path, e)
                }
            })
            .collect();

        if !errors.is_empty() {
            return Err(Error::schema_validation(errors));
        }

        Ok(())
    }

    /// Validate YAML string against a schema
    pub fn validate_yaml(&self, yaml: &str, schema_name: &str) -> Result<()> {
        let value: Value = serde_yaml_ng::from_str(yaml)?;
        self.validate(&value, schema_name)
    }

    /// Validate a file against a schema
    pub fn validate_file(&self, path: &std::path::Path, schema_name: &str) -> Result<()> {
        let content = std::fs::read_to_string(path)?;

        // Determine format by extension
        let value: Value = if path.extension().is_some_and(|e| e == "json") {
            serde_json::from_str(&content)?
        } else {
            serde_yaml_ng::from_str(&content)?
        };

        self.validate(&value, schema_name)
    }

    /// Check if a schema exists
    pub fn has_schema(&self, name: &str) -> bool {
        self.schemas.contains_key(name)
    }

    /// List available schemas
    pub fn list_schemas(&self) -> Vec<&str> {
        self.schemas.keys().map(|s| s.as_str()).collect()
    }

    /// Load fallback schemas (minimal schemas for when embedded ones aren't available)
    fn load_fallback_schemas(schemas: &mut HashMap<String, Validator>) -> Result<()> {
        // Minimal sindri schema
        let sindri_schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "required": ["version", "name", "deployment", "extensions"],
            "properties": {
                "version": { "type": "string" },
                "name": { "type": "string", "pattern": "^[a-z][a-z0-9-]*$" },
                "deployment": {
                    "type": "object",
                    "required": ["provider"],
                    "properties": {
                        "provider": {
                            "type": "string",
                            "enum": ["docker", "docker-compose", "fly", "devpod", "e2b", "kubernetes", "runpod", "northflank"]
                        },
                        "image": { "type": "string" },
                        "resources": { "type": "object" },
                        "volumes": { "type": "object" }
                    }
                },
                "extensions": {
                    "type": "object",
                    "properties": {
                        "profile": { "type": "string" },
                        "active": { "type": "array", "items": { "type": "string" } },
                        "additional": { "type": "array", "items": { "type": "string" } },
                        "auto_install": { "type": "boolean" }
                    }
                },
                "secrets": { "type": "array" },
                "providers": { "type": "object" }
            }
        });

        // Minimal extension schema
        let extension_schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "required": ["metadata", "install", "validate"],
            "properties": {
                "metadata": {
                    "type": "object",
                    "required": ["name", "version", "description", "category"],
                    "properties": {
                        "name": { "type": "string", "pattern": "^[a-z][a-z0-9-]*$" },
                        "version": { "type": "string" },
                        "description": { "type": "string" },
                        "category": { "type": "string" },
                        "dependencies": { "type": "array", "items": { "type": "string" } }
                    }
                },
                "requirements": { "type": "object" },
                "install": {
                    "type": "object",
                    "required": ["method"],
                    "properties": {
                        "method": { "type": "string" }
                    }
                },
                "configure": { "type": "object" },
                "validate": { "type": "object" },
                "remove": { "type": "object" },
                "upgrade": { "type": "object" },
                "capabilities": { "type": "object" },
                "bom": { "type": "object" }
            }
        });

        let sindri_compiled = jsonschema::validator_for(&sindri_schema).map_err(|e| {
            Error::invalid_config(format!("Failed to compile fallback sindri schema: {}", e))
        })?;

        let extension_compiled = jsonschema::validator_for(&extension_schema).map_err(|e| {
            Error::invalid_config(format!(
                "Failed to compile fallback extension schema: {}",
                e
            ))
        })?;

        schemas.insert("sindri".to_string(), sindri_compiled);
        schemas.insert("extension".to_string(), extension_compiled);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_creation() {
        let validator = SchemaValidator::new().unwrap();
        assert!(validator.has_schema("sindri") || validator.has_schema("extension"));
    }

    #[test]
    fn test_validate_minimal_config() {
        let validator = SchemaValidator::new().unwrap();

        let config = serde_json::json!({
            "version": "3.0",
            "name": "test-project",
            "deployment": {
                "provider": "docker"
            },
            "extensions": {
                "profile": "minimal"
            }
        });

        let result = validator.validate(&config, "sindri");
        assert!(result.is_ok(), "Validation failed: {:?}", result);
    }

    #[test]
    fn test_validate_invalid_name() {
        let validator = SchemaValidator::new().unwrap();

        let config = serde_json::json!({
            "version": "3.0",
            "name": "Invalid-Name",  // Should be lowercase
            "deployment": {
                "provider": "docker"
            },
            "extensions": {
                "profile": "minimal"
            }
        });

        let result = validator.validate(&config, "sindri");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_yaml() {
        let validator = SchemaValidator::new().unwrap();

        let yaml = r#"
version: "3.0"
name: test-project
deployment:
  provider: docker
extensions:
  profile: minimal
"#;

        let result = validator.validate_yaml(yaml, "sindri");
        assert!(result.is_ok(), "YAML validation failed: {:?}", result);
    }

    // --- Error path tests ---

    #[test]
    fn test_validate_nonexistent_schema() {
        let validator = SchemaValidator::new().unwrap();
        let value = serde_json::json!({"key": "value"});
        let result = validator.validate(&value, "nonexistent-schema");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, Error::SchemaNotFound { .. }),
            "Expected SchemaNotFound, got: {:?}",
            err
        );
        assert!(err.to_string().contains("nonexistent-schema"));
    }

    #[test]
    fn test_validate_missing_required_fields() {
        let validator = SchemaValidator::new().unwrap();

        // Missing required fields: name, deployment, extensions
        let config = serde_json::json!({
            "version": "3.0"
        });

        let result = validator.validate(&config, "sindri");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, Error::SchemaValidation { .. }),
            "Expected SchemaValidation, got: {:?}",
            err
        );
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("required"),
            "Expected 'required' in error, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_validate_yaml_invalid_syntax() {
        let validator = SchemaValidator::new().unwrap();
        let bad_yaml = ":::\n  invalid: [[[yaml";
        let result = validator.validate_yaml(bad_yaml, "sindri");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_wrong_type_for_field() {
        let validator = SchemaValidator::new().unwrap();

        // version should be string, not number
        let config = serde_json::json!({
            "version": 3.0,
            "name": "test-project",
            "deployment": {
                "provider": "docker"
            },
            "extensions": {
                "profile": "minimal"
            }
        });

        let result = validator.validate(&config, "sindri");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, Error::SchemaValidation { .. }),
            "Expected SchemaValidation, got: {:?}",
            err
        );
    }

    #[test]
    fn test_from_directory_empty_dir() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let result = SchemaValidator::from_directory(temp_dir.path());
        assert!(result.is_err());
        match result {
            Err(Error::SchemaNotFound { .. }) => {} // expected
            Err(other) => panic!("Expected SchemaNotFound, got: {:?}", other),
            Ok(_) => panic!("Expected error, got Ok"),
        }
    }
}
