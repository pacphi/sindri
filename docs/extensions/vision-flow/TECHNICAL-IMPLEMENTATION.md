# VisionFlow Technical Implementation Guide

Detailed implementation roadmap for converting VisionFlow capabilities into Sindri extensions.

## Implementation Strategy

### Approach

1. Create extension skeleton with `extension.yaml`, `install.sh`, and `resources/` directory
2. Duplicate all source files from VisionFlow (no external references)
3. Adapt install scripts for Sindri's execution model
4. Register in `registry.yaml` and create profile bundles

### Extension Naming

- Prefix: `vf-` (VisionFlow)
- Location: `docker/lib/extensions/vf-{name}/`
- Resources: `docker/lib/extensions/vf-{name}/resources/`

---

## Priority Tiers

### Tier 1: Quick Wins (12 extensions)

Self-contained, easy to implement. No external service dependencies.

| #   | Extension            | Method          | Est. Time |
| --- | -------------------- | --------------- | --------- |
| 1   | vf-imagemagick       | apt             | 15 min    |
| 2   | vf-ffmpeg-processing | apt             | 15 min    |
| 3   | vf-latex-documents   | apt             | 15 min    |
| 4   | vf-ngspice           | apt             | 15 min    |
| 5   | vf-pdf               | script (python) | 30 min    |
| 6   | vf-docx              | script (python) | 30 min    |
| 7   | vf-pptx              | script (python) | 30 min    |
| 8   | vf-xlsx              | script (python) | 30 min    |
| 9   | vf-wardley-maps      | script (nodejs) | 30 min    |
| 10  | vf-slack-gif-creator | script          | 30 min    |
| 11  | vf-algorithmic-art   | script (python) | 30 min    |
| 12  | vf-mcp-builder       | script          | 30 min    |

### Tier 2: Service Dependencies (10 extensions)

Need API keys, external servers, or complex dependencies.

| #   | Extension             | Method                | Est. Time |
| --- | --------------------- | --------------------- | --------- |
| 1   | vf-perplexity         | script (nodejs)       | 45 min    |
| 2   | vf-web-summary        | script (nodejs)       | 45 min    |
| 3   | vf-deepseek-reasoning | script (nodejs)       | 45 min    |
| 4   | vf-playwright-mcp     | script (nodejs)       | 45 min    |
| 5   | vf-chrome-devtools    | script (nodejs)       | 45 min    |
| 6   | vf-jupyter-notebooks  | script (python)       | 45 min    |
| 7   | vf-kicad              | hybrid (apt + script) | 60 min    |
| 8   | vf-skill-creator      | script                | 30 min    |
| 9   | vf-webapp-testing     | script (nodejs)       | 45 min    |
| 10  | vf-docker-manager     | script                | 45 min    |

### Tier 3: Desktop/GPU (7 extensions)

Require GUI environment or GPU acceleration.

| #   | Extension        | Method                | Est. Time |
| --- | ---------------- | --------------------- | --------- |
| 1   | vf-blender       | hybrid (apt + script) | 90 min    |
| 2   | vf-qgis          | hybrid (apt + script) | 90 min    |
| 3   | vf-pbr-rendering | script                | 60 min    |
| 4   | vf-canvas-design | script (nodejs)       | 45 min    |
| 5   | vf-comfyui       | script (python)       | 120 min   |
| 6   | vf-pytorch-ml    | script (python)       | 90 min    |
| 7   | vf-vnc-desktop   | hybrid (apt + script) | 90 min    |

### Tier 4: Architectural (5 extensions)

Complex multi-component or orchestration systems.

| #   | Extension             | Method          | Est. Time |
| --- | --------------------- | --------------- | --------- |
| 1   | vf-ontology-enrich    | script (python) | 90 min    |
| 2   | vf-import-to-ontology | script (python) | 90 min    |
| 3   | vf-gemini-flow        | script (nodejs) | 90 min    |
| 4   | vf-zai-service        | script (nodejs) | 90 min    |
| 5   | vf-management-api     | script (nodejs) | 120 min   |

---

## Extension Templates

### Template A: APT-based Extension

```yaml
---
metadata:
  name: vf-{name}
  version: 1.0.0
  description: {Description} (from VisionFlow)
  category: utilities
  author: VisionFlow/Sindri Team
  homepage: https://github.com/DreamLab-AI/VisionFlow
  dependencies: []

requirements:
  domains:
    - {domain}
  diskSpace: {size_mb}
  memory: {memory_mb}
  installTime: {seconds}

install:
  method: apt
  apt:
    packages:
      - {package}

configure:
  templates:
    - source: SKILL.md
      destination: ~/extensions/vf-{name}/SKILL.md
      mode: overwrite

validate:
  commands:
    - name: {command}
      versionFlag: --version
      expectedPattern: "{pattern}"

remove:
  apt:
    packages:
      - {package}
  paths:
    - ~/extensions/vf-{name}

bom:
  tools:
    - name: {tool}
      version: dynamic
      source: apt
      type: cli-tool
      license: {license}
      homepage: {url}
      purl: pkg:deb/debian/{package}
```

