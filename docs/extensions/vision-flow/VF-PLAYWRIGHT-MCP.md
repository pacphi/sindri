# VF-Playwright-MCP

Playwright browser automation MCP server.

## Overview

| Property         | Value                                                  |
| ---------------- | ------------------------------------------------------ |
| **Category**     | dev-tools                                              |
| **Version**      | 1.0.0                                                  |
| **Installation** | script                                                 |
| **Disk Space**   | 200 MB                                                 |
| **Memory**       | 512 MB                                                 |
| **Dependencies** | [nodejs](../NODEJS.md), [playwright](../PLAYWRIGHT.md) |

## Description

Playwright browser automation MCP server (from VisionFlow) - extends the Playwright extension with MCP integration for browser automation tasks.

## Installed Tools

| Tool             | Type   | Description               |
| ---------------- | ------ | ------------------------- |
| `playwright-mcp` | server | MCP server for Playwright |

## Configuration

### Templates

| Template   | Destination                               | Description         |
| ---------- | ----------------------------------------- | ------------------- |
| `SKILL.md` | `~/extensions/vf-playwright-mcp/SKILL.md` | Skill documentation |

## Network Requirements

- `registry.npmjs.org` - NPM registry
- `playwright.azureedge.net` - Playwright browsers

## Installation

```bash
extension-manager install vf-playwright-mcp
```

## Validation

```bash
test -d ~/extensions/vf-playwright-mcp/mcp-server
```

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade vf-playwright-mcp
```

## Removal

```bash
extension-manager remove vf-playwright-mcp
```

Removes:

- `~/extensions/vf-playwright-mcp`

## Related Extensions

- [playwright](../PLAYWRIGHT.md) - Base Playwright extension (required)
- [vf-webapp-testing](VF-WEBAPP-TESTING.md) - Web app testing framework
- [vf-chrome-devtools](VF-CHROME-DEVTOOLS.md) - Chrome debugging
