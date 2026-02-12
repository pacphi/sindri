//! Template loading, detection, and alias resolution for project scaffolding

use anyhow::{Context, Result};

use sindri_projects::templates::{
    parser::DetectionResult as TemplateDetectionResult, DependencyConfig, TemplateManager,
};

/// Internal detection result enum for type detection
pub(super) enum DetectionResult {
    Unambiguous(String),
    Ambiguous(Vec<String>),
}

/// Internal project template structure for rendering
/// Wraps the sindri-projects ProjectTemplate with additional context
#[derive(Debug)]
pub(super) struct InternalProjectTemplate {
    pub extensions: Vec<String>,
    pub setup_commands: Vec<String>,
    pub files: Vec<(String, String)>,
    pub claude_md_template: Option<String>,
    #[allow(dead_code)] // Used for future dependency installation enhancements
    pub dependencies: Option<DependencyConfig>,
}

/// Determine project type from name, explicit type, or interactive selection
pub(super) fn determine_project_type(
    name: &str,
    explicit_type: Option<String>,
    interactive: bool,
) -> Result<String> {
    // If type explicitly provided, use it (after resolving aliases)
    if let Some(project_type) = explicit_type {
        return Ok(resolve_template_alias(&project_type));
    }

    // If interactive mode, always prompt
    if interactive {
        return select_project_type_interactive(None);
    }

    // Auto-detect from project name
    let detected = detect_type_from_name(name);

    match detected {
        Some(DetectionResult::Unambiguous(t)) => {
            crate::output::info(&format!("Auto-detected project type: {}", t));
            Ok(t)
        }
        Some(DetectionResult::Ambiguous(types)) => {
            // Prompt user with suggestions
            select_project_type_interactive(Some(types))
        }
        None => {
            // No detection - show all available types
            select_project_type_interactive(None)
        }
    }
}

/// Detect project type from name using project-templates.yaml detection rules
///
/// Implements pattern matching with priority-based disambiguation:
/// - Exact pattern matches take precedence
/// - Multiple matches result in Ambiguous result for user selection
fn detect_type_from_name(name: &str) -> Option<DetectionResult> {
    // Load template configuration for detection rules
    let template_manager = match TemplateManager::new() {
        Ok(mgr) => mgr,
        Err(e) => {
            tracing::warn!("Failed to load template manager for detection: {}", e);
            return detect_type_from_name_fallback(name);
        }
    };

    // Use the template detector with YAML-driven rules
    let result = template_manager.detect_type(name);

    match result {
        TemplateDetectionResult::Single(type_name) => Some(DetectionResult::Unambiguous(type_name)),
        TemplateDetectionResult::Ambiguous(types) => Some(DetectionResult::Ambiguous(types)),
        TemplateDetectionResult::None => {
            // Fall back to simple pattern matching for basic cases
            detect_type_from_name_fallback(name)
        }
    }
}

/// Fallback detection using simple substring matching
/// Used when YAML-driven detection doesn't match
fn detect_type_from_name_fallback(name: &str) -> Option<DetectionResult> {
    let name_lower = name.to_lowercase();

    // Framework-specific patterns (high confidence)
    if name_lower.contains("rails") {
        return Some(DetectionResult::Unambiguous("rails".to_string()));
    }
    if name_lower.contains("django") {
        return Some(DetectionResult::Unambiguous("django".to_string()));
    }
    if name_lower.contains("spring") {
        return Some(DetectionResult::Unambiguous("spring".to_string()));
    }

    // Language-specific patterns
    if name_lower.contains("node") || name_lower.contains("npm") || name_lower.contains("express") {
        return Some(DetectionResult::Unambiguous("node".to_string()));
    }
    if name_lower.contains("python")
        || name_lower.contains("flask")
        || name_lower.contains("fastapi")
    {
        return Some(DetectionResult::Unambiguous("python".to_string()));
    }
    if name_lower.contains("rust") || name_lower.contains("cargo") {
        return Some(DetectionResult::Unambiguous("rust".to_string()));
    }
    if name_lower.contains("golang") {
        return Some(DetectionResult::Unambiguous("go".to_string()));
    }

    // Ambiguous patterns (multiple possible types)
    if name_lower.contains("api")
        || name_lower.contains("service")
        || name_lower.contains("microservice")
    {
        return Some(DetectionResult::Ambiguous(vec![
            "node".to_string(),
            "go".to_string(),
            "python".to_string(),
            "rust".to_string(),
        ]));
    }
    if name_lower.contains("web") {
        return Some(DetectionResult::Ambiguous(vec![
            "node".to_string(),
            "rails".to_string(),
            "django".to_string(),
        ]));
    }

    None
}

