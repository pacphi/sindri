//! Project management commands (new, clone)

use anyhow::{anyhow, Context, Result};
use camino::Utf8PathBuf;
use std::process::Command;

use crate::cli::{CloneProjectArgs, NewProjectArgs, ProjectCommands};
use crate::output;

/// Run project subcommands
pub async fn run(cmd: ProjectCommands) -> Result<()> {
    match cmd {
        ProjectCommands::New(args) => new_project(args).await,
        ProjectCommands::Clone(args) => clone_project(args).await,
    }
}

//====================================
// NEW PROJECT COMMAND
//====================================

/// Create a new project from template
pub async fn new_project(args: NewProjectArgs) -> Result<()> {
    output::header("Create New Project");

    // Determine project directory
    let projects_base = get_projects_dir();
    let project_dir = projects_base.join(&args.name);

    // Check if project already exists
    if project_dir.exists() {
        return Err(anyhow!(
            "Project '{}' already exists at {}",
            args.name,
            project_dir
        ));
    }

    // Determine project type
    let project_type = determine_project_type(&args.name, args.project_type, args.interactive)?;

    output::kv("Project name", &args.name);
    output::kv("Project type", &project_type);
    output::kv("Location", project_dir.as_str());
    println!();

    // Load template
    output::info(&format!("Loading template: {}", project_type));
    let template = load_template(&project_type)?;

    // Activate extensions
    if !template.extensions.is_empty() {
        output::info(&format!(
            "Activating {} extension(s)...",
            template.extensions.len()
        ));
        for ext in &template.extensions {
            output::info(&format!("  Activating: {}", ext));
            // TODO: Call extension-manager to install extension
            // For now, just log the intention
            tracing::debug!("Would activate extension: {}", ext);
        }
    }

    // Create project directory
    output::info("Creating project directory...");
    std::fs::create_dir_all(&project_dir).context("Failed to create project directory")?;

    // Change to project directory for subsequent operations
    std::env::set_current_dir(&project_dir)?;

    // Initialize git repository
    output::info("Initializing git repository...");
    init_git_repo(&args.name)?;

    // Apply git config overrides if provided
    if args.git_name.is_some() || args.git_email.is_some() {
        apply_new_project_git_config(args.git_name.as_deref(), args.git_email.as_deref())?;
    }

    // Collect template variables
    let variables = collect_template_variables(&args.name)?;

    // Execute template setup commands
    if !template.setup_commands.is_empty() {
        output::info("Executing setup commands...");
        execute_template_setup(&template, &variables)?;
    }

    // Create template files
    if !template.files.is_empty() {
        output::info("Creating template files...");
        create_template_files(&template, &variables)?;
    }

    // Generate CLAUDE.md
    output::info("Creating CLAUDE.md...");
    create_project_claude_md(&template, &variables)?;

    // Commit initial files
    commit_initial_project(&args.name)?;

    // Setup project enhancements
    setup_new_project_enhancements(args.skip_tools)?;

    // Display success message
    println!();
    output::success(&format!("Project '{}' created successfully", args.name));
    println!();
    output::kv("Location", project_dir.as_str());

    println!();
    output::info("Next steps:");
    println!("   1. cd {}", project_dir);
    println!("   2. Edit CLAUDE.md with project details");
    println!("   3. Start coding with: claude");

    // Show initialized tools
    println!();
    output::info("Initialized Tools:");
    show_new_project_tools(&project_dir)?;

    // Show git configuration
    println!();
    output::info("Git Configuration:");
    show_new_project_git_config()?;

    Ok(())
}

//====================================
// CLONE PROJECT COMMAND
//====================================

