# CLAUDE.md

This file provides guidance to Claude Code when working with the Sindri codebase.

## Project Overview

Sindri is a declarative, provider-agnostic cloud development environment system. It uses YAML-defined extensions and optimized Docker images to deploy consistent development environments across multiple cloud providers.

**Core Design Principles:**

- **YAML-First Architecture**: Extensions are declarative YAML files, not bash scripts
- **Provider Agnostic**: Single `sindri.yaml` deploys to multiple providers (docker, fly, devpod)
- **Immutable/Mutable Split**: System files baked into image; user data on persistent volume
- **Fast Startup**: Pre-installed tools (mise, Claude Code) with on-demand extension installation

📚 **Comprehensive Documentation:** See [docs/](docs/) for detailed guides

## Quick Command Reference

### Development Commands

```bash
# Validate, lint, and format
pnpm validate                # Validate all code (YAML, shell, markdown)
pnpm lint                    # Run linting
pnpm format                  # Format all files
pnpm test                    # Run all tests

# Build and deploy
pnpm build                   # Build Docker image
./v2/cli/sindri deploy          # Deploy to configured provider
./v2/cli/sindri status          # Check deployment status
./v2/cli/sindri connect         # Connect to deployed instance
```

📖 **Full CLI Reference:** [v2/docs/CLI.md](v2/docs/CLI.md)

### Key CLI Tools

| Tool                      | Purpose                    | Full Documentation                                       |
| ------------------------- | -------------------------- | -------------------------------------------------------- |
| `./v2/cli/sindri`            | Deployment & configuration | [v2/docs/CLI.md](v2/docs/CLI.md)                               |
| `./v2/cli/extension-manager` | Extension management       | [docs/EXTENSIONS.md](docs/EXTENSIONS.md)                 |
| `./v2/cli/new-project`       | Project templates          | [docs/PROJECT_MANAGEMENT.md](docs/PROJECT_MANAGEMENT.md) |
| `./v2/cli/clone-project`     | Repository cloning         | [docs/PROJECT_MANAGEMENT.md](docs/PROJECT_MANAGEMENT.md) |

## Architecture Overview

### Key Concepts

**Directory Layout:**

- `v2/cli/` - Command-line tools (sindri, extension-manager)
- `v2/docker/lib/` - Immutable system files (extensions, schemas, profiles)
- `v2/docker/scripts/` - Container initialization scripts
- `v2/deploy/adapters/` - Provider-specific deployment logic
- `examples/` - Example configurations

**Extension System:**

- Extensions are declarative YAML files, not bash scripts
- `executor.sh` interprets YAML and executes declaratively
- `dependency.sh` resolves dependency DAG
- `validator.sh` validates against JSON schemas

**Volume Architecture:**

- **Immutable:** `/docker/lib` (baked into image, read-only)
- **Mutable:** `/alt/home/developer` (persistent volume, fully writable)
- `$HOME` = `/alt/home/developer` (contains workspace and all user data)

**Provider Adapters:**

| Provider | Adapter             | Config File          |
| -------- | ------------------- | -------------------- |
| docker   | `docker-adapter.sh` | `docker-compose.yml` |
| fly      | `fly-adapter.sh`    | `fly.toml`           |
| devpod   | `devpod-adapter.sh` | `devcontainer.json`  |

🏗️ **Full Architecture Details:** [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)

## Extension Development

**Quick Steps:**

1. Create `v2/docker/lib/extensions/myext/extension.yaml`
2. Define metadata, requirements, install method, validation
3. Add to `v2/docker/lib/registry.yaml`
4. Validate: `./v2/cli/extension-manager validate myext`

**Key Principles:**

- Extensions are YAML files, not bash scripts
- Use declarative configuration (mise, apt, script methods)
- All extensions must validate against JSON schema
- Dependencies are automatically resolved

**Example Extensions:**

- `spec-kit` - Project-init capability, no auth, git auto-commit hook
- `agentic-qe` - Project-init capability, requires anthropic auth
- `claude-flow-v2` - Full capabilities (project-init, auth, hooks, mcp)

🔧 **Complete Extension Guide:** [docs/EXTENSION_AUTHORING.md](docs/EXTENSION_AUTHORING.md)

## Testing

**Test Suites:**

- `smoke` - Quick health checks (~30 seconds)
- `integration` - Extension validation (~5-10 minutes)
- `full` - Complete test suite (~10-15 minutes)

**Quick Commands:**

```bash
pnpm test                    # Run all tests
pnpm test:unit               # YAML validation
pnpm test:extensions         # Extension validation
./v2/cli/sindri test --suite smoke  # Test deployed instance
```

🧪 **Complete Testing Guide:** [v2/docs/TESTING.md](v2/docs/TESTING.md)

## Code Standards

- **Shell Scripts:** Use `set -euo pipefail`; source `common.sh`; validate with `shellcheck`
- **YAML:** 2-space indentation; validate with `yamllint --strict` and JSON schemas
- **Markdown:** Lint with `markdownlint`; format with `prettier`

Run `pnpm validate` before committing.

## Important Patterns

### Schema-Driven Development

All YAML files validate against JSON schemas in `v2/docker/lib/schemas/`. When modifying:

1. Update schema first
2. Update YAML files
3. Run validation: `pnpm validate:yaml`

### Declarative Execution

The `executor.sh` module interprets extension YAML and executes installation/configuration. Never hardcode extension logic in bash - use YAML declarations.

### Dependency Resolution

Extensions declare dependencies in YAML. The `dependency.sh` module builds a DAG and installs in topological order. Dependencies are resolved recursively.

### Provider Abstraction

Never write provider-specific logic in core code. Use adapter pattern in `v2/deploy/adapters/` for provider-specific concerns.

---

## Documentation Index

| Topic                | Document                                                     |
| -------------------- | ------------------------------------------------------------ |
| Architecture         | [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)                 |
| Backup & Restore     | [docs/BACKUP_RESTORE.md](docs/BACKUP_RESTORE.md)             |
| CLI Reference        | [v2/docs/CLI.md](v2/docs/CLI.md)                                   |
| Configuration        | [docs/CONFIGURATION.md](docs/CONFIGURATION.md)               |
| Deployment           | [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md)                     |
| Extensions           | [docs/EXTENSIONS.md](docs/EXTENSIONS.md)                     |
| Extension Authoring  | [docs/EXTENSION_AUTHORING.md](docs/EXTENSION_AUTHORING.md)   |
| Project Management   | [docs/PROJECT_MANAGEMENT.md](docs/PROJECT_MANAGEMENT.md)     |
| Secrets Management   | [docs/SECRETS_MANAGEMENT.md](docs/SECRETS_MANAGEMENT.md)     |
| Testing              | [v2/docs/TESTING.md](v2/docs/TESTING.md)                           |
| Schemas              | [docs/SCHEMA.md](docs/SCHEMA.md)                             |
| Troubleshooting      | [v2/docs/TROUBLESHOOTING.md](v2/docs/TROUBLESHOOTING.md)           |
| CI/CD Workflows      | [v2/docs/CI_WORKFLOW_IN_DEPTH.md](v2/docs/CI_WORKFLOW_IN_DEPTH.md) |
| Provider: Docker     | [docs/providers/DOCKER.md](docs/providers/DOCKER.md)         |
| Provider: Fly.io     | [docs/providers/FLY.md](docs/providers/FLY.md)               |
| Provider: DevPod     | [docs/providers/DEVPOD.md](docs/providers/DEVPOD.md)         |
| Provider: Kubernetes | [docs/providers/KUBERNETES.md](docs/providers/KUBERNETES.md) |