### Template B: Script-based MCP Server

```yaml
---
metadata:
  name: vf-{name}
  version: 1.0.0
  description: {Description} MCP server (from VisionFlow)
  category: ai
  author: VisionFlow/Sindri Team
  homepage: https://github.com/DreamLab-AI/VisionFlow
  dependencies:
    - nodejs  # or python

requirements:
  domains:
    - registry.npmjs.org  # or pypi.org
    - {api_domain}
  diskSpace: {size_mb}
  memory: {memory_mb}
  installTime: {seconds}
  secrets:
    - {api_key_name}

install:
  method: script
  script:
    path: install.sh
    timeout: {timeout}

configure:
  templates:
    - source: SKILL.md
      destination: ~/extensions/vf-{name}/SKILL.md
      mode: overwrite
  environment:
    - key: {ENV_VAR}
      value: "${ENV_VAR}"
      scope: bashrc

validate:
  commands:
    - name: {command}
      versionFlag: {flag}
      expectedPattern: "{pattern}"

remove:
  paths:
    - ~/extensions/vf-{name}

bom:
  tools:
    - name: {tool}-mcp
      version: 1.0.0
      source: script
      type: server
      license: MIT
      homepage: https://github.com/DreamLab-AI/VisionFlow
      purl: pkg:{type}/{package}
```

### Template C: GPU Extension

```yaml
---
metadata:
  name: vf-{name}
  version: 1.0.0
  description: {Description} with GPU support (from VisionFlow)
  category: ai
  author: VisionFlow/Sindri Team
  homepage: https://github.com/DreamLab-AI/VisionFlow
  dependencies:
    - python

requirements:
  domains:
    - pypi.org
    - {download_domain}
  diskSpace: {size_mb}
  memory: {memory_mb}
  installTime: {seconds}
  gpu:
    required: {true|false}
    recommended: {true|false}
    type: nvidia
    minMemory: {vram_mb}

install:
  method: script
  script:
    path: install.sh
    timeout: {timeout}

configure:
  templates:
    - source: SKILL.md
      destination: ~/extensions/vf-{name}/SKILL.md
      mode: overwrite
  environment:
    - key: CUDA_VISIBLE_DEVICES
      value: "0"
      scope: bashrc

validate:
  commands:
    - name: python3
      versionFlag: "-c \"import {module}; print({module}.__version__)\""
      expectedPattern: "\\d+\\.\\d+"

remove:
  paths:
    - ~/extensions/vf-{name}
    - ~/.cache/{name}

bom:
  tools:
    - name: {tool}
      version: dynamic
      source: pip
      type: framework
      license: {license}
      homepage: {url}
      purl: pkg:pypi/{package}
```

### Template D: Desktop Extension

```yaml
---
metadata:
  name: vf-{name}
  version: 1.0.0
  description: {Description} with GUI (from VisionFlow)
  category: desktop
  author: VisionFlow/Sindri Team
  homepage: https://github.com/DreamLab-AI/VisionFlow
  dependencies:
    - xfce-ubuntu

requirements:
  domains:
    - {download_domain}
  diskSpace: {size_mb}
  memory: {memory_mb}
  installTime: {seconds}

install:
  method: hybrid
  apt:
    packages:
      - {package}
  script:
    path: install.sh
    timeout: {timeout}

configure:
  templates:
    - source: SKILL.md
      destination: ~/extensions/vf-{name}/SKILL.md
      mode: overwrite
    - source: {addon_file}
      destination: {addon_destination}
      mode: overwrite

validate:
  commands:
    - name: {command}
      versionFlag: --version
      expectedPattern: "{pattern}"

remove:
  apt:
    packages:
      - {package}
  paths:
    - ~/extensions/vf-{name}
    - {addon_destination}

bom:
  tools:
    - name: {tool}
      version: dynamic
      source: apt
      type: application
      license: {license}
      homepage: {url}
      purl: pkg:deb/debian/{package}
```

---

## Resource Duplication

### Directory Structure Per Extension

```
docker/lib/extensions/vf-{name}/
├── extension.yaml          # Sindri extension definition
├── install.sh              # Installation script
├── upgrade.sh              # Upgrade script (optional)
└── resources/              # Duplicated VisionFlow resources
    ├── SKILL.md            # Original skill documentation
    ├── mcp-server/         # MCP server code (if applicable)
    │   ├── server.js       # or server.py
    │   ├── package.json    # or requirements.txt
    │   └── ...
    ├── tools/              # Tool scripts (if applicable)
    ├── scripts/            # Utility scripts (if applicable)
    ├── templates/          # Config templates (if applicable)
    └── examples/           # Example files (if applicable)
```

