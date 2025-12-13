# VF-Jupyter-Notebooks

Jupyter notebook execution MCP server.

## Overview

| Property         | Value                  |
| ---------------- | ---------------------- |
| **Category**     | dev-tools              |
| **Version**      | 1.0.0                  |
| **Installation** | script                 |
| **Disk Space**   | 500 MB                 |
| **Memory**       | 512 MB                 |
| **Dependencies** | [python](../PYTHON.md) |

## Description

Jupyter notebook execution MCP server (from VisionFlow) - provides interactive notebook execution and data science workflows.

## Installed Tools

| Tool         | Type      | Description             |
| ------------ | --------- | ----------------------- |
| `jupyter`    | framework | Jupyter notebook system |
| `jupyterlab` | framework | Jupyter Lab interface   |
| `ipykernel`  | library   | IPython kernel          |

## Configuration

### Templates

| Template   | Destination                                  | Description         |
| ---------- | -------------------------------------------- | ------------------- |
| `SKILL.md` | `~/extensions/vf-jupyter-notebooks/SKILL.md` | Skill documentation |

## Network Requirements

- `pypi.org` - Python packages

## Installation

```bash
extension-manager install vf-jupyter-notebooks
```

## Validation

```bash
jupyter --version
```

Expected output pattern: `\d+\.\d+`

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade vf-jupyter-notebooks
```

## Removal

```bash
extension-manager remove vf-jupyter-notebooks
```

Removes:

- `~/extensions/vf-jupyter-notebooks`

## Related Extensions

- [vf-pytorch-ml](VF-PYTORCH-ML.md) - PyTorch framework
- [python](../PYTHON.md) - Python runtime (required)

## Additional Notes

- IPython kernel installed automatically
- Supports interactive data science workflows
- Compatible with MCP protocol for Claude Code integration
