# Tmux Workspace Extension

> Version: 2.0.0 | Category: productivity | Last Updated: 2026-01-26

## Overview

Tmux workspace management with helper scripts and auto-start functionality. Provides terminal multiplexing for efficient development workflows.

## What It Provides

| Tool | Type     | License | Description                |
| ---- | -------- | ------- | -------------------------- |
| tmux | cli-tool | ISC     | Terminal multiplexer       |
| htop | cli-tool | GPL-2.0 | Interactive process viewer |

## Requirements

- **Disk Space**: 50 MB
- **Memory**: 64 MB
- **Install Time**: ~30 seconds
- **Dependencies**: None

## Installation

```bash
sindri extension install tmux-workspace
```

## Configuration

### Environment Variables

| Variable            | Value            | Description          |
| ------------------- | ---------------- | -------------------- |
| `TMUX_SESSION_NAME` | sindri-workspace | Default session name |

### Templates

| Template                    | Destination                  | Description        |
| --------------------------- | ---------------------------- | ------------------ |
| tmux.conf.template          | ~/config/tmux.conf           | Tmux configuration |
| tmux-workspace.sh.template  | ~/scripts/tmux-workspace.sh  | Workspace script   |
| tmux-helpers.sh.template    | ~/scripts/tmux-helpers.sh    | Helper functions   |
| tmux-auto-start.sh.template | ~/scripts/tmux-auto-start.sh | Auto-start script  |

### Install Method

Uses apt packages (tmux requires ncurses, libevent system libraries).

### Post-Install

- Creates tmux config symlink: `~/.tmux.conf -> ~/config/tmux.conf`
- Sources tmux helpers in bashrc
- Adds tmux scripts to PATH

## Usage Examples

### Basic Tmux

```bash
# Start tmux
tmux

# Create named session
tmux new -s dev

# Attach to session
tmux attach -t dev

# List sessions
tmux ls

# Kill session
tmux kill-session -t dev
```

### Window Management

```bash
# Inside tmux:
# Create new window: Ctrl-b c
# Next window: Ctrl-b n
# Previous window: Ctrl-b p
# Select window: Ctrl-b [0-9]
# Rename window: Ctrl-b ,
```

### Pane Management

```bash
# Split horizontally: Ctrl-b "
# Split vertically: Ctrl-b %
# Switch panes: Ctrl-b arrow-keys
# Close pane: Ctrl-b x
# Resize: Ctrl-b Ctrl-arrow
```

### Helper Scripts

```bash
# Start workspace
tmux-workspace.sh

# Auto-start on login
source ~/scripts/tmux-auto-start.sh
```

### Htop

```bash
# View processes
htop

# Inside htop:
# F5 - Tree view
# F6 - Sort by
# F9 - Kill process
# F10 - Quit
```

### Custom Configuration

```bash
# Edit tmux config
vim ~/config/tmux.conf

# Reload config
tmux source-file ~/.tmux.conf
```

## Validation

The extension validates the following commands:

- `tmux -V` - Must match pattern `tmux \d+`
- `htop --version` - Must be available

## Removal

```bash
sindri extension remove tmux-workspace
```

**Requires confirmation.** Removes:

- ~/config/tmux.conf
- ~/scripts/tmux-workspace.sh
- ~/scripts/tmux-helpers.sh
- ~/scripts/tmux-auto-start.sh
- ~/.tmux.conf

## Related Extensions

None - Tmux Workspace is a standalone productivity tool.
