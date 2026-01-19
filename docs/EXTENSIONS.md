# Extension Catalog

Comprehensive guide to all available Sindri extensions. Each extension is documented in detail in the [extensions/](extensions/) directory.

## Extension System Overview

Extensions are YAML-defined packages that install and configure development tools. Each extension:

- Declares metadata, dependencies, and requirements
- Uses declarative installation methods (mise, apt, script)
- mise supports multiple backends: core tools (node, python), npm packages (npm:), Go (go:), Rust (cargo:)
- Validates successful installation
- Integrates with the extension manifest system
- Tracks installed software via Bill of Materials (BOM)

## Quick Start

```bash
# Install a single extension
extension-manager install nodejs

# Install a profile (bundle of extensions)
extension-manager install-profile fullstack

# List available extensions
extension-manager list

# Check extension status
extension-manager status nodejs
```

## Extension Profiles

Pre-configured bundles for common workflows. Profiles make it easy to set up complete development environments.

| Profile                       | Extensions                                                                                                                                                                                                                                                         | Use Case                     |
| ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------- |
| **minimal**                   | nodejs, python                                                                                                                                                                                                                                                     | Lightweight scripting        |
| **fullstack**                 | nodejs, python, docker, nodejs-devtools                                                                                                                                                                                                                            | Web development              |
| **ai-dev**                    | nodejs, python, golang, ollama, ai-toolkit, mdflow, openskills, supabase-cli, linear-mcp, monitoring                                                                                                                                                               | AI/ML development            |
| **anthropic-dev**             | agent-manager, claude-flow-v3, agentic-flow, agentic-qe, golang, ollama, ai-toolkit, claudish, claude-marketplace, infra-tools, jvm, mdflow, openskills, nodejs-devtools, playwright, rust, ruvnet-research, linear-mcp, supabase-cli, tmux-workspace, cloud-tools | Anthropic/Claude development |
| **systems**                   | rust, golang, docker, infra-tools                                                                                                                                                                                                                                  | Systems programming          |
| **enterprise**                | All languages + infrastructure                                                                                                                                                                                                                                     | Complete environment         |
| **devops**                    | docker, infra-tools, cloud-tools, monitoring                                                                                                                                                                                                                       | Infrastructure               |
| **mobile**                    | nodejs                                                                                                                                                                                                                                                             | Mobile development (WIP)     |
| **visionflow-core**           | Document processing & automation (9 extensions)                                                                                                                                                                                                                    | Document workflows           |
| **visionflow-data-scientist** | AI research & ML tools (7 extensions)                                                                                                                                                                                                                              | Data science & research      |
| **visionflow-creative**       | 3D modeling & creative tools (5 extensions)                                                                                                                                                                                                                        | Creative development         |
| **visionflow-full**           | All VisionFlow extensions (33 total)                                                                                                                                                                                                                               | Complete VisionFlow suite    |

### Using Profiles

```yaml
# sindri.yaml
extensions:
  profile: fullstack
```

Or via CLI:

```bash
extension-manager install-profile fullstack
```

## Extensions by Category

### Base System

Pre-installed foundational extensions:

| Extension   | Description               | Docs                                        |
| ----------- | ------------------------- | ------------------------------------------- |
| mise-config | Mise tool version manager | [MISE-CONFIG.md](extensions/MISE-CONFIG.md) |

### Language Runtimes

| Extension | Language                  | Version    | Docs                                |
| --------- | ------------------------- | ---------- | ----------------------------------- |
| nodejs    | Node.js                   | LTS        | [NODEJS.md](extensions/NODEJS.md)   |
| python    | Python                    | 3.13       | [PYTHON.md](extensions/PYTHON.md)   |
| golang    | Go                        | 1.24       | [GOLANG.md](extensions/GOLANG.md)   |
| rust      | Rust                      | stable     | [RUST.md](extensions/RUST.md)       |
| ruby      | Ruby                      | 3.4.7      | [RUBY.md](extensions/RUBY.md)       |
| jvm       | Java/Kotlin/Scala/Clojure | Java 25    | [JVM.md](extensions/JVM.md)         |
| dotnet    | .NET                      | 10.0 & 8.0 | [DOTNET.md](extensions/DOTNET.md)   |
| php       | PHP                       | 8.4        | [PHP.md](extensions/PHP.md)         |
| haskell   | Haskell                   | GHC 9.x    | [HASKELL.md](extensions/HASKELL.md) |

