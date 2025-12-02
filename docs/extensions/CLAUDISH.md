# Claudish

Claude Code CLI proxy for OpenRouter models - run Claude Code with any OpenRouter model via local Anthropic API proxy.

## Overview

| Property         | Value               |
| ---------------- | ------------------- |
| **Category**     | ai                  |
| **Version**      | 1.0.0               |
| **Installation** | mise                |
| **Disk Space**   | 50 MB               |
| **Dependencies** | [nodejs](NODEJS.md) |

## Description

Claudish is a standalone CLI that lets you run Claude Code with any OpenRouter model via a local Anthropic API proxy. It's 100% verified to route to real OpenRouter models, NOT Anthropic.

Perfect for:

- Exploring different AI models
- Cost optimization
- Specialized tasks requiring specific model capabilities
- Testing model performance
- Avoiding Anthropic API limitations

## Installed Tools

| Tool       | Type     | Description                            |
| ---------- | -------- | -------------------------------------- |
| `claudish` | cli-tool | OpenRouter model proxy for Claude Code |

## Secrets (Required)

| Secret               | Description           |
| -------------------- | --------------------- |
| `openrouter_api_key` | OpenRouter API key    |

## Network Requirements

- `registry.npmjs.org` - NPM registry
- `openrouter.ai` - OpenRouter website
- `api.openrouter.ai` - OpenRouter API

## Installation

```bash
extension-manager install claudish
```

## Configuration

Set your OpenRouter API key:

```bash
export OPENROUTER_API_KEY="your-api-key"
```

The extension also sets `ANTHROPIC_API_KEY=placeholder` to prevent Claude Code dialogs.

## Usage

### Basic Usage

```bash
# Interactive mode with model selector
claudish "implement user authentication"

# Specific model
claudish --model x-ai/grok-code-fast-1 "add tests"

# List available models
claudish --list-models
```

### Available Models

Prioritized OpenRouter models:

| Model                               | Best For                |
| ----------------------------------- | ----------------------- |
| `x-ai/grok-code-fast-1` (default)   | Fast coding iterations  |
| `openai/gpt-5-codex`                | Complex implementations |
| `minimax/minimax-m2`                | General coding tasks    |
| `zhipu-ai/glm-4.6`                  | Multilingual code       |
| `qwen/qwen3-vl-235b-a22b-instruct`  | UI/visual tasks         |

### Command-Line Options

| Flag                   | Description                              |
| ---------------------- | ---------------------------------------- |
| `-m, --model <model>`  | Specify OpenRouter model                 |
| `-p, --port <port>`    | Proxy server port                        |
| `-i, --interactive`    | Persistent session mode                  |
| `-q, --quiet`          | Suppress logs                            |
| `-v, --verbose`        | Show logs                                |
| `--json`               | Structured output format                 |
| `--dangerous`          | Disable sandbox restrictions             |
| `--agent <name>`       | Use specific agent                       |
| `--init`               | Install Claudish skill in project        |

### Initialize Skill

Install the Claudish skill in your project:

```bash
cd /path/to/your/project
claudish --init
```

This creates `.claude/skills/claudish-usage/` for automatic best practices.

## Validation

```bash
claudish --version
```

## Upgrade

**Strategy:** automatic

Automatically upgrades via mise.

## Removal

```bash
extension-manager remove claudish
```

Removes mise configuration and skill files from `~/.claude/skills/claudish-usage`.

## Source Project

- [MadAppGang/claudish](https://github.com/MadAppGang/claudish)

## Related Extensions

- [claude-code-mux](CLAUDE-CODE-MUX.md) - AI routing proxy (18+ providers)
- [ai-toolkit](AI-TOOLKIT.md) - AI tools suite
- [claude-auth-with-api-key](CLAUDE-AUTH-WITH-API-KEY.md) - Claude API key authentication
