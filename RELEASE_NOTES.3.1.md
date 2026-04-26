# Sindri 3.1.0 Release Notes

**Release Date:** March 2026
**Previous Version:** 3.0.1
**Upgrade Path:** v3.0.x → v3.1.0

> Sindri 3.1.0 is a substantial feature release that adds multi-distro support, collision-aware project initialization, a background service framework, 8 new extensions, and version upgrades across 24 extensions. It also introduces the Ruflo extension as the successor to Claude Flow, a formal extension deprecation system, and significant CI/CD improvements.

---

## Highlights

- **Multi-distro support** — Ubuntu, Fedora, and openSUSE are now first-class targets for containers and extensions
- **Collision-aware project-init** — Smart conflict resolution when extensions share directories (merge-json, append, skip, etc.)
- **Extension service framework** — Background daemons survive container restarts via idempotent service lifecycle management
- **Service port exposure** — Extensions declaratively expose web UIs and network services; providers auto-generate port mappings
- **9 new extensions** — Paperclip, OpenFang, Clarity, OpenClaw, OpenCode, Agent Skills CLI, P-Replicator, RTK, and Ruflo
- **24 extensions upgraded** — Software versions bumped across the entire catalog with BOM tracking
- **Extension deprecation system** — Formal schema support for marking extensions deprecated with migration guidance
- **2 new deployment providers** — RunPod (GPU cloud) and Northflank (container platform)

---

## New Features

### Multi-Distro Container and Extension Support

Sindri containers and extensions now support three Linux distributions out of the box:

| Distribution  | Package Manager | Status  |
| ------------- | --------------- | ------- |
| Ubuntu 24.04  | apt             | Default |
| Fedora 41     | dnf             | New     |
| openSUSE 15.6 | zypper          | New     |

**What's included:**

- `Distro` enum with `Display`, `FromStr`, `Hash`, `Copy` traits in the core type system
- `distros` field on `ExtensionMetadata` for per-extension distro declarations
- `InstallMethod::Dnf` and `InstallMethod::Zypper` install method variants
- `DnfInstallConfig` and `ZypperInstallConfig` with repository, group/pattern support
- Runtime distro detection via `SINDRI_DISTRO` env var and `/etc/os-release` parsing
- Auto-dispatch from apt-declared extensions to dnf/zypper based on detected distro
- `SINDRI_DISTRO` and `SINDRI_PKG_MANAGER_LIB` env vars injected into hooks and scripts
- Multi-arch Dockerfiles for all three distros
- musl static builds for openSUSE glibc compatibility

### Collision-Aware Project Initialization

Extensions that share directories (e.g., `.claude/`) now resolve conflicts intelligently instead of overwriting each other:

- **Priority-based ordering** — Extensions execute `project-init` in priority order (lower = earlier)
- **Conflict rules** — Per-path actions: `merge-json`, `merge-yaml`, `append`, `prepend`, `overwrite`, `backup`, `skip`, `prompt`
- **Version markers** — Detect existing installations via content-match or file/directory-exists patterns
- **Upgrade scenarios** — Stop, Skip, or Proceed based on detected vs. installing version
- **Co-tenancy** — Extensions like Ruflo and Agentic QE can coexist in `.claude/` via merge rules
- **Structured logging** — `SINDRI_LOG_DIR` env var directs extension logs to `~/.sindri/logs/<name>/`

### Extension Service Framework

Background daemons now survive container restarts via a generic `service:` block in `extension.yaml`:

- Declarative service configuration (start, stop, readiness, env requirements)
- Idempotent start scripts generated at `~/.sindri/services/`
- Entrypoint integration auto-starts all registered services on every container boot
- `sindri extension services [list|start|stop|restart]` CLI subcommand
- Draupnir wired as the first consumer

### Service Port Exposure (ADR-050)

Extensions that expose web UIs or network services can now declare their port requirements in `extension.yaml`. Providers automatically generate the correct port mappings — no manual `sindri.yaml` configuration needed.

```yaml
service:
  ports:
    - containerPort: 3100
      protocol: http
      name: web-ui
      ui: true
      healthPath: /api/health
```

**What's included:**

