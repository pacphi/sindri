# Goose Extension

> Version: 1.0.0 | Category: ai-dev | Last Updated: 2026-01-26

## Overview

Block's open-source AI agent that automates engineering tasks - builds projects, executes code, debugs, and orchestrates workflows with any LLM.

## What It Provides

| Tool  | Type     | License    | Description                     |
| ----- | -------- | ---------- | ------------------------------- |
| goose | cli-tool | Apache-2.0 | AI engineering automation agent |

## Requirements

- **Disk Space**: 200 MB
- **Install Time**: ~120 seconds
- **Dependencies**: None

### Network Domains

- github.com
- api.anthropic.com
- api.openai.com
- openrouter.ai

## Installation

```bash
extension-manager install goose
```

## Configuration

### Environment Variables

| Variable | Value                  | Description    |
| -------- | ---------------------- | -------------- |
| `PATH`   | $HOME/.local/bin:$PATH | Local binaries |

### Install Method

Uses a custom installation script with 300 second timeout.

### Upgrade Strategy

Manual - run install.sh script.

## Key Features

- **Multi-LLM Support** - Works with Anthropic, OpenAI, OpenRouter
- **Code Execution** - Run and test code autonomously
- **Debugging** - Intelligent error analysis and fixes
- **Project Building** - Create complete projects from descriptions
- **Workflow Orchestration** - Multi-step automation

## Usage Examples

### Getting Started

```bash
# Check version
goose --version

# Start interactive session
goose

# Run with specific task
goose "Create a Python web scraper"
```

### Configuration

```bash
# Configure LLM provider
goose config set provider anthropic
goose config set api_key $ANTHROPIC_API_KEY

# Or use OpenAI
goose config set provider openai
goose config set api_key $OPENAI_API_KEY
```

### Project Building

```bash
# Create a new project
goose "Create a REST API with Express and TypeScript"

# Build from specification
goose "Build a CLI tool that converts CSV to JSON"
```

### Code Execution

```bash
# Run code with Goose
goose "Write and run a Python script that fetches weather data"

# Debug code
goose "Find and fix the bug in src/auth.py"
```

### Workflow Automation

```bash
# Multi-step tasks
goose "Set up a new React project with TypeScript, ESLint, and Prettier"

# Complex orchestration
goose "Create tests for the authentication module and run them"
```

## Validation

The extension validates the following commands:

- `goose --version` - Must match pattern `goose \d+\.\d+\.\d+`

## Removal

```bash
extension-manager remove goose
```

This removes:

- ~/.local/bin/goose
- ~/.config/goose
- ~/.local/share/goose

## Related Extensions

- [ollama](OLLAMA.md) - Local LLM hosting for Goose
- [ai-toolkit](AI-TOOLKIT.md) - Additional AI tools
