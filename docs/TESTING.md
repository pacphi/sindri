# Testing Guide

Comprehensive guide to testing in Sindri.

## Test Philosophy

Sindri uses multiple layers of testing:

1. **Static Analysis** - Linting and schema validation
2. **Unit Tests** - Individual component testing
3. **Integration Tests** - End-to-end workflow testing
4. **Extension Tests** - Per-extension validation
5. **Infrastructure Tests** - Provider deployment testing

## Quick Test Commands

```bash
# Run all tests
pnpm test

# Run all validations (linting)
pnpm validate

# Specific test suites
pnpm test:unit              # Unit tests
pnpm test:integration       # Integration tests
pnpm test:extensions        # Extension tests

# Specific linting
pnpm lint                   # All linting
pnpm lint:yaml              # YAML linting
pnpm lint:shell             # Shell script linting
pnpm lint:md                # Markdown linting

# Formatting
pnpm format                 # Format all files
pnpm format:md              # Format markdown only
```

## Static Analysis

### YAML Validation

**Tool:** yamllint

```bash
pnpm lint:yaml
```

**Configuration:** `.yamllint.yml`

**Validates:**

- Indentation (2 spaces)
- Line length
- Trailing spaces
- Document structure

**Schema Validation:**

```bash
# Validate extension against schema
./cli/extension-manager validate <extension-name>

# Validate sindri.yaml
./cli/sindri config validate
```

**Schemas:**

- `docker/lib/schemas/extension.schema.json`
- `docker/lib/schemas/manifest.schema.json`
- `docker/lib/schemas/sindri.schema.json`

### Shell Script Validation

**Tool:** shellcheck

```bash
pnpm lint:shell
```

**Strictness:** Warning level (`-S warning`)

**Common issues:**

- Unquoted variables
- Missing error handling
- Unsafe file operations

**Fix automatically:**

```bash
# shellcheck suggests fixes in output
shellcheck -f diff script.sh | patch
```

### Markdown Validation

**Tool:** markdownlint

```bash
pnpm lint:md
```

**Configuration:** `.markdownlint.json`

**Common issues:**

- Missing blank lines
- Inconsistent heading levels
- Bare URLs (should be in brackets)

**Fix automatically:**

```bash
pnpm format:md
```

## Unit Tests

### Running Unit Tests

```bash
pnpm test:unit
```

### Test Structure

Located in `test/unit/`:

```
test/unit/
├── extension-manager/
│   ├── dependency-resolution.test.sh
│   ├── manifest-management.test.sh
│   └── validation.test.sh
├── adapters/
│   ├── docker-adapter.test.sh
│   └── fly-adapter.test.sh
└── common/
    └── utilities.test.sh
```

### Writing Unit Tests

Use bash testing framework (bats or simple assertions):

```bash
#!/usr/bin/env bash
set -euo pipefail

# Test dependency resolution
test_dependency_resolution() {
    local result
    result=$(resolve_dependencies "nodejs")
    assert_contains "$result" "workspace-structure"
}

# Test manifest update
test_manifest_update() {
    update_manifest "nodejs" "1.0.0"
    assert_file_exists "/workspace/.system/manifest/nodejs.yaml"
}

# Run tests
test_dependency_resolution
test_manifest_update
echo "All tests passed"
```

## Integration Tests

### Running Integration Tests

```bash
pnpm test:integration
```

### Test Scenarios

Integration tests validate end-to-end workflows:

1. **Extension Installation Flow:**
   - Install extension
   - Verify installation
   - Check manifest
   - Validate tool availability

2. **Dependency Resolution:**
   - Install extension with dependencies
   - Verify dependency chain
   - Check installation order

3. **Provider Deployment:**
   - Generate provider config
   - Validate config format
   - Test deployment (dry-run)

### Integration Test Structure

Located in `.github/scripts/`:

```
.github/scripts/
├── lib/
│   ├── assertions.sh       # Test assertion helpers
│   └── test-helpers.sh     # Test utilities
├── extensions/
│   └── test-extension-complete.sh
└── test-all-extensions.sh
```

### Example Integration Test

```bash
#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "$0")/lib/test-helpers.sh"
source "$(dirname "$0")/lib/assertions.sh"

test_extension_install() {
    print_test "Testing extension installation"

    # Install extension
    extension-manager install nodejs

    # Assertions
    assert_command_exists "node"
    assert_command_exists "npm"
    assert_manifest_exists "nodejs"

    print_success "Extension installation test passed"
}

test_extension_install
```

## Extension Tests

### Running Extension Tests

Test all extensions:

```bash
pnpm test:extensions
```

Test specific extension:

```bash
./.github/scripts/test-all-extensions.sh nodejs
```

### Extension Test Matrix

Tests run for each extension:

1. **Schema Validation:**
   - extension.yaml validates against schema
   - Required fields present
   - Valid enum values

2. **Installation:**
   - Extension installs successfully
   - Dependencies resolved
   - No errors during installation

3. **Validation:**
   - Commands exist
   - Version patterns match
   - Files created

4. **BOM Generation:**
   - BOM file created
   - Contains expected tools
   - Valid BOM format

### Extension Test Output

```
Testing extension: nodejs
✓ Schema validation passed
✓ Dependency resolution passed
✓ Installation completed
✓ Command 'node' found (v22.0.0)
✓ Command 'npm' found (v10.0.0)
✓ Manifest created
✓ BOM generated
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✓ All tests passed for nodejs
```

## Infrastructure Tests

### Running Infrastructure Tests

Test provider deployments:

```bash
pnpm test:infrastructure
```

### Test Scenarios

