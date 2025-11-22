# Extension Catalog

Comprehensive guide to all available Sindri extensions.

## Extension System Overview

Extensions are YAML-defined packages that install and configure development tools. Each extension:

- Declares metadata, dependencies, and requirements
- Uses declarative installation methods (mise, apt, npm, binary, script)
- Validates successful installation
- Integrates with the extension manifest system

## Extension Profiles

Pre-configured bundles for common workflows:

| Profile        | Extensions                                         | Use Case                         |
| -------------- | -------------------------------------------------- | -------------------------------- |
| **minimal**    | nodejs, python                                     | Lightweight scripting            |
| **fullstack**  | nodejs, python, docker, postgres, nodejs-devtools  | Web development                  |
| **ai-dev**     | nodejs, python, ai-toolkit, openskills, monitoring | AI development                   |
| **systems**    | rust, golang, docker, infra-tools                  | Systems programming              |
| **enterprise** | All languages + infrastructure                     | Complete development environment |

### Using Profiles

```yaml
# sindri.yaml
extensions:
  profile: fullstack
```

Or install via CLI:

```bash
extension-manager install-profile fullstack
```

## Base System Extensions

Pre-installed in all environments:

### workspace-structure

**Category:** base
**Description:** Base directory structure for user workspace

Creates:

- `/workspace/projects` - User projects
- `/workspace/config` - Configuration files
- `/workspace/bin` - User binaries
- `/workspace/.local` - Local installations
- `/workspace/.config` - Tool configs
- `/workspace/.system` - Extension state

**No installation required** - automatically configured at first boot.

### mise-config

**Category:** base
**Description:** mise tool version manager configuration

