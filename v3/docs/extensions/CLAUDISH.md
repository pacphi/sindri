# Claudish Extension

> Version: 1.0.0 | Category: claude | Last Updated: 2026-01-26

## Overview

Claude Code CLI proxy for OpenRouter models - run Claude Code with any OpenRouter model via local Anthropic API proxy. Enables using alternative models with Claude Code.

## What It Provides

| Tool     | Type     | License | Description                      |
| -------- | -------- | ------- | -------------------------------- |
| claudish | cli-tool | MIT     | OpenRouter proxy for Claude Code |

## Requirements

- **Disk Space**: 50 MB
- **Memory**: 256 MB
- **Install Time**: ~30 seconds
- **Dependencies**: nodejs

### Network Domains

- registry.npmjs.org
- openrouter.ai
- api.openrouter.ai

### Secrets Required

- `openrouter_api_key` - OpenRouter API key for model access

## Installation

```bash
extension-manager install claudish
```

## Configuration

### Environment Variables

| Variable            | Value       | Description                      |
| ------------------- | ----------- | -------------------------------- |
| `ANTHROPIC_API_KEY` | placeholder | Placeholder (proxy handles auth) |

### Install Method

Uses mise for tool management with automatic shim refresh.

### Upgrade Strategy

Automatic via mise upgrade.

## How It Works

Claudish acts as a local proxy server that:

1. Receives requests intended for Anthropic API
2. Translates them to OpenRouter format
3. Forwards to your chosen OpenRouter model
4. Returns responses in Anthropic-compatible format

This allows using Claude Code with models like:

- GPT-4
- Gemini
- Llama
- Mistral
- And any other OpenRouter-supported model

## Usage Examples

### Start Proxy

```bash
# Start claudish proxy
claudish

# The proxy listens on localhost and forwards to OpenRouter
# Claude Code connects to the local proxy instead of Anthropic
```

### Configuration

```bash
# Set OpenRouter API key
export OPENROUTER_API_KEY="your-key"

# Configure model preference
claudish --model gpt-4-turbo

# Use specific endpoint
claudish --endpoint https://openrouter.ai/api/v1
```

### With Claude Code

Once claudish is running:

1. Set `ANTHROPIC_API_KEY` to any value (proxy handles auth)
2. Set `ANTHROPIC_BASE_URL` to claudish proxy address
3. Use Claude Code normally

```bash
# Example setup
export ANTHROPIC_BASE_URL="http://localhost:8080"
export ANTHROPIC_API_KEY="placeholder"
claude code
```

## Validation

The extension validates the following commands:

- `claudish --version` - Must be available

## Removal

```bash
extension-manager remove claudish
```

This removes:

- mise claudish tools
- ~/.claude/skills/claudish-usage

## Related Extensions

- [nodejs](NODEJS.md) - Required dependency
- [claude-code-mux](CLAUDE-CODE-MUX.md) - Alternative multi-provider proxy