### Development Tools

| Extension          | Purpose                        | Docs                                                      |
| ------------------ | ------------------------------ | --------------------------------------------------------- |
| nodejs-devtools    | TypeScript, ESLint, Prettier   | [NODEJS-DEVTOOLS.md](extensions/NODEJS-DEVTOOLS.md)       |
| github-cli         | GitHub CLI (`gh`)              | [GITHUB-CLI.md](extensions/GITHUB-CLI.md)                 |
| playwright         | Browser automation testing     | [PLAYWRIGHT.md](extensions/PLAYWRIGHT.md)                 |
| agent-browser      | Browser automation for AI      | [AGENT-BROWSER.md](extensions/AGENT-BROWSER.md)           |
| agentic-qe         | AI-powered test generation     | [AGENTIC-QE.md](extensions/AGENTIC-QE.md)                 |
| tmux-workspace     | Terminal multiplexer workspace | [TMUX-WORKSPACE.md](extensions/TMUX-WORKSPACE.md)         |
| claude-marketplace | Claude Code plugin marketplace | [CLAUDE-MARKETPLACE.md](extensions/CLAUDE-MARKETPLACE.md) |

### AI Tools

| Extension       | Purpose                                       | Docs                                                |
| --------------- | --------------------------------------------- | --------------------------------------------------- |
| claude-codepro  | TDD-enforced Claude Code environment          | [CLAUDE-CODEPRO.md](extensions/CLAUDE-CODEPRO.md)   |
| ollama          | Local LLM runtime (Llama, Mistral, etc.)      | [OLLAMA.md](extensions/OLLAMA.md)                   |
| ai-toolkit      | AI CLI tools (Fabric, Codex, Gemini, etc.)    | [AI-TOOLKIT.md](extensions/AI-TOOLKIT.md)           |
| openskills      | Claude Code skills manager                    | [OPENSKILLS.md](extensions/OPENSKILLS.md)           |
| claude-code-mux | AI routing proxy (18+ providers)              | [CLAUDE-CODE-MUX.md](extensions/CLAUDE-CODE-MUX.md) |
| claudish        | OpenRouter model proxy for Claude Code        | [CLAUDISH.md](extensions/CLAUDISH.md)               |
| claudeup        | TUI for Claude Code plugins & MCP config      | [CLAUDEUP.md](extensions/CLAUDEUP.md)               |
| agent-manager   | AI agent orchestration                        | [AGENT-MANAGER.md](extensions/AGENT-MANAGER.md)     |
| claude-flow-v2  | Multi-agent orchestration (stable)            | [CLAUDE-FLOW-V2.md](extensions/CLAUDE-FLOW-V2.md)   |
| claude-flow-v3  | Multi-agent orchestration (alpha, 10x faster) | [CLAUDE-FLOW-V3.md](extensions/CLAUDE-FLOW-V3.md)   |
| agentic-flow    | Agentic workflow orchestration                | [AGENTIC-FLOW.md](extensions/AGENTIC-FLOW.md)       |
| goose           | AI coding agent with tool use                 | [GOOSE.md](extensions/GOOSE.md)                     |
| mdflow          | Markdown to AI agent CLI                      | [MDFLOW.md](extensions/MDFLOW.md)                   |
| ralph           | AI-driven autonomous development system       | [RALPH.md](extensions/RALPH.md)                     |
| context7-mcp    | Context7 library documentation MCP            | [CONTEXT7-MCP.md](extensions/CONTEXT7-MCP.md)       |
| ruvnet-research | AI research tools (Goalie, Research-Swarm)    | [RUVNET-RESEARCH.md](extensions/RUVNET-RESEARCH.md) |

