# Linear MCP Server

This extension provides integration with Linear's project management system via the official Linear Remote MCP server with OAuth authentication.

## Features

- Query Linear issues, projects, and teams
- Create and update issues
- Manage issue status, priority, and labels
- Search across your Linear workspace
- Real-time access to project management data
- **No API key required** - uses secure OAuth flow

## Authentication

This extension uses Linear's official OAuth-based MCP server. On first use:

1. Run `/mcp` in Claude Code
2. Click to authenticate with Linear
3. Authorize the connection in your browser
4. Start using Linear tools immediately

The OAuth token is stored securely by Claude Code.

## Configuration

The Linear MCP server is automatically added to your user-scope configuration at `~/.claude.json`:

```json
{
  "mcpServers": {
    "linear": {
      "command": "npx",
      "args": ["-y", "mcp-remote", "https://mcp.linear.app/sse"]
    }
  }
}
```

### Manual Installation

If automatic installation fails, add manually:

```bash
claude mcp add-json --scope user linear '{"command":"npx","args":["-y","mcp-remote","https://mcp.linear.app/sse"]}'
```

## Usage

Once authenticated, you can ask Claude to:

- "List my Linear issues"
- "Create a new issue in the Backend project"
- "Update issue ABC-123 to In Progress"
- "Search for issues about authentication"
- "Show me issues assigned to me"
- "What's the status of my current sprint?"

## Available Tools

The MCP server exposes these tools:

- `search_issues` - Search for issues by query
- `get_issue` - Get details of a specific issue
- `create_issue` - Create a new issue
- `update_issue` - Update an existing issue
- `list_teams` - List all teams
- `list_projects` - List projects in a team
- `get_viewer` - Get current user info

## Troubleshooting

### Re-authenticate

If you need to re-authenticate:
1. Run `/mcp` in Claude Code
2. Find the Linear server
3. Click "Authenticate" or "Reconnect"

### Check Status

```bash
claude mcp list --scope user
```

### Remove and Reinstall

```bash
claude mcp remove --scope user linear
extension-manager reinstall linear-mcp
```

## Links

- [Linear MCP Documentation](https://linear.app/docs/mcp)
- [Remote MCP in Claude Code](https://claude.com/blog/claude-code-remote-mcp)
- [MCP Protocol](https://modelcontextprotocol.io/)
