# Linear MCP Server

This extension provides integration with Linear's project management system via the Model Context Protocol (MCP).

## Features

- Query Linear issues, projects, and teams
- Create and update issues
- Manage issue status, priority, and labels
- Search across your Linear workspace
- Real-time access to project management data

## Configuration

### Required Environment Variable

Set your Linear API key:

```bash
export LINEAR_API_KEY="your_linear_api_key_here"
```

Get your API key from: `https://linear.app/YOUR-TEAM/settings/api`

### Claude Code Configuration

Add to your Claude Code MCP configuration (`~/.claude/settings.json`):

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

## Usage

Once configured, you can ask Claude to:

- "List my Linear issues"
- "Create a new issue in the Backend project"
- "Update issue ABC-123 to In Progress"
- "Search for issues about authentication"
- "Show me issues assigned to me"

## Available Tools

The MCP server exposes these tools:

- `search_issues` - Search for issues by query
- `get_issue` - Get details of a specific issue
- `create_issue` - Create a new issue
- `update_issue` - Update an existing issue
- `list_teams` - List all teams
- `list_projects` - List projects in a team
- `get_viewer` - Get current user info

## Links

- [Linear MCP Server](https://github.com/jerhadf/linear-mcp-server)
- [Linear API Documentation](https://developers.linear.app/docs)
- [MCP Protocol](https://modelcontextprotocol.io/)
