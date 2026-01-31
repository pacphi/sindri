# Claude CLI Extension

> Version: 1.0.0 | Category: claude | Last Updated: 2026-01-30

## Overview

Claude Code CLI - Official Anthropic AI coding assistant. This extension installs the official Claude Code CLI from Anthropic, providing intelligent code assistance, generation, and analysis directly in your terminal.

Many other extensions in the Claude ecosystem depend on this extension being installed first.

## What It Provides

| Tool   | Type     | License     | Description                      |
| ------ | -------- | ----------- | -------------------------------- |
| claude | cli-tool | Proprietary | Claude Code CLI coding assistant |

## Requirements

- **Disk Space**: 200 MB
- **Memory**: 256 MB
- **Install Time**: ~120 seconds
- **Dependencies**: None

### Network Domains

- claude.ai
- api.anthropic.com

## Installation

```bash
sindri extension install claude-cli
```

Or install as part of a profile:

```bash
sindri profile install anthropic-dev
sindri profile install enterprise
```

## Configuration

### Environment Variables

| Variable | Value                  | Description    |
| -------- | ---------------------- | -------------- |
| `PATH`   | $HOME/.local/bin:$PATH | Local binaries |

### Config Files

| File                      | Description             |
| ------------------------- | ----------------------- |
| `~/.claude/settings.json` | Default Claude settings |

### Install Method

Uses script-based installation.

### Upgrade Strategy

Reinstall.

## Usage Examples

### Basic Usage

```bash
# Check version
claude --version

# Start interactive session
claude

# Run with a prompt
claude "explain this code"
```

### Authentication

```bash
# Login via browser
claude login

# Or set API key directly
export ANTHROPIC_API_KEY="your-api-key"
```

### Project Context

```bash
# Create project context file
echo "# Project Overview" > CLAUDE.md

# Claude will automatically read CLAUDE.md for context
claude "what does this project do?"
```

## Validation

The extension validates the following commands:

- `claude` - Must match pattern `\d+\.\d+\.\d+.*Claude Code`

## Dependent Extensions

The following extensions require `claude-cli` to be installed:

### Authentication Validators

- [claude-flow-v3](CLAUDE-FLOW-V3.md) - Next-gen multi-agent orchestration
- [claude-flow-v2](CLAUDE-FLOW-V2.md) - AI-powered multi-agent orchestration (stable)
- agentic-flow - Multi-model AI agent framework
- agentic-qe - AI-powered quality engineering
- ralph - Autonomous development system

### Install Validators

- jira-mcp - Atlassian MCP server integration
- linear-mcp - Linear MCP server integration
- context7-mcp - Context7 documentation MCP server
- [claude-marketplace](CLAUDE-MARKETPLACE.md) - Claude Code plugin marketplace

## Removal

```bash
sindri extension remove claude-cli
```

**Requires confirmation.** Removes:

- ~/.local/bin/claude
- ~/.claude

**Warning:** Removing claude-cli will break extensions that depend on it.

## Related Extensions

- [claude-flow-v3](CLAUDE-FLOW-V3.md) - Multi-agent orchestration
- [claude-flow-v2](CLAUDE-FLOW-V2.md) - Multi-agent orchestration (stable)
- [claude-codepro](CLAUDE-CODEPRO.md) - TDD-enforced development
- [claude-code-mux](CLAUDE-CODE-MUX.md) - AI routing proxy
- [claudeup](CLAUDEUP.md) - TUI manager for Claude Code
- [claude-marketplace](CLAUDE-MARKETPLACE.md) - Plugin marketplace

## Links

- [Claude Code Documentation](https://claude.ai/code)
- [Anthropic](https://anthropic.com)
- [Claude Code GitHub Issues](https://github.com/anthropics/claude-code/issues)
