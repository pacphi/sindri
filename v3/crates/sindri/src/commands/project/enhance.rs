//! Project enhancement helpers shared between new and clone commands
//!
//! Includes extension activation, git hooks, CLAUDE.md creation,
//! dependency installation, and tool initialization.

use anyhow::{anyhow, Context, Result};
use camino::Utf8PathBuf;
use std::process::Command;

use crate::output;
use crate::utils::{get_cache_dir, get_extensions_dir};

use sindri_core::types::ExtensionState;
use sindri_extensions::{ExtensionDistributor, ExtensionSourceResolver, StatusLedger};

use super::template::InternalProjectTemplate;

/// Template variable collection
#[derive(Debug)]
pub(super) struct TemplateVariables {
    pub project_name: String,
    pub author: String,
    pub date: String,
    pub year: String,
}

/// Activate an extension by installing it via the extension manager
///
/// This function:
/// 1. Checks if the extension is already installed via the status ledger
/// 2. If not, downloads and installs it via ExtensionDistributor
pub(super) async fn activate_extension(extension_name: &str) -> Result<()> {
    // Get home directory for paths
    let cache_dir = get_cache_dir()?;
    let extensions_dir = get_extensions_dir()?;

    // Check if already installed
    if let Ok(ledger) = StatusLedger::load_default() {
        if let Ok(status_map) = ledger.get_all_latest_status() {
            let is_installed = status_map
                .get(extension_name)
                .map(|s| s.current_state == ExtensionState::Installed)
                .unwrap_or(false);
            if is_installed {
                tracing::debug!("Extension {} is already installed", extension_name);
                return Ok(());
            }
        }
    }

    // Parse CLI version for compatibility checking
    let cli_version =
        semver::Version::parse(env!("CARGO_PKG_VERSION")).context("Failed to parse CLI version")?;

    // Initialize distributor for installation
    let distributor = ExtensionDistributor::new(cache_dir, extensions_dir, cli_version)
        .context("Failed to initialize extension distributor")?;

    // Install the extension (latest compatible version)
    distributor
        .install(extension_name, None)
        .await
        .context(format!("Failed to install extension: {}", extension_name))?;

    output::success(&format!("    Activated: {}", extension_name));
    Ok(())
}

/// Check if a command is available on the system
pub(super) fn is_command_available(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Get the projects directory from environment or default
pub(super) fn get_projects_dir() -> Utf8PathBuf {
    // Use WORKSPACE_PROJECTS if set, then $WORKSPACE/projects, then ~/projects
    std::env::var("WORKSPACE_PROJECTS")
        .ok()
        .filter(|v| !v.is_empty())
        .map(Utf8PathBuf::from)
        .or_else(|| {
            std::env::var("WORKSPACE")
                .ok()
                .filter(|v| !v.is_empty())
                .map(|w| Utf8PathBuf::from(w).join("projects"))
        })
        .unwrap_or_else(|| {
            let home = sindri_core::utils::get_home_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| "/home/user".to_string());
            Utf8PathBuf::from(home).join("projects")
        })
}

/// Substitute template variables in string
pub(super) fn substitute_variables(content: &str, variables: &TemplateVariables) -> String {
    content
        .replace("{project_name}", &variables.project_name)
        .replace("{author}", &variables.author)
        .replace("{date}", &variables.date)
        .replace("{year}", &variables.year)
}

