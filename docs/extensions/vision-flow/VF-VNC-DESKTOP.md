# VF-VNC-Desktop

VNC desktop server with terminal grid.

## Overview

| Property         | Value   |
| ---------------- | ------- |
| **Category**     | desktop |
| **Version**      | 1.0.0   |
| **Installation** | script  |
| **Disk Space**   | 500 MB  |
| **Memory**       | 1024 MB |
| **Dependencies** | None    |

## Description

VNC desktop server with terminal grid (from VisionFlow) - provides VNC remote desktop with 9 color-coded terminals in a 3x3 grid layout.

## Installed Tools

| Tool             | Type     | Description         |
| ---------------- | -------- | ------------------- |
| `x11vnc`         | server   | VNC server          |
| `xvfb`           | server   | Virtual framebuffer |
| `openbox`        | wm       | Window manager      |
| `tint2`          | panel    | Desktop panel       |
| `xfce4-terminal` | terminal | Terminal emulator   |

## Configuration

### Templates

| Template           | Destination                                    | Description       |
| ------------------ | ---------------------------------------------- | ----------------- |
| `supervisord.conf` | `~/extensions/vf-vnc-desktop/supervisord.conf` | Supervisor config |

## Network Requirements

None

## Installation

```bash
extension-manager install vf-vnc-desktop
```

## Validation

```bash
x11vnc -version
```

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade vf-vnc-desktop
```

## Removal

```bash
extension-manager remove vf-vnc-desktop
```

Removes:

- `~/extensions/vf-vnc-desktop`

## Related Extensions

- [xfce-ubuntu](../XFCE-UBUNTU.md) - XFCE desktop
- [guacamole](../GUACAMOLE.md) - Web-based gateway

## Additional Notes

- VNC port: 5901
- 9 color-coded terminals in 3x3 grid
- Display: 2560x1440 @ 24-bit color
- No password authentication (configure for production)
