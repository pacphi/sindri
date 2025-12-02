# GitHub Actions Workflow Architecture

## Overview

This document describes the YAML-driven GitHub Actions workflow architecture for Sindri.
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
│   ├── ci.yml                    # Main CI orchestrator (unified provider testing)
│   ├── validate-yaml.yml         # Comprehensive YAML validation
│   ├── test-sindri-config.yml    # Config-driven testing
│   ├── deploy-sindri.yml         # Reusable deployment
│   ├── teardown-sindri.yml       # Reusable teardown
│   ├── test-provider.yml         # Full test suite per provider (CLI + extensions + integration)
│   ├── manual-deploy.yml         # Manual deployment workflow
│   └── release.yml               # Release automation
│
├── actions/                      # Composite Actions
│   ├── core/                     # Core functionality
│   │   ├── setup-sindri/         # Environment setup, config parsing
│   │   ├── build-image/          # Docker image building with caching
│   │   └── test-cli/             # CLI command testing
│   │
│   └── providers/                # Provider-specific actions
│       ├── docker/
│       │   └── setup/            # Docker/Buildx setup
│       ├── fly/
│       │   ├── setup/            # Fly CLI install, app creation
│       │   ├── deploy/           # Fly.io deployment
│       │   ├── test/             # Fly.io testing
│       │   └── cleanup/          # Fly.io resource cleanup
│       └── devpod/
│           ├── setup/            # DevPod CLI, cloud provider setup
│           ├── deploy/           # DevPod workspace deployment
│           ├── test/             # DevPod workspace testing
│           └── cleanup/          # DevPod resource cleanup
│
├── scripts/                      # Test scripts and utilities
│   ├── test-all-extensions.sh    # Extension validation script
│   ├── calculate-profile-resources.sh  # Profile resource calculator
│   ├── generate-slack-notification.sh  # Slack message generator
│   ├── lib/
│   │   ├── test-helpers.sh       # Shared test functions
│   │   └── assertions.sh         # Test assertion functions
│   └── extensions/
│       └── test-extension-complete.sh  # Full extension test suite
│
└── WORKFLOW_ARCHITECTURE.md      # This document

examples/                         # Test fixtures AND user examples
├── fly/
│   └── regions/                  # Region-specific examples
├── docker/
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

test/                             # Test suites
├── unit/
│   └── yaml/                     # YAML validation tests
└── integration/                  # Integration tests
```

## Workflows

### Main CI Workflow (`ci.yml`)

The primary CI orchestrator with **unified provider testing**:

- Validates shell scripts (shellcheck)
- Validates markdown (markdownlint)
- Validates all YAML (via `validate-yaml.yml`)
- Builds Docker images
- Runs **unified provider tests** (CLI + extensions + integration) for each selected provider

**Key Design Principle:** Each provider receives identical test coverage:

```text
FOR EACH provider in [docker, fly, devpod-aws, devpod-do, ...]:
  └─> test-provider.yml
      ├─> Phase 1: Deploy infrastructure
      ├─> Phase 2: CLI tests (sindri, extension-manager)
      ├─> Phase 3: Extension tests (validate, install profile)
      ├─> Phase 4: Run test suites (smoke, integration, full)
      └─> Phase 5: Cleanup
```

**Triggers:**

- Push to main/develop branches
- Pull requests
- Manual dispatch with provider selection

### YAML Validation Workflow (`validate-yaml.yml`)

Comprehensive YAML validation:

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

### Config-Driven Test Workflow (`test-sindri-config.yml`)

The core of the YAML-driven approach:

- **Discovers** sindri.yaml files in specified path
- **Validates** each configuration against schema
- **Deploys** using the configuration
- **Tests** with specified suite (smoke/integration/full)
- **Tears down** resources

```yaml
# Example: Test all Fly.io examples
- uses: ./.github/workflows/test-sindri-config.yml
  with:
    config-path: examples/fly/
    test-suite: smoke
```

### Deploy Workflow (`deploy-sindri.yml`)

Reusable deployment accepting only a config file:

```yaml
- uses: ./.github/workflows/deploy-sindri.yml
  with:
    config-path: examples/fly/minimal.sindri.yaml
```

### Teardown Workflow (`teardown-sindri.yml`)

Reusable cleanup accepting only a config file:

```yaml
- uses: ./.github/workflows/teardown-sindri.yml
  with:
    config-path: examples/fly/minimal.sindri.yaml
    force: true
