# Monitoring Extension

> Version: 2.0.0 | Category: devops | Last Updated: 2026-01-26

## Overview

Claude monitoring and usage tracking tools including UV, claude-monitor, and claude-usage-cli. Track and manage your Claude API usage.

## What It Provides

| Tool           | Type     | License           | Description                 |
| -------------- | -------- | ----------------- | --------------------------- |
| uv             | cli-tool | MIT OR Apache-2.0 | Fast Python package manager |
| claude-monitor | cli-tool | MIT               | Claude API monitoring tool  |
| claude-usage   | cli-tool | MIT               | Claude usage CLI            |

## Requirements

- **Disk Space**: 200 MB
- **Memory**: 256 MB
- **Install Time**: ~60 seconds
- **Dependencies**: python

### Network Domains

- pypi.org
- astral.sh

## Installation

```bash
extension-manager install monitoring
```

## Configuration

### Install Method

Uses a custom installation script with 600 second timeout.

### Upgrade Strategy

Automatic - run upgrade.sh script.

## Usage Examples

### Claude Monitor

```bash
# Start monitoring
claude-monitor

# Monitor with specific interval
claude-monitor --interval 60

# Export metrics
claude-monitor export --format json

# View dashboard
claude-monitor dashboard
```

### Claude Usage CLI

```bash
# Check usage
claude-usage

# View detailed stats
claude-usage --detailed

# Usage by date range
claude-usage --from 2024-01-01 --to 2024-01-31

# Export usage data
claude-usage export --format csv
```

### UV Package Manager

```bash
# Install packages
uv pip install package-name

# Create virtual environment
uv venv

# Sync requirements
uv pip sync requirements.txt

# Install from requirements
uv pip install -r requirements.txt
```

## Validation

The extension validates the following commands:

- `uv` - Must be available
- `claude-monitor` - Must be available
- `claude-usage` - Must be available

## Removal

```bash
extension-manager remove monitoring
```

This removes:

- ~/.cargo/bin/uv
- ~/.local/bin/claude-monitor
- ~/.local/bin/claude-usage

## Related Extensions

- [python](PYTHON.md) - Required dependency
