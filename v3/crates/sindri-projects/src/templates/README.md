# Template System Implementation

Complete template system for Sindri v3 Phase 7 project scaffolding.

## Architecture

The template system is organized into four focused modules:

### 1. `parser.rs` - YAML Structure Parsing
- Parses `project-templates.yaml` into strongly-typed Rust structures
- Defines `TemplateConfig`, `ProjectTemplate`, `DependencyConfig`, `DetectionRules`
- Handles both single and multiple detection patterns
- Provides validation and type-safe access to template data

### 2. `loader.rs` - Template Loading
- Loads templates from embedded YAML (compiled into binary via `include_str!`)
- Supports runtime YAML loading for testing/development
- Provides fallback mechanisms
- Caches loaded configuration for performance

### 3. `detector.rs` - Project Type Detection
- Auto-detects project types from names using regex patterns
- Handles ambiguous matches (e.g., "api-server" → [node, go, python])
- Resolves aliases (e.g., "nodejs" → "node", "py" → "python")
- Provides formatted suggestions for user selection
- Supports numeric choice resolution

### 4. `renderer.rs` - Template Rendering
- Renders template files with variable substitution
- Uses simple `{var}` syntax (not Tera `{{var}}`) to match bash implementation
- Supports multi-file creation from template definitions
- Generates CLAUDE.md with project-specific content
- Handles file path rendering (in case paths contain variables)

## Usage Examples

### Basic Template Loading

```rust
use sindri_projects::templates::{TemplateLoader, TemplateVars, TemplateRenderer};

// Load from embedded resources
let loader = TemplateLoader::from_embedded()?;

// Get a template
let template = loader.get_template("node").expect("node template exists");

// Render files
let vars = TemplateVars::new("my-app".to_string())
    .with_author("Alice".to_string());

let renderer = TemplateRenderer::new();
let files = renderer.render_files(template, &vars, target_dir)?;
```

### Project Type Detection

```rust
use sindri_projects::templates::{TemplateLoader, TypeDetector, parser::DetectionResult};

let loader = TemplateLoader::from_embedded()?;
let detector = TypeDetector::new(&loader);

// Detect from name
match detector.detect_from_name("my-rails-app") {
    DetectionResult::Single(type_name) => {
        println!("Detected: {}", type_name); // "rails"
    }
    DetectionResult::Ambiguous(types) => {
        // Multiple matches - show suggestions
        let formatted = detector.format_suggestions(&types);
        println!("Select one:\n{}", formatted);
    }
    DetectionResult::None => {
        println!("No match - use default");
    }
}

// Resolve aliases
assert_eq!(detector.resolve_alias("nodejs"), Some("node".to_string()));
assert_eq!(detector.resolve_alias("py"), Some("python".to_string()));
```

### High-Level Template Manager

```rust
use sindri_projects::templates::TemplateManager;
use camino::Utf8Path;

// Initialize manager
let manager = TemplateManager::new()?;

// Auto-detect type
let detection = manager.detect_type("my-api-server");

// Get template
let template = manager.get_template("node")?;

// Render complete project
let vars = TemplateVars::new("my-api-server".to_string());
let files = manager.render_project("node", &vars, Utf8Path::new("/path/to/project"))?;

// Generate CLAUDE.md
let claude_md = manager.generate_claude_md("node", &vars, Utf8Path::new("/path/to/project"))?;
```

## Template Variable Substitution

Templates use `{variable_name}` syntax for substitution:

```yaml
files:
  "package.json": |
    {
      "name": "{project_name}",
      "version": "1.0.0",
      "author": "{author}"
    }
```

Available variables:
- `{project_name}` - Project name
- `{author}` - Author name
- `{git_user_name}` - Git user name
- `{git_user_email}` - Git user email
- `{date}` - Current date (YYYY-MM-DD)
- `{year}` - Current year
- `{description}` - Project description
- `{license}` - License type (default: MIT)

## Detection Logic

From `project-templates.yaml`:

```yaml
detection_rules:
  name_patterns:
    # Unambiguous matches
    - pattern: ".*-?rails?-?.*"
      type: "rails"

    # Ambiguous matches (requires user selection)
    - pattern: ".*-?api.*"
      types: ["node", "go", "python"]

    - pattern: ".*-?web.*"
      types: ["node", "rails"]
```

Pattern matching is case-insensitive and uses regex.

## Template Structure

Each template in `project-templates.yaml` defines:

```yaml
templates:
  node:
    description: "Node.js application"
    aliases: ["nodejs", "javascript"]           # Alternative names
    extensions: ["nodejs"]                       # Sindri extensions to activate
    detection_patterns: ["node", "npm", "express"]  # Keywords for detection
    setup_commands:                              # Commands to run after creation
      - "npm init -y"
    dependencies:                                # Dependency management
      detect: "package.json"
      command: "npm install"
      requires: "npm"
      description: "Node.js dependencies"
    files:                                       # Template files to create
      "package.json": |
        {
          "name": "{project_name}",
          "version": "1.0.0"
        }
    claude_md_template: |                        # Custom CLAUDE.md template
      # {project_name}
      Node.js application
```

## Testing

Run template tests:

```bash
# Test all template functionality
cargo test -p sindri-projects --lib templates

# Test specific module
cargo test -p sindri-projects --lib templates::parser
cargo test -p sindri-projects --lib templates::detector
cargo test -p sindri-projects --lib templates::renderer
```

## Integration with Existing Types

The template system integrates with existing types in `sindri-projects`:

- Uses `anyhow::Result` for error handling
- Compatible with `ProjectTemplate` from `types.rs` (parallel structure)
- Can be used alongside git operations and enhancements
- Supports all project types from the v2 bash implementation

## Embedded Templates

Templates are embedded at compile time using `include_str!`:

```rust
const EMBEDDED_YAML: &str = include_str!("../../templates/project-templates.yaml");
```

This ensures templates are always available without external file dependencies.

## Migration from v2

This Rust implementation maintains compatibility with the v2 bash scripts:

| v2 Bash | v3 Rust |
|---------|---------|
| `detect_type_from_name()` | `TypeDetector::detect_from_name()` |
| `resolve_template_alias()` | `TypeDetector::resolve_alias()` |
| `get_type_suggestions()` | `TypeDetector::format_suggestions()` |
| `resolve_type_choice()` | `TypeDetector::resolve_choice()` |
| `load_project_template()` | `TemplateLoader::get_template()` |
| `resolve_template_variables()` | `TemplateRenderer::render_string()` |
| `create_template_files()` | `TemplateRenderer::render_files()` |

## Error Handling

The template system uses `anyhow::Result` for comprehensive error context:

```rust
use anyhow::{Context, Result};

let template = loader.get_template("node")
    .context("Failed to load node template")?;

let files = renderer.render_files(template, &vars, target_dir)
    .context("Failed to render template files")?;
```

This provides rich error messages for debugging and user feedback.