### Infrastructure

| Extension    | Purpose                                           | Docs                                          |
| ------------ | ------------------------------------------------- | --------------------------------------------- |
| docker       | Docker Engine & Compose                           | [DOCKER.md](extensions/DOCKER.md)             |
| infra-tools  | Terraform, Kubernetes, Ansible, Pulumi + 10 more  | [INFRA-TOOLS.md](extensions/INFRA-TOOLS.md)   |
| cloud-tools  | AWS, Azure, GCP, OCI, Alibaba, DO, IBM CLIs       | [CLOUD-TOOLS.md](extensions/CLOUD-TOOLS.md)   |
| supabase-cli | Supabase CLI for local dev, migrations, functions | [SUPABASE-CLI.md](extensions/SUPABASE-CLI.md) |
| monitoring   | Claude usage monitoring (uv, claude-monitor)      | [MONITORING.md](extensions/MONITORING.md)     |

### Agile

| Extension  | Purpose                                  | Docs                                      |
| ---------- | ---------------------------------------- | ----------------------------------------- |
| linear-mcp | Linear MCP server for project management | [LINEAR-MCP.md](extensions/LINEAR-MCP.md) |
| jira-mcp   | Atlassian Jira/Confluence MCP server     | [JIRA-MCP.md](extensions/JIRA-MCP.md)     |

### Desktop & Utilities

| Extension   | Purpose                                | Docs                                        |
| ----------- | -------------------------------------- | ------------------------------------------- |
| guacamole   | Web-based remote desktop (SSH/RDP/VNC) | [GUACAMOLE.md](extensions/GUACAMOLE.md)     |
| xfce-ubuntu | XFCE desktop with xRDP                 | [XFCE-UBUNTU.md](extensions/XFCE-UBUNTU.md) |

## VisionFlow Extensions

