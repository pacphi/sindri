# Testing Guide

Comprehensive guide to testing in Sindri.

## Test Philosophy

Sindri uses a **YAML-driven testing architecture** where:

1. **sindri.yaml is the single source of truth** - All provider-specific details live in configuration files
2. **Tests iterate over configuration files** - No provider logic in workflows
3. **End consumers pass sindri.yaml** - Deploy and teardown accept a config file
4. **Test fixtures are pre-defined configs** - The `examples/` directory covers all test scenarios
5. **All YAML files are validated** - Extensions, profiles, registry, categories, templates all have schema validation

## Quick Test Commands

```bash
# Run all tests
pnpm test

# Run all validations (linting)
pnpm validate

# Specific test suites
pnpm test:unit              # Unit tests (YAML validation)
pnpm test:extensions        # Extension validation tests

# YAML validation (new)
./../test/unit/yaml/run-all-yaml-tests.sh

# Specific linting
pnpm lint                   # All linting
pnpm lint:yaml              # YAML linting
pnpm lint:shell             # Shell script linting
pnpm lint:md                # Markdown linting

# Formatting
pnpm format                 # Format all files
pnpm format:md              # Format markdown only
```

## YAML-Driven Testing

### Test Examples as Fixtures

All test scenarios are defined as `sindri.yaml` files in the `examples/` directory:

```text
examples/
├── fly/
│   ├── minimal.sindri.yaml       # Basic Fly.io test
│   ├── fullstack.sindri.yaml     # Full profile test
│   └── regions/                  # Region-specific tests
├── docker/
│   ├── minimal.sindri.yaml       # Local Docker test
│   └── fullstack.sindri.yaml
├── devpod/
│   ├── aws/                      # AWS EC2 via DevPod
│   ├── gcp/                      # GCP via DevPod
│   ├── azure/                    # Azure via DevPod
│   ├── digitalocean/             # DigitalOcean via DevPod
│   └── kubernetes/               # K8s via DevPod
└── profiles/                     # Profile-specific tests
```

### Running Tests Against Examples

```bash
# Test a single configuration
../cli/sindri test --config examples/fly/minimal.sindri.yaml --level quick

# Test all examples in a directory
../cli/sindri test --config examples/fly/ --level quick

# Validate configuration before testing
../cli/sindri config validate --config examples/fly/minimal.sindri.yaml
```

### Test Levels

| Level       | Purpose                                        | Duration | Tests |
| ----------- | ---------------------------------------------- | -------- | ----- |
| `quick`     | CLI validation only                            | ~10-15s  | 7     |
| `extension` | Single extension lifecycle (install/remove)    | ~45-60s  | 11    |
| `profile`   | Profile lifecycle (install-profile/remove all) | ~90-120s | 11    |
| `all`       | All levels sequentially                        | ~2-3min  | 29    |

**Note**: Extension and profile levels include idempotency testing (reinstall + revalidate).

See [CI_WORKFLOW_IN_DEPTH.md](CI_WORKFLOW_IN_DEPTH.md) for detailed test specifications.

## Static Analysis

### YAML Validation

The new YAML validation system provides comprehensive checks:

```bash
# Run all YAML validation tests
./../test/unit/yaml/run-all-yaml-tests.sh
```

**Individual YAML Tests:**

| Script                        | Purpose                                      |
| ----------------------------- | -------------------------------------------- |
| `validate-schema.sh`          | Unified schema validation (all YAML schemas) |
| `test-cross-references.sh`    | Validate cross-file references               |
| `test-domain-requirements.sh` | Validate extension domain requirements       |
| `test-yaml-lint.sh`           | Run yamllint on all YAML files               |

**Quality Checks:**

| Script                           | Purpose                                  |
| -------------------------------- | ---------------------------------------- |
| `test-extension-completeness.sh` | Verify extensions have required files    |
| `test-profile-dependencies.sh`   | Check dependency ordering                |
| `test-description-quality.sh`    | Check for placeholder/short descriptions |
| `test-naming-consistency.sh`     | Verify naming conventions                |

**Schemas:**

- `../docker/lib/schemas/extension.schema.json` - Extension definitions
- `../docker/lib/schemas/sindri.schema.json` - Sindri configurations
- `../docker/lib/schemas/profiles.schema.json` - Profile definitions
- `../docker/lib/schemas/registry.schema.json` - Extension registry
- `../docker/lib/schemas/categories.schema.json` - Category definitions
- `../docker/lib/schemas/project-templates.schema.json` - Project templates
- `../docker/lib/schemas/vm-sizes.schema.json` - VM size mappings across providers

### Domain Requirements Validation

Extensions declare `requirements.domains` to list external domains accessed during
installation. The domain validation test (`test-domain-requirements.sh`) checks:

