//! Project type auto-detection from names and patterns.
//!
//! Implements intelligent type detection logic using:
//! - Name pattern matching (e.g., "my-rails-app" → rails)
//! - Ambiguous detection (e.g., "api-server" → [node, go, python])
//! - Alias resolution (e.g., nodejs → node, py → python)

use super::loader::TemplateLoader;
use super::parser::{DetectionResult, TemplateConfig};

/// Project type detector
#[derive(Debug)]
pub struct TypeDetector<'a> {
    config: &'a TemplateConfig,
}

impl<'a> TypeDetector<'a> {
    /// Create a new detector from a loader
    pub fn new(loader: &'a TemplateLoader) -> Self {
        Self {
            config: loader.config(),
        }
    }

    /// Create a detector from a template config
    pub fn from_config(config: &'a TemplateConfig) -> Self {
        Self { config }
    }

    /// Detect project type from name using detection rules
    ///
    /// Returns:
    /// - `DetectionResult::Single(type)` if unambiguous match
    /// - `DetectionResult::Ambiguous(types)` if multiple matches (user selection needed)
    /// - `DetectionResult::None` if no match found
    pub fn detect_from_name(&self, project_name: &str) -> DetectionResult {
        let name_lower = project_name.to_lowercase();

        // Iterate through name patterns in order
        for pattern in &self.config.detection_rules.name_patterns {
            if pattern.matches(&name_lower) {
                return pattern.detect_result();
            }
        }

        DetectionResult::None
    }

    /// Check if a type exists in the templates
    pub fn is_valid_type(&self, template_type: &str) -> bool {
        self.config.get_template(template_type).is_some()
    }

    /// Resolve an alias to its canonical template name
    pub fn resolve_alias(&self, input: &str) -> Option<String> {
        self.config.resolve_alias(input)
    }

    /// Get suggestions with descriptions for ambiguous types
    pub fn get_suggestions(&self, types: &[String]) -> Vec<(String, String)> {
        types
            .iter()
            .filter_map(|t| self.config.get_description(t).map(|desc| (t.clone(), desc)))
            .collect()
    }

