//! Project enhancement with Claude tools and extensions
//!
//! This module handles:
//! - Extension activation via extension-manager
//! - Claude authentication verification
//! - Dependency installation
//! - CLAUDE.md creation
//! - Tool initialization (claude-flow, aqe, etc.)

use crate::error::Error;
use crate::templates::{TemplateLoader, TemplateManager, TemplateRenderer, TemplateVars};
use crate::types::{EnhancementOptions, ProjectTemplate};
use crate::Result;
use camino::{Utf8Path, Utf8PathBuf};
use std::process::Command;
use tracing::{debug, info, warn};

/// Enhancement manager for Claude tools and extensions
///
/// Responsible for setting up project enhancements including:
/// - CLAUDE.md context files for AI assistants
/// - Extension activation based on project type
/// - Dependency installation
/// - Tool verification
pub struct EnhancementManager {
    /// Template manager for CLAUDE.md generation and project scaffolding
    template_manager: TemplateManager,
}

impl EnhancementManager {
    /// Create a new enhancement manager
    ///
    /// Initializes the template system from embedded templates.
    ///
    /// # Returns
    /// A new `EnhancementManager` instance with loaded templates.
    ///
    /// # Errors
    /// Returns an error if the embedded templates cannot be loaded.
    pub fn new() -> Result<Self> {
        let template_manager = TemplateManager::new().map_err(|e| Error::EnhancementError {
            message: format!("Failed to initialize template manager: {}", e),
        })?;

        Ok(Self { template_manager })
    }

    /// Create an enhancement manager with a custom template manager
    ///
    /// Useful for testing or when custom templates are needed.
    pub fn with_template_manager(template_manager: TemplateManager) -> Self {
        Self { template_manager }
    }

    /// Activate extensions for the project
    ///
    /// This method prepares extensions for installation based on the project type.
    /// The actual installation is typically handled by the extension executor.
    ///
    /// # Arguments
    /// * `path` - Path to the project directory
    /// * `extensions` - List of extension names to activate
    ///
    /// # Returns
    /// A list of successfully validated extension names ready for installation.
    pub fn activate_extensions(
        &self,
        path: &Utf8PathBuf,
        extensions: &[String],
    ) -> Result<Vec<String>> {
        info!("Activating extensions for project at {}", path);

        if extensions.is_empty() {
            debug!("No extensions to activate");
            return Ok(Vec::new());
        }

        let mut activated = Vec::new();

        for ext in extensions {
            debug!("Checking extension: {}", ext);

            // Validate extension name (basic validation)
            if ext.trim().is_empty() {
                warn!("Skipping empty extension name");
                continue;
            }

            // Check if the extension is a known type from our templates
            // Extensions are typically named after their ecosystem: nodejs, python, rust, etc.
            let is_known = self.is_known_extension(ext);

            if is_known {
                info!("Extension '{}' is ready for activation", ext);
                activated.push(ext.clone());
            } else {
                // Unknown extensions are still added but with a warning
                // They may be custom or third-party extensions
                warn!(
                    "Extension '{}' is not a known built-in extension, but will be added",
                    ext
                );
                activated.push(ext.clone());
            }
        }

        // Create .sindri directory for extension tracking if it doesn't exist
        let sindri_dir = path.join(".sindri");
        if !sindri_dir.exists() {
            std::fs::create_dir_all(&sindri_dir).map_err(|e| {
                Error::enhancement_error(format!("Failed to create .sindri directory: {}", e))
            })?;
        }

        // Write activated extensions list for reference
        let extensions_file = sindri_dir.join("extensions.txt");
        let content = activated.join("\n");
        std::fs::write(&extensions_file, content).map_err(|e| {
            Error::enhancement_error(format!("Failed to write extensions list: {}", e))
        })?;

        info!(
            "Prepared {} extensions for activation: {:?}",
            activated.len(),
            activated
        );

        Ok(activated)
    }

