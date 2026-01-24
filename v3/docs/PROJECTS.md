# Project Management in Sindri V3

This document describes the project management capabilities in Sindri V3, including creating new projects from templates, cloning repositories, and managing project configurations.

## Overview

The `sindri-projects` crate provides comprehensive project management functionality:

- **Project Creation** - Create new projects from language/framework templates
- **Repository Cloning** - Clone repositories with automatic enhancements
- **Fork Workflows** - Fork and clone repositories with proper remote configuration
- **Template System** - YAML-driven templates with intelligent type detection
- **Git Integration** - Repository initialization, branching, and configuration
- **Project Enhancement** - Automatic CLAUDE.md generation and extension activation

## CLI Commands

### Creating New Projects

```bash
sindri project new <name> [options]
```

Create a new project from a template with automatic scaffolding.

#### Arguments

| Argument | Description |
|----------|-------------|
| `<name>` | Project name (required). Used as directory name and in template variables. |

#### Options

| Option | Short | Description |
|--------|-------|-------------|
| `--type <TYPE>` | `-t` | Explicit project type (e.g., `node`, `python`, `rust`). If omitted, type is auto-detected from the project name. |
| `--interactive` | `-i` | Enable interactive type selection. Useful when auto-detection is ambiguous. |
| `--git-name <NAME>` | | Git user name for this project (uses local config, not global). |
| `--git-email <EMAIL>` | | Git user email for this project. |
| `--skip-extensions` | | Skip extension installation. |
| `--skip-tools` | | Skip tool initialization (CLAUDE.md, hooks). |
| `--list-types` | | List all available project types and exit. |

#### Examples

```bash
# Create a Node.js project (auto-detected from name)
sindri project new my-express-api

# Create a Python project with explicit type
sindri project new data-pipeline --type python

# Create a Rails project with custom git config
sindri project new my-rails-app --type rails --git-name "John Doe" --git-email "john@example.com"

# Interactive type selection
sindri project new my-new-project --interactive

# List available project types
sindri project new --list-types
```

### Cloning Repositories

```bash
sindri project clone <repository> [options]
```

Clone a repository with optional enhancements, including CLAUDE.md generation and extension activation.

#### Arguments

| Argument | Description |
|----------|-------------|
| `<repository>` | Repository URL (HTTPS or SSH format). |

#### Options

| Option | Short | Description |
|--------|-------|-------------|
| `--fork` | | Fork the repository before cloning. Requires GitHub CLI (`gh`). |
| `--branch <BRANCH>` | `-b` | Checkout a specific branch after cloning. |
| `--depth <DEPTH>` | | Shallow clone with specified depth (e.g., `--depth 1`). |
| `--feature <BRANCH>` | | Create a feature branch after cloning. |
| `--git-name <NAME>` | | Git user name for this project. |
| `--git-email <EMAIL>` | | Git user email for this project. |
| `--skip-deps` | | Skip dependency installation. |
| `--skip-enhance` | | Skip enhancements (CLAUDE.md, hooks, extensions). |

#### Examples

```bash
# Clone a repository
sindri project clone https://github.com/user/repo.git

# Shallow clone for faster download
sindri project clone https://github.com/user/large-repo.git --depth 1

# Clone and checkout a specific branch
sindri project clone https://github.com/user/repo.git --branch develop

# Fork and clone (requires gh CLI)
sindri project clone https://github.com/original/repo.git --fork

# Fork with feature branch
sindri project clone https://github.com/original/repo.git --fork --feature add-new-feature

# Clone without enhancements
sindri project clone https://github.com/user/repo.git --skip-enhance
```

## Project Templates

Sindri V3 includes built-in templates for common project types, embedded in the binary for offline use.

### Available Project Types

| Type | Description | Aliases |
|------|-------------|---------|
| `node` | Node.js application | `nodejs`, `javascript`, `js`, `ts`, `typescript` |
| `python` | Python application | `py`, `python3` |
| `go` | Go application | `golang` |
| `rust` | Rust application | `rs` |
| `rails` | Ruby on Rails application | `ruby`, `ror` |
| `django` | Django web application | - |
| `spring` | Spring Boot application | `spring-boot`, `spring-web`, `spring-webmvc` |
| `dotnet` | .NET application | `csharp`, `c#`, `.net` |
| `terraform` | Terraform infrastructure project | `tf`, `infra`, `infrastructure` |
| `docker` | Dockerized application | `container`, `containerized` |
| `general` | General purpose project | - |

### Template Structure

Templates are defined in YAML format with the following structure:

```yaml
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
      priority: 10

    # Setup commands (executed after scaffolding)
    setup:
      commands:
        - "npm init -y"
      dependencies:
        detect_file: "package.json"
        install_command: "npm install"
        requires_tool: "npm"

    # Files to generate
    files:
      - src: "node/package.json.tera"
        dest: "package.json"
      - src: "gitignore/node.txt"
        dest: ".gitignore"

    # CLAUDE.md template
    claude_template: "claude/node.md.tera"
```

### Template Variables

The following variables are available for substitution in templates:

