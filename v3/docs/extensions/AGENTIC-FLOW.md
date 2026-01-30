# Agentic Flow Extension

> Version: 1.0.0 | Category: ai-agents | Last Updated: 2026-01-26

## Overview

Multi-model AI agent framework for Claude Code with cost optimization (alpha). Provides intelligent agent orchestration with support for multiple AI providers.

## What It Provides

| Tool         | Type     | License | Description                 |
| ------------ | -------- | ------- | --------------------------- |
| agentic-flow | cli-tool | MIT     | Multi-model agent framework |

## Requirements

- **Disk Space**: 80 MB
- **Memory**: 128 MB
- **Install Time**: ~60 seconds
- **Dependencies**: nodejs

### Network Domains

- registry.npmjs.org
- github.com

## Installation

```bash
sindri extension install agentic-flow
```

## Configuration

### Environment Variables

| Variable | Value                  | Description    |
| -------- | ---------------------- | -------------- |
| `PATH`   | $HOME/.local/bin:$PATH | Local binaries |

### Templates

- agentic-flow.aliases - Shell aliases

### Install Method

Uses mise for tool management.

### Upgrade Strategy

Reinstall.

## Key Features

- **Multi-model support** - Use different AI models for different tasks
- **Cost optimization** - Route tasks to cost-effective models
- **Hooks system** - Extensible automation hooks
- **Intelligence bootstrap** - Pretrain from codebase

## Usage Examples

### Initialization

```bash
# Initialize agentic flow
npx agentic-flow init

# Bootstrap intelligence from codebase
npx agentic-flow hooks pretrain
```

### Agent Management

```bash
# Run with agentic flow
npx agentic-flow run --task "analyze code"

# Check status
npx agentic-flow status
```

## Capabilities

### Project Initialization (Priority 60)

Lower priority than claude-flow. Initializes:

- Agentic Flow project structure
- Hooks system (optional pretrain)

### Authentication

Optional Anthropic API key:

- `ANTHROPIC_API_KEY` - Required for Anthropic models

### State Markers

| Path                  | Type      | Description             |
| --------------------- | --------- | ----------------------- |
| `.agentic-flow`       | directory | Configuration directory |
| `.agentic-flow/hooks` | directory | Hooks system            |

## Collision Handling

Designed to work alongside claude-flow:

- **CLAUDE.md** - Appends content instead of overwriting
- **JSON files** - Merges configuration
- **Helper scripts** - Skips if claude-flow's version exists (claude-flow runs first at priority 20)

## Validation

The extension validates the following commands:

- `agentic-flow` - Must be available

## Removal

```bash
sindri extension remove agentic-flow
```

**Requires confirmation.** Removes mise agentic-flow tools.

## Related Extensions

- [nodejs](NODEJS.md) - Required dependency
- [claude-flow-v3](CLAUDE-FLOW-V3.md) - Primary orchestration (higher priority)
