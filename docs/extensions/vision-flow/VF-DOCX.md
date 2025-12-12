# VF-DOCX

Word document processing with python-docx.

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

Word document processing with python-docx (from VisionFlow) - DOCX manipulation including OOXML pack/unpack operations.

## Installed Tools

| Tool          | Type    | Description     |
| ------------- | ------- | --------------- |
| `python-docx` | library | DOCX processing |
| `lxml`        | library | XML processing  |

## Configuration

### Templates

| Template   | Destination                     | Description         |
| ---------- | ------------------------------- | ------------------- |
| `SKILL.md` | `~/extensions/vf-docx/SKILL.md` | Skill documentation |

## Network Requirements

- `pypi.org` - Python packages

## Installation

```bash
extension-manager install vf-docx
```

## Validation

```bash
python3 -c "import docx; print('ok')"
```

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade vf-docx
```

## Removal

```bash
extension-manager remove vf-docx
```

Removes:

- `~/extensions/vf-docx`

## Related Extensions

- [vf-pdf](VF-PDF.md) - PDF processing
- [vf-pptx](VF-PPTX.md) - PowerPoint
- [vf-xlsx](VF-XLSX.md) - Excel
