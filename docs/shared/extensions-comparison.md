# Sindri Extensions Comparison: V2 vs V3

This document provides a comprehensive comparison of extensions available in Sindri V2 and V3, including feature parity status and key architectural differences.

## Summary

| Metric                            | V2  | V3  |
| --------------------------------- | --- | --- |
| **Total Extensions**              | 77  | 44  |
| **Language Extensions**           | 11  | 11  |
| **AI/Agent Extensions**           | 10  | 9   |
| **MCP Extensions**                | 8   | 8   |
| **Desktop/VisionFlow Extensions** | 33  | 0   |
| **Dev Tools Extensions**          | 15  | 16  |

## Extension Comparison Table

### Language Extensions

| Extension | V2  | V3  | Description                                | Notes                   |
| --------- | :-: | :-: | ------------------------------------------ | ----------------------- |
| nodejs    | YES | YES | Node.js LTS via mise with pnpm             | Identical configuration |
| python    | YES | YES | Python 3.13 with uv package manager        | Identical configuration |
| rust      | YES | YES | Rust stable via rustup                     | Identical configuration |
| golang    | YES | YES | Go 1.25 via mise                           | Identical configuration |
| ruby      | YES | YES | Ruby via mise                              | Identical configuration |
| php       | YES | YES | PHP via mise                               | Identical configuration |
| haskell   | YES | YES | Haskell via ghcup (GHC, Cabal, Stack, HLS) | Identical configuration |
| dotnet    | YES | YES | .NET SDK                                   | Identical configuration |
| jvm       | YES | YES | Java/JVM languages                         | Identical configuration |

### AI and Agent Extensions

| Extension             | V2  | V3  | Description                        | Notes                        |
| --------------------- | :-: | :-: | ---------------------------------- | ---------------------------- |
| ai-toolkit            | YES | YES | AI development toolkit             | Identical configuration      |
| agentic-flow          | YES | YES | Multi-model AI agent framework     | Identical configuration      |
| agentic-qe            | YES | YES | Quality engineering agents         | Identical configuration      |
| claude-flow-v2        | YES | YES | Claude Flow v2 orchestration       | Legacy support               |
| claude-flow-v3        | YES | YES | Next-gen multi-agent orchestration | 10x performance, HNSW search |
| claude-codepro        | YES | YES | Claude Code professional tools     | Identical configuration      |
| ollama                | YES | YES | Local LLM inference                | Identical configuration      |
| goose                 | YES | YES | AI coding assistant                | Identical configuration      |
| ruvnet-research       | YES | YES | Research tools                     | Identical configuration      |
| vf-deepseek-reasoning | YES | NO  | DeepSeek reasoning model           | V2 only (VisionFlow)         |

### MCP Server Extensions

| Extension         | V2  | V3  | Description                   | Notes                   |
| ----------------- | :-: | :-: | ----------------------------- | ----------------------- |
| context7-mcp      | YES | YES | Context management MCP server | Identical configuration |
| jira-mcp          | YES | YES | Jira integration MCP server   | Identical configuration |
| linear-mcp        | YES | YES | Linear integration MCP server | Identical configuration |
| pal-mcp-server    | YES | YES | PAL MCP server                | Identical configuration |
| spec-kit          | YES | YES | Specification toolkit         | Identical configuration |
| ralph             | YES | YES | Ralph MCP server              | Identical configuration |
| vf-playwright-mcp | YES | NO  | Playwright MCP server         | V2 only (VisionFlow)    |
| vf-mcp-builder    | YES | NO  | MCP builder tools             | V2 only (VisionFlow)    |

### Development Tools

