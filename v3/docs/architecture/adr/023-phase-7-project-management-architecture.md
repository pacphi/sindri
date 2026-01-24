# ADR 023: Phase 7 Project Management Architecture

**Status**: Accepted
**Date**: 2026-01-22
**Deciders**: Core Team
**Related**: [ADR-001: Rust Migration Workspace Architecture](001-rust-migration-workspace-architecture.md), [ADR-003: Template-Based Configuration](003-template-based-configuration.md), [ADR-008: Extension Type System YAML Deserialization](008-extension-type-system-yaml-deserialization.md), [ADR-011: Multi-Method Extension Installation](011-multi-method-extension-installation.md), [Rust Migration Plan](../../planning/active/rust-cli-migration-v3.md#phase-7-project-management-weeks-20-21)

## Context

The Sindri CLI v3 requires robust project management capabilities to create and clone development projects with intelligent scaffolding. Unlike v2's bash-based approach, v3 needs a type-safe, extensible system that integrates with the broader Rust architecture while maintaining compatibility with existing workflows.

### Current v2 Bash Implementation

In v2, project management is handled by bash scripts:

**Strengths**:

1. **Template-Driven**: YAML templates define project types (node, python, go, rust, etc.)
2. **Auto-Detection**: Intelligent type detection from project name patterns
3. **Extension Integration**: Automatically activates relevant extensions
4. **Git Integration**: Initializes repos with proper configuration
5. **Claude Enhancement**: Creates CLAUDE.md files with project context
6. **Fork Support**: GitHub fork workflow with remote configuration

**Weaknesses**:

1. **No Type Safety**: String-based processing, easy to introduce bugs
2. **Limited Error Handling**: Difficult to recover from partial failures
3. **Hard to Test**: Bash testing infrastructure is limited
4. **No Async**: Sequential operations, slow for network calls
5. **String Template Engine**: Basic variable substitution only
6. **Maintenance Burden**: Complex bash functions hard to refactor

### Requirements from Phase 7 Specification

The [Rust Migration Plan Phase 7](../../planning/active/rust-cli-migration-v3.md#phase-7-project-management-weeks-20-21) defines:

**Commands**:

- `sindri new <name>` - Create new project from template
- `sindri new <name> --type <type>` - Explicit type selection
- `sindri new <name> --interactive` - Interactive type selection
- `sindri new <name> --git-name/--git-email` - Git configuration
- `sindri clone <repo>` - Clone repository with enhancements
- `sindri clone <repo> --fork` - Fork and clone repository
- `sindri clone <repo> --feature <branch>` - Create feature branch

**Core Features**:

1. **Template System**: YAML-driven project scaffolding
2. **Type Detection**: Intelligent detection from project names
3. **Extension Integration**: Auto-install relevant extensions
4. **Git Operations**: Clone, fork, remote configuration
5. **Variable Substitution**: Template variable replacement
6. **Project Enhancement**: CLAUDE.md generation, hooks setup

### User Experience Goals

1. **Single Command Creation**: `sindri new my-app` should "just work"
2. **Intelligent Defaults**: Auto-detect project type from name
3. **Flexible Override**: Easy to specify type explicitly
4. **Git-First**: All projects start with proper Git configuration
5. **Extension-Aware**: Automatically set up relevant tooling
6. **Fast**: Async operations for network calls

## Decision

We implement a comprehensive project management architecture with four key components organized in the `sindri-project` crate.

### a) Overall Architecture

**Decision**: Create dedicated `sindri-project` crate with modular structure separating concerns.

**Architecture**:

```
v3/crates/sindri-project/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public API
│   ├── template.rs         # Template loading and parsing
│   ├── detector.rs         # Type detection algorithms
│   ├── scaffolder.rs       # Project file creation
│   ├── git.rs              # Git operations (clone, fork, config)
│   ├── enhancer.rs         # CLAUDE.md, hooks, extensions
│   └── error.rs            # Error types
├── templates/              # Embedded template definitions
│   ├── project-templates.yaml
│   └── files/              # Template file snippets
│       ├── gitignore/
│       │   ├── node.txt
│       │   ├── python.txt
│       │   └── rust.txt
│       └── claude/
│           ├── node.md.tera
│           ├── python.md.tera
│           └── default.md.tera
└── tests/
    ├── integration/
    │   ├── test_new_project.rs
    │   └── test_clone_project.rs
    └── fixtures/
        └── test-templates.yaml
```

**Public API** (`lib.rs`):

```rust
// crates/sindri-project/src/lib.rs

use std::path::PathBuf;
use semver::Version;

pub mod template;
pub mod detector;
pub mod scaffolder;
pub mod git;
pub mod enhancer;
pub mod error;

pub use error::{ProjectError, Result};

/// Project creation configuration
#[derive(Debug, Clone)]
pub struct NewProjectConfig {
    /// Project name
    pub name: String,

    /// Project type (node, python, rust, etc.)
    pub project_type: Option<String>,

    /// Enable interactive type selection
    pub interactive: bool,

    /// Git user name override
    pub git_name: Option<String>,

    /// Git user email override
    pub git_email: Option<String>,

    /// Skip extension installation
    pub skip_extensions: bool,

    /// Skip tool initialization
    pub skip_tools: bool,

    /// Base directory for projects
    pub base_dir: Option<PathBuf>,
}

/// Clone/fork configuration
#[derive(Debug, Clone)]
pub struct CloneProjectConfig {
    /// Repository URL
    pub repo_url: String,

    /// Fork before cloning
    pub fork: bool,

    /// Specific branch to checkout
    pub branch: Option<String>,

    /// Clone depth (for shallow clones)
    pub depth: Option<u32>,

    /// Feature branch to create after clone
    pub feature_branch: Option<String>,

    /// Git user name override
    pub git_name: Option<String>,

    /// Git user email override
    pub git_email: Option<String>,

    /// Skip dependency installation
    pub skip_deps: bool,

    /// Skip enhancement (CLAUDE.md, hooks, etc.)
    pub skip_enhance: bool,

    /// Base directory for projects
    pub base_dir: Option<PathBuf>,
}

/// Result of project creation
#[derive(Debug)]
pub struct ProjectResult {
    /// Project name
    pub name: String,

    /// Project type
    pub project_type: String,

    /// Absolute path to project directory
    pub path: PathBuf,

    /// Installed extensions
    pub extensions: Vec<String>,

    /// Git configuration
    pub git_config: GitConfig,
}

/// Git configuration details
#[derive(Debug, Clone)]
pub struct GitConfig {
    pub user_name: String,
    pub user_email: String,
    pub branch: String,
    pub remotes: Vec<GitRemote>,
}

#[derive(Debug, Clone)]
pub struct GitRemote {
    pub name: String,
    pub url: String,
}

/// Main project management interface
pub struct ProjectManager {
    template_loader: template::TemplateLoader,
    type_detector: detector::TypeDetector,
    scaffolder: scaffolder::ProjectScaffolder,
    git_manager: git::GitManager,
    enhancer: enhancer::ProjectEnhancer,
}

impl ProjectManager {
    /// Create new project manager with default configuration
    pub fn new() -> Result<Self> {
        Ok(Self {
            template_loader: template::TemplateLoader::new()?,
            type_detector: detector::TypeDetector::new()?,
            scaffolder: scaffolder::ProjectScaffolder::new(),
            git_manager: git::GitManager::new(),
            enhancer: enhancer::ProjectEnhancer::new()?,
        })
    }

    /// Create new project from template
    pub async fn create_project(
        &self,
        config: NewProjectConfig,
    ) -> Result<ProjectResult> {
        // 1. Determine project type (auto-detect or explicit)
        let project_type = self.resolve_project_type(&config).await?;

        // 2. Load template for type
        let template = self.template_loader.load_template(&project_type)?;

        // 3. Determine project directory
        let base_dir = config.base_dir
            .unwrap_or_else(|| Self::default_projects_dir());
        let project_dir = base_dir.join(&config.name);

        // 4. Check if directory already exists
        if project_dir.exists() {
            return Err(ProjectError::AlreadyExists(project_dir));
        }

        // 5. Create directory structure
        std::fs::create_dir_all(&project_dir)?;

        // 6. Initialize Git repository
        let git_config = self.git_manager.init_repo(
            &project_dir,
            config.git_name.as_deref(),
            config.git_email.as_deref(),
        ).await?;

        // 7. Scaffold project files from template
        let variables = self.collect_variables(&config, &git_config);
        self.scaffolder.create_files(
            &project_dir,
            &template,
            &variables,
        ).await?;

        // 8. Install extensions (unless skipped)
        let extensions = if !config.skip_extensions {
            self.enhancer.install_extensions(
                &project_dir,
                &template.extensions,
            ).await?
        } else {
            Vec::new()
        };

        // 9. Apply enhancements (CLAUDE.md, hooks, etc.)
        if !config.skip_tools {
            self.enhancer.apply_enhancements(
                &project_dir,
                &template,
                &variables,
            ).await?;
        }

        // 10. Create initial commit
        self.git_manager.create_initial_commit(
            &project_dir,
            &config.name,
        ).await?;

        Ok(ProjectResult {
            name: config.name,
            project_type,
            path: project_dir,
            extensions,
            git_config,
        })
    }

    /// Clone or fork repository
    pub async fn clone_project(
        &self,
        config: CloneProjectConfig,
    ) -> Result<ProjectResult> {
        // Implementation in section c) below
        todo!("Implemented in git.rs")
    }

    /// Resolve project type from config
    async fn resolve_project_type(
        &self,
        config: &NewProjectConfig,
    ) -> Result<String> {
        if let Some(ref t) = config.project_type {
            // Explicit type provided
            Ok(self.type_detector.resolve_alias(t)?)
        } else if config.interactive {
            // Interactive selection
            self.type_detector.select_interactive(None).await
        } else {
            // Auto-detect from name
            let detected = self.type_detector.detect_from_name(&config.name)?;

            match detected {
                detector::DetectionResult::Unambiguous(t) => Ok(t),
                detector::DetectionResult::Ambiguous(suggestions) => {
                    // Multiple matches - prompt user
                    self.type_detector.select_interactive(Some(suggestions)).await
                }
                detector::DetectionResult::None => {
                    // No match - use default or prompt
                    if config.interactive {
                        self.type_detector.select_interactive(None).await
                    } else {
                        Ok("node".to_string()) // Default
                    }
                }
            }
        }
    }

    fn collect_variables(
        &self,
        config: &NewProjectConfig,
        git_config: &GitConfig,
    ) -> template::TemplateVariables {
        template::TemplateVariables::new()
            .with("project_name", &config.name)
            .with("author", &git_config.user_name)
            .with("git_user_name", &git_config.user_name)
            .with("git_user_email", &git_config.user_email)
            .with("date", &chrono::Local::now().format("%Y-%m-%d").to_string())
            .with("year", &chrono::Local::now().format("%Y").to_string())
            .with("description", "Project description")
            .with("license", "MIT")
    }

    fn default_projects_dir() -> PathBuf {
        std::env::var("WORKSPACE_PROJECTS")
            .ok()
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .expect("Cannot determine home directory")
                    .join("projects")
            })
    }
}

impl Default for ProjectManager {
    fn default() -> Self {
        Self::new().expect("Failed to create ProjectManager")
    }
}
```

**Reasoning**: Modular architecture separates concerns and enables:

- **Type Safety**: Rust's type system prevents common errors
- **Testability**: Each module can be tested independently
- **Async Support**: Parallel network operations for extensions and Git
- **Maintainability**: Clear separation of responsibilities
- **Extensibility**: Easy to add new project types or enhancements

### b) CLI Command Integration

**Decision**: Integrate project management into main CLI with subcommands `new` and `clone`.

**Architecture**:

```rust
// crates/sindri/src/commands/project.rs

use clap::{Args, Subcommand};
use sindri_project::{
    ProjectManager, NewProjectConfig, CloneProjectConfig,
};

#[derive(Debug, Subcommand)]
pub enum ProjectCommand {
    /// Create new project from template
    New(NewArgs),

    /// Clone or fork repository with enhancements
    Clone(CloneArgs),
}

#[derive(Debug, Args)]
pub struct NewArgs {
    /// Project name
    #[arg(value_name = "NAME")]
    pub name: String,

    /// Project type (node, python, rust, etc.)
    #[arg(long, short = 't')]
    pub r#type: Option<String>,

    /// List available project types
    #[arg(long, conflicts_with = "name")]
    pub list_types: bool,

    /// Interactive type selection
    #[arg(long, short = 'i')]
    pub interactive: bool,

    /// Git user name for this project
    #[arg(long)]
    pub git_name: Option<String>,

    /// Git user email for this project
    #[arg(long)]
    pub git_email: Option<String>,

    /// Skip extension installation
    #[arg(long)]
    pub skip_extensions: bool,

    /// Skip tool initialization
    #[arg(long)]
    pub skip_tools: bool,
}

#[derive(Debug, Args)]
pub struct CloneArgs {
    /// Repository URL
    #[arg(value_name = "REPOSITORY")]
    pub repo_url: String,

    /// Fork repository before cloning
    #[arg(long)]
    pub fork: bool,

    /// Checkout specific branch
    #[arg(long, short = 'b')]
    pub branch: Option<String>,

    /// Shallow clone with specified depth
    #[arg(long)]
    pub depth: Option<u32>,

    /// Create feature branch after clone
    #[arg(long)]
    pub feature: Option<String>,

    /// Git user name for this project
    #[arg(long)]
    pub git_name: Option<String>,

    /// Git user email for this project
    #[arg(long)]
    pub git_email: Option<String>,

    /// Skip dependency installation
    #[arg(long)]
    pub skip_deps: bool,

    /// Skip enhancements (CLAUDE.md, hooks, etc.)
    #[arg(long)]
    pub skip_enhance: bool,
}

pub async fn run(cmd: ProjectCommand) -> anyhow::Result<()> {
    let manager = ProjectManager::new()?;

    match cmd {
        ProjectCommand::New(args) => run_new(manager, args).await,
        ProjectCommand::Clone(args) => run_clone(manager, args).await,
    }
}

async fn run_new(
    manager: ProjectManager,
    args: NewArgs,
) -> anyhow::Result<()> {
    if args.list_types {
        return list_project_types(&manager).await;
    }

    let config = NewProjectConfig {
        name: args.name,
        project_type: args.r#type,
        interactive: args.interactive,
        git_name: args.git_name,
        git_email: args.git_email,
        skip_extensions: args.skip_extensions,
        skip_tools: args.skip_tools,
        base_dir: None,
    };

    println!("Creating new project: {}", config.name);

    let spinner = indicatif::ProgressBar::new_spinner();
    spinner.set_message("Setting up project...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    let result = manager.create_project(config).await?;

    spinner.finish_and_clear();

    println!("✓ Project {} created successfully", result.name);
    println!("  Type: {}", result.project_type);
    println!("  Path: {}", result.path.display());

    if !result.extensions.is_empty() {
        println!("  Extensions: {}", result.extensions.join(", "));
    }

    println!("\nNext steps:");
    println!("  1. cd {}", result.path.display());
    println!("  2. Edit CLAUDE.md with project details");
    println!("  3. Start coding!");

    Ok(())
}

async fn run_clone(
    manager: ProjectManager,
    args: CloneArgs,
) -> anyhow::Result<()> {
    let config = CloneProjectConfig {
        repo_url: args.repo_url,
        fork: args.fork,
        branch: args.branch,
        depth: args.depth,
        feature_branch: args.feature,
        git_name: args.git_name,
        git_email: args.git_email,
        skip_deps: args.skip_deps,
        skip_enhance: args.skip_enhance,
        base_dir: None,
    };

    let action = if config.fork { "Forking" } else { "Cloning" };
    println!("{} repository: {}", action, config.repo_url);

    let spinner = indicatif::ProgressBar::new_spinner();
    spinner.set_message(format!("{}...", action));
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    let result = manager.clone_project(config).await?;

    spinner.finish_and_clear();

    println!("✓ Project {} cloned successfully", result.name);
    println!("  Path: {}", result.path.display());
    println!("  Branch: {}", result.git_config.branch);

    if result.git_config.remotes.len() > 1 {
        println!("  Remotes:");
        for remote in &result.git_config.remotes {
            println!("    {} -> {}", remote.name, remote.url);
        }
    }

    println!("\nNext steps:");
    println!("  1. cd {}", result.path.display());
    println!("  2. Start coding!");

    Ok(())
}

async fn list_project_types(manager: &ProjectManager) -> anyhow::Result<()> {
    println!("Available Project Types:\n");

    let types = manager.template_loader.list_types()?;

    for (name, desc) in types {
        println!("  {:12} {}", name, desc);
    }

    Ok(())
}
```

**CLI Registration** (`crates/sindri/src/cli.rs`):

```rust
#[derive(Debug, Subcommand)]
pub enum Commands {
    // ... existing commands ...

    /// Create or clone projects
    #[command(subcommand)]
    Project(commands::project::ProjectCommand),
}
```

**Reasoning**: Clean CLI interface with:

- **Intuitive Commands**: `sindri new` and `sindri clone` are self-explanatory
- **Progressive Disclosure**: Simple default behavior, advanced options available
- **Consistent Patterns**: Follows same structure as other Sindri commands
- **Help Text**: Auto-generated from clap attributes

## Consequences

### Positive

1. **Type Safety**: Compile-time guarantees prevent common bash scripting errors
2. **Async Performance**: Parallel operations (Git + extensions + network) are faster
3. **Better Error Handling**: Structured errors with context and recovery strategies
4. **Testability**: Unit and integration tests for all components
5. **Extensibility**: Easy to add new project types or templates
6. **Template Engine**: Tera provides powerful variable substitution
7. **Cross-Platform**: Works consistently on Linux, macOS, Windows
8. **Git Integration**: Native Git operations via git2 crate
9. **Extension Awareness**: Seamless integration with extension system
10. **User Experience**: Progress bars, clear messages, helpful defaults

### Negative

1. **Migration Complexity**: Must port existing YAML templates to new format
2. **Binary Size**: Additional dependencies (git2, tera) increase binary size
3. **Template Compatibility**: Breaking change from v2 bash templates
4. **Maintenance Burden**: Must maintain both template system and detector logic
5. **Dependency on git2**: Native Git library has platform-specific requirements

### Neutral

1. **Template Location**: Embedded vs external templates (decided in ADR-024)
2. **Variable Syntax**: Tera vs handlebars vs custom (decided in ADR-024)
3. **Git Library**: git2 vs gitoxide vs CLI wrapper (decided in ADR-025)

## Alternatives Considered

### 1. Keep Bash Scripts

**Description**: Continue using v2's bash implementation, called from Rust CLI.

**Pros**:

- No porting effort required
- Existing templates work as-is
- Less binary size

**Cons**:

- No type safety
- Poor error handling
- Hard to test
- Async not possible
- Platform compatibility issues

**Rejected**: Defeats purpose of Rust migration. Need type safety and async.

### 2. External Template Repository

**Description**: Store templates in separate GitHub repository, fetch at runtime.

**Pros**:

- Templates can be updated independently
- Users can contribute templates easily
- No binary bloat

**Cons**:

- Requires network for every project creation
- Version compatibility issues
- Offline use impossible
- Complex caching logic

**Rejected**: Poor offline experience. Embed templates in binary (ADR-024 details).

### 3. Cookiecutter/Yeoman Integration

**Description**: Delegate to external project generators (cookiecutter, yeoman, etc.).

**Pros**:

- Leverage existing ecosystem
- No template maintenance
- Rich feature set

**Cons**:

- External dependencies (Python, Node.js)
- Inconsistent UX across generators
- No extension integration
- Limited customization

**Rejected**: Want native Rust solution with extension integration.

### 4. Single `project` Command

**Description**: Use `sindri project --new` and `sindri project --clone` instead of subcommands.

**Pros**:

- Fewer top-level commands
- Grouped functionality

**Cons**:

- More verbose
- Less intuitive
- Conflicts with potential `sindri project list` command

**Rejected**: `sindri new` and `sindri clone` are clearer and more ergonomic.

### 5. No Auto-Detection

**Description**: Always require explicit `--type` flag.

**Pros**:

- Explicit is better than implicit
- No complex detection logic
- Faster (no heuristics)

**Cons**:

- Poor UX for common cases
- Requires memorizing type names
- Extra typing for every project

**Rejected**: Auto-detection significantly improves UX for 90% of cases.

## Compliance

- ✅ Implements `sindri new <name>` command
- ✅ Implements `sindri clone <repo>` command
- ✅ Template-based project scaffolding
- ✅ Type detection from project names
- ✅ Extension integration
- ✅ Git initialization and configuration
- ✅ Fork workflow support
- ✅ Async operations for performance
- ✅ Cross-platform compatibility
- ✅ Comprehensive error handling

## Notes

### Project Type Categories

Project types are organized by language and framework:

**Languages**:

- `node` - Node.js/JavaScript/TypeScript
- `python` - Python 3.x
- `rust` - Rust with Cargo
- `go` - Go modules
- `java` - Java with Maven/Gradle
- `ruby` - Ruby with Bundler

**Frameworks**:

- `rails` - Ruby on Rails
- `django` - Django (Python)
- `flask` - Flask (Python)
- `fastapi` - FastAPI (Python)
- `express` - Express.js (Node)
- `nextjs` - Next.js (Node)
- `spring` - Spring Boot (Java)

**Specialized**:

- `ml` - Machine Learning (Python + Jupyter)
- `api` - REST API (language-agnostic)
- `web` - Web application (HTML/CSS/JS)
- `cli` - Command-line tool (Rust/Go)

### Template Variable Reference

Available variables for template substitution:

| Variable         | Description         | Example             |
| ---------------- | ------------------- | ------------------- |
| `project_name`   | Project name        | `my-awesome-app`    |
| `author`         | Git user name       | `John Doe`          |
| `git_user_name`  | Git user name       | `John Doe`          |
| `git_user_email` | Git user email      | `john@example.com`  |
| `date`           | Current date        | `2026-01-22`        |
| `year`           | Current year        | `2026`              |
| `description`    | Project description | `A web application` |
| `license`        | License type        | `MIT`               |

### Integration Points

The project management system integrates with:

1. **Extension System** ([ADR-011](011-multi-method-extension-installation.md)): Auto-installs relevant extensions based on project type
2. **Template Engine** ([ADR-003](003-template-based-configuration.md)): Uses Tera for variable substitution
3. **Git Operations** ([ADR-025](025-git-operations-repository-management.md)): Native Git operations via git2
4. **Type System** ([ADR-008](008-extension-type-system-yaml-deserialization.md)): Type-safe YAML deserialization

### Error Recovery

The system implements graceful error recovery:

1. **Partial Creation Failure**: Clean up directory if scaffolding fails
2. **Git Init Failure**: Create directory without Git, warn user
3. **Extension Install Failure**: Continue with project creation, list failed extensions
4. **Template Parse Failure**: Fall back to default template, warn user
5. **Network Failure** (clone): Retry with exponential backoff, max 3 attempts

## Related Decisions

- [ADR-001: Rust Migration Workspace Architecture](001-rust-migration-workspace-architecture.md) - Defines `sindri-project` crate
- [ADR-003: Template-Based Configuration](003-template-based-configuration.md) - Tera template engine pattern
- [ADR-008: Extension Type System YAML Deserialization](008-extension-type-system-yaml-deserialization.md) - YAML parsing approach
- [ADR-011: Multi-Method Extension Installation](011-multi-method-extension-installation.md) - Extension installation integration
- [ADR-024: Template-Based Project Scaffolding](024-template-based-project-scaffolding.md) - Template system details (next)
- [ADR-025: Git Operations and Repository Management](025-git-operations-repository-management.md) - Git integration details (next)
