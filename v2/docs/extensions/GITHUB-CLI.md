# GitHub CLI

GitHub CLI authentication and workflow configuration.

## Overview

| Property         | Value     |
| ---------------- | --------- |
| **Category**     | dev-tools |
| **Version**      | 2.0.0     |
| **Installation** | script    |
| **Disk Space**   | 50 MB     |
| **Dependencies** | None      |

## Description

GitHub CLI authentication and workflow configuration - provides the `gh` command for GitHub repository management, pull requests, issues, and more.

## Installed Tools

| Tool | Type     | Description |
| ---- | -------- | ----------- |
| `gh` | cli-tool | GitHub CLI  |

## Secrets Required

| Secret         | Description                  |
| -------------- | ---------------------------- |
| `github_token` | GitHub personal access token |

## Installation

```bash
extension-manager install github-cli
```

## Usage

```bash
# Authenticate
gh auth login

# Repository operations
gh repo create
gh repo clone owner/repo

# Pull requests
gh pr create
gh pr list
gh pr checkout 123

# Issues
gh issue create
gh issue list
```

## Validation

```bash
gh --version    # Expected: gh version X.X.X
```

## Upgrade

**Strategy:** none

Pre-installed in Docker image, upgraded with image rebuilds.

## Removal

```bash
extension-manager remove github-cli
```

Removes:

- `~/.config/gh`
- `~/.gh-workflow-helper.sh`
