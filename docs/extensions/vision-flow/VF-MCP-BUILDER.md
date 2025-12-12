# VF-MCP-Builder

MCP server scaffolding and generation tool.

## Overview

| Property         | Value                  |
| ---------------- | ---------------------- |
| **Category**     | dev-tools              |
| **Version**      | 1.0.0                  |
| **Installation** | script                 |
| **Disk Space**   | 50 MB                  |
| **Memory**       | 128 MB                 |
| **Dependencies** | [nodejs](../NODEJS.md) |

## Description

MCP server scaffolding and generation tool (from VisionFlow) - helps create new MCP servers with templates and reference files.

## Installed Tools

| Tool          | Type     | Description            |
| ------------- | -------- | ---------------------- |
| `mcp-builder` | cli-tool | MCP server scaffolding |

## Configuration

### Templates

| Template   | Destination                            | Description         |
| ---------- | -------------------------------------- | ------------------- |
| `SKILL.md` | `~/extensions/vf-mcp-builder/SKILL.md` | Skill documentation |

## Network Requirements

- `registry.npmjs.org` - NPM registry

## Installation

```bash
extension-manager install vf-mcp-builder
```

## Validation

```bash
test -d ~/extensions/vf-mcp-builder
```

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade vf-mcp-builder
```

## Removal

```bash
extension-manager remove vf-mcp-builder
```

Removes:

- `~/extensions/vf-mcp-builder`

## Related Extensions

- [vf-skill-creator](VF-SKILL-CREATOR.md) - Skill scaffolding

## Additional Notes

- Templates available in extension directory
- Includes reference MCP server implementations
