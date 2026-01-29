# VF-NGSpice

NGSpice circuit simulation with MCP server.

## Overview

| Property         | Value     |
| ---------------- | --------- |
| **Category**     | dev-tools |
| **Version**      | 1.0.0     |
| **Installation** | script    |
| **Disk Space**   | 100 MB    |
| **Memory**       | 256 MB    |
| **Dependencies** | None      |

## Description

NGSpice circuit simulation with MCP server (from VisionFlow) - provides circuit simulation with netlist parsing capabilities.

## Installed Tools

| Tool      | Type     | Description       |
| --------- | -------- | ----------------- |
| `ngspice` | cli-tool | Circuit simulator |

## Configuration

### Templates

| Template   | Destination                        | Description         |
| ---------- | ---------------------------------- | ------------------- |
| `SKILL.md` | `~/extensions/vf-ngspice/SKILL.md` | Skill documentation |

## Network Requirements

- `ngspice.sourceforge.io` - NGSpice downloads

## Installation

```bash
extension-manager install vf-ngspice
```

## Validation

```bash
ngspice --version
```

Expected output pattern: `ngspice`

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade vf-ngspice
```

## Removal

```bash
extension-manager remove vf-ngspice
```

Removes:

- `~/extensions/vf-ngspice`

## Related Extensions

- [vf-kicad](VF-KICAD.md) - PCB design