pub async fn clone_project(args: CloneProjectArgs) -> Result<()> {
    output::header("Clone Project");

    // Validate repository URL
    if !is_valid_repo_url(&args.repository) {
        return Err(anyhow!("Invalid repository URL: {}", args.repository));
    }

    // Extract project name from URL
    let project_name = extract_project_name(&args.repository)?;
    output::kv("Repository", &args.repository);
    output::kv("Project name", &project_name);

    // Determine projects directory
    let projects_base = get_projects_dir();
    let project_dir = projects_base.join(&project_name);

    // Check if project already exists
    if project_dir.exists() {
        return Err(anyhow!(
            "Project {} already exists at {}",
            project_name,
            project_dir
        ));
    }

    output::kv("Target directory", project_dir.as_str());
    println!();

    // Fork if requested
    if args.fork {
        output::info("Forking repository...");
        fork_repository(&args.repository, &projects_base, &project_name).await?;

        // Change to project directory
        std::env::set_current_dir(&project_dir).context("Failed to change to project directory")?;

        if !args.no_enhance {
            output::info("Setting up fork remotes and aliases...");
            setup_fork_remotes()?;
            setup_fork_aliases()?;
        }
    } else {
        output::info("Cloning repository...");
        clone_repository(
            &args.repository,
            &project_dir,
            args.depth,
            args.branch.as_deref(),
        )?;

        // Change to project directory
        std::env::set_current_dir(&project_dir).context("Failed to change to project directory")?;
    }

    // Checkout branch if specified (for fork mode)
    if args.fork && args.branch.is_some() {
        checkout_branch(args.branch.as_ref().unwrap())?;
    }

    // Apply git config overrides
    if args.git_name.is_some() || args.git_email.is_some() {
        apply_git_config(args.git_name.as_deref(), args.git_email.as_deref())?;
    }

    // Apply enhancements
    if !args.no_enhance {
        output::info("Applying Claude enhancements...");

        // Setup git hooks
        setup_git_hooks(&project_dir)?;

        // Create CLAUDE.md
        create_claude_md(&project_dir)?;

        // Install dependencies
        if !args.no_deps {
            install_dependencies(&project_dir)?;
        }

        // Initialize tools
        if !args.skip_tools {
            init_tools(&project_dir)?;
        }
    }

    // Create feature branch if requested
    if let Some(feature_branch) = &args.feature {
        output::info(&format!("Creating feature branch: {}", feature_branch));
        create_feature_branch(feature_branch)?;
    }

    // Display success message
    println!();
    output::success(&format!("Project {} cloned successfully", project_name));
    println!();
    output::kv("Location", project_dir.as_str());
    println!();
    output::info("Next steps:");
    println!("  1. cd {}", project_dir);
    if !project_dir.join("CLAUDE.md").exists() || args.no_enhance {
        println!("  2. Run 'claude /init' to set up project context");
    }
    println!("  3. Start coding with: claude");
    println!();

    // Show initialized tools
    show_initialized_tools(&project_dir)?;

    // Show git configuration
    show_git_config(args.fork)?;

    Ok(())
}

// Helper functions

fn is_valid_repo_url(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("http://") || url.starts_with("git@")
}

fn extract_project_name(url: &str) -> Result<String> {
    let name = url
        .trim_end_matches('/')
        .split('/')
        .last()
        .ok_or_else(|| anyhow!("Could not determine project name from URL"))?
        .trim_end_matches(".git");

    if name.is_empty() {
        return Err(anyhow!("Could not determine project name from URL"));
    }

    Ok(name.to_string())
}

fn get_projects_dir() -> Utf8PathBuf {
    // Use WORKSPACE_PROJECTS if available, otherwise use ~/projects
    std::env::var("WORKSPACE_PROJECTS")
        .ok()
        .map(Utf8PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/alt/home/developer".to_string());
            Utf8PathBuf::from(home).join("projects")
        })
}

async fn fork_repository(
    repo_url: &str,
    projects_base: &Utf8PathBuf,
    _project_name: &str,
) -> Result<()> {
    // Check if gh CLI is available
    if !is_command_available("gh") {
        return Err(anyhow!(
            "GitHub CLI (gh) is required for forking. Please install it first."
        ));
    }

    // Check if gh is authenticated
    let auth_status = Command::new("gh")
        .arg("auth")
        .arg("status")
        .output()
        .context("Failed to check gh auth status")?;

    if !auth_status.status.success() {
        return Err(anyhow!(
            "GitHub CLI is not authenticated. Please run: gh auth login"
        ));
    }

    // Fork and clone
    let spinner = output::spinner("Forking repository...");

    let output = Command::new("gh")
        .arg("repo")
        .arg("fork")
        .arg(repo_url)
        .arg("--clone")
        .current_dir(projects_base)
        .output()
        .context("Failed to fork repository")?;

    spinner.finish_and_clear();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failed to fork repository: {}", stderr));
    }

    output::success("Repository forked successfully");
    Ok(())
}

