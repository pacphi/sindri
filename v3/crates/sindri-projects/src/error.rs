//! Error types for sindri-projects

use thiserror::Error;

/// Result type alias using sindri-projects's Error type
pub type Result<T> = std::result::Result<T, Error>;

/// Project management error types
#[derive(Error, Debug)]
pub enum Error {
    /// Project already exists
    #[error("Project already exists at: {path}")]
    ProjectExists { path: String },

    /// Project not found
    #[error("Project not found: {name}")]
    ProjectNotFound { name: String },

    /// Invalid project name
    #[error("Invalid project name: {name}. Must be lowercase alphanumeric with hyphens")]
    InvalidProjectName { name: String },

    /// Unknown project type
    #[error("Unknown project type: {project_type}. Available types: {available}")]
    UnknownProjectType {
        project_type: String,
        available: String,
    },

    /// Template not found
    #[error("Template not found: {template}")]
    TemplateNotFound { template: String },

    /// Template variable missing
    #[error("Required template variable missing: {variable}")]
    MissingTemplateVariable { variable: String },

    /// Template rendering error
    #[error("Template rendering failed: {message}")]
    TemplateRenderError { message: String },

    /// Git operation failed
    #[error("Git operation failed: {message}")]
    GitOperation { message: String },

    /// Git command not found
    #[error("Git command not found. Please ensure git is installed and in PATH")]
    GitNotFound,

    /// GitHub CLI (gh) not found
    #[error("GitHub CLI (gh) not found. Please install gh CLI: https://cli.github.com/")]
    GhNotFound,

    /// GitHub CLI not authenticated
    #[error("GitHub CLI is not authenticated. Please run: gh auth login")]
    GhNotAuthenticated,

    /// Invalid repository URL
    #[error("Invalid repository URL: {url}")]
    InvalidRepoUrl { url: String },

    /// Repository already exists
    #[error("Repository already exists at: {path}")]
    RepoExists { path: String },

    /// Repository not found
    #[error("Repository not found at: {path}")]
    RepoNotFound { path: String },

    /// Invalid branch name
    #[error("Invalid branch name: {branch}")]
    InvalidBranch { branch: String },

    /// Branch already exists
    #[error("Branch already exists: {branch}")]
    BranchExists { branch: String },

    /// Remote not found
    #[error("Remote '{remote}' not found")]
    RemoteNotFound { remote: String },

    /// Remote already exists
    #[error("Remote '{remote}' already exists")]
    RemoteExists { remote: String },

    /// Invalid git config
    #[error("Invalid git config: {message}")]
    InvalidConfig { message: String },

    /// Fork failed
    #[error("Failed to fork repository: {message}")]
    ForkFailed { message: String },

    /// Clone failed
    #[error("Failed to clone repository: {message}")]
    CloneFailed { message: String },

    /// Process execution error
    #[error("Process execution failed: {0}")]
    ProcessExecution(String),

    /// Git user configuration invalid
    #[error("Git user configuration invalid: {message}")]
    InvalidGitConfig { message: String },

    /// Enhancement setup failed
    #[error("Enhancement setup failed: {message}")]
    EnhancementError { message: String },

    /// Extension activation failed
    #[error("Failed to activate extension: {extension}. Reason: {reason}")]
    ExtensionActivationError { extension: String, reason: String },

    /// Dependency installation failed
    #[error("Dependency installation failed: {dependency}")]
    DependencyError { dependency: String },

    /// Command not found
    #[error("Required command not found: {command}")]
    CommandNotFound { command: String },

