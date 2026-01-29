# Context7 MCP Extension

> Version: 1.0.0 | Category: mcp | Last Updated: 2026-01-26

## Overview

Context7 MCP server for up-to-date, version-specific library documentation. Provides real-time access to documentation for programming libraries and frameworks.

## What It Provides

| Tool         | Type   | License | Description              |
| ------------ | ------ | ------- | ------------------------ |
| context7-mcp | server | MIT     | Documentation MCP server |

## Requirements

- **Disk Space**: 10 MB
- **Memory**: 64 MB
- **Install Time**: ~10 seconds
- **Dependencies**: None

### Network Domains

- mcp.context7.com
- context7.com

### Secrets (Optional)

- `context7_api_key` - Enables higher rate limits and private repo access

## Installation

```bash
extension-manager install context7-mcp
```

## Configuration

### Templates

- resources/SKILL.md - Claude Code skill at ~/extensions/context7-mcp/SKILL.md

### Install Method

Uses a custom installation script with 60 second timeout.

### Upgrade Strategy

Reinstall.

## Key Features

- **Real-time Docs** - Always up-to-date documentation
- **Version-specific** - Documentation for specific library versions
- **Remote Server** - No local installation required
- **Claude Code Integration** - Native MCP support

## Usage Examples

### With Claude Code

Context7 registers as an MCP server in Claude Code settings. Once registered, Claude Code can automatically fetch documentation for libraries you're working with.

```bash
# Claude Code will use context7 for queries like:
# "How do I use React hooks?"
# "What's the syntax for Express middleware?"
# "Show me the Next.js API routes documentation"
```

### API Key Benefits

With an API key:

- Higher rate limits
- Access to private repository documentation
- Priority support

## Validation

The extension validates the following commands:

- `claude --version` - Verifies Claude CLI is available

## Removal

```bash
extension-manager remove context7-mcp
```

Runs uninstall script and removes ~/extensions/context7-mcp.

## Related Extensions

None - Context7 MCP is a standalone documentation service.
