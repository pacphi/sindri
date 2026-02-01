# AI Toolkit Extension

> Version: 2.1.0 | Category: ai-dev | Last Updated: 2026-01-26

## Overview

AI CLI tools and coding assistants including Fabric, Codex, Gemini, Droid, and Grok. For local LLM hosting, install the separate ollama extension.

## What It Provides

| Tool   | Type     | License    | Description                               |
| ------ | -------- | ---------- | ----------------------------------------- |
| fabric | cli-tool | MIT        | AI-powered command patterns and workflows |
| codex  | cli-tool | MIT        | OpenAI Codex CLI for code generation      |
| gemini | cli-tool | Apache-2.0 | Google Gemini AI CLI                      |
| droid  | cli-tool | MIT        | AI coding assistant                       |
| grok   | cli-tool | Apache-2.0 | xAI Grok CLI                              |

## Requirements

- **Disk Space**: 1000 MB
- **Memory**: 4096 MB
- **Install Time**: ~300 seconds (5 minutes)
- **Dependencies**: nodejs, python, golang, github-cli

### Network Domains

- github.com
- factory.ai
- app.factory.ai
- api.openai.com
- generativelanguage.googleapis.com

### Secrets (Optional)

- `google_gemini_api_key` - For Google Gemini access
- `grok_api_key` - For xAI Grok access

## Installation

```bash
sindri extension install ai-toolkit
```

## Configuration

### Environment Variables

| Variable | Value                    | Description          |
| -------- | ------------------------ | -------------------- |
| `PATH`   | $HOME/.local/bin:$PATH   | Local binaries       |
| `PATH`   | $HOME/go/bin:$PATH       | Go binaries          |
| `PATH`   | $HOME/.factory/bin:$PATH | Factory CLI binaries |

### Templates

- bashrc.template - Shell configuration
- readme.template - Installation documentation at ~/extensions/ai-tools/README.md

### Install Method

Uses a custom installation script with 600 second timeout.

### Upgrade Strategy

Manual - run upgrade.sh script.

## Usage Examples

### Fabric

```bash
# List available patterns
fabric --list

# Run a pattern
fabric --pattern explain "complex code here"

# Summarize content
fabric --pattern summarize < document.txt

# Extract wisdom from text
fabric --pattern extract_wisdom < article.txt
```

### Codex

```bash
# Generate code
codex "Write a Python function to calculate fibonacci"

# Interactive mode
codex --interactive
```

### Gemini

```bash
# Chat with Gemini
gemini "Explain machine learning"

# Code generation
gemini --code "Create a REST API in Python"
```

### Grok

```bash
# Query Grok
grok "What are the latest developments in AI?"

# Code assistance
grok --code "Debug this function"
```

### Droid

```bash
# Code review
droid review main.py

# Suggestions
droid suggest "How can I optimize this?"
```

## Validation

The extension validates the following commands:

- `fabric` - Must be available
- `codex` - Must be available
- `gemini` - Must be available
- `droid` - Must be available
- `grok` - Must be available

## Removal

```bash
sindri extension remove ai-toolkit
```

This removes:

- ~/.local/share/fabric
- ~/.local/bin/fabric
- ~/.factory
- ~/.grok
- ~/.config/gemini
- ~/extensions/ai-tools

## Related Extensions

- [ollama](OLLAMA.md) - Local LLM hosting (separate extension)
- [nodejs](NODEJS.md) - Required dependency
- [python](PYTHON.md) - Required dependency
- [golang](GOLANG.md) - Required dependency
- [github-cli](GITHUB-CLI.md) - Required dependency
