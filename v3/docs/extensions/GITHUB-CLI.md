# GitHub CLI Extension

> Version: 2.0.0 | Category: devops | Last Updated: 2026-01-26

## Overview

GitHub CLI authentication and workflow configuration. Provides the `gh` command for interacting with GitHub from the command line.

## What It Provides

| Tool | Type     | License | Description                          |
| ---- | -------- | ------- | ------------------------------------ |
| gh   | cli-tool | MIT     | GitHub CLI for repository management |

## Requirements

- **Disk Space**: 50 MB
- **Memory**: 0 MB (minimal)
- **Install Time**: ~30 seconds
- **Dependencies**: None (protected extension)

### Network Domains

- github.com
- api.github.com

### Secrets Required

- `github_token` - GitHub personal access token or OAuth token

## Installation

```bash
sindri extension install github-cli
```

## Configuration

### Install Method

Uses a custom installation script. The extension is marked as **protected** indicating it's a core system component.

### Upgrade Strategy

None - Pre-installed in Docker image via apt.

## Usage Examples

### Authentication

```bash
# Login with browser
gh auth login

# Login with token
gh auth login --with-token < token.txt

# Check auth status
gh auth status

# Logout
gh auth logout
```

### Repository Operations

```bash
# Clone a repository
gh repo clone owner/repo

# Create a new repository
gh repo create my-repo --public

# Fork a repository
gh repo fork owner/repo

# View repository
gh repo view
```

### Pull Requests

```bash
# Create a PR
gh pr create --title "My PR" --body "Description"

# List PRs
gh pr list

# View a PR
gh pr view 123

# Checkout a PR
gh pr checkout 123

# Merge a PR
gh pr merge 123
```

### Issues

```bash
# Create an issue
gh issue create --title "Bug report"

# List issues
gh issue list

# View an issue
gh issue view 123

# Close an issue
gh issue close 123
```

### GitHub Actions

```bash
# List workflow runs
gh run list

# View a run
gh run view 12345

# Watch a run
gh run watch 12345

# Download artifacts
gh run download 12345
```

### Gists

```bash
# Create a gist
gh gist create file.txt

# List gists
gh gist list

# View a gist
gh gist view abc123
```

### API Access

```bash
# Make API requests
gh api repos/owner/repo

# POST to API
gh api repos/owner/repo/issues -f title="Issue" -f body="Body"

# GraphQL queries
gh api graphql -f query='{ viewer { login } }'
```

## Validation

The extension validates the following commands:

- `gh` - Must match pattern `gh version \d+\.\d+\.\d+`

## Removal

```bash
sindri extension remove github-cli
```

This removes the GitHub CLI configuration files:

- ~/.config/gh
- ~/.gh-workflow-helper.sh

## Related Extensions

- [ai-toolkit](AI-TOOLKIT.md) - Depends on github-cli
- [claude-codepro](CLAUDE-CODEPRO.md) - Depends on github-cli
