# CI Testing Deep Dive: Extension and Integration Tests

This document describes the CI testing architecture for Sindri, with a streamlined
approach that puts the `sindri` and `extension-manager` CLIs at the heart of all tests.

## Table of Contents

- [CI Testing Deep Dive: Extension and Integration Tests](#ci-testing-deep-dive-extension-and-integration-tests)
  - [Table of Contents](#table-of-contents)
  - [Design Principles](#design-principles)
  - [CI Test Configuration](#ci-test-configuration)
    - [Resource Allocation Strategy](#resource-allocation-strategy)
    - [autoInstall Setting](#autoinstall-setting)
    - [Environment Variables](#environment-variables)
  - [Architecture Overview](#architecture-overview)
  - [Provider Support](#provider-support)
    - [Secret Management](#secret-management)
  - [Test Levels](#test-levels)
    - [Quick Level (CLI Validation)](#quick-level-cli-validation)
    - [Extension Level (Single Extension Lifecycle)](#extension-level-single-extension-lifecycle)
    - [Profile Level (Profile Lifecycle)](#profile-level-profile-lifecycle)
  - [Functional Tests Reference](#functional-tests-reference)
    - [Quick Level Tests](#quick-level-tests)
    - [Extension Level Tests](#extension-level-tests)
    - [Profile Level Tests](#profile-level-tests)
    - [Extension-Manager Operations Coverage](#extension-manager-operations-coverage)
  - [Test Output Format](#test-output-format)
  - [Fail-Fast Behavior](#fail-fast-behavior)
  - [Local Development](#local-development)
  - [CI Workflow Structure](#ci-workflow-structure)
    - [Stage 1: Validation (Parallel)](#stage-1-validation-parallel)
    - [Stage 2: Build](#stage-2-build)
    - [Stage 3: Provider Testing (Parallel Matrix)](#stage-3-provider-testing-parallel-matrix)
  - [Key Files](#key-files)
  - [Related Documentation](#related-documentation)

---

## Design Principles

1. **CLI-Centric**: The `sindri` and `extension-manager` CLIs are the primary
   interface for all tests. If the CLI works, the system works.

2. **In-Container Testing**: Tests run INSIDE the deployed container via a
   single unified script, eliminating shell quoting issues and reducing
   remote call overhead.

3. **Fail-Fast**: Stop on first failure for faster feedback during development.

4. **Local + CI**: The same test script works both locally and in CI.

5. **Provider-Agnostic**: One test script works across all 7 providers through
   a clean abstraction layer.

6. **Clean Slate Testing**: Container starts without pre-installed extensions
   (`autoInstall: false`), giving tests full control over the extension lifecycle.

---

## CI Test Configuration

### Resource Allocation Strategy

The deploy action automatically selects the appropriate config based on the
test level and profile:

| Test Level  | Profile Being Tested | Config Used                 | Resources            |
| ----------- | -------------------- | --------------------------- | -------------------- |
| `quick`     | any                  | `minimal.sindri.yaml`       | Small (1-2GB RAM)    |
| `extension` | any                  | **`fullstack.sindri.yaml`** | Medium (4GB RAM)     |
| `profile`   | minimal              | `minimal.sindri.yaml`       | Small (1-2GB RAM)    |
| `profile`   | fullstack            | `fullstack.sindri.yaml`     | Medium (4GB RAM)     |
| `profile`   | ai-dev               | `ai-dev.sindri.yaml`        | Large (8GB RAM, GPU) |
| `profile`   | enterprise           | `enterprise.sindri.yaml`    | XLarge (16GB+ RAM)   |

**Why fullstack for extension tests?**

Some extensions have high resource requirements (guacamole: 2GB memory,
xfce-ubuntu: 1GB memory + 2.5GB disk). Using fullstack config ensures
sufficient headroom for any extension being tested.

If a profile-specific config doesn't exist, it falls back to minimal.

### autoInstall Setting

**Default Behavior**: `autoInstall` defaults to **`true`** for end users, automatically
installing the configured extension profile on container startup.

**CI Override**: When deploying with `--ci-mode` flag, all adapters force
`autoInstall=false` to ensure clean slate testing, **regardless of the value in
sindri.yaml**.

```yaml
# examples/docker/minimal.sindri.yaml
extensions:
  profile: minimal
  # autoInstall defaults to true for end users
  # In CI, --ci-mode flag forces this to false for clean testing
```

**Why CI needs clean slate**:

- **Predictable State**: Container starts clean without any extensions installed
- **Explicit Testing**: Tests explicitly call `extension-manager install` commands
- **Full Lifecycle Coverage**: We can test install, validate, and remove operations
- **Failure Isolation**: If install fails, we know it's the install operation, not startup
- **Pre-check Detection**: Tests verify NO extensions installed before starting

### CI Mode Implementation

When `--ci-mode` is enabled, adapters internally set:

```bash
SKIP_AUTO_INSTALL=true  # Forces clean slate regardless of config
```

**Usage**:
```bash
# CI deployment (via deploy-provider action)
./deploy/adapters/docker-adapter.sh deploy --ci-mode --skip-build minimal.sindri.yaml
./deploy/adapters/fly-adapter.sh deploy --ci-mode minimal.sindri.yaml
./deploy/adapters/devpod-adapter.sh deploy --ci-mode minimal.sindri.yaml
```

**Note**: The `--ci-mode` flag is for CI environments only. End users should set
`autoInstall: false` in sindri.yaml if they want manual control.

---

## Architecture Overview

```text
┌──────────────────────────────────────────────────────────────────┐
│                    SIMPLIFIED CI PIPELINE                        │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ci.yml (orchestrator)                                           │
│  ├── validate (shellcheck, yamllint, markdown)                   │
│  ├── build (Docker image)                                        │
│  └── test-provider.yml (per provider)                            │
│      │                                                           │
│      └── Single Job:                                             │
│          1. Setup credentials                                    │
│          2. Deploy to provider                                   │
│          3. Run sindri-test.sh (ONE remote call)                 │
│          4. Cleanup                                              │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

The key insight is **inverting the test architecture**:

- **Before**: Tests run on the host, making repeated remote calls per command
- **After**: Tests run inside the container via a single unified script

---

## Provider Support

All 7 providers are tested with the same unified approach:

| Provider       | Credential Setup     | Deploy Method                | Remote Execution     |
| -------------- | -------------------- | ---------------------------- | -------------------- |
| `docker`       | None                 | `sindri deploy`              | `docker exec`        |
| `fly`          | `FLY_API_TOKEN`      | `fly-adapter.sh`             | `flyctl ssh console` |
| `devpod-aws`   | `AWS_*` env vars     | `devpod up --provider aws`   | `devpod ssh`         |
| `devpod-gcp`   | JSON key file        | `devpod up --provider gcp`   | `devpod ssh`         |
| `devpod-azure` | `AZURE_*` env vars   | `devpod up --provider azure` | `devpod ssh`         |
| `devpod-do`    | `DIGITALOCEAN_TOKEN` | `devpod up --provider do`    | `devpod ssh`         |
| `devpod-k8s`   | `KUBECONFIG`         | `devpod up --provider k8s`   | `devpod ssh`         |

All remote execution flows through `run-on-provider.sh`, which abstracts the
provider-specific execution method.

### Secret Management

Secrets flow from GitHub repository → workflow → environment variables:

| Provider       | Secrets Required                                            | Used In                              |
| -------------- | ----------------------------------------------------------- | ------------------------------------ |
| `docker`       | None                                                        | N/A                                  |
| `fly`          | `FLY_API_TOKEN`                                             | Deploy (flyctl), Test (flyctl ssh)   |
| `devpod-aws`   | `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`                | Deploy (devpod up)                   |
| `devpod-gcp`   | `GCP_SERVICE_ACCOUNT_KEY`                                   | Deploy (written to credentials file) |
| `devpod-azure` | `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET`, `AZURE_TENANT_ID` | Deploy (Azure CLI)                   |
| `devpod-do`    | `DIGITALOCEAN_TOKEN`                                        | Deploy (DevPod)                      |
| `devpod-k8s`   | `KUBECONFIG` (optional)                                     | Deploy (written to ~/.kube/config)   |

**Secret Flow**:

1. `ci.yml` declares `secrets: inherit`
2. `test-provider.yml` declares all secrets as inputs
3. Secrets passed as env vars to `setup-credentials`, `deploy`, and `run-tests` steps
4. Adapter scripts (fly-adapter.sh, devpod-adapter.sh) read from environment

---

## Test Levels

### Quick Level (CLI Validation)

**Purpose**: Verify the Sindri CLI and extension-manager are functional.

**When to Use**: Smoke tests, quick validation, debugging.

**Duration**: ~10-15 seconds

### Extension Level (Single Extension Lifecycle)

**Purpose**: Test the full lifecycle of a single extension install/remove with idempotency check.

**When to Use**: Validating extension-manager core functionality.

**Duration**: ~45-60 seconds (includes idempotency testing)

### Profile Level (Profile Lifecycle)

**Purpose**: Test the full lifecycle of a profile install/remove with idempotency check.

**When to Use**: Default for PRs and CI, comprehensive validation.

**Duration**: ~90-120 seconds (depending on profile, includes idempotency)

---

## Functional Tests Reference

### Quick Level Tests

These tests verify core CLI functionality:

| Test Name                    | Command                             | Success Criteria                                           |
| ---------------------------- | ----------------------------------- | ---------------------------------------------------------- |
| sindri-version               | `sindri --version`                  | Exit code 0, outputs version string                        |
| sindri-help                  | `sindri --help`                     | Exit code 0, outputs help text                             |
| extension-manager-list       | `extension-manager list`            | Exit code 0, lists all available extensions                |
| extension-manager-profiles   | `extension-manager list-profiles`   | Exit code 0, lists all profiles (minimal, fullstack, etc.) |
| extension-manager-categories | `extension-manager list-categories` | Exit code 0, lists categories (base, language, etc.)       |
| mise-available               | `command -v mise`                   | Exit code 0, returns path to mise                          |
| yq-available                 | `command -v yq`                     | Exit code 0, returns path to yq                            |

### Extension Level Tests

Tests the full lifecycle of a single extension:

| Step                           | Command                             | Success Criteria            |
| ------------------------------ | ----------------------------------- | --------------------------- |
| 1. List                        | `extension-manager list`            | Exit 0, registry accessible |
| 2. **Pre-check**               | `extension-manager status nodejs`   | **Status = NOT installed**  |
| 3. Install                     | `extension-manager install nodejs`  | Exit 0, extension installed |
| 4. Validate                    | `extension-manager validate nodejs` | Exit 0, validation passes   |
| 5. Status                      | `extension-manager status nodejs`   | Exit 0, status = installed  |
| 6. Verify                      | `node --version`                    | Exit 0, tool works          |
| 7. **Idempotency: Reinstall**  | `extension-manager install nodejs`  | Exit 0, reinstall succeeds  |
| 8. **Idempotency: Revalidate** | `extension-manager validate nodejs` | Exit 0, still valid         |
| 9. BOM                         | `extension-manager bom`             | Exit 0, BOM generated       |
| 10. Remove                     | `extension-manager remove nodejs`   | Exit 0, extension removed   |
| 11. Verify Removed             | `extension-manager status nodejs`   | Status = not installed      |

> **Pre-check (Step 2)**: Fails immediately if extension is already installed.
> This catches stale volumes, accidental autoInstall, or incomplete cleanup.
>
> **Idempotency (Steps 7-8)**: Verifies reinstalling doesn't cause errors or state corruption.

### Profile Level Tests

Tests the full lifecycle of a profile installation:

| Step                           | Command                                                              | Success Criteria                   |
| ------------------------------ | -------------------------------------------------------------------- | ---------------------------------- |
| 1. List                        | `extension-manager list`                                             | Exit 0, registry accessible        |
| 2. **Pre-check**               | `extension-manager status`                                           | **ALL extensions = NOT installed** |
| 3. Install                     | `extension-manager install-profile minimal`                          | Exit 0, profile installed          |
| 4. Validate                    | `extension-manager validate-all`                                     | Exit 0, all extensions pass        |
| 5. Status                      | `extension-manager status`                                           | Exit 0, all = installed            |
| 6. Verify                      | `node --version && python --version`                                 | Exit 0, tools work                 |
| 7. **Idempotency: Reinstall**  | `extension-manager install-profile minimal`                          | Exit 0, reinstall succeeds         |
| 8. **Idempotency: Revalidate** | `extension-manager validate-all`                                     | Exit 0, all still valid            |
| 9. BOM                         | `extension-manager bom`                                              | Exit 0, BOM generated              |
| 10. Remove                     | `extension-manager remove nodejs && extension-manager remove python` | Exit 0, extensions removed         |
| 11. Verify Removed             | `extension-manager status`                                           | All = not installed                |

> **Pre-check (Step 2)**: Fails immediately if ANY profile extension is already installed.
> This detects dirty state from previous CI runs or misconfigured autoInstall.
>
> **Idempotency (Steps 7-8)**: Verifies reinstalling profile doesn't cause errors or state corruption.

### Extension-Manager Operations Coverage

Summary of which operations are tested at each level:

| Category         | Operation       | Command                                       | Tested In          |
| ---------------- | --------------- | --------------------------------------------- | ------------------ |
| **Discovery**    | List all        | `extension-manager list`                      | Quick              |
|                  | List profiles   | `extension-manager list-profiles`             | Quick              |
|                  | List categories | `extension-manager list-categories`           | Quick              |
|                  | Search          | `extension-manager search {term}`             | Extension          |
|                  | Info            | `extension-manager info {ext}`                | Extension          |
| **Installation** | Install single  | `extension-manager install {ext}`             | Extension          |
|                  | Install profile | `extension-manager install-profile {profile}` | Profile            |
|                  | Resolve deps    | `extension-manager resolve {ext}`             | Extension          |
| **Validation**   | Validate single | `extension-manager validate {ext}`            | Extension          |
|                  | Validate all    | `extension-manager validate-all`              | Profile            |
| **Status**       | Status single   | `extension-manager status {ext}`              | Extension          |
|                  | Status all      | `extension-manager status`                    | Profile            |
| **Removal**      | Remove single   | `extension-manager remove {ext}`              | Extension, Profile |
| **BOM**          | Show BOM        | `extension-manager bom`                       | Extension, Profile |

---

## Test Output Format

The test script outputs structured, human-readable results:

```text
=== Quick Tests ===
PASS: sindri-version (1s)
PASS: sindri-help (1s)
PASS: extension-manager-list (2s)
PASS: extension-manager-profiles (1s)
PASS: extension-manager-categories (1s)
PASS: mise-available (0s)
PASS: yq-available (0s)

=== Extension Lifecycle ===
PASS: list (1s)
PASS: pre-check-nodejs (1s)      # Verified NOT installed
PASS: install-nodejs (25s)
PASS: validate-nodejs (3s)
PASS: status-nodejs (1s)
PASS: verify-node (1s)
PASS: idempotency-reinstall-nodejs (12s)
PASS: idempotency-revalidate-nodejs (3s)
PASS: bom (2s)
PASS: remove-nodejs (2s)
PASS: verify-removed (1s)

=== Profile Lifecycle ===
PASS: list (1s)
PASS: pre-check-profile (2s)     # Verified NO extensions installed
PASS: install-profile-minimal (45s)
PASS: validate-all (8s)
PASS: status-all (2s)
PASS: verify-tools (2s)
PASS: idempotency-reinstall-profile (20s)
PASS: idempotency-revalidate-all (8s)
PASS: bom (2s)
PASS: remove-all (4s)
PASS: verify-removed (1s)

RESULT:PASSED
Summary: 29 passed, 0 failed
```

**On Pre-check Failure** (dirty state detected):

```text
=== Extension Lifecycle ===
PASS: list (1s)
FAIL: pre-check-nodejs (1s)
  Error: DIRTY STATE DETECTED - nodejs is already installed!
  This indicates stale volumes, autoInstall misconfiguration, or incomplete cleanup.
  Expected: not installed, Got: installed

RESULT:FAILED
Summary: 1 passed, 1 failed
```

**On Validation Failure**:

```text
=== Extension Lifecycle ===
PASS: list (1s)
PASS: pre-check-nodejs (1s)
PASS: install-nodejs (25s)
FAIL: validate-nodejs (3s)
  Error: nodejs validation failed - node command not found

RESULT:FAILED
Summary: 3 passed, 1 failed
```

---

## Fail-Fast Behavior

**Default behavior (`--fail-fast`)**:

- Stops execution on first failure
- Returns exit code 1 immediately
- Provides faster feedback during development

**Alternative (`--no-fail-fast`)**:

- Runs all tests regardless of failures
- Reports total pass/fail count at end
- Useful for comprehensive test runs

---

## Local Development

The same test script used in CI can be run locally for debugging:

```bash
# Build the image
docker build -t sindri:local .

# Start a container (with autoInstall disabled)
docker run -d --name test -e SKIP_AUTO_INSTALL=true sindri:local

# Run quick tests (CLI validation only)
docker exec test /docker/scripts/sindri-test.sh --level quick

# Run extension lifecycle tests (single extension)
docker exec test /docker/scripts/sindri-test.sh --level extension

# Run profile lifecycle tests
docker exec test /docker/scripts/sindri-test.sh --level profile --profile minimal

# Run with different profile
docker exec test /docker/scripts/sindri-test.sh --level profile --profile fullstack

# Run all levels
docker exec test /docker/scripts/sindri-test.sh --level all

# Run without fail-fast to see all results
docker exec test /docker/scripts/sindri-test.sh --level all --no-fail-fast

# Interactive debugging
docker exec -it test bash
/docker/scripts/sindri-test.sh --level quick
```

---

## CI Workflow Structure

The simplified CI workflow has three main stages:

### Stage 1: Validation (Parallel)

- `shellcheck` - Shell script linting
- `yamllint` - YAML validation
- `markdownlint` - Markdown validation
- Schema validation against JSON schemas

### Stage 2: Build

- Build Docker image
- Save as artifact for provider tests

### Stage 3: Provider Testing (Parallel Matrix)

For each provider in the matrix (docker, fly, devpod-\*):

1. Setup credentials (provider-specific)
2. Deploy infrastructure
3. **Run tests** (single remote call to `sindri-test.sh`)
4. Cleanup resources

---

## Key Files

| File                                             | Purpose                                     |
| ------------------------------------------------ | ------------------------------------------- |
| `/docker/scripts/sindri-test.sh`                 | Unified test script (runs inside container) |
| `.github/workflows/ci.yml`                       | Main CI orchestrator                        |
| `.github/workflows/test-provider.yml`            | Per-provider test workflow                  |
| `.github/scripts/providers/run-on-provider.sh`   | Provider execution abstraction              |
| `.github/scripts/providers/setup-credentials.sh` | Unified credential setup                    |

---

## Related Documentation

- [TESTING.md](TESTING.md) - General testing philosophy
- [ARCHITECTURE.md](ARCHITECTURE.md) - System architecture overview
- [EXTENSION_AUTHORING.md](EXTENSION_AUTHORING.md) - Creating new extensions
- [CONTRIBUTING.md](CONTRIBUTING.md) - Development workflow and standards
