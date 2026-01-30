# Agent Manager Extension

> Version: 2.0.0 | Category: ai-agents | Last Updated: 2026-01-26

## Overview

Claude Code agent manager for managing AI agents. Provides centralized management and discovery of AI agents.

## What It Provides

| Tool          | Type     | License | Description             |
| ------------- | -------- | ------- | ----------------------- |
| agent-manager | cli-tool | MIT     | AI agent management CLI |

## Requirements

- **Disk Space**: 100 MB
- **Memory**: 0 MB (minimal)
- **Install Time**: ~30 seconds
- **Dependencies**: None

### Network Domains

- github.com
- api.github.com
- raw.githubusercontent.com

## Installation

```bash
sindri extension install agent-manager
```

## Configuration

### Environment Variables

| Variable | Value                  | Description    |
| -------- | ---------------------- | -------------- |
| `PATH`   | $HOME/.local/bin:$PATH | Local binaries |

### Templates

- agents-config.yaml.template - Configuration at ~/config/agents-config.yaml (skip if exists)

### Install Method

Uses a custom installation script with 300 second timeout.

### Upgrade Strategy

Automatic - run upgrade.sh script.

## Usage Examples

### Basic Commands

```bash
# Check version
agent-manager version

# List agents
agent-manager list

# Get agent status
agent-manager status agent-id
```

### Agent Discovery

```bash
# Discover available agents
agent-manager discover

# Register an agent
agent-manager register --name "my-agent" --type coder

# Unregister an agent
agent-manager unregister agent-id
```

### Configuration

```yaml
# ~/config/agents-config.yaml
agents:
  - name: coder
    type: development
    capabilities:
      - code-generation
      - refactoring
  - name: researcher
    type: analysis
    capabilities:
      - documentation
      - research
```

### Agent Operations

```bash
# Start an agent
agent-manager start agent-id

# Stop an agent
agent-manager stop agent-id

# Restart an agent
agent-manager restart agent-id
```

## Validation

The extension validates the following commands:

- `agent-manager version` - Must match pattern `version`

## Removal

```bash
sindri extension remove agent-manager
```

**Requires confirmation.** Removes:

- ~/.local/bin/agent-manager
- ~/config/agents-config.yaml
- ~/scripts/agent-discovery.sh

## Related Extensions

None - Agent Manager is a standalone tool.
