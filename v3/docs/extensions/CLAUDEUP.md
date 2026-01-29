# ClaudeUp Extension

> Version: 1.0.0 | Category: claude | Last Updated: 2026-01-26

## Overview

TUI tool for managing Claude Code plugins, MCPs (Model Context Protocol), and configuration settings. Provides an interactive terminal interface for Claude Code management.

## What It Provides

| Tool     | Type     | License | Description             |
| -------- | -------- | ------- | ----------------------- |
| claudeup | cli-tool | MIT     | Claude Code TUI manager |

## Requirements

- **Disk Space**: 50 MB
- **Memory**: 128 MB
- **Install Time**: ~30 seconds
- **Dependencies**: nodejs

### Network Domains

- registry.npmjs.org
- github.com

## Installation

```bash
extension-manager install claudeup
```

## Configuration

### Environment Variables

| Variable | Value                  | Description    |
| -------- | ---------------------- | -------------- |
| `PATH`   | $HOME/.local/bin:$PATH | Local binaries |

### Install Method

Uses mise for tool management.

### Upgrade Strategy

Reinstall.

## Usage Examples

### Launch TUI

```bash
# Start the TUI
claudeup

# The TUI provides:
# - Plugin management
# - MCP server configuration
# - Settings editor
```

### Plugin Management

```bash
# Through TUI:
# 1. Navigate to Plugins section
# 2. Browse available plugins
# 3. Install/uninstall plugins
# 4. Configure plugin settings
```

### MCP Configuration

```bash
# Through TUI:
# 1. Navigate to MCPs section
# 2. View registered MCP servers
# 3. Add/remove MCP servers
# 4. Test MCP connections
```

### Settings Management

```bash
# Through TUI:
# 1. Navigate to Settings section
# 2. Edit Claude Code configuration
# 3. Manage environment variables
# 4. Configure preferences
```

## Validation

The extension validates the following commands:

- `claudeup` - Must match pattern `\d+\.\d+\.\d+`

## Removal

```bash
extension-manager remove claudeup
```

**Requires confirmation.** Removes:

- mise claudeup tools
- ~/.config/claudeup

## Related Extensions

- [nodejs](NODEJS.md) - Required dependency
- [claude-marketplace](CLAUDE-MARKETPLACE.md) - Plugin sources
