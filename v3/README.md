# Sindri V3

A modern, high-performance CLI for declarative cloud development environments, rewritten in Rust.

## Overview

Sindri V3 is a complete rewrite of the Sindri CLI in Rust, delivering improved performance, enhanced security features, and native container image management. It provides a declarative approach to provisioning development environments across multiple cloud providers.

## Key Features

- **Multi-Provider Support** - Deploy to Docker, Fly.io, DevPod, Kubernetes, E2B, and VMs (AWS, Azure, GCP, OCI, Alibaba)
- **Golden VM Images** - Build pre-configured VM images with Packer across all major cloud providers
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

| Document                                               | Description                         |
| ------------------------------------------------------ | ----------------------------------- |
| [Quickstart Guide](docs/QUICKSTART.md)                 | Zero to deployed in 10 minutes      |
| [Getting Started](docs/GETTING_STARTED.md)             | Detailed setup instructions         |
| [CLI Reference](docs/CLI.md)                           | Complete command-line reference     |
| [Configuration](docs/CONFIGURATION.md)                 | Full sindri.yaml specification      |
| [Runtime Configuration](docs/RUNTIME_CONFIGURATION.md) | Runtime configuration and overrides |

### Core Features

| Document                                         | Description                           |
| ------------------------------------------------ | ------------------------------------- |
| [Secrets Management](docs/SECRETS_MANAGEMENT.md) | Multi-backend secrets configuration   |
| [Backup & Restore](docs/BACKUP_RESTORE.md)       | Workspace backup strategies           |
| [Projects](docs/PROJECTS.md)                     | Project scaffolding and git workflows |
| [Image Management](docs/IMAGE_MANAGEMENT.md)     | Container versioning and security     |
| [Deployment](docs/DEPLOYMENT.md)                 | Deployment modes and Docker images    |
| [Doctor](docs/DOCTOR.md)                         | System diagnostics and auto-fix       |
| [Schema Reference](docs/SCHEMA.md)               | YAML schema documentation             |

### Providers

| Document                                             | Description                            |
| ---------------------------------------------------- | -------------------------------------- |
| [Providers Overview](docs/providers/README.md)       | Overview of all deployment providers   |
| [Docker](docs/providers/DOCKER.md)                   | Docker Compose local development       |
| [Fly.io](docs/providers/FLY.md)                      | Fly.io cloud deployment                |
| [E2B](docs/providers/E2B.md)                         | E2B cloud sandboxes                    |
| [Kubernetes](docs/providers/KUBERNETES.md)           | Kubernetes cluster deployment          |
| [DevPod](docs/providers/DEVPOD.md)                   | DevPod multi-cloud support             |
| [VM Providers](docs/providers/VM.md)                 | Virtual machine deployment overview    |
| [VM Distribution](docs/providers/vm/DISTRIBUTION.md) | VM image distribution and CDN strategy |
| [VM Security](docs/providers/vm/SECURITY.md)         | VM security and hardening              |
| [AWS](docs/providers/vm/AWS.md)                      | Amazon EC2 deployment                  |
| [Azure](docs/providers/vm/AZURE.md)                  | Microsoft Azure VM deployment          |
| [GCP](docs/providers/vm/GCP.md)                      | Google Compute Engine deployment       |
| [OCI](docs/providers/vm/OCI.md)                      | Oracle Cloud Infrastructure deployment |
| [Alibaba Cloud](docs/providers/vm/ALIBABA.md)        | Alibaba Cloud ECS deployment           |

### Extension System

| Document                                                                           | Description                               |
| ---------------------------------------------------------------------------------- | ----------------------------------------- |
| [Extensions Overview](docs/EXTENSIONS.md)                                          | Extension system architecture and usage   |
| [Extension Guides](docs/extensions/guides/README.md)                               | Index of all extension development guides |
| [Authoring Extensions](docs/extensions/guides/AUTHORING.md)                        | Complete guide to creating extensions     |
| [Sourcing Modes](docs/extensions/guides/SOURCING_MODES.md)                         | Extension sourcing strategies             |
| [Support Files](docs/extensions/guides/SUPPORT_FILE_INTEGRATION.md)                | Support file integration patterns         |
| [Support File Versioning](docs/extensions/guides/SUPPORT_FILE_VERSION_HANDLING.md) | Version-aware support file management     |
| [Support Files CLI](docs/extensions/guides/SUPPORT_FILES_CLI_COMMAND.md)           | Support file CLI commands                 |
| [Conditional Templates](docs/extensions/guides/CONDITIONAL_TEMPLATES_MIGRATION.md) | Environment-based template selection      |

### Advanced Topics

| Document                                         | Description                       |
| ------------------------------------------------ | --------------------------------- |
| [Kubernetes](docs/K8S.md)                        | Advanced Kubernetes configuration |
| [Multi-Architecture](docs/MULTI_ARCH_SUPPORT.md) | Multi-platform image builds       |
| [Troubleshooting](docs/TROUBLESHOOTING.md)       | Common issues and solutions       |
| [Maintainer Guide](docs/MAINTAINER_GUIDE.md)     | Release process and maintenance   |