Provides declarative tool management via [mise](https://mise.jdx.dev/).

**Pre-installed** in base image.

## Language Runtimes

### nodejs

**Category:** language
**Installation:** mise
**Disk Space:** 600 MB

Node.js LTS runtime with npm.

**Installed tools:**

- `node` - Node.js runtime
- `npm` - Package manager

**Configuration:**

```yaml
# mise.toml
[tools]
node = "lts"
```

### python

**Category:** language
**Installation:** mise
**Disk Space:** 800 MB

Python 3.13 with pipx tools.

**Installed tools:**

- `python` - Python 3.13
- `pip` - Package installer
- `pipx` - Install Python applications
- `virtualenv` - Virtual environments
- `poetry` - Dependency management
- `flake8` - Linter
- `mypy` - Type checker
- `black` - Code formatter
- `jupyterlab` - Interactive notebooks
- `uv` - Fast Python package installer

### golang

**Category:** language
**Installation:** mise
**Disk Space:** 500 MB

Go 1.24 with development tools.

**Installed tools:**

- `go` - Go compiler
- `gopls` - Language server
- `delve` - Debugger
- `goimports` - Import formatter
- `golangci-lint` - Linter
- `air` - Live reload
- `goreleaser` - Release automation

### rust

**Category:** language
**Installation:** mise
**Disk Space:** 1000 MB

Rust stable toolchain with cargo utilities.

**Installed tools:**

- `rustc` - Rust compiler
- `cargo` - Package manager
- `rustfmt` - Code formatter
- `clippy` - Linter
- `ripgrep` - Fast grep alternative
- `fd-find` - Fast find alternative
- `exa` - Modern ls replacement
- `bat` - Cat with syntax highlighting
- `tokei` - Code statistics

### ruby

**Category:** language
**Installation:** mise
**Disk Space:** 400 MB

Ruby 3.4 with gems.

**Installed tools:**

- `ruby` - Ruby interpreter
- `gem` - Package manager
- `bundler` - Dependency management
- `rubocop` - Linter and formatter

**Configuration:**

```yaml
# mise.toml
[tools]
ruby = "3.4"
```

### jvm

**Category:** language
**Installation:** script (SDKMAN)
**Disk Space:** 800 MB

Java, Kotlin, Scala via SDKMAN.

**Installed tools:**

- `java` - Java 21 LTS
- `javac` - Java compiler
- `gradle` - Build tool
- `maven` - Build tool
- `kotlin` - Kotlin compiler
- `scala` - Scala compiler

**Dependencies:** None

### dotnet

**Category:** language
**Installation:** script (Microsoft installer)
**Disk Space:** 1200 MB

.NET SDK 9.0 and 8.0.

**Installed tools:**

- `dotnet` - .NET CLI
- SDK 9.0 (current)
- SDK 8.0 (LTS)

**Configuration:**

- `Directory.Build.props` - Global MSBuild props
- `nuget.config` - NuGet configuration
- `.editorconfig` - Editor settings
- `global.json` - SDK version pinning

### php

**Category:** language
**Installation:** apt + script
**Disk Space:** 300 MB

PHP 8.4 with Composer.

**Installed tools:**

- `php` - PHP 8.4
- `composer` - Dependency manager
- `php-cs-fixer` - Code style fixer

**Configuration:**

```ini
# development.ini
display_errors = On
error_reporting = E_ALL
```

## Development Tools

### nodejs-devtools

**Category:** dev-tools
**Installation:** mise (npm)
**Dependencies:** nodejs
**Disk Space:** 300 MB

Node.js development tools.

**Installed tools:**

- `typescript` - TypeScript compiler
- `eslint` - JavaScript linter
- `prettier` - Code formatter
- `nodemon` - Auto-restart server
- `goalie` - Package update manager

**Configuration:**

```yaml
# .eslintrc.yml
extends:
  - eslint:recommended
```

### github-cli

**Category:** dev-tools
**Installation:** apt
**Disk Space:** 50 MB

GitHub CLI for repository management.

**Installed tools:**

- `gh` - GitHub CLI

**Usage:**

```bash
gh auth login
gh repo create
gh pr create
```

### docker

**Category:** infrastructure
**Installation:** apt (Docker Engine)
**Disk Space:** 500 MB

Docker Engine for containerization.

**Installed tools:**

- `docker` - Docker CLI
- `docker-compose` - Multi-container orchestration

**Configuration:** Docker daemon runs as service.

### playwright

**Category:** dev-tools
**Installation:** npm
**Dependencies:** nodejs
**Disk Space:** 800 MB

Browser automation testing framework.

**Installed tools:**

- `playwright` - Browser automation
- Chromium, Firefox, WebKit browsers

**Configuration:**

```typescript
// playwright.config.ts
export default {
  testDir: "./tests",
  use: {
    headless: true,
  },
};
```

## AI Tools

### ai-toolkit

**Category:** ai
**Installation:** hybrid (multiple methods)
**Dependencies:** nodejs, python
**Disk Space:** 2000 MB

Comprehensive AI coding assistants.

**Installed tools:**

- **Codex** - OpenAI Codex integration
- **Gemini** - Google Gemini CLI
- **Ollama** - Local LLM runtime
- **Fabric** - AI pattern executor
- **Hector** - Code review assistant
- **Droid** - Android AI helper
- **AWS Q** - AWS code assistant
- **GitHub Copilot** - GitHub Copilot CLI
- **Grok** - xAI Grok integration

**Configuration:** Each tool has dedicated setup. See [ai-toolkit/README.md](../docker/lib/extensions/ai-toolkit/README.md)

### openskills

**Category:** ai
**Installation:** script
**Dependencies:** nodejs
**Disk Space:** 100 MB

Claude Code skills management system.

**Installed tools:**

- `openskills` - Skills CLI for Claude Code

**Usage:**

```bash
openskills install skill-name
openskills list
openskills update skill-name
```

### claude-marketplace

**Category:** ai
**Installation:** script
**Dependencies:** None
**Disk Space:** 50 MB

Claude Code plugin marketplace integration.

**Configuration:**

- `marketplaces.yml` - Plugin sources
- `default-settings.json` - Default Claude settings

### claude-auth-with-api-key

**Category:** ai
**Installation:** script
**Dependencies:** None
**Disk Space:** 10 MB

Claude Code API key authentication wrapper.

**Usage:** Automatically wraps `claude` command to use API key instead of session auth.

**Environment:** Requires `ANTHROPIC_API_KEY` secret.

### agent-manager

**Category:** ai
**Installation:** script
**Dependencies:** None
**Disk Space:** 20 MB

Agent orchestration and management.

**Installed tools:**

- Agent search and discovery
- Agent execution tracking

## Infrastructure Tools

### infra-tools

**Category:** infrastructure
**Installation:** mise
**Disk Space:** 1000 MB

Infrastructure as code and orchestration tools.

**Installed tools:**

- `terraform` - Infrastructure provisioning
- `ansible` - Configuration management
- `kubectl` - Kubernetes CLI
- `helm` - Kubernetes package manager
- `k9s` - Kubernetes TUI
- `kubectx` - Kubernetes context switcher

**Configuration:**

```yaml
# mise.toml
[tools]
terraform = "latest"
kubectl = "latest"
helm = "latest"
```

### cloud-tools

**Category:** infrastructure
**Installation:** hybrid (apt + script)
**Disk Space:** 800 MB

Cloud provider CLIs.

**Installed tools:**

- `aws` - AWS CLI
- `az` - Azure CLI
- `gcloud` - Google Cloud CLI

**SSH Configuration:** Includes SSH wrapper for cloud instances.

## Monitoring & Utilities

### monitoring

**Category:** monitoring
**Installation:** apt
**Disk Space:** 200 MB

System monitoring and observability tools.

**Installed tools:**

- `htop` - Process viewer
- `iotop` - I/O monitor
- `nethogs` - Network monitor
- `glances` - System monitor
- `dstat` - System statistics

### tmux-workspace

**Category:** utilities
**Installation:** apt + script
**Disk Space:** 50 MB

Tmux session management for persistent workflows.

**Installed tools:**

- `tmux` - Terminal multiplexer
- Workspace management scripts

**Configuration:**

```bash
# Auto-start tmux on login
tmux attach || tmux new-session
```

**Aliases:**

```bash
tw-create <name>   # Create workspace
tw-switch <name>   # Switch workspace
tw-list            # List workspaces
```

### guacamole

**Category:** utilities
**Installation:** hybrid
**Disk Space:** 400 MB

Apache Guacamole for remote desktop access (VNC/RDP).

**Installed tools:**

- Guacamole server
- VNC/RDP support

**Configuration:**

- `guacamole.properties` - Server config
- `user-mapping.xml` - User authentication

### xfce-ubuntu

**Category:** utilities
**Installation:** apt
**Disk Space:** 1500 MB

XFCE desktop environment for graphical applications.

**Installed tools:**

- XFCE desktop
- VNC server for remote access
- Common GUI applications

## Utility Aliases

### ruvnet-aliases

**Category:** utilities
**Installation:** script
**Disk Space:** 5 MB

Workflow automation aliases for agentic and Claude flows.

**Installed aliases:**

- Agentic flow commands
- Claude flow shortcuts
- Productivity helpers

**Source:** [ruvnet/agentic-flow](https://github.com/ruvnet/agentic-flow) and [ruvnet/claude-flow](https://github.com/ruvnet/claude-flow)

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

### Extension Status

```bash
extension-manager status nodejs
```

### Remove Extensions

```bash
extension-manager remove nodejs
```

## Extension Dependencies

Some extensions require other extensions to be installed first. Dependencies are automatically resolved:

- **nodejs-devtools** → requires nodejs
- **playwright** → requires nodejs
- **openskills** → requires nodejs
- **cloud-tools** → provides SSH environment templates

## Creating Custom Extensions

See [Extension Authoring Guide](EXTENSION_AUTHORING.md) for creating your own extensions.

## Extension Storage

- Extension definitions: `/docker/lib/extensions/`
- Installed manifest: `/workspace/.system/manifest/`
- Extension logs: `/workspace/.system/logs/`
- BOM tracking: `/workspace/.system/bom/`

## Related Documentation

- [Extension Authoring](EXTENSION_AUTHORING.md)
- [Bill of Materials](BOM.md)
- [Architecture](ARCHITECTURE.md)