/// Collect template variables for substitution
pub(super) fn collect_template_variables(project_name: &str) -> Result<TemplateVariables> {
    let user_name = Command::new("git")
        .args(["config", "user.name"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default()
        .trim()
        .to_string();

    let now = chrono::Local::now();

    Ok(TemplateVariables {
        project_name: project_name.to_string(),
        author: if user_name.is_empty() {
            "Developer".to_string()
        } else {
            user_name
        },
        date: now.format("%Y-%m-%d").to_string(),
        year: now.format("%Y").to_string(),
    })
}

/// Execute template setup commands
pub(super) fn execute_template_setup(
    template: &InternalProjectTemplate,
    variables: &TemplateVariables,
) -> Result<()> {
    for cmd in &template.setup_commands {
        let resolved_cmd = substitute_variables(cmd, variables);
        tracing::debug!("Running setup command: {}", resolved_cmd);

        // Parse command and args
        let parts: Vec<&str> = resolved_cmd.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        let output = Command::new(parts[0]).args(&parts[1..]).output();

        match output {
            Ok(out) if out.status.success() => {
                tracing::debug!("Command succeeded: {}", resolved_cmd);
            }
            Ok(out) => {
                tracing::warn!(
                    "Command failed: {} (exit code: {})",
                    resolved_cmd,
                    out.status
                );
            }
            Err(e) => {
                tracing::warn!("Failed to execute command: {} (error: {})", resolved_cmd, e);
            }
        }
    }

    Ok(())
}

/// Create template files with variable substitution
pub(super) fn create_template_files(
    template: &InternalProjectTemplate,
    variables: &TemplateVariables,
) -> Result<()> {
    for (filepath, content) in &template.files {
        let resolved_path = substitute_variables(filepath, variables);
        let resolved_content = substitute_variables(content, variables);

        // Create parent directories
        if let Some(parent) = std::path::Path::new(&resolved_path).parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Write file
        std::fs::write(&resolved_path, resolved_content)
            .context(format!("Failed to create file: {}", resolved_path))?;

        tracing::debug!("Created file: {}", resolved_path);
    }

    Ok(())
}

/// Create CLAUDE.md file for new project
pub(super) fn create_project_claude_md(
    template: &InternalProjectTemplate,
    variables: &TemplateVariables,
) -> Result<()> {
    if std::path::Path::new("CLAUDE.md").exists() {
        output::info("  CLAUDE.md already exists");
        return Ok(());
    }

    let content = if let Some(template_content) = &template.claude_md_template {
        substitute_variables(template_content, variables)
    } else {
        format!(
            r#"# {}

## Project Overview
This project was created with Sindri.

## Setup Instructions
[Add setup instructions here]

## Development Commands
[Add common commands here]

## Architecture Notes
[Add architectural decisions and patterns]

## Important Files
[List key files and their purposes]
"#,
            variables.project_name
        )
    };

    std::fs::write("CLAUDE.md", content).context("Failed to create CLAUDE.md")?;

    Ok(())
}

/// Initialize git repository
pub(super) fn init_git_repo(project_name: &str) -> Result<()> {
    // Initialize git
    let output = Command::new("git")
        .arg("init")
        .output()
        .context("Failed to run git init")?;

    if !output.status.success() {
        return Err(anyhow!("git init failed"));
    }

    // Set default branch name
    let _ = Command::new("git").args(["branch", "-M", "main"]).output();

    // Configure git user if not already configured globally
    let user_name = Command::new("git").args(["config", "user.name"]).output()?;

    if user_name.stdout.is_empty() {
        let _ = Command::new("git")
            .args(["config", "user.name", "Developer"])
            .output();
    }

    let user_email = Command::new("git")
        .args(["config", "user.email"])
        .output()?;

    if user_email.stdout.is_empty() {
        let _ = Command::new("git")
            .args(["config", "user.email", "developer@localhost"])
            .output();
    }

    tracing::debug!("Initialized git repository for {}", project_name);
    Ok(())
}

/// Commit initial project files
pub(super) fn commit_initial_project(project_name: &str) -> Result<()> {
    // Add all files
    Command::new("git")
        .args(["add", "."])
        .output()
        .context("Failed to run git add")?;

    // Commit
    let message = format!("feat: initial project setup for {}", project_name);
    let output = Command::new("git")
        .args(["commit", "-m", &message])
        .output()
        .context("Failed to run git commit")?;

    if !output.status.success() {
        tracing::warn!("git commit failed, continuing...");
    }

    Ok(())
}

/// Apply git config (user.name, user.email) for a project
pub(super) fn apply_git_config(name: Option<&str>, email: Option<&str>) -> Result<()> {
    if name.is_none() && email.is_none() {
        return Ok(());
    }

    output::info("Configuring Git for this project...");

    if let Some(n) = name {
        let output = Command::new("git")
            .arg("config")
            .arg("user.name")
            .arg(n)
            .output()
            .context("Failed to set git user.name")?;

        if !output.status.success() {
            return Err(anyhow!("Failed to set Git user name"));
        }
        output::success(&format!("Git user name set to: {}", n));
    }

    if let Some(e) = email {
        let output = Command::new("git")
            .arg("config")
            .arg("user.email")
            .arg(e)
            .output()
            .context("Failed to set git user.email")?;

        if !output.status.success() {
            return Err(anyhow!("Failed to set Git user email"));
        }
        output::success(&format!("Git user email set to: {}", e));
    }

    Ok(())
}

/// Setup git hooks for a cloned project
pub(super) fn setup_git_hooks(project_dir: &Utf8PathBuf) -> Result<()> {
    let hooks_dir = project_dir.join(".git/hooks");
    std::fs::create_dir_all(&hooks_dir).context("Failed to create hooks directory")?;

    output::info("Setting up Git hooks...");

    // Pre-commit hook
    let pre_commit_hook = r#"#!/bin/bash
# Pre-commit hook for code quality checks

# Utility functions for output
print_status() { echo "[INFO] $1"; }
print_error() { echo "[ERROR] $1"; }

# Check for debugging code
if git diff --cached --name-only | xargs grep -E "console\.(log|debug|info|warn|error)" 2>/dev/null; then
    print_error "Debugging code detected. Please remove console statements."
    exit 1
fi

# Run prettier if available
if command -v prettier >/dev/null 2>&1; then
    files=$(git diff --cached --name-only --diff-filter=ACM | grep -E '\.(js|jsx|ts|tsx|json|css|scss|md)$')
    if [ -n "$files" ]; then
        echo "$files" | xargs prettier --write
        echo "$files" | xargs git add
    fi
fi

# Run eslint if available
if command -v eslint >/dev/null 2>&1; then
    files=$(git diff --cached --name-only --diff-filter=ACM | grep -E '\.(js|jsx|ts|tsx)$')
    if [ -n "$files" ]; then
        echo "$files" | xargs eslint --fix
        echo "$files" | xargs git add
    fi
fi

exit 0
"#;

    std::fs::write(hooks_dir.join("pre-commit"), pre_commit_hook)
        .context("Failed to write pre-commit hook")?;

    // Set executable permission
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(hooks_dir.join("pre-commit"))?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(hooks_dir.join("pre-commit"), perms)?;
    }

    // Commit message hook
    let commit_msg_hook = r#"#!/bin/bash
# Commit message validation hook

commit_regex='^(feat|fix|docs|style|refactor|test|chore|perf|ci|build|revert)(\(.+\))?: .{1,50}'

if ! grep -qE "$commit_regex" "$1"; then
    echo "Invalid commit message format!"
    echo "Format: <type>(<scope>): <subject>"
    echo "Example: feat(auth): add login functionality"
    echo ""
    echo "Types: feat, fix, docs, style, refactor, test, chore, perf, ci, build, revert"
    exit 1
fi
"#;

    std::fs::write(hooks_dir.join("commit-msg"), commit_msg_hook)
        .context("Failed to write commit-msg hook")?;

    // Set executable permission
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(hooks_dir.join("commit-msg"))?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(hooks_dir.join("commit-msg"), perms)?;
    }

    output::success("Git hooks configured");
    Ok(())
}