```

### Manual Deploy vs Deploy Sindri: When to Use Each

Two deployment workflows serve different use cases:

| Aspect                   | `manual-deploy.yml`                        | `deploy-sindri.yml`                              |
| ------------------------ | ------------------------------------------ | ------------------------------------------------ |
| **Trigger**              | `workflow_dispatch` only (human-initiated) | `workflow_call` + `workflow_dispatch` (reusable) |
| **Configuration Source** | Generates `sindri.yaml` from UI inputs     | Reads existing `sindri.yaml` file from path      |
| **Design Pattern**       | Monolithic, self-contained                 | Reusable building block                          |
| **Lines of Code**        | ~400                                       | ~130                                             |

**Input Approach:**

- **manual-deploy**: UI-driven with extensive options (provider, environment, VM size, region, extension profile, auto-cleanup, test toggles, Slack notifications). Includes provider-specific size/region mapping logic.
- **deploy-sindri**: Single input (`config-path`). All deployment parameters come from the YAML file itself.

**Job Structure:**

- **manual-deploy** (7 jobs): validate-inputs → build-image → deploy → test-deployment → schedule-cleanup → notify → summary
- **deploy-sindri** (1 job): parse config → deploy

**Provider Handling:**

```yaml
# manual-deploy: Uses composite actions
- uses: ./.github/actions/providers/fly/setup
- uses: ./.github/actions/providers/fly/deploy

# deploy-sindri: Direct CLI calls
./cli/sindri deploy --config "$CONFIG" --provider fly
```

**When to Use Each:**

| Use Case                                      | Recommended Workflow            |
| --------------------------------------------- | ------------------------------- |
| One-off manual deployments with UI            | `manual-deploy`                 |
| CI/CD pipeline integration                    | `deploy-sindri`                 |
| Calling from other workflows                  | `deploy-sindri` (workflow_call) |
| Complex deployment with tests + notifications | `manual-deploy`                 |
| Simple "deploy this config file"              | `deploy-sindri`                 |

**Trade-offs:**

| `manual-deploy`                                        | `deploy-sindri`                                       |
| ------------------------------------------------------ | ----------------------------------------------------- |
| ✅ Rich UI with sensible defaults                      | ✅ Config-as-code (sindri.yaml is source of truth)    |
| ✅ Built-in testing, cleanup scheduling, notifications | ✅ Reusable from other workflows                      |
| ✅ Provider-specific size/region mapping               | ✅ Simpler, easier to maintain                        |
| ❌ Harder to version control (inputs are ephemeral)    | ❌ No built-in extras (tests, notifications, cleanup) |
| ❌ More complex, more maintenance                      | ❌ Less provider-specific intelligence in workflow    |

### Provider Test Workflow (`test-provider.yml`)

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

| Provider       | Execution Method     | Required Credentials                         |
| -------------- | -------------------- | -------------------------------------------- |
| `docker`       | `docker exec`        | None                                         |
| `fly`          | `flyctl ssh console` | `FLY_API_TOKEN`                              |
| `devpod-aws`   | `devpod exec`        | `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY` |
| `devpod-gcp`   | `devpod exec`        | `GCP_SERVICE_ACCOUNT_KEY`                    |
| `devpod-azure` | `devpod exec`        | `AZURE_CLIENT_ID/SECRET/TENANT_ID`           |
| `devpod-do`    | `devpod exec`        | `DIGITALOCEAN_TOKEN`                         |
| `devpod-k8s`   | `devpod exec`        | `KUBECONFIG` (optional - uses kind if absent)|

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

Profiles are defined in `docker/lib/profiles.yaml` with varying resource requirements:

| Profile         | Extensions | Disk Required | Timeout |
| --------------- | ---------- | ------------- | ------- |
| `minimal`       | 2          | ~1.0 GB       | 15 min  |
| `data-science`  | 2          | ~0.6 GB       | 15 min  |
| `mobile`        | 1          | ~0.6 GB       | 15 min  |
| `fullstack`     | 4          | ~2.2 GB       | 25 min  |
| `ai-dev`        | 5          | ~3.3 GB       | 30 min  |
| `systems`       | 4          | ~4.8 GB       | 35 min  |
| `devops`        | 4          | ~6.2 GB       | 35 min  |
| `anthropic-dev` | 11         | ~6.8 GB       | 40 min  |
| `enterprise`    | 9          | ~12.8 GB      | 45 min  |

### Profile Resource Calculation

The `test-provider.yml` workflow calculates resource requirements based on the selected profile:

1. **Resource Aggregation**: Sums `diskSpace`, `memory`, and `installTime` from all extensions in a profile
2. **Tier Classification**: Maps totals to resource tiers (small/medium/large/xlarge)
3. **Provider Mapping**: Translates tiers to provider-specific VM sizes using `docker/lib/vm-sizes.yaml`

**VM Size Mappings** (`docker/lib/vm-sizes.yaml`):

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
# Manual trigger inputs in ci.yml
providers: "docker,fly,devpod-aws" # Comma-separated or "all"
extension-profile: "fullstack" # Profile to install and test
test-suite: "full" # smoke | integration | full
skip-cleanup: true # Keep resources for debugging
```

