# VisionFlow Capability Catalog

Complete inventory of all capabilities extracted from VisionFlow multi-agent-docker.

## Summary

| Category       | Extensions | Description                                     |
| -------------- | ---------- | ----------------------------------------------- |
| AI             | 9          | AI/ML frameworks, research tools, orchestration |
| Dev-Tools      | 8          | Development, testing, engineering tools         |
| Desktop        | 5          | GUI applications requiring display              |
| Utilities      | 10         | Document processing, media, visualization       |
| Infrastructure | 2          | Container and API management                    |
| **Total**      | **34**     | (3 capabilities already exist in Sindri)        |

---

## Category: AI (9 extensions)

### vf-perplexity

- **Type**: MCP Server
- **Complexity**: Medium
- **Source**: `/skills/perplexity/`
- **Description**: Real-time web research with Perplexity AI Sonar API. Supports deep research, citations, and UK English prioritization.
- **Dependencies**: nodejs
- **Requirements**: `PERPLEXITY_API_KEY`

### vf-web-summary

- **Type**: MCP Server
- **Complexity**: Medium
- **Source**: `/skills/web-summary/`
- **Description**: URL summarization and YouTube transcript extraction via Z.AI service.
- **Dependencies**: nodejs

### vf-deepseek-reasoning

- **Type**: MCP Server
- **Complexity**: Medium
- **Source**: `/skills/deepseek-reasoning/`
- **Description**: Complex reasoning tasks via Deepseek API.
- **Dependencies**: nodejs
- **Requirements**: `DEEPSEEK_API_KEY`

### vf-comfyui

- **Type**: MCP Server
- **Complexity**: High
- **Source**: `/skills/comfyui/`, `/comfyui/`
- **Description**: Node-based image generation with Stable Diffusion, FLUX, SAM3D integration.
- **Dependencies**: python
- **Requirements**: GPU (NVIDIA, 8GB+ VRAM)
- **Disk**: ~5GB (models)

### vf-pytorch-ml

- **Type**: Installed Framework
- **Complexity**: High
- **Source**: `/skills/pytorch-ml/`
- **Description**: PyTorch deep learning framework with CUDA 13.0 support.
- **Dependencies**: python
- **Requirements**: GPU recommended (NVIDIA, 8GB+ VRAM)
- **Disk**: ~5GB

### vf-ontology-enrich

- **Type**: Multi-agent
- **Complexity**: High
- **Source**: `/skills/ontology-enrich/`
- **Description**: AI-powered knowledge base enrichment with OWL2 validation, batch processing, and git rollback.
- **Dependencies**: python

### vf-import-to-ontology

- **Type**: Multi-agent
- **Complexity**: High
- **Source**: `/skills/import-to-ontology/`
- **Description**: Document ingestion and semantic indexing to ontologies.
- **Dependencies**: python

### vf-gemini-flow

- **Type**: Service
- **Complexity**: High
- **Source**: `/config/gemini-flow.config.ts`
- **Description**: Google Gemini multi-agent orchestration daemon.
- **Dependencies**: nodejs
- **Requirements**: `GOOGLE_GEMINI_API_KEY`

### vf-zai-service

- **Type**: Service
- **Complexity**: Medium
- **Source**: `/claude-zai/`
- **Description**: Cost-effective Claude API wrapper with worker pool. Internal service on port 9600.
- **Dependencies**: nodejs
- **Requirements**: `ZAI_ANTHROPIC_API_KEY`

---

## Category: Dev-Tools (8 extensions)

### vf-playwright-mcp

- **Type**: MCP Server
- **Complexity**: Easy
- **Source**: `/skills/playwright/`
- **Description**: Browser automation MCP server extending Playwright.
- **Dependencies**: nodejs, playwright

### vf-chrome-devtools

- **Type**: MCP Server
- **Complexity**: Easy
- **Source**: `/skills/chrome-devtools/`
- **Description**: Chrome DevTools Protocol integration for web debugging.
- **Dependencies**: nodejs

### vf-jupyter-notebooks

- **Type**: MCP Server
- **Complexity**: Easy
- **Source**: `/skills/jupyter-notebooks/`
- **Description**: Interactive notebook execution via MCP.
- **Dependencies**: python

### vf-webapp-testing

