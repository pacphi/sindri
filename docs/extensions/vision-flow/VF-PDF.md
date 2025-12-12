# VF-PDF

PDF manipulation with PyMuPDF and pdfplumber.

## Overview

| Property         | Value                  |
| ---------------- | ---------------------- |
| **Category**     | utilities              |
| **Version**      | 1.0.0                  |
| **Installation** | script                 |
| **Disk Space**   | 100 MB                 |
| **Memory**       | 256 MB                 |
| **Dependencies** | [python](../PYTHON.md) |

## Description

PDF manipulation with PyMuPDF and pdfplumber (from VisionFlow) - provides comprehensive PDF processing including form filling, image extraction, text extraction, and bounding box analysis.

## Installed Tools

| Tool         | Type    | Description               |
| ------------ | ------- | ------------------------- |
| `pdfplumber` | library | PDF text/table extraction |
| `pymupdf`    | library | PDF rendering and editing |
| `PyPDF2`     | library | PDF manipulation          |
| `reportlab`  | library | PDF generation            |

## Configuration

### Templates

| Template   | Destination                    | Description         |
| ---------- | ------------------------------ | ------------------- |
| `SKILL.md` | `~/extensions/vf-pdf/SKILL.md` | Skill documentation |

## Network Requirements

- `pypi.org` - Python packages

## Installation

```bash
extension-manager install vf-pdf
```

## Validation

```bash
python3 -c "import pdfplumber; print('ok')"
```

Expected output: `ok`

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade vf-pdf
```

## Removal

```bash
extension-manager remove vf-pdf
```

Removes:

- `~/extensions/vf-pdf`

## Related Extensions

- [vf-docx](VF-DOCX.md) - Word document processing
- [vf-pptx](VF-PPTX.md) - PowerPoint processing
- [vf-xlsx](VF-XLSX.md) - Excel processing
- [vf-latex-documents](VF-LATEX-DOCUMENTS.md) - LaTeX documents