- `ServicePort` and `PortProtocol` types in `sindri-core` with serde support
- `service.ports[]` array in extension schema with 8 fields (`containerPort`, `hostPort`, `protocol`, `name`, `description`, `envOverride`, `ui`, `healthPath`)
- `ServicePortContext` in template context for structured port data in provider templates
- **Docker** — Extension ports rendered as `-p host:container` mappings with descriptive comments
- **Fly.io** — HTTP ports generate `[[services]]` blocks with TLS handlers and optional health checks; TCP ports get plain TCP service blocks
- **Kubernetes** — Extension ports added to the Service spec alongside SSH
- **RunPod** — HTTP ports merged into `expose_ports` for RunPod's proxy system
- **Northflank** — Extension ports mapped to `NorthflankPortConfig` entries with protocol mapping
- Manual `sindri.yaml` ports take precedence (override) over extension defaults
- `envOverride` field enables runtime port remapping via environment variables

**7 extensions updated with port declarations:**

| Extension       | Port(s)    | Protocol  | Web UI |
| --------------- | ---------- | --------- | ------ |
| paperclip       | 3100, 5432 | http, tcp | Yes    |
| excalidraw-mcp  | 3000       | http      | Yes    |
| guacamole       | 8080, 3389 | http, tcp | Yes    |
| openclaw        | 18789      | http      | Yes    |
| ollama          | 11434      | http      | No     |
| claude-code-mux | 13456      | http      | No     |
| xfce-ubuntu     | 3389       | tcp       | Yes    |

### Extension Deprecation System

Extensions can now be formally marked as deprecated with migration guidance:

- Schema support for `deprecated` field with `message`, `replacement`, and `since` metadata
- Registry and validation integration — deprecated extensions show warnings
- Claude Flow V2 and V3 marked as deprecated in favor of Ruflo

### CLI JSON Mode

- `--json` flag redirects informational output to stderr so stdout contains only the JSON payload
- Enables clean piping and machine-readable output for scripting

### Force Reinstall and Removed State

- `sindri extension install --force` bypasses the "already installed" check
- Passes `--force` to mise with explicit `TOOL@VERSION` arguments parsed from mise TOML configs
- New `Removed` extension state — removal events use a proper terminal state instead of reusing `Installed`
- Bundled (flat-directory) extensions can now be removed properly

### Compatibility Matrix Pre-Commit Validation

- `scripts/validate-compat-matrix.py` validates extension versions against `compatibility-matrix.yaml` semver ranges
- Wired into `.husky/pre-commit` (conditional on extension/matrix file changes)
- `make v3-validate-compat` target for CI integration

### New Deployment Providers

- **RunPod** — GPU cloud provider with config templates and documented options
- **Northflank** — Container platform provider
- `config providers` subcommand with `--json` support
- `Provider::all_names()` and `Provider::description()` helpers
- Credential-gated CI validation: Tier 1 (always) and Tier 2 (gated by secrets)

### Cosign Baked into Docker Images

- Cosign v3.0.5 binary baked into all three Dockerfiles using parallel multi-stage download
- `ImageVerifier` can now find cosign in PATH for signature verification, provenance checks, and SBOM downloads

---

## New Extensions

| Extension            | Version | Category  | Description                                                                                                     |
| -------------------- | ------- | --------- | --------------------------------------------------------------------------------------------------------------- |
| **paperclip**        | 1.0.0   | ai-dev    | AI agent orchestrator with React dashboard for managing agent teams (ports 3100, 5432)                          |
| **ruflo**            | 3.5.36  | claude    | AI Agent Orchestration Platform — successor to Claude Flow with HNSW search, Flash Attention, and 215 MCP tools |
| **openfang**         | 1.1.0   | ai-agents | Open-source agent OS for autonomous AI agents across 40+ messaging platforms                                    |
| **clarity**          | 1.0.0   | ai-dev    | Autonomous spec generation skill from reference materials (5-phase workflow)                                    |
| **openclaw**         | 1.0.0   | ai-dev    | Multi-channel AI gateway for messaging platforms with browser Control UI                                        |
| **opencode**         | 1.0.0   | ai-dev    | Open source AI coding agent for terminal, desktop, and IDE                                                      |
| **agent-skills-cli** | 1.1.7   | claude    | Claude Code agent skills management CLI                                                                         |
| **p-replicator**     | 1.0.0   | ai-dev    | Claude Code toolkit for AI-assisted product development (Vibe Coding)                                           |
| **rtk**              | 1.0.0   | ai-dev    | RTK (Rust Token Killer) — high-performance CLI proxy reducing LLM token consumption by 60-90%                   |

