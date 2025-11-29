# Agent Manager

Claude Code agent manager for managing AI agents.

## Overview

| Property         | Value  |
| ---------------- | ------ |
| **Category**     | ai     |
| **Version**      | 2.0.0  |
| **Installation** | script |
| **Disk Space**   | 100 MB |
| **Dependencies** | None   |

## Description

Claude Code agent manager for managing AI agents - provides agent search, discovery, and execution tracking capabilities.

## Installed Tools

| Tool            | Type     | Description          |
| --------------- | -------- | -------------------- |
| `agent-manager` | cli-tool | Agent management CLI |

## Configuration

### Environment Variables

| Variable | Value                    | Scope  |
| -------- | ------------------------ | ------ |
| `PATH`   | `$HOME/.local/bin:$PATH` | bashrc |

### Templates

| Template                      | Destination                             | Mode           | Description         |
| ----------------------------- | --------------------------------------- | -------------- | ------------------- |
| `agents-config.yaml.template` | `/workspace/config/agents-config.yaml`  | skip-if-exists | Agent configuration |
| `agent-discovery.sh.template` | `/workspace/scripts/agent-discovery.sh` | overwrite      | Discovery script    |

## Network Requirements

- `github.com` - GitHub
- `api.github.com` - GitHub API
- `raw.githubusercontent.com` - Raw content

## Installation

```bash
extension-manager install agent-manager
```

## Usage

```bash
# List available agents
agent-manager list

# Search for agents
agent-manager search "code review"

# Run an agent
agent-manager run agent-name

# Check agent status
agent-manager status
```

## Validation

```bash
agent-manager --version    # Expected: version
```

## Upgrade

**Strategy:** automatic

```bash
extension-manager upgrade agent-manager
```

## Removal

Requires confirmation before removal.

```bash
extension-manager remove agent-manager
```

Removes:

- `~/.local/bin/agent-manager`
- `/workspace/config/agents-config.yaml`
- `/workspace/scripts/agent-discovery.sh`

## Related Extensions

- [ai-toolkit](AI-TOOLKIT.md) - AI tools suite
- [ruvnet-aliases](RUVNET-ALIASES.md) - Flow aliases
