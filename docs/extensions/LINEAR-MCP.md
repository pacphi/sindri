# Linear MCP

Linear MCP server for AI-powered project management integration using OAuth authentication.

## Overview

| Property         | Value                                    |
| ---------------- | ---------------------------------------- |
| **Category**     | agile                                    |
| **Version**      | 2.1.0                                    |
| **Installation** | script                                   |
| **Disk Space**   | 10 MB                                    |
| **Dependencies** | none                                     |
| **Auth Method**  | OAuth (no API key required)              |
| **Transport**    | Native HTTP (no mcp-remote needed)       |

## Description

This extension provides integration with Linear's project management system via the official [Linear Remote MCP server](https://linear.app/docs/mcp). It uses Claude Code's native HTTP transport and OAuth authentication, eliminating the need for API keys or npm wrapper packages.

## Key Changes in v2.1.0

- **Native HTTP Transport**: Uses Claude Code's built-in HTTP transport directly
- **No npm Dependencies**: Removed mcp-remote wrapper - no Node.js required
- **Faster Installation**: Simplified setup with no package downloads
- **OAuth Authentication**: Uses Linear's official OAuth flow via Claude Code

## Installed Components

| Component | Type   | Description                      |
| --------- | ------ | -------------------------------- |
| `linear`  | server | Linear MCP in Claude Code config |

## Installation

```bash
extension-manager install linear-mcp
```

The installer will:
1. Add Linear MCP to your user-scope Claude Code configuration
2. Use native HTTP transport (no npm packages needed)
3. Merge with any existing MCP servers (non-destructive)

## First-Time Setup

After installation, complete the OAuth flow:

1. Open Claude Code
2. Run `/mcp` to see configured servers
3. Click "Authenticate" next to Linear
4. Authorize Linear access in your browser
5. Return to Claude Code - you're ready!

## Features

- Query Linear issues, projects, and teams
- Create and update issues
- Manage issue status, priority, and labels
- Search across your Linear workspace
- Real-time access to project management data

## Usage

Once authenticated, ask Claude:

- "List my Linear issues"
- "Create a new issue in the Backend project"
- "Update issue ABC-123 to In Progress"
- "Search for issues about authentication"
- "Show me issues assigned to me"
- "What's on my current sprint?"

## Configuration

The extension uses `claude mcp add --transport http` with user scope. Configuration is stored in `~/.claude.json`:

```json
{
  "mcpServers": {
    "linear": {
      "type": "http",
      "url": "https://mcp.linear.app/mcp"
    }
  }
}
```

### Manual Installation

If automatic installation fails:

```bash
claude mcp add --transport http --scope user linear https://mcp.linear.app/mcp
```

Or using JSON:

```bash
claude mcp add-json --scope user linear '{"type":"http","url":"https://mcp.linear.app/mcp"}'
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
# Check Claude CLI is available
claude --version

# Verify Linear MCP is configured
claude mcp list --scope user

# Get Linear MCP details
claude mcp get linear
```

## Troubleshooting

### Re-authenticate

```bash
# Run /mcp in Claude Code and click "Authenticate" on Linear
```

### Server Not Responding

```bash
# Check status
claude mcp get linear

# Remove and reinstall
claude mcp remove --scope user linear
extension-manager reinstall linear-mcp
```

### View Configuration

```bash
cat ~/.claude.json | jq '.mcpServers.linear'
```

## Removal

```bash
extension-manager remove linear-mcp
```

This removes:

- Linear MCP from Claude Code user configuration
- Extension directory at `~/extensions/linear-mcp`

## Migration from v2.0.x

If upgrading from the mcp-remote based version:

1. Run: `extension-manager reinstall linear-mcp`
2. The new version uses native HTTP transport (no npm packages needed)
3. Complete OAuth flow in Claude Code

## Migration from v1.x (API Key)

If upgrading from the API-key based version:

1. Remove old API key from environment: `unset LINEAR_API_KEY`
2. Remove from secrets in `sindri.yaml` (no longer required)
3. Run: `extension-manager reinstall linear-mcp`
4. Complete OAuth flow in Claude Code

## Links

- [Linear MCP Documentation](https://linear.app/docs/mcp)
- [Linear Changelog - MCP Server](https://linear.app/changelog/2025-05-01-mcp)
- [Claude Code MCP Docs](https://code.claude.com/docs/en/mcp)
- [Remote MCP in Claude Code](https://claude.com/blog/claude-code-remote-mcp)
- [MCP Protocol](https://modelcontextprotocol.io/)

## Related Extensions

- [jira-mcp](JIRA-MCP.md) - Atlassian Jira/Confluence integration
- [supabase-cli](SUPABASE-CLI.md) - Supabase database backend