| Extension       | V2  | V3  | Description                   | Notes                   |
| --------------- | :-: | :-: | ----------------------------- | ----------------------- |
| docker          | YES | YES | Docker container tools        | Identical configuration |
| github-cli      | YES | YES | GitHub CLI (gh)               | Identical configuration |
| playwright      | YES | YES | Browser automation framework  | Identical configuration |
| nodejs-devtools | YES | YES | Node.js development utilities | Identical configuration |
| infra-tools     | YES | YES | Infrastructure tooling        | Identical configuration |
| monitoring      | YES | YES | System monitoring tools       | Identical configuration |
| cloud-tools     | YES | YES | Cloud provider CLIs           | Identical configuration |
| supabase-cli    | YES | YES | Supabase CLI tools            | Identical configuration |
| mdflow          | YES | YES | Markdown workflow tools       | Identical configuration |
| openskills      | YES | YES | OpenSkills framework          | Identical configuration |
| mise-config     | YES | YES | Mise configuration base       | Required dependency     |
| tmux-workspace  | YES | YES | Tmux workspace manager        | Identical configuration |

### Claude Extensions

| Extension          | V2  | V3  | Description              | Notes                   |
| ------------------ | :-: | :-: | ------------------------ | ----------------------- |
| claude-code-mux    | YES | YES | Claude Code multiplexer  | Identical configuration |
| claude-marketplace | YES | YES | Extension marketplace    | Identical configuration |
| claudeup           | YES | YES | Claude upgrade utilities | Identical configuration |
| claudish           | YES | YES | Claude shell integration | Identical configuration |

### Desktop and GUI Extensions

| Extension     | V2  | V3  | Description              | Notes                   |
| ------------- | :-: | :-: | ------------------------ | ----------------------- |
| xfce-ubuntu   | YES | YES | XFCE desktop environment | Required for GUI apps   |
| guacamole     | YES | YES | Remote desktop gateway   | Identical configuration |
| agent-browser | YES | YES | Browser agent interface  | Identical configuration |
| agent-manager | YES | YES | Agent management UI      | Identical configuration |

### VisionFlow Extensions (V2 Only)

These extensions are from the VisionFlow project and are currently only available in V2. They typically require GPU support and desktop environment.

| Extension             | V2  | V3  | Description                  | Category       |
| --------------------- | :-: | :-: | ---------------------------- | -------------- |
| vf-algorithmic-art    | YES | NO  | Algorithmic art generation   | Creative       |
| vf-blender            | YES | NO  | Blender 3D modeling with MCP | 3D/Design      |
| vf-canvas-design      | YES | NO  | Canvas-based design tools    | Design         |
| vf-chrome-devtools    | YES | NO  | Chrome DevTools integration  | Development    |
| vf-comfyui            | YES | NO  | ComfyUI image generation     | AI/Creative    |
| vf-docker-manager     | YES | NO  | Docker management GUI        | Infrastructure |
| vf-docx               | YES | NO  | Word document processing     | Documents      |
| vf-ffmpeg-processing  | YES | NO  | FFmpeg video processing      | Media          |
| vf-gemini-flow        | YES | NO  | Google Gemini integration    | AI             |
| vf-imagemagick        | YES | NO  | ImageMagick processing       | Media          |
| vf-import-to-ontology | YES | NO  | Ontology import tools        | Data           |
| vf-jupyter-notebooks  | YES | NO  | Jupyter notebook execution   | Development    |
| vf-kicad              | YES | NO  | KiCad PCB design             | Engineering    |
| vf-latex-documents    | YES | NO  | LaTeX document generation    | Documents      |
| vf-management-api     | YES | NO  | Management API tools         | Infrastructure |
| vf-ngspice            | YES | NO  | NGSpice circuit simulation   | Engineering    |
| vf-ontology-enrich    | YES | NO  | Ontology enrichment tools    | Data           |
| vf-pbr-rendering      | YES | NO  | PBR rendering pipeline       | 3D/Design      |
| vf-pdf                | YES | NO  | PDF processing tools         | Documents      |
| vf-perplexity         | YES | NO  | Perplexity AI integration    | AI             |
| vf-pptx               | YES | NO  | PowerPoint processing        | Documents      |
| vf-pytorch-ml         | YES | NO  | PyTorch ML framework         | AI/ML          |
| vf-qgis               | YES | NO  | QGIS geographic tools        | GIS            |
| vf-slack-gif-creator  | YES | NO  | Slack GIF creation           | Media          |
| vf-vnc-desktop        | YES | NO  | VNC desktop server           | Desktop        |
| vf-wardley-maps       | YES | NO  | Wardley mapping tools        | Strategy       |
| vf-web-summary        | YES | NO  | Web page summarization       | AI             |
| vf-webapp-testing     | YES | NO  | Web application testing      | Testing        |
| vf-xlsx               | YES | NO  | Excel processing             | Documents      |
| vf-zai-service        | YES | NO  | ZAI service integration      | AI             |

