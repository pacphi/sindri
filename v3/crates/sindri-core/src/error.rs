//! Error types for sindri-core

use thiserror::Error;

/// Result type alias using sindri-core's Error type
pub type Result<T> = std::result::Result<T, Error>;

/// Core error types for Sindri
#[derive(Error, Debug)]
pub enum Error {
    /// Configuration file not found
    #[error("Configuration file not found: {path}")]
    ConfigNotFound { path: String },

    /// Invalid configuration format
    #[error("Invalid configuration format: {message}")]
    InvalidConfig { message: String },

    /// YAML parsing error
    #[error("YAML parsing error: {0}")]
    YamlParse(#[from] serde_yaml_ng::Error),

    /// JSON parsing error
    #[error("JSON parsing error: {0}")]
    JsonParse(#[from] serde_json::Error),

    /// Schema validation error
    #[error("Schema validation failed:\n{errors}")]
    SchemaValidation { errors: String },

    /// Schema not found
    #[error("Schema not found: {name}")]
    SchemaNotFound { name: String },

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Invalid semver version
    #[error("Invalid version format: {version}")]
    InvalidVersion { version: String },

    /// Missing required field
    #[error("Missing required field: {field}")]
    MissingField { field: String },

    /// Invalid provider
    #[error("Unknown provider: {provider}. Valid providers: docker, fly, devpod, e2b, kubernetes")]
    InvalidProvider { provider: String },

    /// Invalid extension
    #[error("Unknown extension: {extension}")]
    InvalidExtension { extension: String },

    /// Invalid profile
    #[error("Unknown profile: {profile}")]
    InvalidProfile { profile: String },

    /// Extension conflict
    #[error("Extension conflict: {ext1} conflicts with {ext2}")]
    ExtensionConflict { ext1: String, ext2: String },

    /// Circular dependency
    #[error("Circular dependency detected: {cycle}")]
    CircularDependency { cycle: String },

    /// Template rendering error
    #[error("Template error: {0}")]
    Template(String),
}

impl Error {
    /// Create a config not found error
    pub fn config_not_found(path: impl Into<String>) -> Self {
        Self::ConfigNotFound { path: path.into() }
    }

    /// Create an invalid config error
    pub fn invalid_config(message: impl Into<String>) -> Self {
        Self::InvalidConfig {
            message: message.into(),
        }
    }

    /// Create a schema validation error from a list of errors
    pub fn schema_validation(errors: Vec<String>) -> Self {
        Self::SchemaValidation {
            errors: errors.join("\n"),
        }
    }

    /// Create a schema not found error
    pub fn schema_not_found(name: impl Into<String>) -> Self {
        Self::SchemaNotFound { name: name.into() }
    }

    /// Create an invalid version error
    pub fn invalid_version(version: impl Into<String>) -> Self {
        Self::InvalidVersion {
            version: version.into(),
        }
    }

    /// Create a missing field error
    pub fn missing_field(field: impl Into<String>) -> Self {
        Self::MissingField {
            field: field.into(),
        }
    }

    /// Create an invalid provider error
    pub fn invalid_provider(provider: impl Into<String>) -> Self {
        Self::InvalidProvider {
            provider: provider.into(),
        }
    }

    /// Create an invalid extension error
    pub fn invalid_extension(extension: impl Into<String>) -> Self {
        Self::InvalidExtension {
            extension: extension.into(),
        }
    }

    /// Create an invalid profile error
    pub fn invalid_profile(profile: impl Into<String>) -> Self {
        Self::InvalidProfile {
            profile: profile.into(),
        }
    }

    /// Create an extension conflict error
    pub fn extension_conflict(ext1: impl Into<String>, ext2: impl Into<String>) -> Self {
        Self::ExtensionConflict {
            ext1: ext1.into(),
            ext2: ext2.into(),
        }
    }

    /// Create a circular dependency error
    pub fn circular_dependency(cycle: impl Into<String>) -> Self {
        Self::CircularDependency {
            cycle: cycle.into(),
        }
    }
}
