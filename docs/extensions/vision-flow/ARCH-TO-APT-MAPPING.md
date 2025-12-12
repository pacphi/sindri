# Arch Linux to Debian/Ubuntu Package Mapping

VisionFlow uses CachyOS (Arch Linux) with `pacman`. Sindri uses Debian/Ubuntu with `apt`.
This document tracks the package conversions.

## Core Desktop/GUI Packages

| Arch (pacman) | Debian/Ubuntu (apt)           | Notes                       |
| ------------- | ----------------------------- | --------------------------- |
| `blender`     | `blender`                     | Same name                   |
| `qgis`        | `qgis qgis-plugin-grass`      | Same name, add grass plugin |
| `kicad`       | `kicad kicad-libraries`       | Same name                   |
| `imagemagick` | `imagemagick imagemagick-doc` | Same name                   |
| `ngspice`     | `ngspice`                     | Same name                   |

## LaTeX Packages

| Arch (pacman)              | Debian/Ubuntu (apt)         | Notes          |
| -------------------------- | --------------------------- | -------------- |
| `texlive-basic`            | `texlive-base`              | Different name |
| `texlive-bin`              | `texlive-binaries`          | Different name |
| `texlive-binextra`         | `texlive-extra-utils`       | Different name |
| `texlive-fontsrecommended` | `texlive-fonts-recommended` | Similar        |
| `texlive-latexrecommended` | `texlive-latex-recommended` | Similar        |
| `biber`                    | `biber`                     | Same name      |

## VNC/Desktop Packages

| Arch (pacman)      | Debian/Ubuntu (apt) | Notes          |
| ------------------ | ------------------- | -------------- |
| `x11vnc`           | `x11vnc`            | Same name      |
| `xorg-server-xvfb` | `xvfb`              | Different name |
| `openbox`          | `openbox`           | Same name      |
| `tint2`            | `tint2`             | Same name      |
| `xfce4-terminal`   | `xfce4-terminal`    | Same name      |
| `kitty`            | `kitty`             | Same name      |

## Media Processing

| Arch (pacman) | Debian/Ubuntu (apt) | Notes     |
| ------------- | ------------------- | --------- |
| `ffmpeg`      | `ffmpeg`            | Same name |

## Python-based Extensions (No apt packages needed)

These extensions use pip packages instead of system packages:

- vf-pdf, vf-docx, vf-pptx, vf-xlsx (python-docx, openpyxl, etc.)
- vf-pytorch-ml (PyTorch via pip)
- vf-comfyui (PyTorch + custom)
- vf-perplexity, vf-web-summary, etc. (MCP servers)

## Node.js-based Extensions (No apt packages needed)

These extensions use npm packages:

- vf-playwright-mcp, vf-chrome-devtools, vf-wardley-maps, etc.

## Verified Extensions

All extensions using apt packages have been verified for correct Debian/Ubuntu equivalents:

| Extension            | Arch Original          | Our apt Equivalent            | Status  |
| -------------------- | ---------------------- | ----------------------------- | ------- |
| vf-blender           | `blender`              | `blender`                     | Correct |
| vf-qgis              | `qgis`                 | `qgis qgis-plugin-grass`      | Correct |
| vf-kicad             | `kicad`                | `kicad kicad-libraries`       | Correct |
| vf-imagemagick       | `imagemagick`          | `imagemagick imagemagick-doc` | Correct |
| vf-ngspice           | `ngspice`              | `ngspice`                     | Correct |
| vf-ffmpeg-processing | `ffmpeg`               | `ffmpeg`                      | Correct |
| vf-latex-documents   | `texlive-*`            | `texlive-*` (Debian names)    | Correct |
| vf-vnc-desktop       | `x11vnc openbox tint2` | Same + xvfb                   | Correct |
