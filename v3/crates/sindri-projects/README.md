# sindri-projects

Project management library for Sindri CLI v3 (Phase 7: Project Management).

This crate provides async Git operations for project scaffolding, repository management, and development workflow automation.

## Features

- **Repository Initialization**: Create new git repositories with default configuration
- **Clone Operations**: Clone repositories with shallow clone, branch selection, and local config
- **Fork Management**: Fork repositories via GitHub CLI with automatic remote setup
- **Git Configuration**: Local and global git user configuration
- **Remote Management**: Add, remove, and manage git remotes (origin, upstream)
- **Branch Operations**: Create, checkout, and manage branches
- **Fork Workflows**: Setup fork-specific git aliases for upstream synchronization

## Architecture

This crate is part of the Sindri v3 Rust migration, implementing functionality previously handled by bash scripts:

- `/v2/docker/lib/git.sh` - Git utility functions
- `/v2/cli/clone-project` - Repository cloning
- `/v2/cli/new-project` - Project scaffolding

## Module Structure

```
sindri-projects/
├── src/
│   ├── lib.rs           # Public API
│   ├── error.rs         # Error types
│   └── git/             # Git operations
│       ├── mod.rs       # Public git API
│       ├── init.rs      # Repository initialization
│       ├── clone.rs     # Clone and fork operations
│       ├── config.rs    # Git configuration
│       └── remote.rs    # Remote management
```

## Usage Examples

### Initialize a New Repository

```rust
use sindri_projects::git::{init_repository, InitOptions};
use camino::Utf8Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Utf8Path::new("/tmp/my-project");
    tokio::fs::create_dir_all(path).await?;

    let options = InitOptions {
        default_branch: Some("main".to_string()),
        initial_commit_message: "chore: initial commit".to_string(),
        create_initial_commit: true,
    };

    init_repository(path, &options).await?;
    println!("Repository initialized at: {}", path);

    Ok(())
}
```

### Clone a Repository with Options

```rust
use sindri_projects::git::{clone_repository, CloneOptions};
use camino::Utf8Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = CloneOptions {
        depth: Some(1),  // Shallow clone
        branch: Some("main".to_string()),
        feature_branch: Some("my-feature".to_string()),
        git_name: Some("John Doe".to_string()),
        git_email: Some("john@example.com".to_string()),
    };

    let repo_path = clone_repository(
        "https://github.com/user/repo.git",
        Utf8Path::new("/tmp/my-clone"),
        &options
    ).await?;

    println!("Cloned to: {}", repo_path);

    Ok(())
}
```

### Fork a Repository

```rust
use sindri_projects::git::{fork_repository, ForkOptions};
use camino::Utf8Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Requires GitHub CLI (gh) to be installed and authenticated
    let options = ForkOptions {
        setup_aliases: true,
        feature_branch: Some("add-feature".to_string()),
        git_name: Some("John Doe".to_string()),
        git_email: Some("john@example.com".to_string()),
        ..Default::default()
    };

    let fork_path = fork_repository(
        "https://github.com/original/repo.git",
        Utf8Path::new("/tmp/my-fork"),
        &options
    ).await?;

    println!("Forked to: {}", fork_path);

    Ok(())
}
```

### Configure Git User

```rust
use sindri_projects::git::{configure_user, ConfigScope};
use camino::Utf8Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo_path = Utf8Path::new("/tmp/my-repo");

    // Set local (repository-specific) git config
    configure_user(
        Some(repo_path),
        Some("John Doe"),
        Some("john@example.com"),
        ConfigScope::Local,
    ).await?;

    println!("Git user configured");

    Ok(())
}
```

### Manage Remotes

