//! `sindri project clone` command handler

use anyhow::{anyhow, Context, Result};
use camino::Utf8PathBuf;
use std::process::Command;

use crate::cli::CloneProjectArgs;
use crate::output;

use super::enhance::{
    apply_git_config, create_clone_claude_md, get_initialized_extensions_for_project,
    get_projects_dir, initialize_project_tools, install_dependencies, is_command_available,
    setup_git_hooks,
};

/// Clone and enhance an existing project
pub(super) async fn run(args: CloneProjectArgs) -> Result<()> {
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
    if let Some(branch) = &args.branch {
        if args.fork {
            checkout_branch(branch)?;
        }
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
        create_clone_claude_md(&project_dir)?;

        // Install dependencies
        if !args.no_deps {
            install_dependencies(&project_dir)?;
        }

        // Initialize tools (run project-init capabilities from installed extensions)
        if !args.skip_tools {
            output::info("Initializing agentic tools...");
            initialize_project_tools()?;
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

fn is_valid_repo_url(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("http://") || url.starts_with("git@")
}

fn extract_project_name(url: &str) -> Result<String> {
    let name = url
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .ok_or_else(|| anyhow!("Could not determine project name from URL"))?
        .trim_end_matches(".git");

    if name.is_empty() {
        return Err(anyhow!("Could not determine project name from URL"));
    }

    Ok(name.to_string())
}

async fn fork_repository(
    repo_url: &str,
    projects_base: &Utf8PathBuf,
    _project_name: &str,
) -> Result<()> {
    // Check if gh CLI is available
    if !is_command_available("gh") {
        output::error("GitHub CLI (gh) is required for forking.");
        output::info("");
        output::info("Run 'sindri doctor --command project' for installation instructions");
        return Err(anyhow!("GitHub CLI (gh) not installed"));
    }

    // Check if gh is authenticated
    let auth_status = Command::new("gh")
        .arg("auth")
        .arg("status")
        .output()
        .context("Failed to check gh auth status")?;

    if !auth_status.status.success() {
        output::error("GitHub CLI is not authenticated.");
        output::info("Run: gh auth login");
        output::info("");
        output::info("Run 'sindri doctor --command project --check-auth' for more details");
        return Err(anyhow!("GitHub CLI not authenticated"));
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
            .arg(format!("upstream/{}", branch))
            .output()
            .context("Failed to checkout branch from upstream")?;

        if !checkout_output.status.success() {
            return Err(anyhow!("Could not checkout branch: {}", branch));
        }
    }

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

    // Query extension system for project-relevant capabilities
    let initialized_extensions = get_initialized_extensions_for_project(project_dir)?;
    for (name, description) in initialized_extensions {
        println!("  ✓ {} - {}", name, description);
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

#[cfg(test)]
mod tests {
    use super::*;

    // ---- is_valid_repo_url ----

    #[test]
    fn test_valid_https_url() {
        assert!(is_valid_repo_url("https://github.com/user/repo.git"));
    }

    #[test]
    fn test_valid_http_url() {
        assert!(is_valid_repo_url("http://github.com/user/repo"));
    }

    #[test]
    fn test_valid_git_ssh_url() {
        assert!(is_valid_repo_url("git@github.com:user/repo.git"));
    }

    #[test]
    fn test_invalid_url_ftp() {
        assert!(!is_valid_repo_url("ftp://example.com/repo"));
    }

    #[test]
    fn test_invalid_url_plain_path() {
        assert!(!is_valid_repo_url("/home/user/repo"));
    }

    #[test]
    fn test_invalid_url_empty() {
        assert!(!is_valid_repo_url(""));
    }

    // ---- extract_project_name ----

    #[test]
    fn test_extract_name_from_https_url() {
        let name = extract_project_name("https://github.com/user/my-project.git").unwrap();
        assert_eq!(name, "my-project");
    }

    #[test]
    fn test_extract_name_from_url_without_git_suffix() {
        let name = extract_project_name("https://github.com/user/my-project").unwrap();
        assert_eq!(name, "my-project");
    }

    #[test]
    fn test_extract_name_from_url_with_trailing_slash() {
        let name = extract_project_name("https://github.com/user/my-project/").unwrap();
        assert_eq!(name, "my-project");
    }

    #[test]
    fn test_extract_name_from_ssh_url() {
        let name = extract_project_name("git@github.com:user/repo-name.git").unwrap();
        assert_eq!(name, "repo-name");
    }
}
