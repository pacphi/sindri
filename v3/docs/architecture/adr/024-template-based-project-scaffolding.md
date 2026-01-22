# ADR 024: Template-Based Project Scaffolding

**Status**: Accepted
**Date**: 2026-01-22
**Deciders**: Core Team
**Related**: [ADR-003: Template-Based Configuration](003-template-based-configuration.md), [ADR-008: Extension Type System YAML Deserialization](008-extension-type-system-yaml-deserialization.md), [ADR-023: Phase 7 Project Management Architecture](023-phase-7-project-management-architecture.md), [Rust Migration Plan](../../planning/rust-cli-migration-v3.md#phase-7-project-management-weeks-20-21)

## Context

The Sindri CLI v3 requires a robust template system for project scaffolding that can:

1. Define project structures for different languages and frameworks
2. Support intelligent type detection from project names
3. Perform variable substitution in generated files
4. Integrate with the extension system for automatic tool setup
5. Provide both embedded (offline) and runtime templates

### Current v2 Template System

In v2, templates are defined in `project-templates.yaml`:

```yaml
templates:
  node:
    description: "Node.js application"
    aliases: ["nodejs", "javascript"]
    extensions: ["nodejs"]
    detection_patterns: ["node", "npm", "express", "js"]
    setup_commands:
      - "npm init -y"
    files:
      "package.json": |
        {
          "name": "{project_name}",
          "version": "1.0.0"
        }
      ".gitignore": "node_modules/\n"
```

**Strengths**:

- Simple YAML format
- Easy to add new templates
- Basic variable substitution with `{var}`
- Type detection via pattern matching

**Weaknesses**:

- Limited template logic (no conditionals, loops)
- String-based substitution prone to errors
- No template composition or inheritance
- Hard-coded file content in YAML (poor readability)
- No validation of generated output

### Requirements

**Template Definition**:

1. YAML-based template definitions for maintainability
2. Support 15+ project types (node, python, rust, go, java, rails, django, etc.)
3. Type aliases (e.g., "nodejs" → "node", "py" → "python")
4. Extension associations (auto-install relevant extensions)
5. Setup commands (e.g., `npm init`, `cargo init`)

**Type Detection**:

1. Pattern matching on project name
2. Ambiguity resolution (multiple matches)
3. Interactive fallback when no match
4. Explicit type override (`--type`)

**Variable Substitution**:

1. Project metadata (name, author, date)
2. Git configuration (user.name, user.email)
3. Conditional logic (e.g., include test config if testing enabled)
4. Template composition (include common snippets)

**Template Storage**:

1. Embedded in binary for offline use
2. Support for user-defined templates (future)
3. Template versioning and compatibility

## Decision

We implement a YAML-driven template system with Tera for variable substitution, embedded templates for offline use, and intelligent type detection algorithms.

### a) YAML Template Definition Format

**Decision**: Use structured YAML format with separate files for templates and file content.

**Template Schema** (`templates/project-templates.yaml`):

```yaml
version: "3.0"

# Template definitions
templates:
  node:
    description: "Node.js application"
    aliases: ["nodejs", "javascript", "js"]
    category: "language"

    # Extensions to auto-install
    extensions:
      - nodejs
      - prettier
      - eslint

    # Detection patterns for auto-detection
    detection:
      patterns:
        - "node"
        - "npm"
        - "express"
        - "typescript"
        - "ts"
      priority: 10 # Higher = higher priority when ambiguous

    # Setup commands (executed after scaffolding)
    setup:
      commands:
        - "npm init -y"
      dependencies:
        detect_file: "package.json"
        install_command: "npm install"
        requires_tool: "npm"
        description: "Node.js dependencies"

    # Files to generate (references to template files)
    files:
      - src: "node/package.json.tera"
        dest: "package.json"
      - src: "node/index.js.tera"
        dest: "index.js"
      - src: "gitignore/node.txt"
        dest: ".gitignore"
      - src: "node/README.md.tera"
        dest: "README.md"

    # CLAUDE.md template
    claude_template: "claude/node.md.tera"

  python:
    description: "Python application"
    aliases: ["py", "python3"]
    category: "language"

    extensions:
      - python
      - ruff
      - pytest

    detection:
      patterns:
        - "python"
        - "py"
        - "django"
        - "flask"
        - "fastapi"
        - "ml"
        - "data"
      priority: 10

    setup:
      commands:
        - "python3 -m venv venv"
        - "touch requirements.txt"
      dependencies:
        detect_file: "requirements.txt"
        install_command: "pip3 install -r requirements.txt"
        requires_tool: "pip3"
        description: "Python dependencies"

    files:
      - src: "python/main.py.tera"
        dest: "main.py"
        mode: "0755" # Executable
      - src: "python/requirements.txt"
        dest: "requirements.txt"
      - src: "gitignore/python.txt"
        dest: ".gitignore"
      - src: "python/README.md.tera"
        dest: "README.md"

    claude_template: "claude/python.md.tera"

  rust:
    description: "Rust application"
    aliases: ["rs"]
    category: "language"

    extensions:
      - rust
      - clippy

    detection:
      patterns: ["rust", "rs", "cargo"]
      priority: 10

    setup:
      commands:
        - "cargo init --name {project_name}"
      dependencies:
        detect_file: "Cargo.toml"
        install_command: "cargo build"
        requires_tool: "cargo"
        description: "Rust dependencies"

    files:
      - src: "gitignore/rust.txt"
        dest: ".gitignore"
      - src: "rust/README.md.tera"
        dest: "README.md"
      # Cargo.toml created by `cargo init`

    claude_template: "claude/rust.md.tera"

  # Framework templates
  nextjs:
    description: "Next.js application (React framework)"
    aliases: ["next"]
    category: "framework"
    parent: "node" # Inherit node template

    extensions:
      - nodejs
      - typescript
      - tailwind

    detection:
      patterns: ["next", "nextjs", "react-app"]
      priority: 15 # Higher than base "node"

    setup:
      commands:
        - "npx create-next-app@latest {project_name} --typescript --tailwind --app"

    # Override parent files
    files:
      - src: "nextjs/package.json.tera"
        dest: "package.json"

    claude_template: "claude/nextjs.md.tera"

  django:
    description: "Django web framework (Python)"
    category: "framework"
    parent: "python"

    extensions:
      - python
      - django-extensions

    detection:
      patterns: ["django", "django-app"]
      priority: 15

    setup:
      commands:
        - "pip3 install django"
        - "django-admin startproject {project_name} ."

    files:
      - src: "django/requirements.txt.tera"
        dest: "requirements.txt"

    claude_template: "claude/django.md.tera"

# Type detection configuration
detection:
  # Default type when no match
  default: "node"

  # Ambiguity threshold (0-100)
  # If multiple types match with score difference < threshold, consider ambiguous
  ambiguity_threshold: 5
```

**Reasoning**: Structured YAML provides:

- **Readability**: Clean separation of metadata and file content
- **Maintainability**: Easy to add new templates
- **Validation**: Schema can be validated with JSON Schema
- **Extensibility**: Parent templates enable composition
- **Flexibility**: File mode, conditional inclusion, etc.

### b) Type Detection Algorithm

**Decision**: Implement fuzzy pattern matching with priority-based disambiguation.

**Implementation** (`crates/sindri-project/src/detector.rs`):

```rust
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct DetectionConfig {
    pub patterns: Vec<String>,
    pub priority: u8,
}

#[derive(Debug)]
pub enum DetectionResult {
    /// Unambiguous match
    Unambiguous(String),
    /// Multiple matches with similar scores
    Ambiguous(Vec<String>),
    /// No matches
    None,
}

pub struct TypeDetector {
    templates: HashMap<String, DetectionConfig>,
    aliases: HashMap<String, String>,
}

impl TypeDetector {
    pub fn new() -> Result<Self, ProjectError> {
        let templates_yaml = include_str!("../templates/project-templates.yaml");
        let config: TemplateConfig = serde_yaml::from_str(templates_yaml)?;

        let mut templates = HashMap::new();
        let mut aliases = HashMap::new();

        for (name, template) in config.templates {
            // Store detection config
            templates.insert(name.clone(), template.detection);

            // Register aliases
            for alias in template.aliases {
                aliases.insert(alias, name.clone());
            }
        }

        Ok(Self { templates, aliases })
    }

    /// Detect project type from name using pattern matching
    pub fn detect_from_name(&self, name: &str) -> Result<DetectionResult, ProjectError> {
        let name_lower = name.to_lowercase();
        let mut scores: Vec<(String, u32)> = Vec::new();

        for (template_name, config) in &self.templates {
            let mut score = 0u32;

            for pattern in &config.patterns {
                let pattern_lower = pattern.to_lowercase();

                if name_lower.contains(&pattern_lower) {
                    // Exact substring match
                    score += 10;

                    // Bonus for word boundary match
                    if is_word_boundary_match(&name_lower, &pattern_lower) {
                        score += 5;
                    }

                    // Bonus for prefix match
                    if name_lower.starts_with(&pattern_lower) {
                        score += 3;
                    }
                }

                // Fuzzy match (edit distance)
                let distance = levenshtein(&name_lower, &pattern_lower);
                if distance <= 2 {
                    score += (3 - distance as u32);
                }
            }

            // Apply priority multiplier
            score = score.saturating_mul(config.priority as u32);

            if score > 0 {
                scores.push((template_name.clone(), score));
            }
        }

        // Sort by score descending
        scores.sort_by(|a, b| b.1.cmp(&a.1));

        match scores.as_slice() {
            [] => Ok(DetectionResult::None),
            [(name, _)] => Ok(DetectionResult::Unambiguous(name.clone())),
            [(name1, score1), (name2, score2), ..] => {
                // Check if top scores are close (ambiguous)
                let diff = score1.abs_diff(*score2);
                let threshold = 5; // From config

                if diff < threshold {
                    // Ambiguous - collect all types with similar scores
                    let mut similar = vec![name1.clone()];
                    for (name, score) in &scores[1..] {
                        if score1.abs_diff(*score) < threshold {
                            similar.push(name.clone());
                        }
                    }
                    Ok(DetectionResult::Ambiguous(similar))
                } else {
                    Ok(DetectionResult::Unambiguous(name1.clone()))
                }
            }
        }
    }

    /// Resolve type alias to canonical name
    pub fn resolve_alias(&self, alias: &str) -> Result<String, ProjectError> {
        let lower = alias.to_lowercase();

        // Check if it's already a valid type
        if self.templates.contains_key(&lower) {
            return Ok(lower);
        }

        // Check aliases
        if let Some(canonical) = self.aliases.get(&lower) {
            return Ok(canonical.clone());
        }

        Err(ProjectError::UnknownType(alias.to_string()))
    }

    /// Interactive type selection
    pub async fn select_interactive(
        &self,
        suggestions: Option<Vec<String>>,
    ) -> Result<String, ProjectError> {
        use dialoguer::Select;

        let items: Vec<String> = if let Some(sugg) = suggestions {
            sugg
        } else {
            // Show all types
            self.templates.keys().cloned().collect()
        };

        let selection = Select::new()
            .with_prompt("Select project type")
            .items(&items)
            .default(0)
            .interact()?;

        Ok(items[selection].clone())
    }
}

/// Check if pattern matches on word boundary
fn is_word_boundary_match(text: &str, pattern: &str) -> bool {
    let words: Vec<&str> = text.split(|c: char| !c.is_alphanumeric()).collect();
    words.iter().any(|w| w.contains(pattern))
}

/// Calculate Levenshtein distance between two strings
fn levenshtein(a: &str, b: &str) -> usize {
    let len_a = a.chars().count();
    let len_b = b.chars().count();

    if len_a == 0 {
        return len_b;
    }
    if len_b == 0 {
        return len_a;
    }

    let mut matrix = vec![vec![0usize; len_b + 1]; len_a + 1];

    for i in 0..=len_a {
        matrix[i][0] = i;
    }
    for j in 0..=len_b {
        matrix[0][j] = j;
    }

    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();

    for (i, ca) in a_chars.iter().enumerate() {
        for (j, cb) in b_chars.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            matrix[i + 1][j + 1] = std::cmp::min(
                std::cmp::min(
                    matrix[i][j + 1] + 1,     // deletion
                    matrix[i + 1][j] + 1      // insertion
                ),
                matrix[i][j] + cost           // substitution
            );
        }
    }

    matrix[len_a][len_b]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_node_project() {
        let detector = TypeDetector::new().unwrap();

        match detector.detect_from_name("my-express-app").unwrap() {
            DetectionResult::Unambiguous(t) => assert_eq!(t, "node"),
            _ => panic!("Expected unambiguous node detection"),
        }
    }

    #[test]
    fn test_detect_python_project() {
        let detector = TypeDetector::new().unwrap();

        match detector.detect_from_name("ml-model").unwrap() {
            DetectionResult::Unambiguous(t) => assert_eq!(t, "python"),
            _ => panic!("Expected unambiguous python detection"),
        }
    }

    #[test]
    fn test_ambiguous_detection() {
        let detector = TypeDetector::new().unwrap();

        // "api" could match multiple frameworks
        match detector.detect_from_name("api-server").unwrap() {
            DetectionResult::Ambiguous(types) => {
                assert!(types.len() > 1);
            }
            _ => panic!("Expected ambiguous detection"),
        }
    }

    #[test]
    fn test_resolve_alias() {
        let detector = TypeDetector::new().unwrap();

        assert_eq!(detector.resolve_alias("nodejs").unwrap(), "node");
        assert_eq!(detector.resolve_alias("py").unwrap(), "python");
        assert_eq!(detector.resolve_alias("rs").unwrap(), "rust");
    }

    #[test]
    fn test_levenshtein() {
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("node", "nodes"), 1);
        assert_eq!(levenshtein("python", "pythons"), 1);
    }
}
```

**Reasoning**: Fuzzy matching with priority provides:

- **Accuracy**: Multi-factor scoring (substring, word boundary, prefix, edit distance)
- **Disambiguation**: Priority system breaks ties
- **User Control**: Interactive fallback when ambiguous
- **Flexibility**: Easy to tune matching thresholds

### c) Variable Substitution with Tera

**Decision**: Use Tera template engine for powerful variable substitution with logic.

**Template Example** (`templates/files/node/package.json.tera`):

```json
{
  "name": "{{ project_name }}",
  "version": "1.0.0",
  "description": "{{ description }}",
  "author": "{{ author }} <{{ git_user_email }}>",
  "license": "{{ license }}",
  "main": "index.js",
  "scripts": {
    "start": "node index.js",
    "dev": "nodemon index.js",
    "test": "jest"
  },
  "dependencies": {},
  "devDependencies": {
    {% if include_typescript -%}
    "@types/node": "^20.0.0",
    "typescript": "^5.0.0",
    "ts-node": "^10.0.0",
    {% endif -%}
    "nodemon": "^3.0.0",
    "jest": "^29.0.0"
  },
  "engines": {
    "node": ">=20.0.0"
  }
}
```

**CLAUDE.md Template** (`templates/files/claude/node.md.tera`):

````markdown
# {{ project_name }}

**Created**: {{ date }}
**Author**: {{ author }}
**Type**: Node.js Application

## Project Overview

This is a Node.js application scaffolded by Sindri CLI.

## Setup Instructions

```bash
# Install dependencies
npm install

# Run in development mode
npm run dev

# Run tests
npm test
```
````

## Project Structure

```
{{ project_name }}/
├── index.js           # Main entry point
├── package.json       # Dependencies and scripts
├── .gitignore         # Git ignore rules
└── README.md          # Project documentation
```

## Development Commands

- `npm start` - Start the application
- `npm run dev` - Start with auto-reload (nodemon)
- `npm test` - Run tests with Jest

## Architecture Notes

{% if extensions | length > 0 -%}
**Installed Extensions**:
{% for ext in extensions -%}

- {{ ext }}
  {% endfor %}
  {% endif -%}

## Next Steps

1. Edit this file to add project-specific context
2. Implement core functionality in `index.js`
3. Add tests in `__tests__/` directory
4. Update `package.json` with actual dependencies

````

**Implementation** (`crates/sindri-project/src/scaffolder.rs`):
```rust
use tera::{Tera, Context};
use std::path::{Path, PathBuf};
use std::collections::HashMap;

pub struct ProjectScaffolder {
    tera: Tera,
}

impl ProjectScaffolder {
    pub fn new() -> Self {
        let mut tera = Tera::default();

        // Load all embedded templates
        let templates = load_embedded_templates();
        for (name, content) in templates {
            tera.add_raw_template(&name, &content)
                .expect("Failed to load template");
        }

        Self { tera }
    }

    /// Create project files from template
    pub async fn create_files(
        &self,
        project_dir: &Path,
        template: &ProjectTemplate,
        variables: &TemplateVariables,
    ) -> Result<(), ProjectError> {
        // Convert variables to Tera context
        let mut context = Context::new();
        for (key, value) in variables.iter() {
            context.insert(key, value);
        }

        // Create each file from template
        for file_spec in &template.files {
            let content = self.tera.render(&file_spec.src, &context)?;

            let dest_path = project_dir.join(&file_spec.dest);

            // Create parent directories
            if let Some(parent) = dest_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }

            // Write file
            tokio::fs::write(&dest_path, content).await?;

            // Set file mode (Unix only)
            #[cfg(unix)]
            if let Some(mode) = file_spec.mode {
                use std::os::unix::fs::PermissionsExt;
                let perms = std::fs::Permissions::from_mode(mode);
                std::fs::set_permissions(&dest_path, perms)?;
            }
        }

        Ok(())
    }
}

/// Load embedded template files
fn load_embedded_templates() -> HashMap<String, String> {
    let mut templates = HashMap::new();

    // Embed templates at compile time
    macro_rules! include_template {
        ($name:expr, $path:expr) => {
            templates.insert($name.to_string(), include_str!($path).to_string());
        };
    }

    // Node.js templates
    include_template!("node/package.json.tera", "../templates/files/node/package.json.tera");
    include_template!("node/index.js.tera", "../templates/files/node/index.js.tera");
    include_template!("node/README.md.tera", "../templates/files/node/README.md.tera");

    // Python templates
    include_template!("python/main.py.tera", "../templates/files/python/main.py.tera");
    include_template!("python/requirements.txt", "../templates/files/python/requirements.txt");
    include_template!("python/README.md.tera", "../templates/files/python/README.md.tera");

    // Rust templates
    include_template!("rust/README.md.tera", "../templates/files/rust/README.md.tera");

    // Gitignore templates
    include_template!("gitignore/node.txt", "../templates/files/gitignore/node.txt");
    include_template!("gitignore/python.txt", "../templates/files/gitignore/python.txt");
    include_template!("gitignore/rust.txt", "../templates/files/gitignore/rust.txt");

    // CLAUDE.md templates
    include_template!("claude/node.md.tera", "../templates/files/claude/node.md.tera");
    include_template!("claude/python.md.tera", "../templates/files/claude/python.md.tera");
    include_template!("claude/rust.md.tera", "../templates/files/claude/rust.md.tera");
    include_template!("claude/default.md.tera", "../templates/files/claude/default.md.tera");

    templates
}

#[derive(Debug, Clone)]
pub struct TemplateVariables {
    vars: HashMap<String, String>,
}

impl TemplateVariables {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
        }
    }

    pub fn with(mut self, key: &str, value: &str) -> Self {
        self.vars.insert(key.to_string(), value.to_string());
        self
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.vars.iter()
    }
}
````

**Reasoning**: Tera provides:

- **Rich Logic**: Conditionals, loops, filters, macros
- **Template Inheritance**: Base templates with blocks
- **Auto-Escaping**: Prevents injection attacks
- **Error Messages**: Clear error reporting for template errors
- **Performance**: Compiles templates once, renders fast
- **Ecosystem**: Well-maintained, widely used in Rust

### d) Embedded vs Runtime Templates

**Decision**: Embed templates in binary for offline use, with future support for user templates.

**Implementation**:

```rust
// Compile-time embedding
const TEMPLATES_YAML: &str = include_str!("../templates/project-templates.yaml");

pub struct TemplateLoader {
    config: TemplateConfig,
    user_templates_dir: Option<PathBuf>,
}

impl TemplateLoader {
    pub fn new() -> Result<Self, ProjectError> {
        // Load embedded templates
        let config: TemplateConfig = serde_yaml::from_str(TEMPLATES_YAML)?;

        // Check for user templates directory
        let user_templates_dir = dirs::config_dir()
            .map(|d| d.join("sindri").join("templates"));

        Ok(Self {
            config,
            user_templates_dir,
        })
    }

    pub fn load_template(&self, name: &str) -> Result<ProjectTemplate, ProjectError> {
        // Try user templates first (future)
        if let Some(ref dir) = self.user_templates_dir {
            if dir.exists() {
                if let Ok(template) = self.load_user_template(dir, name) {
                    return Ok(template);
                }
            }
        }

        // Fall back to embedded templates
        self.config.templates
            .get(name)
            .cloned()
            .ok_or_else(|| ProjectError::UnknownType(name.to_string()))
    }

    fn load_user_template(
        &self,
        dir: &Path,
        name: &str,
    ) -> Result<ProjectTemplate, ProjectError> {
        let template_file = dir.join(format!("{}.yaml", name));
        let content = std::fs::read_to_string(&template_file)?;
        let template: ProjectTemplate = serde_yaml::from_str(&content)?;
        Ok(template)
    }

    pub fn list_types(&self) -> Result<Vec<(String, String)>, ProjectError> {
        let mut types: Vec<_> = self.config.templates
            .iter()
            .map(|(name, template)| (name.clone(), template.description.clone()))
            .collect();

        types.sort_by(|a, b| a.0.cmp(&b.0));

        Ok(types)
    }
}
```

**Reasoning**: Embedded templates provide:

- **Offline Use**: No network required
- **Consistency**: Same templates across all installations
- **Performance**: No I/O for template loading
- **Simplicity**: No template versioning issues
- **Future Extension**: User templates can override embedded ones

## Consequences

### Positive

1. **Powerful Templates**: Tera enables conditionals, loops, inheritance
2. **Type Safety**: YAML validated against schema, Rust type checking
3. **Offline First**: Embedded templates work without network
4. **Intelligent Detection**: Fuzzy matching with priority provides good UX
5. **Extensible**: Easy to add new project types
6. **Maintainable**: YAML format is human-readable and version-controllable
7. **Fast**: Compile-time template embedding, runtime rendering is cached
8. **Flexible**: Support for file modes, conditional content, etc.

### Negative

1. **Binary Size**: Embedded templates increase binary size (~50KB per template)
2. **Template Updates**: Require CLI update to get new templates
3. **Learning Curve**: Tera syntax different from bash `{var}` substitution
4. **Migration Effort**: Must port v2 templates to new format
5. **Complexity**: Detection algorithm more complex than simple matching

### Neutral

1. **Template Engine Choice**: Tera vs Handlebars vs MiniJinja (Tera chosen)
2. **Embedding Strategy**: Compile-time vs runtime loading (compile-time chosen)
3. **Detection Algorithm**: Fuzzy vs exact matching (fuzzy chosen)

## Alternatives Considered

### 1. Handlebars Template Engine

**Description**: Use Handlebars instead of Tera for templates.

**Pros**:

- Widely known (JavaScript ecosystem)
- Simpler syntax for basic templates
- Smaller compiled size

**Cons**:

- Less powerful (no macros, limited logic)
- Separate crate for each feature (helpers, etc.)
- No template inheritance
- Less Rust-idiomatic

**Rejected**: Tera provides better feature set for complex templates.

### 2. Runtime Template Loading Only

**Description**: Load templates from filesystem at runtime, no embedding.

**Pros**:

- Smaller binary
- Templates can be updated without CLI update
- Users can customize templates

**Cons**:

- Requires network/filesystem access
- Version compatibility issues
- Offline use impossible
- Installation complexity

**Rejected**: Offline-first is critical for developer tool.

### 3. Exact String Matching for Detection

**Description**: Simple exact substring matching, no fuzzy logic.

**Pros**:

- Simpler implementation
- Faster (no Levenshtein distance)
- Predictable behavior

**Cons**:

- Misses typos ("pytho" won't match "python")
- No disambiguation for similar patterns
- Poor UX for edge cases

**Rejected**: Fuzzy matching provides significantly better UX.

### 4. Template Inheritance via YAML References

**Description**: Use YAML anchors/aliases instead of explicit `parent` field.

**Pros**:

- Native YAML feature
- No custom parsing logic

**Cons**:

- Less readable
- Hard to override specific fields
- YAML anchors limited to same file
- Confusing error messages

**Rejected**: Explicit `parent` field is clearer and more flexible.

### 5. No Type Detection

**Description**: Always require explicit `--type` flag, no auto-detection.

**Pros**:

- Simpler implementation
- No ambiguity
- Explicit is better than implicit

**Cons**:

- Poor UX for 90% of cases
- Requires memorizing type names
- Extra typing for every project

**Rejected**: Auto-detection improves UX significantly for common cases.

## Compliance

- ✅ YAML-driven template definitions
- ✅ Support for 15+ project types
- ✅ Type detection from project name
- ✅ Alias resolution (nodejs → node)
- ✅ Extension integration
- ✅ Tera template engine for variable substitution
- ✅ Embedded templates for offline use
- ✅ Interactive type selection
- ✅ Template inheritance (parent templates)
- ✅ File mode specification

## Notes

### Template File Organization

```
v3/crates/sindri-project/templates/
├── project-templates.yaml       # Template definitions
└── files/
    ├── node/
    │   ├── package.json.tera
    │   ├── index.js.tera
    │   ├── README.md.tera
    │   └── tsconfig.json.tera
    ├── python/
    │   ├── main.py.tera
    │   ├── requirements.txt
    │   ├── setup.py.tera
    │   └── README.md.tera
    ├── rust/
    │   └── README.md.tera
    ├── gitignore/
    │   ├── node.txt
    │   ├── python.txt
    │   ├── rust.txt
    │   ├── go.txt
    │   └── java.txt
    └── claude/
        ├── node.md.tera
        ├── python.md.tera
        ├── rust.md.tera
        ├── django.md.tera
        ├── nextjs.md.tera
        └── default.md.tera
```

### Supported Project Types

**Languages** (15 total):

1. `node` - Node.js/JavaScript/TypeScript
2. `python` - Python 3.x
3. `rust` - Rust with Cargo
4. `go` - Go modules
5. `java` - Java with Maven
6. `kotlin` - Kotlin
7. `scala` - Scala
8. `ruby` - Ruby with Bundler
9. `php` - PHP with Composer
10. `csharp` - C# with .NET
11. `cpp` - C++ with CMake
12. `swift` - Swift
13. `dart` - Dart/Flutter
14. `elixir` - Elixir
15. `clojure` - Clojure

**Frameworks** (additional):

- `rails` - Ruby on Rails
- `django` - Django (Python)
- `flask` - Flask (Python)
- `fastapi` - FastAPI (Python)
- `express` - Express.js (Node)
- `nextjs` - Next.js (React)
- `spring` - Spring Boot (Java)
- `laravel` - Laravel (PHP)

### Tera Filter Extensions

Custom Tera filters for project templates:

```rust
// Add custom filters to Tera
tera.register_filter("snake_case", snake_case_filter);
tera.register_filter("kebab_case", kebab_case_filter);
tera.register_filter("pascal_case", pascal_case_filter);
tera.register_filter("camel_case", camel_case_filter);

// Usage in templates:
// {{ project_name | snake_case }}  → my_project
// {{ project_name | kebab_case }}  → my-project
// {{ project_name | pascal_case }} → MyProject
// {{ project_name | camel_case }}  → myProject
```

## Related Decisions

- [ADR-003: Template-Based Configuration](003-template-based-configuration.md) - Tera template engine pattern
- [ADR-008: Extension Type System YAML Deserialization](008-extension-type-system-yaml-deserialization.md) - YAML parsing approach
- [ADR-023: Phase 7 Project Management Architecture](023-phase-7-project-management-architecture.md) - Overall architecture
- [ADR-025: Git Operations and Repository Management](025-git-operations-repository-management.md) - Git integration (next)
