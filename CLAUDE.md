# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Sindri is a declarative, provider-agnostic cloud development environment system. It uses YAML-defined extensions and optimized Docker images to deploy consistent development environments to Fly.io, Kubernetes, or local Docker.

**Core Design Principles:**

- **YAML-First Architecture**: Extensions are declarative YAML files, not bash scripts. All configuration is driven by YAML schemas.
- **Provider Agnostic**: Single `sindri.yaml` deploys to multiple providers (docker, fly, devpod, kubernetes via devpod).
- **Immutable/Mutable Split**: System files in `/docker/` are baked into the image (immutable), while `$HOME` (`/alt/home/developer`) is a persistent volume containing workspace and all user data.
- **Fast Startup**: Optimized base image with pre-installed runtimes (Node.js, Python) enables 10-15s cold starts.

## Commands

### Development Workflow

```bash
# Validate all code (YAML, shell, markdown)
pnpm validate

# Run linting
pnpm lint
pnpm lint:yaml     # Lint YAML with yamllint
pnpm lint:shell    # Lint shell with shellcheck
pnpm lint:md       # Lint markdown with markdownlint

# Format code
pnpm format        # Format all files
pnpm format:md     # Format markdown only

# Testing
pnpm test                    # Run all tests
pnpm test:unit              # Unit tests (YAML validation)
pnpm test:extensions        # Validate all extensions

# Build Docker image
pnpm build         # Build as sindri:local
pnpm build:latest  # Build as sindri:latest
```

### CLI Usage

```bash
# Configuration
./cli/sindri config init       # Create sindri.yaml
./cli/sindri config validate   # Validate configuration

# Deployment
./cli/sindri deploy                            # Deploy using provider in sindri.yaml
./cli/sindri deploy --provider docker  # Deploy to Docker Compose
./cli/sindri deploy --provider fly             # Deploy to Fly.io
./cli/sindri deploy --provider devpod          # Deploy as DevContainer

# Project Management
./cli/new-project <name> [template]    # Create new project from template
./cli/clone-project <url> [path]       # Clone and setup project

# Secrets Management
./cli/sindri secrets list              # List configured secrets
./cli/sindri secrets validate          # Validate secrets configuration

# Extension Management
./cli/extension-manager list                # List all extensions
./cli/extension-manager list-profiles       # List extension profiles
./cli/extension-manager list-categories     # List extension categories
./cli/extension-manager info nodejs         # Show extension details
./cli/extension-manager search <term>       # Search extensions
./cli/extension-manager install nodejs      # Install single extension
./cli/extension-manager install-profile fullstack  # Install profile
./cli/extension-manager validate nodejs     # Validate extension
./cli/extension-manager validate-all        # Validate all extensions
./cli/extension-manager status nodejs       # Check extension status
./cli/extension-manager resolve nodejs      # Show dependency resolution
./cli/extension-manager bom                 # Show bill of materials
./cli/extension-manager bom-regenerate      # Regenerate all BOMs
```

## Architecture

### Directory Structure

```text
sindri/
├── cli/                           # CLI entry points
│   ├── sindri                     # Main deployment CLI
│   ├── extension-manager          # Extension management CLI
│   └── extension-manager-modules/ # Modular components for extension system
│       ├── cli.sh                 # Argument parsing
│       ├── manifest.sh            # Manifest CRUD operations
│       ├── dependency.sh          # Dependency resolution
│       ├── executor.sh            # YAML-driven execution engine
│       ├── validator.sh           # Schema validation
│       ├── reporter.sh            # Status reporting
│       └── bom.sh                 # Bill of Materials tracking
│
├── docker/
│   ├── Dockerfile                 # Multi-stage optimized build
│   ├── lib/                       # Immutable system files (baked into image)
│   │   ├── extensions/            # 27+ YAML extension definitions
│   │   ├── schemas/               # JSON schemas for validation
│   │   │   ├── extension.schema.json
│   │   │   ├── manifest.schema.json
│   │   │   └── sindri.schema.json
│   │   ├── profiles.yaml          # Extension profile definitions
│   │   ├── registry.yaml          # Extension registry
│   │   ├── categories.yaml        # Category definitions
│   │   └── common.sh              # Shared utility functions
│   └── scripts/
│       └── entrypoint.sh          # Container initialization
│
├── deploy/
│   └── adapters/                  # Provider-specific deployment logic
│       ├── docker-adapter.sh      # Docker Compose
│       ├── fly-adapter.sh         # Fly.io
│       └── devpod-adapter.sh      # DevContainer
│
└── examples/                      # Example sindri.yaml configurations
```