### Copy Commands

```bash
# For each skill in VisionFlow
SKILL_NAME="perplexity"
SRC="/tmp/VisionFlow/multi-agent-docker/skills/${SKILL_NAME}"
DST="docker/lib/extensions/vf-${SKILL_NAME}/resources"

# Create extension directory
mkdir -p "${DST}"

# Copy all resources
cp -r "${SRC}/SKILL.md" "${DST}/" 2>/dev/null || true
cp -r "${SRC}/mcp-server" "${DST}/" 2>/dev/null || true
cp -r "${SRC}/tools" "${DST}/" 2>/dev/null || true
cp -r "${SRC}/scripts" "${DST}/" 2>/dev/null || true
cp -r "${SRC}/templates" "${DST}/" 2>/dev/null || true
cp -r "${SRC}/examples" "${DST}/" 2>/dev/null || true
```

---

## Install Script Pattern

### Node.js MCP Server

```bash
#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-{name}
# VisionFlow capability: {description}

EXTENSION_DIR="${HOME}/extensions/vf-{name}"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-{name}/resources"

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources
cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"

# Install dependencies
cd "${EXTENSION_DIR}/mcp-server"
npm install --production

# Make server executable
chmod +x server.js

echo "vf-{name} installed successfully"
```

### Python MCP Server

```bash
#!/usr/bin/env bash
set -euo pipefail

# Install script for vf-{name}
# VisionFlow capability: {description}

EXTENSION_DIR="${HOME}/extensions/vf-{name}"
RESOURCE_DIR="${DOCKER_LIB:-/docker/lib}/extensions/vf-{name}/resources"

# Create extension directory
mkdir -p "${EXTENSION_DIR}"

# Copy resources
cp -r "${RESOURCE_DIR}"/* "${EXTENSION_DIR}/"

# Create virtual environment
python3 -m venv "${EXTENSION_DIR}/.venv"
source "${EXTENSION_DIR}/.venv/bin/activate"

# Install dependencies
if [[ -f "${EXTENSION_DIR}/mcp-server/requirements.txt" ]]; then
    pip install -r "${EXTENSION_DIR}/mcp-server/requirements.txt"
fi

# Install common MCP dependencies
pip install mcp pydantic httpx

echo "vf-{name} installed successfully"
```

---

## Registry Updates

Add to `docker/lib/registry.yaml`:

```yaml
extensions:
  # ... existing extensions ...

  # VisionFlow AI Extensions
  vf-perplexity:
    category: ai
    description: Perplexity AI real-time web research MCP server
    dependencies: [nodejs]

  vf-web-summary:
    category: ai
    description: URL/YouTube summarization MCP server
    dependencies: [nodejs]

  vf-deepseek-reasoning:
    category: ai
    description: Deepseek reasoning MCP server
    dependencies: [nodejs]

  vf-comfyui:
    category: ai
    description: ComfyUI image generation MCP server
    dependencies: [python]

  vf-pytorch-ml:
    category: ai
    description: PyTorch deep learning framework
    dependencies: [python]

  vf-ontology-enrich:
    category: ai
    description: AI-powered ontology enrichment
    dependencies: [python]

  vf-import-to-ontology:
    category: ai
    description: Document to ontology import
    dependencies: [python]

  vf-gemini-flow:
    category: ai
    description: Gemini multi-agent orchestration
    dependencies: [nodejs]

  vf-zai-service:
    category: ai
    description: Cost-effective Claude API wrapper
    dependencies: [nodejs]

  # VisionFlow Dev-Tools Extensions
  vf-playwright-mcp:
    category: dev-tools
    description: Playwright browser automation MCP server
    dependencies: [nodejs, playwright]

  vf-chrome-devtools:
    category: dev-tools
    description: Chrome DevTools Protocol MCP server
    dependencies: [nodejs]

  vf-jupyter-notebooks:
    category: dev-tools
    description: Jupyter notebook execution MCP server
    dependencies: [python]

  vf-webapp-testing:
    category: dev-tools
    description: Web app testing framework
    dependencies: [nodejs, playwright]

  vf-kicad:
    category: dev-tools
    description: KiCad PCB design MCP server
    dependencies: [xfce-ubuntu]

  vf-ngspice:
    category: dev-tools
    description: NGSpice circuit simulation MCP server
    dependencies: []

  vf-mcp-builder:
    category: dev-tools
    description: MCP server scaffolding tool
    dependencies: [nodejs]

  vf-skill-creator:
    category: dev-tools
    description: Claude Code skill scaffolding tool
    dependencies: []

  # VisionFlow Desktop Extensions
  vf-blender:
    category: desktop
    description: Blender 3D modeling MCP server
    dependencies: [xfce-ubuntu]

  vf-qgis:
    category: desktop
    description: QGIS GIS operations MCP server
    dependencies: [xfce-ubuntu]

  vf-pbr-rendering:
    category: desktop
    description: PBR material generation MCP server
    dependencies: [vf-blender]

  vf-canvas-design:
    category: desktop
    description: Design system framework
    dependencies: [nodejs]

  vf-vnc-desktop:
    category: desktop
    description: VNC desktop server
    dependencies: []

  # VisionFlow Utilities Extensions
  vf-imagemagick:
    category: utilities
    description: ImageMagick processing MCP server
    dependencies: []

  vf-ffmpeg-processing:
    category: utilities
    description: FFmpeg media processing
    dependencies: []

  vf-latex-documents:
    category: utilities
    description: LaTeX document system
    dependencies: []

  vf-pdf:
    category: utilities
    description: PDF manipulation tools
    dependencies: [python]

  vf-docx:
    category: utilities
    description: Word document tools
    dependencies: [python]

  vf-pptx:
    category: utilities
    description: PowerPoint tools
    dependencies: [python]

  vf-xlsx:
    category: utilities
    description: Excel tools
    dependencies: [python]

  vf-wardley-maps:
    category: utilities
    description: Strategic mapping visualization
    dependencies: [nodejs]

  vf-slack-gif-creator:
    category: utilities
    description: Slack GIF generation
    dependencies: [nodejs, vf-ffmpeg-processing]

  vf-algorithmic-art:
    category: utilities
    description: Generative algorithmic art
    dependencies: [python]

  # VisionFlow Infrastructure Extensions
  vf-docker-manager:
    category: infrastructure
    description: Docker container management MCP server
    dependencies: [docker]

  vf-management-api:
    category: infrastructure
    description: HTTP task orchestration API
    dependencies: [nodejs]
```