| Variable | Description | Example |
|----------|-------------|---------|
| `{project_name}` | Project name | `my-awesome-app` |
| `{author}` | Author name (from git config) | `John Doe` |
| `{git_user_name}` | Git user name | `John Doe` |
| `{git_user_email}` | Git user email | `john@example.com` |
| `{date}` | Current date | `2026-01-24` |
| `{year}` | Current year | `2026` |
| `{description}` | Project description | `A web application` |
| `{license}` | License type | `MIT` |

### Template Inheritance

Framework templates can inherit from language templates:

```yaml
templates:
  nextjs:
    description: "Next.js application (React framework)"
    category: "framework"
    parent: "node"  # Inherit from node template

    extensions:
      - nodejs
      - typescript
      - tailwind

    # Override parent's setup commands
    setup:
      commands:
        - "npx create-next-app@latest {project_name} --typescript --tailwind"
```

## Project Type Detection

Sindri automatically detects project types from project names using pattern matching.

### Detection Algorithm

1. **Pattern Matching** - Project name is matched against detection patterns
2. **Priority Resolution** - Higher priority patterns take precedence
3. **Ambiguity Handling** - Multiple matches with similar scores trigger interactive selection
4. **Alias Resolution** - Input like "py" or "nodejs" is resolved to canonical type names

### Detection Examples

| Project Name | Detected Type | Reason |
|--------------|---------------|--------|
| `my-express-app` | `node` | Contains "express" pattern |
| `django-blog` | `django` | Contains "django" pattern |
| `ml-model` | `python` | Contains "ml" pattern |
| `api-server` | Ambiguous | Could be `node`, `go`, or `python` |
| `rust-cli` | `rust` | Contains "rust" pattern |

### Handling Ambiguous Detection

When multiple types match with similar confidence:

```bash
# Interactive selection is triggered
$ sindri project new api-server

Multiple project types match "api-server":
  1) node       - Node.js application
  2) go         - Go application
  3) python     - Python application

Select project type [1-3]:
```

You can avoid this by specifying the type explicitly:

```bash
sindri project new api-server --type go
```

## Git Integration

### Repository Initialization

When creating a new project, Sindri:

1. Creates the project directory
2. Initializes a git repository
3. Configures local user.name and user.email
4. Sets the default branch to `main`
5. Creates an initial commit with scaffolded files

### Git Configuration

Sindri uses **project-local git configuration** to avoid modifying global settings:

```bash
# Configuration is stored in .git/config, not ~/.gitconfig
git config --local user.name "John Doe"
git config --local user.email "john@example.com"
```

Configuration precedence (highest to lowest):

1. Command-line overrides (`--git-name`, `--git-email`)
2. Project-local config (`.git/config`)
3. Global config (`~/.gitconfig`)
4. Interactive prompt (if no config found)

### Fork Workflow

When using `--fork`, Sindri:

1. Forks the repository on GitHub (requires `gh` CLI)
2. Clones the fork to local machine
3. Configures remotes:
   - `origin` → Your fork
   - `upstream` → Original repository
4. Fetches upstream branches
5. Optionally creates a feature branch

#### Fork Git Aliases

Sindri sets up helpful aliases for fork management:

| Alias | Command | Description |
|-------|---------|-------------|
| `sync-upstream` | `git fetch upstream && git checkout main && git merge upstream/main` | Sync fork with upstream |
| `update-from-upstream` | `git fetch upstream && git rebase upstream/main` | Rebase on upstream |
| `push-fork` | `push origin HEAD` | Push to your fork |
| `pr-branch` | `git checkout -b <name> upstream/main` | Create PR-ready branch |
| `fork-status` | Shows remotes and branch tracking | View fork status |

Usage:

```bash
git sync-upstream       # Sync fork with upstream
git update-from-upstream # Rebase on upstream
git push-fork           # Push to your fork
git pr-branch my-feature # Create feature branch from upstream/main
git fork-status         # View fork configuration
```

### Branch Operations

```rust
use sindri_projects::git::{create_branch, checkout_branch};

// Create and checkout a new branch
create_branch(path, "feature/my-feature", true).await?;

// Checkout an existing branch
checkout_branch(path, "develop").await?;
```

### Remote Management

```rust
use sindri_projects::git::{add_remote, remove_remote, list_remotes};

// Add a remote
add_remote(path, "upstream", "https://github.com/original/repo.git").await?;

// List all remotes
let remotes = list_remotes(path).await?;
for (name, url) in remotes {
    println!("{}: {}", name, url);
}

// Remove a remote
remove_remote(path, "old-remote").await?;
```

## Project Enhancement

### CLAUDE.md Generation

Sindri automatically generates a `CLAUDE.md` file for AI-first development:

```markdown
# my-project

## Project Overview
Node.js application

## Setup Instructions
```bash
npm install
npm run dev
```

## Development Commands
- `npm start` - Start the application
- `npm run dev` - Development mode with hot reload
- `npm test` - Run tests

## Architecture Notes
[Add architectural decisions and patterns]

---
Created: 2026-01-24
Author: John Doe
```

### Extension Activation

Templates can specify extensions to activate:

