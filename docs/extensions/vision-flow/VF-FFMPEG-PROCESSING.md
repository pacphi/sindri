# VF-FFmpeg-Processing

FFmpeg professional video/audio transcoding.

## Overview

| Property         | Value     |
| ---------------- | --------- |
| **Category**     | utilities |
| **Version**      | 1.0.0     |
| **Installation** | script    |
| **Disk Space**   | 200 MB    |
| **Memory**       | 512 MB    |
| **Dependencies** | None      |

## Description

FFmpeg professional video/audio transcoding (from VisionFlow) - provides comprehensive media processing capabilities for video and audio files.

## Installed Tools

| Tool     | Type     | Description           |
| -------- | -------- | --------------------- |
| `ffmpeg` | cli-tool | Media processing tool |

## Configuration

### Templates

| Template   | Destination                                  | Description         |
| ---------- | -------------------------------------------- | ------------------- |
| `SKILL.md` | `~/extensions/vf-ffmpeg-processing/SKILL.md` | Skill documentation |

## Network Requirements

- `ffmpeg.org` - FFmpeg downloads

## Installation

```bash
extension-manager install vf-ffmpeg-processing
```

## Validation

```bash
ffmpeg -version
```

Expected output pattern: `ffmpeg version`

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade vf-ffmpeg-processing
```

## Removal

```bash
extension-manager remove vf-ffmpeg-processing
```

Removes:

- `~/extensions/vf-ffmpeg-processing`

## Related Extensions

- [vf-slack-gif-creator](VF-SLACK-GIF-CREATOR.md) - GIF generation (uses FFmpeg)
- [vf-comfyui](VF-COMFYUI.md) - Video generation
