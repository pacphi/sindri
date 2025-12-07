# Sindri

[![License](https://img.shields.io/github/license/pacphi/sindri)](LICENSE)
[![CI](https://github.com/pacphi/sindri/actions/workflows/ci.yml/badge.svg)](https://github.com/pacphi/sindri/actions/workflows/ci.yml)

A declarative, provider-agnostic cloud development environment system. Deploy consistent development environments to Fly.io, local Docker, or via DevPod to Kubernetes, AWS, GCP, Azure, and other cloud providers using YAML-defined extensions.

```text
   ███████╗██╗███╗   ██╗██████╗ ██████╗ ██╗
   ██╔════╝██║████╗  ██║██╔══██╗██╔══██╗██║
   ███████╗██║██╔██╗ ██║██║  ██║██████╔╝██║
   ╚════██║██║██║╚██╗██║██║  ██║██╔══██╗██║
   ███████║██║██║ ╚████║██████╔╝██║  ██║██║
   ╚══════╝╚═╝╚═╝  ╚═══╝╚═════╝ ╚═╝  ╚═╝╚═╝

   🔨 Forging Development Environments
   📦 https://github.com/pacphi/sindri
```

## About the Name

**Sindri** (Old Norse: "spark") was a legendary dwarf blacksmith in Norse mythology, renowned for forging three of the most powerful artifacts: Mjölnir (Thor's hammer), Draupnir (Odin's ring), and Gullinbursti (Freyr's golden boar).

Like its mythological namesake, Sindri forges powerful development environments from raw materials—transforming cloud infrastructure, YAML configuration, and Docker into consistent, reproducible developer workspaces.

## Quick Start

```bash
# Clone repository
git clone https://github.com/pacphi/sindri
cd sindri

# Initialize configuration
./cli/sindri config init

# Edit sindri.yaml for your needs
# See examples/ directory for templates

# Deploy locally
./cli/sindri deploy --provider docker

# Or deploy to Fly.io
./cli/sindri deploy --provider fly
```

**Prerequisites:** [Docker](https://www.docker.com/get-started/), [yq](https://github.com/mikefarah/yq). For Fly.io: [flyctl](https://fly.io/docs/flyctl/install/) CLI. For DevPod: [devpod](https://devpod.sh/) CLI.

## Core Features

- **Modular Extension System** - YAML-driven with dependency resolution
- **Fast Startup** - Optimized Docker images with pre-installed tools (10-15s cold start)
- **Extension System** - 32 modular extensions for languages, tools, and infrastructure
- **Schema Validation** - All YAML validated against JSON schemas
- **Provider Adapters** - Clean abstraction for Docker, Fly.io, and DevPod (with Kubernetes, AWS, GCP, Azure backends)
- **Volume Architecture** - Immutable `/docker/lib` system, mutable `$HOME` volume containing workspace
- **BOM Tracking** - Comprehensive software bill of materials for security auditing

## Documentation

### Getting Started

- **[Quickstart Guide](docs/QUICKSTART.md)** - Fast setup and deployment
- **[Architecture Overview](docs/ARCHITECTURE.md)** - System design and concepts
- **[Configuration Reference](docs/CONFIGURATION.md)** - Complete sindri.yaml guide

### Extensions

- **[Extension Catalog](docs/EXTENSIONS.md)** - Available extensions and usage
- **[Extension Authoring](docs/EXTENSION_AUTHORING.md)** - Creating custom extensions
- **[Bill of Materials](docs/BOM.md)** - Software tracking and SBOM generation

### Deployment

- **[Deployment Overview](docs/DEPLOYMENT.md)** - Provider comparison and selection
- **[Fly.io Deployment](docs/providers/FLY.md)** - Fly.io-specific guide
- **[DevPod Integration](docs/providers/DEVPOD.md)** - DevContainer setup
- **[Docker Deployment](docs/providers/DOCKER.md)** - Local Docker setup
- **[Kubernetes Deployment](docs/providers/KUBERNETES.md)** - Enterprise K8s guide
- **[Secrets Management](docs/SECRETS_MANAGEMENT.md)** - Managing secrets across providers

### Development & Operations

- **[Project Management](docs/PROJECT_MANAGEMENT.md)** - Using new-project and clone-project
- **[Contributing Guide](docs/CONTRIBUTING.md)** - Development workflow and standards
- **[Testing Guide](docs/TESTING.md)** - Running tests and CI/CD
- **[Workflow Architecture](.github/WORKFLOW_ARCHITECTURE.md)** - CI/CD workflow structure and design
- **[CI Testing Deep Dive](docs/CI_WORKFLOW_IN_DEPTH.md)** - Comprehensive CI testing guide
- **[Release Process](docs/RELEASE.md)** - Creating releases and changelog automation
- **[Troubleshooting](docs/TROUBLESHOOTING.md)** - Common issues and solutions
- **[Security](docs/SECURITY.md)** - Security best practices

### Claude Code Integration

A Claude Code skill is available to guide extension development. When using Claude Code, ask about creating extensions and it will automatically provide guidance. See [Sindri Extension Guide](.claude/skills/sindri-extension-guide/SKILL.md) for the skill definition.

```text
Example questions:
- "Help me create a new extension for Lua development"
- "What fields are required in extension.yaml?"
- "How do I use the apt installation method?"
- "Show me an example of a script-based extension"
```

## License

MIT License - see [LICENSE](LICENSE) file for details.
