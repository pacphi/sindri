# VF-Webapp-Testing

Web app testing framework with element discovery.

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

Web app testing framework with element discovery (from VisionFlow) - provides automated web app testing with element discovery, console logging, and static HTML automation.

## Installed Tools

| Tool             | Type      | Description           |
| ---------------- | --------- | --------------------- |
| `webapp-testing` | framework | Web testing framework |

## Configuration

### Templates

| Template   | Destination                               | Description         |
| ---------- | ----------------------------------------- | ------------------- |
| `SKILL.md` | `~/extensions/vf-webapp-testing/SKILL.md` | Skill documentation |

## Network Requirements

- `registry.npmjs.org` - NPM registry

## Installation

```bash
extension-manager install vf-webapp-testing
```

## Validation

```bash
test -d ~/extensions/vf-webapp-testing
```

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade vf-webapp-testing
```

## Removal

```bash
extension-manager remove vf-webapp-testing
```

Removes:

- `~/extensions/vf-webapp-testing`

## Related Extensions

- [playwright](../PLAYWRIGHT.md) - Playwright automation (required)
- [vf-playwright-mcp](VF-PLAYWRIGHT-MCP.md) - Playwright MCP
- [vf-chrome-devtools](VF-CHROME-DEVTOOLS.md) - Chrome debugging
