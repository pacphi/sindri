# VF-PBR-Rendering

PBR material generation MCP server.

## Overview

| Property         | Value                       |
| ---------------- | --------------------------- |
| **Category**     | desktop                     |
| **Version**      | 1.0.0                       |
| **Installation** | script                      |
| **Disk Space**   | 500 MB                      |
| **Memory**       | 4096 MB                     |
| **Dependencies** | [vf-blender](VF-BLENDER.md) |
| **GPU**          | Required (NVIDIA, 4GB+)     |

## Description

PBR material generation MCP server (from VisionFlow) - provides physically-based rendering material generation using nvdiffrast.

## Installed Tools

| Tool            | Type   | Description            |
| --------------- | ------ | ---------------------- |
| `pbr-rendering` | server | PBR material generator |

## Configuration

### Templates

| Template   | Destination                              | Description         |
| ---------- | ---------------------------------------- | ------------------- |
| `SKILL.md` | `~/extensions/vf-pbr-rendering/SKILL.md` | Skill documentation |

## GPU Requirements

- **Required**: Yes (NVIDIA GPU with 4GB+ VRAM)
- **Type**: nvidia
- **Min Memory**: 4096 MB

## Network Requirements

- `pypi.org` - Python packages

## Installation

```bash
extension-manager install vf-pbr-rendering
```

**Note:** Requires GPU and Blender extension.

## Validation

```bash
test -d ~/extensions/vf-pbr-rendering
```

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade vf-pbr-rendering
```

## Removal

```bash
extension-manager remove vf-pbr-rendering
```

Removes:

- `~/extensions/vf-pbr-rendering`

## Related Extensions

- [vf-blender](VF-BLENDER.md) - Blender 3D (required)
- [vf-comfyui](VF-COMFYUI.md) - Image generation

## Additional Notes

- MCP socket: 9878
- Requires CUDA for nvdiffrast
- Depends on Blender for 3D operations
