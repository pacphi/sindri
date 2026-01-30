# Sindri V3 Extensions

This documentation applies to **Sindri V3** (Rust CLI).

---

## Overview

Extensions are the modular building blocks of a Sindri development environment. Each extension packages a tool, language runtime, framework, or integration, providing declarative installation, configuration, and lifecycle management.

### V3 vs V2 Extensions

Sindri V3 introduces significant improvements to the extension system:

| Feature               | V2 (Bash)            | V3 (Rust)                                          |
| --------------------- | -------------------- | -------------------------------------------------- |
| Type system           | Runtime YAML parsing | Compile-time typed structs (80+ types)             |
| Installation methods  | 4 methods            | 6 methods (mise, apt, binary, npm, script, hybrid) |
| Security              | Basic validation     | Multi-layer path validation, checksum verification |
| Configuration         | Template copying     | Templates + environment variables + conditions     |
| Lifecycle hooks       | Pre/post install     | Pre/post install, project-init, collision handling |
| BOM generation        | None                 | Full SBOM with SPDX/CycloneDX support              |
| Dependency resolution | Linear               | DAG with topological sort                          |

### Key Concepts

- **extension.yaml**: Declarative YAML file defining the extension's metadata, requirements, installation method, configuration, and capabilities
- **Categories**: Extensions are grouped by function (languages, devops, claude, mcp, ai-agents, etc.)
- **Profiles**: Pre-configured sets of extensions for common use cases (minimal, fullstack, ai-dev, etc.)
- **Dependencies**: Extensions can declare dependencies on other extensions for ordered installation

---

## Extension Categories

V3 extensions are organized into the following categories:

| Category            | Description                                      | Count |
| ------------------- | ------------------------------------------------ | ----- |
| **languages**       | Programming language runtimes and toolchains     | 10    |
| **claude**          | Claude Code integrations and workflow tools      | 9     |
| **devops**          | Infrastructure, deployment, and operations tools | 4     |
| **ai-agents**       | AI agent frameworks and orchestration            | 4     |
| **ai-dev**          | AI development tools and LLM interfaces          | 3     |
| **mcp**             | Model Context Protocol servers                   | 6     |
| **package-manager** | SDK and package management tools                 | 2     |
| **productivity**    | Developer productivity and workflow tools        | 3     |
| **testing**         | Testing frameworks and automation                | 1     |
| **documentation**   | Documentation generation and management          | 2     |
| **cloud**           | Cloud service integrations                       | 1     |
| **desktop**         | Desktop environments and remote access           | 2     |
| **research**        | Research and analysis tools                      | 1     |

---

## Extension List

### Languages

| Extension           | Version | Description                                                                     | Install Method |
| ------------------- | ------- | ------------------------------------------------------------------------------- | -------------- |
| **nodejs**          | 1.1.0   | Node.js LTS via mise with pnpm package manager                                  | hybrid         |
| **python**          | 1.1.1   | Python 3.13 with uv package manager via mise                                    | mise           |
| **rust**            | 1.0.2   | Rust stable via rustup                                                          | script         |
| **golang**          | 1.0.1   | Go 1.25 via mise                                                                | mise           |
| **dotnet**          | 2.1.0   | .NET SDK 10.0 and 8.0 with ASP.NET Core and development tools                   | script         |
| **jvm**             | 2.1.0   | JVM languages (Java, Kotlin, Scala) via SDKMAN, plus Clojure/Leiningen via mise | script         |
| **php**             | 2.1.0   | PHP 8.4 with Composer, Symfony CLI, and development tools                       | script         |
| **ruby**            | 2.0.0   | Ruby 3.4.7 via mise with Rails and Bundler                                      | script         |
| **haskell**         | 1.0.1   | Haskell development environment with GHC, Cabal, Stack, and HLS                 | mise           |
| **nodejs-devtools** | 2.2.0   | TypeScript, ESLint, Prettier, and Node.js development tools                     | mise           |

### DevOps

