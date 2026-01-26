# Claude Flow V2 Extension

> Version: 2.7.47 | Category: claude | Last Updated: 2026-01-26

## Overview

AI-powered multi-agent orchestration system for Claude Code workflows (v2 stable). The proven, stable version of Claude Flow for production use.

## What It Provides

| Tool        | Type     | License | Description                   |
| ----------- | -------- | ------- | ----------------------------- |
| claude-flow | cli-tool | MIT     | Multi-agent orchestration CLI |

## Requirements

- **Disk Space**: 100 MB
- **Memory**: 128 MB
- **Install Time**: ~60 seconds
- **Dependencies**: nodejs

### Network Domains

- registry.npmjs.org
- github.com

## Installation

```bash
extension-manager install claude-flow-v2
```

## Configuration

### Environment Variables

| Variable | Value                  | Description    |
| -------- | ---------------------- | -------------- |
| `PATH`   | $HOME/.local/bin:$PATH | Local binaries |

### Templates

- claude-flow.aliases - Shell aliases
- commands/ms.md - MetaSaver intelligent command router

### Install Method

Uses mise for tool management.

### Upgrade Strategy

Reinstall.

## Usage Examples

### Initialization

```bash
# Initialize Claude Flow
claude-flow init --force

# Check version
claude-flow --version
```

### Agent Operations

```bash
# Spawn an agent
claude-flow agent spawn --type researcher

# List active agents
claude-flow agent list

# Terminate an agent
claude-flow agent terminate --id agent-123
```

### Memory Management

```bash
# Store in memory
claude-flow memory store --key "context" --value "project info"

# Retrieve from memory
claude-flow memory get --key "context"

# Search memory
claude-flow memory search --query "implementation"
```

### Swarm Coordination

```bash
# Start swarm
claude-flow swarm start

# Coordinate task
claude-flow swarm coordinate --task "analyze codebase"

# Status
claude-flow swarm status
```

## Capabilities

### Project Initialization (Priority 20)

Automatically initializes Claude Flow with:

- Memory and context setup
- Optional AgentDB backend initialization

### Authentication

Optional Anthropic API key:

- `ANTHROPIC_API_KEY` - Required for API integration features

### State Markers

| Path        | Type      | Description             |
| ----------- | --------- | ----------------------- |
| `.claude`   | directory | Configuration directory |
| `CLAUDE.md` | file      | Project context file    |

### MCP Server

Provides an MCP server with core tools:

```bash
npx -y @claude-flow/cli@alpha mcp start
```

**Available Tools:**

- `claude-flow-agent-spawn` - Spawn specialized agents
- `claude-flow-memory-store` - Store patterns in memory
- `claude-flow-swarm-coordinate` - Coordinate swarms

## Collision Handling

V2 includes collision detection for project safety:

- **Same version**: Skips if V2 already present
- **V3 detected**: Stops to prevent downgrade
- **Unknown origin**: Stops with manual resolution options

### Version Markers

- `.claude/memory.db` - V2 marker
- `.claude/memory` - V2 marker (directory)
- `.claude/config.json` with swarm/sona - V3 marker

## Validation

The extension validates the following commands:

- `claude-flow` - Must match pattern `claude-flow v\d+\.\d+\.\d+(-[a-z]+(\. \d+)?)?`

## Removal

```bash
extension-manager remove claude-flow-v2
```

**Requires confirmation.** Removes:

- mise claude-flow tools
- ~/.claude/commands/ms.md

## Related Extensions

- [claude-flow-v3](CLAUDE-FLOW-V3.md) - Next-gen V3 version
- [nodejs](NODEJS.md) - Required dependency
