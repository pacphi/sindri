# VF-ComfyUI

ComfyUI node-based image generation MCP server.

## Overview

| Property         | Value                   |
| ---------------- | ----------------------- |
| **Category**     | ai                      |
| **Version**      | 1.0.0                   |
| **Installation** | script                  |
| **Disk Space**   | 5000 MB                 |
| **Memory**       | 8192 MB                 |
| **Dependencies** | [python](../PYTHON.md)  |
| **GPU**          | Required (NVIDIA, 8GB+) |

## Description

ComfyUI node-based image generation MCP server (from VisionFlow) - provides powerful node-based workflows for AI image and video generation using Stable Diffusion, FLUX, and SAM3D integration.

## Installed Tools

| Tool      | Type   | Description                 |
| --------- | ------ | --------------------------- |
| `comfyui` | server | Node-based image generation |

## Configuration

### Environment Variables

| Variable               | Value | Scope  |
| ---------------------- | ----- | ------ |
| `CUDA_VISIBLE_DEVICES` | `0`   | bashrc |

### Templates

| Template   | Destination                        | Description         |
| ---------- | ---------------------------------- | ------------------- |
| `SKILL.md` | `~/extensions/vf-comfyui/SKILL.md` | Skill documentation |

## GPU Requirements

- **Required**: Yes (NVIDIA GPU with 8GB+ VRAM)
- **Type**: nvidia
- **Min Memory**: 8192 MB
- **Purpose**: AI model inference

## Network Requirements

- `pypi.org` - Python packages
- `huggingface.co` - Model downloads
- `github.com` - ComfyUI repository

## Installation

```bash
extension-manager install vf-comfyui
```

**Note:** Requires GPU. Installation includes PyTorch with CUDA support.

## Validation

```bash
test -d ~/extensions/vf-comfyui
```

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade vf-comfyui
```

## Removal

```bash
extension-manager remove vf-comfyui
```

Removes:

- `~/extensions/vf-comfyui`
- `~/.cache/comfyui`

## Related Extensions

- [vf-pytorch-ml](VF-PYTORCH-ML.md) - PyTorch framework
- [vf-ffmpeg-processing](VF-FFMPEG-PROCESSING.md) - Video processing

## Additional Notes

- Port: 8188 (WebSocket)
- Includes FLUX2 workflows for phase-1 generation and phase-2 SAM3D conversion
- Model cache requires significant disk space (5GB+)
