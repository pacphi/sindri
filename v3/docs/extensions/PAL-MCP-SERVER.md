# PAL MCP Server Extension

> Version: 9.8.2 | Category: mcp | Last Updated: 2026-01-26

## Overview

AI orchestration and multi-model collaboration MCP server with 18 specialized tools. Provides advanced AI coordination capabilities.

## What It Provides

| Tool           | Type   | License | Description                      |
| -------------- | ------ | ------- | -------------------------------- |
| pal-mcp-server | server | MIT     | Multi-model AI orchestration MCP |

## Requirements

- **Disk Space**: 150 MB
- **Memory**: 256 MB
- **Install Time**: ~180 seconds
- **Install Timeout**: 600 seconds
- **Dependencies**: python, mise-config

### Network Domains

- github.com
- raw.githubusercontent.com
- pypi.org
- files.pythonhosted.org

## Installation

```bash
sindri extension install pal-mcp-server
```

## Configuration

### Install Method

Uses a custom installation script with 600 second timeout.

### Upgrade Strategy

Reinstall.

## Key Features

- **18 Specialized Tools** - Comprehensive AI orchestration
- **Multi-model Collaboration** - Coordinate multiple AI models
- **Python-based** - Easy to extend
- **MCP Standard** - Native Claude Code integration

## Available Tools (18)

The PAL MCP server provides specialized tools for:

1. **Orchestration** - Coordinate multiple AI agents
2. **Memory Management** - Persistent context storage
3. **Task Planning** - Break down complex tasks
4. **Code Analysis** - Analyze and understand code
5. **Documentation** - Generate and update docs
6. **Testing** - Create and run tests
7. **Debugging** - Assist with debugging
8. **Refactoring** - Suggest improvements
9. **Search** - Semantic code search
10. **File Operations** - Safe file management
11. **Git Operations** - Version control integration
12. **API Interaction** - External API calls
13. **Data Processing** - Transform data
14. **Validation** - Input/output validation
15. **Logging** - Structured logging
16. **Metrics** - Performance tracking
17. **Caching** - Response caching
18. **Error Handling** - Graceful error management

## Usage Examples

### With Claude Code

PAL MCP server tools are available through Claude Code MCP integration:

```bash
# Claude Code can use PAL tools like:
# - Orchestrate multi-agent workflows
# - Manage persistent memory
# - Plan complex tasks
```

## Validation

Uses a custom validation script:

```bash
scripts/validate.sh
```

## Removal

```bash
sindri extension remove pal-mcp-server
```

**Requires confirmation.**

Runs uninstall script and removes ~/extensions/pal-mcp-server.

**Warning:** This removes the MCP server configuration from ~/.claude/settings.json.

## Related Extensions

- [python](PYTHON.md) - Required dependency
- [mise-config](MISE-CONFIG.md) - Required dependency
