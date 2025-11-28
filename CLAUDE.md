# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Sindri is a declarative, provider-agnostic cloud development environment system. It uses YAML-defined extensions and optimized Docker images to deploy consistent development environments to Fly.io, Kubernetes, or local Docker.

**Core Design Principles:**

- **YAML-First Architecture**: Extensions are declarative YAML files, not bash scripts. All configuration is driven by YAML schemas.
- **Provider Agnostic**: Single `sindri.yaml` deploys to multiple providers (docker, fly, devpod).
- **Immutable/Mutable Split**: System files in `/docker/` are baked into the image (immutable), while `/workspace/` is a persistent volume (fully writable by developer user).
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
pnpm test:unit              # Unit tests
pnpm test:integration       # Integration tests
pnpm test:extensions        # Test all extensions

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
./cli/sindri deploy                    # Deploy using provider in sindri.yaml
./cli/sindri deploy --provider docker  # Deploy to Docker Compose
./cli/sindri deploy --provider fly     # Deploy to Fly.io
./cli/sindri deploy --provider devpod  # Deploy as DevContainer

# Extension Management
./cli/extension-manager list                # List all extensions
./cli/extension-manager list-profiles       # List extension profiles
./cli/extension-manager install nodejs      # Install single extension
./cli/extension-manager install-profile fullstack  # Install profile
./cli/extension-manager validate nodejs     # Validate extension
./cli/extension-manager validate-all        # Validate all extensions
./cli/extension-manager status nodejs       # Check extension status
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
│       └── reporter.sh            # Status reporting
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
│       ├── entrypoint.sh          # Container initialization
│       └── init-volume.sh         # Kubernetes volume setup
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
   - `manifest.sh` tracks installed extensions in `/workspace/.system/manifest/`

3. **Extension Manager Modules**:
   - Fully modular design - each module has single responsibility
   - No logic duplication between modules
   - All modules source `common.sh` for shared utilities

### Volume Architecture

Critical concept: **Two-tier filesystem**

**Immutable System (`/docker/lib`):**

- Baked into Docker image at build time
- Contains extension definitions, schemas, scripts
- Read-only, owned by root
- Updated only by rebuilding the image

**Mutable Workspace (`/workspace/`):**

- Persistent volume mount
- **Fully writable** by `developer` user (uid 1001)
- Contains user projects, configs, installed tools
- Survives container restarts
- Structure:

  ```text
  /workspace/
  ├── projects/         # User projects
  ├── config/          # User configs
  ├── bin/             # User binaries (in PATH)
  ├── .local/          # mise installations
  ├── .config/         # Tool configurations
  └── .system/         # Extension state
      ├── manifest/    # Active extensions
      └── logs/        # Extension logs
  ```

### Multi-Provider Architecture

**Adapter Pattern**: Each provider has a dedicated adapter script in `deploy/adapters/`:

- Reads `sindri.yaml` (single source of truth)
- Translates to provider-specific format (docker-compose.yml, fly.toml, devcontainer.json)
- Handles provider-specific deployment commands
- Manages secrets via provider mechanisms

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

remove:
  mise:
    removeConfig: true
    tools: [tool1]
  script:
    path: uninstall.sh
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

- **Unit tests**: `test/unit/` - Test individual functions
- **Integration tests**: `test/integration/` - Test full workflows
- **Extension tests**: `.github/scripts/test-all-extensions.sh` - Validate all extensions

### GitHub Actions

10 workflows in `.github/workflows/`:

- `ci.yml` - Main CI orchestrator (linting, building, testing)
- `validate-yaml.yml` - Comprehensive YAML validation with schema checks
- `test-provider.yml` - Provider-specific testing (docker, fly, devpod, k8s)
- `test-extensions.yml` - Extension testing across providers
- `test-sindri-config.yml` - User configuration testing
- `deploy-sindri.yml` - Reusable deployment workflow
- `teardown-sindri.yml` - Reusable teardown workflow
- `manual-deploy.yml` - Manual deployment trigger
- `self-service-deploy-fly.yml` - Self-service Fly.io deployment
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
