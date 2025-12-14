# VF-Blender

Blender 3D modeling with MCP server and addons.

## Overview

| Property         | Value                            |
| ---------------- | -------------------------------- |
| **Category**     | desktop                          |
| **Version**      | 1.0.0                            |
| **Installation** | script                           |
| **Disk Space**   | 1500 MB                          |
| **Memory**       | 4096 MB                          |
| **Dependencies** | [xfce-ubuntu](../XFCE-UBUNTU.md) |
| **GPU**          | Recommended (NVIDIA, 4GB+)       |

## Description

Blender 3D modeling with MCP server and addons (from VisionFlow) - provides 3D modeling, rendering, and animation capabilities with MCP integration. Supports PolyHaven, Sketchfab, and Hyper3D Rodin integrations.

## Installed Tools

| Tool          | Type        | Description            |
| ------------- | ----------- | ---------------------- |
| `blender`     | application | 3D modeling software   |
| `blender-mcp` | server      | MCP server for Blender |

## Configuration

### Templates

| Template               | Destination                                                 | Description         |
| ---------------------- | ----------------------------------------------------------- | ------------------- |
| `SKILL.md`             | `~/extensions/vf-blender/SKILL.md`                          | Skill documentation |
| `blender_mcp_addon.py` | `~/.config/blender/4.0/scripts/addons/blender_mcp_addon.py` | MCP addon           |

## GPU Requirements

- **Recommended**: NVIDIA GPU with 4GB+ VRAM
- **Type**: nvidia
- **Purpose**: Accelerated rendering

## Network Requirements

- `www.blender.org` - Blender downloads
- `download.blender.org` - Blender assets

## Installation

```bash
extension-manager install vf-blender
```

**Note:** Requires desktop environment (xfce-ubuntu) for GUI.

## Validation

```bash
blender --version
```

Expected output pattern: `Blender \d+\.\d+`

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade vf-blender
```

## Removal

```bash
extension-manager remove vf-blender
```

Removes:

- `~/extensions/vf-blender`
- `~/.config/blender/4.0/scripts/addons/blender_mcp*`

## Related Extensions

- [vf-pbr-rendering](VF-PBR-RENDERING.md) - PBR material generation (depends on Blender)
- [vf-qgis](VF-QGIS.md) - GIS operations
- [xfce-ubuntu](../XFCE-UBUNTU.md) - Desktop environment (required)

## Additional Notes

- MCP socket: 9876
- Requires X11/VNC for GUI operations
- GPU acceleration significantly improves render times
