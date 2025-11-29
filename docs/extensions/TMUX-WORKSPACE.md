# Tmux Workspace

Tmux workspace management with helper scripts and auto-start functionality.

## Overview

| Property         | Value     |
| ---------------- | --------- |
| **Category**     | dev-tools |
| **Version**      | 2.0.0     |
| **Installation** | apt       |
| **Disk Space**   | 50 MB     |
| **Dependencies** | None      |

## Description

Tmux workspace management with helper scripts and auto-start functionality - provides terminal multiplexing and workspace organization for persistent development sessions.

## Installed Tools

| Tool   | Type     | Description          |
| ------ | -------- | -------------------- |
| `tmux` | cli-tool | Terminal multiplexer |
| `htop` | cli-tool | Process viewer       |

## Configuration

### Environment Variables

| Variable            | Value              | Scope   |
| ------------------- | ------------------ | ------- |
| `TMUX_SESSION_NAME` | `sindri-workspace` | profile |

### Templates

| Template                      | Destination                             | Permissions | Description          |
| ----------------------------- | --------------------------------------- | ----------- | -------------------- |
| `tmux.conf.template`          | `/workspace/config/tmux.conf`           | -           | Tmux configuration   |
| `tmux-workspace.sh.template`  | `/workspace/scripts/tmux-workspace.sh`  | 755         | Workspace management |
| `tmux-helpers.sh.template`    | `/workspace/scripts/tmux-helpers.sh`    | 755         | Helper functions     |
| `tmux-auto-start.sh.template` | `/workspace/scripts/tmux-auto-start.sh` | 755         | Auto-start script    |

### Post-Install Configuration

- Creates symlink: `~/.tmux.conf` → `/workspace/config/tmux.conf`
- Sources tmux helpers in bashrc
- Adds `/workspace/scripts` to PATH

## Installation

```bash
extension-manager install tmux-workspace
```

## Usage

### Workspace Commands

```bash
# Create a new workspace
tw-create myproject

# Switch to a workspace
tw-switch myproject

# List all workspaces
tw-list

# Auto-start tmux on login
tmux attach || tmux new-session
```

### Tmux Basics

```bash
# Start new session
tmux new -s session-name

# Attach to session
tmux attach -t session-name

# List sessions
tmux ls

# Detach (inside tmux)
Ctrl+b d
```

## Validation

```bash
tmux -V    # Expected: tmux X.X
htop --version
```

## Upgrade

**Strategy:** manual

## Removal

### Requires confirmation

```bash
extension-manager remove tmux-workspace
```

Removes:

- `/workspace/config/tmux.conf`
- `/workspace/scripts/tmux-*.sh`
- `~/.tmux.conf`

## Related Extensions

- [xfce-ubuntu](XFCE-UBUNTU.md) - Desktop environment
