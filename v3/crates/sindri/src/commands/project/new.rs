//! `sindri project new` command handler

use anyhow::{anyhow, Context, Result};
use camino::Utf8PathBuf;
use std::process::Command;

use crate::cli::NewProjectArgs;
use crate::output;

use super::enhance::{
    activate_extension, apply_git_config, collect_template_variables, commit_initial_project,
    create_project_claude_md, create_template_files, execute_template_setup,
    get_initialized_extensions_for_project, get_projects_dir, init_git_repo, is_command_available,
    setup_new_project_enhancements,
};
use super::template::{determine_project_type, load_template};

/// Create a new project from template
pub(super) async fn run(args: NewProjectArgs) -> Result<()> {
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

    // Activate extensions via extension-manager
    if !template.extensions.is_empty() {
        output::info(&format!(
            "Activating {} extension(s)...",
            template.extensions.len()
        ));
        for ext in &template.extensions {
            output::info(&format!("  Activating: {}", ext));
            if let Err(e) = activate_extension(ext).await {
                output::warning(&format!("Failed to activate extension {}: {}", ext, e));
                tracing::warn!("Extension activation failed for {}: {}", ext, e);
            }
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
        apply_git_config(args.git_name.as_deref(), args.git_email.as_deref())?;
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

/// Show initialized tools for new project
///
/// Queries the extension manager for installed extensions and displays
/// their capabilities relevant to the project.
fn show_new_project_tools(project_dir: &Utf8PathBuf) -> Result<()> {
    // Always show Claude Code if available
    if is_command_available("claude") {
        println!("  [ok] Claude Code");
    }

    // Show spec-kit if configured
    if project_dir.join(".github/spec.json").exists() && is_command_available("uvx") {
        println!("  [ok] GitHub spec-kit");
    }

    // Query extension manager for initialized extensions with project-relevant capabilities
    let initialized_extensions = get_initialized_extensions_for_project(project_dir)?;

    for (name, description) in initialized_extensions {
        println!("  [ok] {} - {}", name, description);
    }

    Ok(())
}

/// Show git configuration for new project
fn show_new_project_git_config() -> Result<()> {
    let user_name = Command::new("git").args(["config", "user.name"]).output()?;

    let user_email = Command::new("git")
        .args(["config", "user.email"])
        .output()?;

    let branch = Command::new("git")
        .args(["branch", "--show-current"])
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
