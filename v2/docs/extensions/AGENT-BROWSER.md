# Agent Browser

Headless browser automation CLI for AI agents with snapshot-based element selection.

## Overview

| Property         | Value                                            |
| ---------------- | ------------------------------------------------ |
| **Category**     | dev-tools                                        |
| **Version**      | 0.6.0                                            |
| **Installation** | mise                                             |
| **Disk Space**   | 200 MB                                           |
| **Dependencies** | [nodejs](NODEJS.md), [playwright](PLAYWRIGHT.md) |

## Description

agent-browser provides a fast, AI-friendly headless browser automation CLI built on Playwright. It uses snapshot-based accessibility trees with ref-based element selection (@e1, @e2) for reliable browser automation, form filling, screenshot capture, and web data extraction.

**Key Features:**

- Snapshot-based accessibility tree with AI-friendly refs
- Fast Rust CLI with Node.js fallback
- Comprehensive browser automation (click, fill, navigate, wait)
- Screenshot, PDF, and video recording
- Multi-tab and session support
- Network interception and mocking
- Cookie and storage management
- WebSocket streaming for live preview
- Claude Code skill integration

## Installed Tools

| Tool            | Type | Description                      |
| --------------- | ---- | -------------------------------- |
| `agent-browser` | cli  | Browser automation for AI agents |

## Configuration

### Templates

| Template             | Destination                                    | Description            |
| -------------------- | ---------------------------------------------- | ---------------------- |
| `resources/SKILL.md` | `/workspace/extensions/agent-browser/SKILL.md` | Claude Code skill file |

### Environment Variables

| Variable                        | Value                                                   | Description          |
| ------------------------------- | ------------------------------------------------------- | -------------------- |
| `AGENT_BROWSER_EXECUTABLE_PATH` | `~/.cache/ms-playwright/chromium-*/chrome-linux/chrome` | Shared Chromium path |

## Network Requirements

- `registry.npmjs.org` - NPM registry
- `playwright.azureedge.net` - Playwright browsers (via playwright extension)

## Installation

```bash
extension-manager install agent-browser
```

This installs agent-browser globally via npm and configures it to use the Chromium browser provided by the playwright extension.

## Usage Examples

### Basic Workflow

```bash
# Navigate to a page
agent-browser open https://example.com

# Get snapshot with interactive elements
agent-browser snapshot -i

# Interact using refs from snapshot
agent-browser fill @e1 "test@example.com"
agent-browser click @e2

# Take screenshot
agent-browser screenshot result.png
```

### Form Automation

```bash
agent-browser open https://example.com/form
agent-browser snapshot -i
# Output: textbox "Email" [ref=e1], textbox "Password" [ref=e2], button "Submit" [ref=e3]

agent-browser fill @e1 "user@example.com"
agent-browser fill @e2 "password123"
agent-browser click @e3
agent-browser wait --load networkidle
```

### Recording Demo Videos

```bash
agent-browser open https://example.com
agent-browser record start demo.webm
# Perform actions...
agent-browser click @e1
agent-browser fill @e2 "data"
agent-browser record stop
```

### Session Management

```bash
# Save authentication state
agent-browser state save auth.json

# Later: load saved state
agent-browser state load auth.json
agent-browser open https://app.example.com/dashboard
```

## Validation

```bash
agent-browser --version
```

Expected output: Version number (e.g., `0.6.0`)

## Upgrade

**Strategy:** automatic

```bash
extension-manager upgrade agent-browser
```

## Removal

```bash
extension-manager remove agent-browser
```

Removes:

- `agent-browser` npm package
- `/workspace/extensions/agent-browser` directory

## Source Project

- **Repository:** [vercel-labs/agent-browser](https://github.com/vercel-labs/agent-browser)
- **License:** Apache-2.0
- **PURL:** `pkg:npm/agent-browser@0.6.0`

## Related Extensions

- [playwright](PLAYWRIGHT.md) - Provides the Chromium browser used by agent-browser
- [agentic-qe](AGENTIC-QE.md) - AI-powered test generation framework