| Extension       | Version | Description                                                                                        | Install Method |
| --------------- | ------- | -------------------------------------------------------------------------------------------------- | -------------- |
| **docker**      | 1.1.0   | Docker Engine and Compose with Docker-in-Docker support                                            | hybrid         |
| **infra-tools** | 2.0.0   | Infrastructure and DevOps tooling (Terraform, K8s, Ansible, Helm)                                  | hybrid         |
| **github-cli**  | 2.0.0   | GitHub CLI authentication and workflow configuration                                               | script         |
| **cloud-tools** | 2.0.0   | Cloud provider CLI tools (AWS, Azure, GCP, Fly.io, OCI, Alibaba, DO, IBM) - sudo-free installation | script         |

### Claude Code Integrations

| Extension              | Version | Description                                                                         | Install Method |
| ---------------------- | ------- | ----------------------------------------------------------------------------------- | -------------- |
| **claude-cli**         | latest  | Claude Code CLI - Official Anthropic AI coding assistant                            | script         |
| **claude-flow-v3**     | 3.0.0   | Next-gen multi-agent orchestration with 10x performance, 150x faster search (alpha) | mise           |
| **claude-flow-v2**     | 2.7.47  | AI-powered multi-agent orchestration system for Claude Code workflows (stable)      | mise           |
| **claude-codepro**     | 4.5.29  | Production-grade TDD-enforced development environment with automated quality checks | script         |
| **claude-code-mux**    | 1.0.0   | High-performance AI routing proxy with automatic failover across 18+ providers      | script         |
| **claudish**           | 1.0.0   | Claude Code CLI proxy for OpenRouter models via local Anthropic API proxy           | mise           |
| **claudeup**           | 1.0.0   | TUI tool for managing Claude Code plugins, MCPs, and configuration settings         | mise           |
| **claude-marketplace** | 2.0.0   | Claude Code plugin marketplace integration via YAML configuration                   | script         |
| **compahook**          | 1.0.0   | Persistent memory layer for Claude Code's /compact command                          | mise           |

### AI Agents

| Extension         | Version | Description                                                                          | Install Method |
| ----------------- | ------- | ------------------------------------------------------------------------------------ | -------------- |
| **agentic-flow**  | 1.0.0   | Multi-model AI agent framework for Claude Code with cost optimization (alpha)        | mise           |
| **agentic-qe**    | 1.1.0   | Agentic Quality Engineering v3 with AI-powered test generation and coverage analysis | mise           |
| **agent-manager** | 2.0.0   | Claude Code agent manager for managing AI agents                                     | script         |
| **agent-browser** | 0.6.0   | Headless browser automation CLI for AI agents with snapshot-based element selection  | mise           |

### AI Development

| Extension      | Version | Description                                                                      | Install Method |
| -------------- | ------- | -------------------------------------------------------------------------------- | -------------- |
| **ai-toolkit** | 2.1.0   | AI CLI tools and coding assistants (Fabric, Codex, Gemini, Droid, Grok, Copilot) | script         |
| **ollama**     | 1.0.0   | Ollama - Run large language models locally (Llama, Mistral, CodeLlama)           | script         |
| **goose**      | 1.0.0   | Block's open-source AI agent that automates engineering tasks                    | script         |

### MCP Servers

| Extension          | Version | Description                                                                   | Install Method |
| ------------------ | ------- | ----------------------------------------------------------------------------- | -------------- |
| **linear-mcp**     | 2.1.0   | Linear MCP server using Claude Code's native HTTP transport                   | script         |
| **jira-mcp**       | 2.0.0   | Atlassian MCP server using Claude Code's native SSE transport                 | script         |
| **context7-mcp**   | 1.0.0   | Context7 MCP server for version-specific library documentation                | script         |
| **excalidraw-mcp** | 1.0.0   | Real-time Excalidraw diagram creation and manipulation via MCP server         | hybrid         |
| **pal-mcp-server** | 9.8.2   | AI orchestration and multi-model collaboration MCP server with 18 tools       | script         |
| **monitoring**     | 2.0.0   | Claude monitoring and usage tracking tools (claude-monitor, claude-usage-cli) | script         |

### Package Managers

| Extension       | Version | Description                                                 | Install Method |
| --------------- | ------- | ----------------------------------------------------------- | -------------- |
| **mise-config** | 2.0.0   | Global mise configuration and settings                      | script         |
| **sdkman**      | 1.0.0   | SDKMAN - The Software Development Kit Manager for JVM tools | script         |

### Productivity