| Check      | Behavior     | Description                                 |
| ---------- | ------------ | ------------------------------------------- |
| Format     | Hard fail    | Valid hostname syntax (RFC 1123)            |
| Duplicates | Hard fail    | No duplicate domain entries                 |
| DNS        | Warning only | Domains resolve (optional, off by default)  |
| Undeclared | Warning only | Domains in scripts not declared (heuristic) |

**Running locally:**

```bash
# Via test script (format + duplicates only)
./../test/unit/yaml/test-domain-requirements.sh

# Via extension-manager with DNS check
extension-manager --check-dns validate-domains
```

**Environment variables:**

- `VALIDATE_DNS=true` - Enable DNS resolution checks
- `DNS_TIMEOUT=3` - DNS lookup timeout in seconds

### Shell Script Validation

**Tool:** shellcheck

```bash
pnpm lint:shell
```

**Strictness:** Warning level (`-S warning`)

### Markdown Validation

**Tool:** markdownlint

```bash
pnpm lint:md
```

## GitHub Actions CI/CD

### Workflow Overview

The CI system uses these workflows:

| Workflow              | Purpose                                                       |
| --------------------- | ------------------------------------------------------------- |
| `ci.yml`              | Main CI orchestrator - validation, build, unified testing     |
| `validate-yaml.yml`   | Comprehensive YAML validation                                 |
| `test-extensions.yml` | Registry-based extension testing (single, multiple, or all)   |
| `test-profiles.yml`   | Config-driven profile testing (discovers sindri.yaml files)   |
| `deploy-sindri.yml`   | Reusable deployment workflow                                  |
| `teardown-sindri.yml` | Reusable cleanup workflow                                     |
| `test-provider.yml`   | Full test suite per provider (CLI + extensions + integration) |
| `release.yml`         | Release automation                                            |

### CI Test Flow (Simplified)

```text
┌─────────────────┐
│  Push to main   │
└────────┬────────┘
         │
         ├─> shellcheck (shell validation)
         │
         ├─> markdownlint (markdown validation)
         │
         ├─> validate-yaml.yml
         │   ├─> YAML lint
         │   ├─> Schema validation
         │   ├─> Cross-references
         │   └─> Extension consistency
         │
         ├─> build (Docker image)
         │
         └─> test-providers (matrix: each provider tested)
             │
             FOR EACH provider in [docker, fly, devpod-k8s]:
             │
             ├─> Setup credentials
             │
             ├─> Deploy infrastructure
             │
             ├─> Run sindri-test.sh (ONE remote call)
             │   │
             │   └─> Executes INSIDE container:
             │       - Quick: CLI validation
             │       - Extension: Single extension lifecycle
             │       - Profile: Profile lifecycle
             │
             └─> Cleanup
```

**Key Simplification**: All tests run INSIDE the container via a single unified
script (`/docker/scripts/sindri-test.sh`), eliminating shell quoting issues and
reducing complexity from 2,400 lines to ~550 lines.

**CI Mode**: All provider deployments use `--ci-mode` flag to force `autoInstall=false`,
ensuring clean slate testing regardless of the sindri.yaml configuration. This allows
testing any profile/config combination without modifying files.

### Kubernetes Testing with Kind

The CI workflow supports testing against Kubernetes using DevPod with automatic cluster
bootstrapping. This provides fast feedback without requiring users to maintain external clusters.

**Behavior:**

| KUBECONFIG Secret | Result                                     |
| ----------------- | ------------------------------------------ |
| Not provided      | Automatically creates a local kind cluster |
| Provided          | Uses the external Kubernetes cluster       |

**To test with kind (default for CI):**

```yaml
# No KUBECONFIG secret needed - kind cluster is auto-created
providers: devpod-k8s
```

**To test with an external cluster:**

1. Add a `KUBECONFIG` secret to your repository containing the kubeconfig content
2. The workflow will detect it and use your external cluster instead of creating kind

**Kind cluster configuration:**

- Cluster name: `sindri-ci-<run-id>` (unique per workflow run)
- Kubernetes version: v1.32.0 (configurable via `k8s-kind-node-image`)
- Namespace: `sindri-test`
- Automatically cleaned up after tests complete

**Manual override:**

```yaml
# Force kind cluster creation even with KUBECONFIG present
k8s-use-kind: "true"

# Force external cluster (fails if no KUBECONFIG)
k8s-use-kind: "false"
```

### Example Configuration Used

CI uses `examples/devpod/kubernetes/minimal.sindri.yaml` for `devpod-k8s` tests.

**Why not `examples/k8s/`?**

The `examples/k8s/` folder contains configs that CREATE local clusters (kind/k3d)
then deploy via DevPod. CI handles cluster creation separately via the setup action,
so it uses the simpler `examples/devpod/kubernetes/` configs that assume an existing cluster.

| Directory                     | Purpose                              | When to Use           |
| ----------------------------- | ------------------------------------ | --------------------- |
| `examples/devpod/kubernetes/` | Deploy to existing K8s cluster       | CI, external clusters |
| `examples/k8s/`               | Create cluster + deploy (all-in-one) | Local development     |

