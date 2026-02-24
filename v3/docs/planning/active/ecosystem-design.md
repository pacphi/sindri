# Sindri Ecosystem: CLI, Fleet Management & Instance Agent

## Design Document — February 2026

---

## 1. Sindri's Core Identity

Sindri is a declarative, provider-agnostic cloud development environment system. It lets you define a development environment in YAML (`sindri.yaml`) and deploy that identical environment across multiple providers:

- **Docker** (local)
- **Fly.io** (cloud)
- **DevPod** (Kubernetes, AWS, GCP, Azure backends)
- **RunPod** (GPU workloads)
- **Northflank** (containerized services)
- **E2B** (ultra-fast sandboxes)

The CLI workflow: `sindri config init` → edit `sindri.yaml` → `sindri deploy --provider <target>` → `sindri connect`.

Sindri stays focused on this: **provision, configure, and connect**. Fleet management, observability, and administration are handled by the companion projects in the ecosystem.

---

## 2. The Three-Project Ecosystem

```
sindri (CLI tool)
    │
    │ installs draupnir as extension
    ▼
draupnir (Go agent on each instance)
    │
    │ registers via REST, connects via WebSocket
    ▼
mimir (control plane: API + web dashboard)
```

| Repository | Role | Language |
|---|---|---|
| **sindri** (this repo) | CLI tool — provisions, configures, and connects to instances | Rust |
| [mimir](https://github.com/pacphi/mimir) | Fleet management control plane — orchestrates, observes, and administers instances at scale | TypeScript |
| [draupnir](https://github.com/pacphi/draupnir) | Lightweight per-instance agent — bridges each instance to mimir | Go |

### Dependency model

- **Build-time:** All three repos build independently. No compile-time dependency.
- **Runtime:** sindri installs draupnir (via extension system); draupnir connects to mimir.
- **Shared contract:** WebSocket protocol (JSON envelope with `protocol_version` field).

### Safe independent changes (no coordination needed)

- New sindri extensions, provider adapters, or CLI commands
- Mimir UI changes, new dashboard views
- Draupnir performance improvements
- Additive fields in protocol payloads

### Coordinated changes (require aligned releases)

- Protocol version bumps (breaking message format changes)
- Registration endpoint schema changes
- Mimir API breaking changes that draupnir calls

---

## 3. Sindri CLI (v3 Rust Implementation)

### Workspace Structure

```
v3/
├── crates/
│   ├── sindri/              # Main CLI binary and command routing
│   ├── sindri-core/         # Shared types (BomConfig, Extension, etc.)
│   ├── sindri-extensions/   # BOM engine, extension registry, distributor
│   ├── sindri-providers/    # Provider adapters (Docker, Fly, DevPod, RunPod, Northflank, ...)
│   ├── sindri-secrets/      # Cross-provider secrets management
│   ├── sindri-backup/       # Workspace backup and restore
│   ├── sindri-projects/     # new-project / clone-project
│   ├── sindri-doctor/       # Environment health checks
│   ├── sindri-clusters/     # Cluster management
│   ├── sindri-image/        # Docker image operations
│   ├── sindri-packer/       # Packer image building
│   └── sindri-update/       # Self-update mechanism
├── extensions/              # 60+ extension YAML definitions
├── docs/                    # CLI.md, EXTENSIONS.md, AUTHORING.md, ADRs
├── profiles.yaml            # Named extension profiles
├── registry.yaml            # Extension registry metadata
└── compatibility-matrix.yaml # CLI version compatibility
```

### Key Architectural Concepts

| Concept | Description |
|---|---|
| **Extension System** | YAML-driven modules with dependency resolution. 60+ extensions across AI, languages, infrastructure, databases, and tools. |
| **Provider Adapters** | Common interface for deploy, destroy, status, connect across all providers. |
| **Volume Architecture** | Immutable system layer + mutable `$HOME` volume that survives redeployments. |
| **BOM Tracking** | Every installed tool's version, source, and hash tracked for SBOM and security auditing. |
| **Schema Validation** | All YAML validated against JSON schemas before deployment. |

---

## 4. Draupnir Extension

Draupnir is distributed as a Sindri v3 extension. Users install it with:

```bash
sindri extension install draupnir
```

The extension definition at `v3/extensions/draupnir/extension.yaml`:

```yaml
metadata:
  name: draupnir
  version: 1.0.0
  description: Sindri instance agent for mimir fleet management
  category: devops
  homepage: https://github.com/pacphi/draupnir

install:
  method: script
  script:
    url: https://raw.githubusercontent.com/pacphi/draupnir/main/extension/install.sh
    timeout: 120

validate:
  commands:
    - name: sindri-agent
      versionFlag: --version
      expectedPattern: "\\d+\\.\\d+\\.\\d+"
```

When draupnir releases a new version, its CI automatically opens a PR to this repo updating the `version` field.

---

## 5. `@sindri/cli` npm Package Distribution

The Sindri CLI (Rust binary) is distributed as an npm package using the **optionalDependencies + platform packages** pattern (same as esbuild, Biome, SWC). This allows mimir to add `@sindri/cli` as a regular npm dependency and resolve the binary without system installation.

### Package layout (`packages/@sindri/`)

| Package | Platform | Contents |
|---|---|---|
| `@sindri/cli` | wrapper | `optionalDependencies` + resolver script |
| `@sindri/cli-darwin-arm64` | macOS/ARM64 | `sindri` binary |
| `@sindri/cli-darwin-x64` | macOS/x64 | `sindri` binary |
| `@sindri/cli-linux-x64` | Linux/x64 | `sindri` binary |
| `@sindri/cli-linux-arm64` | Linux/ARM64 | `sindri` binary |
| `@sindri/cli-win32-x64` | Windows/x64 | `sindri.exe` binary |

### Version alignment

The npm packages are versioned identically to the Rust binary (`Cargo.toml` is the source of truth). **cargo-dist** automates multi-platform builds and npm publishing triggered by `git tag v3.x.y`.

### cargo-dist configuration (`v3/Cargo.toml`)

```toml
[workspace.metadata.dist]
installers = ["shell", "powershell", "npm"]
targets = [
    "x86_64-unknown-linux-musl",
    "aarch64-unknown-linux-musl",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
]
npm-scope = "@sindri"
npm-package = "@sindri/cli"
ci = ["github"]
create-release = true
checksum = "sha256"
```

---

## 6. Instance Version Awareness

Draupnir agents report `sindri_version` and `cli_target` in heartbeat payloads. Mimir surfaces this as compatibility badges per instance.

The `GET /api/v1/version` endpoint (backed by `sindri version --json`) exposes:

```json
{
  "console_api": "0.1.0",
  "sindri_cli": "3.0.1",
  "cli_target": "aarch64-apple-darwin",
  "min_instance_version": "3.0.0"
}
```

Badge semantics:
- **Green**: instance's sindri version matches console's minor version
- **Yellow**: older patch/minor (minor feature gaps possible)
- **Red**: major version mismatch (API calls may fail)

---

## 7. Implementation Status (as of February 2026)

| Component | Status |
|---|---|
| Rust CLI v3 (12 crates) | ✅ v3.0.1 released |
| 60+ extensions | ✅ Complete |
| RunPod + Northflank providers | ✅ Complete |
| `v3/extensions/draupnir/extension.yaml` | ✅ Present (v1.0.0) |
| `packages/@sindri/cli*` npm packages | ✅ Scaffolded at v3.0.1 |
| cargo-dist configuration | ✅ Complete |
| Automated cross-platform builds | ✅ CI on `v3.*.*` tags |
| SLSA provenance + Cosign signing | ✅ Complete |
| `sindri version --json` | ✅ Used by mimir's `/api/v1/version` |

---

## 8. Why This Fits Sindri's Philosophy

Sindri's ethos is "define once, deploy anywhere." The ecosystem extends this to "define once, deploy anywhere, **observe everywhere**." The control plane (mimir) and agent (draupnir) don't replace the CLI or the YAML-driven workflow. Instead they:

- **Complement** the CLI for users who prefer visual interfaces
- **Aggregate** what was previously invisible across isolated instances
- **Enable** workflows impossible from a single terminal (fleet management, cross-instance comparison)
- **Dogfood** Sindri itself — mimir is deployable via `sindri deploy`

The agent (draupnir) is opt-in, installed like any other extension, following Sindri's declarative philosophy.