fn clone_repository(
    repo_url: &str,
    target_dir: &Utf8PathBuf,
    depth: Option<u32>,
    branch: Option<&str>,
) -> Result<()> {
    let spinner = output::spinner("Cloning repository...");

    let mut cmd = Command::new("git");
    cmd.arg("clone");

    if let Some(d) = depth {
        cmd.arg("--depth").arg(d.to_string());
    }

    if let Some(b) = branch {
        cmd.arg("--branch").arg(b);
    }

    cmd.arg(repo_url).arg(target_dir.as_str());

    let output = cmd.output().context("Failed to execute git clone")?;

    spinner.finish_and_clear();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failed to clone repository: {}", stderr));
    }

    output::success("Repository cloned successfully");
    Ok(())
}

fn setup_fork_remotes() -> Result<()> {
    // Verify upstream remote exists (should be set by gh repo fork)
    let output = Command::new("git")
        .arg("remote")
        .arg("get-url")
        .arg("upstream")
        .output()
        .context("Failed to check upstream remote")?;

    if output.status.success() {
        let upstream_url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        output::success(&format!("Fork configured with upstream: {}", upstream_url));
    } else {
        output::warning("Upstream remote not configured. Fork may not have been set up correctly.");
    }

    Ok(())
}

fn setup_fork_aliases() -> Result<()> {
    output::info("Setting up fork management aliases...");

    // Sync with upstream
    Command::new("git")
        .arg("config")
        .arg("alias.sync-upstream")
        .arg("!git fetch upstream && git checkout main && git merge upstream/main")
        .output()
        .context("Failed to set sync-upstream alias")?;

    // Push to fork's origin
    Command::new("git")
        .arg("config")
        .arg("alias.push-fork")
        .arg("push origin HEAD")
        .output()
        .context("Failed to set push-fork alias")?;

    // Update from upstream
    Command::new("git")
        .arg("config")
        .arg("alias.update-from-upstream")
        .arg("!git fetch upstream && git rebase upstream/main")
        .output()
        .context("Failed to set update-from-upstream alias")?;

    // PR branch
    Command::new("git")
        .arg("config")
        .arg("alias.pr-branch")
        .arg("!f() { git checkout -b \"$1\" upstream/main; }; f")
        .output()
        .context("Failed to set pr-branch alias")?;

    // Fork status
    Command::new("git")
        .arg("config")
        .arg("alias.fork-status")
        .arg("!echo \"=== Remotes ===\" && git remote -v && echo && echo \"=== Branch Tracking ===\" && git branch -vv")
        .output()
        .context("Failed to set fork-status alias")?;

    output::success("Fork aliases configured:");
    println!("  • git sync-upstream    - Fetch and merge upstream changes");
    println!("  • git push-fork        - Push current branch to your fork");
    println!("  • git update-from-upstream - Rebase current branch on upstream/main");
    println!("  • git pr-branch <name> - Create new branch from upstream/main");
    println!("  • git fork-status      - Show fork remotes and branch tracking");

    Ok(())
}

fn checkout_branch(branch: &str) -> Result<()> {
    output::info(&format!("Checking out branch: {}", branch));

    // Try to checkout the branch
    let output = Command::new("git")
        .arg("checkout")
        .arg(branch)
        .output()
        .context("Failed to checkout branch")?;

    if !output.status.success() {
        // Try to fetch from upstream and checkout
        output::warning(&format!(
            "Branch {} not found locally, trying to fetch from upstream",
            branch
        ));

        Command::new("git")
            .arg("fetch")
            .arg("upstream")
            .arg(branch)
            .output()
            .context("Failed to fetch from upstream")?;

        let checkout_output = Command::new("git")
            .arg("checkout")
            .arg("-b")
            .arg(branch)
            .arg(&format!("upstream/{}", branch))
            .output()
            .context("Failed to checkout branch from upstream")?;

        if !checkout_output.status.success() {
            return Err(anyhow!("Could not checkout branch: {}", branch));
        }
    }

    Ok(())
}