**Manual local K8s testing:**

```bash
# Option 1: Use examples/k8s/ (creates cluster + deploys)
../cli/sindri deploy --config examples/k8s/kind-minimal.sindri.yaml

# Option 2: Create cluster yourself, then use devpod/kubernetes
kind create cluster --name my-cluster
../cli/sindri deploy --config examples/devpod/kubernetes/minimal.sindri.yaml
```

### Testing Profiles (Config-based)

The `test-profiles.yml` workflow discovers and tests sindri.yaml configuration files:

```yaml
# Run via workflow_dispatch
config-path: examples/fly/ # Test all Fly.io examples
test-level: quick # Test level (quick, profile, all)
skip-cleanup: false # Cleanup after tests
```

### Testing Extensions (Registry-based)

The `test-extensions.yml` workflow tests individual extensions directly from the registry:

```yaml
# Run via workflow_dispatch
extensions: nodejs           # Single extension
extensions: nodejs,python    # Multiple extensions (comma-separated)
extensions: all              # All non-protected extensions (~29)
skip-cleanup: false          # Cleanup after tests
```

**Key features:**

- **Docker-only**: Tests run locally in Docker containers (fast feedback)
- **Matrix execution**: Each extension runs as a separate job
- **Protected exclusion**: Base extensions (mise-config, github-cli) excluded from "all"
- **Max parallel**: 4 concurrent extension tests

### Running CI Locally

```bash
# Validation checks
pnpm validate

# YAML validation
./../test/unit/yaml/run-all-yaml-tests.sh

# Unit + integration tests
pnpm test

# Extension tests
pnpm test:extensions

# Docker build test
pnpm build
docker run -it sindri:local extension-manager validate-all
```

## Test CLI Commands

### Validate Configuration

```bash
# Validate against schema
../cli/sindri config validate --config examples/fly/minimal.sindri.yaml
```

### Run Tests

```bash
# Quick test (CLI validation)
../cli/sindri test --config examples/fly/minimal.sindri.yaml --level quick

# Extension lifecycle test (single extension)
../cli/sindri test --config examples/fly/minimal.sindri.yaml --level extension

# Profile lifecycle test (full profile)
../cli/sindri test --config examples/fly/minimal.sindri.yaml --level profile

# All tests
../cli/sindri test --config examples/fly/minimal.sindri.yaml --level all
```

### Deploy for Manual Testing

```bash
# Deploy
../cli/sindri deploy --config examples/fly/minimal.sindri.yaml

# Connect
../cli/sindri connect --config examples/fly/minimal.sindri.yaml

# Teardown
../cli/sindri destroy --config examples/fly/minimal.sindri.yaml --force
```

## Unit Tests

### Running Unit Tests

```bash
pnpm test:unit
```

### Test Structure

```text
../test/unit/
├── yaml/                          # YAML validation tests
│   ├── run-all-yaml-tests.sh      # Master test runner
│   ├── validate-schema.sh         # Unified schema validation
│   ├── test-cross-references.sh
│   └── ...
├── extension-manager/
│   ├── dependency-resolution.test.sh
│   └── validation.test.sh
└── common/
    └── utilities.test.sh
```

## Extension Tests

### Running Extension Tests

```bash
# Test all extensions
pnpm test:extensions

# Test specific extension
../cli/extension-manager validate nodejs
```

### Extension Test Matrix

Tests run for each extension:

1. **Schema Validation:** extension.yaml validates against schema
2. **Installation:** Extension installs successfully
3. **Validation:** Commands exist and version patterns match
4. **BOM Generation:** Bill of materials generated correctly

## Best Practices

1. **Test Before Push:**

   ```bash
   pnpm validate && ./../test/unit/yaml/run-all-yaml-tests.sh && pnpm test
   ```

2. **Add Example Configs for New Scenarios:**
   - Create a new `sindri.yaml` in `examples/`
   - The CI will automatically discover and test it

3. **Validate YAML Changes:**
   - Run `./../test/unit/yaml/run-all-yaml-tests.sh` after YAML changes
   - Check cross-references if modifying registry/profiles

4. **Keep Tests Fast:**
   - Use `--level quick` for fast CLI validation
   - Use `--level profile` for comprehensive testing

5. **Clean Up Resources:**
   - Always use `--force` with destroy in automated scripts
   - CI workflows handle cleanup automatically

## Debugging Tests

### Enable Debug Output

```bash
export DEBUG=true
../cli/sindri test --config examples/fly/minimal.sindri.yaml --level quick
```

### Test in Docker

```bash
pnpm build
docker run -it sindri:local bash
extension-manager install nodejs
extension-manager validate nodejs
```

## Related Documentation

- [Configuration Guide](CONFIGURATION.md)
- [Extension Authoring](../../docs/EXTENSION_AUTHORING.md)
- [Contributing Guide](../../docs/CONTRIBUTING.md)
