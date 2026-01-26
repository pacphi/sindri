# VF-Slack-GIF-Creator

Slack-compatible animated GIF generation.

## Overview

| Property         | Value                                                                   |
| ---------------- | ----------------------------------------------------------------------- |
| **Category**     | utilities                                                               |
| **Version**      | 1.0.0                                                                   |
| **Installation** | script                                                                  |
| **Disk Space**   | 50 MB                                                                   |
| **Memory**       | 256 MB                                                                  |
| **Dependencies** | [nodejs](../NODEJS.md), [vf-ffmpeg-processing](VF-FFMPEG-PROCESSING.md) |

## Description

Slack-compatible animated GIF generation (from VisionFlow) - creates optimized GIFs for Slack with 13 animation templates and global palette optimization.

## Installed Tools

| Tool                | Type     | Description         |
| ------------------- | -------- | ------------------- |
| `slack-gif-creator` | cli-tool | GIF generation tool |

## Configuration

### Templates

| Template   | Destination                                  | Description         |
| ---------- | -------------------------------------------- | ------------------- |
| `SKILL.md` | `~/extensions/vf-slack-gif-creator/SKILL.md` | Skill documentation |

## Network Requirements

- `registry.npmjs.org` - NPM registry

## Installation

```bash
extension-manager install vf-slack-gif-creator
```

## Validation

```bash
test -d ~/extensions/vf-slack-gif-creator
```

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade vf-slack-gif-creator
```

## Removal

```bash
extension-manager remove vf-slack-gif-creator
```

Removes:

- `~/extensions/vf-slack-gif-creator`

## Related Extensions

- [vf-ffmpeg-processing](VF-FFMPEG-PROCESSING.md) - FFmpeg (required)
- [vf-imagemagick](VF-IMAGEMAGICK.md) - Image processing

## Additional Notes

- 13 animation templates available
- Global palette optimization for Slack compatibility
- Requires FFmpeg for video-to-GIF conversion
