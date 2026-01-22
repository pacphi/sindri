//! Core types for project management

use camino::Utf8PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Project type enumeration based on v2 templates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    /// Node.js application
    Node,
    /// Python application
    Python,
    /// Go application
    Go,
    /// Rust application
    Rust,
    /// Ruby on Rails application
    Rails,
    /// Django web application
    Django,
    /// Spring Boot application
    Spring,
    /// .NET application
    Dotnet,
    /// Terraform infrastructure
    Terraform,
    /// Docker/containerized application
    Docker,
    /// General purpose project
    General,
}

impl ProjectType {
    /// Get all available project types
    pub fn all() -> Vec<Self> {
        vec![
            Self::Node,
            Self::Python,
            Self::Go,
            Self::Rust,
            Self::Rails,
            Self::Django,
            Self::Spring,
            Self::Dotnet,
            Self::Terraform,
            Self::Docker,
            Self::General,
        ]
    }

    /// Get the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Node => "node",
            Self::Python => "python",
            Self::Go => "go",
            Self::Rust => "rust",
            Self::Rails => "rails",
            Self::Django => "django",
            Self::Spring => "spring",
            Self::Dotnet => "dotnet",
            Self::Terraform => "terraform",
            Self::Docker => "docker",
            Self::General => "general",
        }
    }

    /// Get description for the project type
    pub fn description(&self) -> &'static str {
        match self {
            Self::Node => "Node.js application",
            Self::Python => "Python application",
            Self::Go => "Go application",
            Self::Rust => "Rust application",
            Self::Rails => "Ruby on Rails application",
            Self::Django => "Django web application",
            Self::Spring => "Spring Boot application",
            Self::Dotnet => ".NET application",
            Self::Terraform => "Terraform infrastructure project",
            Self::Docker => "Dockerized application",
            Self::General => "General purpose project",
        }
    }

    /// Get aliases for this project type
    pub fn aliases(&self) -> Vec<&'static str> {
        match self {
            Self::Node => vec!["nodejs", "javascript", "js", "ts", "typescript"],
            Self::Python => vec!["py", "python3"],
            Self::Go => vec!["golang"],
            Self::Rust => vec!["rs"],
            Self::Rails => vec!["ruby", "ror"],
            Self::Django => vec![],
            Self::Spring => vec!["spring-boot", "spring-web", "spring-webmvc"],
            Self::Dotnet => vec!["csharp", "c#", ".net"],
            Self::Terraform => vec!["tf", "infra", "infrastructure"],
            Self::Docker => vec!["container", "containerized"],
            Self::General => vec![],
        }
    }

    /// Parse from string, checking aliases
    pub fn from_str_with_aliases(s: &str) -> Option<Self> {
        let s_lower = s.to_lowercase();

        for project_type in Self::all() {
            if project_type.as_str() == s_lower {
                return Some(project_type);
            }

            for alias in project_type.aliases() {
                if alias == s_lower {
                    return Some(project_type);
                }
            }
        }

        None
    }
}

impl std::fmt::Display for ProjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for ProjectType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_str_with_aliases(s).ok_or_else(|| {
            format!(
                "Unknown project type: {}. Valid types: {}",
                s,
                Self::all()
                    .iter()
                    .map(|t| t.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
    }
}

/// Project template configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectTemplate {
    /// Project type
    pub project_type: ProjectType,

    /// Human-readable description
    pub description: String,

    /// Type aliases for detection
    #[serde(default)]
    pub aliases: Vec<String>,

    /// Extensions to activate
    #[serde(default)]
    pub extensions: Vec<String>,

    /// Detection patterns for auto-type selection
    #[serde(default)]
    pub detection_patterns: Vec<String>,

    /// Setup commands to run after project creation
    #[serde(default)]
    pub setup_commands: Vec<String>,

    /// Dependency configuration
    #[serde(default)]
    pub dependencies: Option<DependencyConfig>,

    /// Template files to create
    #[serde(default)]
    pub files: HashMap<String, String>,

    /// Claude.md template
    #[serde(default)]
    pub claude_md_template: Option<String>,
}

/// Dependency configuration for a project template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyConfig {
    /// File(s) to detect (e.g., "package.json", ["*.csproj", "*.sln"])
    #[serde(deserialize_with = "deserialize_string_or_vec")]
    pub detect: Vec<String>,

    /// Command to install dependencies
    pub command: String,

    /// Required tool (e.g., "npm", "pip3")
    pub requires: String,

    /// Description for logging
    #[serde(default)]
    pub description: Option<String>,

    /// Command to fetch dependencies without building (optional)
    #[serde(default)]
    pub fetch_command: Option<String>,
}

/// Custom deserializer for string or vec
fn deserialize_string_or_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Deserialize};

    struct StringOrVec;

    impl<'de> de::Visitor<'de> for StringOrVec {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("string or list of strings")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(vec![value.to_owned()])
        }

        fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            Deserialize::deserialize(de::value::SeqAccessDeserializer::new(seq))
        }
    }

    deserializer.deserialize_any(StringOrVec)
}

/// Project metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    /// Project name
    pub name: String,

    /// Project type
    pub project_type: ProjectType,

    /// Project directory path
    pub path: Utf8PathBuf,

    /// Git repository URL (if cloned)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_url: Option<String>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last modified timestamp
    pub updated_at: DateTime<Utc>,

    /// Git configuration used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_config: Option<GitConfig>,

    /// Extensions activated
    #[serde(default)]
    pub extensions: Vec<String>,

    /// Whether enhancements were applied
    #[serde(default)]
    pub enhanced: bool,
}