```yaml
templates:
  node:
    extensions:
      - nodejs
      - prettier
      - eslint
```

When a project is created, these extensions are:
1. Validated for availability
2. Recorded in `.sindri/extensions.txt`
3. Ready for installation via the extension system

### Dependency Installation

Templates can define dependency installation:

```yaml
dependencies:
  detect: "package.json"  # File(s) that indicate dependencies exist
  command: "npm install"   # Installation command
  requires: "npm"          # Required tool
  fetch_command: "npm ci"  # Alternative for CI (fetch-only)
```

## API Usage

### Creating Projects Programmatically

```rust
use sindri_projects::{
    templates::{TemplateManager, TemplateVars, TypeDetector, DetectionResult},
    git::{init_repository, InitOptions},
    enhancement::EnhancementManager,
};
use sindri_core::types::GitWorkflowConfig;
use camino::Utf8Path;

// Initialize template system
let manager = TemplateManager::new()?;

// Detect project type from name
let detector = manager.detector();
let detection = detector.detect_from_name("my-express-app");

let project_type = match detection {
    DetectionResult::Single(t) => t,
    DetectionResult::Ambiguous(types) => {
        // Handle ambiguous detection (prompt user or use default)
        types[0].clone()
    }
    DetectionResult::None => "node".to_string(),
};

// Get template
let template = manager.get_template(&project_type)?;

// Create template variables
let vars = TemplateVars::new("my-express-app".to_string())
    .with_author("John Doe".to_string())
    .with_description("An Express.js API server".to_string());

// Render project files
let target_dir = Utf8Path::new("/path/to/my-express-app");
std::fs::create_dir_all(target_dir)?;
manager.render_project(&project_type, &vars, target_dir)?;

// Initialize git repository
let init_options = InitOptions::default();
let git_config = GitWorkflowConfig::default();
init_repository(target_dir, &init_options, &git_config).await?;

// Apply enhancements
let enhancer = EnhancementManager::new();
enhancer.create_claude_md(
    &target_dir.to_path_buf(),
    Some(&project_type),
    "my-express-app",
)?;
```

### Cloning Repositories Programmatically

```rust
use sindri_projects::git::{clone_repository, CloneOptions};
use camino::Utf8Path;

let options = CloneOptions {
    depth: Some(1),              // Shallow clone
    branch: Some("main".into()), // Specific branch
    feature_branch: None,
    git_name: Some("John Doe".into()),
    git_email: Some("john@example.com".into()),
};

let destination = Utf8Path::new("/path/to/my-clone");
clone_repository("https://github.com/user/repo.git", destination, &options).await?;
```

### Forking Repositories Programmatically

```rust
use sindri_projects::git::{fork_repository, ForkOptions};
use camino::Utf8Path;

let options = ForkOptions {
    setup_aliases: true,
    feature_branch: Some("add-feature".into()),
    git_name: Some("John Doe".into()),
    git_email: Some("john@example.com".into()),
    ..Default::default()
};

let destination = Utf8Path::new("/path/to/my-fork");
fork_repository("https://github.com/original/repo.git", destination, &options).await?;
```

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `WORKSPACE_PROJECTS` | Base directory for new projects | `~/projects` |

### Git Workflow Configuration

The `GitWorkflowConfig` struct controls git operation behavior:

```rust
pub struct GitWorkflowConfig {
    /// Default branch name
    pub default_branch: String,           // "main"

    /// Initial commit message
    pub initial_commit_message: String,   // "feat: initial project setup"

    /// Main branch names to check (in order)
    pub main_branch_names: Vec<String>,   // ["main", "master"]

    /// Origin remote name
    pub origin_remote: String,            // "origin"

    /// Upstream remote name
    pub upstream_remote: String,          // "upstream"
}
```

## Error Handling

The project system provides structured errors for better diagnostics:

| Error | Description |
|-------|-------------|
| `ProjectExists` | Project directory already exists |
| `InvalidProjectName` | Project name contains invalid characters |
| `UnknownProjectType` | Specified type not found in templates |
| `GitNotFound` | Git command not available |
| `GhNotFound` | GitHub CLI not available (required for fork) |
| `GhNotAuthenticated` | GitHub CLI not authenticated |
| `CloneFailed` | Repository clone operation failed |
| `ForkFailed` | Repository fork operation failed |
| `InvalidBranch` | Invalid branch name |
| `BranchExists` | Branch already exists |
| `RemoteNotFound` | Specified remote not found |
| `RemoteExists` | Remote already exists |

## Requirements

### Required Tools

- **git** - Required for all project operations

### Optional Tools

- **gh** - GitHub CLI, required for fork workflow
- **claude** - Claude CLI, used for authentication check
- Language-specific tools (npm, pip3, cargo, etc.) for dependency installation

## Related Documentation

- [ADR-023: Phase 7 Project Management Architecture](architecture/adr/023-phase-7-project-management-architecture.md)
- [ADR-024: Template-Based Project Scaffolding](architecture/adr/024-template-based-project-scaffolding.md)
- [ADR-025: Git Operations and Repository Management](architecture/adr/025-git-operations-repository-management.md)