/// Resolve template alias to canonical name using project-templates.yaml aliases
///
/// Aliases defined in templates include:
/// - nodejs, javascript, js -> node
/// - py, python3 -> python
/// - golang -> go
/// - rs -> rust
/// - ruby, ror -> rails
pub(super) fn resolve_template_alias(input: &str) -> String {
    let input_lower = input.to_lowercase();

    // Try to use the template loader for alias resolution
    if let Ok(template_manager) = TemplateManager::new() {
        if let Some(resolved) = template_manager.resolve_alias(&input_lower) {
            return resolved;
        }
    }

    // Fallback alias resolution if template loading fails
    match input_lower.as_str() {
        "nodejs" | "javascript" | "js" => "node".to_string(),
        "py" | "python3" => "python".to_string(),
        "golang" => "go".to_string(),
        "rs" => "rust".to_string(),
        "ruby" | "ror" => "rails".to_string(),
        "csharp" | "c#" | ".net" => "dotnet".to_string(),
        "spring-boot" | "spring-web" => "spring".to_string(),
        "tf" | "infra" | "infrastructure" => "terraform".to_string(),
        "container" | "containerized" => "docker".to_string(),
        _ => input_lower,
    }
}

/// Interactive project type selection using available types from project-templates.yaml
fn select_project_type_interactive(suggestions: Option<Vec<String>>) -> Result<String> {
    use dialoguer::Select;

    let available_types = if let Some(types) = suggestions {
        types
    } else {
        // Load all available types from project-templates.yaml
        get_available_template_types()
    };

    // Build items with descriptions for better UX
    let items_with_desc = get_types_with_descriptions(&available_types);

    let selection = Select::new()
        .with_prompt("Select project type")
        .items(&items_with_desc)
        .default(0)
        .interact()?;

    Ok(available_types[selection].clone())
}

/// Get all available template types from project-templates.yaml
fn get_available_template_types() -> Vec<String> {
    match TemplateManager::new() {
        Ok(mgr) => mgr.available_types(),
        Err(_) => {
            // Fallback to default list if template loading fails
            vec![
                "node".to_string(),
                "python".to_string(),
                "rust".to_string(),
                "go".to_string(),
                "rails".to_string(),
                "django".to_string(),
                "spring".to_string(),
                "dotnet".to_string(),
                "terraform".to_string(),
                "docker".to_string(),
            ]
        }
    }
}

/// Get types with descriptions for interactive selection
fn get_types_with_descriptions(types: &[String]) -> Vec<String> {
    match TemplateManager::new() {
        Ok(mgr) => types
            .iter()
            .map(|t| {
                if let Some(template) = mgr.get_template(t) {
                    format!("{:12} - {}", t, template.description)
                } else {
                    t.clone()
                }
            })
            .collect(),
        Err(_) => types.to_vec(),
    }
}

/// Load project template from project-templates.yaml
///
/// Uses the embedded YAML configuration via sindri-projects TemplateLoader
pub(super) fn load_template(project_type: &str) -> Result<InternalProjectTemplate> {
    // Try to load from embedded project-templates.yaml
    let template_manager =
        TemplateManager::new().context("Failed to initialize template manager")?;

    if let Some(template) = template_manager.get_template(project_type) {
        // Convert sindri-projects template to internal format
        let files: Vec<(String, String)> = template
            .files
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        Ok(InternalProjectTemplate {
            extensions: template.extensions.clone(),
            setup_commands: template.setup_commands.clone(),
            files,
            claude_md_template: template.claude_md_template.clone(),
            dependencies: template.dependencies.clone(),
        })
    } else {
        // Fallback to generated template for unknown types
        tracing::warn!(
            "Template '{}' not found in project-templates.yaml, using fallback",
            project_type
        );
        load_template_fallback(project_type)
    }
}