- **Type**: Multi-agent
- **Complexity**: Medium
- **Source**: `/skills/webapp-testing/`
- **Description**: Automated web app testing with element discovery and console logging.
- **Dependencies**: nodejs, playwright

### vf-kicad

- **Type**: MCP Server
- **Complexity**: Medium
- **Source**: `/skills/kicad/`
- **Description**: PCB design and schematic generation with KiCad.
- **Dependencies**: xfce-ubuntu (for GUI)
- **Install**: apt (kicad)

### vf-ngspice

- **Type**: MCP Server
- **Complexity**: Easy
- **Source**: `/skills/ngspice/`
- **Description**: Circuit simulation with NGSpice netlist parsing.
- **Install**: apt (ngspice)

### vf-mcp-builder

- **Type**: Tool
- **Complexity**: Easy
- **Source**: `/skills/mcp-builder/`
- **Description**: Generate new MCP servers from templates.
- **Dependencies**: nodejs

### vf-skill-creator

- **Type**: Tool
- **Complexity**: Easy
- **Source**: `/skills/skill-creator/`
- **Description**: Scaffold new Claude Code skills with init, package, and validate commands.

---

## Category: Desktop (5 extensions)

### vf-blender

- **Type**: MCP Server
- **Complexity**: Medium
- **Source**: `/skills/blender/`
- **Description**: Blender 3D modeling with MCP addon. Supports PolyHaven, Sketchfab, Hyper3D Rodin.
- **Dependencies**: xfce-ubuntu
- **Install**: apt (blender)
- **Socket**: 9876

### vf-qgis

- **Type**: MCP Server
- **Complexity**: Medium
- **Source**: `/skills/qgis/`
- **Description**: GIS operations and geospatial mapping via QGIS.
- **Dependencies**: xfce-ubuntu
- **Install**: apt (qgis)
- **Socket**: 9877

### vf-pbr-rendering

- **Type**: MCP Server
- **Complexity**: Medium
- **Source**: `/skills/pbr-rendering/`
- **Description**: PBR material generation with nvdiffrast.
- **Dependencies**: vf-blender
- **Requirements**: GPU (NVIDIA)
- **Socket**: 9878

### vf-canvas-design

- **Type**: Tool
- **Complexity**: Easy
- **Source**: `/skills/canvas-design/`
- **Description**: Design system framework with 36+ font families and brand guidelines.
- **Dependencies**: nodejs

### vf-vnc-desktop

- **Type**: Service
- **Complexity**: Medium
- **Source**: `/unified-config/`
- **Description**: VNC server with Xvfb, 9 color-coded terminals in 3x3 grid.
- **Install**: apt + script

---

## Category: Utilities (10 extensions)

### vf-imagemagick

- **Type**: MCP Server
- **Complexity**: Easy
- **Source**: `/skills/imagemagick/`
- **Description**: Image processing and batch operations via ImageMagick.
- **Install**: apt (imagemagick)

### vf-ffmpeg-processing

- **Type**: Installed
- **Complexity**: Easy
- **Source**: `/skills/ffmpeg-processing/`
- **Description**: Professional video/audio transcoding with FFmpeg.
- **Install**: apt (ffmpeg)

### vf-latex-documents

- **Type**: Installed
- **Complexity**: Easy
- **Source**: `/skills/latex-documents/`
- **Description**: TeX Live with BibTeX, Beamer for academic documents.
- **Install**: apt (texlive-full or texlive-base)

### vf-pdf

- **Type**: Tool
- **Complexity**: Easy
- **Source**: `/skills/pdf/`
- **Description**: PDF manipulation with PyMuPDF and pdfplumber. Form filling, image extraction, bounding boxes.
- **Dependencies**: python

### vf-docx

- **Type**: Tool
- **Complexity**: Easy
- **Source**: `/skills/docx/`
- **Description**: Word document processing with python-docx. OOXML pack/unpack.
- **Dependencies**: python

### vf-pptx

- **Type**: Tool
- **Complexity**: Easy
- **Source**: `/skills/pptx/`
- **Description**: PowerPoint manipulation with python-pptx. Inventory, rearrange, thumbnails.
- **Dependencies**: python

### vf-xlsx

- **Type**: Tool
- **Complexity**: Easy
- **Source**: `/skills/xlsx/`
- **Description**: Excel operations with openpyxl. Formula recalculation.
- **Dependencies**: python

