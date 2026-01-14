# Claude Marketplace

Claude Code plugin marketplace integration via YAML configuration.

## Overview

| Property         | Value     |
| ---------------- | --------- |
| **Category**     | dev-tools |
| **Version**      | 2.0.0     |
| **Installation** | script    |
| **Disk Space**   | 50 MB     |
| **Dependencies** | None      |

## Description

Claude Code plugin marketplace integration via YAML configuration - provides plugin sources and default settings for Claude Code.

## Installed Tools

| Tool          | Type     | Description                  |
| ------------- | -------- | ---------------------------- |
| `claude-code` | cli-tool | Claude Code CLI (configured) |

## Configuration

### Templates

| Template                      | Destination                             | Mode      | Description             |
| ----------------------------- | --------------------------------------- | --------- | ----------------------- |
| `marketplaces.yml.example`    | `/workspace/config/marketplaces.yml`    | overwrite | Plugin sources          |
| `marketplaces.ci.yml.example` | `/workspace/config/marketplaces.ci.yml` | overwrite | CI plugin sources       |
| `default-settings.json`       | `~/.claude/settings.json`               | merge     | Default Claude settings |

### Sample Marketplaces Config

```yaml
# marketplaces.yml
marketplaces:
  - name: official
    url: https://marketplace.claude.ai
  - name: community
    url: https://community.plugins.claude.ai
```

## Installation

```bash
extension-manager install claude-marketplace
```

## Validation

```bash
claude --version    # Expected: claude
```

## Upgrade

**Strategy:** none

Configuration-only extension, no upgrades needed.

## Removal

```bash
extension-manager remove claude-marketplace
```

Removes:

- `/workspace/config/marketplaces.yml`
- `/workspace/config/marketplaces.ci.yml`

## Related Extensions

- [openskills](OPENSKILLS.md) - Skills management
