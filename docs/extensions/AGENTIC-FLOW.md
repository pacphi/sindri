# Agentic Flow

Multi-model AI agent framework for Claude Code with cost optimization (alpha version).

## Overview

| Property         | Value                                  |
| ---------------- | -------------------------------------- |
| **Category**     | ai                                     |
| **Version**      | 1.0.0                                  |
| **Installation** | script (npm)                           |
| **Disk Space**   | 80 MB                                  |
| **Memory**       | 128 MB                                 |
| **Dependencies** | [nodejs](NODEJS.md)                    |
| **Author**       | ruvnet                                 |
| **License**      | MIT                                    |
| **Homepage**     | https://github.com/ruvnet/agentic-flow |

## Description

Agentic Flow is a multi-model AI agent framework designed for Claude Code integration with cost optimization capabilities. This extension installs the **alpha** version (2.x). It provides specialized agents for coding, code review, and research tasks, with support for multiple AI providers and optimization strategies.

## Installed Tools

| Tool           | Type     | Description      |
| -------------- | -------- | ---------------- |
| `agentic-flow` | cli-tool | Agentic Flow CLI |

## Configuration

### Environment Variables

| Variable | Value                    | Scope  | Description |
| -------- | ------------------------ | ------ | ----------- |
| `PATH`   | `$HOME/.local/bin:$PATH` | bashrc | Binary path |

### Shell Aliases

The extension installs convenience aliases:

| Alias           | Command                                          | Description             |
| --------------- | ------------------------------------------------ | ----------------------- |
| `af`            | `agentic-flow`                                   | Core command shortcut   |
| `af-help`       | `agentic-flow --help`                            | Help command            |
| `af-coder`      | `agentic-flow --agent coder --task`              | Coding agent            |
| `af-reviewer`   | `agentic-flow --agent reviewer --task`           | Code review agent       |
| `af-researcher` | `agentic-flow --agent researcher --task`         | Research agent          |
| `af-claude`     | `agentic-flow claude-code`                       | Claude Code integration |
| `af-openrouter` | `agentic-flow claude-code --provider openrouter` | OpenRouter provider     |
| `af-gemini`     | `agentic-flow claude-code --provider gemini`     | Gemini provider         |
| `af-cost`       | `agentic-flow --optimize cost`                   | Cost-optimized mode     |
| `af-speed`      | `agentic-flow --optimize speed`                  | Speed-optimized mode    |
| `af-quality`    | `agentic-flow --optimize quality`                | Quality-optimized mode  |

### Utility Functions

| Function      | Usage                           | Description                      |
| ------------- | ------------------------------- | -------------------------------- |
| `af-task`     | `af-task <agent> "task"`        | Execute task with specific agent |
| `af-provider` | `af-provider <provider> [args]` | Execute with specific provider   |

## Network Requirements

- `registry.npmjs.org` - npm package registry
- `github.com` - Source code repository

## Installation

```bash
# Install with dependency
extension-manager install agentic-flow

# Node.js is installed automatically as a dependency
```

## Validation

```bash
agentic-flow --version
```

## Usage

### Agent-Based Tasks

```bash
# Use coder agent for development tasks
af-coder "implement a REST API endpoint"

# Use reviewer agent for code review
af-reviewer "review the authentication module"

# Use researcher agent for research
af-researcher "analyze best practices for error handling"
```

### Provider Integration

```bash
# Use with Claude Code
af-claude

# Use with OpenRouter
af-openrouter

# Use with Gemini
af-gemini
```

### Optimization Strategies

```bash
# Optimize for cost
af-cost "generate unit tests"

# Optimize for speed
af-speed "quick code fix"

# Optimize for quality
af-quality "architect new feature"
```

### Task Execution

```bash
# Execute task with balanced optimization
af-task coder "refactor the data layer"

# Execute with specific provider
af-provider openrouter "analyze code complexity"
```

## Upgrade

**Strategy:** automatic

```bash
extension-manager upgrade agentic-flow
```

## Removal

### Requires confirmation

```bash
extension-manager remove agentic-flow
```

Removes:

- Global npm package `agentic-flow`
- Shell aliases from `~/.bashrc`

## Related Extensions

- [nodejs](NODEJS.md) - Required Node.js runtime
- [claude-flow](CLAUDE-FLOW.md) - Multi-agent orchestration
- [ai-toolkit](AI-TOOLKIT.md) - AI tools suite
- [goose](GOOSE.md) - Block's AI agent
