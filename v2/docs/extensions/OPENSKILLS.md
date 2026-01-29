# OpenSkills

OpenSkills CLI for managing Claude Code skills.

## Overview

| Property         | Value               |
| ---------------- | ------------------- |
| **Category**     | ai                  |
| **Version**      | 2.0.0               |
| **Installation** | script              |
| **Disk Space**   | 100 MB              |
| **Dependencies** | [nodejs](NODEJS.md) |

## Description

OpenSkills CLI for managing Claude Code skills from Anthropic's marketplace - provides skill installation, management, and updates for Claude Code.

## Installed Tools

| Tool         | Type     | Pinned Version | Description                |
| ------------ | -------- | -------------- | -------------------------- |
| `openskills` | cli-tool | 1.3            | Skills CLI for Claude Code |

## Configuration

### Environment Variables

| Variable | Value                    | Scope  |
| -------- | ------------------------ | ------ |
| `PATH`   | `$HOME/.local/bin:$PATH` | bashrc |

## Network Requirements

- `registry.npmjs.org` - NPM registry

## Installation

```bash
extension-manager install openskills
```

## Usage

```bash
# Install a skill
openskills install skill-name

# List installed skills
openskills list

# Update a skill
openskills update skill-name

# Remove a skill
openskills remove skill-name
```

## Validation

```bash
openskills --version    # Expected: X.X.X
```

## Upgrade

**Strategy:** automatic

```bash
extension-manager upgrade openskills
```

## Removal

Requires confirmation before removal.

```bash
extension-manager remove openskills
```

Removes:

- `~/.openskills`
- `~/.local/bin/openskills`

## Related Extensions

- [ai-toolkit](AI-TOOLKIT.md) - AI tools suite
- [claude-marketplace](CLAUDE-MARKETPLACE.md) - Plugin marketplace
