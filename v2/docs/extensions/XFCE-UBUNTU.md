# XFCE Ubuntu

XFCE desktop with xRDP remote access for GUI development.

## Overview

| Property         | Value                 |
| ---------------- | --------------------- |
| **Category**     | utilities             |
| **Version**      | 2.0.0                 |
| **Installation** | hybrid (apt + script) |
| **Disk Space**   | 2500 MB               |
| **Dependencies** | None                  |

## Description

XFCE desktop with xRDP remote access for GUI development - provides a full graphical desktop environment accessible via RDP (Remote Desktop Protocol).

## Installed Tools

| Tool       | Type      | Description                  |
| ---------- | --------- | ---------------------------- |
| `xfce4`    | framework | XFCE desktop environment     |
| `xrdp`     | server    | RDP server for remote access |
| `firefox`  | utility   | Web browser                  |
| `mousepad` | utility   | Text editor                  |
| `thunar`   | utility   | File manager                 |

### APT Packages

- `xfce4` - Desktop environment
- `xfce4-goodies` - Additional utilities
- `xrdp` - RDP server
- `dbus-x11` - D-Bus X11 integration
- `xfonts-base` - Base fonts
- `xauth` - X authentication
- `x11-xserver-utils` - X server utilities
- `firefox` - Web browser
- `mousepad` - Text editor
- `thunar` - File manager

## Configuration

### Environment Variables

| Variable           | Value | Scope  |
| ------------------ | ----- | ------ |
| `DISPLAY`          | `:0`  | bashrc |
| `XDG_SESSION_TYPE` | `x11` | bashrc |

### Templates

| Template             | Destination                                            | Description           |
| -------------------- | ------------------------------------------------------ | --------------------- |
| `xsession.template`  | `~/.xsession`                                          | X session config      |
| `xfwm4.xml.template` | `~/.config/xfce4/xfconf/xfce-perchannel-xml/xfwm4.xml` | Window manager config |

## Network Requirements

- `archive.ubuntu.com` - Ubuntu packages
- `security.ubuntu.com` - Security updates

## Installation

```bash
extension-manager install xfce-ubuntu
```

## Access

Connect via RDP client to the container's IP on port 3389:

```text
Host: container-ip
Port: 3389
```

## Validation

```bash
startxfce4 --version
xrdp -v              # Expected: xrdp X.X.X
firefox --version    # Expected: Firefox X
mousepad --version
thunar --version
```

## Upgrade

**Strategy:** automatic

Automatically updates all apt packages.

## Removal

### Requires confirmation

```bash
extension-manager remove xfce-ubuntu
```

## Related Extensions

- [guacamole](GUACAMOLE.md) - Web-based remote access
- [tmux-workspace](TMUX-WORKSPACE.md) - Terminal workspace