/// Fallback template generation for types not in YAML
fn load_template_fallback(project_type: &str) -> Result<InternalProjectTemplate> {
    let (setup_commands, files, claude_template) = match project_type {
        "node" => (
            vec!["npm init -y".to_string()],
            vec![
                (".gitignore".to_string(), generate_gitignore("node")),
                ("package.json".to_string(), generate_package_json()),
            ],
            Some(generate_template_claude_md("node")),
        ),
        "python" => (
            vec![],
            vec![
                (".gitignore".to_string(), generate_gitignore("python")),
                (
                    "requirements.txt".to_string(),
                    "# Add dependencies here\n".to_string(),
                ),
            ],
            Some(generate_template_claude_md("python")),
        ),
        "rust" => (
            vec!["cargo init".to_string()],
            vec![(".gitignore".to_string(), generate_gitignore("rust"))],
            Some(generate_template_claude_md("rust")),
        ),
        "go" => (
            vec!["go mod init {project_name}".to_string()],
            vec![(".gitignore".to_string(), generate_gitignore("go"))],
            Some(generate_template_claude_md("go")),
        ),
        _ => (
            vec![],
            vec![(".gitignore".to_string(), generate_gitignore("generic"))],
            None,
        ),
    };

    Ok(InternalProjectTemplate {
        extensions: vec![project_type.to_string()],
        setup_commands,
        files,
        claude_md_template: claude_template,
        dependencies: None,
    })
}

