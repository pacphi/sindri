# mdflow

Multi-backend CLI that transforms markdown files into executable AI agents - run prompts against Claude, Codex, Gemini, or Copilot via markdown.

## Overview

| Property         | Value               |
| ---------------- | ------------------- |
| **Category**     | ai                  |
| **Version**      | 1.0.0               |
| **Installation** | mise                |
| **Disk Space**   | 50 MB               |
| **Dependencies** | [nodejs](NODEJS.md) |

## Description

mdflow transforms your markdown files into executable AI agents. Write prompts in markdown format and execute them against various AI backends by simply running the file like a script.

Key features:

- **Filename patterns** determine which AI backend runs (e.g., `task.claude.md` executes Claude)
- **YAML frontmatter** converts to CLI flags
- **Markdown body** becomes the prompt
- Supports piping, file imports, and template variables
- Interactive and print modes for different use cases

## Installed Tools

| Tool     | Type     | Description                                   |
| -------- | -------- | --------------------------------------------- |
| `mdflow` | cli-tool | CLI for executing markdown files as AI agents |

## Supported Backends

| Backend | File Pattern   | Required Tool |
| ------- | -------------- | ------------- |
| Claude  | `*.claude.md`  | Claude CLI    |
| Codex   | `*.codex.md`   | Codex CLI     |
| Gemini  | `*.gemini.md`  | Gemini CLI    |
| Copilot | `*.copilot.md` | Copilot CLI   |

## Network Requirements

- `registry.npmjs.org` - NPM registry
- `github.com` - GitHub for package metadata

## Installation

```bash
extension-manager install mdflow
```

## Usage

### Basic Usage

```bash
# Execute a markdown file as an AI prompt
mdflow task.claude.md

# Execute with input from stdin
echo "What is the capital of France?" | mdflow prompt.claude.md
```

### Markdown File Format

Create a markdown file with optional YAML frontmatter:

```markdown
---
model: claude-3-opus
temperature: 0.7
---

# My Prompt

Write a function that calculates the fibonacci sequence.
```

### Commands

| Command         | Description                     |
| --------------- | ------------------------------- |
| `mdflow <file>` | Execute markdown file as prompt |
| `mdflow setup`  | Configure shell integration     |
| `mdflow logs`   | Display log directory location  |
| `mdflow help`   | Show help information           |

### Command-Line Flags

Use flags to override frontmatter settings:

```bash
mdflow task.claude.md --temperature 0.9 --max-tokens 2000
```

### Template Variables

Use template variables in your markdown:

```markdown
Analyze the following code:

{{file:./src/main.ts}}
```

## Validation

```bash
# mdflow uses 'help' for validation (does not support --version)
mdflow help

# Verify bun runtime (required by mdflow)
bun --version
```

## Upgrade

**Strategy:** automatic

Automatically upgrades via mise.

## Removal

```bash
extension-manager remove mdflow
```

Removes mise configuration.

## Source Project

- [johnlindquist/mdflow](https://github.com/johnlindquist/mdflow)

## Related Extensions

- [ai-toolkit](AI-TOOLKIT.md) - AI tools suite including Codex, Gemini
- [claudish](CLAUDISH.md) - OpenRouter model proxy for Claude Code
- [agentic-flow](AGENTIC-FLOW.md) - Multi-model AI agent framework