/// Template variables for rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariables {
    /// Project name
    pub project_name: String,

    /// Author name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// Git user name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_user_name: Option<String>,

    /// Git user email
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_user_email: Option<String>,

    /// Current date (YYYY-MM-DD)
    pub date: String,

    /// Current year
    pub year: String,

    /// Project description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// License type
    #[serde(default = "default_license")]
    pub license: String,

    /// Additional custom variables
    #[serde(flatten)]
    pub custom: HashMap<String, serde_json::Value>,
}

fn default_license() -> String {
    "MIT".to_string()
}

impl Default for TemplateVariables {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            project_name: String::new(),
            author: None,
            git_user_name: None,
            git_user_email: None,
            date: now.format("%Y-%m-%d").to_string(),
            year: now.format("%Y").to_string(),
            description: None,
            license: default_license(),
            custom: HashMap::new(),
        }
    }
}

/// Git configuration for a project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitConfig {
    /// Git user name
    pub name: String,

    /// Git user email
    pub email: String,

    /// Default branch name
    #[serde(default = "default_branch")]
    pub default_branch: String,
}

fn default_branch() -> String {
    "main".to_string()
}

impl GitConfig {
    /// Create a new Git configuration
    pub fn new(name: impl Into<String>, email: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            email: email.into(),
            default_branch: default_branch(),
        }
    }

    /// Validate the git configuration
    pub fn validate(&self) -> Result<(), crate::error::Error> {
        if self.name.trim().is_empty() {
            return Err(crate::error::Error::invalid_git_config(
                "Git user name cannot be empty",
            ));
        }

        if self.email.trim().is_empty() {
            return Err(crate::error::Error::invalid_git_config(
                "Git user email cannot be empty",
            ));
        }

        // Basic email validation
        if !self.email.contains('@') {
            return Err(crate::error::Error::invalid_git_config(
                "Git user email must contain @",
            ));
        }

        Ok(())
    }
}

/// Enhancement options for Claude tools setup
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnhancementOptions {
    /// Skip all tool initialization
    #[serde(default)]
    pub skip_tools: bool,

    /// Skip Claude authentication check
    #[serde(default)]
    pub skip_auth_check: bool,

    /// Git configuration override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_config: Option<GitConfig>,

    /// Additional extensions to activate
    #[serde(default)]
    pub additional_extensions: Vec<String>,

    /// Whether to run in interactive mode
    #[serde(default)]
    pub interactive: bool,
}

impl EnhancementOptions {
    /// Create new enhancement options
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether to skip tools
    pub fn with_skip_tools(mut self, skip: bool) -> Self {
        self.skip_tools = skip;
        self
    }

    /// Set git configuration
    pub fn with_git_config(mut self, config: GitConfig) -> Self {
        self.git_config = Some(config);
        self
    }

    /// Add an extension to activate
    pub fn with_extension(mut self, extension: impl Into<String>) -> Self {
        self.additional_extensions.push(extension.into());
        self
    }

    /// Set interactive mode
    pub fn with_interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_type_from_str() {
        assert_eq!("node".parse::<ProjectType>().unwrap(), ProjectType::Node);
        assert_eq!("nodejs".parse::<ProjectType>().unwrap(), ProjectType::Node);
        assert_eq!("py".parse::<ProjectType>().unwrap(), ProjectType::Python);
        assert_eq!("golang".parse::<ProjectType>().unwrap(), ProjectType::Go);
    }

    #[test]
    fn test_project_type_all() {
        let all = ProjectType::all();
        assert!(all.len() >= 11);
        assert!(all.contains(&ProjectType::Node));
        assert!(all.contains(&ProjectType::Python));
    }

    #[test]
    fn test_git_config_validation() {
        let valid = GitConfig::new("John Doe", "john@example.com");
        assert!(valid.validate().is_ok());

        let invalid_name = GitConfig::new("", "john@example.com");
        assert!(invalid_name.validate().is_err());

        let invalid_email = GitConfig::new("John Doe", "invalid-email");
        assert!(invalid_email.validate().is_err());
    }

    #[test]
    fn test_template_variables_default() {
        let vars = TemplateVariables::default();
        assert_eq!(vars.license, "MIT");
        assert!(!vars.date.is_empty());
        assert!(!vars.year.is_empty());
    }

    #[test]
    fn test_enhancement_options_builder() {
        let opts = EnhancementOptions::new()
            .with_skip_tools(true)
            .with_git_config(GitConfig::new("Test", "test@example.com"))
            .with_extension("nodejs")
            .with_interactive(true);

        assert!(opts.skip_tools);
        assert!(opts.git_config.is_some());
        assert_eq!(opts.additional_extensions.len(), 1);
        assert!(opts.interactive);
    }

    #[test]
    fn test_dependency_config_deserialize() {
        let json = r#"{
            "detect": "package.json",
            "command": "npm install",
            "requires": "npm"
        }"#;

        let config: DependencyConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.detect, vec!["package.json"]);

        let json_array = r#"{
            "detect": ["*.csproj", "*.sln"],
            "command": "dotnet restore",
            "requires": "dotnet"
        }"#;

        let config: DependencyConfig = serde_json::from_str(json_array).unwrap();
        assert_eq!(config.detect, vec!["*.csproj", "*.sln"]);
    }
}
