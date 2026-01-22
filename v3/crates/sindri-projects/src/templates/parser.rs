//! YAML template parsing and structure definitions.
//!
//! Parses project-templates.yaml into strongly-typed Rust structures.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete template configuration from YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateConfig {
    pub version: String,
    pub templates: HashMap<String, ProjectTemplate>,
    #[serde(default)]
    pub detection_rules: DetectionRules,
}

/// Individual project template definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectTemplate {
    pub description: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub detection_patterns: Vec<String>,
    #[serde(default)]
    pub setup_commands: Vec<String>,
    #[serde(default)]
    pub dependencies: Option<DependencyConfig>,
    #[serde(default)]
    pub files: HashMap<String, String>,
    #[serde(default)]
    pub claude_md_template: Option<String>,
}

/// Dependency detection and installation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyConfig {
    /// File(s) to detect for dependency existence (e.g., "package.json")
    pub detect: DetectPattern,
    /// Command to install/download dependencies
    pub command: String,
    /// Required tool/binary to run the command
    pub requires: String,
    /// Human-readable description
    pub description: String,
    /// Optional fetch command (e.g., "cargo fetch")
    #[serde(default)]
    pub fetch_command: Option<String>,
}

/// Pattern for detecting dependency files
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DetectPattern {
    Single(String),
    Multiple(Vec<String>),
}

impl DetectPattern {
    /// Get all patterns as a vector
    pub fn patterns(&self) -> Vec<String> {
        match self {
            DetectPattern::Single(s) => vec![s.clone()],
            DetectPattern::Multiple(v) => v.clone(),
        }
    }
}

/// Detection rules for auto-type selection
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DetectionRules {
    #[serde(default)]
    pub name_patterns: Vec<NamePattern>,
    #[serde(default)]
    pub framework_keywords: HashMap<String, Vec<String>>,
}

/// Name pattern for project type detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamePattern {
    /// Regex pattern to match against project name
    pub pattern: String,
    /// Single type if unambiguous
    #[serde(default)]
    pub r#type: Option<String>,
    /// Multiple types if ambiguous (requires user selection)
    #[serde(default)]
    pub types: Option<Vec<String>>,
}

impl NamePattern {
    /// Check if pattern matches the given name
    pub fn matches(&self, name: &str) -> bool {
        match regex::Regex::new(&self.pattern) {
            Ok(re) => re.is_match(&name.to_lowercase()),
            Err(_) => false,
        }
    }

    /// Get detection result
    pub fn detect_result(&self) -> DetectionResult {
        if let Some(t) = &self.r#type {
            DetectionResult::Single(t.clone())
        } else if let Some(types) = &self.types {
            DetectionResult::Ambiguous(types.clone())
        } else {
            DetectionResult::None
        }
    }
}

/// Result of type detection
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectionResult {
    /// Single unambiguous type detected
    Single(String),
    /// Multiple possible types (ambiguous)
    Ambiguous(Vec<String>),
    /// No match found
    None,
}

impl TemplateConfig {
    /// Parse template configuration from YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        serde_yaml::from_str(yaml).context("Failed to parse template YAML")
    }

    /// Get a template by name
    pub fn get_template(&self, name: &str) -> Option<&ProjectTemplate> {
        self.templates.get(name)
    }

    /// Resolve an alias to its canonical template name
    pub fn resolve_alias(&self, input: &str) -> Option<String> {
        let input_lower = input.to_lowercase();

        // Check if it's already a valid template name
        if self.templates.contains_key(&input_lower) {
            return Some(input_lower);
        }

        // Search through aliases
        for (name, template) in &self.templates {
            if template.aliases.iter().any(|a| a == &input_lower) {
                return Some(name.clone());
            }
        }

        None
    }

    /// Get all available template types
    pub fn template_types(&self) -> Vec<String> {
        let mut types: Vec<String> = self.templates.keys().cloned().collect();
        types.sort();
        types
    }

    /// Get template description
    pub fn get_description(&self, template_type: &str) -> Option<String> {
        self.get_template(template_type)
            .map(|t| t.description.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_config() {
        let yaml = r#"
version: "2.0"
templates:
  node:
    description: "Node.js application"
    aliases: ["nodejs", "javascript"]
    detection_patterns: ["node", "npm"]
    files:
      "package.json": '{"name": "{project_name}"}'
"#;

        let config = TemplateConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.version, "2.0");
        assert_eq!(config.templates.len(), 1);
        assert!(config.templates.contains_key("node"));

        let node = config.get_template("node").unwrap();
        assert_eq!(node.description, "Node.js application");
        assert_eq!(node.aliases, vec!["nodejs", "javascript"]);
    }

    #[test]
    fn test_resolve_alias() {
        let yaml = r#"
version: "2.0"
templates:
  python:
    description: "Python application"
    aliases: ["py", "python3"]
"#;

        let config = TemplateConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.resolve_alias("py"), Some("python".to_string()));
        assert_eq!(config.resolve_alias("python3"), Some("python".to_string()));
        assert_eq!(config.resolve_alias("python"), Some("python".to_string()));
        assert_eq!(config.resolve_alias("unknown"), None);
    }

    #[test]
    fn test_name_pattern_matching() {
        let pattern = NamePattern {
            pattern: ".*-?rails?-?.*".to_string(),
            r#type: Some("rails".to_string()),
            types: None,
        };

        assert!(pattern.matches("my-rails-app"));
        assert!(pattern.matches("rails-blog"));
        assert!(pattern.matches("todo-rail-service"));
        assert!(!pattern.matches("python-app"));
    }

    #[test]
    fn test_detection_result() {
        let single = NamePattern {
            pattern: ".*rails.*".to_string(),
            r#type: Some("rails".to_string()),
            types: None,
        };
        assert_eq!(
            single.detect_result(),
            DetectionResult::Single("rails".to_string())
        );

        let ambiguous = NamePattern {
            pattern: ".*api.*".to_string(),
            r#type: None,
            types: Some(vec!["node".to_string(), "go".to_string()]),
        };
        assert_eq!(
            ambiguous.detect_result(),
            DetectionResult::Ambiguous(vec!["node".to_string(), "go".to_string()])
        );
    }

    #[test]
    fn test_detect_pattern() {
        let single = DetectPattern::Single("package.json".to_string());
        assert_eq!(single.patterns(), vec!["package.json"]);

        let multiple = DetectPattern::Multiple(vec!["*.csproj".to_string(), "*.sln".to_string()]);
        assert_eq!(multiple.patterns(), vec!["*.csproj", "*.sln"]);
    }
}
