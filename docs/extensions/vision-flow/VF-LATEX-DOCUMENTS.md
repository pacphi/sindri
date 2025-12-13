# VF-LaTeX-Documents

TeX Live with BibTeX and Beamer for academic documents.

## Overview

| Property         | Value     |
| ---------------- | --------- |
| **Category**     | utilities |
| **Version**      | 1.0.0     |
| **Installation** | script    |
| **Disk Space**   | 3000 MB   |
| **Memory**       | 512 MB    |
| **Dependencies** | None      |

## Description

TeX Live with BibTeX and Beamer for academic documents (from VisionFlow) - provides comprehensive LaTeX document system with templates, themes, and examples for academic papers, reports, and presentations.

## Installed Tools

| Tool       | Type     | Description                 |
| ---------- | -------- | --------------------------- |
| `pdflatex` | cli-tool | LaTeX to PDF compiler       |
| `bibtex`   | cli-tool | Bibliography processor      |
| `biber`    | cli-tool | Modern bibliography backend |
| `latexmk`  | cli-tool | Build automation            |

## Configuration

### Templates

| Template   | Destination                                | Description         |
| ---------- | ------------------------------------------ | ------------------- |
| `SKILL.md` | `~/extensions/vf-latex-documents/SKILL.md` | Skill documentation |

## Network Requirements

- `tug.org` - TeX User Group
- `ctan.org` - Comprehensive TeX Archive

## Installation

```bash
extension-manager install vf-latex-documents
```

**Note:** This installation may take several minutes due to TeX Live package size.

## Validation

```bash
pdflatex --version
```

Expected output pattern: `pdfTeX`

## Upgrade

**Strategy:** reinstall

```bash
extension-manager upgrade vf-latex-documents
```

## Removal

```bash
extension-manager remove vf-latex-documents
```

Removes:

- `~/extensions/vf-latex-documents`

## Related Extensions

- [vf-pdf](VF-PDF.md) - PDF processing
- [vf-jupyter-notebooks](VF-JUPYTER-NOTEBOOKS.md) - Interactive notebooks

## Additional Notes

- Templates available in `~/extensions/vf-latex-documents/templates/`
- Beamer themes for presentations
- Includes verification script for module testing