---

## Extension Upgrades

### Software Version Bumps (24 extensions)

#### Cloud & Infrastructure

| Extension                     | Component              | Old Version | New Version |
| ----------------------------- | ---------------------- | ----------- | ----------- |
| **cloud-tools** (2.2.0→2.3.0) | aws-cli                | 2.33.21     | 2.34.11     |
|                               | azure-cli              | 2.83.0      | 2.84.0      |
|                               | gcloud SDK             | 556.0.0     | 561.0.0     |
|                               | flyctl                 | 0.4.11      | 0.4.23      |
|                               | aliyun-cli             | 3.2.9       | 3.3.2       |
|                               | doctl                  | 1.150.0     | 1.152.0     |
| **infra-tools** (2.2.0→2.3.0) | pulumi                 | 3.220.0     | 3.226.0     |
|                               | crossplane             | 2.1.4       | 2.2.0       |
|                               | packer                 | 1.14        | 1.15        |
|                               | kbld                   | 0.47.1      | 0.47.2      |
|                               | kapp/ytt/vendir/imgpkg | various     | patch bumps |

#### Languages & Runtimes

| Extension                 | Component    | Old Version | New Version |
| ------------------------- | ------------ | ----------- | ----------- |
| **jvm** (2.1.1→2.2.0)     | maven        | 3.9.12      | 3.9.14      |
|                           | gradle       | 9.3.1       | 9.4.0       |
|                           | kotlin       | 2.3.10      | 2.3.20      |
|                           | scala        | 3.8.1       | 3.8.2       |
| **haskell** (2.1.0→2.2.0) | ghc          | 9.12.2      | 9.12.3      |
|                           | cabal        | 3.14.1      | 3.16.1.0    |
|                           | stack        | 3.3.1       | 3.9.3       |
|                           | ghcup        | 0.1.30      | 0.1.50      |
| **python** (1.1.0→1.2.0)  | Python       | 3.13        | 3.14        |
|                           | uv           | 0.9         | 0.10        |
| **swift** (1.0.0→1.0.1)   | swift/swiftc | 6.2.3       | 6.2.4       |
| **php** (2.1.0→2.2.0)     | PHP          | 8.4         | 8.5         |

#### AI & Agent Tools

| Extension                         | Component      | Old Version | New Version |
| --------------------------------- | -------------- | ----------- | ----------- |
| **ruflo** (3.5.2→3.5.36)          | ruflo          | 3.5.2       | 3.5.36      |
| **claude-codepro** (4.5.29→7.6.2) | claude-codepro | 4.5.29      | 7.6.2       |
| **ai-toolkit** (2.3.0→2.4.0)      | gemini-cli     | 0.27.1      | 0.32.1      |
| **agent-browser** (1.1.0→1.2.0)   | agent-browser  | 0.9.3       | 0.21.0      |
| **agentic-qe** (1.3.0→1.3.1)      | agentic-qe     | 3.6.4       | 3.8.2       |
| **gitnexus** (1.0.1→1.1.0)        | gitnexus       | 1.3.7       | 1.4.6       |

#### Other

| Extension                         | Component | Old Version | New Version |
| --------------------------------- | --------- | ----------- | ----------- |
| **supabase-cli** (2.1.0→2.2.0)    | supabase  | 2.76.4      | 2.78.1      |
| **mdflow** (1.0.0→1.1.0)          | mdflow    | 2.33        | 2.35.5      |
| **nodejs-devtools** (2.2.0→2.3.0) | eslint    | 9           | 10          |
|                                   | prettier  | 3.6         | 3.8         |
| **draupnir** (1.0.0→1.2.2)        | draupnir  | 1.0.0       | 1.2.2       |

