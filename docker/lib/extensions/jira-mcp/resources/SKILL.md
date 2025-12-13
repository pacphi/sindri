# Atlassian Jira/Confluence MCP Server

This extension provides integration with Atlassian Jira and Confluence via the Model Context Protocol (MCP).

## Features

### Jira Operations

- Search issues using JQL queries
- Create, update, and delete issues
- Manage issue transitions and workflows
- Add comments and attachments
- Manage sprints and boards
- Query project metadata

### Confluence Operations

- Search pages and spaces
- Create and update pages
- Manage page comments and labels
- Navigate documentation hierarchy

## Installation Options

### Option 1: Docker-based MCP (Self-hosted)

Uses the community [mcp-atlassian](https://github.com/sooperset/mcp-atlassian) Docker image.

**Requirements:**

- Docker running
- Atlassian API token

**Configuration:**

1. Get your API token from: <https://id.atlassian.com/manage-profile/security/api-tokens>

2. Set environment variables:

```bash
export JIRA_URL="https://your-company.atlassian.net"
export JIRA_USERNAME="your-email@company.com"
export JIRA_API_TOKEN="your_api_token"
```

3. Add to Claude Code MCP configuration (`~/.claude/settings.json`):

```json
{
  "mcpServers": {
    "atlassian": {
      "command": "docker",
      "args": [
        "run", "-i", "--rm",
        "-e", "JIRA_URL",
        "-e", "JIRA_USERNAME",
        "-e", "JIRA_API_TOKEN",
        "ghcr.io/sooperset/mcp-atlassian:latest"
      ],
      "env": {
        "JIRA_URL": "https://your-company.atlassian.net",
        "JIRA_USERNAME": "your-email@company.com",
        "JIRA_API_TOKEN": "your_token"
      }
    }
  }
}
```

### Option 2: Official Atlassian Remote MCP (Cloud)

Uses Atlassian's official hosted MCP server with OAuth 2.1 authentication.

**Requirements:**

- Atlassian Cloud account (Jira/Confluence)
- Claude for Teams or Claude Code

**Setup:**

```bash
claude mcp add --transport sse atlassian https://mcp.atlassian.com/v1/sse
```

This initiates an OAuth flow in your browser - no API tokens needed.

## Usage Examples

Once configured, you can ask Claude to:

- "Search for open bugs in project BACKEND"
- "Create a new story for user authentication"
- "What issues are assigned to me?"
- "Update PROJ-123 to In Progress"
- "Show me the sprint backlog"
- "Find Confluence pages about API documentation"
- "Create a new Confluence page with meeting notes"

## Environment Variables

### Required (Docker method)

| Variable | Description |
| -------- | ----------- |
| `JIRA_URL` | Your Jira instance URL (e.g., `https://company.atlassian.net`) |
| `JIRA_USERNAME` | Your email address |
| `JIRA_API_TOKEN` | API token from Atlassian |

### Optional

| Variable | Description |
| -------- | ----------- |
| `CONFLUENCE_URL` | Confluence instance URL |
| `CONFLUENCE_USERNAME` | Confluence username (usually same as Jira) |
| `CONFLUENCE_API_TOKEN` | Confluence API token |
| `JIRA_PROJECTS_FILTER` | Comma-separated project keys to limit access |
| `CONFLUENCE_SPACES_FILTER` | Comma-separated space keys to limit access |
| `READ_ONLY_MODE` | Set to `true` to disable write operations |

## Available Tools

### Jira Tools

- `jira_search` - Search issues using JQL
- `jira_get_issue` - Get issue details
- `jira_create_issue` - Create new issue
- `jira_update_issue` - Update existing issue
- `jira_delete_issue` - Delete issue
- `jira_add_comment` - Add comment to issue
- `jira_transition_issue` - Change issue status
- `jira_get_transitions` - Get available transitions
- `jira_get_projects` - List projects
- `jira_get_boards` - List boards
- `jira_get_sprints` - List sprints

### Confluence Tools

- `confluence_search` - Search pages
- `confluence_get_page` - Get page content
- `confluence_create_page` - Create new page
- `confluence_update_page` - Update page content
- `confluence_delete_page` - Delete page
- `confluence_get_spaces` - List spaces

## Links

- [mcp-atlassian (GitHub)](https://github.com/sooperset/mcp-atlassian)
- [Official Atlassian MCP Server](https://support.atlassian.com/atlassian-rovo-mcp-server/)
- [Atlassian API Tokens](https://id.atlassian.com/manage-profile/security/api-tokens)
- [Jira REST API](https://developer.atlassian.com/cloud/jira/platform/rest/v3/)
- [Confluence REST API](https://developer.atlassian.com/cloud/confluence/rest/v2/)
