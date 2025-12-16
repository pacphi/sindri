# Atlassian MCP Server

This extension provides integration with Atlassian Jira and Confluence via the official Atlassian Remote MCP server with OAuth authentication.

## Features

### Jira Operations

- Search issues using JQL queries
- Create and update issues
- Manage issue transitions and workflows
- Add comments to issues
- Query project and sprint information
- Real-time access to issue tracking data

### Confluence Operations

- Search pages and spaces
- Create and update pages
- Navigate documentation hierarchy
- Summarize existing documentation

### Key Benefits

- **No API key required** - uses secure OAuth 2.1 flow
- **No Docker dependencies** - native SSE transport
- **Real-time access** - direct connection to Atlassian Cloud

## Authentication

This extension uses Atlassian's official OAuth-based MCP server. On first use:

1. Run `/mcp` in Claude Code
2. Click "Connect Atlassian Account"
3. Authorize the connection in your browser
4. Grant access to Jira and/or Confluence
5. Start using Atlassian tools immediately

The OAuth token is stored securely by Claude Code.

## Configuration

The Atlassian MCP server is automatically added to your user-scope configuration at `~/.claude.json`:

```json
{
  "mcpServers": {
    "atlassian": {
      "type": "sse",
      "url": "https://mcp.atlassian.com/v1/sse"
    }
  }
}
```

### Manual Installation

If automatic installation fails, add manually:

```bash
claude mcp add --transport sse --scope user atlassian https://mcp.atlassian.com/v1/sse
```

## Usage

Once authenticated, you can ask Claude to:

### Jira Examples

- "Search for open bugs in project BACKEND"
- "Create a new story for user authentication"
- "What issues are assigned to me?"
- "Update PROJ-123 to In Progress"
- "Show me the sprint backlog"
- "Add a comment to issue ABC-456"

### Confluence Examples

- "Find Confluence pages about API documentation"
- "Create a new Confluence page with meeting notes"
- "Summarize the architecture documentation"
- "What pages are in the DEV space?"

## Available Tools

The MCP server exposes these tools:

### Jira Tools

- `jira_search` - Search issues using JQL
- `jira_get_issue` - Get issue details
- `jira_create_issue` - Create new issue
- `jira_update_issue` - Update existing issue
- `jira_add_comment` - Add comment to issue
- `jira_transition_issue` - Change issue status
- `jira_get_projects` - List projects

### Confluence Tools

- `confluence_search` - Search pages
- `confluence_get_page` - Get page content
- `confluence_create_page` - Create new page
- `confluence_update_page` - Update page content
- `confluence_get_spaces` - List spaces

## Troubleshooting

### Re-authenticate

If you need to re-authenticate:

1. Run `/mcp` in Claude Code
2. Find the Atlassian server
3. Click "Authenticate" or "Reconnect"

### Check Status

```bash
claude mcp list --scope user
```

### Remove and Reinstall

```bash
claude mcp remove --scope user atlassian
extension-manager reinstall jira-mcp
```

### Known Issues

There's a [reported issue](https://github.com/anthropics/claude-code/issues/9133) where tools may not appear in conversations despite successful connection. Try:

1. Starting a new conversation
2. Running `/mcp` to refresh connections
3. Updating to the latest Claude Code version

## Rate Limits

The Atlassian Remote MCP server has usage limits based on your Atlassian plan:

- **Standard plan**: Moderate usage thresholds
- **Premium/Enterprise**: Higher quotas (1,000 requests/hour plus per-user limits)

## Links

- [Atlassian Remote MCP Server](https://support.atlassian.com/atlassian-rovo-mcp-server/)
- [Getting Started Guide](https://support.atlassian.com/atlassian-rovo-mcp-server/docs/getting-started-with-the-atlassian-remote-mcp-server/)
- [Setting up Claude.ai](https://support.atlassian.com/atlassian-rovo-mcp-server/docs/setting-up-claude-ai/)
- [MCP Protocol](https://modelcontextprotocol.io/)