/// Generate .gitignore content for project type
pub(super) fn generate_gitignore(project_type: &str) -> String {
    match project_type {
        "node" => r#"# Dependencies
node_modules/
jspm_packages/

# Build outputs
dist/
build/
*.min.js
*.min.css

# Logs
logs/
*.log
npm-debug.log*
yarn-debug.log*
yarn-error.log*

# Environment
.env
.env.local
.env.*.local

# IDE
.vscode/
.idea/
*.swp
*.swo
*~

# OS
.DS_Store
Thumbs.db

# Testing
coverage/
.nyc_output/

# Temporary
tmp/
temp/
"#
        .to_string(),
        "python" => r#"# Byte-compiled / optimized / DLL files
__pycache__/
*.py[cod]
*$py.class

# Virtual environments
venv/
env/
ENV/
.venv

# Distribution / packaging
dist/
build/
*.egg-info/

# Testing
.pytest_cache/
.coverage
htmlcov/

# IDE
.vscode/
.idea/
*.swp

# OS
.DS_Store
Thumbs.db

# Environment
.env
.env.local
"#
        .to_string(),
        "rust" => r#"# Build outputs
/target/
**/*.rs.bk
*.pdb

# Cargo
Cargo.lock

# IDE
.vscode/
.idea/
*.swp

# OS
.DS_Store
Thumbs.db
"#
        .to_string(),
        "go" => r#"# Binaries
*.exe
*.exe~
*.dll
*.so
*.dylib

# Test binary
*.test

# Output
/bin/
/dist/

# IDE
.vscode/
.idea/
*.swp

# OS
.DS_Store
Thumbs.db

# Go workspace
go.work
"#
        .to_string(),
        _ => r#"# IDE
.vscode/
.idea/
*.swp
*.swo
*~

# OS
.DS_Store
Thumbs.db

# Logs
*.log

# Environment
.env
.env.local
"#
        .to_string(),
    }
}

/// Generate package.json for Node.js projects
fn generate_package_json() -> String {
    r#"{
  "name": "{project_name}",
  "version": "1.0.0",
  "description": "",
  "main": "index.js",
  "scripts": {
    "start": "node index.js",
    "test": "echo \"Error: no test specified\" && exit 1"
  },
  "keywords": [],
  "author": "{author}",
  "license": "MIT"
}
"#
    .to_string()
}

/// Generate CLAUDE.md template for project type
fn generate_template_claude_md(project_type: &str) -> String {
    let setup_commands = match project_type {
        "node" => "```bash\nnpm install\nnpm start\n```",
        "python" => "```bash\npython -m venv venv\nsource venv/bin/activate  # or venv\\Scripts\\activate on Windows\npip install -r requirements.txt\n```",
        "rust" => "```bash\ncargo build\ncargo run\n```",
        "go" => "```bash\ngo mod download\ngo run .\n```",
        _ => "```bash\n# Add setup instructions here\n```",
    };

    let dev_commands = match project_type {
        "node" => "- `npm start` - Start the application\n- `npm test` - Run tests\n- `npm run dev` - Start with auto-reload (if configured)",
        "python" => "- `python main.py` - Run the application\n- `pytest` - Run tests\n- `python -m pip install -r requirements.txt` - Install dependencies",
        "rust" => "- `cargo run` - Run the application\n- `cargo test` - Run tests\n- `cargo build --release` - Build optimized binary",
        "go" => "- `go run .` - Run the application\n- `go test ./...` - Run tests\n- `go build` - Build binary",
        _ => "- [Add common commands here]",
    };

    format!(
        r#"# {{project_name}}

## Project Overview
This is a {} application for [brief description].

## Setup Instructions
{}

## Development Commands
{}

## Architecture Notes
[Add architectural decisions and patterns]

## Important Files
[List key files and their purposes]
"#,
        project_type, setup_commands, dev_commands
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- resolve_template_alias (fallback paths) ----

    #[test]
    fn test_resolve_alias_nodejs() {
        let result = resolve_template_alias("nodejs");
        assert_eq!(result, "node");
    }

    #[test]
    fn test_resolve_alias_js() {
        let result = resolve_template_alias("js");
        assert_eq!(result, "node");
    }

    #[test]
    fn test_resolve_alias_py() {
        let result = resolve_template_alias("py");
        assert_eq!(result, "python");
    }

    #[test]
    fn test_resolve_alias_golang() {
        let result = resolve_template_alias("golang");
        assert_eq!(result, "go");
    }

    #[test]
    fn test_resolve_alias_rs() {
        let result = resolve_template_alias("rs");
        assert_eq!(result, "rust");
    }

    #[test]
    fn test_resolve_alias_identity_for_canonical() {
        // Canonical names should pass through unchanged
        assert_eq!(resolve_template_alias("node"), "node");
        assert_eq!(resolve_template_alias("python"), "python");
        assert_eq!(resolve_template_alias("rust"), "rust");
        assert_eq!(resolve_template_alias("go"), "go");
    }

    #[test]
    fn test_resolve_alias_case_insensitive() {
        let result = resolve_template_alias("NodeJS");
        assert_eq!(result, "node");
    }

    // ---- detect_type_from_name_fallback ----

    #[test]
    fn test_detect_rails_from_name() {
        let result = detect_type_from_name_fallback("my-rails-app");
        assert!(matches!(result, Some(DetectionResult::Unambiguous(ref t)) if t == "rails"));
    }

    #[test]
    fn test_detect_django_from_name() {
        let result = detect_type_from_name_fallback("django-blog");
        assert!(matches!(result, Some(DetectionResult::Unambiguous(ref t)) if t == "django"));
    }

    #[test]
    fn test_detect_node_from_name() {
        let result = detect_type_from_name_fallback("node-backend");
        assert!(matches!(result, Some(DetectionResult::Unambiguous(ref t)) if t == "node"));
    }

    #[test]
    fn test_detect_python_from_name() {
        let result = detect_type_from_name_fallback("python-scraper");
        assert!(matches!(result, Some(DetectionResult::Unambiguous(ref t)) if t == "python"));
    }

    #[test]
    fn test_detect_rust_from_name() {
        let result = detect_type_from_name_fallback("rust-cli-tool");
        assert!(matches!(result, Some(DetectionResult::Unambiguous(ref t)) if t == "rust"));
    }

    #[test]
    fn test_detect_go_from_name() {
        let result = detect_type_from_name_fallback("golang-service");
        assert!(matches!(result, Some(DetectionResult::Unambiguous(ref t)) if t == "go"));
    }

    #[test]
    fn test_detect_ambiguous_api_name() {
        let result = detect_type_from_name_fallback("my-api");
        assert!(matches!(result, Some(DetectionResult::Ambiguous(ref types)) if types.len() > 1));
    }

    #[test]
    fn test_detect_ambiguous_web_name() {
        let result = detect_type_from_name_fallback("web-dashboard");
        assert!(matches!(result, Some(DetectionResult::Ambiguous(ref types)) if types.len() > 1));
    }

    #[test]
    fn test_detect_none_for_generic_name() {
        let result = detect_type_from_name_fallback("my-cool-thing");
        assert!(result.is_none());
    }

    // ---- generate_gitignore ----

    #[test]
    fn test_generate_gitignore_node() {
        let content = generate_gitignore("node");
        assert!(content.contains("node_modules/"));
        assert!(content.contains(".env"));
    }

    #[test]
    fn test_generate_gitignore_python() {
        let content = generate_gitignore("python");
        assert!(content.contains("__pycache__/"));
        assert!(content.contains("venv/"));
    }

    #[test]
    fn test_generate_gitignore_rust() {
        let content = generate_gitignore("rust");
        assert!(content.contains("/target/"));
    }

    #[test]
    fn test_generate_gitignore_go() {
        let content = generate_gitignore("go");
        assert!(content.contains("*.exe"));
        assert!(content.contains("go.work"));
    }

    #[test]
    fn test_generate_gitignore_generic_fallback() {
        let content = generate_gitignore("unknown");
        assert!(content.contains(".env"));
        assert!(content.contains(".DS_Store"));
    }
}
