# Context7 MCP

Context7 MCP server for up-to-date, version-specific library documentation.

## Overview

| Property         | Value                                    |
| ---------------- | ---------------------------------------- |
| **Category**     | ai                                       |
| **Version**      | 1.0.0                                    |
| **Installation** | script                                   |
| **Disk Space**   | 10 MB                                    |
| **Dependencies** | none                                     |
| **Auth Method**  | Optional API key (works without)         |
| **Transport**    | Native HTTP (no npm/mcp-remote required) |

## Description

This extension provides integration with the [Context7 MCP server](https://github.com/upstash/context7), which delivers up-to-date, version-specific library documentation directly into Claude prompts. It reduces hallucination by fetching current docs instead of relying on training data.

## Key Features

- **No API Key Required**: Works with standard rate limits out of the box
- **Optional Authentication**: Add API key for higher limits and private repo access
- **Version-Specific Docs**: Get documentation for exact library versions
- **Wide Language Support**: Covers thousands of libraries across multiple ecosystems
- **Native HTTP Transport**: Uses Claude Code's built-in HTTP transport (no npm packages)

## Installed Components

| Component  | Type   | Description                        |
| ---------- | ------ | ---------------------------------- |
| `context7` | server | Context7 MCP in Claude Code config |

## Installation

```bash
extension-manager install context7-mcp
```

The installer will:

1. Add Context7 MCP to your user-scope Claude Code configuration
2. Use native HTTP transport (no packages needed)
3. Configure API key authentication if `CONTEXT7_API_KEY` is set
4. Merge with existing MCP servers (non-destructive)

## Authentication (Optional)

Context7 works without an API key. To enable higher rate limits:

1. Get a free API key at [context7.com/dashboard](https://context7.com/dashboard)
2. Add to `sindri.yaml`:
   ```yaml
   secrets:
     - name: CONTEXT7_API_KEY
       source: env
   ```
3. Set environment variable or add to `.env`:
   ```bash
   export CONTEXT7_API_KEY=your_api_key_here
   ```
4. Reinstall extension:
   ```bash
   extension-manager reinstall context7-mcp
   ```

## Features

- Resolve library identifiers (e.g., "react" → Context7 ID)
- Get current, version-specific documentation
- Filter documentation by topic
- Paginated results for large documentation
- Reduce AI hallucination with real-time docs

## Usage

Once installed, ask Claude:

- "What's the latest React hooks documentation?"
- "Show me pandas 2.0 DataFrame examples"
- "How do I use FastAPI authentication?"
- "Get Next.js 14 app router docs"
- "Find TypeScript 5.3 decorator documentation"

## Configuration

The extension uses `claude mcp add --transport http` with user scope. Configuration is stored in `~/.claude.json`:

**Without API key:**
```json
{
  "mcpServers": {
    "context7": {
      "type": "http",
      "url": "https://mcp.context7.com/mcp"
    }
  }
}
```

**With API key:**
```json
{
  "mcpServers": {
    "context7": {
      "type": "http",
      "url": "https://mcp.context7.com/mcp",
      "headers": {
        "CONTEXT7_API_KEY": "YOUR_API_KEY"
      }
    }
  }
}
```

### Manual Installation

If automatic installation fails:

```bash
# Without API key
claude mcp add --transport http --scope user context7 https://mcp.context7.com/mcp

# With API key
claude mcp add --transport http --scope user --header "CONTEXT7_API_KEY: YOUR_KEY" context7 https://mcp.context7.com/mcp
```

## Available Tools

| Tool                 | Description                                            |
| -------------------- | ------------------------------------------------------ |
| `resolve-library-id` | Convert general library names to Context7 identifiers |
| `get-library-docs`   | Retrieve version-specific docs with topic filtering   |

## Rate Limits

| Tier           | Limits  | Cost | Use Case               |
| -------------- | ------- | ---- | ---------------------- |
| No API key     | Standard| Free | Personal, testing      |
| Free API key   | Higher  | Free | Development, teams     |
| Paid API tiers | Premium | Paid | Production, enterprise |

Visit [context7.com/dashboard](https://context7.com/dashboard) for current rate limit details.

## Validation

```bash
# Check Claude CLI is available
claude --version

# Verify Context7 MCP is configured
claude mcp list --scope user

# Get Context7 MCP details
claude mcp get context7
```

## Troubleshooting

### Server Not Responding

```bash
# Check status
claude mcp get context7

# Remove and reinstall
claude mcp remove --scope user context7
extension-manager reinstall context7-mcp
```

### Add API Key After Installation

1. Get API key from [context7.com/dashboard](https://context7.com/dashboard)
2. Add to `sindri.yaml` secrets section
3. Set environment variable
4. Reinstall: `extension-manager reinstall context7-mcp`

### View Configuration

```bash
cat ~/.claude.json | jq '.mcpServers.context7'
```

## Removal

```bash
extension-manager remove context7-mcp
```

This removes:
- Context7 MCP from Claude Code user configuration
- Extension directory at `~/extensions/context7-mcp`

## Links

- [Context7 GitHub](https://github.com/upstash/context7)
- [Context7 Dashboard](https://context7.com/dashboard)
- [MCP Protocol](https://modelcontextprotocol.io/)
- [Claude Code MCP Docs](https://code.claude.com/docs/en/mcp)

## Related Extensions

- [linear-mcp](LINEAR-MCP.md) - Linear project management
- [jira-mcp](JIRA-MCP.md) - Atlassian Jira/Confluence integration
- [ai-toolkit](AI-TOOLKIT.md) - AI CLI tools suite
- [ollama](OLLAMA.md) - Local LLM runtime