1. **Docker Deployment:**
   - Build image
   - Start container
   - Install extensions
   - Validate environment

2. **Fly.io Deployment:**
   - Generate fly.toml
   - Validate configuration
   - Deploy to Fly.io (in CI)
   - Test SSH access

3. **DevPod Deployment:**
   - Generate devcontainer.json
   - Validate against spec
   - Test container build

### Infrastructure Test Matrix

Located in `.github/workflows/infrastructure-tests.yml`:

```yaml
strategy:
  matrix:
    provider: [docker, fly, devpod]
    profile: [minimal, fullstack]
```

## GitHub Actions CI/CD

### Workflow Overview

1. **validation.yml** - Code quality checks
   - yamllint
   - shellcheck
   - markdownlint

2. **integration.yml** - Main test orchestration
   - Unit tests
   - Integration tests
   - Extension tests

3. **per-extension-tests.yml** - Extension validation
   - Test each extension individually
   - Parallel execution
   - Detailed reports

4. **infrastructure-tests.yml** - Provider deployments
   - Test Docker, Fly.io, DevPod
   - Validate configurations
   - Deployment smoke tests

5. **extension-combinations.yml** - Profile testing
   - Test extension profiles
   - Validate dependency resolution
   - Check for conflicts

### CI Test Flow

```
┌─────────────────┐
│  Push to main   │
└────────┬────────┘
         │
         ├─> validation.yml
         │   ├─> yamllint
         │   ├─> shellcheck
         │   └─> markdownlint
         │
         ├─> integration.yml
         │   ├─> Unit tests
         │   ├─> Integration tests
         │   └─> Extension tests
         │
         ├─> per-extension-tests.yml
         │   ├─> Test nodejs
         │   ├─> Test python
         │   └─> ... (parallel)
         │
         └─> infrastructure-tests.yml
             ├─> Docker build & test
             ├─> Fly.io config validation
             └─> DevPod config validation
```

### Running CI Locally

Approximate CI environment locally:

```bash
# Validation checks
pnpm validate

# Unit + integration tests
pnpm test

# Extension tests
pnpm test:extensions

# Docker build test
pnpm build
docker run -it sindri:local extension-manager validate-all
```

## Test Helpers

### Assertion Functions

Located in `.github/scripts/lib/assertions.sh`:

```bash
assert_command_exists() {
    command -v "$1" >/dev/null 2>&1 || fail "Command not found: $1"
}

assert_file_exists() {
    [[ -f "$1" ]] || fail "File not found: $1"
}

assert_contains() {
    echo "$1" | grep -q "$2" || fail "String not found: $2"
}

assert_manifest_exists() {
    assert_file_exists "/workspace/.system/manifest/$1.yaml"
}
```

### Test Helpers

Located in `.github/scripts/lib/test-helpers.sh`:

```bash
print_test() {
    echo "━━━ TEST: $1"
}

setup_test_workspace() {
    export WORKSPACE="/tmp/sindri-test-$$"
    mkdir -p "$WORKSPACE"
}

cleanup_test_workspace() {
    rm -rf "$WORKSPACE"
}

run_in_container() {
    docker run --rm sindri:local "$@"
}
```

## Test Coverage

### Current Coverage

- **Extensions:** 100% (all extensions have tests)
- **Core modules:** ~80% (main paths covered)
- **Adapters:** ~70% (provider-specific logic)

### Coverage Goals

- Increase core module coverage to 90%
- Add negative test cases
- Expand edge case testing

## Debugging Tests

### Enable Debug Output

```bash
# Enable debug mode
export DEBUG=true

# Run tests
pnpm test

# Or for specific test
DEBUG=true ./.github/scripts/test-all-extensions.sh nodejs
```

### Verbose Output

```bash
# Verbose shellcheck
shellcheck -f tty script.sh

# Verbose yamllint
yamllint -f parsable file.yaml
```

### Test in Docker

Run tests inside Docker container:

```bash
# Build image
pnpm build

# Run test inside container
docker run -it sindri:local bash
extension-manager install nodejs
extension-manager validate nodejs
```

## Continuous Testing

### Pre-Commit Hook

Install pre-commit hook:

```bash
# .git/hooks/pre-commit
#!/usr/bin/env bash
pnpm validate || exit 1
pnpm test:unit || exit 1
```

Make executable:

```bash
chmod +x .git/hooks/pre-commit
```

### Watch Mode (Local Development)

Watch files and re-run tests:

```bash
# Using watchexec or similar
watchexec -e sh,yaml,md pnpm validate
```

## Performance Testing

### Extension Install Time

Measure extension installation time:

```bash
time extension-manager install nodejs
```

### Docker Build Time

Measure image build time:

```bash
time pnpm build
```

### Startup Time

Measure container startup time:

```bash
time docker run --rm sindri:local echo "ready"
```

## Best Practices

1. **Test Before Push:**

   ```bash
   pnpm validate && pnpm test
   ```

2. **Write Tests for New Features:**
   - Add unit tests for new functions
   - Add integration tests for workflows
   - Add extension tests for new extensions

3. **Keep Tests Fast:**
   - Use mocks where appropriate
   - Parallelize independent tests
   - Cache test dependencies

4. **Meaningful Assertions:**
   - Test behavior, not implementation
   - Assert expected outcomes
   - Include failure messages

5. **Clean Up Resources:**
   - Remove test files after tests
   - Clean up Docker containers
   - Reset state between tests

## Related Documentation

- [Contributing Guide](CONTRIBUTING.md)
- [Extension Authoring](EXTENSION_AUTHORING.md)
- [Architecture](ARCHITECTURE.md)