### vf-wardley-maps

- **Type**: Tool
- **Complexity**: Easy
- **Source**: `/skills/wardley-maps/`
- **Description**: Strategic context mapping with 7 modules (mapper, analyzer, parser, heuristics).
- **Dependencies**: nodejs

### vf-slack-gif-creator

- **Type**: Tool
- **Complexity**: Easy
- **Source**: `/skills/slack-gif-creator/`
- **Description**: Generate Slack-compatible animated GIFs. 13 animation templates with global palette optimization.
- **Dependencies**: nodejs, vf-ffmpeg-processing

### vf-algorithmic-art

- **Type**: Tool
- **Complexity**: Easy
- **Source**: `/skills/algorithmic-art/`
- **Description**: Generative algorithmic art patterns with p5.js.
- **Dependencies**: python

---

## Category: Infrastructure (2 extensions)

### vf-docker-manager

- **Type**: MCP Server
- **Complexity**: Medium
- **Source**: `/skills/docker-manager/`
- **Description**: Docker container lifecycle management. Build, up, down, restart, logs, exec, discovery.
- **Dependencies**: docker

### vf-management-api

- **Type**: Service
- **Complexity**: High
- **Source**: `/management-api/`
- **Description**: HTTP REST API for external task orchestration. Authentication, rate limiting, task isolation.
- **Dependencies**: nodejs
- **Port**: 9090

---

## Skipped (Already in Sindri)

| VisionFlow Capability | Existing Sindri Extension          |
| --------------------- | ---------------------------------- |
| rust-development      | `rust`                             |
| claude-flow           | `claude-flow`                      |
| code-server           | Similar to existing IDE extensions |

---

## Dependency Matrix

| Extension             | nodejs | python | xfce-ubuntu | docker | GPU |
| --------------------- | ------ | ------ | ----------- | ------ | --- |
| vf-perplexity         | X      |        |             |        |     |
| vf-web-summary        | X      |        |             |        |     |
| vf-deepseek-reasoning | X      |        |             |        |     |
| vf-comfyui            |        | X      |             |        | X   |
| vf-pytorch-ml         |        | X      |             |        | rec |
| vf-ontology-enrich    |        | X      |             |        |     |
| vf-import-to-ontology |        | X      |             |        |     |
| vf-gemini-flow        | X      |        |             |        |     |
| vf-zai-service        | X      |        |             |        |     |
| vf-playwright-mcp     | X      |        |             |        |     |
| vf-chrome-devtools    | X      |        |             |        |     |
| vf-jupyter-notebooks  |        | X      |             |        |     |
| vf-webapp-testing     | X      |        |             |        |     |
| vf-kicad              |        |        | X           |        |     |
| vf-ngspice            |        |        |             |        |     |
| vf-mcp-builder        | X      |        |             |        |     |
| vf-skill-creator      |        |        |             |        |     |
| vf-blender            |        |        | X           |        | rec |
| vf-qgis               |        |        | X           |        |     |
| vf-pbr-rendering      |        |        | X           |        | X   |
| vf-canvas-design      | X      |        |             |        |     |
| vf-vnc-desktop        |        |        |             |        |     |
| vf-imagemagick        |        |        |             |        |     |
| vf-ffmpeg-processing  |        |        |             |        |     |
| vf-latex-documents    |        |        |             |        |     |
| vf-pdf                |        | X      |             |        |     |
| vf-docx               |        | X      |             |        |     |
| vf-pptx               |        | X      |             |        |     |
| vf-xlsx               |        | X      |             |        |     |
| vf-wardley-maps       | X      |        |             |        |     |
| vf-slack-gif-creator  | X      |        |             |        |     |
| vf-algorithmic-art    |        | X      |             |        |     |
| vf-docker-manager     |        |        |             | X      |     |
| vf-management-api     | X      |        |             |        |     |

---

## API Key Requirements

| Extension             | Environment Variable    |
| --------------------- | ----------------------- |
| vf-perplexity         | `PERPLEXITY_API_KEY`    |
| vf-deepseek-reasoning | `DEEPSEEK_API_KEY`      |
| vf-gemini-flow        | `GOOGLE_GEMINI_API_KEY` |
| vf-zai-service        | `ZAI_ANTHROPIC_API_KEY` |