/// Create CLAUDE.md for a cloned project
pub(super) fn create_clone_claude_md(project_dir: &Utf8PathBuf) -> Result<()> {
    let claude_md_path = project_dir.join("CLAUDE.md");

    // Don't overwrite if already exists
    if claude_md_path.exists() {
        output::info("CLAUDE.md already exists, skipping creation");
        return Ok(());
    }

    output::info("Creating CLAUDE.md...");

    let project_name = project_dir
        .file_name()
        .ok_or_else(|| anyhow!("Invalid project directory"))?;

    let content = format!(
        r#"# {}

## Project Overview

[Brief description of the project]

## Architecture

[High-level architecture overview]

## Development

### Setup

```bash
# Install dependencies
# [Add project-specific setup commands]
```

### Running

```bash
# [Add commands to run the project]
```

### Testing

```bash
# [Add testing commands]
```

## Key Files

- [List important files and their purposes]

## Conventions

- [Code style guidelines]
- [Naming conventions]
- [Best practices]
"#,
        project_name
    );

    std::fs::write(&claude_md_path, content).context("Failed to write CLAUDE.md")?;
    output::success("CLAUDE.md created");
    Ok(())
}

/// Install project dependencies based on detected project files
pub(super) fn install_dependencies(project_dir: &Utf8PathBuf) -> Result<()> {
    output::info("Detecting and installing project dependencies...");

    let mut installed = false;

    // Node.js projects
    if project_dir.join("package.json").exists() && is_command_available("npm") {
        output::info("Installing Node.js dependencies...");
        let output = Command::new("npm")
            .arg("install")
            .current_dir(project_dir)
            .output()
            .context("Failed to run npm install")?;

        if output.status.success() {
            output::success("Node.js dependencies installed");
            installed = true;
        } else {
            output::warning("Failed to install Node.js dependencies");
        }
    }

    // Python projects
    let has_python_deps = project_dir.join("requirements.txt").exists()
        || project_dir.join("pyproject.toml").exists();
    if has_python_deps && is_command_available("pip") {
        output::info("Installing Python dependencies...");

        if project_dir.join("requirements.txt").exists() {
            let output = Command::new("pip")
                .arg("install")
                .arg("-r")
                .arg("requirements.txt")
                .current_dir(project_dir)
                .output()
                .context("Failed to run pip install")?;

            if output.status.success() {
                output::success("Python dependencies installed");
                installed = true;
            } else {
                output::warning("Failed to install Python dependencies");
            }
        }
    }

    // Rust projects
    if project_dir.join("Cargo.toml").exists() && is_command_available("cargo") {
        output::info("Fetching Rust dependencies...");
        let output = Command::new("cargo")
            .arg("fetch")
            .current_dir(project_dir)
            .output()
            .context("Failed to run cargo fetch")?;

        if output.status.success() {
            output::success("Rust dependencies fetched");
            installed = true;
        } else {
            output::warning("Failed to fetch Rust dependencies");
        }
    }

    // Go projects
    if project_dir.join("go.mod").exists() && is_command_available("go") {
        output::info("Installing Go dependencies...");
        let output = Command::new("go")
            .arg("mod")
            .arg("download")
            .current_dir(project_dir)
            .output()
            .context("Failed to run go mod download")?;

        if output.status.success() {
            output::success("Go dependencies installed");
            installed = true;
        } else {
            output::warning("Failed to install Go dependencies");
        }
    }

    if !installed {
        output::info("No dependency files detected");
    }

    Ok(())
}

