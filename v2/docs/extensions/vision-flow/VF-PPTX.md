# VF-PPTX

PowerPoint manipulation with python-pptx.

## Overview

| Property         | Value                  |
| ---------------- | ---------------------- |
| **Category**     | utilities              |
| **Version**      | 1.0.0                  |
| **Installation** | script                 |
| **Disk Space**   | 50 MB                  |
| **Memory**       | 128 MB                 |
| **Dependencies** | [python](../PYTHON.md) |

## Description

PowerPoint manipulation with python-pptx (from VisionFlow) - provides inventory, rearrange, replace, and thumbnail generation for PPTX files.

## Installed Tools

| Tool          | Type    | Description      |
| ------------- | ------- | ---------------- |
| `python-pptx` | library | PPTX processing  |
| `Pillow`      | library | Image processing |

## Configuration

### Templates

| Template   | Destination                     | Description         |
| ---------- | ------------------------------- | ------------------- |
| `SKILL.md` | `~/extensions/vf-pptx/SKILL.md` | Skill documentation |

## Network Requirements

- `pypi.org` - Python packages

## Installation

```bash
extension-manager install vf-pptx
```

## Validation

```bash
python3 -c "import pptx; print('ok')"
```

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade vf-pptx
```

## Removal

```bash
extension-manager remove vf-pptx
```

Removes:

- `~/extensions/vf-pptx`

## Related Extensions

- [vf-pdf](VF-PDF.md) - PDF processing
- [vf-docx](VF-DOCX.md) - Word processing
- [vf-xlsx](VF-XLSX.md) - Excel processing