### Extension System Architecture

**Extensions are YAML files, not bash scripts.** The system is built on declarative configuration:

1. **Extension Definition** (`extension.yaml`):
   - Metadata (name, version, description, category)
   - Requirements (domains, disk space)
   - Install method (mise, script, apt)
   - Configuration (environment variables)
   - Validation (commands to verify installation)
   - Dependencies (other extensions required)

2. **Extension Execution Flow**:
   - `executor.sh` reads YAML and executes declaratively
   - `dependency.sh` resolves dependency DAG
   - `validator.sh` validates against JSON schemas
   - `manifest.sh` tracks installed extensions in `$WORKSPACE/.system/manifest/`

3. **Extension Manager Modules**:
   - Fully modular design - each module has single responsibility
   - No logic duplication between modules
   - All modules source `common.sh` for shared utilities

### Volume Architecture

Critical concept: **Two-tier filesystem with home directory as volume**

**Immutable System (`/docker/lib`):**

- Baked into Docker image at build time
- Contains extension definitions, schemas, scripts
- Read-only, owned by root
- Updated only by rebuilding the image

**Mutable Home Directory (`/alt/home/developer`):**

- Persistent volume mount point = `$HOME`
- **Fully writable** by `developer` user (uid 1001)
- Contains workspace, XDG directories, and all user data
- Survives container restarts
- Structure:

  ```text
  /alt/home/developer/      # $HOME - volume mount point
  ├── workspace/            # $WORKSPACE - projects and scripts
  │   ├── projects/         # User projects
  │   ├── config/           # User configs
  │   ├── scripts/          # User scripts
  │   ├── bin/              # User binaries (in PATH)
  │   └── .system/          # Extension state
  │       ├── manifest/     # Active extensions
  │       ├── installed/    # Installation markers
  │       └── logs/         # Extension logs
  ├── .local/               # XDG local (mise installations)
  │   ├── share/mise/       # mise data
  │   ├── state/mise/       # mise state
  │   └── bin/              # Local binaries
  ├── .config/              # XDG config
  │   └── mise/             # mise configuration
  ├── .cache/               # XDG cache
  │   └── mise/             # mise cache
  ├── .bashrc               # Shell configuration
  ├── .profile              # Profile configuration
  └── .initialized          # Initialization marker
  ```

**Key Environment Variables:**

| Variable          | Value                           |
| ----------------- | ------------------------------- |
| `ALT_HOME`        | `/alt/home/developer`           |
| `HOME`            | `/alt/home/developer`           |
| `WORKSPACE`       | `/alt/home/developer/workspace` |
| `DOCKER_LIB`      | `/docker/lib`                   |
| `MISE_DATA_DIR`   | `$HOME/.local/share/mise`       |
| `MISE_CONFIG_DIR` | `$HOME/.config/mise`            |
| `MISE_CACHE_DIR`  | `$HOME/.cache/mise`             |
| `MISE_STATE_DIR`  | `$HOME/.local/state/mise`       |

### Multi-Provider Architecture

**Adapter Pattern**: Each provider has a dedicated adapter script in `deploy/adapters/`:

- Reads `sindri.yaml` (single source of truth)
- Translates to provider-specific format (docker-compose.yml, fly.toml, devcontainer.json)
- Handles provider-specific deployment commands
- Manages secrets via provider mechanisms

**Available Adapters:**

| Provider        | Adapter                  | Notes                                        |
| --------------- | ------------------------ | -------------------------------------------- |
| `docker`        | `docker-adapter.sh`      | Local Docker Compose deployment              |
| `fly`           | `fly-adapter.sh`         | Fly.io cloud deployment                      |
| `devpod`        | `devpod-adapter.sh`      | DevContainer (supports AWS, GCP, Azure, K8s) |

