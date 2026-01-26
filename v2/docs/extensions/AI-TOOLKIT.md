# AI Toolkit

AI CLI tools and coding assistants.

## Overview

| Property         | Value                                                                                      |
| ---------------- | ------------------------------------------------------------------------------------------ |
| **Category**     | ai                                                                                         |
| **Version**      | 2.1.0                                                                                      |
| **Installation** | script                                                                                     |
| **Disk Space**   | 1000 MB                                                                                    |
| **Dependencies** | [nodejs](NODEJS.md), [python](PYTHON.md), [golang](GOLANG.md), [github-cli](GITHUB-CLI.md) |

## Description

AI CLI tools and coding assistants (Fabric, Codex, Gemini, Droid, Grok, Copilot) - provides a comprehensive suite of AI-powered development tools.

For local LLM capabilities, install the separate [ollama](OLLAMA.md) extension.

## Installed Tools

| Tool     | Type     | Pinned Version | Description              |
| -------- | -------- | -------------- | ------------------------ |
| `fabric` | cli-tool | latest (Go)    | AI pattern executor      |
| `codex`  | cli-tool | 0.80           | OpenAI Codex integration |
| `gemini` | cli-tool | 0.22           | Google Gemini CLI        |
| `droid`  | cli-tool | latest         | Factory AI helper        |
| `grok`   | cli-tool | 0.0.34         | xAI Grok integration     |

> **Note:** npm-based tools (codex, gemini, grok) use pinned versions to avoid npm registry query timeouts.

## Configuration

### Environment Variables

| Variable | Value                      | Scope  |
| -------- | -------------------------- | ------ |
| `PATH`   | `$HOME/.local/bin:$PATH`   | bashrc |
| `PATH`   | `$HOME/go/bin:$PATH`       | bashrc |
| `PATH`   | `$HOME/.factory/bin:$PATH` | bashrc |

### Templates

| Template          | Destination                                | Description |
| ----------------- | ------------------------------------------ | ----------- |
| `bashrc.template` | `~/.bashrc`                                | Tool paths  |
| `readme.template` | `/workspace/extensions/ai-tools/README.md` | Usage guide |

## Secrets (Optional)

| Secret                  | Description           |
| ----------------------- | --------------------- |
| `google_gemini_api_key` | Google Gemini API key |
| `grok_api_key`          | xAI Grok API key      |

## Network Requirements

- `github.com` - GitHub dependencies
- `factory.ai` - Factory AI
- `app.factory.ai` - Factory AI CLI
- `api.openai.com` - OpenAI API
- `generativelanguage.googleapis.com` - Gemini API

## Installation

```bash
extension-manager install ai-toolkit
```

## Validation

```bash
fabric --version
codex --version
gemini --version
droid --version
grok --version
```

## Upgrade

**Strategy:** manual

```bash
extension-manager upgrade ai-toolkit
```

## Removal

```bash
extension-manager remove ai-toolkit
```

Removes all AI tools and their configuration directories.

## Related Extensions

- [ollama](OLLAMA.md) - Local LLM runtime (Llama, Mistral, CodeLlama)
- [openskills](OPENSKILLS.md) - Claude Code skills
- [claude-code-mux](CLAUDE-CODE-MUX.md) - AI routing proxy
