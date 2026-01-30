# VF-Chrome-DevTools

Chrome DevTools Protocol MCP server.

## Overview

| Property         | Value                  |
| ---------------- | ---------------------- |
| **Category**     | dev-tools              |
| **Version**      | 1.0.0                  |
| **Installation** | script                 |
| **Disk Space**   | 100 MB                 |
| **Memory**       | 256 MB                 |
| **Dependencies** | [nodejs](../NODEJS.md) |

## Description

Chrome DevTools Protocol MCP server (from VisionFlow) - provides web debugging capabilities via Chrome DevTools Protocol integration.

## Installed Tools

| Tool                  | Type   | Description                  |
| --------------------- | ------ | ---------------------------- |
| `chrome-devtools-mcp` | server | Chrome DevTools Protocol MCP |

## Configuration

### Templates

| Template   | Destination                                | Description         |
| ---------- | ------------------------------------------ | ------------------- |
| `SKILL.md` | `~/extensions/vf-chrome-devtools/SKILL.md` | Skill documentation |

## Network Requirements

- `registry.npmjs.org` - NPM registry

## Installation

```bash
extension-manager install vf-chrome-devtools
```

## Validation

```bash
test -d ~/extensions/vf-chrome-devtools
```

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade vf-chrome-devtools
```

## Removal

```bash
extension-manager remove vf-chrome-devtools
```

Removes:

- `~/extensions/vf-chrome-devtools`

## Related Extensions

- [vf-playwright-mcp](VF-PLAYWRIGHT-MCP.md) - Browser automation
- [vf-webapp-testing](VF-WEBAPP-TESTING.md) - Web app testing

## Additional Notes

- Chrome DevTools Protocol port: 9222
- Provides debugging and profiling capabilities