## Architecture

Sindri V3 uses a modular architecture with clear separation between providers, extensions, and core functionality.

- **[Architecture Overview](docs/ARCHITECTURE.md)** - High-level system design and component overview
- **[Architecture Decision Records](docs/architecture/adr/README.md)** - Detailed design rationale covering:
  - Rust workspace organization
  - Provider abstraction layer
  - Extension type system and dependency resolution
  - Secrets resolver architecture
  - Backup/restore system design
  - Local Kubernetes cluster management

## Providers

| Provider         | Description                                      | Requirements                    |
| ---------------- | ------------------------------------------------ | ------------------------------- |
| `docker-compose` | Local development                                | Docker                          |
| `fly`            | Cloud deployment                                 | flyctl + Fly.io account         |
| `devpod`         | Multi-cloud workspaces (AWS, GCP, Azure, K8s)    | DevPod CLI                      |
| `kubernetes`     | K8s clusters                                     | kubectl + cluster access        |
| `e2b`            | Ultra-fast cloud sandboxes                       | E2B account                     |
| `vm`             | Golden VM images (AWS, Azure, GCP, OCI, Alibaba) | Packer 1.9+ + cloud CLI + creds |

## Extension Profiles

Pre-configured extension bundles for common use cases:

| Profile         | Includes                          | Best For               |
| --------------- | --------------------------------- | ---------------------- |
| `minimal`       | Node.js, Python                   | Quick tasks, scripting |
| `fullstack`     | Node.js, Python, Docker, devtools | Web development        |
| `anthropic-dev` | Claude tools, AI agents, agentic  | AI/Anthropic projects  |
| `systems`       | Rust, Go, Docker                  | Systems programming    |
| `devops`        | Docker, Terraform, cloud tools    | Infrastructure         |
| `enterprise`    | All languages + infrastructure    | Large projects         |

## System Requirements

- **Docker** 20.10+ (required for all providers)
- **Git** (for project management)
- Additional requirements vary by provider (see [Quickstart](docs/QUICKSTART.md#prerequisites))

Run `sindri doctor` to check your system and optionally install missing tools.

## Building Docker Images

Sindri v3 provides two Dockerfiles for different use cases:

### Production Image (`Dockerfile`)

**Use Case**: Production deployments, CI/CD pipelines, smaller images

**Features**:

- Uses pre-compiled binary (from GitHub releases or CI artifacts)
- No bundled extensions (installed at runtime via `sindri extension install`)
- Smaller image size (~800MB)
- Faster builds (2-5 minutes)
- Extensions installed to `${HOME}/.sindri/extensions` (respects `ALT_HOME=/alt/home/developer` volume mount)

**Build Command**:

```bash
# Build production image (downloads binary from releases)
docker build -f v3/Dockerfile -t sindri:prod .

# Or using Makefile
make v3-docker-build-from-binary
```

### Development Image (`Dockerfile.dev`)

**Use Case**: Development, testing, air-gapped deployments

**Features**:

- Builds from Rust source (`cargo build --release`)
- Bundled extensions at `/opt/sindri/extensions`
- Larger image size (~1.2GB)
- Longer builds (~8 minutes)
- Extensions available immediately without installation

**Build Command**:

```bash
# Build development image (compiles from source)
docker build -f v3/Dockerfile.dev -t sindri:dev .

# Or using Makefile
make v3-docker-build-from-source
```

### Environment Variables

Both images use `SINDRI_EXT_HOME` to configure the extensions directory:

- **Production**: `SINDRI_EXT_HOME=${HOME}/.sindri/extensions` (respects volume-mounted home directory)
- **Development**: `SINDRI_EXT_HOME=/opt/sindri/extensions` (bundled in image)

The `${HOME}` variable expansion respects the `ALT_HOME=/alt/home/developer` volume mount used by deployment providers, ensuring extensions are installed to the correct location in containerized environments.

### Choosing the Right Dockerfile

| Criterion           | Production (`Dockerfile`)   | Development (`Dockerfile.dev`) |
| ------------------- | --------------------------- | ------------------------------ |
| Build time          | 2-5 minutes                 | ~8 minutes                     |
| Image size          | ~800MB                      | ~1.2GB                         |
| Extensions          | Runtime installation        | Pre-bundled                    |
| Use case            | Production, CI/CD           | Development, testing           |
| Binary source       | Pre-compiled                | Built from source              |
| Air-gapped support  | No                          | Yes                            |
| Custom path support | Yes (via `SINDRI_EXT_HOME`) | Yes (via `SINDRI_EXT_HOME`)    |

For more details on deployment modes and Dockerfile selection, see [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md).

## Contributing

See the project-level [Contributing Guide](../docs/CONTRIBUTING.md) for development workflow and standards.

## Support

- **Issues**: [GitHub Issues](https://github.com/pacphi/sindri/issues)
- **Discussions**: [GitHub Discussions](https://github.com/pacphi/sindri/discussions)
- **FAQ**: [sindri-faq.fly.dev](https://sindri-faq.fly.dev)