| Extension          | Version | Description                                                                      | Install Method |
| ------------------ | ------- | -------------------------------------------------------------------------------- | -------------- |
| **openskills**     | 2.0.0   | OpenSkills CLI for managing Claude Code skills from Anthropic's marketplace      | mise           |
| **ralph**          | 1.0.0   | AI-driven autonomous development system with discovery, planning, and deployment | script         |
| **tmux-workspace** | 2.0.0   | Tmux workspace management with helper scripts and auto-start functionality       | apt            |

### Testing

| Extension      | Version | Description                                           | Install Method |
| -------------- | ------- | ----------------------------------------------------- | -------------- |
| **playwright** | 2.0.0   | Playwright browser automation framework with Chromium | script         |

### Documentation

| Extension    | Version | Description                                                          | Install Method |
| ------------ | ------- | -------------------------------------------------------------------- | -------------- |
| **mdflow**   | 1.0.0   | Multi-backend CLI that transforms markdown into executable AI agents | mise           |
| **spec-kit** | 1.0.0   | GitHub specification kit for AI-powered repository documentation     | script         |

### Cloud Services

| Extension        | Version | Description                                                        | Install Method |
| ---------------- | ------- | ------------------------------------------------------------------ | -------------- |
| **supabase-cli** | 2.0.0   | Supabase CLI for local development, migrations, and edge functions | script         |

### Desktop

| Extension       | Version | Description                                                     | Install Method |
| --------------- | ------- | --------------------------------------------------------------- | -------------- |
| **guacamole**   | 2.0.0   | Apache Guacamole web-based remote desktop gateway (SSH/RDP/VNC) | script         |
| **xfce-ubuntu** | 2.0.0   | XFCE desktop with xRDP remote access for GUI development        | hybrid         |

### Research

| Extension           | Version | Description                                                                 | Install Method |
| ------------------- | ------- | --------------------------------------------------------------------------- | -------------- |
| **ruvnet-research** | 1.0.0   | AI research tools including Goalie and Research-Swarm multi-agent framework | mise           |

---

## How to Use

### Installing Extensions

```bash
# Install a single extension
sindri extension install nodejs

# Install multiple extensions
sindri extension install nodejs python docker

# Install a profile (collection of extensions)
sindri extension install --profile fullstack

# Force reinstall
sindri extension install --force nodejs
```

### Listing Extensions

```bash
# List all available extensions
sindri extension list

# List installed extensions
sindri extension list --installed

# Filter by category
sindri extension list --category languages
```

### Removing Extensions

```bash
# Remove an extension
sindri extension remove nodejs

# Remove with confirmation skip
sindri extension remove --yes nodejs
```

### Upgrading Extensions

```bash
# Upgrade a specific extension
sindri extension upgrade nodejs
```

### Using Profiles

Profiles provide pre-configured extension sets for common use cases:

| Profile           | Extensions                                                      |
| ----------------- | --------------------------------------------------------------- |
| **minimal**       | nodejs, python                                                  |
| **fullstack**     | nodejs, python, docker, nodejs-devtools                         |
| **ai-dev**        | claude-cli, nodejs, python, golang, ai-toolkit, mdflow          |
| **anthropic-dev** | claude-cli, claude-flow-v3, agentic-qe, ralph, ai-toolkit, etc. |
| **systems**       | rust, golang, docker, infra-tools                               |
| **enterprise**    | claude-cli, all languages, jira-mcp, cloud-tools                |
| **devops**        | docker, infra-tools, monitoring, cloud-tools                    |
| **mobile**        | nodejs, linear-mcp, supabase-cli                                |

Configure profiles in `sindri.yaml`:

```yaml
extensions:
  profile: fullstack
  additional:
    - github-cli
    - monitoring
```

---

## Extension Structure

Each V3 extension consists of an `extension.yaml` file that defines its complete specification.

### extension.yaml Schema

