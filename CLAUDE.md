# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Sindri is a declarative, provider-agnostic cloud development environment system. It uses YAML-defined extensions and optimized Docker images to deploy consistent development environments to Fly.io, local Docker, or via DevPod to Kubernetes, AWS, GCP, Azure, and other cloud providers.

**Core Design Principles:**

- **YAML-First Architecture**: Extensions are declarative YAML files, not bash scripts. All configuration is driven by YAML schemas.
- **Provider Agnostic**: Single `sindri.yaml` deploys to multiple providers (docker, fly, devpod). DevPod supports multiple backends including Kubernetes, AWS, GCP, Azure, DigitalOcean, and SSH hosts.
- **Immutable/Mutable Split**: System files in `/docker/` are baked into the image (immutable), while `$HOME` (`/alt/home/developer`) is a persistent volume containing workspace and all user data.
- **Fast Startup**: Optimized base image with pre-installed tools (mise, Claude Code) enables fast startup. Development tools are installed via extensions on persistent volume.

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
├── Dockerfile                     # Multi-stage optimized build
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
│   ├── config/                    # Configuration files copied at build time
│   │   ├── sshd_config            # SSH daemon configuration (port 2222)
│   │   └── developer-sudoers      # Sudoers configuration for developer user
│   ├── lib/                       # Immutable system files (baked into image)
│   │   ├── extensions/            # 32 YAML extension definitions
│   │   ├── schemas/               # JSON schemas for validation
│   │   │   ├── extension.schema.json
│   │   │   ├── manifest.schema.json
│   │   │   ├── sindri.schema.json
│   │   │   ├── vm-sizes.schema.json
│   │   │   ├── profiles.schema.json
│   │   │   ├── registry.schema.json
│   │   │   ├── categories.schema.json
│   │   │   └── project-templates.schema.json
│   │   ├── profiles.yaml          # Extension profile definitions
│   │   ├── registry.yaml          # Extension registry
│   │   ├── categories.yaml        # Category definitions
│   │   ├── vm-sizes.yaml          # VM size mappings by provider
│   │   └── common.sh              # Shared utility functions
│   └── scripts/
│       ├── entrypoint.sh          # Container initialization (runs as root)
│       ├── setup-ssh-environment.sh # SSH environment for CI/CD
│       ├── install-mise.sh        # mise tool manager installation
│       └── install-claude.sh      # Claude Code CLI installation
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

| Variable          | Value                              |
| ----------------- | ---------------------------------- |
| `ALT_HOME`        | `/alt/home/developer`              |
| `HOME`            | `/alt/home/developer`              |
| `WORKSPACE`       | `/alt/home/developer/workspace`    |
| `DOCKER_LIB`      | `/docker/lib`                      |
| `SSH_PORT`        | `2222`                             |
| `CI_MODE`         | `true` in CI (disables SSH daemon) |
| `MISE_DATA_DIR`   | `$HOME/.local/share/mise`          |
| `MISE_CONFIG_DIR` | `$HOME/.config/mise`               |
| `MISE_CACHE_DIR`  | `$HOME/.cache/mise`                |
| `MISE_STATE_DIR`  | `$HOME/.local/state/mise`          |

### Pre-installed Tools

The base image includes these tools system-wide (in `/usr/local/bin`):

| Tool     | Purpose                                     | Installation Script |
| -------- | ------------------------------------------- | ------------------- |
| `mise`   | Unified tool version manager                | `install-mise.sh`   |
| `claude` | Claude Code CLI for AI-assisted development | `install-claude.sh` |
| `gh`     | GitHub CLI                                  | APT package         |
| `yq`     | YAML processor                              | Binary download     |

**Development tools (Node.js, Python, etc.)** are installed via extensions:

```bash
extension-manager install nodejs    # Installs Node.js via mise
extension-manager install python    # Installs Python via mise
```

Tools installed via extensions are stored on the persistent volume (`$HOME/.local/share/mise/`).

**Claude Code Installation:**

- Uses Anthropic's official curl installer with 5-minute timeout
- Binary installed to `/usr/local/bin/claude` for system-wide access
- User config directory (`~/.claude/`) created from `/etc/skel/.claude/` on first login
- Available immediately after container startup

### Container Startup Architecture

The container runs as **root** to properly initialize volumes and start the SSH daemon:

1. **Entrypoint** (`/docker/scripts/entrypoint.sh`) - runs as root:
   - Initializes home directory on volume (first boot)
   - Sets correct ownership for developer user
   - Configures SSH authorized keys from `AUTHORIZED_KEYS` env
   - Configures Git user from `GIT_USER_NAME`, `GIT_USER_EMAIL`
   - Starts SSH daemon on port 2222 (unless `CI_MODE=true`)

2. **SSH Sessions** - run as developer user:
   - SSH daemon drops privileges to developer user for sessions
   - Full environment available via `BASH_ENV` configuration

3. **CI Mode** (`CI_MODE=true`):
   - SSH daemon is NOT started
   - Container stays alive with `sleep infinity`
   - Use `flyctl ssh console` for access (Fly.io hallpass)
   - fly.toml has `services = []` to avoid port conflicts

### SSH Configuration

SSH is configured for secure, non-standard port access:

- **Internal Port**: 2222 (avoids conflict with Fly.io hallpass on port 22)
- **External Port**: Configurable (default: 10022)
- **Authentication**: Key-only (password disabled)
- **Environment**: Full shell environment available in non-interactive SSH commands

The `setup-ssh-environment.sh` script configures `BASH_ENV` so that SSH commands
(like those from CI/CD) get the full environment including mise-managed tools.

### Multi-Provider Architecture

**Adapter Pattern**: Each provider has a dedicated adapter script in `deploy/adapters/` that handles
the full lifecycle using a command-based interface:

```bash
<adapter>.sh <command> [options] [config]

Commands:
  deploy   - Create/update deployment
  connect  - Connect to running environment
  destroy  - Tear down deployment
  plan     - Show deployment plan
  status   - Show deployment status
```

The sindri CLI delegates all operations to adapters:

```bash
sindri deploy  → <adapter>.sh deploy
sindri connect → <adapter>.sh connect
sindri status  → <adapter>.sh status
sindri plan    → <adapter>.sh plan
sindri destroy → <adapter>.sh destroy
```

**Available Adapters:**

| Provider | Adapter             | Generated Config     | Deploy Command         |
| -------- | ------------------- | -------------------- | ---------------------- |
| `docker` | `docker-adapter.sh` | `docker-compose.yml` | `docker compose up -d` |
| `fly`    | `fly-adapter.sh`    | `fly.toml`           | `flyctl deploy`        |
| `devpod` | `devpod-adapter.sh` | `devcontainer.json`  | `devpod up`            |

**Note:** Kubernetes deployment is supported via the DevPod provider with `type: kubernetes`. There is no native kubernetes-adapter; use DevPod for K8s deployments.

**Fly.io Adapter CI Mode:**

```bash
# Generate CI-compatible fly.toml (empty services, no health checks)
./deploy/adapters/fly-adapter.sh deploy --ci-mode --config-only sindri.yaml
```

When `--ci-mode` is enabled:

- `services = []` is generated (no SSH service, avoids hallpass conflicts)
- Health checks are disabled
- `CI_MODE=true` is added to environment
- Container uses `sleep infinity` instead of SSH daemon

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
  category: base|language|dev-tools|infrastructure|ai|utilities|desktop|monitoring
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
  strategy: reinstall|in-place # How to handle version upgrades
  script:
    path: upgrade.sh # Optional upgrade script

remove:
  mise:
    removeConfig: true
    tools: [tool1]
  script:
    path: uninstall.sh

bom:
  components: # Bill of Materials - auto-generated, do not edit manually
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
  - CI: Integrated into `test-provider.yml` - Full extension testing in CI/CD with 9 phases (installation, discovery, validation, functionality, idempotency, filesystem, environment, uninstall, results)

### GitHub Actions

8 workflows in `.github/workflows/`:

- `ci.yml` - Main CI orchestrator with unified provider testing
- `validate-yaml.yml` - Comprehensive YAML validation with schema checks
- `test-provider.yml` - Full test suite per provider (CLI + extensions + integration)
- `test-sindri-config.yml` - User configuration testing
- `deploy-sindri.yml` - Reusable deployment workflow
- `teardown-sindri.yml` - Reusable teardown workflow
- `manual-deploy.yml` - Manual deployment trigger
- `release.yml` - Automated release with changelog generation (see [docs/RELEASE.md](docs/RELEASE.md))

**Unified Provider Testing**: Each selected provider runs CLI tests, extension tests, and
integration tests. This ensures consistent coverage across Docker, Fly.io, and DevPod providers.

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
