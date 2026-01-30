# Agent Browser Extension

> Version: 0.6.0 | Category: ai-agents | Last Updated: 2026-01-26

## Overview

Headless browser automation CLI for AI agents with snapshot-based element selection. Built by Vercel Labs for reliable browser automation.

## What It Provides

| Tool          | Type     | License    | Description               |
| ------------- | -------- | ---------- | ------------------------- |
| agent-browser | cli-tool | Apache-2.0 | AI browser automation CLI |

## Requirements

- **Disk Space**: 200 MB
- **Memory**: 512 MB
- **Install Time**: ~120 seconds
- **Install Timeout**: 300 seconds
- **Dependencies**: nodejs, playwright

### Network Domains

- registry.npmjs.org
- playwright.azureedge.net

## Installation

```bash
sindri extension install agent-browser
```

## Configuration

### Environment Variables

| Variable                        | Value                                                  | Description  |
| ------------------------------- | ------------------------------------------------------ | ------------ |
| `AGENT_BROWSER_EXECUTABLE_PATH` | ~/.cache/ms-playwright/chromium-\*/chrome-linux/chrome | Browser path |

### Templates

- resources/SKILL.md - Claude Code skill at ~/extensions/agent-browser/SKILL.md

### Install Method

Uses mise for tool management.

### Upgrade Strategy

Automatic via mise upgrade.

## Key Features

- **Snapshot-based Selection** - Reliable element targeting
- **Headless Operation** - Run without visible browser
- **AI Integration** - Designed for AI agent workflows
- **Action Recording** - Capture and replay interactions

## Usage Examples

### Basic Usage

```bash
# Check version
agent-browser --version

# Navigate to URL
agent-browser navigate https://example.com

# Take screenshot
agent-browser screenshot --output page.png
```

### Element Interaction

```bash
# Click element
agent-browser click --selector "button.submit"

# Type text
agent-browser type --selector "input.email" --text "user@example.com"

# Select from dropdown
agent-browser select --selector "select.country" --value "US"
```

### Snapshot Mode

```bash
# Take snapshot for AI analysis
agent-browser snapshot --output snapshot.json

# Analyze page structure
agent-browser analyze --url https://example.com
```

### Scripted Automation

```bash
# Run script
agent-browser run script.json

# Example script.json:
# {
#   "steps": [
#     { "action": "navigate", "url": "https://example.com" },
#     { "action": "click", "selector": "button.login" },
#     { "action": "type", "selector": "input.email", "text": "user@example.com" },
#     { "action": "screenshot", "output": "result.png" }
#   ]
# }
```

## Claude Code Integration

The extension includes a Claude Code skill for browser automation tasks:

```
~/extensions/agent-browser/SKILL.md
```

This enables Claude Code to perform browser automation as part of AI workflows.

## Validation

The extension validates the following commands:

- `agent-browser --version` - Must match pattern `\d+\.\d+\.\d+`

## Removal

```bash
sindri extension remove agent-browser
```

**Requires confirmation.** Removes:

- mise agent-browser tools
- ~/extensions/agent-browser

## Related Extensions

- [nodejs](NODEJS.md) - Required dependency
- [playwright](PLAYWRIGHT.md) - Required dependency (provides browser binaries)
