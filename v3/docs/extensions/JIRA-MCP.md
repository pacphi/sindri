# Jira MCP Extension

> Version: 2.0.0 | Category: mcp | Last Updated: 2026-01-26

## Overview

Atlassian MCP server using Claude Code's native SSE transport. No API key required - uses OAuth flow via Claude Code. Provides access to Jira and Confluence.

## What It Provides

| Tool          | Type   | License | Description          |
| ------------- | ------ | ------- | -------------------- |
| atlassian-mcp | server | MIT     | Atlassian MCP server |

## Requirements

- **Disk Space**: 10 MB
- **Memory**: 64 MB
- **Install Time**: ~10 seconds
- **Dependencies**: None (no Docker, no API key)

### Network Domains

- atlassian.net
- atlassian.com
- mcp.atlassian.com

## Installation

```bash
sindri extension install jira-mcp
```

## Configuration

### Templates

- resources/SKILL.md - Claude Code skill at ~/extensions/jira-mcp/SKILL.md

### Install Method

Uses a custom installation script with 60 second timeout.

### Upgrade Strategy

Reinstall.

## Key Features

- **No API Key Required** - Uses OAuth flow via Claude Code
- **Native SSE Transport** - Direct integration with Claude Code
- **Jira Access** - Create, update, and manage issues
- **Confluence Access** - Read and create documentation
- **No Docker** - Lightweight installation

## Usage Examples

### With Claude Code

Once registered, Claude Code can interact with your Atlassian workspace:

```bash
# Example queries:
# "Create a Jira issue for the authentication bug"
# "What are my assigned Jira tickets?"
# "Update PROJ-123 status to In Progress"
# "Find Confluence pages about deployment"
```

### OAuth Authentication

When first using Jira features, Claude Code will prompt for OAuth authentication with Atlassian. This is a one-time setup per workspace.

### Issue Management

```bash
# Through Claude Code:
# - Create issues
# - Update status
# - Add comments
# - Assign tickets
# - Search issues
```

### Confluence Integration

```bash
# Through Claude Code:
# - Search documentation
# - Create pages
# - Update content
# - Read spaces
```

## Validation

The extension validates the following commands:

- `claude --version` - Verifies Claude CLI is available

## Removal

```bash
sindri extension remove jira-mcp
```

Runs uninstall script and removes ~/extensions/jira-mcp.

## Related Extensions

- [linear-mcp](LINEAR-MCP.md) - Alternative issue tracking (Linear)
