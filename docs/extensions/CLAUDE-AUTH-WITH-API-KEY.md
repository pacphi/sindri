# Claude Auth with API Key

Claude Code CLI authentication via ANTHROPIC_API_KEY.

## Overview

| Property         | Value  |
| ---------------- | ------ |
| **Category**     | ai     |
| **Version**      | 2.0.0  |
| **Installation** | script |
| **Disk Space**   | 10 MB  |
| **Dependencies** | None   |

## Description

Claude Code CLI authentication via ANTHROPIC_API_KEY - wraps the `claude` command to use API key authentication instead of session-based authentication.

## Installed Tools

| Tool          | Type     | Description                       |
| ------------- | -------- | --------------------------------- |
| `claude-code` | cli-tool | Claude Code CLI (API key wrapper) |

## Secrets Required

| Secret              | Description                          |
| ------------------- | ------------------------------------ |
| `anthropic_api_key` | Anthropic API key for authentication |

## Installation

```bash
extension-manager install claude-auth-with-api-key
```

## Usage

The extension automatically configures Claude Code to use the `ANTHROPIC_API_KEY` environment variable for authentication instead of requiring a browser-based login.

```bash
# Set your API key (or configure via secrets)
export ANTHROPIC_API_KEY="your-api-key"

# Use claude normally
claude "your prompt"
```

## Validation

```bash
claude --version    # Expected: claude
```

## Upgrade

**Strategy:** none

Configuration-only extension.

## Removal

```bash
extension-manager remove claude-auth-with-api-key
```

Removes:

- `/workspace/bin/claude`

## Related Extensions

- [claude-marketplace](CLAUDE-MARKETPLACE.md) - Plugin marketplace
- [claude-code-mux](CLAUDE-CODE-MUX.md) - AI routing proxy
