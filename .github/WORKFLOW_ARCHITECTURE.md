# GitHub Actions CI/CD & Workflow Architecture

## Overview

This document describes the CI/CD pipeline and YAML-driven workflow architecture for Sindri v2 (Bash/Docker) and v3 (Rust).

Sindri maintains two parallel versions with independent CI/CD pipelines:

- **v2**: Bash/Docker-based CLI (stable, production-ready)
- **v3**: Rust-based CLI (in active development)

The architecture follows a configuration-first approach where `sindri.yaml` files drive all testing and deployment.

## Architecture Principles

1. **Configuration-Driven**: All provider details live in `sindri.yaml` files, not workflow logic
2. **Examples as Test Fixtures**: The `examples/` directory contains both user documentation AND test fixtures
3. **Reusability**: Common functionality extracted into reusable workflows
4. **Comprehensive Validation**: All YAML files validated against JSON schemas
5. **Single Source of Truth**: `sindri.yaml` is the only input needed for deploy/test/teardown

## Directory Structure

```text
.github/
├── workflows/                    # GitHub Workflows
│   ├── ci-v2.yml                 # v2 CI pipeline (Docker builds, provider tests)
│   ├── ci-v3.yml                 # v3 CI pipeline (Rust builds, cargo tests)
│   ├── validate-yaml.yml         # YAML/schema validation (both versions)
│   ├── validate-shell.yml        # Shell script validation (shellcheck)
│   ├── validate-markdown.yml     # Markdown validation (markdownlint)
│   ├── v2-test-extensions.yml    # Registry-based v2 extension testing (Docker-only)
│   ├── v2-test-profiles.yml         # Config-driven profile testing (discovers sindri.yaml)
│   ├── v2-test-provider.yml         # Full test suite per provider (CLI + extensions + integration)
│   ├── v2-deploy-sindri.yml         # Reusable deployment
│   ├── v2-teardown-sindri.yml       # Reusable teardown
│   ├── v2-manual-deploy.yml      # v2 manual deployment (UI-driven)
│   ├── release-v2.yml            # v2 release automation (Docker images)
│   ├── release-v3.yml            # v3 release automation (Rust binaries)
│   ├── check-links.yml           # Documentation link checking
│   └── cleanup-workflow-runs.yml # Workflow run cleanup
│
├── actions/                      # Composite Actions
│   ├── shared/                   # Shared actions (used by v2, available to v3)
│   │   ├── build-image/          # Docker image build
│   │   ├── deploy-provider/      # Deploy to provider
│   │   └── cleanup-provider/     # Provider cleanup
│   ├── v3/                       # v3-specific actions
│   │   ├── setup-rust/           # Rust toolchain setup with caching
│   │   └── build-rust/           # Rust workspace build
│   ├── packer/                   # Multi-cloud VM image actions
│   │   ├── launch-instance/      # Launch test instances
│   │   ├── terminate-instance/   # Terminate test instances
│   │   └── providers/            # Cloud-specific implementations
│   │       ├── aws/              # AWS EC2
│   │       ├── azure/            # Azure VMs
│   │       ├── gcp/              # GCP Compute
│   │       ├── oci/              # Oracle Cloud Infrastructure
│   │       └── alibaba/          # Alibaba Cloud ECS
│   └── providers/                # Provider-specific actions
│       ├── fly/                  # Fly.io (setup, deploy, test, cleanup)
│       └── devpod/               # DevPod (setup, deploy, test, cleanup)
│
├── scripts/                      # Scripts and utilities
│   ├── generate-slack-notification.sh  # Slack message generator
│   ├── providers/                # Provider-specific scripts
│   │   ├── common-setup.sh       # Common provider setup utilities
│   │   ├── setup-credentials.sh  # Credential setup for providers
│   │   ├── run-on-provider.sh    # Execute commands on providers
│   │   ├── docker-setup.sh       # Docker provider setup
│   │   ├── fly-setup.sh          # Fly.io provider setup
│   │   └── devpod-setup.sh       # DevPod provider setup
│   └── v3/                       # v3-specific scripts
│       ├── discover-extensions.sh  # Extension discovery (profiles, categories)
│       └── k3d-manager.sh          # k3d cluster lifecycle management
│
└── dependabot.yml                # Dependency updates

examples/                         # Test fixtures AND user examples
├── fly/
│   └── regions/                  # Region-specific examples
├── v2/docker/
├── devpod/
│   ├── aws/
│   │   └── regions/
│   ├── gcp/
│   │   └── regions/
│   ├── azure/
│   │   └── regions/
│   ├── digitalocean/
│   │   └── regions/
│   └── kubernetes/               # K8s examples (uses kind in CI if no KUBECONFIG)
└── profiles/

v2/test/                          # v2 Test suites
├── unit/
│   └── yaml/                     # YAML validation tests
└── e2b/                          # E2B provider tests
```

## Shared Actions

The `.github/actions/shared/` directory contains reusable composite actions:

### build-image

Builds Docker images with intelligent caching.

