# Python

Python 3.13 runtime with uv package manager via mise.

## Overview

| Property         | Value                           |
| ---------------- | ------------------------------- |
| **Category**     | language                        |
| **Version**      | 1.1.1                           |
| **Installation** | mise                            |
| **Disk Space**   | 450 MB                          |
| **Dependencies** | [mise-config](MISE-CONFIG.md)   |

## Description

Python 3.13 with uv package manager via mise - provides the Python runtime, pip, and Astral's uv for fast Python package management and project operations.

## Installed Tools

| Tool     | Type            | Description                                    |
| -------- | --------------- | ---------------------------------------------- |
| `python` | runtime         | Python 3.13 interpreter                        |
| `pip`    | package-manager | Python package installer                       |
| `uv`     | package-manager | Fast Python package manager (Astral)           |
| `uvx`    | cli-tool        | Execute packages from PyPI (like npx for node) |

## Configuration

### Environment Variables

| Variable                  | Value | Scope  |
| ------------------------- | ----- | ------ |
| `PYTHONDONTWRITEBYTECODE` | `1`   | bashrc |

### mise.toml

```toml
[tools]
python = "3.13"
uv = "latest"
```

## Network Requirements

- `pypi.org` - Python Package Index
- `python.org` - Python downloads
- `github.com` - uv releases

## Installation

```bash
extension-manager install python
```

## Validation

```bash
python --version    # Expected: Python 3.X.X
pip --version
uv --version        # Expected: uv X.X.X
uvx --version
```

## uv Usage Examples

```bash
# Install packages (faster than pip)
uv pip install requests

# Run a package without installing
uvx ruff check .

# Create a new Python project
uv init my-project

# Sync project dependencies
uv sync
```

## Removal

```bash
extension-manager remove python
```

Removes mise configuration and Python installation.

## Related Extensions

- [ai-toolkit](AI-TOOLKIT.md) - AI tools (requires python)
- [monitoring](MONITORING.md) - Claude monitoring (requires python)