    /// Format suggestions for display (numbered list)
    pub fn format_suggestions(&self, types: &[String]) -> String {
        let suggestions = self.get_suggestions(types);
        suggestions
            .iter()
            .enumerate()
            .map(|(i, (name, desc))| format!("  {}) {:10} - {}", i + 1, name, desc))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Resolve a numeric choice to a type from a list
    ///
    /// If choice is "1", "2", etc., return the corresponding type from the list.
    /// Otherwise, return the choice as-is (user typed a name directly).
    pub fn resolve_choice(&self, choice: &str, types: &[String]) -> Option<String> {
        // Try to parse as number
        if let Ok(num) = choice.parse::<usize>() {
            if num > 0 && num <= types.len() {
                return Some(types[num - 1].clone());
            }
            return None;
        }

        // Otherwise, treat as a type name and try to resolve alias
        self.resolve_alias(choice)
    }

    /// Get all available template types
    pub fn available_types(&self) -> Vec<String> {
        self.config.template_types()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::templates::loader::TemplateLoader;

    fn create_test_loader() -> TemplateLoader {
        let yaml = r#"
version: "2.0"
templates:
  node:
    description: "Node.js application"
    aliases: ["nodejs", "javascript"]
    detection_patterns: ["node", "npm"]
  python:
    description: "Python application"
    aliases: ["py", "python3"]
    detection_patterns: ["python", "py"]
  rails:
    description: "Ruby on Rails application"
    aliases: ["ruby", "ror"]
    detection_patterns: ["rails", "ruby"]
  go:
    description: "Go application"
    aliases: ["golang"]
    detection_patterns: ["go", "golang"]
  spring:
    description: "Spring Boot application"
    aliases: ["spring-boot"]
    detection_patterns: ["spring", "java"]

detection_rules:
  name_patterns:
    - pattern: ".*-?rails?-?.*"
      type: "rails"
    - pattern: ".*-?django-?.*"
      type: "python"
    - pattern: ".*-?spring-?.*"
      type: "spring"
    - pattern: ".*-?api.*"
      types: ["node", "go", "python"]
    - pattern: ".*-?web.*"
      types: ["node", "rails"]
    - pattern: ".*-?service.*"
      types: ["go", "node", "spring"]
"#;
        TemplateLoader::from_yaml(yaml).unwrap()
    }

    #[test]
    fn test_detect_rails() {
        let loader = create_test_loader();
        let detector = TypeDetector::new(&loader);

        assert_eq!(
            detector.detect_from_name("my-rails-app"),
            DetectionResult::Single("rails".to_string())
        );
        assert_eq!(
            detector.detect_from_name("rails-blog"),
            DetectionResult::Single("rails".to_string())
        );
        assert_eq!(
            detector.detect_from_name("todo-rail-service"),
            DetectionResult::Single("rails".to_string())
        );
    }

    #[test]
    fn test_detect_ambiguous_api() {
        let loader = create_test_loader();
        let detector = TypeDetector::new(&loader);

        let result = detector.detect_from_name("my-api-server");
        match result {
            DetectionResult::Ambiguous(types) => {
                assert_eq!(types, vec!["node", "go", "python"]);
            }
            _ => panic!("Expected ambiguous result"),
        }
    }

    #[test]
    fn test_detect_ambiguous_web() {
        let loader = create_test_loader();
        let detector = TypeDetector::new(&loader);

        let result = detector.detect_from_name("my-web-app");
        match result {
            DetectionResult::Ambiguous(types) => {
                assert_eq!(types, vec!["node", "rails"]);
            }
            _ => panic!("Expected ambiguous result"),
        }
    }

    #[test]
    fn test_detect_no_match() {
        let loader = create_test_loader();
        let detector = TypeDetector::new(&loader);

        assert_eq!(
            detector.detect_from_name("random-project"),
            DetectionResult::None
        );
    }

    #[test]
    fn test_resolve_alias() {
        let loader = create_test_loader();
        let detector = TypeDetector::new(&loader);

        assert_eq!(detector.resolve_alias("nodejs"), Some("node".to_string()));
        assert_eq!(detector.resolve_alias("py"), Some("python".to_string()));
        assert_eq!(detector.resolve_alias("golang"), Some("go".to_string()));
        assert_eq!(detector.resolve_alias("node"), Some("node".to_string()));
        assert_eq!(detector.resolve_alias("unknown"), None);
    }

    #[test]
    fn test_is_valid_type() {
        let loader = create_test_loader();
        let detector = TypeDetector::new(&loader);

        assert!(detector.is_valid_type("node"));
        assert!(detector.is_valid_type("python"));
        assert!(detector.is_valid_type("rails"));
        assert!(!detector.is_valid_type("unknown"));
    }

    #[test]
    fn test_resolve_choice() {
        let loader = create_test_loader();
        let detector = TypeDetector::new(&loader);

        let types = vec!["node".to_string(), "go".to_string(), "python".to_string()];

        assert_eq!(
            detector.resolve_choice("1", &types),
            Some("node".to_string())
        );
        assert_eq!(detector.resolve_choice("2", &types), Some("go".to_string()));
        assert_eq!(
            detector.resolve_choice("3", &types),
            Some("python".to_string())
        );
        assert_eq!(detector.resolve_choice("4", &types), None);

        // Test name resolution with alias
        assert_eq!(
            detector.resolve_choice("nodejs", &types),
            Some("node".to_string())
        );
        assert_eq!(
            detector.resolve_choice("golang", &types),
            Some("go".to_string())
        );
    }

    #[test]
    fn test_get_suggestions() {
        let loader = create_test_loader();
        let detector = TypeDetector::new(&loader);

        let types = vec!["node".to_string(), "go".to_string()];
        let suggestions = detector.get_suggestions(&types);

        assert_eq!(suggestions.len(), 2);
        assert_eq!(
            suggestions[0],
            ("node".to_string(), "Node.js application".to_string())
        );
        assert_eq!(
            suggestions[1],
            ("go".to_string(), "Go application".to_string())
        );
    }

    #[test]
    fn test_format_suggestions() {
        let loader = create_test_loader();
        let detector = TypeDetector::new(&loader);

        let types = vec!["node".to_string(), "go".to_string()];
        let formatted = detector.format_suggestions(&types);

        assert!(formatted.contains("1) node"));
        assert!(formatted.contains("2) go"));
        assert!(formatted.contains("Node.js application"));
        assert!(formatted.contains("Go application"));
    }
}