```yaml
---
metadata:
  name: example-extension # Required: Extension identifier
  version: 1.0.0 # Required: Semantic version
  description: Example extension # Required: Brief description
  category: languages # Required: Category for grouping
  author: Sindri Team # Optional: Author/maintainer
  license: MIT # Optional: License
  homepage: https://example.com # Optional: Project URL
  dependencies: # Optional: Required extensions
    - nodejs

requirements:
  domains: # Optional: Network domains needed
    - example.com
  diskSpace: 500 # Optional: Disk space in MB
  memory: 256 # Optional: Memory in MB
  installTime: 60 # Optional: Expected install time (seconds)
  secrets: # Optional: Required secrets
    - api_key_name

install:
  method: mise # Required: mise | apt | binary | npm | script | hybrid
  mise: # Method-specific configuration
    configFile: mise.toml
    reshimAfterInstall: true
  # OR
  script:
    path: install.sh
    timeout: 300

configure:
  templates: # Optional: Files to copy/merge
    - source: config.template
      destination: ~/.config/app/config.yaml
      mode: overwrite # overwrite | append | merge | skip-if-exists
  environment: # Optional: Environment variables
    - key: APP_HOME
      value: "$HOME/.app"
      scope: bashrc # bashrc | profile | session

validate:
  commands: # Required: Validation commands
    - name: app
      expectedPattern: "v\\d+\\.\\d+\\.\\d+"

remove:
  paths: # Optional: Paths to remove
    - ~/.app
  confirmation: true # Optional: Require confirmation

upgrade:
  strategy: automatic # automatic | manual | reinstall | none

capabilities: # Optional: Advanced features
  project-init:
    enabled: true
    commands:
      - command: "app init"
        description: "Initialize app project"
  mcp:
    enabled: true
    server:
      command: "npx"
      args: ["@app/mcp-server"]

bom: # Optional: Software Bill of Materials
  tools:
    - name: app
      version: dynamic
      source: mise
      type: cli-tool
      license: MIT
      homepage: https://example.com
      purl: pkg:npm/app
```

### Installation Methods

| Method     | Description                         | Use Case                        |
| ---------- | ----------------------------------- | ------------------------------- |
| **mise**   | Install via mise (dev tool manager) | Language runtimes, npm packages |
| **apt**    | Install via apt package manager     | System packages                 |
| **binary** | Download pre-compiled binary        | GitHub releases                 |
| **npm**    | Install via npm globally            | Node.js packages                |
| **script** | Run custom shell script             | Complex installations           |
| **hybrid** | Combine multiple methods            | Multi-step installations        |

### Configure Modes

| Mode               | Description                                              |
| ------------------ | -------------------------------------------------------- |
| **overwrite**      | Replace file completely (creates backup)                 |
| **append**         | Add to end of file                                       |
| **merge**          | Deep merge for YAML/JSON, marker-based for shell configs |
| **skip-if-exists** | Only create if file does not exist                       |

---

## Extension Loading Mechanisms

Sindri uses a prioritized extension loading system that supports different deployment modes (local development, development Docker images, and production Docker images).

### Loading Priority

When loading extension definitions, Sindri checks the following locations in order:

#### 1. SINDRI_EXT_HOME Path

**When used**: All Docker deployments (production and development)

**How it works**: The `SINDRI_EXT_HOME` environment variable specifies the extensions directory path. Sindri checks both flat structure (bundled extensions) and versioned structure (downloaded extensions):

**Flat structure** (bundled in development images):

```
/opt/sindri/extensions/
├── nodejs/extension.yaml
├── python/extension.yaml
├── docker/extension.yaml
└── ...
```

**Versioned structure** (downloaded in production):

```
${HOME}/.sindri/extensions/
├── nodejs/
│   ├── 2.1.0/
│   │   ├── extension.yaml
│   │   └── scripts/
│   └── 2.0.0/              # Old version kept for rollback
└── python/
    └── 1.3.0/
        ├── extension.yaml
        └── scripts/
```

**Environment variable**:

- `SINDRI_EXT_HOME` - Path to extensions directory

**Docker Deployment Modes**:

| Mode            | Dockerfile       | SINDRI_EXT_HOME              | Extensions Bundled        |
| --------------- | ---------------- | ---------------------------- | ------------------------- |
| **Production**  | `Dockerfile`     | `${HOME}/.sindri/extensions` | No (installed at runtime) |
| **Development** | `Dockerfile.dev` | `/opt/sindri/extensions`     | Yes (bundled in image)    |

The production Dockerfile uses `${HOME}` which expands at runtime to respect the `ALT_HOME=/alt/home/developer` volume mount, ensuring extensions are installed to `/alt/home/developer/.sindri/extensions` in containers.

**Example (Production)**:

