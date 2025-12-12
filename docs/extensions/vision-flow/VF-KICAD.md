# VF-KiCad

KiCad PCB design MCP server.

## Overview

| Property         | Value                            |
| ---------------- | -------------------------------- |
| **Category**     | dev-tools                        |
| **Version**      | 1.0.0                            |
| **Installation** | script                           |
| **Disk Space**   | 800 MB                           |
| **Memory**       | 1024 MB                          |
| **Dependencies** | [xfce-ubuntu](../XFCE-UBUNTU.md) |

## Description

KiCad PCB design MCP server (from VisionFlow) - provides PCB design, schematic capture, and Gerber export capabilities.

## Installed Tools

| Tool        | Type        | Description      |
| ----------- | ----------- | ---------------- |
| `kicad`     | application | PCB design suite |
| `kicad-cli` | cli-tool    | KiCad CLI        |

## Configuration

### Templates

| Template   | Destination                      | Description         |
| ---------- | -------------------------------- | ------------------- |
| `SKILL.md` | `~/extensions/vf-kicad/SKILL.md` | Skill documentation |

## Network Requirements

- `kicad.org` - KiCad downloads

## Installation

```bash
extension-manager install vf-kicad
```

**Note:** Requires desktop environment (xfce-ubuntu) for GUI.

## Validation

```bash
kicad-cli --version
```

Expected output pattern: `\d+\.\d+`

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade vf-kicad
```

## Removal

```bash
extension-manager remove vf-kicad
```

Removes:

- `~/extensions/vf-kicad`

## Related Extensions

- [vf-ngspice](VF-NGSPICE.md) - Circuit simulation
- [xfce-ubuntu](../XFCE-UBUNTU.md) - Desktop environment (required)

## Additional Notes

- Requires X11/VNC for GUI operations
- Includes KiCad libraries for components
- Supports schematic and PCB design
