//! Integration tests for the template system
//!
//! These tests verify the complete template workflow from loading to rendering.

use sindri_projects::templates::{
    parser::DetectionResult, TemplateLoader, TemplateManager, TemplateRenderer, TemplateVars,
    TypeDetector,
};

#[test]
fn test_load_embedded_templates() {
    let loader = TemplateLoader::from_embedded().expect("Failed to load embedded templates");
    let types = loader.template_types();

    // Verify expected templates exist
    assert!(types.contains(&"node".to_string()));
    assert!(types.contains(&"python".to_string()));
    assert!(types.contains(&"rust".to_string()));
    assert!(types.contains(&"rails".to_string()));
    assert!(types.len() >= 10, "Should have at least 10 templates");
}

#[test]
fn test_get_template_description() {
    let loader = TemplateLoader::from_embedded().unwrap();

    assert_eq!(
        loader.get_description("node"),
        Some("Node.js application".to_string())
    );
    assert_eq!(
        loader.get_description("python"),
        Some("Python application".to_string())
    );
    assert_eq!(loader.get_description("nonexistent"), None);
}

#[test]
fn test_resolve_aliases() {
    let loader = TemplateLoader::from_embedded().unwrap();

    // Test node aliases
    assert_eq!(loader.resolve_alias("nodejs"), Some("node".to_string()));
    assert_eq!(loader.resolve_alias("javascript"), Some("node".to_string()));

    // Test python aliases
    assert_eq!(loader.resolve_alias("py"), Some("python".to_string()));
    assert_eq!(loader.resolve_alias("python3"), Some("python".to_string()));

    // Test go aliases
    assert_eq!(loader.resolve_alias("golang"), Some("go".to_string()));

    // Test rust aliases
    assert_eq!(loader.resolve_alias("rs"), Some("rust".to_string()));

    // Test that canonical names still work
    assert_eq!(loader.resolve_alias("node"), Some("node".to_string()));
    assert_eq!(loader.resolve_alias("python"), Some("python".to_string()));
}

#[test]
fn test_type_detection_unambiguous() {
    let loader = TemplateLoader::from_embedded().unwrap();
    let detector = TypeDetector::new(&loader);

    // Test Rails detection
    assert_eq!(
        detector.detect_from_name("my-rails-app"),
        DetectionResult::Single("rails".to_string())
    );

    // Test Django detection
    assert_eq!(
        detector.detect_from_name("my-django-site"),
        DetectionResult::Single("django".to_string())
    );

    // Test Spring detection
    assert_eq!(
        detector.detect_from_name("my-spring-boot-app"),
        DetectionResult::Single("spring".to_string())
    );
}

#[test]
fn test_type_detection_ambiguous() {
    let loader = TemplateLoader::from_embedded().unwrap();
    let detector = TypeDetector::new(&loader);

    // Test API detection (ambiguous - could be node, go, python, etc.)
    match detector.detect_from_name("my-api-server") {
        DetectionResult::Ambiguous(types) => {
            assert!(types.contains(&"node".to_string()));
            assert!(types.contains(&"go".to_string()));
            assert!(types.contains(&"python".to_string()));
        }
        other => panic!("Expected Ambiguous, got: {:?}", other),
    }

    // Test web detection (ambiguous - could be node, rails, etc.)
    match detector.detect_from_name("my-web-app") {
        DetectionResult::Ambiguous(types) => {
            assert!(types.contains(&"node".to_string()));
            assert!(types.contains(&"rails".to_string()));
        }
        other => panic!("Expected Ambiguous, got: {:?}", other),
    }
}

#[test]
fn test_type_detection_none() {
    let loader = TemplateLoader::from_embedded().unwrap();
    let detector = TypeDetector::new(&loader);

    // Random project name that doesn't match any pattern
    assert_eq!(
        detector.detect_from_name("random-project-xyz"),
        DetectionResult::None
    );
}

#[test]
fn test_template_vars_creation() {
    let vars = TemplateVars::new("test-project".to_string())
        .with_author("Alice Smith".to_string())
        .with_description("A test project".to_string())
        .with_license("Apache-2.0".to_string());

    assert_eq!(vars.project_name, "test-project");
    assert_eq!(vars.author, "Alice Smith");
    assert_eq!(vars.description, "A test project");
    assert_eq!(vars.license, "Apache-2.0");
    assert!(!vars.year.is_empty());
    assert!(!vars.date.is_empty());
}