| Input              | Required | Default         | Description                                                   |
| ------------------ | -------- | --------------- | ------------------------------------------------------------- |
| `dockerfile`       | **Yes**  | -               | Path to Dockerfile (e.g., `v2/Dockerfile` or `v3/Dockerfile`) |
| `image-tag`        | No       | `sindri:latest` | Docker image tag to build                                     |
| `context`          | No       | `.`             | Build context path                                            |
| `push`             | No       | `false`         | Whether to push to registry                                   |
| `registry`         | No       | -               | Docker registry URL                                           |
| `cache-key-prefix` | No       | `sindri-docker` | Cache key prefix for layer caching                            |
| `platforms`        | No       | `linux/amd64`   | Target platforms for multi-arch build                         |
| `no-cache`         | No       | `false`         | Disable build cache                                           |

### deploy-provider

Deploys to a specified provider using v2 adapters.

### cleanup-provider

Cleans up provider resources after deployment.

## Path-Based Triggers

| Changed Path                  | Triggers         | Example                                 |
| ----------------------------- | ---------------- | --------------------------------------- |
| `v2/**`                       | `ci-v2.yml`      | Changes to v2 code, scripts, extensions |
| `v3/**`                       | `ci-v3.yml`      | Changes to v3 Rust code, extensions     |
| `.github/workflows/ci-v2.yml` | `ci-v2.yml`      | Self-trigger for workflow changes       |
| `.github/workflows/ci-v3.yml` | `ci-v3.yml`      | Self-trigger for workflow changes       |
| `.github/actions/shared/**`   | `ci-v2.yml`      | Shared action changes (build, deploy)   |
| `.github/actions/v3/**`       | `ci-v3.yml`      | v3 action changes                       |
| `.github/actions/packer/**`   | `ci-v3.yml`      | Packer VM image action changes          |
| `.github/workflows/v3-*.yml`  | `ci-v3.yml`      | v3 extension testing workflows          |
| `.github/scripts/v3/**`       | `ci-v3.yml`      | v3 scripts (discovery, k3d management)  |
| `package.json`                | `ci-v2.yml`      | Root tooling affects v2 validation      |
| Tags `v2.*.*`                 | `release-v2.yml` | v2 release trigger                      |
| Tags `v3.*.*`                 | `release-v3.yml` | v3 release trigger                      |

## CI Workflows

### ci-v2.yml - v2 Bash/Docker CI

**Triggers**: Changes to `v2/` directory

**Jobs**:

- **build**: Docker image from `v2/Dockerfile`
- **generate-matrix**: Provider test matrix
- **test-providers**: Unified provider testing
- **ci-required/ci-status**: Status gates

**Key Design Principle:** Each provider receives identical test coverage:

```text
FOR EACH provider in [docker, fly, devpod-aws, devpod-do, ...]:
  └─> v2-test-provider.yml
      ├─> Setup credentials
      ├─> Deploy infrastructure
      ├─> Run sindri-test.sh (inside container)
      │   ├─> Quick: CLI validation
      │   ├─> Extension: Single extension lifecycle
      │   └─> Profile: Profile lifecycle
      └─> Cleanup
```

### ci-v3.yml - v3 Rust CI

**Triggers**: Changes to `v3/` directory

**Jobs**:

- **rust-format**: `cargo fmt --check`
- **rust-clippy**: `cargo clippy` linting
- **rust-test**: `cargo test` unit tests
- **rust-build**: Release build
- **security-audit**: `cargo audit`
- **docs-build**: `cargo doc`
- **test-extensions**: Extension validation
- **ci-required/ci-status**: Status gates

## Validation Workflows

Validation is handled by dedicated workflows (not by ci-\* workflows):

| Workflow                | Triggers            | Purpose                                           |
| ----------------------- | ------------------- | ------------------------------------------------- |
| `validate-yaml.yml`     | `**.yaml`, `**.yml` | YAML linting, schema validation, cross-references |
| `validate-shell.yml`    | `**.sh`             | Shellcheck for v2 and GitHub scripts              |
| `validate-markdown.yml` | `**.md`             | Markdownlint for v2, v3, and root docs            |

### YAML Validation (`validate-yaml.yml`)

Comprehensive YAML validation for both v2 and v3:

- YAML linting (yamllint)
- Schema validation (all YAML files against their schemas):
  - Extension definitions (`extension.yaml`)
  - Sindri configuration examples (`*.sindri.yaml`)
  - Profiles (`profiles.yaml`)
  - Registry (`registry.yaml`)
  - Categories (`categories.yaml`)
  - Project templates (`project-templates.yaml`)
  - VM size mappings (`vm-sizes.yaml`)
- Cross-reference validation (profiles → registry → extensions → categories)
- Extension consistency checks

### Shell Validation (`validate-shell.yml`)

Shell script validation using shellcheck:

- **shellcheck-v2**: Validates all `v2/**/*.sh` scripts
- **shellcheck-github**: Validates `.github/scripts/**/*.sh`
- Skips zsh scripts (shellcheck doesn't support zsh)
- Triggers on changes to `**.sh` files

### Markdown Validation (`validate-markdown.yml`)

Markdown validation using markdownlint:

- **markdownlint-v2**: Validates `v2/**/*.md`
- **markdownlint-v3**: Validates `v3/**/*.md`
- **markdownlint-root**: Validates root and `.github/**/*.md`
- Triggers on changes to `**.md` files

## Release Workflows

### release-v2.yml - v2 Docker Releases

**Trigger**: Git tags matching `v2.*.*` (e.g., `v2.3.0`, `v2.3.1-beta.1`)

**Process**:

1. Validate tag format (`v2.x.y`)
2. Generate changelog from `v2/` commits
3. Build Docker image from `v2/Dockerfile`
4. Push to GHCR with tags:
   - `ghcr.io/pacphi/sindri:v2.3.0`
   - `ghcr.io/pacphi/sindri:v2.3`
   - `ghcr.io/pacphi/sindri:v2`
   - `ghcr.io/pacphi/sindri:latest` (for stable releases)
5. Update `v2/cli/VERSION` and `v2/CHANGELOG.md`
6. Create GitHub release with install script
7. Commit version updates to main branch

**Creating a v2 Release**:

```bash
# Create and push tag
git tag v2.3.0
git push origin v2.3.0

# Or with message
git tag -a v2.3.0 -m "Release v2.3.0"
git push origin v2.3.0
```

### release-v3.yml - v3 Rust Binary Releases

**Trigger**: Git tags matching `v3.*.*` (e.g., `v3.0.0`, `v3.1.0-alpha.1`)

**Process**:

1. Validate tag format (`v3.x.y`)
2. Generate changelog from `v3/` commits
3. Build release binaries for multiple platforms:
   - Linux (x86_64, aarch64)
   - macOS (x86_64, aarch64/Apple Silicon)
   - Windows (x86_64)
4. Create release archives:
   - `.tar.gz` for Unix platforms
   - `.zip` for Windows
5. Update `v3/Cargo.toml` version and `v3/CHANGELOG.md`
6. Create GitHub release with binary assets
7. Include smart install script (auto-detects platform)
8. Commit version updates to main branch

**Creating a v3 Release**:

```bash
# Create and push tag
git tag v3.0.0
git push origin v3.0.0

# Or with message
git tag -a v3.0.0 -m "Release v3.0.0 - First Rust release"
git push origin v3.0.0
```

## Testing Workflows

### Extension Testing Workflow (`v2-test-extensions.yml`)

Registry-based extension testing that runs in Docker (fast, local):

- **Reads** extensions directly from `v2/docker/lib/registry.yaml`
- **Supports** single extension, comma-separated list, or `all`
- **Matrix** runs each extension as a separate job (max 4 parallel)
- **Excludes** protected base extensions from `all` (mise-config, github-cli)

```yaml
# Example: Test specific extensions
- uses: ./.github/workflows/v2-test-extensions.yml
  with:
    extensions: nodejs,python,golang

# Example: Test all non-protected extensions
- uses: ./.github/workflows/v2-test-extensions.yml
  with:
    extensions: all
```

### V3 Extension Testing System

The v3 extension testing system uses a multi-workflow architecture with dynamic extension discovery and multi-provider support.

**Main Workflow** (`v3-extension-test.yml`):

- **Selection modes**: profile, category, specific, all, changed
- **Providers**: docker, fly, k3d, devpod, packer
- **Dynamic discovery** from `v3/extensions/` folder

**Selection Modes**:

| Mode       | Description                       | Example                           |
| ---------- | --------------------------------- | --------------------------------- |
| `profile`  | Test extensions in a profile      | `minimal`, `ai-dev`, `fullstack`  |
| `category` | Test all extensions in a category | `languages`, `ai-agents`, `cloud` |
| `specific` | Test specific extensions          | `nodejs,python,docker`            |
| `all`      | Test all discovered extensions    | N/A                               |
| `changed`  | Test extensions modified in PR    | N/A                               |

**Provider Workflows**:

| Workflow                 | Provider | Resource Limit | Use Case                    |
| ------------------------ | -------- | -------------- | --------------------------- |
| `v3-provider-docker.yml` | Docker   | 2GB            | Local testing, CI runners   |
| `v3-provider-fly.yml`    | Fly.io   | 8GB            | Cloud VMs with auto-suspend |
| `v3-provider-k3d.yml`    | k3d      | 4GB            | Kubernetes testing          |
| `v3-provider-devpod.yml` | DevPod   | 16GB           | Multi-cloud (AWS/GCP/Azure) |
| `v3-provider-packer.yml` | Packer   | 32GB           | VM image testing            |

**Extension Lifecycle Test Pattern**:

```bash
# Each provider runs the same test sequence
sindri extension install "$EXT" --yes
sindri extension validate "$EXT"
sindri extension test "$EXT"  # optional
sindri extension remove "$EXT" --yes
```

**Example Usage**:

```bash
# Test minimal profile on Docker
gh workflow run v3-extension-test.yml \
  -f selection-mode=profile \
  -f profile=minimal \
  -f providers=docker

# Test specific extensions on multiple providers
gh workflow run v3-extension-test.yml \
  -f selection-mode=specific \
  -f extensions="nodejs,python,golang" \
  -f providers=docker,k3d,fly
```

### Profile Testing Workflow (`v2-test-profiles.yml`)

Config-driven testing for sindri.yaml files:

- **Discovers** sindri.yaml files in specified path
- **Validates** each configuration against schema
- **Deploys** using the configuration
- **Tests** with specified level (quick/profile/all)
- **Tears down** resources

```yaml
# Example: Test all Fly.io examples
- uses: ./.github/workflows/v2-test-profiles.yml
  with:
    config-path: examples/fly/
    test-level: quick
```

### Provider Test Workflow (`v2-test-provider.yml`)

**Unified provider testing** that runs the complete test suite for a single provider:

**Test Phases:**

1. **Infrastructure Deployment** - Sets up Docker/Fly.io/DevPod infrastructure
2. **CLI Tests** - Uses `test-cli` action to run commands on deployed infrastructure
3. **Extension Tests** - Validates and installs extensions on the provider
4. **Integration Tests** - Smoke and full test suites
5. **Cleanup** - Tears down infrastructure (unless skip-cleanup is set)

**Supported Providers:**

- `docker` - Local Docker containers
- `fly` - Fly.io cloud VMs
- `devpod-aws` - AWS EC2 via DevPod
- `devpod-gcp` - GCP Compute via DevPod
- `devpod-azure` - Azure VMs via DevPod
- `devpod-do` - DigitalOcean Droplets via DevPod
- `devpod-k8s` - Kubernetes pods via DevPod (auto-provisions kind cluster if no KUBECONFIG)
- `devpod-ssh` - SSH hosts via DevPod

**CLI Test Action (`test-cli`):**

The refactored `test-cli` action supports all providers with provider-specific execution:

| Provider       | Execution Method       | Required Credentials                          |
| -------------- | ---------------------- | --------------------------------------------- |
| `docker`       | `docker exec`          | None                                          |
| `fly`          | `flyctl ssh console`   | `FLY_API_TOKEN`                               |
| `devpod-aws`   | `devpod ssh --command` | `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`  |
| `devpod-gcp`   | `devpod ssh --command` | `GCP_SERVICE_ACCOUNT_KEY`                     |
| `devpod-azure` | `devpod ssh --command` | `AZURE_CLIENT_ID/SECRET/TENANT_ID`            |
| `devpod-do`    | `devpod ssh --command` | `DIGITALOCEAN_TOKEN`                          |
| `devpod-k8s`   | `devpod ssh --command` | `KUBECONFIG` (optional - uses kind if absent) |

## Test Suites

The CI system supports three test suite levels, selectable via workflow dispatch or determined automatically.

### Smoke Tests

**Purpose:** Quick sanity check to verify deployment is alive.

**What runs:**

- Verifies container/VM is running
- Executes `sindri --version` inside the environment
- Confirms basic connectivity

**Duration:** ~30 seconds

**When used:** Quick validation, debugging deployments

### Integration Tests

**Purpose:** Validate extension system and profiles work correctly.

**What runs:**

- `extension-manager list` - enumerate available extensions
- `extension-manager install-profile <profile>` - install extension profile
- `extension-manager validate-all` - validate all installed extensions
- Provider-specific test scripts (if present at `.github/scripts/test-provider-<provider>.sh`)

**Duration:** 2-5 minutes depending on profile

**When used:** Default for PRs and pushes

### Full Tests

**Purpose:** Comprehensive validation running both smoke AND integration tests sequentially.

**What runs:**

- All smoke tests
- All integration tests

**Duration:** 3-6 minutes

**When used:** Thorough validation before releases or major merges

### Test Suite Selection

| Trigger            | Default Suite   | Providers Tested                        |
| ------------------ | --------------- | --------------------------------------- |
| PR to main/develop | `integration`   | `["docker"]`                            |
| Push to main       | `integration`   | `["docker", "fly"]`                     |
| Manual dispatch    | User-selectable | User-selectable (default: `docker,fly`) |

## Profile-Driven Extension Testing

Extension testing is **profile-driven**: the selected profile determines which extensions are installed and validated.

### How It Works

1. **Profile Selection**: CI workflow selects an extension profile (default: `minimal`)
2. **Profile Installation**: `extension-manager install-profile <profile>` installs all extensions in the profile
3. **Validation**: Each installed extension is validated with `extension-manager validate <ext>`

### Available Profiles

Profiles are defined in `v2/docker/lib/profiles.yaml` with varying resource requirements:

| Profile         | Extensions | Disk Required | Timeout |
| --------------- | ---------- | ------------- | ------- |
| `minimal`       | 2          | ~1.0 GB       | 15 min  |
| `mobile`        | 1          | ~0.6 GB       | 15 min  |
| `fullstack`     | 4          | ~2.2 GB       | 25 min  |
| `ai-dev`        | 5          | ~3.3 GB       | 30 min  |
| `systems`       | 4          | ~4.8 GB       | 35 min  |
| `devops`        | 4          | ~6.2 GB       | 35 min  |
| `anthropic-dev` | 11         | ~6.8 GB       | 40 min  |
| `enterprise`    | 9          | ~12.8 GB      | 45 min  |

### Profile Resource Calculation

The `v2-test-provider.yml` workflow calculates resource requirements based on the selected profile:

1. **Resource Aggregation**: Sums `diskSpace`, `memory`, and `installTime` from all extensions in a profile
2. **Tier Classification**: Maps totals to resource tiers (small/medium/large/xlarge)
3. **Provider Mapping**: Translates tiers to provider-specific VM sizes using `v2/docker/lib/vm-sizes.yaml`

**VM Size Mappings** (`v2/docker/lib/vm-sizes.yaml`):

| Provider     | Small         | Medium        | Large           | XLarge          |
| ------------ | ------------- | ------------- | --------------- | --------------- |
| Fly.io       | shared-cpu-1x | shared-cpu-2x | performance-2x  | performance-4x  |
| Docker       | default       | default       | default         | default         |
| AWS          | t3.small      | t3.medium     | t3.large        | t3.xlarge       |
| GCP          | e2-small      | e2-medium     | e2-standard-4   | e2-standard-8   |
| Azure        | Standard_B1s  | Standard_B2s  | Standard_D2s_v3 | Standard_D4s_v3 |
| DigitalOcean | s-1vcpu-2gb   | s-2vcpu-4gb   | s-4vcpu-8gb     | s-8vcpu-16gb    |

This enables right-sizing CI infrastructure based on profile complexity.

### Configuring Extension Tests

**Via Workflow Dispatch (UI):**

```yaml
# Manual trigger inputs in ci-v2.yml
providers: "docker,fly,devpod-aws" # Comma-separated or "all"
extension-profile: "fullstack" # Profile to install and test
test-suite: "full" # smoke | integration | full
skip-cleanup: true # Keep resources for debugging
```

**Via sindri.yaml Configuration:**

The `sindri.yaml` file specifies the extension profile to deploy:

```yaml
extensions:
  profile: fullstack # Uses profile from v2/docker/lib/profiles.yaml
```

## Deployment Workflows

### Deploy Workflow (`v2-deploy-sindri.yml`)

Reusable deployment accepting only a config file:

```yaml
- uses: ./.github/workflows/v2-deploy-sindri.yml
  with:
    config-path: examples/fly/minimal.sindri.yaml
```

### Teardown Workflow (`v2-teardown-sindri.yml`)

Reusable cleanup accepting only a config file:

```yaml
- uses: ./.github/workflows/v2-teardown-sindri.yml
  with:
    config-path: examples/fly/minimal.sindri.yaml
    force: true
```

### Manual Deploy v2 vs Deploy Sindri: When to Use Each

Two deployment workflows serve different use cases (note: `v2-manual-deploy.yml` is for v2 Bash/Docker deployments only):

| Aspect                   | `v2-manual-deploy.yml`                     | `v2-deploy-sindri.yml`                           |
| ------------------------ | ------------------------------------------ | ------------------------------------------------ |
| **Version**              | v2 only (Bash/Docker)                      | v2 (v3 support planned)                          |
| **Trigger**              | `workflow_dispatch` only (human-initiated) | `workflow_call` + `workflow_dispatch` (reusable) |
| **Configuration Source** | Generates `sindri.yaml` from UI inputs     | Reads existing `sindri.yaml` file from path      |
| **Design Pattern**       | Monolithic, self-contained                 | Reusable building block                          |
| **Lines of Code**        | ~400                                       | ~130                                             |

**Input Approach:**

- **manual-deploy-v2**: UI-driven with extensive options (provider, environment, VM size, region, extension profile, auto-cleanup, test toggles, Slack notifications). Includes provider-specific size/region mapping logic.
- **deploy-sindri**: Single input (`config-path`). All deployment parameters come from the YAML file itself.

**Job Structure:**

- **manual-deploy-v2** (7 jobs): validate-inputs → build-image → deploy → test-deployment → schedule-cleanup → notify → summary
- **deploy-sindri** (1 job): parse config → deploy

**Provider Handling:**

```yaml
# manual-deploy-v2: Uses composite actions
- uses: ./.github/actions/providers/fly/setup
- uses: ./.github/actions/providers/fly/deploy

# deploy-sindri: Direct CLI calls
./v2/cli/sindri deploy --config "$CONFIG" --provider fly
```

**When to Use Each:**

| Use Case                                      | Recommended Workflow            |
| --------------------------------------------- | ------------------------------- |
| One-off v2 manual deployments with UI         | `manual-deploy-v2`              |
| CI/CD pipeline integration                    | `deploy-sindri`                 |
| Calling from other workflows                  | `deploy-sindri` (workflow_call) |
| Complex deployment with tests + notifications | `manual-deploy-v2`              |
| Simple "deploy this config file"              | `deploy-sindri`                 |

**Trade-offs:**

| `manual-deploy-v2`                                     | `deploy-sindri`                                       |
| ------------------------------------------------------ | ----------------------------------------------------- |
| ✅ Rich UI with sensible defaults                      | ✅ Config-as-code (sindri.yaml is source of truth)    |
| ✅ Built-in testing, cleanup scheduling, notifications | ✅ Reusable from other workflows                      |
| ✅ Provider-specific size/region mapping               | ✅ Simpler, easier to maintain                        |
| ❌ v2 only (no v3 support yet)                         | ❌ No built-in extras (tests, notifications, cleanup) |
| ❌ Harder to version control (inputs are ephemeral)    | ❌ Less provider-specific intelligence in workflow    |

## Scripts Directory

The `.github/scripts/` directory contains test utilities:

| Script                           | Purpose                                               |
| -------------------------------- | ----------------------------------------------------- |
| `generate-slack-notification.sh` | Generates Slack messages for deployment notifications |
| `providers/common-setup.sh`      | Common provider setup utilities                       |
| `providers/setup-credentials.sh` | Credential setup for providers                        |
| `providers/run-on-provider.sh`   | Execute commands on providers                         |
| `providers/docker-setup.sh`      | Docker provider setup                                 |
| `providers/fly-setup.sh`         | Fly.io provider setup                                 |
| `providers/devpod-setup.sh`      | DevPod provider setup                                 |
| `v3/discover-extensions.sh`      | Extension discovery for v3 (profiles, categories)     |
| `v3/k3d-manager.sh`              | k3d cluster lifecycle management                      |

**Extension Testing:** All extension tests are now integrated into the `v2-test-provider.yml` workflow with 9 phases:

1. Profile Installation
2. Extension Discovery
3. Extension Validation
4. Functionality Tests (integration/full only)
5. Idempotency Tests (integration/full only)
6. File System Checks (integration/full only)
7. Environment Checks (integration/full only)
8. Uninstall & Cleanup (integration/full only)
9. Results Summary

**Extensibility:** Workflows support optional provider-specific test scripts at
`.github/scripts/test-provider-{provider}.sh`. If present, these are executed as part of the integration test phase.

## YAML-Driven Testing Flow

### Profile Testing (v2-test-profiles.yml)

```text
┌───────────────────────────────────┐
│  examples/fly/minimal.sindri.yaml │
└────────────────┬──────────────────┘
                 │
                 ▼
┌─────────────────────────────────┐
│  v2-test-profiles.yml              │
│  - Discover configs             │
│  - Parse provider/profile       │
└────────────────┬────────────────┘
                 │
         ┌───────┴───────┐
         ▼               ▼
┌─────────────┐   ┌─────────────┐
│ Validate    │   │ Deploy      │
│ (schema)    │   │ (provider)  │
└─────────────┘   └──────┬──────┘
                         │
                         ▼
                  ┌─────────────┐
                  │ Test        │
                  │ (level)     │
                  └──────┬──────┘
                         │
                         ▼
                  ┌─────────────┐
                  │ Teardown    │
                  │ (cleanup)   │
                  └─────────────┘
```

### Extension Testing (v2-test-extensions.yml)

```text
┌───────────────────────────────────┐
│  Input: "nodejs,python" or "all"  │
└────────────────┬──────────────────┘
                 │
                 ▼
┌─────────────────────────────────┐
│  v2-test-extensions.yml            │
│  - Parse input (split/expand)   │
│  - Query registry for "all"     │
└────────────────┬────────────────┘
                 │
                 ▼
┌─────────────────────────────────┐
│  Build Docker image (once)      │
└────────────────┬────────────────┘
                 │
                 ▼
┌─────────────────────────────────┐
│  Matrix: FOR EACH extension     │
│  ├─> Start container            │
│  ├─> Run sindri-test.sh         │
│  │   --level extension          │
│  │   --extension <name>         │
│  └─> Cleanup container          │
└─────────────────────────────────┘
```

## Benefits Over Previous Approach

| Aspect                 | Previous (Workflow Inputs)   | Current (YAML-Driven)                    |
| ---------------------- | ---------------------------- | ---------------------------------------- |
| **Regions**            | Polluted workflow inputs     | Each provider's regions in example files |
| **Adding providers**   | Edit workflow inputs, matrix | Just add new example files               |
| **Adding regions**     | Edit choice options          | Add a new example file                   |
| **Consumer testing**   | Different interface          | Same interface as consumers              |
| **Provider options**   | Scattered in workflows       | Consolidated in schema                   |
| **Test maintenance**   | Complex workflow logic       | Simple file iteration                    |
| **Debugging**          | Which inputs were used?      | Just look at the YAML file               |
| **User documentation** | Separate from test fixtures  | Examples ARE the docs                    |

## Required Secrets by Provider

| Provider            | Required Secrets                                                    |
| ------------------- | ------------------------------------------------------------------- |
| Docker              | None (local)                                                        |
| Fly.io              | `FLY_API_TOKEN`                                                     |
| DevPod AWS          | `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`                        |
| DevPod GCP          | `GCP_SERVICE_ACCOUNT_KEY`                                           |
| DevPod Azure        | `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET`, `AZURE_TENANT_ID`         |
| DevPod DigitalOcean | `DIGITALOCEAN_TOKEN`                                                |
| Kubernetes          | `KUBECONFIG` (optional - auto-creates kind cluster if not provided) |

### Kubernetes Testing with Kind

The `devpod-k8s` provider supports automatic kind cluster bootstrapping for CI environments:

**Auto-detection behavior:**

- If `KUBECONFIG` secret is provided → uses your external Kubernetes cluster
- If `KUBECONFIG` is not set → automatically creates a local kind cluster

**Kind cluster details:**

- Cluster name: `sindri-ci-<run-id>` (unique per workflow run)
- Kubernetes version: v1.32.0 (configurable)
- Namespace: `sindri-test`
- Automatically cleaned up after tests

This enables fast CI feedback without requiring users to maintain external Kubernetes clusters.

### Kubernetes Example Directory Structure

Two directories serve different Kubernetes use cases:

| Directory                     | Purpose                                             | Used By CI         |
| ----------------------------- | --------------------------------------------------- | ------------------ |
| `examples/devpod/kubernetes/` | Deploy Sindri TO an existing K8s cluster via DevPod | Yes (`devpod-k8s`) |
| `examples/k8s/`               | Create AND deploy to local clusters (kind, k3d)     | No (manual use)    |

**CI Config Path Selection:**

- `devpod-k8s` provider → `examples/devpod/kubernetes/minimal.sindri.yaml`
- The `examples/k8s/` configs are for users who want to create local clusters first

**KUBECONFIG Decision Flow:**

```text
┌─────────────────────────────────────┐
│  devpod-k8s provider selected       │
└────────────────┬────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────┐
│  KUBECONFIG secret provided?        │
└────────────────┬────────────────────┘
                 │
         ┌───────┴───────┐
         │               │
         ▼               ▼
┌─────────────┐   ┌─────────────────────┐
│   Yes       │   │   No                │
│             │   │                     │
│ Use external│   │ Auto-create kind    │
│ cluster     │   │ cluster for CI      │
└─────────────┘   └─────────────────────┘
```

**Manual override options:**

```yaml
# Force kind cluster creation even with KUBECONFIG present
k8s-use-kind: "true"

# Force external cluster (fails if no KUBECONFIG)
k8s-use-kind: "false"
```

## Dependabot Configuration

Automated dependency updates for all ecosystems:

```yaml
# Root npm (tooling)
- package-ecosystem: "npm"
  directory: "/"
  schedule: weekly
  labels: ["dependencies", "tooling"]

# v2 extensions npm
- package-ecosystem: "npm"
  directory: "/v2/docker/lib/extensions"
  schedule: weekly
  labels: ["dependencies", "v2", "extensions"]

# v3 Cargo workspace
- package-ecosystem: "cargo"
  directory: "/v3"
  schedule: weekly
  labels: ["dependencies", "v3"]
  groups: workspace-dependencies

# Docker (v2)
- package-ecosystem: "docker"
  directory: "/v2"
  schedule: weekly
  labels: ["dependencies", "v2"]

# GitHub Actions
- package-ecosystem: "github-actions"
  directory: "/"
  schedule: monthly
  labels: ["dependencies", "ci"]
```

## Package.json Scripts

All scripts are version-prefixed to avoid confusion:

**v2 Commands**:

```bash
pnpm v2:validate        # Validate v2 code
pnpm v2:lint            # Lint v2 code
pnpm v2:test            # Run v2 tests
pnpm v2:build           # Build v2 Docker image
pnpm v2:deploy          # Deploy v2
pnpm v2:ci              # Run v2 CI locally
```

**v3 Commands**:

```bash
pnpm v3:validate        # Validate v3 code (Rust + YAML)
pnpm v3:lint            # Lint v3 code
pnpm v3:test            # Run v3 tests (cargo test)
pnpm v3:build           # Build v3 binaries (cargo build --release)
pnpm v3:clippy          # Run clippy linter
pnpm v3:fmt             # Check Rust formatting
pnpm v3:audit           # Security audit
pnpm v3:ci              # Run v3 CI locally
```

**Shared Commands** (apply to both versions):

```bash
pnpm format             # Format all files (prettier)
pnpm links:check        # Check markdown links
pnpm deps:check         # Check for dependency updates
pnpm audit              # Security audit (npm)
```

## Branch Protection

Recommended branch protection rules for `main`:

**Status Checks Required**:

- `CI v2 Required Checks` (from ci-v2.yml)
- `CI v3 Required Checks` (from ci-v3.yml)

**Settings**:

- Require pull request reviews (1 approver)
- Require status checks to pass
- Require branches to be up to date
- Include administrators

## Common Tasks

### Running CI Locally

**v2**:

```bash
# Full v2 CI
pnpm v2:ci

# Individual steps
pnpm v2:validate
pnpm v2:lint
pnpm v2:test
pnpm v2:build
```

**v3**:

```bash
# Full v3 CI
pnpm v3:ci

# Individual steps
pnpm v3:validate
pnpm v3:lint
pnpm v3:test
pnpm v3:build
```

### Creating Releases

**v2 Release**:

```bash
# Update version in v2/cli/VERSION if needed
echo "2.3.0" > v2/cli/VERSION

# Commit changes
git add v2/
git commit -m "chore(v2): prepare for v2.3.0 release"

# Tag and push
git tag v2.3.0
git push origin main v2.3.0
```

**v3 Release**:

```bash
# Update version in v3/Cargo.toml (workspace.package.version)
sed -i 's/version = ".*"/version = "3.0.0"/' v3/Cargo.toml

# Commit changes
git add v3/
git commit -m "chore(v3): prepare for v3.0.0 release"

# Tag and push
git tag v3.0.0
git push origin main v3.0.0
```

### Debugging Failed Workflows

1. **Check the logs**: Click on the failed job in GitHub Actions
2. **Run locally**: Use `pnpm v2:ci` or `pnpm v3:ci`
3. **Manual trigger**: Use workflow_dispatch with custom options
4. **Skip cleanup**: Enable "skip-cleanup" option to inspect state

### Adding New Actions

**Shared (used by both v2 and v3)**:

```bash
mkdir -p .github/actions/shared/my-action
# Create action.yml
# Reference in ci-v2.yml and/or ci-v3.yml
```

**For v3 only**:

```bash
mkdir -p .github/actions/v3/my-action
# Create action.yml
# Reference in ci-v3.yml
```

**For Packer (multi-cloud VM images)**:

```bash
mkdir -p .github/actions/packer/providers/my-cloud
# Create action.yml with launch/terminate actions
# Reference in launch-instance/action.yml and terminate-instance/action.yml
```

## Extension Management

### v2 Extensions

**Location**: `v2/docker/lib/extensions/`
**Registry**: `v2/docker/lib/registry.yaml`
**Includes**: All extensions, including VisionFlow (vf-\* prefixed)

### v3 Extensions

**Location**: `v3/extensions/`
**Registry**: `v3/registry.yaml`
**Excludes**: VisionFlow extensions (clean break from v2)

**Migrated**: 44 extensions from v2 (excluding 33 vf-\* extensions)

## Usage Examples

### Test All Config Examples (v2-test-profiles.yml)

```yaml
# Via workflow_dispatch
config-path: examples/
test-level: quick
```

### Test Specific Provider Configs (v2-test-profiles.yml)

```yaml
config-path: examples/fly/
test-level: profile
```

### Test Single Configuration (v2-test-profiles.yml)

```yaml
config-path: examples/fly/minimal.sindri.yaml
test-level: all
```

### Test Individual Extensions (v2-test-extensions.yml)

```yaml
# Single extension
extensions: nodejs

# Multiple extensions
extensions: nodejs,python,golang

# All non-protected extensions (70+)
extensions: all
```

### Local Testing

```bash
# Validate YAML
./test/unit/yaml/run-all-yaml-tests.sh

# Test specific config
./v2/cli/sindri test --config examples/fly/minimal.sindri.yaml --suite smoke

# Deploy and connect
./v2/cli/sindri deploy --config examples/fly/minimal.sindri.yaml
./v2/cli/sindri connect --config examples/fly/minimal.sindri.yaml
```

## Adding New Test Scenarios

### Adding Profile/Config Tests

1. Create a new `sindri.yaml` file in appropriate `examples/` subdirectory
2. The file is automatically:
   - Discovered by `v2-test-profiles.yml`
   - Validated against schema
   - Used as documentation for users
3. No workflow changes needed

### Adding Extension Tests

Extensions are automatically tested via `v2-test-extensions.yml`:

1. Add new extension to `v2/docker/lib/registry.yaml`
2. Create extension definition in `v2/docker/lib/extensions/<name>/extension.yaml`
3. Test individually: trigger workflow with `extensions: <name>`
4. Test with all: trigger workflow with `extensions: all` (excludes protected extensions)

## Troubleshooting

### Common Issues

1. **Validation Failures**
   - Run `./test/unit/yaml/run-all-yaml-tests.sh` locally
   - Check cross-references if modifying registry/profiles

2. **Provider Authentication**
   - Verify secrets are set in repository settings
   - Check credential expiration

3. **Test Timeouts**
   - Increase `timeout-minutes` in workflow
   - Check provider resource limits

4. **CI Not Triggering**
   - Check path patterns: Ensure changed files match path triggers in workflow `on.paths`

```yaml
# ci-v2.yml triggers on:
- v2/**
- .github/workflows/ci-v2.yml
- .github/actions/v2/**
```

5. **Both v2 and v3 CI Running**
   - This is expected if you change files in both directories or shared actions

6. **Release Tag Format Error**
   - **Error**: "Invalid tag format"
   - **Solution**: Use correct format:
     - v2 releases: `v2.x.y` (e.g., v2.3.0, v2.3.1-beta.1)
     - v3 releases: `v3.x.y` (e.g., v3.0.0, v3.1.0-alpha.1)

7. **Cache Issues**
   - **Clear cache**: Go to Actions → Caches → Delete cache
   - **Or**: Push with `[skip ci]` in commit message, then push again

### Debug Mode

```bash
# Local debugging
export DEBUG=true
./v2/cli/sindri test --config examples/fly/minimal.sindri.yaml --suite smoke
```

## Future Enhancements

1. **v3 Extension Testing**: Once v3 CLI is functional, enable extension tests
2. **Cross-version Testing**: Test v2 → v3 migration scenarios
3. **Performance Benchmarks**: Compare v2 and v3 performance
4. **Automated Migration Tool**: Help users migrate from v2 to v3
5. **Feature Parity Dashboard**: Track v2 vs v3 capabilities

## Related Documentation

- [ADR-021: Bifurcated CI/CD v2 and v3](../v3/docs/architecture/adr/021-bifurcated-ci-cd-v2-v3.md)
- [v2 Documentation](../v2/docs/)
- [v3 Documentation](../v3/docs/)
- [Contributing Guide](../CONTRIBUTING.md)
- [Testing Guide](../docs/TESTING.md)
- [Examples README](../examples/README.md)