```rust
use sindri_projects::git::{add_remote, setup_fork_remotes};
use camino::Utf8Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo_path = Utf8Path::new("/tmp/my-repo");

    // Add a remote
    add_remote(
        repo_path,
        "origin",
        "https://github.com/user/repo.git"
    ).await?;

    // Setup fork remotes (origin + upstream)
    setup_fork_remotes(
        repo_path,
        "https://github.com/user/repo.git",      // fork URL (origin)
        "https://github.com/original/repo.git"   // upstream URL
    ).await?;

    println!("Remotes configured");

    Ok(())
}
```

### Create and Checkout Branches

```rust
use sindri_projects::git::{create_branch, checkout_branch};
use camino::Utf8Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo_path = Utf8Path::new("/tmp/my-repo");

    // Create and checkout a new branch
    create_branch(repo_path, "feature/new-feature", true).await?;

    // Checkout an existing branch
    checkout_branch(repo_path, "main").await?;

    Ok(())
}
```

## Git Operations Reference

### Repository Initialization

- `init_repository(path, options)` - Initialize a new git repository
- `create_branch(path, branch_name, checkout)` - Create a new branch
- `checkout_branch(path, branch_name)` - Checkout an existing branch

### Clone Operations

- `clone_repository(url, destination, options)` - Clone a repository
- `fork_repository(url, destination, options)` - Fork via GitHub CLI

### Configuration

- `configure_user(path, name, email, scope)` - Set git user info
- `set_config_value(path, key, value, scope)` - Set any git config
- `get_config_value(path, key, scope)` - Get git config value
- `get_current_branch(path)` - Get current branch name
- `setup_fork_aliases(path)` - Setup fork management aliases

### Remote Management

- `add_remote(path, name, url)` - Add a remote
- `remove_remote(path, name)` - Remove a remote
- `get_remote_url(path, name)` - Get remote URL
- `remote_exists(path, name)` - Check if remote exists
- `list_remotes(path)` - List all remotes
- `setup_fork_remotes(path, fork_url, upstream_url)` - Setup fork remotes
- `fetch_remote(path, remote, branch)` - Fetch from remote

## Fork Management Aliases

When `setup_fork_aliases()` is called, the following git aliases are configured:

- `git sync-upstream` - Fetch and merge upstream changes to main
- `git push-fork` - Push current branch to your fork
- `git update-from-upstream` - Rebase current branch on upstream/main
- `git pr-branch <name>` - Create new branch from upstream/main
- `git fork-status` - Show fork remotes and branch tracking

## Error Handling

All operations return `Result<T, Error>` where `Error` provides detailed error messages for:

- Git command failures
- Missing dependencies (git, gh CLI)
- Invalid URLs or paths
- Remote/branch conflicts
- Configuration errors

## Dependencies

### Required

- `git` - Must be installed and in PATH
- `gh` (GitHub CLI) - Required only for fork operations

### Crate Dependencies

- `tokio` - Async runtime for process execution
- `camino` - UTF-8 path handling
- `thiserror` - Error type definitions
- `serde` - Configuration serialization
- `tera` - Template rendering (future use)

## Testing

Run the test suite:

```bash
cargo test --package sindri-projects
```

Tests include:

- Repository initialization
- Branch creation and validation
- Remote management
- Git configuration
- Fork setup
- URL validation

## Future Enhancements

Phase 7 will expand this crate with:

- Project template management
- Scaffolding for different project types
- Enhancement setup (hooks, dependencies)
- Integration with `new-project` and `clone-project` commands

## Migration from v2

This crate replaces bash script functionality:

| v2 Bash Script                       | v3 Rust Module                    |
| ------------------------------------ | --------------------------------- |
| `git.sh::init_git_repo`              | `git::init::init_repository`      |
| `git.sh::setup_fork_remotes`         | `git::remote::setup_fork_remotes` |
| `git.sh::setup_fork_aliases`         | `git::config::setup_fork_aliases` |
| `git.sh::apply_git_config_overrides` | `git::config::configure_user`     |
| `clone-project` (lines 142-220)      | `git::clone::clone_repository`    |
| `clone-project --fork`               | `git::clone::fork_repository`     |

## License

MIT - See workspace LICENSE file
