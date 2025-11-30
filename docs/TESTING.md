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
./test/unit/yaml/run-all-yaml-tests.sh

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
./cli/sindri test --config examples/fly/minimal.sindri.yaml --suite smoke

# Test all examples in a directory
./cli/sindri test --config examples/fly/ --suite smoke

# Validate configuration before testing
./cli/sindri config validate --config examples/fly/minimal.sindri.yaml
```

### Test Suites

| Suite         | Purpose                                 | Duration |
| ------------- | --------------------------------------- | -------- |
| `smoke`       | Basic connectivity and health checks    | Fast     |
| `integration` | Extension validation and functionality  | Medium   |
| `full`        | All tests including smoke + integration | Slow     |

## Static Analysis

### YAML Validation

The new YAML validation system provides comprehensive checks:

```bash
# Run all YAML validation tests
./test/unit/yaml/run-all-yaml-tests.sh
```

**Individual YAML Tests:**

| Script                      | Purpose                                      |
| --------------------------- | -------------------------------------------- |
| `test-extension-schemas.sh` | Validate extension.yaml files against schema |
| `test-profile-schema.sh`    | Validate profiles.yaml                       |
| `test-registry-schema.sh`   | Validate registry.yaml                       |
| `test-categories-schema.sh` | Validate categories.yaml                     |
| `test-templates-schema.sh`  | Validate project-templates.yaml              |
| `test-sindri-examples.sh`   | Validate all sindri.yaml examples            |
| `test-cross-references.sh`  | Validate cross-file references               |
| `test-yaml-lint.sh`         | Run yamllint on all YAML files               |

**Quality Checks:**

| Script                           | Purpose                                  |
| -------------------------------- | ---------------------------------------- |
| `test-extension-completeness.sh` | Verify extensions have required files    |
| `test-profile-dependencies.sh`   | Check dependency ordering                |
| `test-description-quality.sh`    | Check for placeholder/short descriptions |
| `test-naming-consistency.sh`     | Verify naming conventions                |

**Schemas:**

- `docker/lib/schemas/extension.schema.json` - Extension definitions
- `docker/lib/schemas/sindri.schema.json` - Sindri configurations
- `docker/lib/schemas/profiles.schema.json` - Profile definitions
- `docker/lib/schemas/registry.schema.json` - Extension registry
- `docker/lib/schemas/categories.schema.json` - Category definitions
- `docker/lib/schemas/project-templates.schema.json` - Project templates

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

| Workflow                 | Purpose                                                      |
| ------------------------ | ------------------------------------------------------------ |
| `ci.yml`                 | Main CI orchestrator - validation, build, unified testing    |
| `validate-yaml.yml`      | Comprehensive YAML validation                                |
| `test-sindri-config.yml` | Config-driven testing (discovers examples)                   |
| `deploy-sindri.yml`      | Reusable deployment workflow                                 |
| `teardown-sindri.yml`    | Reusable cleanup workflow                                    |
| `test-provider.yml`      | Full test suite per provider (CLI + extensions + integration)|
| `release.yml`            | Release automation                                           |

### CI Test Flow (Unified Provider Testing)

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
         └─> test-providers (matrix: each provider gets FULL test coverage)
             │
             FOR EACH provider in [docker, fly, devpod-aws, devpod-do, etc.]:
             │
             ├─> Phase 1: Deploy infrastructure
             │
             ├─> Phase 2: CLI tests (sindri, extension-manager)
             │
             ├─> Phase 3: Extension tests (validate, install profile)
             │
             ├─> Phase 4: Run test suites (smoke, integration, full)
             │
             └─> Phase 5: Cleanup
```

**Key Change**: CLI and extension tests now run on EACH selected provider, not just Docker.
This ensures consistent test coverage and catches provider-specific issues.

### Testing with Examples

The `test-sindri-config.yml` workflow discovers and tests all examples:

```yaml
# Run via workflow_dispatch
config-path: examples/fly/ # Test all Fly.io examples
test-suite: smoke # Test suite to run
skip-cleanup: false # Cleanup after tests
```

### Running CI Locally

```bash
# Validation checks
pnpm validate

# YAML validation
./test/unit/yaml/run-all-yaml-tests.sh

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
./cli/sindri config validate --config examples/fly/minimal.sindri.yaml
```

### Run Tests

```bash
# Smoke test (basic connectivity)
./cli/sindri test --config examples/fly/minimal.sindri.yaml --suite smoke

# Integration test (full extension validation)
./cli/sindri test --config examples/fly/minimal.sindri.yaml --suite integration

# Full test suite
./cli/sindri test --config examples/fly/minimal.sindri.yaml --suite full
```

### Deploy for Manual Testing

```bash
# Deploy
./cli/sindri deploy --config examples/fly/minimal.sindri.yaml

# Connect
./cli/sindri connect --config examples/fly/minimal.sindri.yaml

# Teardown
./cli/sindri destroy --config examples/fly/minimal.sindri.yaml --force
```

## Unit Tests

### Running Unit Tests

```bash
pnpm test:unit
```

### Test Structure

```text
test/unit/
├── yaml/                          # YAML validation tests
│   ├── run-all-yaml-tests.sh      # Master test runner
│   ├── test-extension-schemas.sh
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
./cli/extension-manager validate nodejs
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
   pnpm validate && ./test/unit/yaml/run-all-yaml-tests.sh && pnpm test
   ```

2. **Add Example Configs for New Scenarios:**
   - Create a new `sindri.yaml` in `examples/`
   - The CI will automatically discover and test it

3. **Validate YAML Changes:**
   - Run `./test/unit/yaml/run-all-yaml-tests.sh` after YAML changes
   - Check cross-references if modifying registry/profiles

4. **Keep Tests Fast:**
   - Use `--suite smoke` for quick validation
   - Use `--suite full` for comprehensive testing

5. **Clean Up Resources:**
   - Always use `--force` with destroy in automated scripts
   - CI workflows handle cleanup automatically

## Debugging Tests

### Enable Debug Output

```bash
export DEBUG=true
./cli/sindri test --config examples/fly/minimal.sindri.yaml --suite smoke
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
- [Extension Authoring](EXTENSION_AUTHORING.md)
- [Contributing Guide](CONTRIBUTING.md)
