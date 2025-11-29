# Extension Catalog

Comprehensive guide to all available Sindri extensions. Each extension is documented in detail in the [extensions/](extensions/) directory.

## Extension System Overview

Extensions are YAML-defined packages that install and configure development tools. Each extension:

- Declares metadata, dependencies, and requirements
- Uses declarative installation methods (mise, apt, npm, binary, script, hybrid)
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

| Profile           | Extensions                                                                                                                                                 | Use Case                     |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- |
| **minimal**       | nodejs, python                                                                                                                                             | Lightweight scripting        |
| **fullstack**     | nodejs, python, docker, nodejs-devtools                                                                                                                    | Web development              |
| **ai-dev**        | nodejs, python, ai-toolkit, openskills, monitoring                                                                                                         | AI/ML development            |
| **anthropic-dev** | agent-manager, ai-toolkit, claude-code-mux, claude-marketplace, cloud-tools, openskills, nodejs-devtools, playwright, rust, ruvnet-aliases, tmux-workspace | Anthropic/Claude development |
| **systems**       | rust, golang, docker, infra-tools                                                                                                                          | Systems programming          |
| **enterprise**    | All languages + infrastructure                                                                                                                             | Complete environment         |
| **data-science**  | python, monitoring                                                                                                                                         | Data analysis                |
| **devops**        | docker, infra-tools, cloud-tools, monitoring                                                                                                               | Infrastructure               |
| **mobile**        | nodejs                                                                                                                                                     | Mobile development (WIP)     |

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

| Extension           | Description               | Docs                                                        |
| ------------------- | ------------------------- | ----------------------------------------------------------- |
| workspace-structure | Base directory structure  | [WORKSPACE-STRUCTURE.md](extensions/WORKSPACE-STRUCTURE.md) |
| mise-config         | Mise tool version manager | [MISE-CONFIG.md](extensions/MISE-CONFIG.md)                 |

### Language Runtimes

| Extension | Language                  | Version    | Docs                              |
| --------- | ------------------------- | ---------- | --------------------------------- |
| nodejs    | Node.js                   | LTS        | [NODEJS.md](extensions/NODEJS.md) |
| python    | Python                    | 3.13       | [PYTHON.md](extensions/PYTHON.md) |
| golang    | Go                        | 1.24       | [GOLANG.md](extensions/GOLANG.md) |
| rust      | Rust                      | stable     | [RUST.md](extensions/RUST.md)     |
| ruby      | Ruby                      | 3.4.7      | [RUBY.md](extensions/RUBY.md)     |
| jvm       | Java/Kotlin/Scala/Clojure | Java 21    | [JVM.md](extensions/JVM.md)       |
| dotnet    | .NET                      | 10.0 & 8.0 | [DOTNET.md](extensions/DOTNET.md) |
| php       | PHP                       | 8.4        | [PHP.md](extensions/PHP.md)       |

### Development Tools

| Extension          | Purpose                        | Docs                                                      |
| ------------------ | ------------------------------ | --------------------------------------------------------- |
| nodejs-devtools    | TypeScript, ESLint, Prettier   | [NODEJS-DEVTOOLS.md](extensions/NODEJS-DEVTOOLS.md)       |
| github-cli         | GitHub CLI (`gh`)              | [GITHUB-CLI.md](extensions/GITHUB-CLI.md)                 |
| playwright         | Browser automation testing     | [PLAYWRIGHT.md](extensions/PLAYWRIGHT.md)                 |
| tmux-workspace     | Terminal multiplexer workspace | [TMUX-WORKSPACE.md](extensions/TMUX-WORKSPACE.md)         |
| claude-marketplace | Claude Code plugin marketplace | [CLAUDE-MARKETPLACE.md](extensions/CLAUDE-MARKETPLACE.md) |

### AI Tools

