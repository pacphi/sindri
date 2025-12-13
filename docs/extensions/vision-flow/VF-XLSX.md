# VF-XLSX

Excel processing with openpyxl.

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

Excel processing with openpyxl (from VisionFlow) - provides Excel manipulation including formula recalculation, sheet operations, and data analysis.

## Installed Tools

| Tool       | Type    | Description      |
| ---------- | ------- | ---------------- |
| `openpyxl` | library | Excel processing |
| `xlrd`     | library | Excel reading    |
| `pandas`   | library | Data analysis    |

## Configuration

### Templates

| Template   | Destination                     | Description         |
| ---------- | ------------------------------- | ------------------- |
| `SKILL.md` | `~/extensions/vf-xlsx/SKILL.md` | Skill documentation |

## Network Requirements

- `pypi.org` - Python packages

## Installation

```bash
extension-manager install vf-xlsx
```

## Validation

```bash
python3 -c "import openpyxl; print('ok')"
```

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade vf-xlsx
```

## Removal

```bash
extension-manager remove vf-xlsx
```

Removes:

- `~/extensions/vf-xlsx`

## Related Extensions

- [vf-pdf](VF-PDF.md) - PDF processing
- [vf-docx](VF-DOCX.md) - Word processing
- [vf-pptx](VF-PPTX.md) - PowerPoint processing