/// Initialize project tools using capability-manager
///
/// Discovers installed extensions with project-init capabilities and runs
/// their initialization commands in the project directory.
pub(super) fn initialize_project_tools() -> Result<()> {
    // Load ledger to get installed extensions
    let ledger = match StatusLedger::load_default() {
        Ok(l) => l,
        Err(e) => {
            tracing::debug!(
                "Failed to load status ledger for tool initialization: {}",
                e
            );
            return Ok(());
        }
    };

    let status_map = match ledger.get_all_latest_status() {
        Ok(m) => m,
        Err(e) => {
            tracing::debug!("Failed to get extension status: {}", e);
            return Ok(());
        }
    };

    if status_map.is_empty() {
        tracing::debug!("No installed extensions found in status ledger");
        return Ok(());
    }

    let workspace_dir = std::env::current_dir().context("Failed to get current directory")?;

    // Use canonical resolver -- handles bundled (flat), downloaded (versioned), local-dev
    let resolver = match ExtensionSourceResolver::from_env() {
        Ok(r) => r,
        Err(e) => {
            tracing::debug!("Failed to create extension source resolver: {}", e);
            return Ok(());
        }
    };

    // Iterate through installed extensions looking for project-init capabilities
    let mut initialized_count = 0;
    for (name, _status) in status_map
        .iter()
        .filter(|(_, s)| s.current_state == ExtensionState::Installed)
    {
        let ext_yaml_path = match resolver.extension_path(name) {
            Some(p) => p,
            None => continue,
        };

        // Load extension definition
        let content = match std::fs::read_to_string(&ext_yaml_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let extension: sindri_core::types::Extension = match serde_yaml_ng::from_str(&content) {
            Ok(e) => e,
            Err(_) => continue,
        };

        // Check for project-init capability
        if let Some(capabilities) = &extension.capabilities {
            if let Some(project_init) = &capabilities.project_init {
                if project_init.enabled {
                    tracing::debug!("Running project-init for extension: {}", name);

                    // Execute project-init commands
                    for cmd_config in &project_init.commands {
                        // Check if auth is required
                        let auth_ok = match cmd_config.requires_auth {
                            sindri_core::types::AuthProvider::None => true,
                            _ => {
                                // Check if auth is configured
                                check_auth_configured(&extension, &cmd_config.requires_auth)
                            }
                        };

                        if !auth_ok && cmd_config.conditional {
                            tracing::debug!(
                                "Skipping {} command (requires auth): {}",
                                name,
                                cmd_config.description
                            );
                            continue;
                        }

                        output::info(&format!("    Running: {}", cmd_config.description));

                        let result = Command::new("sh")
                            .arg("-c")
                            .arg(&cmd_config.command)
                            .current_dir(&workspace_dir)
                            .output();

                        match result {
                            Ok(output) if output.status.success() => {
                                initialized_count += 1;
                            }
                            Ok(output) => {
                                let stderr = String::from_utf8_lossy(&output.stderr);
                                tracing::debug!(
                                    "Project-init command failed for {}: {}",
                                    name,
                                    stderr
                                );
                            }
                            Err(e) => {
                                tracing::debug!(
                                    "Failed to run project-init command for {}: {}",
                                    name,
                                    e
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    if initialized_count > 0 {
        output::success(&format!("  Initialized {} tool(s)", initialized_count));
    } else {
        tracing::debug!("No extensions with project-init capabilities found");
    }

    Ok(())
}

/// Check if authentication is configured for a given provider
fn check_auth_configured(
    extension: &sindri_core::types::Extension,
    _required_auth: &sindri_core::types::AuthProvider,
) -> bool {
    if let Some(capabilities) = &extension.capabilities {
        if let Some(auth) = &capabilities.auth {
            // Check environment variables for auth
            for env_var in &auth.env_vars {
                if std::env::var(env_var).is_ok() {
                    return true;
                }
            }

            // Run validator if configured
            if let Some(validator) = &auth.validator {
                if let Ok(output) = Command::new("sh")
                    .arg("-c")
                    .arg(&validator.command)
                    .output()
                {
                    return output.status.code() == Some(validator.expected_exit_code);
                }
            }
        }
    }

    false
}

/// Get initialized extensions with project-relevant capabilities
///
/// Queries the status ledger and extension definitions to find extensions that:
/// 1. Are installed and active
/// 2. Have project-init, project-context, or MCP capabilities
pub(super) fn get_initialized_extensions_for_project(
    _project_dir: &Utf8PathBuf,
) -> Result<Vec<(String, String)>> {
    let mut results = Vec::new();

    // Load ledger
    let ledger = match StatusLedger::load_default() {
        Ok(l) => l,
        Err(_) => return Ok(results),
    };

    let status_map = match ledger.get_all_latest_status() {
        Ok(m) => m,
        Err(_) => return Ok(results),
    };

    // Use canonical resolver -- handles bundled (flat), downloaded (versioned), local-dev
    let resolver = match ExtensionSourceResolver::from_env() {
        Ok(r) => r,
        Err(_) => return Ok(results),
    };

    // Check each installed extension for relevant capabilities
    for (name, _status) in status_map
        .iter()
        .filter(|(_, s)| s.current_state == ExtensionState::Installed)
    {
        let ext_yaml_path = match resolver.extension_path(name) {
            Some(p) => p,
            None => continue,
        };

        // Load extension definition
        let content = match std::fs::read_to_string(&ext_yaml_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let extension: sindri_core::types::Extension = match serde_yaml_ng::from_str(&content) {
            Ok(e) => e,
            Err(_) => continue,
        };

        // Check for project-relevant capabilities
        if let Some(capabilities) = &extension.capabilities {
            let mut capability_descriptions = Vec::new();

            // Project-init capability
            if let Some(project_init) = &capabilities.project_init {
                if project_init.enabled {
                    capability_descriptions.push("project-init");
                }
            }

            // Project-context capability
            if let Some(project_context) = &capabilities.project_context {
                if project_context.enabled {
                    capability_descriptions.push("context");
                }
            }

            // MCP capability
            if let Some(mcp) = &capabilities.mcp {
                if mcp.enabled {
                    capability_descriptions.push("MCP tools");
                }
            }

            if !capability_descriptions.is_empty() {
                results.push((
                    extension.metadata.name.clone(),
                    capability_descriptions.join(", "),
                ));
            }
        }
    }

    Ok(results)
}

/// Setup project enhancements (hooks, tools, etc.) for new project
pub(super) fn setup_new_project_enhancements(skip_tools: bool) -> Result<()> {
    output::info("Setting up project enhancements...");

    // Detect and install dependencies based on project files
    detect_and_install_dependencies()?;

    // Initialize project tools using capability-manager
    if !skip_tools {
        output::info("  Initializing project tools...");
        initialize_project_tools()?;
    } else {
        output::info("  Skipping project tools (--skip-tools)");
    }

    Ok(())
}

/// Detect project dependencies and install them
///
/// Uses project-templates.yaml dependency configuration to:
/// 1. Detect dependency files (package.json, requirements.txt, etc.)
/// 2. Run appropriate install commands
fn detect_and_install_dependencies() -> Result<()> {
    use sindri_projects::templates::TemplateManager;

    let current_dir = std::env::current_dir().context("Failed to get current directory")?;

    // Try to load template manager for dependency rules
    let template_manager = match TemplateManager::new() {
        Ok(mgr) => mgr,
        Err(e) => {
            tracing::debug!(
                "Template manager not available for dependency detection: {}",
                e
            );
            // Fall back to basic detection
            return install_dependencies_basic(&Utf8PathBuf::try_from(current_dir)?);
        }
    };

    // Check each template's dependency configuration
    for template_type in template_manager.available_types() {
        if let Some(template) = template_manager.get_template(&template_type) {
            if let Some(dep_config) = &template.dependencies {
                // Check if dependency files exist
                let files_exist = dep_config.detect.patterns().iter().any(|pattern| {
                    if pattern.contains('*') {
                        // Glob pattern - check for any matching files
                        let glob_pattern = current_dir.join(pattern);
                        glob::glob(&glob_pattern.to_string_lossy())
                            .map(|paths| paths.count() > 0)
                            .unwrap_or(false)
                    } else {
                        current_dir.join(pattern).exists()
                    }
                });

                if files_exist {
                    output::info(&format!("  Detected {}", dep_config.description));

                    // Check if required tool is available
                    if is_command_available(&dep_config.requires) {
                        output::info(&format!("  Installing {}...", dep_config.description));

                        let result = Command::new("sh")
                            .arg("-c")
                            .arg(&dep_config.command)
                            .current_dir(&current_dir)
                            .output();

                        match result {
                            Ok(output) if output.status.success() => {
                                output::success(&format!("  {} installed", dep_config.description));
                            }
                            Ok(output) => {
                                let stderr = String::from_utf8_lossy(&output.stderr);
                                output::warning(&format!(
                                    "  Failed to install {}: {}",
                                    dep_config.description, stderr
                                ));
                            }
                            Err(e) => {
                                output::warning(&format!(
                                    "  Failed to run {}: {}",
                                    dep_config.command, e
                                ));
                            }
                        }
                    } else {
                        output::warning(&format!(
                            "  {} not available, skipping {}",
                            dep_config.requires, dep_config.description
                        ));
                    }
                }
            }
        }
    }

    Ok(())
}

/// Basic dependency installation fallback
fn install_dependencies_basic(project_dir: &Utf8PathBuf) -> Result<()> {
    // Node.js
    if project_dir.join("package.json").exists() && is_command_available("npm") {
        output::info("  Installing Node.js dependencies...");
        let _ = Command::new("npm")
            .arg("install")
            .current_dir(project_dir)
            .output();
    }

    // Python
    if project_dir.join("requirements.txt").exists() && is_command_available("pip3") {
        output::info("  Installing Python dependencies...");
        let _ = Command::new("pip3")
            .args(["install", "-r", "requirements.txt"])
            .current_dir(project_dir)
            .output();
    }

    // Rust
    if project_dir.join("Cargo.toml").exists() && is_command_available("cargo") {
        output::info("  Fetching Rust dependencies...");
        let _ = Command::new("cargo")
            .arg("fetch")
            .current_dir(project_dir)
            .output();
    }

    // Go
    if project_dir.join("go.mod").exists() && is_command_available("go") {
        output::info("  Downloading Go dependencies...");
        let _ = Command::new("go")
            .args(["mod", "download"])
            .current_dir(project_dir)
            .output();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- substitute_variables ----

    #[test]
    fn test_substitute_variables() {
        let vars = TemplateVariables {
            project_name: "my-app".to_string(),
            author: "Dev".to_string(),
            date: "2026-01-15".to_string(),
            year: "2026".to_string(),
        };
        let result = substitute_variables("Name: {project_name}, by {author} ({year})", &vars);
        assert_eq!(result, "Name: my-app, by Dev (2026)");
    }
}
