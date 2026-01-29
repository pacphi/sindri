# Ruvnet Research

AI research tools including Goalie goal management and Research-Swarm multi-agent research framework.

## Overview

| Property         | Value               |
| ---------------- | ------------------- |
| **Category**     | ai                  |
| **Version**      | 1.0.0               |
| **Installation** | mise                |
| **Disk Space**   | 100 MB              |
| **Dependencies** | [nodejs](NODEJS.md) |

## Description

AI research tools including Goalie goal management and Research-Swarm multi-agent research framework - provides AI-powered research and goal tracking capabilities.

## Installed Tools

| Tool             | Type     | Pinned Version | Description                       |
| ---------------- | -------- | -------------- | --------------------------------- |
| `goalie`         | cli-tool | 1.3            | Goal management and tracking      |
| `research-swarm` | cli-tool | 1.2            | Multi-agent AI research framework |

> **Note:** `research-swarm` has native dependencies (`better-sqlite3`) that require node-gyp compilation. Ensure build tools (gcc, python3, make) are available if installation fails.

## Secrets (Optional)

| Secret               | Description                 |
| -------------------- | --------------------------- |
| `perplexity_api_key` | Perplexity API for research |

## Network Requirements

- `registry.npmjs.org` - NPM registry
- `api.perplexity.ai` - Perplexity API

## Installation

```bash
extension-manager install ruvnet-research
```

## Usage

### Goalie

Goalie helps manage goals and track progress:

```bash
goalie --help
```

### Research-Swarm

Research-Swarm enables multi-agent AI research:

```bash
research-swarm --help
```

## Validation

```bash
goalie --version
research-swarm --version
```

## Upgrade

**Strategy:** automatic

Automatically upgrades all mise-managed npm tools.

## Removal

```bash
extension-manager remove ruvnet-research
```

## Source Projects

- [goalie](https://github.com/cmurczek/goalie)
- [research-swarm](https://github.com/ruvnet/research-swarm)

## Related Extensions

- [ai-toolkit](AI-TOOLKIT.md) - AI tools suite
- [nodejs-devtools](NODEJS-DEVTOOLS.md) - Node.js development tools
