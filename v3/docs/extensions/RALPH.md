# Ralph Extension

> Version: 1.0.0 | Category: productivity | Last Updated: 2026-01-26

## Overview

AI-driven autonomous development system. Build software projects with minimal intervention using Claude. Supports discovery, planning, deployment, and review phases.

## What It Provides

| Tool          | Type      | License | Description                      |
| ------------- | --------- | ------- | -------------------------------- |
| ralph-inferno | framework | MIT     | Autonomous development framework |

## Requirements

- **Disk Space**: 300 MB
- **Memory**: 512 MB
- **Install Time**: ~90 seconds
- **Dependencies**: nodejs

### Network Domains

- registry.npmjs.org
- github.com
- api.anthropic.com

## Installation

```bash
extension-manager install ralph
```

## Configuration

### Environment Variables

| Variable     | Value        | Description          |
| ------------ | ------------ | -------------------- |
| `RALPH_HOME` | $HOME/.ralph | Ralph home directory |

### Install Method

Uses a custom installation script with 120 second timeout.

### Upgrade Strategy

Manual - run upgrade.sh script.

## Key Features

- **Autonomous Development** - Minimal human intervention
- **Discovery Phase** - Understand requirements
- **Planning Phase** - Architecture and task breakdown
- **Deployment Phase** - Build and deploy
- **Review Phase** - Quality assurance

## Usage Examples

### Installation

```bash
# Initialize Ralph
npx ralph-inferno install

# This sets up:
# - .ralph directory
# - Configuration files
# - Claude Code commands
```

### Project Commands

```bash
# Ralph provides Claude Code commands in ~/.claude/commands/
# These enable autonomous workflows:

# /ralph:discover - Analyze requirements
# /ralph:plan - Create implementation plan
# /ralph:build - Execute development
# /ralph:review - Quality review
```

### Configuration

```json
// .ralph/config.json
{
  "project": {
    "name": "my-project",
    "description": "Project description"
  },
  "phases": {
    "discovery": true,
    "planning": true,
    "deployment": true,
    "review": true
  },
  "ai": {
    "provider": "anthropic",
    "model": "claude-sonnet-4-20250514"
  }
}
```

## Capabilities

### Project Initialization (Priority 70)

Lower priority - runs after other orchestration tools:

```bash
bash scripts/init-project.sh
```

### Authentication

**Required:** Anthropic API key for autonomous features:

- `ANTHROPIC_API_KEY` - Required for AI-driven development

### State Markers

| Path                 | Type      | Description             |
| -------------------- | --------- | ----------------------- |
| `.ralph`             | directory | Configuration directory |
| `.ralph/config.json` | file      | Project configuration   |

### VM Deployment

Optional feature for remote VM deployment capabilities (no API key required).

## Collision Handling

Ralph is designed to coexist with other extensions:

- **Commands directory** - Merges (Ralph commands are uniquely named with ralph: prefix)
- No conflicts with CLAUDE.md or JSON files

## Validation

Uses a custom validation script:

```bash
scripts/validate.sh
```

## Removal

```bash
extension-manager remove ralph
```

**Requires confirmation.** Removes:

- ~/.ralph
- ./.ralph (project directory)
- Runs uninstall script

## Related Extensions

- [nodejs](NODEJS.md) - Required dependency