#[test]
fn test_render_string() {
    let renderer = TemplateRenderer::new();
    let vars = TemplateVars::new("my-app".to_string())
        .with_author("Bob".to_string())
        .with_git_user("Bob".to_string(), "bob@example.com".to_string());

    let template =
        "Project: {project_name}\nAuthor: {author}\nEmail: {git_user_email}\nYear: {year}";
    let result = renderer.render_string(template, &vars).unwrap();

    assert!(result.contains("Project: my-app"));
    assert!(result.contains("Author: Bob"));
    assert!(result.contains("Email: bob@example.com"));
    assert!(result.contains("Year:"));
}

#[test]
fn test_get_template_files() {
    let loader = TemplateLoader::from_embedded().unwrap();
    let node_template = loader.get_template("node").expect("node template exists");

    // Node template should have package.json and .gitignore
    assert!(node_template.files.contains_key("package.json"));
    assert!(node_template.files.contains_key(".gitignore"));

    // Verify package.json contains variables
    let package_json = &node_template.files["package.json"];
    assert!(package_json.contains("{project_name}"));
}

#[test]
fn test_template_manager() {
    let manager = TemplateManager::new().expect("Failed to create manager");

    // Test available types
    let types = manager.available_types();
    assert!(types.contains(&"node".to_string()));

    // Test get template
    assert!(manager.get_template("node").is_some());
    assert!(manager.get_template("nonexistent").is_none());

    // Test resolve alias
    assert_eq!(manager.resolve_alias("nodejs"), Some("node".to_string()));

    // Test detect type
    let detection = manager.detect_type("my-rails-app");
    assert_eq!(detection, DetectionResult::Single("rails".to_string()));
}

#[test]
fn test_choice_resolution() {
    let loader = TemplateLoader::from_embedded().unwrap();
    let detector = TypeDetector::new(&loader);

    let types = vec!["node".to_string(), "go".to_string(), "python".to_string()];

    // Test numeric choice
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

    // Test name choice with alias
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
fn test_format_suggestions() {
    let loader = TemplateLoader::from_embedded().unwrap();
    let detector = TypeDetector::new(&loader);

    let types = vec!["node".to_string(), "python".to_string()];
    let formatted = detector.format_suggestions(&types);

    assert!(formatted.contains("1) node"));
    assert!(formatted.contains("2) python"));
    assert!(formatted.contains("Node.js application"));
    assert!(formatted.contains("Python application"));
}

#[test]
fn test_template_has_claude_md() {
    let loader = TemplateLoader::from_embedded().unwrap();

    // Check that templates have claude_md_template
    let node = loader.get_template("node").unwrap();
    assert!(node.claude_md_template.is_some());

    let python = loader.get_template("python").unwrap();
    assert!(python.claude_md_template.is_some());

    // Verify template contains variables
    let claude_template = node.claude_md_template.as_ref().unwrap();
    assert!(claude_template.contains("{project_name}"));
}

#[test]
fn test_dependency_config() {
    let loader = TemplateLoader::from_embedded().unwrap();

    // Node has dependencies
    let node = loader.get_template("node").unwrap();
    assert!(node.dependencies.is_some());
    let deps = node.dependencies.as_ref().unwrap();
    assert_eq!(deps.detect.patterns(), vec!["package.json"]);
    assert_eq!(deps.command, "npm install");
    assert_eq!(deps.requires, "npm");

    // Python has dependencies
    let python = loader.get_template("python").unwrap();
    assert!(python.dependencies.is_some());
    let deps = python.dependencies.as_ref().unwrap();
    assert_eq!(deps.detect.patterns(), vec!["requirements.txt"]);
    assert_eq!(deps.command, "pip3 install -r requirements.txt");
}

#[test]
fn test_case_insensitive_detection() {
    let loader = TemplateLoader::from_embedded().unwrap();
    let detector = TypeDetector::new(&loader);

    // Detection should be case-insensitive
    assert_eq!(
        detector.detect_from_name("MY-RAILS-APP"),
        DetectionResult::Single("rails".to_string())
    );
    assert_eq!(
        detector.detect_from_name("My-Django-Site"),
        DetectionResult::Single("django".to_string())
    );
}