**Via sindri.yaml Configuration:**

The `sindri.yaml` file specifies the extension profile to deploy:

```yaml
extensions:
  profile: fullstack # Uses profile from docker/lib/profiles.yaml
```

## Scripts Directory

The `.github/scripts/` directory contains test utilities:

| Script                                  | Purpose                                                              |
| --------------------------------------- | -------------------------------------------------------------------- |
| `test-all-extensions.sh`                | Validates all extensions (used by `pnpm test:extensions`)            |
| `calculate-profile-resources.sh`        | Calculates aggregate resources for a profile (disk, memory, timeout) |
| `generate-slack-notification.sh`        | Generates Slack messages for deployment notifications                |
| `lib/test-helpers.sh`                   | Shared logging, retry, and VM interaction functions                  |
| `lib/assertions.sh`                     | Test assertion functions (equals, contains, file exists, etc.)       |
| `extensions/test-extension-complete.sh` | Full test suite for individual extensions                            |

**Extensibility:** Workflows support optional extension-specific test scripts at
`.github/scripts/test-{extension}.sh`. If present, these are executed; otherwise,
generic tests run. Currently no extension-specific scripts exist - the generic
tests handle all cases.

## YAML-Driven Testing Flow

```text
┌───────────────────────────────────┐
│  examples/fly/minimal.sindri.yaml │
└────────────────┬──────────────────┘
                 │
                 ▼
┌─────────────────────────────────┐
│  test-sindri-config.yml         │
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
                  │ (suite)     │
                  └──────┬──────┘
                         │
                         ▼
                  ┌─────────────┐
                  │ Teardown    │
                  │ (cleanup)   │
                  └─────────────┘
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

| Provider            | Required Secrets                                                      |
| ------------------- | --------------------------------------------------------------------- |
| Docker              | None (local)                                                          |
| Fly.io              | `FLY_API_TOKEN`                                                       |
| DevPod AWS          | `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`                          |
| DevPod GCP          | `GCP_SERVICE_ACCOUNT_KEY`                                             |
| DevPod Azure        | `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET`, `AZURE_TENANT_ID`           |
| DevPod DigitalOcean | `DIGITALOCEAN_TOKEN`                                                  |
| Kubernetes          | `KUBECONFIG` (optional - auto-creates kind cluster if not provided)   |

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

## Usage Examples

### Test All Examples

```yaml
# Via workflow_dispatch
config-path: examples/
test-suite: smoke
```

### Test Specific Provider

```yaml
config-path: examples/fly/
test-suite: integration
```

### Test Single Configuration

```yaml
config-path: examples/fly/minimal.sindri.yaml
test-suite: full
```

### Local Testing

```bash
# Validate YAML
./test/unit/yaml/run-all-yaml-tests.sh

# Test specific config
./cli/sindri test --config examples/fly/minimal.sindri.yaml --suite smoke

# Deploy and connect
./cli/sindri deploy --config examples/fly/minimal.sindri.yaml
./cli/sindri connect --config examples/fly/minimal.sindri.yaml
```

## Adding New Test Scenarios

1. Create a new `sindri.yaml` file in appropriate `examples/` subdirectory
2. The file is automatically:
   - Discovered by `test-sindri-config.yml`
   - Validated against schema
   - Used as documentation for users
3. No workflow changes needed

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

### Debug Mode

```bash
# Local debugging
export DEBUG=true
./cli/sindri test --config examples/fly/minimal.sindri.yaml --suite smoke
```

## References

- [Testing Guide](../docs/TESTING.md)
- [Examples README](../examples/README.md)
