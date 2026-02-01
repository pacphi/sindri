# Spec-Kit Extension

> Version: 1.0.0 | Category: documentation | Last Updated: 2026-01-26

## Overview

GitHub specification kit for AI-powered repository documentation and workflows. Provides tools to initialize and manage GitHub spec-kit configurations for enhanced AI-driven development.

## What It Provides

| Tool     | Type     | License | Description                  |
| -------- | -------- | ------- | ---------------------------- |
| spec-kit | cli-tool | MIT     | GitHub specification toolkit |

## Requirements

- **Disk Space**: 50 MB
- **Memory**: 64 MB
- **Install Time**: ~30 seconds
- **Dependencies**: python (provides uvx command)

### Network Domains

- github.com
- raw.githubusercontent.com

## Installation

```bash
sindri extension install spec-kit
```

## Configuration

### Install Method

Uses a custom installation script.

### Capabilities

#### Project Initialization

The extension supports automatic project initialization with priority 10:

```bash
uvx --from git+https://github.com/github/spec-kit.git specify init --here --force --ai claude --script sh
```

This creates a `.github/spec.json` configuration file for AI-powered workflows.

### Post-Project Init Hook

After initialization, runs:

```bash
bash scripts/commit-spec-kit.sh
```

## Usage Examples

### Initialize Spec-Kit

```bash
# Initialize in current directory (via uvx)
uvx --from git+https://github.com/github/spec-kit.git specify init --here --force --ai claude --script sh

# This creates .github/spec.json
```

### Spec-Kit Configuration

The `.github/spec.json` file contains:

- AI model preferences (Claude)
- Script configurations
- Repository metadata for AI context

### Validation

```bash
# Verify spec-kit is initialized
test -f .github/spec.json && echo "spec-kit configured"
```

## State Markers

| Path                | Type | Description                        |
| ------------------- | ---- | ---------------------------------- |
| `.github/spec.json` | file | GitHub spec-kit configuration file |

## Validation

The extension validates the following commands:

- `uvx` - Must match pattern `uv \d+\.\d+\.\d+`

## Removal

Script-based removal.

## Related Extensions

- [python](PYTHON.md) - Required for uvx command
