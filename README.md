# Sindri

[![License](https://img.shields.io/github/license/pacphi/sindri)](LICENSE)
[![CI V2](https://github.com/pacphi/sindri/actions/workflows/ci-v2.yml/badge.svg)](https://github.com/pacphi/sindri/actions/workflows/ci-v2.yml)
[![CI V3](https://github.com/pacphi/sindri/actions/workflows/ci-v3.yml/badge.svg)](https://github.com/pacphi/sindri/actions/workflows/ci-v3.yml)
[![FAQ](https://img.shields.io/badge/FAQ-on%20fly.dev-blue)](https://sindri-faq.fly.dev)
[![GHCR](https://img.shields.io/badge/GHCR-container%20registry-blue)](https://github.com/pacphi/sindri/pkgs/container/sindri)
[![Security](https://img.shields.io/badge/Security-Cosign%20%2B%20SBOM-green)](v3/docs/IMAGE_MANAGEMENT.md#security)

A declarative, provider-agnostic cloud development environment system. Deploy consistent development environments to Fly.io, local Docker, or via DevPod to Kubernetes, AWS, GCP, Azure, and other cloud providers using YAML-defined extensions.

**🔐 Secure Supply Chain:** All release images are signed with Cosign, include SBOM (Software Bill of Materials), and have SLSA Level 3 provenance attestations.

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

## Versions

- **[v2](v2)** - Bash/Docker implementation (stable)
- **[v3](v3)** - Rust CLI implementation (active development)
- **[Console](v3/console)** - Web-based management console (Go agent + TypeScript API + React UI)

## Quick Start (v3)

### Install

```bash
# Pull the Docker image
docker pull ghcr.io/pacphi/sindri:v3

# Or download the CLI binary
wget https://github.com/pacphi/sindri/releases/latest/download/sindri-linux-x86_64.tar.gz
tar -xzf sindri-linux-x86_64.tar.gz
sudo mv sindri /usr/local/bin/
```

### Deploy

```bash
# Initialize configuration
sindri config init --provider kubernetes

# Deploy with automatic image version resolution
sindri deploy

# Verify image security
sindri image verify ghcr.io/pacphi/sindri:v3.0.0
```

### Image Management

```bash
# List available versions
sindri image list

# Inspect image details
sindri image inspect ghcr.io/pacphi/sindri:v3.0.0 --sbom

# Check current image
sindri image current
```

## Features

### 🎯 Provider-Agnostic

Deploy to multiple cloud providers using the same configuration:

- Docker & Docker Compose
- Fly.io
- Kubernetes (kind, k3d, EKS, GKE, AKS)
- DevPod (AWS, GCP, Azure, DigitalOcean)
- E2B
- [RunPod](v3/docs/providers/RUNPOD.md) - GPU-intensive ML workloads (40+ GPU types)
- [Northflank](v3/docs/providers/NORTHFLANK.md) - Auto-scaling production apps with GPU support

### 🔐 Security-First

- **Signed Images**: All releases signed with Cosign (keyless)
- **SBOM**: Software Bill of Materials in SPDX format
- **SLSA Provenance**: Level 3 build attestations
- **Vulnerability Scanning**: Trivy scans on every CI build

### 📦 Container Image Lifecycle

- **Build Once, Promote Often**: CI builds once, releases retag (no rebuild)
- **Version Resolution**: Semantic versioning constraints
- **Signature Verification**: Automatic verification before deployment
- **Registry-First**: GHCR as single source of truth

### 🔧 Extension System

Install development tools declaratively using profiles or individual extensions.

### 🤖 Claude Code Skills for Extension Development

Claude Code skills are available to guide extension development for both V2 and V3:

| Skill                 | Description                               |
| --------------------- | ----------------------------------------- |
| `/extension-guide`    | Router skill - helps choose V2 or V3      |
| `/extension-guide-v2` | V2 extension guide (Bash/Docker platform) |
| `/extension-guide-v3` | V3 extension guide (Rust CLI platform)    |

When using Claude Code, ask about creating extensions and it will automatically provide guidance:

```text
Example prompts:
- "Help me create a new extension for Lua development"
- "Create a V3 extension for an MCP server"
- "What's the difference between V2 and V3 extensions?"
- "Add capabilities to my extension for project initialization"
```

See [Version Comparison](docs/migration/COMPARISON_GUIDE.md) for detailed differences.

## Console Development

The Sindri Console is a full-stack management UI for deployed instances. It consists of:

- **Agent** (`v3/console/agent/`) — Go binary, runs on managed instances
- **API** (`v3/console/apps/api/`) — Hono/TypeScript backend with PostgreSQL + Redis
- **Web** (`v3/console/apps/web/`) — React + Vite frontend

### Quick Start

```bash
# Install TypeScript dependencies
make console-install

# Start dev servers (API + Web with hot reload)
make console-dev

# Build the agent binary for the current platform
make console-agent-build
```

### Makefile Target Reference

All targets are available from the repo root. Run `make help` for the full list.

#### Console Agent (Go)

```bash
make console-agent-build       # Build for current platform → dist/sindri-agent
make console-agent-build-all   # Cross-compile: linux/amd64, linux/arm64, darwin/amd64, darwin/arm64
make console-agent-test        # Unit tests (with -race)
make console-agent-vet         # go vet
make console-agent-fmt         # Format with gofmt
make console-agent-fmt-check   # Check formatting (non-destructive)
make console-agent-lint        # golangci-lint
make console-agent-audit       # govulncheck vulnerability scan
make console-agent-install     # Install binary to GOPATH/bin
make console-agent-clean       # Remove dist/
make console-agent-ci          # Agent CI: vet + test + build-all
```

#### Console TypeScript (API + Web)

```bash
make console-install             # Install all npm dependencies (pnpm)
make console-dev                 # Start development servers
make console-build               # Production build
make console-test                # Vitest unit tests
make console-test-coverage       # Tests with coverage report
make console-lint                # ESLint
make console-fmt                 # Prettier (write)
make console-fmt-check           # Prettier (check only)
make console-typecheck           # TypeScript type check
make console-audit               # npm vulnerability audit
make console-audit-fix           # Apply safe audit fixes
make console-db-migrate          # Apply Prisma migrations
make console-db-generate         # Regenerate Prisma client
make console-clean               # Remove build artifacts + node_modules
make console-ci                  # Full console CI pipeline
```

#### Aggregate targets

```bash
make ci      # Full CI: v2-ci + v3-ci + console-ci
make clean   # Remove all build artifacts (v2, v3, console, agent)
```

### Prerequisites

| Tool            | Required by              | Minimum version |
| --------------- | ------------------------ | --------------- |
| `go`            | console-agent-\* targets | 1.22            |
| `pnpm`          | console-\* TS targets    | 9               |
| `node`          | console-\* TS targets    | 20              |
| `golangci-lint` | console-agent-lint       | latest          |
| `govulncheck`   | console-agent-audit      | latest          |

Optional tools (`golangci-lint`, `govulncheck`) print a warning and are skipped if absent.

### Verifying Makefile Targets

```bash
# Fast syntax check — no compilation
./scripts/test-makefile-targets.sh --dry-run

# Agent targets only
./scripts/test-makefile-targets.sh --agent-only

# All targets (requires Go + pnpm + node_modules)
./scripts/test-makefile-targets.sh
```

CI automatically validates all console Makefile targets on every PR via
`.github/workflows/console-makefile-ci.yml`.

See [CONTRIBUTING.md](docs/CONTRIBUTING.md#console-development) for the full Console development guide.

---

## License

MIT License - see [LICENSE](LICENSE) file for details.

![Sindri at his forge](sindri.png)
