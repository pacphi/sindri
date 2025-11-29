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
│   ├── ci.yml                    # Main CI orchestrator
│   ├── validate-yaml.yml         # Comprehensive YAML validation
│   ├── test-sindri-config.yml    # Config-driven testing
│   ├── deploy-sindri.yml         # Reusable deployment
│   ├── teardown-sindri.yml       # Reusable teardown
│   ├── test-provider.yml         # Provider-specific testing
│   ├── test-extensions.yml       # Extension testing (multi-provider)
│   ├── manual-deploy.yml         # Manual deployment workflow
│   ├── self-service-deploy-fly.yml # Self-service Fly.io deployment
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
│   ├── generate-slack-notification.sh  # Slack message generator
│   ├── lib/
│   │   ├── test-helpers.sh       # Shared test functions
│   │   └── assertions.sh         # Test assertion functions
│   └── extensions/
│       └── test-extension-complete.sh  # Full extension test suite
│
├── test-configs/                 # Test configuration files
│   ├── providers.yaml            # Provider test parameters
│   └── extensions.yaml           # Extension test parameters
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
│   └── digitalocean/
│       └── regions/
└── profiles/

test/                             # Test suites
├── unit/
│   └── yaml/                     # YAML validation tests
└── integration/                  # Integration tests
```

## Workflows

### Main CI Workflow (`ci.yml`)

The primary CI orchestrator that:

- Validates shell scripts (shellcheck)
- Validates markdown (markdownlint)
- Validates all YAML (via `validate-yaml.yml`)
- Builds Docker images
- Tests CLI commands
- Runs provider and extension tests in parallel

**Triggers:**

- Push to main/develop branches
- Pull requests
- Scheduled (nightly)
- Manual dispatch with provider selection

### YAML Validation Workflow (`validate-yaml.yml`)

Comprehensive YAML validation:

- YAML linting (yamllint)
- Schema validation (all YAML files against their schemas)
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

Provider-specific testing with smoke/integration/full suites.

### Extension Test Workflow (`test-extensions.yml`)

Tests extensions across providers with combination support.

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

**When used:** Default for PRs, pushes, and scheduled runs

### Full Tests

**Purpose:** Comprehensive validation running both smoke AND integration tests sequentially.

**What runs:**

- All smoke tests
- All integration tests

**Duration:** 3-6 minutes

**When used:** Thorough validation before releases or major merges

### Test Suite Selection

| Trigger                     | Default Suite   | Providers Tested                                |
| --------------------------- | --------------- | ----------------------------------------------- |
| PR to main/develop          | `integration`   | `["docker"]`                                    |
| Push to main                | `integration`   | `["docker", "fly"]`                             |
| Scheduled (nightly 2AM UTC) | `integration`   | `["docker", "fly", "devpod-aws", "kubernetes"]` |
| Manual dispatch             | User-selectable | User-selectable (default: `docker,fly`)         |

## Extension Testing by Provider

Each provider tests a specific set of extensions defined in `.github/test-configs/providers.yaml`:

| Provider     | Extensions Tested                         |
| ------------ | ----------------------------------------- |
| docker       | `nodejs`, `python`, `docker`              |
| fly          | `nodejs`, `python`, `golang`, `terraform` |
| devpod-aws   | `nodejs`, `aws-cli`, `terraform`          |
| devpod-gcp   | `nodejs`, `gcloud`, `kubernetes-tools`    |
| devpod-azure | `nodejs`, `azure-cli`, `dotnet`           |
| devpod-do    | `nodejs`, `docker`, `kubectl`             |
| kubernetes   | `kubernetes-tools`, `helm`, `kubectl`     |

### Extension Test Matrices

The CI uses different extension matrices based on context (from `.github/test-configs/extensions.yaml`):

| Matrix          | Extensions                             | When Used              |
| --------------- | -------------------------------------- | ---------------------- |
| `quick`         | `nodejs`, `python`                     | PRs, branch pushes     |
| `standard`      | `nodejs`, `python`, `golang`, `docker` | Main branch, manual    |
| `comprehensive` | All extensions                         | Scheduled nightly runs |

### Overriding Test Configuration

**Via Workflow Dispatch (UI):**

```yaml
# Manual trigger inputs in ci.yml
providers: "docker,fly,devpod-aws" # Comma-separated or "all"
test-suite: "full" # smoke | integration | full
skip-cleanup: true # Keep resources for debugging
```

**Via sindri.yaml Configuration:**

The `sindri.yaml` file specifies the extension profile to deploy:

```yaml
extensions:
  profile: fullstack # Uses profile from docker/lib/profiles.yaml
```

**Via Test Config Files:**

Modify `.github/test-configs/extensions.yaml` to change:

- Which extensions are tested (`test_matrices.quick.extensions`)
- Extension-specific test commands (`extensions.<name>.test_commands`)
- Test combinations (`test_combinations`)

Modify `.github/test-configs/providers.yaml` to change:

- Provider-specific extensions (`providers.<name>.extensions_to_test`)
- Test profiles (`test_profiles`)
- Timeout and resource limits

### Extension Profile in Integration Tests

Integration tests use the `extension-profile` input (default: `minimal`):

```yaml
# From test-provider.yml
extension-profile:
  description: Extension profile to test
  required: false
  default: minimal
```

Available profiles are defined in `docker/lib/profiles.yaml`. Override via workflow dispatch or by calling the reusable workflow with a different value.

## Scripts Directory

The `.github/scripts/` directory contains test utilities:

| Script                                  | Purpose                                                        |
| --------------------------------------- | -------------------------------------------------------------- |
| `test-all-extensions.sh`                | Validates all extensions (used by `pnpm test:extensions`)      |
| `generate-slack-notification.sh`        | Generates Slack messages for deployment notifications          |
| `lib/test-helpers.sh`                   | Shared logging, retry, and VM interaction functions            |
| `lib/assertions.sh`                     | Test assertion functions (equals, contains, file exists, etc.) |
| `extensions/test-extension-complete.sh` | Full test suite for individual extensions                      |

**Extensibility:** Workflows support optional extension-specific test scripts at
`.github/scripts/test-{extension}.sh`. If present, these are executed; otherwise,
generic tests run. Currently no extension-specific scripts exist - the generic
tests handle all cases.

## Test Configurations

The `.github/test-configs/` directory contains reference configurations:

| File              | Purpose                                                             |
| ----------------- | ------------------------------------------------------------------- |
| `providers.yaml`  | Defines provider test parameters, regions, VM sizes, test commands  |
| `extensions.yaml` | Defines extension test commands, validation patterns, test projects |

These configs serve as **reference documentation** for test parameters and are
partially consumed by workflows. Workflows may also use inline test logic for
simpler cases.

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

| Provider            | Required Secrets                                            |
| ------------------- | ----------------------------------------------------------- |
| Docker              | None (local)                                                |
| Fly.io              | `FLY_API_TOKEN`                                             |
| DevPod AWS          | `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`                |
| DevPod GCP          | `GCP_SERVICE_ACCOUNT_KEY`                                   |
| DevPod Azure        | `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET`, `AZURE_TENANT_ID` |
| DevPod DigitalOcean | `DIGITALOCEAN_TOKEN`                                        |
| Kubernetes          | `KUBECONFIG`                                                |

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