| Extension                | Purpose                                     | Docs                                                                  |
| ------------------------ | ------------------------------------------- | --------------------------------------------------------------------- |
| ai-toolkit               | AI CLI tools (Ollama, Gemini, Fabric, etc.) | [AI-TOOLKIT.md](extensions/AI-TOOLKIT.md)                             |
| openskills               | Claude Code skills manager                  | [OPENSKILLS.md](extensions/OPENSKILLS.md)                             |
| claude-code-mux          | AI routing proxy (18+ providers)            | [CLAUDE-CODE-MUX.md](extensions/CLAUDE-CODE-MUX.md)                   |
| claude-auth-with-api-key | Claude API key authentication               | [CLAUDE-AUTH-WITH-API-KEY.md](extensions/CLAUDE-AUTH-WITH-API-KEY.md) |
| agent-manager            | AI agent orchestration                      | [AGENT-MANAGER.md](extensions/AGENT-MANAGER.md)                       |
| ruvnet-aliases           | Claude Flow & Agentic Flow aliases          | [RUVNET-ALIASES.md](extensions/RUVNET-ALIASES.md)                     |

### Infrastructure

| Extension   | Purpose                                          | Docs                                        |
| ----------- | ------------------------------------------------ | ------------------------------------------- |
| docker      | Docker Engine & Compose                          | [DOCKER.md](extensions/DOCKER.md)           |
| infra-tools | Terraform, Kubernetes, Ansible, Pulumi + 10 more | [INFRA-TOOLS.md](extensions/INFRA-TOOLS.md) |
| cloud-tools | AWS, Azure, GCP, OCI, Alibaba, DO, IBM CLIs      | [CLOUD-TOOLS.md](extensions/CLOUD-TOOLS.md) |
| monitoring  | Claude usage monitoring (uv, claude-monitor)     | [MONITORING.md](extensions/MONITORING.md)   |

### Desktop & Utilities

| Extension   | Purpose                                | Docs                                        |
| ----------- | -------------------------------------- | ------------------------------------------- |
| guacamole   | Web-based remote desktop (SSH/RDP/VNC) | [GUACAMOLE.md](extensions/GUACAMOLE.md)     |
| xfce-ubuntu | XFCE desktop with xRDP                 | [XFCE-UBUNTU.md](extensions/XFCE-UBUNTU.md) |

## Extension Features

### Upgrade Strategies

Extensions support different upgrade approaches:

| Strategy    | Description               | Extensions                                                                        |
| ----------- | ------------------------- | --------------------------------------------------------------------------------- |
| `automatic` | Auto-upgrade via mise/apt | dotnet, ruby, nodejs-devtools, monitoring, xfce-ubuntu, agent-manager, openskills |
| `manual`    | Custom upgrade script     | ai-toolkit, cloud-tools, jvm, infra-tools, claude-code-mux, playwright, guacamole |
| `none`      | No upgrades (static)      | github-cli, claude-marketplace, ruvnet-aliases, workspace-structure, mise-config  |

### Secret Requirements

Some extensions require API keys or credentials:

| Extension                | Required Secrets                        |
| ------------------------ | --------------------------------------- |
| ai-toolkit               | `google_gemini_api_key`, `grok_api_key` |
| cloud-tools              | AWS, Azure credentials                  |
| claude-auth-with-api-key | `anthropic_api_key`                     |
| github-cli               | `github_token`                          |
| nodejs-devtools          | `perplexity_api_key` (optional)         |

### Removal Confirmation

These extensions require confirmation before removal (destructive operation):

- docker
- infra-tools
- cloud-tools
- claude-code-mux
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
```

## Extension Storage

| Location                       | Purpose                       |
| ------------------------------ | ----------------------------- |
| `/docker/lib/extensions/`      | Extension definitions (YAML)  |
| `/workspace/.system/manifest/` | Installed extension manifests |
| `/workspace/.system/logs/`     | Extension installation logs   |
| `/workspace/.system/bom/`      | Bill of Materials tracking    |

## Related Documentation

- [Extension Authoring](EXTENSION_AUTHORING.md) - Create custom extensions
- [Architecture](ARCHITECTURE.md) - System architecture
- [Configuration](CONFIGURATION.md) - sindri.yaml configuration
