# VF-Wardley-Maps

Strategic Wardley mapping visualization tool.

## Overview

| Property         | Value                  |
| ---------------- | ---------------------- |
| **Category**     | utilities              |
| **Version**      | 1.0.0                  |
| **Installation** | script                 |
| **Disk Space**   | 100 MB                 |
| **Memory**       | 256 MB                 |
| **Dependencies** | [nodejs](../NODEJS.md) |

## Description

Strategic Wardley mapping visualization tool (from VisionFlow) - provides 7 modules for strategic analysis including mapper, analyzer, parser, and heuristics engine for component positioning.

## Installed Tools

| Tool           | Type     | Description       |
| -------------- | -------- | ----------------- |
| `wardley-maps` | cli-tool | Strategic mapping |

## Configuration

### Templates

| Template   | Destination                             | Description         |
| ---------- | --------------------------------------- | ------------------- |
| `SKILL.md` | `~/extensions/vf-wardley-maps/SKILL.md` | Skill documentation |

## Network Requirements

- `registry.npmjs.org` - NPM registry

## Installation

```bash
extension-manager install vf-wardley-maps
```

## Validation

```bash
test -d ~/extensions/vf-wardley-maps
```

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade vf-wardley-maps
```

## Removal

```bash
extension-manager remove vf-wardley-maps
```

Removes:

- `~/extensions/vf-wardley-maps`

## Additional Notes

- Generates HTML/D3.js visualizations
- Includes SWOT analysis and strategic recommendations
- Supports Mermaid and SVG output formats
