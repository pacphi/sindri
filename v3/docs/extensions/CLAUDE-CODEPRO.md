# Claude CodePro Extension

> Version: 4.5.29 | Category: claude | Last Updated: 2026-01-26

## Overview

Production-grade TDD-enforced development environment for Claude Code with automated quality checks. Provides strict development workflows with test-driven development enforcement.

**Note:** This extension takes FULL CONTROL of the `.claude/` directory and is INCOMPATIBLE with claude-flow extensions.

## What It Provides

| Tool                 | Type    | License                 | Description                     |
| -------------------- | ------- | ----------------------- | ------------------------------- |
| claude-codepro (ccp) | utility | Proprietary (Free tier) | TDD enforcement and quality CLI |

## Requirements

- **Disk Space**: 350 MB
- **Memory**: 512 MB
- **Install Time**: ~300 seconds
- **Install Timeout**: 600 seconds
- **Dependencies**: python, nodejs, github-cli, mise-config

### Network Domains

- raw.githubusercontent.com
- github.com
- registry.npmjs.org
- pypi.org

## Installation

```bash
sindri extension install claude-codepro
```

## Configuration

### Install Method

Uses a custom installation script with 600 second timeout.

### Upgrade Strategy

Reinstall.

## Key Features

- **TDD Enforcement** - Pre-edit hooks require tests before code
- **Quality Checks** - Automated Python/TypeScript linting
- **Semantic Search** - Vexor local or OpenAI-powered search
- **Rule Management** - Custom development rules

## Usage Examples

### Setup

```bash
# Initialize in project
ccp setup

# Register license (optional for free tier)
ccp register

# Check status
ccp status
```

### TDD Workflow

```bash
# CodePro enforces test-first development
# 1. Write test
# 2. Run ccp check
# 3. Implement code
# 4. Run tests

ccp check  # Validates TDD compliance
```

### Quality Commands

```bash
# Run all checks
ccp quality

# Lint code
ccp lint

# Format code
ccp format
```

### Rule Management

```bash
# List rules
ccp rules list

# Add custom rule
ccp rules add --name "no-any" --type typescript

# Enable/disable rules
ccp rules enable no-any
ccp rules disable no-any
```

## Capabilities

### Project Initialization (Priority 5)

Lower priority than claude-flow extensions. Runs:

```bash
source ~/.bashrc && ccp setup
```

### State Markers

| Path              | Type      | Description            |
| ----------------- | --------- | ---------------------- |
| `.claude`         | directory | Managed by CodePro     |
| `.claude/bin/ccp` | file      | CodePro binary wrapper |
| `.claude/hooks`   | directory | TDD enforcement hooks  |

### Authentication

Optional API keys for enhanced features:

- `OPENAI_API_KEY` - For Vexor semantic search
- `FIRECRAWL_API_KEY` - Optional feature

## Critical Conflict Warning

Claude CodePro is **INCOMPATIBLE** with:

- claude-flow-v3
- claude-flow-v2
- agentic-flow
- agentic-qe
- Any extension that uses `.claude/` directory

If `.claude` directory exists, installation will be skipped with an error message providing resolution steps.

## Validation

The extension validates the following commands:

- `ccp` - Must be available

## Removal

```bash
sindri extension remove claude-codepro
```

**Requires confirmation.**

**WARNING:** Removes your entire .claude configuration including:

- ~/.claude
- ~/.claude-mem
- ~/.vexor
- ~/.qlty
- ~/.config/ccstatusline
- All custom rules in .claude/rules/custom/

## Related Extensions

- [python](PYTHON.md) - Required dependency
- [nodejs](NODEJS.md) - Required dependency
- [github-cli](GITHUB-CLI.md) - Required dependency
- [mise-config](MISE-CONFIG.md) - Required dependency