## Key Architectural Differences

### Extension Structure

Both V2 and V3 use the same YAML-based extension configuration format:

```yaml
metadata:
  name: extension-name
  version: 1.0.0
  description: Extension description
  category: language|ai|dev-tools|desktop
  dependencies: []

requirements:
  domains: []
  diskSpace: 100
  memory: 256
  installTime: 60

install:
  method: mise|script|hybrid
  # ...

configure:
  environment: []
  templates: []

validate:
  commands: []

remove:
  # cleanup configuration

bom:
  tools: []
```

### Installation Methods

| Method | V2  | V3  | Description                  |
| ------ | :-: | :-: | ---------------------------- |
| mise   | YES | YES | Version manager installation |
| script | YES | YES | Custom shell script          |
| hybrid | YES | YES | Mise + script combination    |

### Platform Differences

| Feature                  | V2                           | V3                |
| ------------------------ | ---------------------------- | ----------------- |
| Platform                 | Docker/Bash                  | Rust CLI          |
| Extension Location       | `/v2/docker/lib/extensions/` | `/v3/extensions/` |
| Configuration Processing | Bash scripts                 | Rust native       |
| GPU Support              | Full (NVIDIA)                | Limited           |
| Desktop Support          | Full (XFCE)                  | Limited           |

### V3-Specific Features

V3 introduces enhanced extension processing capabilities:

1. **Native Rust Processing**: Extensions are processed by the V3 Rust CLI instead of Bash scripts
2. **Template Processing**: Enhanced `configure.templates` processing with variable substitution
3. **Environment Setup**: Automatic environment variable configuration
4. **Validation**: Built-in validation commands execution

### Missing in V3

The VisionFlow extensions (33 extensions) are not yet available in V3 due to:

1. **GPU Dependencies**: Many VF extensions require NVIDIA GPU support
2. **Desktop Requirements**: Extensions like `vf-blender` and `vf-comfyui` require X11/desktop
3. **Complex Installation**: These extensions have multi-step installation scripts
4. **Docker Integration**: Some rely on V2's Docker-based architecture

## Migration Considerations

### For V2 to V3 Migration

1. **Core Language Extensions**: Fully compatible, no changes needed
2. **AI Extensions**: Fully compatible, configuration identical
3. **MCP Extensions**: Fully compatible
4. **VisionFlow Extensions**: Not available in V3; continue using V2 for these

### Extension Compatibility Matrix

| Use Case                                        | Recommended Platform |
| ----------------------------------------------- | -------------------- |
| Language development (Node, Python, Rust, etc.) | V3                   |
| AI agent workflows                              | V3                   |
| MCP server development                          | V3                   |
| 3D modeling (Blender)                           | V2                   |
| Image generation (ComfyUI)                      | V2                   |
| Video processing (FFmpeg)                       | V2                   |
| ML training (PyTorch)                           | V2                   |
| Document processing (PDF, DOCX, XLSX)           | V2                   |

## Future Roadmap

The following VisionFlow extensions are candidates for V3 porting:

1. **Priority 1** (high demand, lower complexity):
   - vf-jupyter-notebooks
   - vf-ffmpeg-processing
   - vf-pdf

2. **Priority 2** (medium complexity):
   - vf-pytorch-ml
   - vf-imagemagick
   - vf-chrome-devtools

3. **Priority 3** (requires GPU/desktop):
   - vf-blender
   - vf-comfyui
   - vf-vnc-desktop

---

_Last updated: January 2025_
_Extension counts: V2 = 77, V3 = 44_
