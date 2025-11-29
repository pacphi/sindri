# AI Toolkit

AI CLI tools and coding assistants.

## Overview

| Property         | Value                                                                                      |
| ---------------- | ------------------------------------------------------------------------------------------ |
| **Category**     | ai                                                                                         |
| **Version**      | 2.0.0                                                                                      |
| **Installation** | script                                                                                     |
| **Disk Space**   | 2000 MB                                                                                    |
| **Dependencies** | [nodejs](NODEJS.md), [python](PYTHON.md), [golang](GOLANG.md), [github-cli](GITHUB-CLI.md) |

## Description

AI CLI tools and coding assistants (Ollama, Fabric, Codex, Gemini, Hector, Droid, Grok, Copilot) - provides a comprehensive suite of AI-powered development tools.

## Installed Tools

| Tool     | Type     | Description              |
| -------- | -------- | ------------------------ |
| `ollama` | server   | Local LLM runtime        |
| `fabric` | cli-tool | AI pattern executor      |
| `codex`  | cli-tool | OpenAI Codex integration |
| `gemini` | cli-tool | Google Gemini CLI        |
| `hector` | cli-tool | Code review assistant    |
| `droid`  | cli-tool | Android AI helper        |
| `grok`   | cli-tool | xAI Grok integration     |

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

- `ollama.com` - Ollama downloads
- `github.com` - GitHub dependencies
- `app.factory.ai` - Factory AI
- `api.openai.com` - OpenAI API
- `generativelanguage.googleapis.com` - Gemini API

## Installation

```bash
extension-manager install ai-toolkit
```

## Validation

```bash
ollama --version
fabric --version
codex --version
gemini --version
hector --version
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

- [openskills](OPENSKILLS.md) - Claude Code skills
- [claude-code-mux](CLAUDE-CODE-MUX.md) - AI routing proxy
