# VF-Skill-Creator

Claude Code skill scaffolding tool.

## Overview

| Property         | Value     |
| ---------------- | --------- |
| **Category**     | dev-tools |
| **Version**      | 1.0.0     |
| **Installation** | script    |
| **Disk Space**   | 50 MB     |
| **Memory**       | 128 MB    |
| **Dependencies** | None      |

## Description

Claude Code skill scaffolding tool (from VisionFlow) - generates new Claude Code skills with init, package, and quick validate commands.

## Installed Tools

| Tool            | Type     | Description       |
| --------------- | -------- | ----------------- |
| `skill-creator` | cli-tool | Skill scaffolding |

## Configuration

### Templates

| Template   | Destination                              | Description         |
| ---------- | ---------------------------------------- | ------------------- |
| `SKILL.md` | `~/extensions/vf-skill-creator/SKILL.md` | Skill documentation |

## Network Requirements

None

## Installation

```bash
extension-manager install vf-skill-creator
```

## Validation

```bash
test -d ~/extensions/vf-skill-creator
```

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade vf-skill-creator
```

## Removal

```bash
extension-manager remove vf-skill-creator
```

Removes:

- `~/extensions/vf-skill-creator`

## Related Extensions

- [vf-mcp-builder](VF-MCP-BUILDER.md) - MCP server scaffolding

## Additional Notes

- Templates available in extension directory
- Includes reference skill implementations
