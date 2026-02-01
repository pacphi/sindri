# Python Extension

> Version: 1.1.1 | Category: languages | Last Updated: 2026-01-26

## Overview

Python 3.13 with uv package manager via mise. Provides a modern Python development environment with the fastest Python package manager available.

## What It Provides

| Tool   | Type            | License           | Description                           |
| ------ | --------------- | ----------------- | ------------------------------------- |
| python | runtime         | PSF-2.0           | Python 3.13 interpreter               |
| pip    | package-manager | MIT               | Standard Python package installer     |
| uv     | package-manager | Apache-2.0 OR MIT | Extremely fast Python package manager |
| uvx    | cli-tool        | Apache-2.0 OR MIT | Run Python tools without installation |

## Requirements

- **Disk Space**: 450 MB
- **Memory**: 128 MB
- **Install Time**: ~60 seconds
- **Dependencies**: mise-config

### Network Domains

- pypi.org
- python.org
- github.com

## Installation

```bash
sindri extension install python
```

## Configuration

### Environment Variables

| Variable                  | Value | Description                             |
| ------------------------- | ----- | --------------------------------------- |
| `PYTHONDONTWRITEBYTECODE` | 1     | Prevents Python from writing .pyc files |

### Install Method

Uses mise for tool management with automatic shim refresh.

## Usage Examples

### Running Python

```bash
# Check version
python --version

# Run a Python script
python script.py

# Start interactive REPL
python
```

### Package Management with uv

```bash
# Create a virtual environment
uv venv

# Install packages (10-100x faster than pip)
uv pip install requests flask

# Install from requirements.txt
uv pip install -r requirements.txt

# Sync dependencies
uv pip sync requirements.txt
```

### Using uvx

```bash
# Run tools without installing
uvx black .
uvx ruff check .
uvx pytest
```

### Traditional pip

```bash
# pip is still available
pip install package-name
pip list
pip freeze > requirements.txt
```

## Validation

The extension validates the following commands:

- `python` - Must match pattern `Python 3\.\d+\.\d+`
- `pip` - Must be available
- `uv` - Must match pattern `uv \d+\.\d+\.\d+`
- `uvx` - Must be available

## Removal

```bash
sindri extension remove python
```

This removes the mise configuration and Python tools.

## Related Extensions

- [mise-config](MISE-CONFIG.md) - Required mise configuration
- [spec-kit](SPEC-KIT.md) - Uses uvx for GitHub spec-kit workflows
