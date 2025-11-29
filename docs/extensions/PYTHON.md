# Python

Python 3.13 runtime via mise.

## Overview

| Property         | Value    |
| ---------------- | -------- |
| **Category**     | language |
| **Version**      | 1.0.0    |
| **Installation** | mise     |
| **Disk Space**   | 400 MB   |
| **Dependencies** | None     |

## Description

Python 3.13 via mise - provides the Python runtime and pip package manager for Python development.

## Installed Tools

| Tool     | Type            | Description              |
| -------- | --------------- | ------------------------ |
| `python` | runtime         | Python 3.13 interpreter  |
| `pip`    | package-manager | Python package installer |

## Configuration

### Environment Variables

| Variable                  | Value | Scope  |
| ------------------------- | ----- | ------ |
| `PYTHONDONTWRITEBYTECODE` | `1`   | bashrc |

### mise.toml

```toml
[tools]
python = "3.13"
```

## Network Requirements

- `pypi.org` - Python Package Index
- `python.org` - Python downloads

## Installation

```bash
extension-manager install python
```

## Validation

```bash
python --version    # Expected: Python 3.X.X
pip --version
```

## Removal

```bash
extension-manager remove python
```

Removes mise configuration and Python installation.

## Related Extensions

- [ai-toolkit](AI-TOOLKIT.md) - AI tools (requires python)
- [monitoring](MONITORING.md) - Claude monitoring (requires python)
