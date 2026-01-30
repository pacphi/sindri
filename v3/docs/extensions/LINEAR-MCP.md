# Linear MCP Extension

> Version: 2.1.0 | Category: mcp | Last Updated: 2026-01-26

## Overview

Linear MCP server using Claude Code's native HTTP transport. No API key required - uses OAuth flow via Claude Code. Modern issue tracking integration.

## What It Provides

| Tool       | Type   | License | Description       |
| ---------- | ------ | ------- | ----------------- |
| linear-mcp | server | MIT     | Linear MCP server |

## Requirements

- **Disk Space**: 10 MB
- **Memory**: 64 MB
- **Install Time**: ~10 seconds
- **Dependencies**: None (no npm, no API key)

### Network Domains

- api.linear.app
- linear.app
- mcp.linear.app

## Installation

```bash
sindri extension install linear-mcp
```

## Configuration

### Templates

- resources/SKILL.md - Claude Code skill at ~/extensions/linear-mcp/SKILL.md

### Install Method

Uses a custom installation script with 60 second timeout.

### Upgrade Strategy

Reinstall.

## Key Features

- **No API Key Required** - Uses OAuth flow via Claude Code
- **Native HTTP Transport** - Direct integration
- **Issue Management** - Full Linear issue lifecycle
- **Project Tracking** - Access to projects and cycles
- **No Dependencies** - Lightweight installation

## Usage Examples

### With Claude Code

Once registered, Claude Code can interact with Linear:

```bash
# Example queries:
# "Create a Linear issue for the performance bug"
# "What are my Linear tasks this cycle?"
# "Update LIN-123 to Done"
# "Show me the current sprint backlog"
```

### OAuth Authentication

When first using Linear features, Claude Code will prompt for OAuth authentication. This is a one-time setup.

### Issue Management

```bash
# Through Claude Code:
# - Create issues
# - Update status
# - Add comments
# - Assign issues
# - Link issues
# - Set priorities
```

### Project Operations

```bash
# Through Claude Code:
# - View projects
# - Track cycles
# - Manage milestones
# - View roadmaps
```

## Validation

The extension validates the following commands:

- `claude --version` - Verifies Claude CLI is available

## Removal

```bash
sindri extension remove linear-mcp
```

Runs uninstall script and removes ~/extensions/linear-mcp.

## Related Extensions

- [jira-mcp](JIRA-MCP.md) - Alternative issue tracking (Atlassian)
