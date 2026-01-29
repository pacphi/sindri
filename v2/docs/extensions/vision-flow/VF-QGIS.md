# VF-QGIS

QGIS GIS operations MCP server.

## Overview

| Property         | Value                            |
| ---------------- | -------------------------------- |
| **Category**     | desktop                          |
| **Version**      | 1.0.0                            |
| **Installation** | script                           |
| **Disk Space**   | 1000 MB                          |
| **Memory**       | 2048 MB                          |
| **Dependencies** | [xfce-ubuntu](../XFCE-UBUNTU.md) |

## Description

QGIS GIS operations MCP server (from VisionFlow) - provides geospatial analysis and mapping capabilities via QGIS desktop application.

## Installed Tools

| Tool   | Type        | Description  |
| ------ | ----------- | ------------ |
| `qgis` | application | GIS software |

## Configuration

### Templates

| Template   | Destination                     | Description         |
| ---------- | ------------------------------- | ------------------- |
| `SKILL.md` | `~/extensions/vf-qgis/SKILL.md` | Skill documentation |

## Network Requirements

- `qgis.org` - QGIS downloads

## Installation

```bash
extension-manager install vf-qgis
```

**Note:** Requires desktop environment (xfce-ubuntu) for GUI.

## Validation

```bash
qgis --version
```

Expected output pattern: `QGIS`

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade vf-qgis
```

## Removal

```bash
extension-manager remove vf-qgis
```

Removes:

- `~/extensions/vf-qgis`

## Related Extensions

- [vf-blender](VF-BLENDER.md) - 3D modeling
- [xfce-ubuntu](../XFCE-UBUNTU.md) - Desktop environment (required)

## Additional Notes

- MCP socket: 9877
- Requires X11/VNC for GUI operations
- Includes GRASS plugin for advanced GIS
