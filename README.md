# Sindri

[![License](https://img.shields.io/github/license/pacphi/sindri)](LICENSE)
[![CI](https://github.com/pacphi/sindri/actions/workflows/ci.yml/badge.svg)](https://github.com/pacphi/sindri/actions/workflows/ci.yml)

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
- **[Fly.io Deployment](docs/providers/FLY.md)** - Fly.io-specific guide
- **[DevPod Integration](docs/providers/DEVPOD.md)** - DevContainer setup
- **[Docker Deployment](docs/providers/DOCKER.md)** - Local Docker setup
- **[Kubernetes Deployment](docs/providers/KUBERNETES.md)** - Enterprise K8s guide
- **[Secrets Management](docs/SECRETS_MANAGEMENT.md)** - Managing secrets across providers

### Development & Operations

- **[Project Management](docs/PROJECT_MANAGEMENT.md)** - Using new-project and clone-project
- **[Contributing Guide](docs/CONTRIBUTING.md)** - Development workflow and standards
- **[Testing Guide](docs/TESTING.md)** - Running tests and CI/CD
- **[Release Process](docs/RELEASE.md)** - Creating releases and changelog automation
- **[Troubleshooting](docs/TROUBLESHOOTING.md)** - Common issues and solutions
- **[Security](docs/SECURITY.md)** - Security best practices

## Extension Profiles

Sindri includes 27+ modular extensions organized into profiles:

| Profile        | Description                                  |
| -------------- | -------------------------------------------- |
| **minimal**    | Node.js, Python basics                       |
| **fullstack**  | Node.js, Python, Docker, Postgres, dev tools |
| **ai-dev**     | AI toolkit with Ollama, Gemini, OpenRouter   |
| **systems**    | Rust, Go, Docker, infrastructure tools       |
| **enterprise** | All languages and infrastructure tools       |

```yaml
# sindri.yaml
extensions:
  profile: fullstack
```

**Available categories:** Languages (nodejs, python, golang, rust, ruby, php, jvm, dotnet), Tools (github-cli, docker, playwright, monitoring), Infrastructure (infra-tools, cloud-tools), AI (ai-toolkit, openskills, claude-marketplace)

See [Extension Catalog](docs/EXTENSIONS.md) for complete list and usage.

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

## Architecture Highlights

- **Modular Extension System** - YAML-driven with dependency resolution
- **Schema Validation** - All YAML validated against JSON schemas
- **Provider Adapters** - Clean abstraction for Docker, Fly, Kubernetes, DevPod
- **Volume Architecture** - Immutable `/docker/lib` system, mutable `$HOME` volume containing workspace
- **BOM Tracking** - Comprehensive software bill of materials for security auditing

## License

MIT License - see [LICENSE](LICENSE) file for details.
