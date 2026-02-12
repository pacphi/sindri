# Sindri 3.0.0 Release Notes

**Release Date:** February 2026
**Previous Version:** 2.5.0
**Upgrade Path:** v2.x.x → v3.0.0

> 📊 **Evaluating versions?** See the [Comparison Guide](docs/migration/COMPARISON_GUIDE.md) for feature matrices and architectural differences.
>
> 📖 **Ready to migrate?** See the [Migration Guide](docs/migration/MIGRATION_GUIDE.md) for step-by-step instructions.

---

## 🚨 Breaking Changes

Sindri 3.0.0 is a **ground-up rewrite** from Bash to Rust. Every aspect of the CLI has been rebuilt for performance, reliability, and cross-platform support. This release is a **major version** due to the complete architectural transformation, configuration schema changes, extension schema updates, removed extensions, and environment variable changes.

### 1. Complete CLI Rewrite (Bash → Rust)

**Impact:** All users, all scripts, all CI/CD pipelines

**What Changed:**

The entire Sindri CLI has been rewritten from ~52,000 lines of Bash across dozens of scripts into ~11,200 lines of Rust organized as a 12-crate workspace. The result is a single statically-linked ~12MB binary with zero runtime dependencies.

| Metric            | V2 (Bash)                 | V3 (Rust)                   |
| ----------------- | ------------------------- | --------------------------- |
| Implementation    | ~52,000 lines of Bash     | ~11,200 lines of Rust       |
| Distribution      | Git clone + Docker        | Binary + Docker + npm       |
| Runtime deps      | bash, yq, jq, jsonschema  | None (single binary)        |
| CLI startup       | 2-5 seconds               | <100ms                      |
| Config parsing    | 100-500ms (yq/jq subproc) | 10-50ms (native serde)      |
| Schema validation | 100-500ms (python)        | 10-50ms (native jsonschema) |
| Error handling    | Exit codes + stderr       | Result<T, E> + anyhow       |
| Async runtime     | None (sequential)         | Tokio 1.49                  |
| Testing           | Limited scripts           | cargo test (28+ unit tests) |

**Workspace Crates:**

```
v3/crates/
├── sindri/             # Main CLI (clap 4.5 derive)
├── sindri-core/        # Types, config, schemas
├── sindri-providers/   # Docker, Fly, DevPod, E2B, K8s
├── sindri-extensions/  # DAG resolution, validation
├── sindri-secrets/     # env, file, vault, S3
├── sindri-update/      # Self-update framework
├── sindri-backup/      # Workspace backup/restore
├── sindri-projects/    # Scaffolding and git workflows
├── sindri-doctor/      # System diagnostics
├── sindri-clusters/    # Kubernetes management
├── sindri-image/       # Container image management
└── sindri-packer/      # VM image building (Packer)
```

**Before (v2.x):**

```bash
# V2: Multiple scripts, relative paths, external dependencies
git clone https://github.com/pacphi/sindri.git
cd sindri
export PATH="$PWD/v2/cli:$PATH"
./v2/cli/sindri deploy --provider docker
```

**After (v3.0):**

```bash
# V3: Single binary, zero dependencies
wget https://github.com/pacphi/sindri/releases/latest/download/sindri-linux-x86_64.tar.gz
tar -xzf sindri-linux-x86_64.tar.gz
sudo mv sindri /usr/local/bin/
sindri deploy
```

**References:**

- [ADR-001: Rust Migration Workspace Architecture](v3/docs/architecture/adr/001-rust-migration-workspace-architecture.md)
- [Architecture Overview](v3/docs/ARCHITECTURE.md)

---

### 2. CLI Command Changes

**Impact:** All scripts and CI/CD pipelines using V2 CLI commands

**What Changed:**

V2 used multiple standalone scripts (`sindri`, `extension-manager`, `secrets-manager`, `backup-restore`, `new-project`, `clone-project`) accessed via relative paths. V3 consolidates everything into a single `sindri` binary with a hierarchical subcommand structure.

**Deployment Commands:**

| V2 Command                               | V3 Command       | Notes                 |
| ---------------------------------------- | ---------------- | --------------------- |
| `./v2/cli/sindri deploy --provider <p>`  | `sindri deploy`  | Provider from config  |
| `./v2/cli/sindri destroy --provider <p>` | `sindri destroy` | Added --volumes flag  |
| `./v2/cli/sindri connect`                | `sindri connect` | Added -c/--command    |
| `./v2/cli/sindri status`                 | `sindri status`  | Added --json, --watch |

**Extension Commands:**

| V2 Command                                       | V3 Command                          | Notes                     |
| ------------------------------------------------ | ----------------------------------- | ------------------------- |
| `./v2/cli/extension-manager list`                | `sindri extension list`             | Merged into CLI           |
| `./v2/cli/extension-manager install <n>`         | `sindri extension install <n>`      | Added @version, --profile |
| `./v2/cli/extension-manager install-profile <n>` | `sindri profile install <n>`        | Moved to profile          |
| `./v2/cli/extension-manager validate <n>`        | `sindri extension validate <n>`     | Added --file              |
| `./v2/cli/extension-manager status [n]`          | `sindri extension status [n]`       | Added --json              |
| `./v2/cli/extension-manager info <n>`            | `sindri extension info <n>`         | Added --json              |
| `./v2/cli/extension-manager bom [n]`             | `sindri extension list --installed` | Auto BOM                  |

**Secrets Commands:**

| V2 Command                          | V3 Command                | Notes           |
| ----------------------------------- | ------------------------- | --------------- |
| `./v2/cli/secrets-manager validate` | `sindri secrets validate` | Merged into CLI |
| `./v2/cli/secrets-manager list`     | `sindri secrets list`     | Added --source  |

**Project Commands:**

| V2 Command                     | V3 Command                   | Notes            |
| ------------------------------ | ---------------------------- | ---------------- |
| `./v2/cli/new-project <name>`  | `sindri project new <name>`  | Moved to project |
| `./v2/cli/clone-project <url>` | `sindri project clone <url>` | Moved to project |

**Configuration Commands:**

| V2 Command                        | V3 Command               | Notes              |
| --------------------------------- | ------------------------ | ------------------ |
| `./v2/cli/sindri config init`     | `sindri config init`     | Added --profile    |
| `./v2/cli/sindri config validate` | `sindri config validate` | Added --check-exts |
| `./v2/cli/sindri profiles list`   | `sindri profile list`    | Singular 'profile' |

**References:**