---

## Profile Bundles

Add to `docker/lib/profiles.yaml`:

```yaml
profiles:
  # ... existing profiles ...

  visionflow-core:
    description: VisionFlow core document processing and automation tools
    extensions:
      - vf-imagemagick
      - vf-ffmpeg-processing
      - vf-latex-documents
      - vf-pdf
      - vf-docx
      - vf-pptx
      - vf-xlsx
      - vf-playwright-mcp
      - vf-jupyter-notebooks

  visionflow-ai:
    description: VisionFlow AI research and ML tools
    extensions:
      - vf-perplexity
      - vf-web-summary
      - vf-deepseek-reasoning
      - vf-pytorch-ml
      - vf-comfyui
      - vf-ontology-enrich
      - vf-import-to-ontology

  visionflow-creative:
    description: VisionFlow 3D modeling and creative tools
    extensions:
      - vf-blender
      - vf-qgis
      - vf-pbr-rendering
      - vf-canvas-design
      - vf-algorithmic-art

  visionflow-full:
    description: All VisionFlow extensions
    extensions:
      - vf-perplexity
      - vf-web-summary
      - vf-deepseek-reasoning
      - vf-comfyui
      - vf-pytorch-ml
      - vf-ontology-enrich
      - vf-import-to-ontology
      - vf-gemini-flow
      - vf-zai-service
      - vf-playwright-mcp
      - vf-chrome-devtools
      - vf-jupyter-notebooks
      - vf-webapp-testing
      - vf-kicad
      - vf-ngspice
      - vf-mcp-builder
      - vf-skill-creator
      - vf-blender
      - vf-qgis
      - vf-pbr-rendering
      - vf-canvas-design
      - vf-vnc-desktop
      - vf-imagemagick
      - vf-ffmpeg-processing
      - vf-latex-documents
      - vf-pdf
      - vf-docx
      - vf-pptx
      - vf-xlsx
      - vf-wardley-maps
      - vf-slack-gif-creator
      - vf-algorithmic-art
      - vf-docker-manager
      - vf-management-api
```

---

## Validation

After creating extensions, run:

```bash
# Validate single extension
./cli/extension-manager validate vf-imagemagick

# Validate all extensions
./cli/extension-manager validate-all

# Run extension tests
pnpm test:extensions
```

---

## Testing Strategy

1. **Schema Validation**: All extension.yaml files must pass schema validation
2. **Install Test**: Run install.sh in isolated container
3. **Validation Commands**: Verify all validate.commands pass
4. **Remove Test**: Verify clean uninstall

```bash
# Test single extension lifecycle
./cli/extension-manager install vf-imagemagick
./cli/extension-manager status vf-imagemagick
./cli/extension-manager remove vf-imagemagick
```
