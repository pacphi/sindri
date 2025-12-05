# Claudeup

TUI tool for managing Claude Code plugins, MCPs, and configuration.

## Overview

| Property         | Value               |
| ---------------- | ------------------- |
| **Category**     | ai                  |
| **Version**      | 1.0.0               |
| **Installation** | script              |
| **Disk Space**   | 50 MB               |
| **Dependencies** | [nodejs](NODEJS.md) |

## Description

Claudeup is a terminal user interface (TUI) tool that streamlines Claude Code setup and management. It provides an interactive interface for configuring MCP servers, managing plugins from multiple marketplaces, and customizing your Claude Code status line.

## Features

- **MCP Server Configuration** - Set up Model Context Protocol servers across categories: file systems, databases, developer tools (GitHub, GitLab), APIs, web services, productivity apps, and AI capabilities
- **Plugin Marketplaces** - Access Anthropic Official and MadAppGang community marketplaces
- **Plugin Management** - Install and configure plugins for frontend development, code analysis, backend services, and agent development
- **Status Line Customization** - Configure Claude Code status line with presets (minimal, standard, detailed, git-aware, token-focused)

## Installed Tools

| Tool       | Type     | Description                       |
| ---------- | -------- | --------------------------------- |
| `claudeup` | cli-tool | TUI for Claude Code configuration |

## Configuration

### Environment Variables

| Variable | Value                    | Scope  |
| -------- | ------------------------ | ------ |
| `PATH`   | `$HOME/.local/bin:$PATH` | bashrc |

### Configuration Files

Claudeup modifies the following Claude Code configuration files:

- `.claude/settings.json` - Shared settings for plugins and marketplaces
- `.claude/settings.local.json` - Local MCP server configurations

## Network Requirements

- `registry.npmjs.org` - NPM registry
- `github.com` - GitHub access for repositories

## Installation

```bash
extension-manager install claudeup
```

Or install directly via npm:

```bash
npm install -g claudeup
```

## Usage

```bash
# Launch the TUI
claudeup

# Alternative methods
npx claudeup
pnpx claudeup
```

### Navigation

| Key             | Action            |
| --------------- | ----------------- |
| Arrow keys / jk | Movement          |
| Enter           | Selection         |
| Escape / q      | Navigation/Exit   |
| ?               | Help              |
| 1-4             | Quick screen jump |

### Main Features

1. **MCP Servers** - Configure Model Context Protocol servers
2. **Marketplaces** - Add plugin marketplaces
3. **Plugins** - Install and manage Claude Code plugins
4. **Status Line** - Customize the Claude Code status line display

## Validation

```bash
claudeup --version    # Expected: X.X.X
```

## Upgrade

**Strategy:** automatic

```bash
extension-manager upgrade claudeup
```

Or via npm:

```bash
npm update -g claudeup
```

## Removal

Requires confirmation before removal.

```bash
extension-manager remove claudeup
```

Removes:

- `~/.config/claudeup`
- Global npm package `claudeup`

## Related Extensions

- [openskills](OPENSKILLS.md) - Claude Code skills manager
- [claude-marketplace](CLAUDE-MARKETPLACE.md) - Plugin marketplace configuration
- [claude-code-mux](CLAUDE-CODE-MUX.md) - AI routing proxy

## Links

- [GitHub Repository](https://github.com/MadAppGang/claude-code/tree/main/tools/claudeup)
- [npm Package](https://www.npmjs.com/package/claudeup)