**Note:** Kubernetes deployment is supported via the DevPod provider with `type: kubernetes`. There is no native kubernetes-adapter; use DevPod for K8s deployments.

All adapters share the same base Docker image and extension system.

## Extension Development

### Creating New Extensions

1. Create directory: `docker/lib/extensions/myext/`
2. Create `extension.yaml` (declarative definition)
3. Add to `docker/lib/registry.yaml`
4. Validate against schema: `./cli/extension-manager validate myext`

**Extension YAML Structure:**

```yaml
metadata:
  name: myext
  version: 1.0.0
  description: Brief description
  category: language|dev-tools|database|cloud|monitoring|security
  dependencies: [] # List of extension names

requirements:
  domains: [example.com] # Network access needed
  diskSpace: 100 # MB required

install:
  method: mise|script|apt
  mise:
    configFile: mise.toml # For tool installation via mise
  script:
    path: install.sh # Custom installation script
  apt:
    packages: [pkg1, pkg2] # APT packages

configure:
  environment:
    - key: VAR_NAME
      value: value
      scope: bashrc|profile

validate:
  commands:
    - name: mycmd
      expectedPattern: "v\\d+\\.\\d+\\.\\d+" # Optional regex

upgrade:
  strategy: reinstall|in-place  # How to handle version upgrades
  script:
    path: upgrade.sh  # Optional upgrade script

remove:
  mise:
    removeConfig: true
    tools: [tool1]
  script:
    path: uninstall.sh

bom:
  components:  # Bill of Materials - auto-generated, do not edit manually
    - name: tool-name
      version: "1.0.0"
      type: runtime|library|tool
      source: mise|apt|script
```

### Extension Profiles

Profiles are defined in `docker/lib/profiles.yaml`. To add a new profile:

```yaml
profiles:
  myprofile:
    description: Description of profile
    extensions:
      - extension1
      - extension2
```

## Testing

### Test Structure

- **Unit tests**: `test/unit/` - YAML validation and schema tests
- **Extension tests**:
  - Local: `./cli/extension-manager validate-all` or `pnpm test:extensions`
  - CI: `.github/scripts/test-all-extensions.sh` - Full extension testing in CI/CD

### GitHub Actions

9 workflows in `.github/workflows/`:

- `ci.yml` - Main CI orchestrator (linting, building, testing)
- `validate-yaml.yml` - Comprehensive YAML validation with schema checks
- `test-provider.yml` - Provider-specific testing (docker, fly, devpod, k8s)
- `test-extensions.yml` - Extension testing across providers
- `test-sindri-config.yml` - User configuration testing
- `deploy-sindri.yml` - Reusable deployment workflow
- `teardown-sindri.yml` - Reusable teardown workflow
- `manual-deploy.yml` - Manual deployment trigger
- `release.yml` - Automated release with changelog generation (see [docs/RELEASE.md](docs/RELEASE.md))

## Code Style

### Shell Scripts

- Use `set -euo pipefail` at the top of all scripts
- Source `docker/lib/common.sh` for shared utilities
- Use `print_status`, `print_success`, `print_warning`, `print_error` for output
- Validate with `shellcheck -S warning`

### YAML Files

- Validate with `yamllint --strict`
- All extensions must validate against `extension.schema.json`
- Use 2-space indentation

### Markdown

- Lint with `markdownlint`
- Format with `prettier`

## Important Patterns

### Schema-Driven Development

All YAML files validate against JSON schemas in `docker/lib/schemas/`. When modifying:

1. Update schema first
2. Update YAML files
3. Run validation: `pnpm validate:yaml`

### Declarative Execution

The `executor.sh` module interprets extension YAML and executes installation/configuration. Never hardcode extension logic in bash - use YAML declarations.

### Dependency Resolution

Extensions declare dependencies in YAML. The `dependency.sh` module builds a DAG and installs in topological order. Dependencies are resolved recursively.

### Provider Abstraction

Never write provider-specific logic in core code. Use adapter pattern in `deploy/adapters/` for provider-specific concerns.