### GitHub CLI Cross-Cutting Upgrade

GitHub CLI upgraded from **2.87.3 → 2.88.1** across:

- `v3/Dockerfile` and `v3/Dockerfile.base` (build args)
- `.github/workflows/build-base-image.yml` (defaults)
- `v3/extensions/github-cli/extension.yaml` (target version)
- `v3/docs/MAINTAINER_GUIDE.md` and `.github/WORKFLOW_ARCHITECTURE.md` (documentation)

---

## Deprecations

| Extension          | Replacement | Notes                                                         |
| ------------------ | ----------- | ------------------------------------------------------------- |
| **claude-flow-v2** | ruflo       | Stable predecessor — use `ruflo` for all new projects         |
| **claude-flow-v3** | ruflo       | Alpha predecessor — ruflo is the production-branded successor |

---

## CI/CD Improvements

- **musl static builds** for openSUSE glibc compatibility (5 fix commits for cross-compilation)
- **Node.js 20 deprecation** warnings resolved in GitHub Actions
- **Credential-gated provider validation** — two-tier CI model (always-run vs. secret-gated)
- **GitHub Actions bumps** — upload-artifact v7, download-artifact v8, attest-build-provenance v4, docker/setup-qemu-action v4, cargo-binstall 1.17.6
- **Rust toolchain** upgraded from 1.93 to 1.94
- **Cosign-installer** bumped and gh version default fixed

## Security

- Removed unused `octocrab` and `self_update` dependencies to reduce attack surface
- Upgraded `markdownlint-cli` to 0.48 (security fix)
- Bumped `quinn-proto` 0.11.13→0.11.14 (security fix)

## Dependency Updates

- `jsonschema` 0.42.2 → 0.44.1
- `aws-config` 1.8.14 → 1.8.15
- `aws-sdk-s3` 1.124.0 → 1.125.0
- `fastify` bumped (console API)
- `minimatch` bumped (v2 Docker dependencies)
- 5 v3 workspace dependency bumps

## Bug Fixes

- Fixed Haskell and PHP extension install failures
- Fixed Clarity extension install script
- Added GitNexus platform guard (skip on linux-arm64 due to tree-sitter native dependency)
- Fixed Claude Code authentication status fallback check
- Fixed registry: moved ai-toolkit to correct category section
- Fixed config file paths: moved to `/docker/config/sindri`
- Fixed draupnir env config to use schema-required `key` field

## Documentation

- Added README files for all v3 crates
- Extension service framework guide (SERVICES.md) and ADR-048
- Service port exposure architecture documented in ADR-050
- Collision handling architecture documented in ADR-047
- Updated extension-guide-v3 skill with `service.ports` documentation and compatibility matrix guidance
- Added service port sections to provider docs (Docker, Fly.io, Kubernetes)
- Added port conflict troubleshooting to TROUBLESHOOTING.md
- Added extension service ports section to GETTING_STARTED.md
- Added service port info to CLI `extension status` reference

---

## Upgrade Instructions

### From v3.0.x

```bash
# Pull the latest CLI binary
sindri self-update

# Or rebuild Docker image
docker build -f v3/Dockerfile -t sindri:v3.1.0 .

# Verify
sindri --version
# sindri 3.1.0

# Update extensions to latest compatible versions
sindri extension upgrade --all
```

### Extension Compatibility

All existing v3.0.x extensions remain compatible. The `compatibility-matrix.yaml` has been updated with corrected ranges. Extensions with software upgrades have bumped metadata versions but remain within their semver ranges for v3.0.x compatibility.

**New in 3.1.x schema (v1.1):**

- `distros` field for multi-distro support
- `service` block for background daemon lifecycle with `ports[]` array for declarative port exposure
- `deprecated` field with migration guidance
- Collision handling capabilities

---

## Stats

| Metric                   | Value                          |
| ------------------------ | ------------------------------ |
| Commits since v3.0.1     | 65                             |
| New extensions           | 9                              |
| Extensions upgraded      | 24                             |
| New deployment providers | 2                              |
| Supported distros        | 3 (was 1)                      |
| New tests added          | 45+ (collision handling alone) |
