# VF-ImageMagick

ImageMagick image processing with MCP server.

## Overview

| Property         | Value     |
| ---------------- | --------- |
| **Category**     | utilities |
| **Version**      | 1.0.0     |
| **Installation** | script    |
| **Disk Space**   | 100 MB    |
| **Dependencies** | None      |

## Description

ImageMagick image processing with MCP server (from VisionFlow) - provides comprehensive image manipulation capabilities including format conversion, resizing, filtering, and batch operations.

## Installed Tools

| Tool              | Type     | Description                |
| ----------------- | -------- | -------------------------- |
| `convert`         | cli-tool | Image format converter     |
| `imagemagick-mcp` | server   | MCP server for ImageMagick |

## Configuration

### Templates

| Template   | Destination                            | Description         |
| ---------- | -------------------------------------- | ------------------- |
| `SKILL.md` | `~/extensions/vf-imagemagick/SKILL.md` | Skill documentation |

## Network Requirements

- `imagemagick.org` - ImageMagick downloads

## Installation

```bash
extension-manager install vf-imagemagick
```

## Validation

```bash
convert --version
```

Expected output pattern: `ImageMagick`

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade vf-imagemagick
```

## Removal

```bash
extension-manager remove vf-imagemagick
```

Removes:

- `~/extensions/vf-imagemagick`

## Related Extensions

- [vf-slack-gif-creator](VF-SLACK-GIF-CREATOR.md) - Uses ImageMagick for GIF creation
- [vf-ffmpeg-processing](VF-FFMPEG-PROCESSING.md) - Media processing
