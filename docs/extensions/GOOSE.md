# Goose

Block's open-source AI agent that automates engineering tasks.

## Overview

| Property         | Value                          |
| ---------------- | ------------------------------ |
| **Category**     | ai                             |
| **Version**      | 1.0.0                          |
| **Installation** | script                         |
| **Disk Space**   | 200 MB                         |
| **Dependencies** | None                           |
| **Author**       | Block                          |
| **License**      | Apache-2.0                     |
| **Homepage**     | https://block.github.io/goose/ |

## Description

Goose is Block's open-source AI agent that automates engineering tasks - builds projects, executes code, debugs, and orchestrates workflows with any LLM. It provides a powerful, extensible agent framework that can work with multiple AI providers.

## Installed Tools

| Tool    | Type     | Description        |
| ------- | -------- | ------------------ |
| `goose` | cli-tool | Goose AI agent CLI |

## Configuration

### Environment Variables

| Variable | Value                    | Scope  | Description |
| -------- | ------------------------ | ------ | ----------- |
| `PATH`   | `$HOME/.local/bin:$PATH` | bashrc | Binary path |

## Network Requirements

- `github.com` - GitHub downloads and releases
- `api.anthropic.com` - Anthropic Claude API
- `api.openai.com` - OpenAI API
- `openrouter.ai` - OpenRouter API

## Installation

```bash
extension-manager install goose
```

## Validation

```bash
goose --version    # Expected: goose X.X.X
```

## Usage

### Configure Provider

```bash
# Interactive configuration
goose configure

# Set up your preferred LLM provider
```

### Start a Session

```bash
# Start an interactive session
goose session

# Execute a task
goose run "build the project and run tests"
```

### Common Commands

```bash
# Get help
goose --help

# Update to latest version
goose update
```

## Upgrade

**Strategy:** manual

```bash
# Using Goose's built-in updater
goose update

# Or reinstall via extension manager
extension-manager upgrade goose
```

## Removal

```bash
extension-manager remove goose
```

Removes:

- `~/.local/bin/goose`
- `~/.config/goose`
- `~/.local/share/goose`

## Related Extensions

- [ai-toolkit](AI-TOOLKIT.md) - AI tools suite
- [claude-flow](CLAUDE-FLOW.md) - Multi-agent orchestration
- [agentic-flow](AGENTIC-FLOW.md) - Multi-model AI framework
