# Sindri

[![License](https://img.shields.io/github/license/pacphi/sindri)](LICENSE)
[![Integration Tests](https://github.com/pacphi/sindri/actions/workflows/integration.yml/badge.svg)](https://github.com/pacphi/sindri/actions/workflows/integration.yml)
[![Validation](https://github.com/pacphi/sindri/actions/workflows/validation.yml/badge.svg)](https://github.com/pacphi/sindri/actions/workflows/validation.yml)

A declarative, provider-agnostic cloud development environment system. Deploy consistent development environments to Fly.io, Kubernetes, or local Docker using YAML-defined extensions.

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

**Prerequisites:** Docker, yq. For Fly.io: flyctl. For DevPod: devpod CLI.

## Core Features

- **YAML-First Architecture** - Declarative extension definitions with JSON schema validation
- **Provider-Agnostic** - Single config deploys to Docker, Fly.io, Kubernetes, or DevPod
- **Fast Startup** - Optimized Docker images with pre-installed tools (10-15s cold start)
- **Extension System** - 27+ modular extensions for languages, tools, and infrastructure
- **Immutable/Mutable Split** - System files baked into images, user workspace fully writable

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
- **[Fly.io Deployment](docs/FLY_DEPLOYMENT.md)** - Fly.io-specific guide
- **[DevPod Integration](docs/DEVPOD_INTEGRATION.md)** - DevContainer setup
- **[Secrets Management](docs/SECRETS_MANAGEMENT.md)** - Managing secrets across providers

### Project Management

- **[Project Management](docs/PROJECT_MANAGEMENT.md)** - Using new-project and clone-project

### Developer Documentation

- **[Contributing Guide](docs/CONTRIBUTING.md)** - Development workflow and standards
- **[Testing Guide](docs/TESTING.md)** - Running tests and CI/CD
- **[Troubleshooting](docs/TROUBLESHOOTING.md)** - Common issues and solutions
- **[Security](docs/SECURITY.md)** - Security best practices

## Extension Profiles

Sindri includes pre-configured profiles for common development scenarios:

- **minimal** - Node.js, Python basics
- **fullstack** - Node.js, Python, Docker, Postgres, dev tools
- **ai-dev** - AI toolkit with Ollama, Gemini, OpenRouter support
- **systems** - Rust, Go, Docker, infrastructure tools
- **enterprise** - All languages and infrastructure tools

```yaml
# sindri.yaml
extensions:
  profile: fullstack # Use a pre-defined profile
```

See [Extension Catalog](docs/EXTENSIONS.md) for complete extension list.

## Available Extensions

**Languages:** nodejs, python, golang, rust, ruby, php, jvm, dotnet
**Tools:** github-cli, docker, playwright, monitoring
**Infrastructure:** infra-tools, cloud-tools
**AI:** ai-toolkit, openskills, claude-marketplace

Complete details: [docs/EXTENSIONS.md](docs/EXTENSIONS.md)

## Development Commands

```bash
# Validation and linting
pnpm validate          # Run all validations
pnpm lint              # Lint all code

# Testing
pnpm test              # Run all tests
pnpm test:extensions   # Test extensions

# Build
pnpm build             # Build Docker image
```

See [CLAUDE.md](CLAUDE.md) for complete development guide.

## Architecture Highlights

- **Modular Extension System** - YAML-driven with dependency resolution
- **Schema Validation** - All YAML validated against JSON schemas
- **Provider Adapters** - Clean abstraction for Docker, Fly, Kubernetes, DevPod
- **Volume Architecture** - Immutable `/docker/lib` system, mutable `/workspace` user data
- **BOM Tracking** - Comprehensive software bill of materials for security auditing

Read more: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Acknowledgments

- [Anthropic](https://www.anthropic.com/) for Claude Code and Claude AI
- [mise](https://mise.jdx.dev/) for declarative tool version management
- [Fly.io](https://fly.io/) for excellent container hosting platform
