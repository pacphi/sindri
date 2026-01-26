# Sindri V3

A modern, high-performance CLI for declarative cloud development environments, rewritten in Rust.

## Overview

Sindri V3 is a complete rewrite of the Sindri CLI in Rust, delivering improved performance, enhanced security features, and native container image management. It provides a declarative approach to provisioning development environments across multiple cloud providers.

## Key Features

- **Multi-Provider Support** - Deploy to Docker, Fly.io, DevPod, Kubernetes, and E2B
- **Extension System** - 40+ modular extensions for languages, tools, and infrastructure
- **Secrets Management** - Multi-backend support (environment, file, HashiCorp Vault, S3)
- **Backup & Restore** - Full workspace backup with encryption and multiple restore modes
- **Project Management** - Scaffolding templates and enhanced git workflows
- **Image Security** - Signature verification and SBOM generation with cosign
- **Local Kubernetes** - Built-in kind and k3d cluster management
- **Self-Update** - Automatic CLI updates with rollback support
- **Schema Validation** - All YAML validated against JSON schemas
- **System Diagnostics** - Comprehensive doctor command with auto-fix

## Quick Start

```bash
# Install Sindri V3
wget https://github.com/pacphi/sindri/releases/latest/download/sindri-linux-x86_64.tar.gz
tar -xzf sindri-linux-x86_64.tar.gz
sudo mv sindri /usr/local/bin/

# Initialize configuration
sindri config init --provider docker --profile fullstack

# Deploy
sindri deploy

# Connect to your environment
sindri connect
```

For detailed installation instructions and first deployment walkthrough, see the [Quickstart Guide](docs/QUICKSTART.md).

## Documentation

### Getting Started

| Document                                   | Description                     |
| ------------------------------------------ | ------------------------------- |
| [Quickstart Guide](docs/QUICKSTART.md)     | Zero to deployed in 10 minutes  |
| [Getting Started](docs/GETTING_STARTED.md) | Detailed setup instructions     |
| [CLI Reference](docs/CLI.md)               | Complete command-line reference |
| [Configuration](docs/CONFIGURATION.md)     | Full sindri.yaml specification  |

### Features

| Document                                         | Description                           |
| ------------------------------------------------ | ------------------------------------- |
| [Secrets Management](docs/SECRETS_MANAGEMENT.md) | Multi-backend secrets configuration   |
| [Backup & Restore](docs/BACKUP_RESTORE.md)       | Workspace backup strategies           |
| [Projects](docs/PROJECTS.md)                     | Project scaffolding and git workflows |
| [Image Management](docs/IMAGE_MANAGEMENT.md)     | Container versioning and security     |
| [Doctor](docs/DOCTOR.md)                         | System diagnostics and auto-fix       |
| [Schema Reference](docs/SCHEMA.md)               | YAML schema documentation             |

### Extension System

| Document                                                                   | Description                           |
| -------------------------------------------------------------------------- | ------------------------------------- |
| [Extension Migration Status](docs/EXTENSION_MIGRATION_STATUS.md)           | V2 to V3 extension migration tracking |
| [Conditional Templates](docs/EXTENSION_CONDITIONAL_TEMPLATES_MIGRATION.md) | Environment-based template selection  |

## Architecture

Sindri V3 uses a modular architecture with clear separation between providers, extensions, and core functionality. Key architectural decisions are documented in Architecture Decision Records (ADRs).

See [Architecture Decision Records](docs/architecture/adr/README.md) for detailed design rationale covering:

- Rust workspace organization
- Provider abstraction layer
- Extension type system and dependency resolution
- Secrets resolver architecture
- Backup/restore system design
- Local Kubernetes cluster management

## Providers

| Provider         | Description                        | Requirements             |
| ---------------- | ---------------------------------- | ------------------------ |
| `docker-compose` | Local development                  | Docker                   |
| `fly`            | Cloud deployment                   | flyctl + Fly.io account  |
| `devpod`         | Multi-cloud (AWS, GCP, Azure, K8s) | DevPod CLI               |
| `kubernetes`     | K8s clusters                       | kubectl + cluster access |
| `e2b`            | Ultra-fast cloud sandboxes         | E2B account              |

## Extension Profiles

Pre-configured extension bundles for common use cases:

| Profile      | Includes                          | Best For               |
| ------------ | --------------------------------- | ---------------------- |
| `minimal`    | Node.js, Python                   | Quick tasks, scripting |
| `fullstack`  | Node.js, Python, Docker, devtools | Web development        |
| `ai-dev`     | Python, AI toolkit, Jupyter       | ML/AI projects         |
| `systems`    | Rust, Go, Docker                  | Systems programming    |
| `devops`     | Docker, Terraform, cloud tools    | Infrastructure         |
| `enterprise` | All languages + infrastructure    | Large projects         |

## System Requirements

- **Docker** 20.10+ (required for all providers)
- **Git** (for project management)
- Additional requirements vary by provider (see [Quickstart](docs/QUICKSTART.md#prerequisites))

Run `sindri doctor` to check your system and optionally install missing tools.

## Contributing

See the project-level [Contributing Guide](../docs/CONTRIBUTING.md) for development workflow and standards.

## Support

- **Issues**: [GitHub Issues](https://github.com/pacphi/sindri/issues)
- **Discussions**: [GitHub Discussions](https://github.com/pacphi/sindri/discussions)
- **FAQ**: [sindri-faq.fly.dev](https://sindri-faq.fly.dev)
