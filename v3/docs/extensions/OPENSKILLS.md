# OpenSkills Extension

> Version: 2.0.0 | Category: productivity | Last Updated: 2026-01-26

## Overview

OpenSkills CLI for managing Claude Code skills from Anthropic's marketplace. Browse, install, and manage skills to extend Claude Code capabilities.

## What It Provides

| Tool       | Type     | License | Description               |
| ---------- | -------- | ------- | ------------------------- |
| openskills | cli-tool | MIT     | Claude Code skill manager |

## Requirements

- **Disk Space**: 100 MB
- **Memory**: 0 MB (minimal)
- **Install Time**: ~30 seconds
- **Dependencies**: nodejs

### Network Domains

- registry.npmjs.org

## Installation

```bash
sindri extension install openskills
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

## Key Features

- **Skill Discovery** - Browse available skills
- **Installation** - Install skills from marketplace
- **Management** - Enable/disable skills
- **Updates** - Keep skills current

## Usage Examples

### Browse Skills

```bash
# List available skills
openskills list

# Search for skills
openskills search "testing"

# Get skill details
openskills info skill-name
```

### Install Skills

```bash
# Install a skill
openskills install testing-helper

# Install specific version
openskills install testing-helper@1.2.0

# Install from URL
openskills install https://github.com/user/skill
```

### Manage Skills

```bash
# List installed skills
openskills installed

# Enable a skill
openskills enable skill-name

# Disable a skill
openskills disable skill-name

# Uninstall a skill
openskills uninstall skill-name
```

### Update Skills

```bash
# Update all skills
openskills update

# Update specific skill
openskills update skill-name

# Check for updates
openskills outdated
```

### Create Skills

```bash
# Initialize a new skill
openskills init my-skill

# Validate skill
openskills validate

# Publish skill
openskills publish
```

## Validation

The extension validates the following commands:

- `openskills` - Must match pattern `\d+\.\d+\.\d+`

## Removal

```bash
sindri extension remove openskills
```

**Requires confirmation.** Removes:

- mise openskills tools
- ~/.openskills

## Related Extensions

- [nodejs](NODEJS.md) - Required dependency
- [claudeup](CLAUDEUP.md) - Alternative skill management via TUI