```bash
# Inside a production container (built with Dockerfile)
$ echo $SINDRI_EXT_HOME
/alt/home/developer/.sindri/extensions

$ echo $HOME
/alt/home/developer

$ sindri extension install python
# Downloads from GitHub releases
# Installs to: /alt/home/developer/.sindri/extensions/python/1.3.0/
```

**Example (Development)**:

```bash
# Inside a development container (built with Dockerfile.dev)
$ echo $SINDRI_EXT_HOME
/opt/sindri/extensions

$ ls /opt/sindri/extensions/
nodejs/  python/  docker/  rust/  golang/  ...

$ sindri extension install ruby
# Loads from: /opt/sindri/extensions/ruby/extension.yaml
```

#### 2. Fallback: Home Directory (`~/.sindri/extensions/`)

**When used**: When `SINDRI_EXT_HOME` is not set (local development, non-Docker environments)

**How it works**: Falls back to `dirs::home_dir()` or `$HOME` environment variable (never hardcoded paths):

```rust
// Fallback resolution order:
1. dirs::home_dir().join(".sindri/extensions")  // XDG-compliant
2. std::env::var("HOME").map(|h| format!("{}/.sindri/extensions", h))
3. "/alt/home/developer/.sindri/extensions"  // Ultimate fallback
```

**Example**:

```bash
# Local development (SINDRI_EXT_HOME not set)
$ sindri extension install python
# Downloads from GitHub releases
# Installs to: ~/.sindri/extensions/python/1.3.0/
```

#### 3. Development Path (`v3/extensions/`)

**When used**: Local development when running `sindri` from source via `cargo run`

**How it works**: Resolves extension paths relative to the compiled binary's location:

```
sindri-extensions/  (crate)
  ↓ parent
crates/
  ↓ parent
v3/
  ↓ join
v3/extensions/<name>/extension.yaml
```

**Example**:

```bash
# Running from v3/ directory during development
$ cargo run -- extension install nodejs
# Loads from: v3/extensions/nodejs/extension.yaml
```

### Custom Extension Path

You can override the default extension location by setting `SINDRI_EXT_HOME`:

```bash
# Use custom extensions directory
export SINDRI_EXT_HOME=/custom/path/to/extensions
sindri extension install nodejs
# Loads from: /custom/path/to/extensions/nodejs/extension.yaml
```

### Registry Loading

The registry metadata (`registry.yaml` and `profiles.yaml`) is loaded based on `SINDRI_EXT_HOME`:

- If `SINDRI_EXT_HOME` is set: Checks parent directory (e.g., `/opt/sindri/registry.yaml`)
- Otherwise: Downloads from GitHub and caches at `~/.sindri/cache/registry.yaml`

**Example**:

```bash
# Development container (bundled registry)
$ echo $SINDRI_EXT_HOME
/opt/sindri/extensions

$ sindri profile list
# Uses: /opt/sindri/registry.yaml and /opt/sindri/profiles.yaml

# Production container (downloaded registry)
$ echo $SINDRI_EXT_HOME
/alt/home/developer/.sindri/extensions

$ sindri profile list
# Downloads from GitHub, caches at ~/.sindri/cache/registry.yaml
```

### Distribution Flow (Production Mode)

1. Registry loaded from GitHub (cached at `~/.sindri/cache/registry.yaml`)
2. Compatibility matrix checked for CLI version
3. Extension archive downloaded: `https://github.com/sindri/sindri-extensions/releases/download/python@1.3.0/python-1.3.0.tar.gz`
4. Archive extracted to: `${SINDRI_EXT_HOME}/python/1.3.0/`
5. Extension definition loaded from extracted location

**Example**:

```bash
# Inside a production container
$ sindri extension install python
# Downloads from GitHub releases
# Extracts to: /alt/home/developer/.sindri/extensions/python/1.3.0/
# Loads: ~/.sindri/extensions/python/1.3.0/extension.yaml
```

### Deployment Comparison