fn apply_git_config(name: Option<&str>, email: Option<&str>) -> Result<()> {
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

fn setup_git_hooks(project_dir: &Utf8PathBuf) -> Result<()> {
    let hooks_dir = project_dir.join(".git/hooks");
    std::fs::create_dir_all(&hooks_dir).context("Failed to create hooks directory")?;

    output::info("Setting up Git hooks...");

    // Pre-commit hook
    let pre_commit_hook = r#"#!/bin/bash
# Pre-commit hook for code quality checks

# Source common utilities if available
if [ -f "/docker/lib/common.sh" ]; then
    source "/docker/lib/common.sh"
else
    print_status() { echo "[INFO] $1"; }
    print_error() { echo "[ERROR] $1"; }
fi

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

fn create_claude_md(project_dir: &Utf8PathBuf) -> Result<()> {
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

fn install_dependencies(project_dir: &Utf8PathBuf) -> Result<()> {
    output::info("Detecting and installing project dependencies...");

    let mut installed = false;

    // Node.js projects
    if project_dir.join("package.json").exists() {
        if is_command_available("npm") {
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
    }

    // Python projects
    if project_dir.join("requirements.txt").exists() || project_dir.join("pyproject.toml").exists()
    {
        if is_command_available("pip") {
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
    }

    // Rust projects
    if project_dir.join("Cargo.toml").exists() {
        if is_command_available("cargo") {
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
    }

    // Go projects
    if project_dir.join("go.mod").exists() {
        if is_command_available("go") {
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
    }

    if !installed {
        output::info("No dependency files detected");
    }

    Ok(())
}

fn init_tools(_project_dir: &Utf8PathBuf) -> Result<()> {
    output::info("Initializing agentic tools...");

    // Check if Claude Code is available
    if is_command_available("claude") {
        output::success("Claude Code available");
    } else {
        output::warning("Claude Code not available");
    }

    // Note: Extension-based tools would be initialized via capability-manager
    // This is a simplified version for v3

    Ok(())
}

fn create_feature_branch(branch_name: &str) -> Result<()> {
    let output = Command::new("git")
        .arg("checkout")
        .arg("-b")
        .arg(branch_name)
        .output()
        .context("Failed to create feature branch")?;

    if !output.status.success() {
        return Err(anyhow!("Failed to create feature branch"));
    }

    output::success(&format!("Switched to new branch: {}", branch_name));
    Ok(())
}

fn show_initialized_tools(project_dir: &Utf8PathBuf) -> Result<()> {
    println!();
    output::info("Initialized Tools:");

    if is_command_available("claude") {
        println!("  ✓ Claude Code");
    }

    if project_dir.join(".github/spec.json").exists() && is_command_available("uvx") {
        println!("  ✓ GitHub spec-kit");
    }

    Ok(())
}

fn show_git_config(is_fork: bool) -> Result<()> {
    println!();
    output::info("Git Configuration:");

    // Get user name
    if let Ok(output) = Command::new("git").arg("config").arg("user.name").output() {
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !name.is_empty() {
            // Get user email
            if let Ok(email_output) = Command::new("git").arg("config").arg("user.email").output() {
                let email = String::from_utf8_lossy(&email_output.stdout)
                    .trim()
                    .to_string();
                println!("  User: {} <{}>", name, email);
            }
        }
    }

    // Get current branch
    if let Ok(output) = Command::new("git")
        .arg("branch")
        .arg("--show-current")
        .output()
    {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !branch.is_empty() {
            println!("  Branch: {}", branch);
        }
    }

    // Get remotes if fork
    if is_fork {
        if let Ok(output) = Command::new("git")
            .arg("remote")
            .arg("get-url")
            .arg("origin")
            .output()
        {
            let origin = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !origin.is_empty() {
                println!("  Origin: {}", origin);
            }
        }

        if let Ok(output) = Command::new("git")
            .arg("remote")
            .arg("get-url")
            .arg("upstream")
            .output()
        {
            let upstream = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !upstream.is_empty() {
                println!("  Upstream: {}", upstream);
            }
        }
    }

    Ok(())
}

fn is_command_available(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

//====================================
// NEW PROJECT HELPER FUNCTIONS
//====================================

/// Determine project type from name, explicit type, or interactive selection
fn determine_project_type(
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
            output::info(&format!("Auto-detected project type: {}", t));
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

enum DetectionResult {
    Unambiguous(String),
    Ambiguous(Vec<String>),
}

/// Detect project type from name using pattern matching
/// TODO: Implement using project-templates.yaml detection rules
fn detect_type_from_name(name: &str) -> Option<DetectionResult> {
    let name_lower = name.to_lowercase();

    // Simple detection rules (will be replaced with YAML-driven detection)
    if name_lower.contains("node") || name_lower.contains("npm") || name_lower.contains("express") {
        return Some(DetectionResult::Unambiguous("node".to_string()));
    }
    if name_lower.contains("python")
        || name_lower.contains("django")
        || name_lower.contains("flask")
    {
        return Some(DetectionResult::Unambiguous("python".to_string()));
    }
    if name_lower.contains("rust") || name_lower.contains("cargo") {
        return Some(DetectionResult::Unambiguous("rust".to_string()));
    }
    if name_lower.contains("go") {
        return Some(DetectionResult::Unambiguous("go".to_string()));
    }

    // Check for ambiguous patterns
    if name_lower.contains("api") || name_lower.contains("service") {
        return Some(DetectionResult::Ambiguous(vec![
            "node".to_string(),
            "python".to_string(),
            "go".to_string(),
            "rust".to_string(),
        ]));
    }

    None
}

/// Resolve template alias to canonical name
/// TODO: Implement using project-templates.yaml aliases
fn resolve_template_alias(input: &str) -> String {
    let input_lower = input.to_lowercase();

    match input_lower.as_str() {
        "nodejs" | "javascript" | "js" => "node".to_string(),
        "py" | "python3" => "python".to_string(),
        _ => input_lower,
    }
}

/// Interactive project type selection
fn select_project_type_interactive(suggestions: Option<Vec<String>>) -> Result<String> {
    use dialoguer::Select;

    let available_types = if let Some(types) = suggestions {
        types
    } else {
        // All available types
        // TODO: Load from project-templates.yaml
        vec![
            "node".to_string(),
            "python".to_string(),
            "rust".to_string(),
            "go".to_string(),
            "ruby".to_string(),
            "java".to_string(),
        ]
    };

    let selection = Select::new()
        .with_prompt("Select project type")
        .items(&available_types)
        .default(0)
        .interact()?;

    Ok(available_types[selection].clone())
}

/// Project template structure
#[derive(Debug)]
struct ProjectTemplate {
    extensions: Vec<String>,
    setup_commands: Vec<String>,
    files: Vec<(String, String)>,
    claude_md_template: Option<String>,
}

/// Load project template from configuration
/// TODO: Implement using project-templates.yaml loader
fn load_template(project_type: &str) -> Result<ProjectTemplate> {
    // Placeholder implementation
    // This will be replaced with actual template loading from YAML
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

    Ok(ProjectTemplate {
        extensions: vec![project_type.to_string()],
        setup_commands,
        files,
        claude_md_template: claude_template,
    })
}

/// Generate .gitignore content for project type
fn generate_gitignore(project_type: &str) -> String {
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

/// Initialize git repository
fn init_git_repo(project_name: &str) -> Result<()> {
    // Initialize git
    let output = Command::new("git")
        .arg("init")
        .output()
        .context("Failed to run git init")?;

    if !output.status.success() {
        return Err(anyhow!("git init failed"));
    }

    // Set default branch name
    let _ = Command::new("git").args(&["branch", "-M", "main"]).output();

    // Configure git user if not already configured globally
    let user_name = Command::new("git")
        .args(&["config", "user.name"])
        .output()?;

    if user_name.stdout.is_empty() {
        let _ = Command::new("git")
            .args(&["config", "user.name", "Developer"])
            .output();
    }

    let user_email = Command::new("git")
        .args(&["config", "user.email"])
        .output()?;

    if user_email.stdout.is_empty() {
        let _ = Command::new("git")
            .args(&["config", "user.email", "developer@localhost"])
            .output();
    }

    tracing::debug!("Initialized git repository for {}", project_name);
    Ok(())
}

/// Template variable collection
#[derive(Debug)]
struct TemplateVariables {
    project_name: String,
    author: String,
    date: String,
    year: String,
}

/// Collect template variables for substitution
fn collect_template_variables(project_name: &str) -> Result<TemplateVariables> {
    let user_name = Command::new("git")
        .args(&["config", "user.name"])
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
fn execute_template_setup(template: &ProjectTemplate, variables: &TemplateVariables) -> Result<()> {
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
fn create_template_files(template: &ProjectTemplate, variables: &TemplateVariables) -> Result<()> {
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
fn create_project_claude_md(
    template: &ProjectTemplate,
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

/// Substitute template variables in string
fn substitute_variables(content: &str, variables: &TemplateVariables) -> String {
    content
        .replace("{project_name}", &variables.project_name)
        .replace("{author}", &variables.author)
        .replace("{date}", &variables.date)
        .replace("{year}", &variables.year)
}

/// Commit initial project files
fn commit_initial_project(project_name: &str) -> Result<()> {
    // Add all files
    Command::new("git")
        .args(&["add", "."])
        .output()
        .context("Failed to run git add")?;

    // Commit
    let message = format!("feat: initial project setup for {}", project_name);
    let output = Command::new("git")
        .args(&["commit", "-m", &message])
        .output()
        .context("Failed to run git commit")?;

    if !output.status.success() {
        tracing::warn!("git commit failed, continuing...");
    }

    Ok(())
}

/// Setup project enhancements (hooks, tools, etc.) for new project
fn setup_new_project_enhancements(skip_tools: bool) -> Result<()> {
    output::info("Setting up project enhancements...");

    // Install dependencies
    // TODO: Implement dependency detection and installation

    // Initialize project tools (claude-flow, aqe, etc.)
    if !skip_tools {
        output::info("  Initializing project tools...");
        // TODO: Call capability-manager to discover and initialize extensions
        // with project-init capabilities
        tracing::debug!("Would initialize project tools");
    } else {
        output::info("  Skipping project tools (--skip-tools)");
    }

    Ok(())
}

/// Apply git config for new project
fn apply_new_project_git_config(name: Option<&str>, email: Option<&str>) -> Result<()> {
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

/// Show initialized tools for new project
fn show_new_project_tools(project_dir: &Utf8PathBuf) -> Result<()> {
    if is_command_available("claude") {
        println!("  ✓ Claude Code");
    }

    if project_dir.join(".github/spec.json").exists() && is_command_available("uvx") {
        println!("  ✓ GitHub spec-kit");
    }

    // TODO: Query extension manager for initialized extensions
    // and display their capabilities

    Ok(())
}

/// Show git configuration for new project
fn show_new_project_git_config() -> Result<()> {
    let user_name = Command::new("git")
        .args(&["config", "user.name"])
        .output()?;

    let user_email = Command::new("git")
        .args(&["config", "user.email"])
        .output()?;

    let branch = Command::new("git")
        .args(&["branch", "--show-current"])
        .output()?;

    let name_str = String::from_utf8_lossy(&user_name.stdout)
        .trim()
        .to_string();
    let email_str = String::from_utf8_lossy(&user_email.stdout)
        .trim()
        .to_string();
    let branch_str = String::from_utf8_lossy(&branch.stdout).trim().to_string();

    println!("   User: {} <{}>", name_str, email_str);
    println!("   Branch: {}", branch_str);

    Ok(())
}
