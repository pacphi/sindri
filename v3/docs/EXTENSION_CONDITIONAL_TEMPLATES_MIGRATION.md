# Migrating to Conditional Template Selection

This guide helps extension authors migrate from bash-based environment detection to declarative YAML template conditions.

## Table of Contents

- [Overview](#overview)
- [Benefits of Migration](#benefits-of-migration)
- [Migration Steps](#migration-steps)
- [Before & After Examples](#before--after-examples)
- [Condition Syntax Reference](#condition-syntax-reference)
- [Testing Your Migration](#testing-your-migration)
- [Common Patterns](#common-patterns)
- [Troubleshooting](#troubleshooting)

## Overview

**Old approach**: Extensions used bash scripts (`install.sh`) with environment variable checks to select different templates:

```bash
if [[ "${CI:-}" == "true" ]]; then
  cp templates/ci-config.yml ~/config.yml
else
  cp templates/local-config.yml ~/config.yml
fi
```

**New approach**: Extensions use declarative YAML conditions in `extension.yaml`:

```yaml
configure:
  templates:
    - source: templates/local-config.yml
      destination: ~/config.yml
      condition:
        env: { CI: { not_equals: "true" } }

    - source: templates/ci-config.yml
      destination: ~/config.yml
      condition:
        env: { CI: { equals: "true" } }
```

## Benefits of Migration

✅ **Declarative**: Template selection logic is visible in extension.yaml
✅ **Type-safe**: Rust validates conditions at runtime
✅ **Testable**: Unit and integration tests for condition evaluation
✅ **Maintainable**: Easier to understand and modify than bash scripts
✅ **Documented**: Conditions self-document template selection logic
✅ **Consistent**: Same syntax across all extensions

## Migration Steps

### Step 1: Identify Current Logic

Find bash conditionals in your `install.sh` or other installation scripts:

```bash
# Example patterns to search for:
grep -n 'if.*CI' install.sh
grep -n 'if.*GITHUB_ACTIONS' install.sh
grep -n 'if.*uname' install.sh
grep -n 'if.*arch' install.sh
```

Common patterns:

- CI detection: `if [[ "${CI:-}" == "true" ]]`
- Platform detection: `if [[ "$(uname)" == "Linux" ]]`
- Architecture: `if [[ "$(uname -m)" == "x86_64" ]]`
- Environment check: `if [[ -n "${NVIDIA_VISIBLE_DEVICES}" ]]`

### Step 2: Map to Condition Syntax

Convert bash logic to YAML conditions:

| Bash Pattern                      | YAML Condition                        |
| --------------------------------- | ------------------------------------- |
| `[[ "${CI}" == "true" ]]`         | `env: { CI: "true" }`                 |
| `[[ "${CI:-}" != "true" ]]`       | `env: { CI: { not_equals: "true" } }` |
| `[[ -n "${VAR}" ]]`               | `env: { VAR: { exists: true } }`      |
| `[[ -z "${VAR}" ]]`               | `env: { VAR: { exists: false } }`     |
| `[[ "$(uname)" == "Linux" ]]`     | `platform: { os: ["linux"] }`         |
| `[[ "$(uname -m)" == "x86_64" ]]` | `platform: { arch: ["x86_64"] }`      |

### Step 3: Update extension.yaml

Add `condition` fields to your template configurations:

```yaml
configure:
  templates:
    # Template for local development
    - source: config/local.yml
      destination: ~/.myapp/config.yml
      mode: overwrite
      condition:
        env:
          not_any:
            - CI: "true"
            - GITHUB_ACTIONS: "true"

    # Template for CI environment
    - source: config/ci.yml
      destination: ~/.myapp/config.yml
      mode: overwrite
      condition:
        env:
          any:
            - CI: "true"
            - GITHUB_ACTIONS: "true"
```

### Step 4: (Optional) Simplify or Remove Bash Scripts

After verifying the migration works, you can:

1. **Remove conditional logic** from bash scripts
2. **Simplify scripts** to only handle non-template tasks
3. **Delete scripts entirely** if only used for template selection

**Important**: Keep bash scripts if they do more than template selection (e.g., downloading binaries, building from source).

### Step 5: Test Both Environments

```bash
# Test local mode (CI not set)
unset CI GITHUB_ACTIONS
sindri extension install <your-extension>
cat ~/.myapp/config.yml  # Verify local template was used

# Test CI mode
export CI=true
sindri extension install <your-extension> --force
cat ~/.myapp/config.yml  # Verify CI template was used
```

## Before & After Examples

### Example 1: claude-marketplace (CI vs Local)

**Before** (`install.sh` + `extension.yaml`):

```bash
# install.sh
if [[ "${CI:-}" == "true" ]] || [[ "${GITHUB_ACTIONS:-}" == "true" ]]; then
  YAML_FILE="${EXTENSION_DIR}/templates/marketplaces.ci.yml.example"
else
  YAML_FILE="${EXTENSION_DIR}/templates/marketplaces.yml.example"
fi

mkdir -p "${HOME}/config"
cp "${YAML_FILE}" "${HOME}/config/marketplaces.yml"
```

```yaml
# extension.yaml (old)
install:
  method: script
  script:
    path: install.sh
```

**After** (`extension.yaml` only):

```yaml
# extension.yaml (new)
install:
  method: mise
  mise:
    configFile: templates/mise.toml

configure:
  templates:
    # Local environment gets full marketplace list
    - source: templates/marketplaces.yml.example
      destination: ~/config/marketplaces.yml
      mode: overwrite
      condition:
        env:
          not_any:
            - CI: "true"
            - GITHUB_ACTIONS: "true"

    # CI environment gets minimal marketplace list
    - source: templates/marketplaces.ci.yml.example
      destination: ~/config/marketplaces.yml
      mode: overwrite
      condition:
        env:
          any:
            - CI: "true"
            - GITHUB_ACTIONS: "true"
```

**Benefits**: No bash script needed, template selection is declarative.

### Example 2: Platform-Specific Configuration

**Before** (`install.sh`):

```bash
# install.sh
OS="$(uname)"

if [[ "${OS}" == "Linux" ]]; then
  cp templates/linux-config.sh ~/.myapp/config.sh
elif [[ "${OS}" == "Darwin" ]]; then
  cp templates/macos-config.sh ~/.myapp/config.sh
elif [[ "${OS}" =~ MINGW|MSYS|CYGWIN ]]; then
  cp templates/windows-config.ps1 ~/.myapp/config.ps1
fi
```

**After** (`extension.yaml`):

```yaml
configure:
  templates:
    - source: templates/linux-config.sh
      destination: ~/.myapp/config.sh
      mode: overwrite
      condition:
        platform:
          os: ["linux"]

    - source: templates/macos-config.sh
      destination: ~/.myapp/config.sh
      mode: overwrite
      condition:
        platform:
          os: ["macos"]

    - source: templates/windows-config.ps1
      destination: ~/.myapp/config.ps1
      mode: overwrite
      condition:
        platform:
          os: ["windows"]
```

### Example 3: GPU-Aware Configuration

**Before** (`install.sh`):

```bash
# install.sh
if [[ -n "${NVIDIA_VISIBLE_DEVICES}" ]] || command -v nvidia-smi &>/dev/null; then
  cp templates/gpu-config.toml ~/.ollama/config.toml
else
  cp templates/cpu-config.toml ~/.ollama/config.toml
fi
```

**After** (`extension.yaml`):

```yaml
configure:
  templates:
    - source: templates/gpu-config.toml
      destination: ~/.ollama/config.toml
      mode: overwrite
      condition:
        env:
          NVIDIA_VISIBLE_DEVICES: { exists: true }

    - source: templates/cpu-config.toml
      destination: ~/.ollama/config.toml
      mode: overwrite
      condition:
        env:
          NVIDIA_VISIBLE_DEVICES: { exists: false }
```

**Note**: Command-based checks (`command -v nvidia-smi`) can't be directly converted. Keep bash for complex detection or use environment variables as proxies.

## Condition Syntax Reference

### Environment Variable Conditions

**Simple key-value matching**:

```yaml
condition:
  env:
    CI: "true"
    DEPLOY_ENV: "production"
```

**Complex expressions**:

```yaml
condition:
  env:
    CI:
      equals: "true" # Exact match
    BUILD_ENV:
      not_equals: "local" # Not equal
    API_KEY:
      exists: true # Variable must exist
    WORKSPACE:
      matches: "^/home/.*/workspace$" # Regex pattern
    DEPLOY_ENV:
      in_list: ["staging", "production"] # Must be in list
```

**Logical operators**:

```yaml
condition:
  env:
    any: # OR logic
      - CI: "true"
      - GITHUB_ACTIONS: "true"
```

```yaml
condition:
  env:
    all: # AND logic
      - CI: "true"
      - DEPLOY_ENV: "production"
```

```yaml
condition:
  env:
    not_any: # NOR logic
      - CI: "true"
      - GITHUB_ACTIONS: "true"
```

### Platform Conditions

**Operating system**:

```yaml
condition:
  platform:
    os: ["linux"]         # Single OS
    os: ["linux", "macos"]  # Multiple OS options
```

**Architecture**:

```yaml
condition:
  platform:
    arch: ["x86_64"]          # 64-bit Intel/AMD
    arch: ["aarch64", "arm64"]  # ARM 64-bit
```

**Combined**:

```yaml
condition:
  platform:
    os: ["linux"]
    arch: ["x86_64", "aarch64"] # Linux on either architecture
```

**Supported values**:

- OS: `linux`, `macos`, `windows`
- Arch: `x86_64`, `aarch64`, `arm64`

### Combining Conditions

**Template-level operators**:

```yaml
# All conditions must match (AND)
condition:
  all:
    - env: { CI: "true" }
    - platform: { os: ["linux"] }

# At least one condition must match (OR)
condition:
  any:
    - env: { CI: "true" }
    - env: { GITHUB_ACTIONS: "true" }

# Invert condition (NOT)
condition:
  not:
    env: { CI: "true" }
```

**Nested combinations**:

```yaml
# Complex logic: (CI=true OR GITHUB_ACTIONS=true) AND os=linux
condition:
  all:
    - any:
        - env: { CI: "true" }
        - env: { GITHUB_ACTIONS: "true" }
    - platform: { os: ["linux"] }
```

## Testing Your Migration

### Unit Testing Strategy

Create test scenarios in different environments:

```bash
# Test 1: Local environment
unset CI GITHUB_ACTIONS
sindri extension install <extension>
# Verify: Local-specific template was used

# Test 2: CI environment
export CI=true
sindri extension install <extension> --force
# Verify: CI-specific template was used

# Test 3: Platform-specific
# Test on actual platform or in Docker container
docker run --rm -v $(pwd):/workspace ubuntu:latest bash -c "cd /workspace && sindri extension install <extension>"
```

### Integration Testing

1. **Create test extension**:

```yaml
# test-extension.yaml
metadata:
  name: test-conditional
  version: 1.0.0
  description: Test conditional templates
  category: testing

install:
  method: script
  script:
    path: install.sh
    args: []

configure:
  templates:
    - source: local.txt
      destination: ~/test-output.txt
      mode: overwrite
      condition:
        env:
          not_any:
            - CI: "true"

    - source: ci.txt
      destination: ~/test-output.txt
      mode: overwrite
      condition:
        env:
          CI: "true"
```

2. **Test both modes**:

```bash
# Local mode
unset CI
sindri extension install test-conditional
cat ~/test-output.txt  # Should show "local" content

# CI mode
export CI=true
sindri extension install test-conditional --force
cat ~/test-output.txt  # Should show "CI" content
```

### Verification Checklist

- [ ] Templates without conditions still work (backwards compatibility)
- [ ] Correct template selected in local mode (CI not set)
- [ ] Correct template selected in CI mode (CI=true)
- [ ] Platform-specific templates work on target platforms
- [ ] Complex conditions (any/all/not) evaluate correctly
- [ ] Install logs show which templates were skipped
- [ ] No bash scripts needed for template selection

## Common Patterns

### Pattern 1: Skip Entire Configure in CI

```yaml
configure:
  templates:
    # All templates skip in CI
    - source: config.yml
      destination: ~/.myapp/config.yml
      condition:
        env:
          CI: { not_equals: "true" }

    - source: data.json
      destination: ~/.myapp/data.json
      condition:
        env:
          CI: { not_equals: "true" }
```

**Better approach**: Use `any` or `all` to avoid repetition:

```yaml
# Use separate extension variants for CI vs local,
# or keep templates simple and use bash for full skipping
```

### Pattern 2: Development vs Production

```yaml
configure:
  templates:
    # Development config
    - source: config.dev.yml
      destination: ~/.myapp/config.yml
      condition:
        env:
          DEPLOY_ENV: { in_list: ["dev", "development", "local"] }

    # Staging config
    - source: config.staging.yml
      destination: ~/.myapp/config.yml
      condition:
        env:
          DEPLOY_ENV: "staging"

    # Production config
    - source: config.prod.yml
      destination: ~/.myapp/config.yml
      condition:
        env:
          DEPLOY_ENV: { in_list: ["prod", "production"] }
```

### Pattern 3: Feature Flags

```yaml
configure:
  templates:
    # Enable experimental features
    - source: experimental-config.yml
      destination: ~/.myapp/config.yml
      condition:
        env:
          ENABLE_EXPERIMENTAL: "true"

    # Standard config
    - source: standard-config.yml
      destination: ~/.myapp/config.yml
      condition:
        env:
          ENABLE_EXPERIMENTAL: { not_equals: "true" }
```

### Pattern 4: Architecture-Specific Binaries

```yaml
install:
  method: binary
  binary:
    downloads:
      - name: myapp
        source:
          type: github-release
          url: "owner/repo"
          asset: "myapp-linux-x86_64.tar.gz"
        destination: ~/.local/bin/myapp
        extract: true

configure:
  templates:
    # x86_64-specific config
    - source: config-x86.yml
      destination: ~/.myapp/config.yml
      condition:
        platform:
          arch: ["x86_64"]

    # ARM-specific config
    - source: config-arm.yml
      destination: ~/.myapp/config.yml
      condition:
        platform:
          arch: ["aarch64", "arm64"]
```

## Troubleshooting

### Problem: Template not being processed

**Symptoms**: Template file exists but is not copied to destination.

**Diagnosis**:

```bash
# Enable debug logging
export RUST_LOG=debug
sindri extension install <extension>

# Look for lines like:
# "Skipping template ... (condition not met)"
```

**Solutions**:

1. Check condition syntax in YAML
2. Verify environment variables are set correctly
3. Test condition in isolation (create minimal test extension)
4. Remove condition temporarily to verify template processing works

### Problem: Wrong template selected

**Symptoms**: CI template used in local mode or vice versa.

**Diagnosis**:

```bash
# Check environment variables
env | grep -E '(CI|GITHUB_ACTIONS)'

# Verify condition logic
# Use debug logging to see which condition matched
```

**Solutions**:

1. Ensure mutually exclusive conditions (use `not_any` for negation)
2. Check for leftover environment variables from previous tests
3. Verify template processing order (first match wins)

### Problem: Condition syntax errors

**Symptoms**: Extension install fails with YAML parsing error.

**Diagnosis**:

```bash
# Validate YAML syntax
yamllint extension.yaml

# Check for common issues:
# - Incorrect indentation
# - Missing quotes around values
# - Typos in field names
```

**Solutions**:

1. Use YAML validator
2. Check examples in this guide
3. Reference ADR 033 for complete syntax
4. Test with minimal condition first, then add complexity

### Problem: Platform detection incorrect

**Symptoms**: Wrong OS/architecture template selected.

**Diagnosis**:

```bash
# Check detected platform
rustc -vV | grep host

# Rust uses these mappings:
# - x86_64-unknown-linux-gnu → os: "linux", arch: "x86_64"
# - x86_64-apple-darwin → os: "macos", arch: "x86_64"
# - x86_64-pc-windows-msvc → os: "windows", arch: "x86_64"
# - aarch64-apple-darwin → os: "macos", arch: "aarch64"
```

**Solutions**:

1. Use correct platform values: `linux`, `macos`, `windows`
2. Use correct architecture values: `x86_64`, `aarch64`, `arm64`
3. Test on actual target platform
4. Consider using Docker for platform-specific testing

## Migration Checklist

Use this checklist to track your migration progress:

- [ ] Identified all bash conditionals in install scripts
- [ ] Mapped bash logic to YAML conditions
- [ ] Updated `extension.yaml` with conditional templates
- [ ] Tested in local mode (CI not set)
- [ ] Tested in CI mode (CI=true)
- [ ] Tested on target platforms (if platform-specific)
- [ ] Verified install logs show correct template selection
- [ ] Removed or simplified bash scripts (if applicable)
- [ ] Updated extension documentation
- [ ] Committed changes to version control

## Additional Resources

- [ADR 033: Environment-Based Template Selection](architecture/adr/033-environment-based-template-selection.md)
- [ADR 032: Extension Configure Processing](architecture/adr/032-extension-configure-processing.md)
- [Extension Schema Documentation](SCHEMA.md)
- [Condition Evaluator Source](../crates/sindri-extensions/src/configure/conditions.rs)
- [Integration Tests](../crates/sindri-extensions/tests/configure_integration_tests.rs)

## Getting Help

If you encounter issues during migration:

1. Check this troubleshooting guide
2. Review integration tests for working examples
3. Open an issue on GitHub with:
   - Extension name and version
   - Condition YAML snippet
   - Error messages or unexpected behavior
   - Environment details (OS, CI/local, env vars)

---

**Happy migrating!** Remember: Start simple, test thoroughly, and iterate.