- [CLI Reference](v3/docs/CLI.md) - Complete V3 command reference
- [Migration Guide: Command Mapping](docs/migration/MIGRATION_GUIDE.md#command-mapping-v2-to-v3)

---

### 3. Configuration Schema (v2 → v3)

**Impact:** All users with sindri.yaml files

**What Changed:**

The `sindri.yaml` configuration schema has been updated to version 3.0 with structured image configuration, enhanced GPU support, and new provider options.

**Before (v2.x):**

```yaml
# V2 sindri.yaml
name: my-project
provider: docker
profile: ai-dev
secrets:
  provider:
    vault:
      path: secret/sindri
```

**After (v3.0):**

```yaml
# V3 sindri.yaml
schemaVersion: "3.0"
name: my-project
provider: docker-compose
profile: ai-dev
image:
  registry: ghcr.io
  repository: pacphi/sindri
  tag: v3
  verify: true
secrets:
  provider:
    vault:
      mount_path: secret/sindri
```

**Key Schema Changes:**

| Change                            | Impact                         | Migration                     |
| --------------------------------- | ------------------------------ | ----------------------------- |
| `schemaVersion` field added       | Required in V3                 | Auto-added during migration   |
| `vault.path` → `vault.mount_path` | Config key rename              | Auto-migrated; backup created |
| `image` section added             | Structured image configuration | Optional new feature          |
| `image.verify` field              | Enable signature verification  | Optional new feature          |

**Migration Required If:**

- ✅ You have a **sindri.yaml** config file
- ✅ You use **Vault secrets** with `secrets.provider.vault.path`

**Action Required:**

```bash
# V3 auto-migrates on first run and creates a backup
sindri config validate
# Creates sindri.yaml.v2.backup automatically

# Review the migrated config
cat sindri.yaml
```

**References:**

- [Configuration Reference](v3/docs/CONFIGURATION.md) - Full V3 sindri.yaml specification
- [Schema Reference](v3/docs/SCHEMA.md) - YAML schema documentation

---

### 4. Manifest Format Changes

**Impact:** Users with existing `~/.sindri/manifest.yaml` files

**What Changed:**

The manifest file that tracks installed extensions and CLI state has been updated to a new format for V3's enhanced extension lifecycle management.

**Migration Required If:**

- ✅ You have an existing `~/.sindri/manifest.yaml` from V2

**Action Required:**

No manual action needed. V3 auto-migrates the manifest on first run and creates `~/.sindri/manifest.yaml.v2.backup`.

```bash
# V3 handles this automatically
sindri version
# Manifest auto-migrated with backup

# If corruption occurs, restore from backup
cp ~/.sindri/manifest.yaml.v2.backup ~/.sindri/manifest.yaml
```

**References:**

- [ADR-012: Registry-Manifest Dual State Architecture](v3/docs/architecture/adr/012-registry-manifest-dual-state-architecture.md)

---

### 5. Extension Schema (v0.x → v1.0)

**Impact:** Extension authors, users with custom extensions

**What Changed:**

Extension definitions have been updated from the informal v0.x schema to a strict v1.0 schema with compile-time type safety, 80+ Rust structs, and stricter validation rules.

**Before (v2.x):**

```yaml
# V2 extension.yaml
metadata:
  name: my-extension
  version: 1.0.0
  description: My custom extension
  category: utilities
install:
  method: manual
  commands:
    - echo "Installing..."
```

**After (v3.0):**

```yaml
# V3 extension.yaml
metadata:
  name: my-extension
  version: 1.0.0
  description: My custom extension
  category: productivity
install:
  method: script
  commands:
    - echo "Installing..."
```

**Key Schema Changes:**

| Change                          | Impact                    | Migration                           |
| ------------------------------- | ------------------------- | ----------------------------------- |
| Install method `manual` removed | Extensions using 'manual' | Use `script` method instead         |
| Category taxonomy updated       | 11 → 13 categories        | Map to new category names           |
| Stricter field validation       | Malformed extensions fail | Run `sindri extension validate-all` |
| Conditional templates added     | New `condition` field     | Optional new feature                |

**Migration Required If:**

- ✅ You have **custom extensions** using install method `manual`
- ✅ You have **custom extensions** with V2-only category names

**Action Required:**

```bash
# Validate all extensions against V3 schema
sindri extension validate-all

# Fix specific extension
sindri extension validate my-extension --file ./extension.yaml
```

**References:**

- [ADR-008: Extension Type System YAML Deserialization](v3/docs/architecture/adr/008-extension-type-system-yaml-deserialization.md)
- [Extension Authoring Guide](v3/docs/extensions/guides/AUTHORING.md)

---

### 6. Removed Extensions

**Impact:** Users with profiles/configurations referencing removed extensions

Three legacy extensions and all 33 VisionFlow extensions have been removed from V3:

**Legacy Extensions:**

| Removed Extension          | Replacement                          | Reason                                         |
| -------------------------- | ------------------------------------ | ---------------------------------------------- |
| `claude-flow` (v1)         | `claude-flow-v2` or `claude-flow-v3` | Split into stable (v2) and alpha (v3) versions |
| `claude-auth-with-api-key` | Native multi-method authentication   | Obsolete with built-in flexible auth           |
| `ruvnet-aliases`           | Consolidated into other extensions   | Functionality moved to individual extensions   |

**VisionFlow Extensions (33 total):**

All `vf-*` prefixed extensions are excluded from V3 due to GPU dependencies, desktop requirements, and Docker integration complexity. This includes: `vf-algorithmic-art`, `vf-blender`, `vf-canvas-design`, `vf-chrome-devtools`, `vf-comfyui`, `vf-docker-manager`, `vf-docx`, `vf-ffmpeg-processing`, `vf-gemini-flow`, `vf-imagemagick`, `vf-import-to-ontology`, `vf-jupyter-notebooks`, `vf-kicad`, `vf-latex-documents`, `vf-management-api`, `vf-ngspice`, `vf-ontology-enrich`, `vf-pbr-rendering`, `vf-pdf`, `vf-perplexity`, `vf-pptx`, `vf-pytorch-ml`, `vf-qgis`, `vf-slack-gif-creator`, `vf-vnc-desktop`, `vf-wardley-maps`, `vf-web-summary`, `vf-webapp-testing`, `vf-xlsx`, `vf-zai-service`, `vf-deepseek-reasoning`, `vf-playwright-mcp`, `vf-mcp-builder`.

**Migration Examples:**

```bash
# Replace claude-flow v1 with v2 or v3
sindri extension install claude-flow-v2   # Stable
sindri extension install claude-flow-v3   # Alpha (advanced features)

# claude-auth-with-api-key: No replacement needed
# Authentication is now built-in to extensions that require it

# ruvnet-aliases: No replacement needed
# Functionality consolidated into individual extensions

# VisionFlow users: Continue using V2 for vf-* workflows
# V2 and V3 can coexist side-by-side
```

**References:**

- [Comparison Guide: Extension Comparison](docs/migration/COMPARISON_GUIDE.md#extension-comparison)

---

### 7. Environment Variable Changes

**Impact:** Docker users, CI/CD pipelines, custom build scripts

**What Changed:**

V3's two-Dockerfile architecture replaces the dual-variable build mode system with a single unified variable.

**Before (v2.x / early v3 alpha):**

```bash
# V2/early V3: Two variables controlled build mode
export SINDRI_BUILD_FROM_SOURCE=true
export SINDRI_EXTENSIONS_SOURCE=/opt/sindri/extensions

docker build -f v3/Dockerfile \
  --build-arg BUILD_FROM_SOURCE=true \
  -t sindri:dev .
```

**After (v3.0):**

```bash
# V3: Single unified variable, Dockerfile choice determines mode
export SINDRI_EXT_HOME=${HOME}/.sindri/extensions

# Production (pre-built binary, runtime extension install)
docker build -f v3/Dockerfile -t sindri:prod .

# Development (source build, bundled extensions)
docker build -f v3/Dockerfile.dev -t sindri:dev .
```

**Variable Changes:**

| Variable                   | Status  | Replacement       | Notes                                |
| -------------------------- | ------- | ----------------- | ------------------------------------ |
| `SINDRI_BUILD_FROM_SOURCE` | Removed | Dockerfile choice | Use `Dockerfile` vs `Dockerfile.dev` |
| `SINDRI_EXTENSIONS_SOURCE` | Removed | `SINDRI_EXT_HOME` | Unified path variable                |
| `SINDRI_EXT_HOME`          | New     | -                 | Extensions directory path            |
| `BUILD_FROM_SOURCE`        | Removed | Dockerfile choice | Was a Docker build arg               |

**References:**

- [ADR-040: Two-Dockerfile Architecture](v3/docs/architecture/adr/040-two-dockerfile-architecture.md)
- [Deployment Guide](v3/docs/DEPLOYMENT.md)

---

### 8. Two-Dockerfile Architecture

**Impact:** Docker builds, CI/CD pipelines

**What Changed:**

V3 replaces the single Dockerfile with build-arg toggling with two purpose-built Dockerfiles for clear separation of concerns.

| Dockerfile          | Target      | Size   | Build Time | Extensions      |
| ------------------- | ----------- | ------ | ---------- | --------------- |
| `v3/Dockerfile`     | Production  | ~800MB | 2-5 min    | Runtime install |
| `v3/Dockerfile.dev` | Development | ~1.2GB | ~8 min     | Pre-bundled     |

**Before (v2.x):**

```bash
# V2: Single Dockerfile with build-arg to toggle mode
docker build -f v3/Dockerfile \
  --build-arg BUILD_FROM_SOURCE=true \
  -t sindri:dev .

docker build -f v3/Dockerfile \
  --build-arg BUILD_FROM_SOURCE=false \
  -t sindri:prod .
```

**After (v3.0):**

```bash
# V3: Dockerfile choice determines mode
docker build -f v3/Dockerfile -t sindri:prod .      # Production (~800MB)
docker build -f v3/Dockerfile.dev -t sindri:dev .    # Development (~1.2GB)

# Or using Makefile
make v3-docker-build-from-binary   # Production
make v3-docker-build-from-source   # Development
```

**Choosing the Right Dockerfile:**

| Criterion          | Production (`Dockerfile`) | Development (`Dockerfile.dev`) |
| ------------------ | ------------------------- | ------------------------------ |
| Build time         | 2-5 minutes               | ~8 minutes                     |
| Image size         | ~800MB                    | ~1.2GB                         |
| Extensions         | Runtime installation      | Pre-bundled                    |
| Use case           | Production, CI/CD         | Development, testing           |
| Binary source      | Pre-compiled              | Built from source              |
| Air-gapped support | No                        | Yes                            |

**Migration Required If:**

- ✅ You have **custom Docker build scripts** using `BUILD_FROM_SOURCE`
- ✅ You have **CI/CD pipelines** building Sindri images
- ✅ You have **Makefiles** or automation using old build args

**References:**

- [ADR-040: Two-Dockerfile Architecture](v3/docs/architecture/adr/040-two-dockerfile-architecture.md)
- [Deployment Guide](v3/docs/DEPLOYMENT.md)
- [V3 README: Building Docker Images](v3/README.md#building-docker-images)

---

## ✨ New Features

### 1. Bill of Materials (BOM) Generation

**Benefit:** Comprehensive Software Bill of Materials (SBOM) for security and compliance

Sindri v3 now includes built-in Software Bill of Materials (SBOM) generation and verification capabilities for tracking all installed extensions and their software versions.

**Key Features:**

- **CLI Commands**: `sindri bom generate` and `sindri bom verify` for creating and validating BOMs
- **Software Version Tracking**: 88% of extensions (44/50) now include pinned software versions
- **Automated Documentation**: BOM sections automatically generate in extension documentation
- **Compliance Ready**: Supports security auditing and compliance workflows
- **Comprehensive Testing**: 120 tests (105 unit + 15 integration) ensure reliability

**Usage:**

```bash
# Generate BOM for installed extensions
sindri bom generate

# Generate BOM with specific output format
sindri bom generate --format json --output bom.json

# Verify BOM against current installation
sindri bom verify --file bom.json

# Generate BOM for specific profile
sindri bom generate --profile fullstack
```

**BOM Output Example:**

```yaml
version: 1.0.0
generated_at: 2026-02-10T12:00:00Z
components:
  - name: nodejs
    version: 1.0.0
    software:
      - name: node
        version: 22.13.1
      - name: pnpm
        version: 10.0.0 (corepack-managed)
    category: languages
```

**References:**

- [ADR-042: BOM Capability Architecture](v3/docs/architecture/adr/042-bom-capability-architecture.md)
- [Extension Authoring Guide: BOM Section](v3/docs/extensions/guides/AUTHORING.md#bom-recommended)

---

### 2. Enhanced Extension Management

**Benefit:** Improved visibility and control over extension installation state

V3 introduces significant enhancements to extension management with unified views, software version display, and installation state tracking.

**Extension List Improvements:**

- **Unified Extension View**: New `--all` flag shows both available and installed extensions in one view
- **Software Version Display**: See exact versions of tools (e.g., "python (3.13), pip (26.0.1)")
- **Installation State Tracking**: Failed installations now visible via `sindri extension status`
- **Parallel Metadata Fetching**: Extension list loads in 3-5 seconds (first run), instant thereafter
- **Smart Caching**: Extension metadata cached in `~/.sindri/cache/extensions/`
- **Status Datetime Field**: Renamed from `installed_at` for better clarity (backward compatible)

**Usage:**

```bash
# List all extensions (available + installed)
sindri extension list --all

# Show only installed extensions with versions
sindri extension list --installed

# Check installation status for specific extension
sindri extension status nodejs

# View detailed extension information
sindri extension info nodejs
```

**Installation State Visibility:**

```bash
$ sindri extension status
mise        ✓ installed    2026-02-10 10:30:00
nodejs      ✓ installed    2026-02-10 10:32:15
python      ⏳ installing   -
docker      ✗ failed       2026-02-10 10:35:42
```

**References:**

- [CLI Reference: Extension Commands](v3/docs/CLI.md#extension)
- [Extensions Overview](v3/docs/EXTENSIONS.md)

---

### 3. Installation Reliability & Error Reporting

**Benefit:** Precise failure diagnostics with phase-by-phase tracking

V3 dramatically improves installation reliability and error reporting with detailed phase tracking and enhanced state management.

**Enhanced Error Reporting:**

- **Phase Tracking**: Shows exactly where installations fail
  - Phases: source resolution → download → install → validate
- **Installation States**: New `installing` and `failed` states visible in status output
- **Script Portability**: Removed `DOCKER_LIB` dependency for better cross-environment support
- **Temp File Handling**: Improved to prevent cross-filesystem move issues
- **SDKMAN Integration**: More robust initialization handling for edge cases

**Usage:**

```bash
# Check for failed installations
sindri extension status | grep failed

# Retry failed installation with verbose output
sindri extension install docker --verbose

# Tail installation logs
tail -f ~/.sindri/logs/install.log
```

**Error Message Example:**

```
Error: Extension installation failed
  Extension: docker
  Phase: install
  Reason: SDKMAN initialization returned non-zero exit code
  Suggestion: Run 'sindri doctor --check-sdkman' to diagnose SDKMAN setup
```

**References:**

- [Troubleshooting Guide](v3/docs/TROUBLESHOOTING.md#extension-install-fails)

---

### 4. Language Runtime Improvements

**Benefit:** Simplified installation and enhanced compatibility

V3 improves language runtime management with simplified installation methods and better dependency handling.

**Node.js Migration:**

- **Simplified Installation**: Migrated from hybrid (mise+script) to mise-only
- **Removed Dependencies**: No more `bootstrap-pnpm.sh` script
- **Corepack Integration**: Corepack manages pnpm automatically
- **Enhanced PATH Support**: Better handling for bundled/downloaded/flat layouts

**Python Enhancements:**

- **Dependency Management**: Added Python where needed for native module compilation (node-gyp)
- **Mise Backend**: Pre-built binaries used to avoid compilation in restricted environments

**SDKMAN Improvements:**

- **Validation Robustness**: Improved across deployment platforms
- **Initialization Handling**: Better error handling for non-zero exit codes

**Pulumi Fixes:**

- **Version Handling**: Fixed version prefix handling issues

**Usage:**

```bash
# Install Node.js with automatic pnpm setup
sindri extension install nodejs

# Python automatically included for Node.js native modules
sindri extension info nodejs  # Shows python as dependency

# Verify SDKMAN installation
sindri doctor --check-sdkman
```

**References:**

- [Extension Authoring Guide: Install Methods](v3/docs/extensions/guides/AUTHORING.md#install-required)

---

### 5. New Extensions

**Shannon v1.0.0** - Autonomous AI Pentester

- **Category**: Testing
- **Features**: White-box source analysis + black-box exploitation
- **Success Rate**: 96.15% on XBOW Benchmark
- **Use Case**: Enterprise security testing
- **Installation**: `sindri extension install shannon`

Shannon is an autonomous AI penetration tester that combines white-box source code analysis with black-box exploitation techniques for comprehensive security testing.

**References:**

- [Extension Registry: Shannon](v3/extensions/shannon/extension.yaml)

---

### 6. Native Binary Distribution

**Benefit:** Zero-dependency installation on 5 platforms

Sindri V3 is distributed as a single statically-linked binary (~12MB) that requires zero runtime dependencies. No more cloning the repository, no more bash/yq/jq/jsonschema prerequisites.

**Supported Platforms:**

| Platform       | Binary                        | Status          |
| -------------- | ----------------------------- | --------------- |
| Linux x86_64   | `sindri-linux-x86_64.tar.gz`  | ✅ Stable       |
| Linux aarch64  | `sindri-linux-aarch64.tar.gz` | ✅ Stable       |
| macOS aarch64  | `sindri-macos-aarch64.tar.gz` | ✅ Stable       |
| Windows x86_64 | `sindri-windows-x86_64.zip`   | 🧪 Experimental |

**Installation:**

```bash
# Linux x86_64
wget https://github.com/pacphi/sindri/releases/latest/download/sindri-linux-x86_64.tar.gz
tar -xzf sindri-linux-x86_64.tar.gz
sudo mv sindri /usr/local/bin/

# macOS Apple Silicon
wget https://github.com/pacphi/sindri/releases/latest/download/sindri-macos-aarch64.tar.gz
tar -xzf sindri-macos-aarch64.tar.gz
sudo mv sindri /usr/local/bin/

# Docker image
docker pull ghcr.io/pacphi/sindri:v3

# Verify installation
sindri version
```

**Build from Source:**

```bash
cd v3 && cargo build --release
sudo cp target/release/sindri /usr/local/bin/
```

---

### 2. Self-Update (`sindri upgrade`)

**Benefit:** Automated CLI updates with compatibility checks and rollback

V3 includes a built-in self-update mechanism that checks extension compatibility before upgrading to prevent breaking changes.

```bash
# Check for available updates
sindri upgrade --check

# List all available versions
sindri upgrade --list

# Check extension compatibility before upgrading
sindri upgrade --compat 3.1.0

# Upgrade to latest stable version
sindri upgrade

# Upgrade to specific version
sindri upgrade --version 3.1.0 -y

# Include pre-releases
sindri upgrade --prerelease

# Downgrade to previous version
sindri upgrade --version 3.0.0 --allow-downgrade
```

**Compatibility Matrix:**

Each CLI version defines compatible extension schema versions. The upgrade command checks these before proceeding:

```yaml
cli_versions:
  "3.0.x":
    extension_schema: "1.0"
    breaking_changes:
      - "CLI rewritten in Rust"
      - "Extension schema updated to v1.0"
    migration_notes:
      - "Auto-migration with backup"
```

**References:**

- [ADR-022: Self-Update Implementation](v3/docs/architecture/adr/022-phase-6-self-update-implementation.md)

---

### 3. System Diagnostics (`sindri doctor`)

**Benefit:** Comprehensive health checks with auto-fix capabilities

The `doctor` command checks your system for required tools, validates configurations, and can automatically install missing dependencies.

```bash
# Basic system check
sindri doctor

# Check all tools regardless of current usage
sindri doctor --all

# Check for specific provider
sindri doctor --provider k8s

# Auto-fix missing tools
sindri doctor --fix

# Dry run to preview fixes
sindri doctor --fix --dry-run

# CI mode with JSON output (non-zero exit on missing tools)
sindri doctor --ci --format json

# Check extension tool requirements
sindri doctor --check-extensions --extension mise
```

**Health Checks Include:**

- Docker installation and version
- Provider-specific tools (flyctl, devpod, kubectl)
- Kubernetes tools (kind, k3d)
- Image verification tools (cosign)
- Extension-specific dependencies
- Authentication status (with `--check-auth`)

**References:**

- [Doctor Guide](v3/docs/DOCTOR.md) - Comprehensive diagnostics documentation

---

### 4. Local Kubernetes (`sindri k8s`)

**Benefit:** Local Kubernetes testing with kind and k3d without cloud resources

V3 introduces built-in local Kubernetes cluster management for testing deployments before pushing to production clusters.

```bash
# Create a kind cluster (default)
sindri k8s create

# Create a k3d cluster with local registry
sindri k8s create --provider k3d --registry

# Multi-node cluster
sindri k8s create --nodes 3 --name dev-cluster

# Specific Kubernetes version
sindri k8s create --k8s-version v1.34.0

# List all clusters
sindri k8s list

# Show cluster status
sindri k8s status --name dev-cluster

# Get kubeconfig
sindri k8s config --name dev-cluster > ~/.kube/dev.yaml

# Destroy cluster
sindri k8s destroy --name dev-cluster --force

# Install cluster tools
sindri k8s install kind
sindri k8s install k3d
```

**Configuration:**

```yaml
# sindri.yaml Kubernetes provider example
provider: kubernetes
kubernetes:
  clusterProvider: kind
  clusterName: sindri-local
  nodes: 1
  k8sVersion: v1.35.0
```

**References:**

- [ADR-029: Local Kubernetes Cluster Management](v3/docs/architecture/adr/029-local-kubernetes-cluster-management.md)
- [Kubernetes Guide](v3/docs/K8S.md)

---

### 5. VM Image Building (`sindri vm`)

**Benefit:** Golden VM images on 5 cloud providers with Packer

V3 introduces VM image building with HashiCorp Packer, enabling pre-configured golden images across AWS, Azure, GCP, OCI, and Alibaba Cloud.

```bash
# Build AWS AMI
sindri vm build --cloud aws

# Build Azure image with custom profile
sindri vm build --cloud azure --profile python-data-science

# Build GCP image with CIS security hardening
sindri vm build --cloud gcp --cis-hardening --disk-size 100

# Dry run to preview Packer template
sindri vm build --cloud aws --dry-run

# Check prerequisites for a cloud provider
sindri vm doctor --cloud aws

# List existing VM images
sindri vm list --cloud aws --region us-east-1

# Deploy a VM from an image
sindri vm deploy --cloud aws ami-0123456789abcdef0

# Generate customizable Packer template
sindri vm init --cloud aws --output ./packer/
```

**Supported Providers:**

| Provider | Default Instance Type | Default Region | Image Output  |
| -------- | --------------------- | -------------- | ------------- |
| AWS      | t3.large              | us-west-2      | AMI           |
| Azure    | Standard_D2s_v3       | eastus         | Managed Image |
| GCP      | e2-standard-2         | us-central1-a  | Image         |
| OCI      | VM.Standard.E4.Flex   | -              | Custom Image  |
| Alibaba  | ecs.g6.xlarge         | cn-hangzhou    | Custom Image  |

**References:**

- [ADR-031: Packer VM Provisioning Architecture](v3/docs/architecture/adr/031-packer-vm-provisioning-architecture.md)
- [VM Provider Overview](v3/docs/providers/VM.md)
- [VM Distribution Strategy](v3/docs/providers/vm/DISTRIBUTION.md)
- [VM Security Guide](v3/docs/providers/vm/SECURITY.md)
- Cloud-specific guides: [AWS](v3/docs/providers/vm/AWS.md), [Azure](v3/docs/providers/vm/AZURE.md), [GCP](v3/docs/providers/vm/GCP.md), [OCI](v3/docs/providers/vm/OCI.md), [Alibaba](v3/docs/providers/vm/ALIBABA.md)

---

### 6. Container Image Management (`sindri image`)

**Benefit:** Secure container image handling with signature verification and SBOM generation

V3 adds comprehensive container image management with cosign signature verification, SBOM (Software Bill of Materials) generation, and SLSA Level 3 provenance tracking.

```bash
# List available images from registry
sindri image list

# Filter to V3 images only
sindri image list --filter "^v3\\."

# Inspect image details and SBOM
sindri image inspect ghcr.io/pacphi/sindri:v3.0.0 --sbom

# Verify image signature and provenance
sindri image verify ghcr.io/pacphi/sindri:v3.0.0

# Show version compatibility matrix
sindri image versions

# Show currently deployed image
sindri image current

# Deploy with image verification (default)
sindri deploy

# Skip verification for local development
sindri deploy --skip-image-verification
```

**Security Features:**

| Feature            | Standard      | Description                          |
| ------------------ | ------------- | ------------------------------------ |
| Image signing      | Cosign (OIDC) | Keyless signing via Sigstore         |
| SBOM generation    | SPDX          | Software bill of materials           |
| Provenance         | SLSA Level 3  | Supply chain integrity verification  |
| Vulnerability scan | Trivy         | CVE detection + cargo-audit for Rust |

**References:**

- [ADR-014: SBOM Generation Industry Standards](v3/docs/architecture/adr/014-sbom-generation-industry-standards.md)
- [Image Management Guide](v3/docs/IMAGE_MANAGEMENT.md)

---

### 7. S3 Encrypted Secrets

**Benefit:** Cloud-native secrets with age encryption for team collaboration

V3 adds an S3-compatible encrypted secrets backend using ChaCha20-Poly1305 encryption via the `age` library, enabling team-wide secret sharing through S3/MinIO.

```bash
# Generate master encryption key
sindri secrets s3 keygen

# Initialize S3 backend
sindri secrets s3 init --bucket my-secrets --region us-east-1 --create-bucket

# Push secrets to S3
sindri secrets s3 push DATABASE_URL --value "postgres://..."
sindri secrets s3 push SSH_KEY --from-file ~/.ssh/id_rsa

# Pull secrets from S3
sindri secrets s3 pull DATABASE_URL --show
sindri secrets s3 pull SSH_KEY --output ./key.pem

# Sync all secrets bidirectionally
sindri secrets s3 sync --direction both

# Rotate master encryption key
sindri secrets s3 rotate --new-key ./new-master.key
```

**Configuration:**

```yaml
# sindri.yaml S3 secrets backend
secrets:
  provider:
    s3:
      bucket: my-team-secrets
      region: us-east-1
      keyFile: .sindri-master.key
      prefix: sindri/secrets/
```

**References:**

- [ADR-020: S3 Encrypted Secret Storage](v3/docs/architecture/adr/020-s3-encrypted-secret-storage.md)
- [Secrets Management Guide](v3/docs/SECRETS_MANAGEMENT.md)

---

### 8. Project Scaffolding Templates

**Benefit:** Rapid project creation with 20+ project types and custom template support

V3 introduces a template-based project scaffolding system that generates new projects with best-practice structure, Git initialization, and optional agentic tool setup.

```bash
# Create project (auto-detects type from name)
sindri project new my-api

# Specify project type
sindri project new my-lib --project-type rust-lib

# Interactive mode
sindri project new my-project -i

# Clone with enhancements
sindri project clone https://github.com/user/repo

# Fork and clone with feature branch
sindri project clone https://github.com/user/repo --fork --feature add-auth
```

**Available Project Types:**

| Language   | Types                                                 |
| ---------- | ----------------------------------------------------- |
| Rust       | `rust`, `rust-lib`, `rust-cli`, `rust-workspace`      |
| Python     | `python`, `python-package`, `python-api`, `python-ml` |
| TypeScript | `typescript`, `typescript-lib`, `typescript-api`      |
| Go         | `go`, `go-cli`, `go-api`                              |
| Java       | `java`, `java-api`                                    |
| Elixir     | `elixir`, `elixir-api`                                |
| Zig        | `zig`, `zig-lib`                                      |
| Generic    | `generic`                                             |

**References:**

- [ADR-024: Template-Based Project Scaffolding](v3/docs/architecture/adr/024-template-based-project-scaffolding.md)
- [Projects Guide](v3/docs/PROJECTS.md)

---

### 9. Conditional Templates

**Benefit:** Environment-aware extension configuration for CI/local divergence

V3.1 introduces declarative template selection based on environment context, replacing imperative bash scripts with structured conditions.

**Condition Types:**

| Type        | Description                         | Example                       |
| ----------- | ----------------------------------- | ----------------------------- |
| Environment | Match environment variables         | `env: { CI: "true" }`         |
| Platform    | Match OS and architecture           | `platform: { os: ["linux"] }` |
| Logical     | Combine conditions with any/all/not | `any: [{ CI: "true" }, ...]`  |
| Regex       | Pattern matching                    | `{ matches: "^/home/.*$" }`   |

**Example: CI vs Local Template Selection:**

```yaml
configure:
  templates:
    # Local environment gets full config
    - source: config.yml.example
      destination: ~/config/app.yml
      condition:
        env:
          not_any:
            - CI: "true"
            - GITHUB_ACTIONS: "true"

    # CI environment gets minimal config
    - source: config.ci.yml.example
      destination: ~/config/app.yml
      condition:
        env:
          any:
            - CI: "true"
            - GITHUB_ACTIONS: "true"
```

**Logical Operators:**

| Operator  | Description                        |
| --------- | ---------------------------------- |
| `any`     | OR - true if any condition matches |
| `all`     | AND - true if all conditions match |
| `not`     | NOT - inverts the condition result |
| `not_any` | NOR - true if no condition matches |
| `not_all` | NAND - true if any condition fails |

**References:**

- [ADR-033: Environment-Based Template Selection](v3/docs/architecture/adr/033-environment-based-template-selection.md)
- [Conditional Templates Guide](v3/docs/extensions/guides/CONDITIONAL_TEMPLATES_MIGRATION.md)

---

## 🚀 Performance Improvements

### 1. CLI Performance

The Rust rewrite delivers dramatic performance improvements across all CLI operations:

| Operation         | V2 (Bash) | V3 (Rust) | Improvement |
| ----------------- | :-------: | :-------: | :---------: |
| CLI startup       |   2-5s    |  <100ms   | **20-50x**  |
| Config parsing    | 100-500ms |  10-50ms  | **10-20x**  |
| Schema validation | 100-500ms |  10-50ms  | **10-20x**  |
| Extension install |   ~30s    |   ~15s    |   **2x**    |

**Why It's Faster:**

- **Native YAML/JSON:** serde_yaml_ng and serde_json replace yq/jq subprocess calls
- **Compiled binary:** No interpreter startup overhead
- **Async runtime:** Tokio enables parallel I/O operations
- **Native schema validation:** jsonschema crate replaces python3-jsonschema

### 2. Docker Build Performance

| Metric            |      V2       |      V3      |   Improvement   |
| ----------------- | :-----------: | :----------: | :-------------: |
| Docker build time |   15-20 min   |   5-8 min    | **2-3x faster** |
| Docker image size |    ~2.5GB     |    ~800MB    | **68% smaller** |
| Binary size       | ~50KB scripts | ~12MB binary |    Trade-off    |

**What Changed:**

- Multi-stage builds optimized for layer caching
- Separate production and development Dockerfiles
- Pre-compiled binary eliminates Rust compilation in production builds
- Removed unnecessary build dependencies from final image

### 3. Extension Installation

| Improvement               | Details                                       |
| ------------------------- | --------------------------------------------- |
| Parallel DAG resolution   | Extensions resolved concurrently via petgraph |
| Async downloads           | Multiple extensions downloaded in parallel    |
| Content-addressable cache | pnpm store deduplicates npm packages          |
| Dependency optimization   | Topological sort minimizes install rounds     |

**Result:**

- Extension profile installation ~2x faster than V2
- 100% success rate (V2 had occasional timeout failures)
- Better error messages with precise failure locations

**References:**

- [ADR-009: Dependency Resolution DAG Topological Sort](v3/docs/architecture/adr/009-dependency-resolution-dag-topological-sort.md)
- [Comparison Guide: Performance Benchmarks](docs/migration/COMPARISON_GUIDE.md#performance-benchmarks)

---

## 📚 Documentation Improvements

### New Documentation

V3 introduces a comprehensive documentation suite:

**Core Documentation (17 docs):**

| Document                                                  | Description                                        |
| --------------------------------------------------------- | -------------------------------------------------- |
| [CLI Reference](v3/docs/CLI.md)                           | Complete command-line reference (140+ subcommands) |
| [Configuration](v3/docs/CONFIGURATION.md)                 | Full sindri.yaml specification                     |
| [Runtime Configuration](v3/docs/RUNTIME_CONFIGURATION.md) | CLI settings and overrides                         |
| [Quickstart Guide](v3/docs/QUICKSTART.md)                 | Zero to deployed in 10 minutes                     |
| [Getting Started](v3/docs/GETTING_STARTED.md)             | Detailed setup instructions                        |
| [Architecture](v3/docs/ARCHITECTURE.md)                   | High-level system design                           |
| [Extensions](v3/docs/EXTENSIONS.md)                       | Extension system architecture                      |
| [Secrets Management](v3/docs/SECRETS_MANAGEMENT.md)       | Multi-backend secrets guide                        |
| [Backup & Restore](v3/docs/BACKUP_RESTORE.md)             | Workspace backup strategies                        |
| [Projects](v3/docs/PROJECTS.md)                           | Project scaffolding guide                          |
| [Image Management](v3/docs/IMAGE_MANAGEMENT.md)           | Container image security                           |
| [Deployment](v3/docs/DEPLOYMENT.md)                       | Deployment modes and Dockerfile guide              |
| [Doctor](v3/docs/DOCTOR.md)                               | System diagnostics guide                           |
| [Kubernetes](v3/docs/K8S.md)                              | Advanced Kubernetes configuration                  |
| [Schema Reference](v3/docs/SCHEMA.md)                     | YAML schema documentation                          |
| [Multi-Architecture](v3/docs/MULTI_ARCH_SUPPORT.md)       | Multi-platform image builds                        |
| [Troubleshooting](v3/docs/TROUBLESHOOTING.md)             | Common issues and solutions                        |

**Provider Documentation (14 docs):**

| Document                                                | Description                      |
| ------------------------------------------------------- | -------------------------------- |
| [Providers Overview](v3/docs/providers/README.md)       | All deployment providers         |
| [Docker](v3/docs/providers/DOCKER.md)                   | Docker Compose local development |
| [Fly.io](v3/docs/providers/FLY.md)                      | Fly.io cloud deployment          |
| [E2B](v3/docs/providers/E2B.md)                         | E2B cloud sandboxes              |
| [Kubernetes](v3/docs/providers/KUBERNETES.md)           | Kubernetes cluster deployment    |
| [DevPod](v3/docs/providers/DEVPOD.md)                   | DevPod multi-cloud support       |
| [VM Providers](v3/docs/providers/VM.md)                 | Virtual machine overview         |
| [VM Distribution](v3/docs/providers/vm/DISTRIBUTION.md) | VM image distribution            |
| [VM Security](v3/docs/providers/vm/SECURITY.md)         | VM security hardening            |
| [AWS](v3/docs/providers/vm/AWS.md)                      | Amazon EC2 deployment            |
| [Azure](v3/docs/providers/vm/AZURE.md)                  | Microsoft Azure VM               |
| [GCP](v3/docs/providers/vm/GCP.md)                      | Google Compute Engine            |
| [OCI](v3/docs/providers/vm/OCI.md)                      | Oracle Cloud Infrastructure      |
| [Alibaba](v3/docs/providers/vm/ALIBABA.md)              | Alibaba Cloud ECS                |

**Extension Guides (8 docs):**

| Document                                                                              | Description                   |
| ------------------------------------------------------------------------------------- | ----------------------------- |
| [Extension Guides Index](v3/docs/extensions/guides/README.md)                         | Index of all guides           |
| [Authoring Guide](v3/docs/extensions/guides/AUTHORING.md)                             | Creating extensions           |
| [Sourcing Modes](v3/docs/extensions/guides/SOURCING_MODES.md)                         | Extension sourcing strategies |
| [Support File Integration](v3/docs/extensions/guides/SUPPORT_FILE_INTEGRATION.md)     | Support file patterns         |
| [Support File Versioning](v3/docs/extensions/guides/SUPPORT_FILE_VERSION_HANDLING.md) | Version-aware management      |
| [Support Files CLI](v3/docs/extensions/guides/SUPPORT_FILES_CLI_COMMAND.md)           | Support file commands         |
| [Conditional Templates](v3/docs/extensions/guides/CONDITIONAL_TEMPLATES_MIGRATION.md) | Template selection            |
| [Maintainer Guide](v3/docs/MAINTAINER_GUIDE.md)                                       | Release process               |

**Architecture Decision Records (40+ ADRs):**

Covering Rust workspace architecture, provider abstraction, extension type system, dependency resolution, secrets management, backup/restore, CI/CD, self-update, project management, Kubernetes, Packer VM provisioning, conditional templates, Dockerfile architecture, and more. See [ADR Index](v3/docs/architecture/adr/README.md).

### Updated Documentation

- [Migration Guide](docs/migration/MIGRATION_GUIDE.md) - Step-by-step V2 → V3 migration
- [Comparison Guide](docs/migration/COMPARISON_GUIDE.md) - Comprehensive V2 vs V3 comparison
- [FAQ](docs/FAQ.md) - 60+ questions covering V2 and V3
- [Contributing Guide](docs/CONTRIBUTING.md) - Updated for V3 development workflow

---

## 🔧 Migration Checklists

### For Extension Authors

**If you maintain custom Sindri extensions:**

- [ ] Review [ADR-008](v3/docs/architecture/adr/008-extension-type-system-yaml-deserialization.md) to understand V3 extension type system
- [ ] Update install method `manual` → `script` in extension.yaml
- [ ] Map V2 categories to V3 categories:

  | V2 Category    | V3 Category        |
  | -------------- | ------------------ |
  | base           | package-manager    |
  | agile          | productivity       |
  | language       | languages          |
  | dev-tools      | devops             |
  | infrastructure | cloud              |
  | ai             | ai-agents / ai-dev |
  | utilities      | productivity       |
  | desktop        | desktop            |
  | monitoring     | devops             |
  | database       | devops             |
  | mobile         | languages          |

- [ ] Validate extension against V3 schema:
  ```bash
  sindri extension validate <extension-name> --file ./extension.yaml
  ```
- [ ] Test extension installation in V3 environment:
  ```bash
  sindri extension install <extension-name>
  sindri extension status <extension-name>
  ```
- [ ] If using capabilities (project-init, auth, hooks, mcp, collision-handling):
  - [ ] Verify capabilities still work with V3 runtime
  - [ ] Test collision handling scenarios
  - [ ] Validate project-init commands execute correctly
- [ ] If using conditional templates (V3.1 feature):
  - [ ] Add `condition` blocks to template definitions
  - [ ] Test with both CI and local environments
- [ ] Update extension documentation with V3-specific notes
- [ ] Bump extension version for V3 compatibility

### For End Users

**If you use Sindri for development:**

- [ ] Create backup of current environment:
  ```bash
  ./v2/cli/sindri backup --profile full --output v2-backup-$(date +%Y%m%d).tar.gz
  cp sindri.yaml sindri.yaml.v2.backup
  ```
- [ ] Download and install V3 binary for your platform:
  ```bash
  wget https://github.com/pacphi/sindri/releases/latest/download/sindri-linux-x86_64.tar.gz
  tar -xzf sindri-linux-x86_64.tar.gz
  sudo mv sindri /usr/local/bin/
  sindri version
  ```
- [ ] Run system diagnostics:
  ```bash
  sindri doctor --all
  sindri doctor --fix  # Auto-install missing tools
  ```
- [ ] Validate and auto-migrate configuration:
  ```bash
  sindri config validate
  # Creates sindri.yaml.v2.backup automatically
  ```
- [ ] Check for removed extensions:
  - [ ] `claude-flow` (v1) → use `claude-flow-v2` or `claude-flow-v3`
  - [ ] `claude-auth-with-api-key` → remove (auth now built-in)
  - [ ] `ruvnet-aliases` → remove (consolidated)
  - [ ] Any `vf-*` extensions → continue using V2 for VisionFlow
- [ ] Install extensions and validate:
  ```bash
  sindri extension install --from-config sindri.yaml
  sindri extension validate-all
  ```
- [ ] Test deployment cycle:
  ```bash
  sindri deploy --dry-run        # Preview changes
  sindri deploy                  # Deploy
  sindri connect                 # Verify access
  sindri destroy                 # Clean up test
  ```
- [ ] Update scripts and aliases to use V3 command syntax
- [ ] Set up self-update:
  ```bash
  sindri upgrade --check         # Check for future updates
  ```

### For DevOps/Platform Teams

**If you manage Sindri deployments at scale:**

- [ ] Review all [breaking changes](#-breaking-changes) in this document
- [ ] Audit current deployments for removed extensions:
  ```bash
  # Check all instances for removed extensions
  for instance in $(list-sindri-instances); do
    ssh $instance "extension-manager list" | grep -E "claude-flow$|claude-auth|ruvnet-aliases|vf-"
  done
  ```
- [ ] Update Docker build pipelines:

  ```yaml
  # Before (V2)
  - run: docker build -f v3/Dockerfile --build-arg BUILD_FROM_SOURCE=true -t sindri .

  # After (V3)
  - run: docker build -f v3/Dockerfile -t sindri:prod . # Production
  # OR
  - run: docker build -f v3/Dockerfile.dev -t sindri:dev . # Development
  ```

- [ ] Update CI/CD pipelines to use V3 binary:

  ```yaml
  # Before (V2)
  - run: ./v2/cli/sindri deploy --provider docker

  # After (V3)
  - name: Install Sindri V3
    run: |
      wget https://github.com/pacphi/sindri/releases/latest/download/sindri-linux-x86_64.tar.gz
      tar -xzf sindri-linux-x86_64.tar.gz && sudo mv sindri /usr/local/bin/
  - run: sindri deploy
  ```

- [ ] Update environment variables:
  - [ ] Remove `SINDRI_BUILD_FROM_SOURCE` references
  - [ ] Remove `SINDRI_EXTENSIONS_SOURCE` references
  - [ ] Add `SINDRI_EXT_HOME` where needed
  - [ ] Remove `BUILD_FROM_SOURCE` Docker build args
- [ ] Plan migration timeline:
  - [ ] Stage 1: Deploy V3 to test/staging environments
  - [ ] Stage 2: Test deployment, extension, and project workflows
  - [ ] Stage 3: Migrate production instances
  - [ ] Stage 4: Decommission V2 deployments
- [ ] Prepare rollback plan:
  ```bash
  # Quick rollback procedure
  sudo rm /usr/local/bin/sindri
  cp sindri.yaml.v2.backup sindri.yaml
  git checkout v2.5.0
  ./v2/cli/sindri deploy --provider <provider>
  ```
- [ ] Update monitoring for V3-specific metrics
- [ ] Communicate breaking changes to team and provide migration timeline

### For Extension Ecosystem Maintainers

**If you maintain a registry of Sindri extensions:**

- [ ] Review V3 extension schema changes:
  - [ ] `manual` install method removed → use `script`
  - [ ] Category taxonomy updated (11 → 13 categories)
  - [ ] Stricter validation rules
- [ ] Update registry schema to support V3 categories:
  ```bash
  # Validate all extensions against V3 schema
  sindri extension validate-all
  ```
- [ ] Audit extensions for V3 compatibility:
  - [ ] Which extensions use `manual` install method? → Update to `script`
  - [ ] Which extensions use deprecated category names? → Update mapping
  - [ ] Which extensions reference VisionFlow components? → V2 only
- [ ] Test extension discovery and installation in V3 environment
- [ ] Update extension documentation templates for V3 format
- [ ] Verify conflict detection works correctly with V3 schema
- [ ] Create migration examples for common extension patterns
- [ ] Update extension CI/CD to validate against both V2 and V3 schemas

---

## 🐛 Bug Fixes

### Recent Fixes

1. **Extension Schema Validation** (commits: 8b6434b, 2bcc072)
   - Fixed null handling for `docs.notes` field (affected 36/50 extensions)
   - Extension schema now properly validates nullable fields
   - Impact: All extensions now validate correctly against V3 schema

2. **Extension Python Validation** (commit: 8b6434b)
   - Removed validation for pnpm and SDKMAN executables (now managed by corepack and bashrc)
   - Simplified extension validation logic
   - Impact: Reduced false-positive validation failures

3. **Logging ANSI Code Pollution** (commit: 2bcc072)
   - Strip ANSI codes from mise and script installation output
   - Prevents log pollution with terminal escape sequences
   - Impact: Cleaner logs in CI/CD environments

4. **Docker Package Dependencies** (commit: 2bcc072)
   - Added gnupg and iproute2 packages for better compatibility
   - Impact: Improved compatibility across different deployment environments

5. **Extension Version Handling** (commit: 2bcc072)
   - Improved version parsing and validation for extensions
   - Impact: More robust version compatibility checks

### Critical Fixes

1. **Python compilation fails with noexec /tmp** (alpha.13, commit: 96211bb)
   - Root cause: Python's `mise` backend attempts compilation in `/tmp`, which has `noexec` mount on security-hardened containers
   - Fix: Disabled Python compilation in mise, using pre-built binaries
   - Impact: Python extension now installs on OrbStack and security-hardened Docker environments

2. **Fly.io deployment hangs indefinitely** (alpha.17, commit: c396487)
   - Root cause: Fly provider missing context variables before early return for pre-built images
   - Fix: Added fly context variables before early return, improved deployment visibility
   - Additional fix: Fly provider now properly uses pre-built images (alpha.16, commit: 24f93d3)
   - Impact: Fly.io deployments complete reliably with progress output

3. **Extension path doubling** (alpha.9, commit: b6bfa24)
   - Root cause: Extension directory lookup appended path component twice
   - Fix: Resolved doubled path in extension directory lookup
   - Impact: Extension operations now resolve correct file paths

4. **SSH private keys and GitHub CLI CVE vulnerabilities** (alpha.1, commit: 439cfbc)
   - Fix: Security patches applied to SSH key handling and GitHub CLI
   - Impact: Addresses CVE vulnerabilities in dependency chain

### Minor Fixes

5. **Health check netstat → ss** (alpha.15, commit: 7d7e6d6)
   - Root cause: `netstat` not available in minimal Docker images, causing Fly.io health check failures
   - Fix: Replaced `netstat` with `ss` in Docker health check
   - Impact: Health checks now work on all deployment targets

6. **Docker home path incorrect** (alpha.18, commit: df80737)
   - Fix: Set developer user home to `/alt/home/developer` for volume mount compatibility
   - Impact: Correct home directory resolution in containerized environments

7. **Deployment progress not visible** (alpha.18, commit: c1b40d8)
   - Fix: Show deployment progress by default
   - Impact: Users can now see real-time deployment status

8. **Docker health check improvements** (alpha.14, commit: 4e5aaf5)
   - Fix: Improved Docker health check and standardized image defaults
   - Impact: More reliable container health reporting

9. **Compatibility matrix URL incorrect** (alpha.6, commit: a496d79)
   - Fix: Corrected compatibility matrix URL and bundled it in Docker image
   - Impact: `sindri upgrade --compat` works correctly

10. **ARM64 and Windows build failures** (alpha.2, commit: fe25b4f)
    - Fix: Resolved V3 build failures for ARM64 and Windows targets
    - Impact: Cross-platform binary releases work correctly

11. **Extension list -c argument conflict** (alpha.7, commit: 4603305)
    - Fix: Resolved `-c` argument conflict in extension list command
    - Impact: `sindri extension list -c <category>` works correctly

12. **Clippy format-in-format-args warning** (alpha.14, commit: ca9349c)
    - Fix: Resolved clippy warning for cleaner builds
    - Impact: No user-facing change; code quality improvement

### Enhancement Fixes

13. **Unified extension source resolver** (alpha.5, commit: 3abcbe0)
    - Added unified extension source resolver with improved GHCR authentication
    - Impact: More reliable extension downloads from multiple sources

14. **GitHub raw content downloads** (alpha.7, commit: 3295c2d)
    - Migrated extension downloads to `raw.githubusercontent.com`
    - Impact: Faster, more reliable extension downloads without API rate limits

15. **Support file versioning** (implemented across multiple alphas)
    - Version-aware support file management for extensions
    - Impact: Extensions can ship version-specific configuration files

16. **Downloaded mode extension support** (alpha.11-12, commits: ababb26, 73a7972)
    - Added `common.sh` and mise config file downloads for downloaded mode extensions
    - Impact: Extensions work correctly in both bundled and downloaded modes

17. **macOS x86_64 support removed from release pipeline** (alpha.4, commit: e27466c)
    - Removed macOS x86_64 from automated release pipeline (legacy hardware declining)
    - Impact: macOS Intel builds available via source compilation only

---

## 🔍 Known Issues

### Limitations

1. **VisionFlow extensions excluded from V3**
   - All 33 `vf-*` extensions require GPU, desktop environment, and complex Docker integration not available in V3
   - Workaround: Continue using V2 for VisionFlow workflows; V2 and V3 can coexist

2. **macOS Intel (x86_64) support is legacy**
   - Pre-built binaries no longer in automated release pipeline (alpha.4)
   - Workaround: Build from source with `cd v3 && cargo build --release`

3. **APT extensions restricted in socket DinD mode**
   - `no-new-privileges` security constraint blocks `sudo` in socket DinD mode
   - Workaround: Use mise, pip, npm, or binary installation methods; or use sysbox/privileged DinD mode
   - Reference: [Comparison Guide: Security Constraints](docs/migration/COMPARISON_GUIDE.md#security-constraints-v3)

4. **Windows support is experimental**
   - `sindri-windows-x86_64.zip` builds but has limited testing
   - Workaround: Use WSL2 + Docker on Windows for production use

5. **First CLI run may be slower**
   - Initial schema compilation and caching adds latency on first command
   - Subsequent runs are at full speed (<100ms startup)

### Planned for Future Releases

- **v3.1:** Conditional templates (ADR-033) - environment-aware extension configuration
- **v3.2:** Enhanced extension marketplace with browsing and ratings
- **v3.3:** Interactive collision resolution with user prompts
- **v3.x:** Capability testing framework for automated capability validation
- **v3.x:** Multi-extension project-init ordering with dependency-aware initialization
- **v3.x:** Extension hot-reload for development workflows

---

## 📦 Installation & Upgrade

### Fresh Installation (V3)

**Option 1: Pre-built binary (recommended)**

```bash
# Linux x86_64
wget https://github.com/pacphi/sindri/releases/latest/download/sindri-linux-x86_64.tar.gz
tar -xzf sindri-linux-x86_64.tar.gz
sudo mv sindri /usr/local/bin/

# Linux aarch64
wget https://github.com/pacphi/sindri/releases/latest/download/sindri-linux-aarch64.tar.gz
tar -xzf sindri-linux-aarch64.tar.gz
sudo mv sindri /usr/local/bin/

# macOS Apple Silicon
wget https://github.com/pacphi/sindri/releases/latest/download/sindri-macos-aarch64.tar.gz
tar -xzf sindri-macos-aarch64.tar.gz
sudo mv sindri /usr/local/bin/

# Verify
sindri version
sindri doctor --all
```

**Option 2: Docker image**

```bash
docker pull ghcr.io/pacphi/sindri:v3

docker run -d --name sindri \
  -e SINDRI_PROFILE=minimal \
  -v sindri_home:/alt/home/developer \
  ghcr.io/pacphi/sindri:v3
```

**Option 3: Build from source**

```bash
git clone https://github.com/pacphi/sindri.git
cd sindri/v3
cargo build --release
sudo cp target/release/sindri /usr/local/bin/
```

### Upgrade from V2

**Option 1: Clean deployment (recommended for major version)**

```bash
# 1. Backup existing data
./v2/cli/sindri backup --profile full --output v2-backup-$(date +%Y%m%d).tar.gz
cp sindri.yaml sindri.yaml.v2.backup

# 2. Destroy V2 deployment
./v2/cli/sindri destroy --provider <your-provider>

# 3. Install V3 binary
wget https://github.com/pacphi/sindri/releases/latest/download/sindri-linux-x86_64.tar.gz
tar -xzf sindri-linux-x86_64.tar.gz
sudo mv sindri /usr/local/bin/

# 4. Validate and auto-migrate configuration
sindri config validate

# 5. Deploy fresh V3 instance
sindri deploy

# 6. Verify extensions
sindri extension list --installed
sindri extension validate-all

# 7. Restore data if needed
sindri restore v2-backup-*.tar.gz --mode merge
```

**Option 2: Side-by-side deployment (zero downtime)**

```bash
# 1. Install V3 binary (coexists with V2 scripts)
sudo mv sindri /usr/local/bin/

# 2. Deploy V3 to separate instance
sindri config init --name sindri-v3 --provider fly --profile fullstack
sindri deploy

# 3. Test V3 thoroughly
sindri connect
# Run tests...

# 4. Switch traffic to V3
# Update DNS/load balancer

# 5. Decommission V2
./v2/cli/sindri destroy --provider <provider>
```

**Option 3: In-place upgrade (advanced users)**

```bash
# 1. Install V3 binary
sudo mv sindri /usr/local/bin/

# 2. Auto-migrate configuration
sindri config validate

# 3. Destroy and redeploy
sindri destroy --force
sindri deploy

# 4. Install extensions
sindri extension install --from-config sindri.yaml

# 5. Verify
sindri extension validate-all
sindri doctor --all
```

**Coexistence Strategy:**

V2 and V3 can run side-by-side during transition:

| Version | CLI Access                        | Docker Image               | CI Workflow |
| ------- | --------------------------------- | -------------------------- | ----------- |
| V2      | `./v2/cli/sindri` (relative path) | `ghcr.io/pacphi/sindri:v2` | ci-v2.yml   |
| V3      | `sindri` (installed binary)       | `ghcr.io/pacphi/sindri:v3` | ci-v3.yml   |

---

## 🙏 Acknowledgments

This release represents a complete architectural transformation of Sindri. The V2 → V3 rewrite touched every component of the system, from the CLI to the Docker images to the extension system. Special thanks to:

- **Extension Authors** - For providing feedback on schema changes and testing V3 compatibility
- **Early Alpha Testers** - For testing 18 alpha releases and reporting critical bugs
- **Contributors** - For bug reports, feature requests, and documentation improvements
- **The Rust Community** - For the exceptional ecosystem (clap, serde, tokio, reqwest) that made this rewrite possible

---

## 📞 Support & Resources

### V3 Documentation

| Category        | Key Documents                                                                                                                                    |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| Getting Started | [Quickstart](v3/docs/QUICKSTART.md), [CLI Reference](v3/docs/CLI.md), [Configuration](v3/docs/CONFIGURATION.md)                                  |
| Core Features   | [Secrets](v3/docs/SECRETS_MANAGEMENT.md), [Backup](v3/docs/BACKUP_RESTORE.md), [Projects](v3/docs/PROJECTS.md), [Doctor](v3/docs/DOCTOR.md)      |
| Providers       | [Docker](v3/docs/providers/DOCKER.md), [Fly.io](v3/docs/providers/FLY.md), [K8s](v3/docs/providers/KUBERNETES.md), [VM](v3/docs/providers/VM.md) |
| Extensions      | [Overview](v3/docs/EXTENSIONS.md), [Authoring](v3/docs/extensions/guides/AUTHORING.md), [Sourcing](v3/docs/extensions/guides/SOURCING_MODES.md)  |
| Security        | [Image Management](v3/docs/IMAGE_MANAGEMENT.md), [VM Security](v3/docs/providers/vm/SECURITY.md)                                                 |
| Architecture    | [Overview](v3/docs/ARCHITECTURE.md), [ADRs](v3/docs/architecture/adr/README.md)                                                                  |
| Migration       | [Comparison Guide](docs/migration/COMPARISON_GUIDE.md), [Migration Guide](docs/migration/MIGRATION_GUIDE.md)                                     |

### Getting Help

- **Issues:** https://github.com/pacphi/sindri/issues
- **Discussions:** https://github.com/pacphi/sindri/discussions
- **FAQ:** https://sindri-faq.fly.dev

### Version History

- **v3.0.0** (TBD) - Complete Rust rewrite, native binary distribution, VM images, K8s, S3 secrets
- **v2.5.0** (January 2026) - Latest V2 stable release
- **v2.0.0** (January 2026) - Extension capabilities system, pnpm migration
- **v1.13.0** (January 2026) - E2B provider, backup/restore

**Full Changelog:** https://github.com/pacphi/sindri/compare/v2.5.0...v3.0.0

**Alpha Releases:** [v3/CHANGELOG.md](v3/CHANGELOG.md) - Detailed alpha.1 through alpha.18 changelog

---

_For the complete commit history, see: https://github.com/pacphi/sindri/commits/v3.0.0_
