# Monitoring

Claude monitoring and usage tracking tools.

## Overview

| Property         | Value               |
| ---------------- | ------------------- |
| **Category**     | infrastructure      |
| **Version**      | 2.0.0               |
| **Installation** | script              |
| **Disk Space**   | 200 MB              |
| **Dependencies** | [python](PYTHON.md) |

## Description

Claude monitoring and usage tracking tools (UV, claude-monitor, claude-usage-cli) - provides tools for tracking Claude API usage, costs, and monitoring deployments.

## Installed Tools

| Tool             | Type     | Description                             |
| ---------------- | -------- | --------------------------------------- |
| `uv`             | cli-tool | Fast Python package installer by Astral |
| `claude-monitor` | cli-tool | Monitor Claude API usage and cost       |
| `claude-usage`   | cli-tool | Track Claude API consumption            |

## Network Requirements

- `pypi.org` - Python packages
- `astral.sh` - UV installer

## Installation

```bash
extension-manager install monitoring
```

## Usage

```bash
# Monitor Claude usage
claude-monitor

# Track API consumption
claude-usage report

# Use UV for fast Python installs
uv pip install package-name
```

## Validation

```bash
uv --version
claude-monitor --version
claude-usage --version
```

## Upgrade

**Strategy:** automatic

```bash
extension-manager upgrade monitoring
```

## Removal

```bash
extension-manager remove monitoring
```

Removes:

- `~/.cargo/bin/uv`
- `~/.local/bin/claude-monitor`
- `~/.local/bin/claude-usage`

## Related Extensions

- [python](PYTHON.md) - Required dependency
- [ai-toolkit](AI-TOOLKIT.md) - AI tools
