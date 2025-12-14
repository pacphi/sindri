# Linear MCP

Linear MCP server for AI-powered project management integration.

## Overview

| Property         | Value  |
| ---------------- | ------ |
| **Category**     | agile  |
| **Version**      | 1.0.0  |
| **Installation** | script |
| **Disk Space**   | 150 MB |
| **Dependencies** | nodejs |

## Description

This extension provides integration with Linear's project management system via the Model Context Protocol (MCP), using the [linear-mcp-server](https://github.com/jerhadf/linear-mcp-server) npm package.

## Installed Tools

| Tool                | Type   | Description             |
| ------------------- | ------ | ----------------------- |
| `linear-mcp-server` | server | Linear MCP server (npm) |

## Secrets Required

| Secret           | Description                       |
| ---------------- | --------------------------------- |
| `LINEAR_API_KEY` | Linear API key for authentication |

### Getting Your API Key

1. Go to [Linear API Settings](https://linear.app/settings/api) (replace with your team URL)
2. Under **Personal API keys**, click **Create key**
3. Give it a label and copy the key

### sindri.yaml Configuration

```yaml
secrets:
  - name: LINEAR_API_KEY
    source: env
```

## Installation

```bash
extension-manager install linear-mcp
```

## Features

- Query Linear issues, projects, and teams
- Create and update issues
- Manage issue status, priority, and labels
- Search across your Linear workspace
- Real-time access to project management data

## Usage

Once configured, you can ask Claude to:

- "List my Linear issues"
- "Create a new issue in the Backend project"
- "Update issue ABC-123 to In Progress"
- "Search for issues about authentication"
- "Show me issues assigned to me"

## Claude Code MCP Configuration

Add to `~/.claude/settings.json`:

```json
{
  "mcpServers": {
    "linear": {
      "command": "npx",
      "args": ["-y", "linear-mcp-server"],
      "env": {
        "LINEAR_API_KEY": "${LINEAR_API_KEY}"
      }
    }
  }
}
```

Or use the Claude CLI:

```bash
claude mcp add linear -- npx -y linear-mcp-server
```

## Available Tools

The MCP server exposes these tools:

| Tool            | Description                     |
| --------------- | ------------------------------- |
| `search_issues` | Search for issues by query      |
| `get_issue`     | Get details of a specific issue |
| `create_issue`  | Create a new issue              |
| `update_issue`  | Update an existing issue        |
| `list_teams`    | List all teams                  |
| `list_projects` | List projects in a team         |
| `get_viewer`    | Get current user info           |

## Validation

```bash
test -d ~/extensions/linear-mcp
```

## Removal

```bash
extension-manager remove linear-mcp
```

## Links

- [Linear MCP Server](https://github.com/jerhadf/linear-mcp-server)
- [Linear API Documentation](https://developers.linear.app/docs)
- [MCP Protocol](https://modelcontextprotocol.io/)

## Related Extensions

- [jira-mcp](JIRA-MCP.md) - Atlassian Jira/Confluence integration
- [supabase-cli](SUPABASE-CLI.md) - Supabase database backend