    /// Check if an extension is a known built-in extension
    fn is_known_extension(&self, name: &str) -> bool {
        // Known extensions based on our template system
        const KNOWN_EXTENSIONS: &[&str] = &[
            "nodejs",
            "python",
            "golang",
            "rust",
            "ruby",
            "jvm",
            "dotnet",
            "docker",
            "infra-tools",
            "claude-tools",
            "git-tools",
        ];

        KNOWN_EXTENSIONS
            .iter()
            .any(|&ext| ext == name.to_lowercase())
    }

    /// Install project dependencies
    ///
    /// Detects and installs dependencies based on the project template configuration.
    /// Supports various package managers: npm, pip, cargo, go mod, bundle, etc.
    ///
    /// # Arguments
    /// * `path` - Path to the project directory
    /// * `template` - Project template containing dependency configuration
    /// * `skip_build` - If true, only fetch dependencies without building
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if dependency installation fails.
    pub fn install_dependencies(
        &self,
        path: &Utf8PathBuf,
        template: &ProjectTemplate,
        skip_build: bool,
    ) -> Result<()> {
        info!(
            "Installing dependencies for project at {} (skip_build: {})",
            path, skip_build
        );

        let dep_config = match &template.dependencies {
            Some(config) => config,
            None => {
                debug!("No dependency configuration in template");
                return Ok(());
            }
        };

        // Check if the required tool exists
        let required_tool = &dep_config.requires;
        if !self.command_exists(required_tool) {
            return Err(Error::command_not_found(required_tool.clone()));
        }

        // Check if dependency files exist
        let detect_patterns = &dep_config.detect;
        let has_deps = detect_patterns.iter().any(|pattern| {
            if pattern.contains('*') {
                // Glob pattern
                self.check_glob_pattern(path, pattern)
            } else {
                // Exact file
                path.join(pattern).exists()
            }
        });

        if !has_deps {
            debug!("No dependency files found matching: {:?}", detect_patterns);
            return Ok(());
        }

        // Determine the command to run
        let command = if skip_build {
            dep_config
                .fetch_command
                .as_ref()
                .unwrap_or(&dep_config.command)
        } else {
            &dep_config.command
        };

        info!(
            "Running dependency command: {} ({})",
            command,
            dep_config
                .description
                .as_deref()
                .unwrap_or("installing dependencies")
        );

        // Execute the command
        self.run_shell_command(path.as_std_path(), command)?;

        info!("Dependencies installed successfully");
        Ok(())
    }

    /// Check if any files match a glob pattern in the given directory
    fn check_glob_pattern(&self, dir: &Utf8Path, pattern: &str) -> bool {
        let full_pattern = dir.join(pattern);
        match glob::glob(full_pattern.as_str()) {
            Ok(paths) => paths.filter_map(|p| p.ok()).next().is_some(),
            Err(_) => false,
        }
    }