    /// YAML parsing error
    #[error("YAML parsing error: {0}")]
    YamlParse(#[from] serde_yaml::Error),

    /// JSON parsing error
    #[error("JSON parsing error: {0}")]
    JsonParse(#[from] serde_json::Error),

    /// Template error from Tera
    #[error("Template error: {0}")]
    Tera(#[from] tera::Error),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Core library error
    #[error("Core error: {0}")]
    Core(#[from] sindri_core::Error),

    /// Regex error
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    /// Invalid path
    #[error("Invalid path: {path}")]
    InvalidPath { path: String },

    /// Ambiguous project type
    #[error("Multiple project types detected: {types}. Please specify one explicitly")]
    AmbiguousProjectType { types: String },
}

impl Error {
    /// Create a project exists error
    pub fn project_exists(path: impl Into<String>) -> Self {
        Self::ProjectExists { path: path.into() }
    }

    /// Create a project not found error
    pub fn project_not_found(name: impl Into<String>) -> Self {
        Self::ProjectNotFound { name: name.into() }
    }

    /// Create an invalid project name error
    pub fn invalid_project_name(name: impl Into<String>) -> Self {
        Self::InvalidProjectName { name: name.into() }
    }

    /// Create an unknown project type error
    pub fn unknown_project_type(
        project_type: impl Into<String>,
        available: impl Into<String>,
    ) -> Self {
        Self::UnknownProjectType {
            project_type: project_type.into(),
            available: available.into(),
        }
    }

    /// Create a template not found error
    pub fn template_not_found(template: impl Into<String>) -> Self {
        Self::TemplateNotFound {
            template: template.into(),
        }
    }

    /// Create a missing template variable error
    pub fn missing_template_variable(variable: impl Into<String>) -> Self {
        Self::MissingTemplateVariable {
            variable: variable.into(),
        }
    }

    /// Create a template render error
    pub fn template_render_error(message: impl Into<String>) -> Self {
        Self::TemplateRenderError {
            message: message.into(),
        }
    }

    /// Create a git operation error
    pub fn git_operation(message: impl Into<String>) -> Self {
        Self::GitOperation {
            message: message.into(),
        }
    }

    /// Create an invalid repo URL error
    pub fn invalid_repo_url(url: impl Into<String>) -> Self {
        Self::InvalidRepoUrl { url: url.into() }
    }

    /// Create a repo exists error
    pub fn repo_exists(path: impl Into<String>) -> Self {
        Self::RepoExists {
            path: path.into(),
        }
    }

    /// Create a repo not found error
    pub fn repo_not_found(path: impl Into<String>) -> Self {
        Self::RepoNotFound {
            path: path.into(),
        }
    }

    /// Create an invalid branch error
    pub fn invalid_branch(branch: impl Into<String>) -> Self {
        Self::InvalidBranch {
            branch: branch.into(),
        }
    }

    /// Create a branch exists error
    pub fn branch_exists(branch: impl Into<String>) -> Self {
        Self::BranchExists {
            branch: branch.into(),
        }
    }

    /// Create a remote not found error
    pub fn remote_not_found(remote: impl Into<String>) -> Self {
        Self::RemoteNotFound {
            remote: remote.into(),
        }
    }

    /// Create a remote exists error
    pub fn remote_exists(remote: impl Into<String>) -> Self {
        Self::RemoteExists {
            remote: remote.into(),
        }
    }

    /// Create an invalid config error
    pub fn invalid_config(message: impl Into<String>) -> Self {
        Self::InvalidConfig {
            message: message.into(),
        }
    }

    /// Create a fork failed error
    pub fn fork_failed(message: impl Into<String>) -> Self {
        Self::ForkFailed {
            message: message.into(),
        }
    }

    /// Create a clone failed error
    pub fn clone_failed(message: impl Into<String>) -> Self {
        Self::CloneFailed {
            message: message.into(),
        }
    }

    /// Create a process execution error
    pub fn process_execution(message: impl Into<String>) -> Self {
        Self::ProcessExecution(message.into())
    }

    /// Create an invalid git config error
    pub fn invalid_git_config(message: impl Into<String>) -> Self {
        Self::InvalidGitConfig {
            message: message.into(),
        }
    }

    /// Create an enhancement error
    pub fn enhancement_error(message: impl Into<String>) -> Self {
        Self::EnhancementError {
            message: message.into(),
        }
    }

    /// Create an extension activation error
    pub fn extension_activation_error(
        extension: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::ExtensionActivationError {
            extension: extension.into(),
            reason: reason.into(),
        }
    }

    /// Create a dependency error
    pub fn dependency_error(dependency: impl Into<String>) -> Self {
        Self::DependencyError {
            dependency: dependency.into(),
        }
    }

    /// Create a command not found error
    pub fn command_not_found(command: impl Into<String>) -> Self {
        Self::CommandNotFound {
            command: command.into(),
        }
    }

    /// Create an invalid path error
    pub fn invalid_path(path: impl Into<String>) -> Self {
        Self::InvalidPath { path: path.into() }
    }

    /// Create an ambiguous project type error
    pub fn ambiguous_project_type(types: impl Into<String>) -> Self {
        Self::AmbiguousProjectType {
            types: types.into(),
        }
    }
}