VisionFlow extensions bring 34 specialized capabilities from the [VisionFlow](https://github.com/DreamLab-AI/VisionFlow) project. These extensions provide AI-powered workflows, document processing, creative tools, and development utilities.

See [VisionFlow README](extensions/vision-flow/README.md) for implementation details and architecture.

### VisionFlow AI Tools

| Extension             | Purpose                                 | Docs                                                                        |
| --------------------- | --------------------------------------- | --------------------------------------------------------------------------- |
| vf-perplexity         | Perplexity AI real-time web research    | [VF-PERPLEXITY.md](extensions/vision-flow/VF-PERPLEXITY.md)                 |
| vf-web-summary        | URL/YouTube summarization MCP           | [VF-WEB-SUMMARY.md](extensions/vision-flow/VF-WEB-SUMMARY.md)               |
| vf-deepseek-reasoning | Deepseek reasoning MCP                  | [VF-DEEPSEEK-REASONING.md](extensions/vision-flow/VF-DEEPSEEK-REASONING.md) |
| vf-comfyui            | ComfyUI image generation (GPU required) | [VF-COMFYUI.md](extensions/vision-flow/VF-COMFYUI.md)                       |
| vf-pytorch-ml         | PyTorch deep learning framework         | [VF-PYTORCH-ML.md](extensions/vision-flow/VF-PYTORCH-ML.md)                 |
| vf-ontology-enrich    | AI-powered ontology enrichment          | [VF-ONTOLOGY-ENRICH.md](extensions/vision-flow/VF-ONTOLOGY-ENRICH.md)       |
| vf-import-to-ontology | Document to ontology import             | [VF-IMPORT-TO-ONTOLOGY.md](extensions/vision-flow/VF-IMPORT-TO-ONTOLOGY.md) |
| vf-gemini-flow        | Gemini multi-agent orchestration        | [VF-GEMINI-FLOW.md](extensions/vision-flow/VF-GEMINI-FLOW.md)               |
| vf-zai-service        | Cost-effective Claude API wrapper       | [VF-ZAI-SERVICE.md](extensions/vision-flow/VF-ZAI-SERVICE.md)               |

### VisionFlow Development Tools

| Extension            | Purpose                           | Docs                                                                      |
| -------------------- | --------------------------------- | ------------------------------------------------------------------------- |
| vf-playwright-mcp    | Playwright browser automation MCP | [VF-PLAYWRIGHT-MCP.md](extensions/vision-flow/VF-PLAYWRIGHT-MCP.md)       |
| vf-chrome-devtools   | Chrome DevTools Protocol MCP      | [VF-CHROME-DEVTOOLS.md](extensions/vision-flow/VF-CHROME-DEVTOOLS.md)     |
| vf-jupyter-notebooks | Jupyter notebook execution MCP    | [VF-JUPYTER-NOTEBOOKS.md](extensions/vision-flow/VF-JUPYTER-NOTEBOOKS.md) |
| vf-webapp-testing    | Web app testing framework         | [VF-WEBAPP-TESTING.md](extensions/vision-flow/VF-WEBAPP-TESTING.md)       |
| vf-kicad             | KiCad PCB design MCP              | [VF-KICAD.md](extensions/vision-flow/VF-KICAD.md)                         |
| vf-ngspice           | NGSpice circuit simulation MCP    | [VF-NGSPICE.md](extensions/vision-flow/VF-NGSPICE.md)                     |
| vf-mcp-builder       | MCP server scaffolding tool       | [VF-MCP-BUILDER.md](extensions/vision-flow/VF-MCP-BUILDER.md)             |

### VisionFlow Desktop & Creative

| Extension        | Purpose                                | Docs                                                              |
| ---------------- | -------------------------------------- | ----------------------------------------------------------------- |
| vf-blender       | Blender 3D modeling MCP                | [VF-BLENDER.md](extensions/vision-flow/VF-BLENDER.md)             |
| vf-qgis          | QGIS GIS operations MCP                | [VF-QGIS.md](extensions/vision-flow/VF-QGIS.md)                   |
| vf-pbr-rendering | PBR material generation (GPU required) | [VF-PBR-RENDERING.md](extensions/vision-flow/VF-PBR-RENDERING.md) |
| vf-canvas-design | Design system framework                | [VF-CANVAS-DESIGN.md](extensions/vision-flow/VF-CANVAS-DESIGN.md) |
| vf-vnc-desktop   | VNC desktop server                     | [VF-VNC-DESKTOP.md](extensions/vision-flow/VF-VNC-DESKTOP.md)     |

### VisionFlow Utilities

| Extension            | Purpose                          | Docs                                                                      |
| -------------------- | -------------------------------- | ------------------------------------------------------------------------- |
| vf-imagemagick       | ImageMagick processing MCP       | [VF-IMAGEMAGICK.md](extensions/vision-flow/VF-IMAGEMAGICK.md)             |
| vf-ffmpeg-processing | FFmpeg media processing          | [VF-FFMPEG-PROCESSING.md](extensions/vision-flow/VF-FFMPEG-PROCESSING.md) |
| vf-latex-documents   | LaTeX document system            | [VF-LATEX-DOCUMENTS.md](extensions/vision-flow/VF-LATEX-DOCUMENTS.md)     |
| vf-pdf               | PDF document generation MCP      | [VF-PDF.md](extensions/vision-flow/VF-PDF.md)                             |
| vf-docx              | Word document generation MCP     | [VF-DOCX.md](extensions/vision-flow/VF-DOCX.md)                           |
| vf-pptx              | PowerPoint generation MCP        | [VF-PPTX.md](extensions/vision-flow/VF-PPTX.md)                           |
| vf-xlsx              | Excel spreadsheet generation MCP | [VF-XLSX.md](extensions/vision-flow/VF-XLSX.md)                           |
| vf-wardley-maps      | Wardley mapping MCP              | [VF-WARDLEY-MAPS.md](extensions/vision-flow/VF-WARDLEY-MAPS.md)           |
| vf-slack-gif-creator | Slack GIF creation tool          | [VF-SLACK-GIF-CREATOR.md](extensions/vision-flow/VF-SLACK-GIF-CREATOR.md) |
| vf-algorithmic-art   | Generative art tools             | [VF-ALGORITHMIC-ART.md](extensions/vision-flow/VF-ALGORITHMIC-ART.md)     |

### VisionFlow Infrastructure

| Extension         | Purpose                       | Docs                                                                |
| ----------------- | ----------------------------- | ------------------------------------------------------------------- |
| vf-docker-manager | Docker container management   | [VF-DOCKER-MANAGER.md](extensions/vision-flow/VF-DOCKER-MANAGER.md) |
| vf-management-api | API management and monitoring | [VF-MANAGEMENT-API.md](extensions/vision-flow/VF-MANAGEMENT-API.md) |

## Extension Features

### Upgrade Strategies

Extensions support different upgrade approaches:

| Strategy    | Description               | Extensions                                                                                                            |
| ----------- | ------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| `automatic` | Auto-upgrade via mise/apt | dotnet, ruby, nodejs-devtools, monitoring, xfce-ubuntu, agent-manager, openskills                                     |
| `reinstall` | Full reinstallation       | ollama                                                                                                                |
| `manual`    | Custom upgrade script     | ai-toolkit, cloud-tools, jvm, infra-tools, claude-code-mux, playwright, guacamole, linear-mcp, jira-mcp, supabase-cli |
| `none`      | No upgrades (static)      | github-cli, claude-marketplace, mise-config                                                                           |

### Secret Requirements

Some extensions require API keys or credentials:

| Extension       | Required Secrets                              |
| --------------- | --------------------------------------------- |
| ai-toolkit      | `google_gemini_api_key`, `grok_api_key`       |
| cloud-tools     | AWS, Azure credentials                        |
| claudish        | `openrouter_api_key`                          |
| github-cli      | `github_token`                                |
| jira-mcp        | `jira_url`, `jira_username`, `jira_api_token` |
| linear-mcp      | `linear_api_key`                              |
| ruvnet-research | `perplexity_api_key` (optional)               |
| supabase-cli    | `supabase_access_token` (optional)            |

### Removal Confirmation

These extensions require confirmation before removal (destructive operation):

- docker
- infra-tools
- cloud-tools
- claude-code-mux
- ollama
- openskills
- agent-manager
- tmux-workspace
- guacamole
- xfce-ubuntu

## Extension Management

### List Extensions

```bash
extension-manager list
extension-manager list-profiles
```

### Install Extensions

```bash
# Single extension
extension-manager install nodejs

# Multiple extensions
extension-manager install nodejs python docker

# From profile
extension-manager install-profile fullstack
```

### Validate Extensions

```bash
# Single extension
extension-manager validate nodejs

# All installed extensions
extension-manager validate-all
```

### Upgrade Extensions

```bash
extension-manager upgrade nodejs
```

### Reinstall Extensions

Reinstall cleans up the existing installation and reinstalls fresh:

```bash
# Reinstall single extension
extension-manager reinstall nodejs

# Reinstall all extensions in a profile
extension-manager reinstall-profile fullstack
```

### Remove Extensions

```bash
extension-manager remove nodejs
```

## Extension Dependencies

Dependencies are automatically resolved and installed:

```text
nodejs-devtools → nodejs
playwright → nodejs
openskills → nodejs
ai-toolkit → nodejs, python, golang, github-cli
monitoring → python
ruvnet-research → nodejs
claudish → nodejs
linear-mcp → nodejs
jira-mcp → docker
supabase-cli → nodejs, docker
```

## Extension Storage

| Location                       | Purpose                       |
| ------------------------------ | ----------------------------- |
| `/docker/lib/extensions/`      | Extension definitions (YAML)  |
| `/workspace/.system/manifest/` | Installed extension manifests |
| `/workspace/.system/logs/`     | Extension installation logs   |
| `/workspace/.system/bom/`      | Bill of Materials tracking    |

## Conflict Resolution

When cloning projects or initializing multiple extensions, conflicts may occur with existing files or directories. Sindri provides automatic conflict resolution with configurable strategies.

### Environment Variables

Control conflict resolution behavior using these environment variables:

#### `EXTENSION_CONFLICT_STRATEGY`

Global default action for all conflicts. Overrides extension-defined conflict rules.

**Valid values:**

- `append` - Add new content to end of file
- `prepend` - Add new content to beginning of file
- `overwrite` - Replace existing file with new content
- `merge-json` - Deep merge JSON files (requires `jq`)
- `merge-yaml` - Deep merge YAML files (requires `yq`)
- `backup` - Backup existing before proceeding
- `skip` - Keep existing, ignore new content

**Example:**

```bash
# Force all conflicts to append (useful for CLAUDE.md)
EXTENSION_CONFLICT_STRATEGY=append clone-project https://github.com/user/repo

# Backup all conflicts for safety
EXTENSION_CONFLICT_STRATEGY=backup clone-project https://github.com/user/repo
```

#### `EXTENSION_CONFLICT_PROMPT`

Control interactive prompts for conflict resolution. Set to `false` to disable prompts in CI/CD.

**Valid values:**

- `true` (default) - Allow interactive prompts
- `false` - Disable prompts, use safe defaults (skip)

**Example:**

```bash
# CI/CD: Disable prompts, skip conflicts
EXTENSION_CONFLICT_PROMPT=false clone-project https://github.com/user/repo

# CI/CD: Disable prompts, backup conflicts
EXTENSION_CONFLICT_STRATEGY=backup EXTENSION_CONFLICT_PROMPT=false clone-project https://github.com/user/repo
```

### Conflict Resolution Priority

Extensions initialize in priority order (lower number = earlier):

| Extension      | Priority | Behavior                                   |
| -------------- | -------- | ------------------------------------------ |
| spec-kit       | 10       | Creates .github/spec.json, no conflicts    |
| claude-flow-v3 | 20       | Primary framework, creates .claude/        |
| claude-flow-v2 | 20       | Primary framework, creates .claude/        |
| agentic-qe     | 50       | Appends to CLAUDE.md, merges into .claude/ |
| agentic-flow   | 60       | Appends to CLAUDE.md                       |
| ralph          | 70       | Appends to CLAUDE.md                       |

### Common Conflict Scenarios

**Scenario 1: Cloning repo with existing CLAUDE.md**

```bash
# Result: All extensions append their sections with separators
clone-project https://github.com/user/repo

# CLAUDE.md contains:
# <Original content from repo>
#
# ---
#
# <claude-flow-v3 content>
#
# ---
#
# <agentic-qe content>
```

**Scenario 2: CI/CD Pipeline**

```bash
# GitHub Actions workflow
- name: Clone project
  env:
    EXTENSION_CONFLICT_STRATEGY: backup
    EXTENSION_CONFLICT_PROMPT: false
  run: clone-project https://github.com/user/repo
```

**Scenario 3: Force overwrite in CI**

```bash
# Replace all conflicting files (dangerous!)
EXTENSION_CONFLICT_STRATEGY=overwrite EXTENSION_CONFLICT_PROMPT=false clone-project https://github.com/user/repo
```

## Related Documentation

- [Extension Authoring](EXTENSION_AUTHORING.md) - Create custom extensions
- [Architecture](ARCHITECTURE.md) - System architecture
- [Configuration](CONFIGURATION.md) - sindri.yaml configuration