    /// Run a shell command in the given directory
    fn run_shell_command(&self, dir: &std::path::Path, command: &str) -> Result<()> {
        debug!("Running command: {} in {:?}", command, dir);

        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(dir)
            .output()
            .map_err(|e| Error::process_execution(format!("Failed to execute command: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::dependency_error(format!(
                "Command '{}' failed: {}",
                command,
                stderr.trim()
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.is_empty() {
            debug!("Command output: {}", stdout.trim());
        }

        Ok(())
    }

    /// Create CLAUDE.md file for the project
    ///
    /// Generates a CLAUDE.md file from templates, providing context for AI assistants.
    ///
    /// # Arguments
    /// * `path` - Path to the project directory
    /// * `template_type` - Optional template type (e.g., "node", "python"). If None, uses "general".
    /// * `project_name` - Name of the project
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if file creation fails.
    pub fn create_claude_md(
        &self,
        path: &Utf8PathBuf,
        template_type: Option<&str>,
        project_name: &str,
    ) -> Result<()> {
        info!(
            "Creating CLAUDE.md for project '{}' at {}",
            project_name, path
        );

        // Resolve template type
        let resolved_type = template_type.unwrap_or("general");
        let canonical_type = self
            .template_manager
            .resolve_alias(resolved_type)
            .unwrap_or_else(|| resolved_type.to_string());

        debug!("Using template type: {}", canonical_type);

        // Create template variables
        let vars = TemplateVars::new(project_name.to_string());

        // Check if CLAUDE.md already exists
        let claude_md_path = path.join("CLAUDE.md");
        if claude_md_path.exists() {
            warn!("CLAUDE.md already exists at {}, skipping creation", path);
            return Ok(());
        }

        // Generate CLAUDE.md using the template manager
        match self
            .template_manager
            .generate_claude_md(&canonical_type, &vars, path)
        {
            Ok(created_path) => {
                info!("Created CLAUDE.md at {}", created_path);
                Ok(())
            }
            Err(e) => {
                // If template not found, fall back to default
                warn!(
                    "Template '{}' not found, using default CLAUDE.md: {}",
                    canonical_type, e
                );
                self.create_default_claude_md(path, project_name)
            }
        }
    }

    /// Create a default CLAUDE.md when no template is available
    fn create_default_claude_md(&self, path: &Utf8PathBuf, project_name: &str) -> Result<()> {
        let content = format!(
            r#"# {}

## Project Overview
[Add project description]

## Setup Instructions
[Add setup instructions]

## Development Commands
[Add development commands]

## Architecture Notes
[Add architectural decisions and patterns]

## Testing
[Add testing instructions]

## Deployment
[Add deployment instructions]
"#,
            project_name
        );

        let claude_md_path = path.join("CLAUDE.md");
        std::fs::write(&claude_md_path, content)
            .map_err(|e| Error::enhancement_error(format!("Failed to write CLAUDE.md: {}", e)))?;

        info!("Created default CLAUDE.md at {}", claude_md_path);
        Ok(())
    }

    /// Setup project enhancements (tools, auth, etc.)
    ///
    /// Orchestrates the full enhancement flow:
    /// 1. Check required commands
    /// 2. Optionally check Claude authentication
    /// 3. Create CLAUDE.md
    /// 4. Activate extensions
    /// 5. Install dependencies (if applicable)
    ///
    /// # Arguments
    /// * `path` - Path to the project directory
    /// * `options` - Enhancement configuration options
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if any enhancement step fails.
    pub fn setup_enhancements(
        &self,
        path: &Utf8PathBuf,
        options: &EnhancementOptions,
    ) -> Result<()> {
        info!("Setting up enhancements for project at {}", path);

        // Step 1: Check if required commands are available
        if !options.skip_tools {
            self.verify_required_commands()?;
        }

        // Step 2: Check Claude authentication if needed
        if !options.skip_auth_check && !self.check_claude_auth() {
            warn!("Claude authentication not detected - some features may be unavailable");
        }

        // Step 3: Derive project name from path
        let project_name = path.file_name().unwrap_or("project").to_string();

        // Step 4: Create CLAUDE.md
        self.create_claude_md(path, None, &project_name)?;

        // Step 5: Activate additional extensions if specified
        if !options.additional_extensions.is_empty() {
            self.activate_extensions(path, &options.additional_extensions)?;
        }

        info!("Enhancement setup completed for {}", project_name);
        Ok(())
    }

    /// Verify that required commands are available
    fn verify_required_commands(&self) -> Result<()> {
        // Essential commands for project enhancement
        const REQUIRED_COMMANDS: &[&str] = &["git"];
        const OPTIONAL_COMMANDS: &[&str] = &["claude", "gh"];

        for cmd in REQUIRED_COMMANDS {
            if !self.command_exists(cmd) {
                return Err(Error::command_not_found(cmd.to_string()));
            }
        }

        for cmd in OPTIONAL_COMMANDS {
            if !self.command_exists(cmd) {
                debug!("Optional command '{}' not found", cmd);
            }
        }

        Ok(())
    }

    /// Check Claude authentication
    ///
    /// Verifies if Claude CLI is authenticated by checking for:
    /// 1. claude command availability
    /// 2. Valid authentication state
    ///
    /// # Returns
    /// `true` if Claude is authenticated, `false` otherwise.
    pub fn check_claude_auth(&self) -> bool {
        // First check if claude command exists
        if !self.command_exists("claude") {
            debug!("Claude CLI not found");
            return false;
        }

        // Try to check auth status
        // claude auth status returns 0 if authenticated
        let output = Command::new("claude").args(["auth", "status"]).output();

        match output {
            Ok(out) => {
                if out.status.success() {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    // Check for indicators of successful auth
                    let is_authed = stdout.contains("authenticated")
                        || stdout.contains("logged in")
                        || stdout.contains("API key");

                    if is_authed {
                        debug!("Claude authentication verified");
                    } else {
                        // Command succeeded but output doesn't indicate auth
                        // Assume authenticated if command returns success
                        debug!("Claude auth command succeeded");
                    }
                    true
                } else {
                    debug!("Claude authentication check failed");
                    false
                }
            }
            Err(e) => {
                debug!("Failed to check Claude auth: {}", e);
                false
            }
        }
    }

    /// Verify command exists
    ///
    /// Checks if a command is available in the system PATH.
    ///
    /// # Arguments
    /// * `command` - Name of the command to check
    ///
    /// # Returns
    /// `true` if the command exists and is executable, `false` otherwise.
    pub fn command_exists(&self, command: &str) -> bool {
        // Use 'which' on Unix-like systems, 'where' on Windows
        #[cfg(unix)]
        let check_cmd = "which";
        #[cfg(windows)]
        let check_cmd = "where";

        let output = Command::new(check_cmd).arg(command).output();

        match output {
            Ok(out) => out.status.success(),
            Err(_) => {
                // Fallback: try to run the command with --version
                Command::new(command)
                    .arg("--version")
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
            }
        }
    }

    /// Get the template manager for advanced template operations
    pub fn template_manager(&self) -> &TemplateManager {
        &self.template_manager
    }

    /// Get the template loader for direct template access
    pub fn template_loader(&self) -> &TemplateLoader {
        self.template_manager.loader()
    }

    /// Get the template renderer for custom rendering
    pub fn template_renderer(&self) -> &TemplateRenderer {
        self.template_manager.renderer()
    }

    /// Get a project template by type
    pub fn get_template(&self, template_type: &str) -> Option<&crate::templates::ProjectTemplate> {
        self.template_manager.get_template(template_type)
    }

    /// List all available template types
    pub fn available_templates(&self) -> Vec<String> {
        self.template_manager.available_types()
    }
}

impl Default for EnhancementManager {
    fn default() -> Self {
        Self::new().expect("Failed to create default EnhancementManager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_enhancement_manager_new() {
        let manager = EnhancementManager::new().unwrap();
        // Should have templates loaded
        assert!(!manager.available_templates().is_empty());
    }

    #[test]
    fn test_command_exists() {
        let manager = EnhancementManager::new().unwrap();

        // 'sh' should always exist on Unix
        #[cfg(unix)]
        assert!(manager.command_exists("sh"));

        // Non-existent command
        assert!(!manager.command_exists("this_command_definitely_does_not_exist_12345"));
    }

    #[test]
    fn test_is_known_extension() {
        let manager = EnhancementManager::new().unwrap();

        assert!(manager.is_known_extension("nodejs"));
        assert!(manager.is_known_extension("python"));
        assert!(manager.is_known_extension("rust"));
        assert!(!manager.is_known_extension("unknown_extension"));
    }

    #[test]
    fn test_activate_extensions() {
        let manager = EnhancementManager::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let path = Utf8PathBuf::try_from(temp_dir.path().to_path_buf()).unwrap();

        let extensions = vec!["nodejs".to_string(), "python".to_string()];
        let activated = manager.activate_extensions(&path, &extensions).unwrap();

        assert_eq!(activated.len(), 2);
        assert!(activated.contains(&"nodejs".to_string()));
        assert!(activated.contains(&"python".to_string()));

        // Check that .sindri/extensions.txt was created
        assert!(path.join(".sindri/extensions.txt").exists());
    }

    #[test]
    fn test_activate_extensions_empty() {
        let manager = EnhancementManager::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let path = Utf8PathBuf::try_from(temp_dir.path().to_path_buf()).unwrap();

        let extensions: Vec<String> = vec![];
        let activated = manager.activate_extensions(&path, &extensions).unwrap();

        assert!(activated.is_empty());
    }

    #[test]
    fn test_create_claude_md() {
        let manager = EnhancementManager::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let path = Utf8PathBuf::try_from(temp_dir.path().to_path_buf()).unwrap();

        manager
            .create_claude_md(&path, Some("node"), "test-project")
            .unwrap();

        let claude_md_path = path.join("CLAUDE.md");
        assert!(claude_md_path.exists());

        let content = std::fs::read_to_string(claude_md_path).unwrap();
        assert!(content.contains("test-project"));
    }

    #[test]
    fn test_create_claude_md_default() {
        let manager = EnhancementManager::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let path = Utf8PathBuf::try_from(temp_dir.path().to_path_buf()).unwrap();

        // Use a non-existent template type to trigger default
        manager
            .create_claude_md(&path, Some("nonexistent_type"), "my-project")
            .unwrap();

        let claude_md_path = path.join("CLAUDE.md");
        assert!(claude_md_path.exists());

        let content = std::fs::read_to_string(claude_md_path).unwrap();
        assert!(content.contains("my-project"));
        assert!(content.contains("Project Overview"));
    }

    #[test]
    fn test_create_claude_md_no_overwrite() {
        let manager = EnhancementManager::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let path = Utf8PathBuf::try_from(temp_dir.path().to_path_buf()).unwrap();

        // Create existing CLAUDE.md
        let existing_content = "# Existing Content\nDo not overwrite!";
        std::fs::write(path.join("CLAUDE.md"), existing_content).unwrap();

        // Try to create CLAUDE.md - should skip
        manager
            .create_claude_md(&path, Some("node"), "test-project")
            .unwrap();

        // Content should be unchanged
        let content = std::fs::read_to_string(path.join("CLAUDE.md")).unwrap();
        assert_eq!(content, existing_content);
    }

    #[test]
    fn test_setup_enhancements() {
        let manager = EnhancementManager::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let path = Utf8PathBuf::try_from(temp_dir.path().to_path_buf()).unwrap();

        let options = EnhancementOptions {
            skip_tools: true, // Skip tool verification for test
            skip_auth_check: true,
            git_config: None,
            additional_extensions: vec![],
            interactive: false,
        };

        manager.setup_enhancements(&path, &options).unwrap();

        // CLAUDE.md should be created
        assert!(path.join("CLAUDE.md").exists());
    }

    #[test]
    fn test_get_template() {
        let manager = EnhancementManager::new().unwrap();

        // Should be able to get node template
        let node_template = manager.get_template("node");
        assert!(node_template.is_some());

        // Unknown template should return None
        let unknown = manager.get_template("unknown_type_xyz");
        assert!(unknown.is_none());
    }

    #[test]
    fn test_available_templates() {
        let manager = EnhancementManager::new().unwrap();
        let templates = manager.available_templates();

        // Should have multiple templates
        assert!(templates.len() >= 5);

        // Should include common types
        assert!(templates.contains(&"node".to_string()));
        assert!(templates.contains(&"python".to_string()));
        assert!(templates.contains(&"rust".to_string()));
    }
}
