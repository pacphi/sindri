# Agentic QE Extension

> Version: 1.1.0 | Category: ai-agents | Last Updated: 2026-01-26

## Overview

Agentic Quality Engineering v3 with AI-powered test generation, coverage analysis, and multi-agent workflows. Comprehensive testing automation framework.

## What It Provides

| Tool             | Type      | License | Description                    |
| ---------------- | --------- | ------- | ------------------------------ |
| agentic-qe (aqe) | framework | MIT     | AI-powered quality engineering |

## Requirements

- **Disk Space**: 500 MB
- **Memory**: 512 MB
- **Install Time**: ~120 seconds
- **Dependencies**: nodejs

### Network Domains

- registry.npmjs.org
- github.com
- api.anthropic.com

## Installation

```bash
sindri extension install agentic-qe
```

## Configuration

### Environment Variables

| Variable   | Value             | Description               |
| ---------- | ----------------- | ------------------------- |
| `AQE_HOME` | $HOME/.agentic-qe | Agentic QE home directory |

### Install Method

Uses mise for tool management with automatic shim refresh.

### Upgrade Strategy

Automatic via mise upgrade.

## Key Features

- **AI Test Generation** - Automatically generate tests from code
- **Coverage Analysis** - Intelligent coverage gap detection
- **Multi-agent Workflows** - Parallel quality analysis
- **Pattern Learning** - Learns from codebase patterns

## Usage Examples

### Initialization

```bash
# Initialize in project
aqe init --auto

# Check version
aqe --version
```

### Test Generation

```bash
# Generate tests for a file
aqe generate tests src/module.ts

# Generate tests for entire project
aqe generate tests --all

# Generate with specific framework
aqe generate tests --framework jest src/
```

### Coverage Analysis

```bash
# Analyze coverage gaps
aqe analyze coverage

# Generate coverage report
aqe report coverage --format html
```

### Quality Workflows

```bash
# Run full quality check
aqe run quality

# Run specific checks
aqe run lint
aqe run security
aqe run complexity
```

## Capabilities

### Project Initialization (Priority 50)

Runs after claude-flow (priority 20) but before agentic-flow (60):

```bash
aqe init --auto
```

### Authentication

**Required:** Anthropic API key for AI-powered features:

- `ANTHROPIC_API_KEY` - Required for test generation

### State Markers

| Path          | Type      | Description             |
| ------------- | --------- | ----------------------- |
| `.agentic-qe` | directory | Configuration directory |

## Collision Handling

Designed to coexist with other extensions:

- **CLAUDE.md** - Appends content
- **JSON files** - Merges configuration
- **.claude** - Merges (no backup since claude-flow creates structure first)

## Validation

The extension validates the following commands:

- `aqe --version` - Must match pattern `3\.\d+\.\d+(-[a-z]+(\.[0-9]+)?)?`

## Removal

```bash
sindri extension remove agentic-qe
```

**Requires confirmation.** Removes:

- mise agentic-qe tools
- ~/.agentic-qe
- ./.agentic-qe (project directory)

## Related Extensions

- [nodejs](NODEJS.md) - Required dependency
