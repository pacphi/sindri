# Context7 MCP Server

This extension provides integration with Context7 MCP server for up-to-date, version-specific library documentation.

## Features

- Resolve library identifiers for accurate documentation lookup
- Get current, version-specific documentation and code examples
- Reduce LLM hallucination by using real-time library docs instead of training data
- Support for thousands of libraries across multiple languages
- **Optional API key** for higher rate limits and private repository access

## What is Context7?

Context7 is an MCP server that delivers current documentation directly into LLM prompts, addressing the problem that AI models rely on outdated training data. It prevents hallucinated APIs and enables accurate code generation.

## Authentication

Context7 works **without an API key** (with standard rate limits). For higher limits and private repo access:

1. Get a free API key at [context7.com/dashboard](https://context7.com/dashboard)
2. Add to your `sindri.yaml`:
   ```yaml
   secrets:
     - name: CONTEXT7_API_KEY
       source: env
   ```
3. Set the environment variable or add to `.env` file
4. Reinstall: `extension-manager reinstall context7-mcp`

## Configuration

The Context7 MCP server is automatically added to your user-scope configuration at `~/.claude.json`:

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

**With API key (higher rate limits):**
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

**Without API key:**
```bash
claude mcp add --transport http --scope user context7 https://mcp.context7.com/mcp
```

**With API key:**
```bash
claude mcp add --transport http --scope user --header "CONTEXT7_API_KEY: YOUR_KEY" context7 https://mcp.context7.com/mcp
```

Or using JSON:
```bash
claude mcp add-json --scope user context7 '{"type":"http","url":"https://mcp.context7.com/mcp"}'
```

## Usage

Once installed, ask Claude:

- "What's the latest React documentation for hooks?"
- "Show me how to use pandas 2.0 DataFrame with examples"
- "Get FastAPI authentication documentation"
- "How do I use Next.js 14 app router?"
- "Find documentation for TypeScript 5.3 decorators"

## Available Tools

The MCP server exposes these tools:

| Tool                 | Description                                            |
| -------------------- | ------------------------------------------------------ |
| `resolve-library-id` | Convert general library names to Context7 identifiers |
| `get-library-docs`   | Retrieve version-specific docs with topic filtering   |

### Tool Examples

**resolve-library-id:**
- Input: "react", "pandas", "fastapi"
- Output: Context7-compatible library identifier

**get-library-docs:**
- Parameters: library_id, topic (optional), page (1-10, optional)
- Returns: Up-to-date documentation and code examples

## Rate Limits

| Tier         | Rate Limit | Cost | Use Case                   |
| ------------ | ---------- | ---- | -------------------------- |
| No API key   | Limited    | Free | Personal projects, testing |
| Free API key | Higher     | Free | Development, small teams   |
| Paid tiers   | Highest    | Paid | Production, large teams    |

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
3. Set environment variable: `export CONTEXT7_API_KEY=your_key`
4. Reinstall: `extension-manager reinstall context7-mcp`

### View Configuration

```bash
cat ~/.claude.json | jq '.mcpServers.context7'
```

### Rate Limit Exceeded

If you see rate limit errors without an API key:
1. Get a free API key at [context7.com/dashboard](https://context7.com/dashboard)
2. Follow "Add API Key After Installation" steps above

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

- linear-mcp - Linear project management
- jira-mcp - Atlassian Jira/Confluence integration
- ai-toolkit - AI CLI tools suite
- ollama - Local LLM runtime
