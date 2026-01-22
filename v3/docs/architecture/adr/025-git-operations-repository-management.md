# ADR 025: Git Operations and Repository Management

**Status**: Accepted
**Date**: 2026-01-22
**Deciders**: Core Team
**Related**: [ADR-023: Phase 7 Project Management Architecture](023-phase-7-project-management-architecture.md), [ADR-024: Template-Based Project Scaffolding](024-template-based-project-scaffolding.md), [Rust Migration Plan](../../planning/rust-cli-migration-v3.md#phase-7-project-management-weeks-20-21)

## Context

The Sindri CLI v3 requires comprehensive Git operations for project management, including repository initialization, cloning, forking, remote configuration, and feature branch workflows. These operations must be reliable, cross-platform, and integrate seamlessly with the project scaffolding system.

### Current v2 Bash Implementation

In v2, Git operations are handled via bash scripts calling the `git` CLI:

**Clone Workflow**:
```bash
git clone "$REPO_URL" "$PROJECT_DIR"
cd "$PROJECT_DIR"
git config user.name "$GIT_NAME"
git config user.email "$GIT_EMAIL"
```

**Fork Workflow** (using GitHub CLI):
```bash
gh repo fork "$REPO_URL" --clone
cd "$PROJECT_NAME"
git remote add upstream "$ORIGINAL_URL"
git fetch upstream
```

**Strengths**:
- Simple CLI wrapper
- Leverages existing `git` and `gh` tools
- Works on all platforms with Git installed

**Weaknesses**:
- External dependency on `git` CLI
- No error recovery for partial failures
- String-based output parsing prone to errors
- Difficult to test without real Git operations
- No async support for parallel operations

### Requirements

**Git Initialization**:
1. Create Git repository in project directory
2. Configure local user.name and user.email
3. Create initial commit with scaffolded files
4. Set default branch name

**Clone Operations**:
1. Clone repository from URL
2. Support shallow clones (`--depth`)
3. Support branch checkout (`--branch`)
4. Progress indication for large repos

**Fork Operations**:
1. Fork repository via GitHub API
2. Configure origin → fork, upstream → original
3. Set up Git aliases for common fork workflows
4. Fetch upstream branches

**Feature Branches**:
1. Create feature branch from main/develop
2. Set up tracking to origin
3. Push branch to remote

**Configuration Management**:
1. Project-local Git configuration (not global)
2. Override user.name and user.email per project
3. Preserve existing Git configuration

## Decision

We implement Git operations using the `git2` crate (libgit2 bindings) for core operations and GitHub CLI (`gh`) for fork workflow, with comprehensive error handling and async support.

### a) Git Library Choice

**Decision**: Use `git2` crate for Git operations, fall back to `git` CLI for unsupported operations.

**Architecture**:
```rust
// crates/sindri-project/src/git.rs

use git2::{Repository, Signature, IndexAddOption, RemoteCallbacks};
use std::path::{Path, PathBuf};
use tokio::process::Command;

pub struct GitManager {
    // Configuration
}

impl GitManager {
    pub fn new() -> Self {
        Self {}
    }

    /// Initialize Git repository in directory
    pub async fn init_repo(
        &self,
        path: &Path,
        user_name: Option<&str>,
        user_email: Option<&str>,
    ) -> Result<GitConfig, ProjectError> {
        // Initialize repository
        let repo = Repository::init(path)?;

        // Get or create Git configuration
        let (name, email) = self.get_or_prompt_git_config(
            &repo,
            user_name,
            user_email,
        ).await?;

        // Set local config (not global)
        let mut config = repo.config()?;
        config.set_str("user.name", &name)?;
        config.set_str("user.email", &email)?;

        // Set default branch name to "main"
        config.set_str("init.defaultBranch", "main")?;

        // Create initial branch
        let head = repo.head()?;
        let branch_name = head
            .shorthand()
            .unwrap_or("main")
            .to_string();

        Ok(GitConfig {
            user_name: name,
            user_email: email,
            branch: branch_name,
            remotes: Vec::new(),
        })
    }

    /// Get Git config from repo, fall back to global, prompt if neither exists
    async fn get_or_prompt_git_config(
        &self,
        repo: &Repository,
        override_name: Option<&str>,
        override_email: Option<&str>,
    ) -> Result<(String, String), ProjectError> {
        // Use override if provided
        if let (Some(name), Some(email)) = (override_name, override_email) {
            return Ok((name.to_string(), email.to_string()));
        }

        // Try repository config
        if let Ok(config) = repo.config() {
            if let (Ok(name), Ok(email)) = (
                config.get_string("user.name"),
                config.get_string("user.email"),
            ) {
                return Ok((name, email));
            }
        }

        // Try global config
        if let Ok(config) = git2::Config::open_default() {
            if let (Ok(name), Ok(email)) = (
                config.get_string("user.name"),
                config.get_string("user.email"),
            ) {
                return Ok((name, email));
            }
        }

        // Prompt user for Git config
        self.prompt_git_config().await
    }

    async fn prompt_git_config(&self) -> Result<(String, String), ProjectError> {
        use dialoguer::Input;

        println!("\nGit configuration not found. Please provide:");

        let name: String = Input::new()
            .with_prompt("Git user name")
            .interact_text()?;

        let email: String = Input::new()
            .with_prompt("Git user email")
            .validate_with(|input: &String| -> Result<(), &str> {
                if input.contains('@') {
                    Ok(())
                } else {
                    Err("Email must contain @")
                }
            })
            .interact_text()?;

        Ok((name, email))
    }

    /// Create initial commit with all files
    pub async fn create_initial_commit(
        &self,
        path: &Path,
        project_name: &str,
    ) -> Result<(), ProjectError> {
        let repo = Repository::open(path)?;

        // Add all files
        let mut index = repo.index()?;
        index.add_all(["."].iter(), IndexAddOption::DEFAULT, None)?;
        index.write()?;

        // Create tree from index
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;

        // Get signature from config
        let sig = repo.signature()?;

        // Create commit
        let message = format!("feat: initial project setup for {}", project_name);

        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            &message,
            &tree,
            &[], // No parents (initial commit)
        )?;

        Ok(())
    }
}
```

**Reasoning**: `git2` provides:
- **No External Dependency**: Bundled libgit2, no `git` CLI required
- **Type Safety**: Rust API prevents common Git errors
- **Cross-Platform**: Works consistently on Linux, macOS, Windows
- **Performance**: Native library is faster than CLI spawning
- **Error Handling**: Structured errors with context

### b) Clone Operations

**Decision**: Use `git2` for cloning with progress callbacks, support shallow clones and branch checkout.

**Implementation**:
```rust
impl GitManager {
    /// Clone repository from URL
    pub async fn clone_repo(
        &self,
        url: &str,
        destination: &Path,
        options: CloneOptions,
    ) -> Result<GitConfig, ProjectError> {
        let mut fetch_options = git2::FetchOptions::new();

        // Configure progress callback
        let progress = indicatif::ProgressBar::new(100);
        progress.set_style(
            indicatif::ProgressStyle::default_bar()
                .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta})")?
                .progress_chars("#>-"),
        );

        let mut callbacks = RemoteCallbacks::new();
        callbacks.transfer_progress(|stats| {
            progress.set_length(stats.total_objects() as u64);
            progress.set_position(stats.received_objects() as u64);
            progress.set_message(format!(
                "Receiving objects: {}/{} ({} bytes)",
                stats.received_objects(),
                stats.total_objects(),
                stats.received_bytes()
            ));
            true
        });

        fetch_options.remote_callbacks(callbacks);

        // Configure shallow clone if requested
        if let Some(depth) = options.depth {
            fetch_options.depth(depth as i32);
        }

        // Build clone options
        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch_options);

        // Set branch if specified
        if let Some(ref branch) = options.branch {
            builder.branch(branch);
        }

        // Perform clone
        let repo = builder.clone(url, destination)?;

        progress.finish_with_message("Clone complete");

        // Get Git config
        let config = repo.config()?;
        let user_name = config.get_string("user.name")
            .unwrap_or_else(|_| "Unknown".to_string());
        let user_email = config.get_string("user.email")
            .unwrap_or_else(|_| "unknown@example.com".to_string());

        // Get current branch
        let head = repo.head()?;
        let branch_name = head
            .shorthand()
            .unwrap_or("main")
            .to_string();

        // Get remotes
        let remotes = repo.remotes()?
            .iter()
            .filter_map(|r| r)
            .map(|name| {
                let remote = repo.find_remote(name).ok()?;
                let url = remote.url()?.to_string();
                Some(GitRemote {
                    name: name.to_string(),
                    url,
                })
            })
            .collect::<Vec<_>>();

        Ok(GitConfig {
            user_name,
            user_email,
            branch: branch_name,
            remotes,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct CloneOptions {
    /// Shallow clone depth
    pub depth: Option<u32>,

    /// Branch to checkout
    pub branch: Option<String>,
}
```

**Reasoning**: Native cloning with `git2` provides:
- **Progress Indication**: Real-time progress bars for large repos
- **Shallow Clones**: Reduces download size and time
- **Branch Checkout**: Directly checkout target branch
- **Error Recovery**: Structured error handling

### c) Fork Workflow

**Decision**: Use GitHub CLI (`gh`) for forking, `git2` for remote configuration.

**Implementation**:
```rust
impl GitManager {
    /// Fork repository and clone
    pub async fn fork_and_clone(
        &self,
        repo_url: &str,
        destination: &Path,
    ) -> Result<GitConfig, ProjectError> {
        // Check if gh CLI is available
        if !self.check_gh_installed().await? {
            return Err(ProjectError::MissingDependency {
                tool: "gh".to_string(),
                install_hint: "Install GitHub CLI: https://cli.github.com/".to_string(),
            });
        }

        // Check if gh is authenticated
        if !self.check_gh_authenticated().await? {
            return Err(ProjectError::GitHubNotAuthenticated {
                hint: "Run: gh auth login".to_string(),
            });
        }

        // Parse repository URL to get owner/repo
        let repo_spec = self.parse_github_url(repo_url)?;

        // Fork repository using gh CLI
        println!("Forking repository...");
        let output = Command::new("gh")
            .args(&["repo", "fork", &repo_spec, "--clone=false"])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ProjectError::ForkFailed {
                repo: repo_spec,
                reason: stderr.to_string(),
            });
        }

        // Get fork URL from gh
        let fork_url = self.get_fork_url(&repo_spec).await?;

        // Clone the fork
        let git_config = self.clone_repo(&fork_url, destination, CloneOptions::default()).await?;

        // Add upstream remote
        let repo = Repository::open(destination)?;
        repo.remote("upstream", repo_url)?;

        // Fetch upstream
        println!("Fetching upstream...");
        let mut upstream = repo.find_remote("upstream")?;
        upstream.fetch(&["refs/heads/*:refs/remotes/upstream/*"], None, None)?;

        // Update git config with remotes
        let remotes = vec![
            GitRemote {
                name: "origin".to_string(),
                url: fork_url,
            },
            GitRemote {
                name: "upstream".to_string(),
                url: repo_url.to_string(),
            },
        ];

        Ok(GitConfig {
            remotes,
            ..git_config
        })
    }

    async fn check_gh_installed(&self) -> Result<bool, ProjectError> {
        let output = Command::new("gh")
            .arg("--version")
            .output()
            .await;

        Ok(output.is_ok())
    }

    async fn check_gh_authenticated(&self) -> Result<bool, ProjectError> {
        let output = Command::new("gh")
            .args(&["auth", "status"])
            .output()
            .await?;

        Ok(output.status.success())
    }

    async fn get_fork_url(&self, repo_spec: &str) -> Result<String, ProjectError> {
        let output = Command::new("gh")
            .args(&["repo", "view", repo_spec, "--json", "sshUrl", "-q", ".sshUrl"])
            .output()
            .await?;

        if !output.status.success() {
            return Err(ProjectError::GitHubAPIError {
                message: "Failed to get fork URL".to_string(),
            });
        }

        let url = String::from_utf8(output.stdout)?
            .trim()
            .to_string();

        Ok(url)
    }

    fn parse_github_url(&self, url: &str) -> Result<String, ProjectError> {
        // Parse various GitHub URL formats:
        // https://github.com/owner/repo
        // git@github.com:owner/repo.git
        // owner/repo

        if url.contains("github.com") {
            // Extract owner/repo from URL
            let parts: Vec<&str> = url
                .trim_end_matches(".git")
                .split('/')
                .collect();

            if parts.len() >= 2 {
                let owner = parts[parts.len() - 2];
                let repo = parts[parts.len() - 1];
                return Ok(format!("{}/{}", owner, repo));
            }
        } else if url.contains('/') && url.split('/').count() == 2 {
            // Already in owner/repo format
            return Ok(url.to_string());
        }

        Err(ProjectError::InvalidGitHubURL {
            url: url.to_string(),
        })
    }

    /// Set up Git aliases for common fork workflows
    pub async fn setup_fork_aliases(
        &self,
        repo_path: &Path,
    ) -> Result<(), ProjectError> {
        let repo = Repository::open(repo_path)?;
        let mut config = repo.config()?;

        // Alias: sync-fork (fetch upstream and merge)
        config.set_str(
            "alias.sync-fork",
            "!git fetch upstream && git merge upstream/main",
        )?;

        // Alias: update-fork (fetch upstream and rebase)
        config.set_str(
            "alias.update-fork",
            "!git fetch upstream && git rebase upstream/main",
        )?;

        // Alias: pr (create pull request via gh)
        config.set_str(
            "alias.pr",
            "!gh pr create",
        )?;

        Ok(())
    }
}
```

**Reasoning**: Hybrid approach (gh + git2) provides:
- **GitHub Integration**: Native fork support via `gh` CLI
- **Type Safety**: `git2` for remote configuration
- **User Experience**: Automated fork workflow
- **Convenience**: Git aliases for common operations

### d) Feature Branch Workflow

**Decision**: Use `git2` for branch creation and checkout, support push to remote.

**Implementation**:
```rust
impl GitManager {
    /// Create feature branch and optionally push to remote
    pub async fn create_feature_branch(
        &self,
        repo_path: &Path,
        branch_name: &str,
        base_branch: Option<&str>,
        push_to_remote: bool,
    ) -> Result<(), ProjectError> {
        let repo = Repository::open(repo_path)?;

        // Determine base branch (default to current branch)
        let base = if let Some(base) = base_branch {
            repo.find_branch(base, git2::BranchType::Local)?
                .get()
                .peel_to_commit()?
        } else {
            repo.head()?.peel_to_commit()?
        };

        // Create new branch
        repo.branch(branch_name, &base, false)?;

        // Checkout new branch
        repo.set_head(&format!("refs/heads/{}", branch_name))?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;

        println!("Created and switched to branch: {}", branch_name);

        // Push to remote if requested
        if push_to_remote {
            self.push_branch(&repo, branch_name).await?;
        }

        Ok(())
    }

    async fn push_branch(
        &self,
        repo: &Repository,
        branch_name: &str,
    ) -> Result<(), ProjectError> {
        let mut remote = repo.find_remote("origin")?;

        // Set up push with progress
        let mut callbacks = RemoteCallbacks::new();
        callbacks.push_update_reference(|refname, status| {
            if let Some(s) = status {
                println!("Push failed for {}: {}", refname, s);
            } else {
                println!("Pushed {}", refname);
            }
            Ok(())
        });

        let mut push_options = git2::PushOptions::new();
        push_options.remote_callbacks(callbacks);

        // Push branch to origin
        let refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);
        remote.push(&[&refspec], Some(&mut push_options))?;

        // Set upstream tracking
        let mut branch = repo.find_branch(branch_name, git2::BranchType::Local)?;
        branch.set_upstream(Some(&format!("origin/{}", branch_name)))?;

        println!("Branch {} pushed to origin and set as upstream", branch_name);

        Ok(())
    }
}
```

**Reasoning**: Native branch operations provide:
- **Atomicity**: Branch creation and checkout are atomic
- **Tracking**: Automatic upstream configuration
- **Error Handling**: Detect and report branch conflicts
- **User Experience**: Clear feedback on branch operations

### e) Local Git Configuration Management

**Decision**: Always use project-local Git configuration, never modify global config.

**Implementation**:
```rust
impl GitManager {
    /// Apply Git configuration overrides to repository
    pub async fn apply_git_config(
        &self,
        repo_path: &Path,
        user_name: Option<&str>,
        user_email: Option<&str>,
    ) -> Result<(), ProjectError> {
        let repo = Repository::open(repo_path)?;
        let mut config = repo.config()?;

        // Set user.name if provided
        if let Some(name) = user_name {
            config.set_str("user.name", name)?;
            println!("Set Git user.name: {}", name);
        }

        // Set user.email if provided
        if let Some(email) = user_email {
            config.set_str("user.email", email)?;
            println!("Set Git user.email: {}", email);
        }

        Ok(())
    }

    /// Get current Git configuration
    pub fn get_git_config(
        &self,
        repo_path: &Path,
    ) -> Result<GitConfig, ProjectError> {
        let repo = Repository::open(repo_path)?;
        let config = repo.config()?;

        let user_name = config.get_string("user.name")
            .unwrap_or_else(|_| "Unknown".to_string());

        let user_email = config.get_string("user.email")
            .unwrap_or_else(|_| "unknown@example.com".to_string());

        let head = repo.head()?;
        let branch = head
            .shorthand()
            .unwrap_or("main")
            .to_string();

        let remotes = repo.remotes()?
            .iter()
            .filter_map(|r| r)
            .map(|name| {
                let remote = repo.find_remote(name).ok()?;
                let url = remote.url()?.to_string();
                Some(GitRemote {
                    name: name.to_string(),
                    url,
                })
            })
            .collect();

        Ok(GitConfig {
            user_name,
            user_email,
            branch,
            remotes,
        })
    }
}
```

**Reasoning**: Project-local configuration ensures:
- **Isolation**: Each project has independent Git config
- **Consistency**: Same config across all team members
- **No Side Effects**: Never modifies user's global Git settings
- **Flexibility**: Different projects can use different identities

## Consequences

### Positive

1. **No Git CLI Dependency**: `git2` bundles libgit2, works without `git` installed
2. **Type Safety**: Compile-time guarantees for Git operations
3. **Cross-Platform**: Consistent behavior on Linux, macOS, Windows
4. **Progress Indication**: Real-time progress for clone operations
5. **Error Recovery**: Structured errors with context and suggestions
6. **GitHub Integration**: Seamless fork workflow via `gh` CLI
7. **Local Configuration**: Project-isolated Git settings
8. **Feature Branch Support**: One-command branch creation and push
9. **Performance**: Native library faster than CLI spawning
10. **Testability**: Mock Git operations for unit tests

### Negative

1. **Binary Size**: libgit2 increases binary size (~2MB)
2. **GitHub CLI Dependency**: Fork workflow requires `gh` CLI
3. **Limited SSH Support**: `git2` SSH support requires additional setup
4. **Complexity**: More complex than simple CLI wrapper
5. **libgit2 Limitations**: Some advanced Git features not available

### Neutral

1. **Git Library Choice**: `git2` vs `gitoxide` (git2 chosen for maturity)
2. **Fork Implementation**: `gh` CLI vs GitHub API (gh chosen for simplicity)
3. **Progress Reporting**: Callbacks vs polling (callbacks chosen)

## Alternatives Considered

### 1. Git CLI Wrapper Only

**Description**: Shell out to `git` CLI for all operations, no `git2` crate.

**Pros**:
- Simpler implementation
- Smaller binary size
- Supports all Git features
- No platform-specific compilation

**Cons**:
- Requires `git` CLI installed
- String parsing prone to errors
- No type safety
- Difficult to test
- Poor error handling

**Rejected**: `git2` provides better type safety and error handling.

### 2. Gitoxide Library

**Description**: Use pure-Rust `gitoxide` instead of `git2` (libgit2 bindings).

**Pros**:
- Pure Rust (no C dependencies)
- Modern API design
- Better async support
- Smaller binary size

**Cons**:
- Less mature than git2
- Fewer features implemented
- Less documentation
- Smaller ecosystem

**Rejected**: `git2` is more mature and feature-complete. Revisit gitoxide in future.

### 3. GitHub API for Forking

**Description**: Use GitHub REST/GraphQL API directly instead of `gh` CLI.

**Pros**:
- No external dependency
- More control over API calls
- Can handle authentication in Rust

**Cons**:
- Must implement OAuth flow
- Token management complexity
- More code to maintain
- Harder to debug

**Rejected**: `gh` CLI handles authentication and provides simpler interface.

### 4. Always Modify Global Git Config

**Description**: Set user.name and user.email globally, not per-project.

**Pros**:
- Simpler implementation
- User config persists across projects
- Fewer configuration files

**Cons**:
- Overwrites user's global settings
- No project-specific identities
- Surprising behavior
- Security risk (wrong identity)

**Rejected**: Project-local config is safer and more flexible.

### 5. No Fork Support

**Description**: Only support `clone`, users manually fork via GitHub UI.

**Pros**:
- No GitHub CLI dependency
- Simpler implementation
- No authentication handling

**Cons**:
- Poor UX for contributors
- Manual remote configuration
- Extra steps for common workflow

**Rejected**: Fork workflow is common enough to warrant native support.

## Compliance

- ✅ Git repository initialization
- ✅ Clone from URL with progress indication
- ✅ Shallow clone support (`--depth`)
- ✅ Branch checkout (`--branch`)
- ✅ Fork workflow via GitHub CLI
- ✅ Remote configuration (origin/upstream)
- ✅ Feature branch creation and push
- ✅ Project-local Git configuration
- ✅ Git aliases for fork workflows
- ✅ Comprehensive error handling

## Notes

### Git Configuration Precedence

Git configuration is loaded in this order (highest to lowest priority):

1. **Command-line overrides**: `--git-name`, `--git-email`
2. **Project-local config**: `.git/config` in project directory
3. **Global config**: `~/.gitconfig`
4. **Prompt user**: If no config found, interactively prompt

This ensures user always has control over Git identity without modifying global settings.

### Fork Remote Configuration

After forking, remotes are configured as:

```
origin   → git@github.com:YOUR_USERNAME/repo.git (your fork)
upstream → git@github.com:ORIGINAL_OWNER/repo.git (original repo)
```

This is the standard convention for contributing to open-source projects.

### Git Aliases for Fork Workflow

Configured aliases:

| Alias | Command | Description |
|-------|---------|-------------|
| `sync-fork` | `git fetch upstream && git merge upstream/main` | Merge upstream changes |
| `update-fork` | `git fetch upstream && git rebase upstream/main` | Rebase on upstream |
| `pr` | `gh pr create` | Create pull request |

Usage:
```bash
git sync-fork       # Sync fork with upstream
git update-fork     # Rebase on upstream
git pr              # Create pull request
```

### Platform-Specific Considerations

**SSH Keys**:
- `git2` uses system SSH agent on Unix
- Windows requires `ssh-agent` service or Pageant
- Fallback to HTTPS if SSH fails

**File Permissions**:
- Unix: Preserve executable bit for scripts
- Windows: Different permission model, handled by `git2`

**Line Endings**:
- Respect `.gitattributes` configuration
- Auto-detect platform defaults (LF on Unix, CRLF on Windows)

### Error Recovery Strategies

| Error | Recovery |
|-------|----------|
| Clone timeout | Retry with exponential backoff (max 3 attempts) |
| Fork already exists | Detect and use existing fork |
| Branch exists | Prompt to checkout or create new name |
| Network failure | Suggest checking connection, retry |
| Authentication failure | Clear instructions to run `gh auth login` |

## Related Decisions

- [ADR-023: Phase 7 Project Management Architecture](023-phase-7-project-management-architecture.md) - Overall architecture
- [ADR-024: Template-Based Project Scaffolding](024-template-based-project-scaffolding.md) - Template system integration