| Aspect                    | Build-from-Source                      | Release-Based                            |
| ------------------------- | -------------------------------------- | ---------------------------------------- |
| **Extension definitions** | `/opt/sindri/extensions/`              | `~/.sindri/extensions/<name>/<version>/` |
| **Registry source**       | `/opt/sindri/registry.yaml`            | GitHub → `~/.sindri/cache/registry.yaml` |
| **Version management**    | Single version (baked in at build)     | Multiple versions with rollback support  |
| **Network dependency**    | None (files copied at build time)      | First install requires GitHub access     |
| **Update mechanism**      | Requires image rebuild                 | `sindri extension upgrade`               |
| **Disk usage**            | Smaller (single version per extension) | Larger (keeps old versions for rollback) |
| **Offline support**       | Full (all files local)                 | Partial (cached files work offline)      |
| **Use case**              | Development, edge, air-gapped          | Production, cloud, CI/CD                 |
| **Set via config**        | `deployment.buildFromSource.enabled`   | `deployment.image: ghcr.io/...`          |

### Manual Extension Installation in Containers

Users can SSH into deployed containers and install additional extensions.

> **DinD Mode Requirement:** Extensions using `apt` packages require sudo, which works in `none`, `sysbox`, and `privileged` DinD modes. In `socket` mode (production security), sudo is blocked by `no-new-privileges`. See [Docker Provider documentation](providers/DOCKER.md#security-model-by-dind-mode) for details.

**Build-from-Source containers**:

```bash
# SSH into container
$ ssh developer@sindri-container

# Extensions load from baked-in source files
$ sindri extension install golang
# ✓ Registry loaded from: /opt/sindri/registry.yaml
# ✓ Extension loaded from: /opt/sindri/extensions/golang/extension.yaml
# ✓ Installs via method defined in extension.yaml (mise/apt/script/etc.)
```

**Release containers**:

```bash
# SSH into container
$ ssh developer@sindri-container

# Extensions download from GitHub
$ sindri extension install golang
# → Fetches registry from GitHub (cached 1 hour)
# → Checks compatibility matrix for CLI version
# → Downloads: golang@1.0.1.tar.gz from GitHub releases
# → Extracts to: ~/.sindri/extensions/golang/1.0.1/
# → Installs via method defined in extension.yaml
```

### Registry Loading Priority

Similar to extensions, the registry (list of available extensions and profiles) uses a prioritized loading system:

1. **Source files** (if `SINDRI_BUILD_FROM_SOURCE=true`):
   - `/opt/sindri/registry.yaml`
   - `/opt/sindri/profiles.yaml`

2. **Cached files** (if valid and fresh):
   - `~/.sindri/cache/registry.yaml` (1-hour TTL)
   - `~/.sindri/cache/profiles.yaml`

3. **GitHub download**:
   - `https://raw.githubusercontent.com/pacphi/sindri/main/v3/registry.yaml`
   - `https://raw.githubusercontent.com/pacphi/sindri/main/v3/profiles.yaml`

4. **Expired cache** (fallback if GitHub unreachable):
   - Uses expired cached files as last resort

### Environment Variables Reference

| Variable                   | Description                     | Example                  | Set By                 |
| -------------------------- | ------------------------------- | ------------------------ | ---------------------- |
| `SINDRI_BUILD_FROM_SOURCE` | Signals source-based deployment | `true`                   | Docker ENV / templates |
| `SINDRI_SOURCE_REF`        | Git reference used for build    | `main`, `v3.0.0`         | Docker ARG             |
| `SINDRI_EXTENSIONS_SOURCE` | Path to source-based extensions | `/opt/sindri/extensions` | Docker ENV             |

These variables are automatically set during deployment when using `buildFromSource.enabled: true` in `sindri.yaml`.

---

## See Also

- [EXTENSION_AUTHORING.md](EXTENSION_AUTHORING.md) - Guide to creating new extensions
- [CONFIGURATION.md](CONFIGURATION.md) - Sindri configuration reference
- [Architecture Decision Records](architecture/adr/README.md):
  - [ADR-008: Extension Type System](architecture/adr/008-extension-type-system-yaml-deserialization.md)
  - [ADR-011: Multi-Method Installation](architecture/adr/011-multi-method-extension-installation.md)
  - [ADR-032: Configure Processing](architecture/adr/032-extension-configure-processing.md)

---

## Summary Statistics

- **Total Extensions**: 48
- **Categories**: 13
- **Installation Methods Used**: 6 (mise, apt, binary, npm, script, hybrid)
- **Extensions with project-init**: 11
- **Extensions with MCP integration**: 6
- **Extensions with dependencies**: 31
