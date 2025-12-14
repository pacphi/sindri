# Jira MCP

Atlassian Jira and Confluence MCP server for AI-powered issue tracking.

## Overview

| Property         | Value  |
| ---------------- | ------ |
| **Category**     | agile  |
| **Version**      | 1.0.0  |
| **Installation** | script |
| **Disk Space**   | 500 MB |
| **Dependencies** | docker |

## Description

This extension provides integration with Atlassian Jira and Confluence via the Model Context Protocol (MCP), using the community [mcp-atlassian](https://github.com/sooperset/mcp-atlassian) Docker image.

## Installed Tools

| Tool            | Type   | Description                         |
| --------------- | ------ | ----------------------------------- |
| `mcp-atlassian` | server | Jira/Confluence MCP server (Docker) |

## Secrets Required

| Secret           | Description                                                     |
| ---------------- | --------------------------------------------------------------- |
| `JIRA_URL`       | Atlassian Jira base URL (e.g., `https://company.atlassian.net`) |
| `JIRA_USERNAME`  | Atlassian account email                                         |
| `JIRA_API_TOKEN` | Atlassian API token                                             |

### Getting Your API Token

1. Go to [Atlassian API Tokens](https://id.atlassian.com/manage-profile/security/api-tokens)
2. Click **Create API token**
3. Give it a label and copy the token

### sindri.yaml Configuration

```yaml
secrets:
  - name: JIRA_URL
    source: env
  - name: JIRA_USERNAME
    source: env
  - name: JIRA_API_TOKEN
    source: env
```

## Installation

```bash
extension-manager install jira-mcp
```

## Features

### Jira Operations

- Search issues using JQL queries
- Create, update, and delete issues
- Manage issue transitions and workflows
- Add comments and attachments
- Manage sprints and boards

### Confluence Operations

- Search pages and spaces
- Create and update pages
- Manage page comments and labels

## Usage

Once configured, you can ask Claude to:

- "Search for open bugs in project BACKEND"
- "Create a new story for user authentication"
- "What issues are assigned to me?"
- "Update PROJ-123 to In Progress"
- "Find Confluence pages about API documentation"

## Claude Code MCP Configuration

Add to `~/.claude/settings.json`:

```json
{
  "mcpServers": {
    "atlassian": {
      "command": "docker",
      "args": [
        "run",
        "-i",
        "--rm",
        "-e",
        "JIRA_URL",
        "-e",
        "JIRA_USERNAME",
        "-e",
        "JIRA_API_TOKEN",
        "ghcr.io/sooperset/mcp-atlassian:latest"
      ],
      "env": {
        "JIRA_URL": "${JIRA_URL}",
        "JIRA_USERNAME": "${JIRA_USERNAME}",
        "JIRA_API_TOKEN": "${JIRA_API_TOKEN}"
      }
    }
  }
}
```

## Alternative: Official Atlassian Remote MCP

For OAuth 2.1 authentication without API tokens:

```bash
claude mcp add --transport sse atlassian https://mcp.atlassian.com/v1/sse
```

## Validation

```bash
docker image inspect ghcr.io/sooperset/mcp-atlassian:latest --format '{{.Id}}'
```

## Removal

```bash
extension-manager remove jira-mcp
```

## Links

- [mcp-atlassian (GitHub)](https://github.com/sooperset/mcp-atlassian)
- [Official Atlassian MCP Server](https://support.atlassian.com/atlassian-rovo-mcp-server/)
- [Atlassian API Tokens](https://id.atlassian.com/manage-profile/security/api-tokens)

## Related Extensions

- [linear-mcp](LINEAR-MCP.md) - Linear project management
- [supabase-cli](SUPABASE-CLI.md) - Supabase database backend
