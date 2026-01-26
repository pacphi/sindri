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

| Category          | Description                                      | Count |
| ----------------- | ------------------------------------------------ | ----- |
| **languages**     | Programming language runtimes and toolchains     | 10    |
| **devops**        | Infrastructure, deployment, and operations tools | 4     |
| **claude**        | Claude Code integrations and workflow tools      | 7     |
| **ai-agents**     | AI agent frameworks and orchestration            | 4     |
| **ai-dev**        | AI development tools and LLM interfaces          | 3     |
| **mcp**           | Model Context Protocol servers                   | 5     |
| **productivity**  | Developer productivity and workflow tools        | 4     |
| **testing**       | Testing frameworks and automation                | 1     |
| **documentation** | Documentation generation and management          | 2     |
| **cloud**         | Cloud service integrations                       | 1     |
| **desktop**       | Desktop environments and remote access           | 2     |
| **research**      | Research and analysis tools                      | 1     |

---

## Extension List

### Languages

| Extension           | Version | Description                                                           | Install Method |
| ------------------- | ------- | --------------------------------------------------------------------- | -------------- |
| **nodejs**          | 1.1.0   | Node.js LTS via mise with pnpm package manager                        | hybrid         |
| **python**          | 1.1.1   | Python 3.13 with uv package manager via mise                          | mise           |
| **rust**            | 1.0.2   | Rust stable via rustup                                                | script         |
| **golang**          | 1.0.1   | Go 1.25 via mise                                                      | mise           |
| **dotnet**          | 2.1.0   | .NET SDK 10.0 and 8.0 with ASP.NET Core and development tools         | script         |
| **jvm**             | 2.0.0   | JVM languages (Java, Kotlin, Scala) with SDKMAN and Clojure/Leiningen | script         |
| **php**             | 2.1.0   | PHP 8.4 with Composer, Symfony CLI, and development tools             | script         |
| **ruby**            | 2.0.0   | Ruby 3.4.7 via mise with Rails and Bundler                            | script         |
| **haskell**         | 1.0.1   | Haskell development environment with GHC, Cabal, Stack, and HLS       | mise           |
| **nodejs-devtools** | 2.2.0   | TypeScript, ESLint, Prettier, and Node.js development tools           | mise           |

### DevOps

| Extension       | Version | Description                                                               | Install Method |
| --------------- | ------- | ------------------------------------------------------------------------- | -------------- |
| **docker**      | 1.1.0   | Docker Engine and Compose with Docker-in-Docker support                   | hybrid         |
| **infra-tools** | 2.0.0   | Infrastructure and DevOps tooling (Terraform, K8s, Ansible, Helm)         | hybrid         |
| **github-cli**  | 2.0.0   | GitHub CLI authentication and workflow configuration                      | script         |
| **cloud-tools** | 2.0.0   | Cloud provider CLI tools (AWS, Azure, GCP, Fly.io, OCI, Alibaba, DO, IBM) | script         |

### Claude Code Integrations

| Extension              | Version | Description                                                                         | Install Method |
| ---------------------- | ------- | ----------------------------------------------------------------------------------- | -------------- |
| **claude-flow-v3**     | 3.0.0   | Next-gen multi-agent orchestration with 10x performance, 150x faster search (alpha) | mise           |
| **claude-flow-v2**     | 2.7.47  | AI-powered multi-agent orchestration system for Claude Code workflows (stable)      | mise           |
| **claude-codepro**     | 4.5.29  | Production-grade TDD-enforced development environment with automated quality checks | script         |
| **claude-code-mux**    | 1.0.0   | High-performance AI routing proxy with automatic failover across 18+ providers      | script         |
| **claudish**           | 1.0.0   | Claude Code CLI proxy for OpenRouter models via local Anthropic API proxy           | mise           |
| **claudeup**           | 1.0.0   | TUI tool for managing Claude Code plugins, MCPs, and configuration settings         | mise           |
| **claude-marketplace** | 2.0.0   | Claude Code plugin marketplace integration via YAML configuration                   | script         |

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
| **pal-mcp-server** | 9.8.2   | AI orchestration and multi-model collaboration MCP server with 18 tools       | script         |
| **monitoring**     | 2.0.0   | Claude monitoring and usage tracking tools (claude-monitor, claude-usage-cli) | script         |

### Productivity

| Extension          | Version | Description                                                                      | Install Method |
| ------------------ | ------- | -------------------------------------------------------------------------------- | -------------- |
| **mise-config**    | 2.0.0   | Global mise configuration and settings                                           | script         |
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

# Upgrade all extensions
sindri extension upgrade --all
```

### Using Profiles

Profiles provide pre-configured extension sets for common use cases:

| Profile           | Extensions                                             |
| ----------------- | ------------------------------------------------------ |
| **minimal**       | nodejs, python                                         |
| **fullstack**     | nodejs, python, docker, nodejs-devtools                |
| **ai-dev**        | nodejs, python, golang, ai-toolkit, mdflow, openskills |
| **anthropic-dev** | claude-flow, agentic-flow, ai-toolkit                  |
| **systems**       | rust, golang, docker, infra-tools                      |
| **enterprise**    | All languages + jira-mcp, cloud-tools                  |
| **devops**        | docker, infra-tools, monitoring, cloud-tools           |
| **mobile**        | nodejs, linear-mcp, supabase-cli                       |

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

## See Also

- [EXTENSION_AUTHORING.md](EXTENSION_AUTHORING.md) - Guide to creating new extensions
- [CONFIGURATION.md](CONFIGURATION.md) - Sindri configuration reference
- [Architecture Decision Records](architecture/adr/README.md):
  - [ADR-008: Extension Type System](architecture/adr/008-extension-type-system-yaml-deserialization.md)
  - [ADR-011: Multi-Method Installation](architecture/adr/011-multi-method-extension-installation.md)
  - [ADR-032: Configure Processing](architecture/adr/032-extension-configure-processing.md)

---

## Summary Statistics

- **Total Extensions**: 44
- **Categories**: 12
- **Installation Methods Used**: 6 (mise, apt, binary, npm, script, hybrid)
- **Extensions with project-init**: 10
- **Extensions with MCP integration**: 3
- **Extensions with dependencies**: 20
