# XFCE Ubuntu Extension

> Version: 2.0.0 | Category: desktop | Last Updated: 2026-01-26

## Overview

XFCE desktop with xRDP remote access for GUI development. Provides a lightweight desktop environment accessible via RDP.

## What It Provides

| Tool     | Type      | License    | Description              |
| -------- | --------- | ---------- | ------------------------ |
| xfce4    | framework | GPL-2.0    | XFCE desktop environment |
| xrdp     | server    | Apache-2.0 | RDP server               |
| firefox  | utility   | MPL-2.0    | Web browser              |
| mousepad | utility   | GPL-2.0    | Text editor              |
| thunar   | utility   | GPL-2.0    | File manager             |

## Requirements

- **Disk Space**: 2500 MB (2.5 GB)
- **Memory**: 1024 MB
- **Install Time**: ~300 seconds (5 minutes)
- **Dependencies**: None

### Network Domains

- archive.ubuntu.com
- security.ubuntu.com

## Installation

```bash
extension-manager install xfce-ubuntu
```

## Configuration

### Environment Variables

| Variable           | Value | Description  |
| ------------------ | ----- | ------------ |
| `DISPLAY`          | :0    | X display    |
| `XDG_SESSION_TYPE` | x11   | Session type |

### Templates

| Template           | Destination                                          | Description           |
| ------------------ | ---------------------------------------------------- | --------------------- |
| xsession.template  | ~/.xsession                                          | X session config      |
| xfwm4.xml.template | ~/.config/xfce4/xfconf/xfce-perchannel-xml/xfwm4.xml | Window manager config |

### Install Method

Hybrid installation with apt packages and post-install script.

### APT Packages

- xfce4, xfce4-goodies
- xrdp
- dbus-x11, xfonts-base, xauth, x11-xserver-utils
- firefox, mousepad, thunar

### Upgrade Strategy

Automatic via apt packages.

## Key Features

- **Lightweight** - Low resource usage
- **RDP Access** - Connect from Windows/Mac/Linux
- **Full Desktop** - Complete GUI environment
- **Development Ready** - GUI development and testing

## Usage Examples

### Connecting via RDP

```bash
# Default RDP port: 3389
# From Windows: mstsc.exe
# From Mac: Microsoft Remote Desktop
# From Linux: rdesktop, freerdp, or Remmina

# Connect to:
# hostname:3389
```

### Starting XFCE

```bash
# Start manually
startxfce4

# Via xRDP (automatic on RDP connection)
# Configure in ~/.xsession
```

### Service Management

```bash
# Start xRDP
sudo systemctl start xrdp

# Check status
sudo systemctl status xrdp

# Enable on boot
sudo systemctl enable xrdp
```

### Desktop Applications

```bash
# File manager
thunar

# Text editor
mousepad file.txt

# Web browser
firefox

# Terminal
xfce4-terminal
```

### Customization

```bash
# XFCE settings
xfce4-settings-manager

# Display settings
xfce4-display-settings

# Panel preferences
xfce4-panel --preferences
```

## Validation

The extension validates the following commands:

- `startxfce4` - Must be available
- `xrdp -v` - Must match pattern `xrdp\s+\d+\.\d+\.\d+`
- `firefox --version` - Must match pattern `Firefox\s+\d+`
- `mousepad --version` - Must be available
- `thunar --version` - Must be available

## Removal

```bash
extension-manager remove xfce-ubuntu
```

**Requires confirmation.** Removes:

- XFCE packages
- xRDP
- ~/.xsession
- ~/.config/xfce4

## Related Extensions

- [guacamole](GUACAMOLE.md) - Web-based remote access
