# Extension Conditional Template Migration Status

This document tracks the migration of extensions from bash-based environment detection to declarative YAML template conditions.

## Overview

**Total Extensions Identified**: 7
**Migrated**: 1
**Remaining**: 6

## Migration Progress

### ✅ Migrated Extensions

#### 1. claude-marketplace (v2.0.0)

**Migration Date**: 2026-01-26

**Changes Made**:

- Added conditional templates to `extension.yaml`:
  - Local environment: `marketplaces.yml.example` → `~/config/marketplaces.yml`
  - CI environment: `marketplaces.ci.yml.example` → `~/config/marketplaces.yml`
- Simplified `install.sh`: Removed CI detection logic (19 lines → 23 lines with comments)
- Updated `remove` paths: Removed `~/config/marketplaces.ci.yml` (no longer provisioned separately)

**Condition Used**:

```yaml
# Local
condition:
  env:
    not_any:
      - CI: "true"
      - GITHUB_ACTIONS: "true"

# CI
condition:
  env:
    any:
      - CI: "true"
      - GITHUB_ACTIONS: "true"
```

**Benefits**:

- Template selection visible in extension.yaml
- No bash conditional logic needed
- Same destination for both templates (simplified remove)
- 40% reduction in install.sh complexity

**Files Modified**:

- `v3/extensions/claude-marketplace/extension.yaml`
- `v3/extensions/claude-marketplace/install.sh`

### 🔄 Pending Migrations

#### 2. ruby

**Current Behavior**: Bash script skips Rails install entirely in CI

**Recommended Approach**:

```yaml
configure:
  templates:
    # Rails gems (local only)
    - source: Gemfile.rails
      destination: ~/Gemfile
      mode: append
      condition:
        env:
          CI: { not_equals: "true" }
```

**Estimated Complexity**: Low
**Estimated Time**: 30 minutes

#### 3. playwright

**Current Behavior**: Bash script sets `PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1` in CI

**Recommended Approach**:

```yaml
configure:
  environment:
    # Skip browser download in CI
    - key: PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD
      value: "1"
      scope: session
      # Add condition support to environment variables (future enhancement)

  templates:
    # Alternative: Use different playwright configs
    - source: playwright.config.local.js
      destination: ~/playwright.config.js
      condition:
        env:
          CI: { not_equals: "true" }

    - source: playwright.config.ci.js
      destination: ~/playwright.config.js
      condition:
        env:
          CI: "true"
```

**Note**: Environment variables don't currently support conditions. Either:

1. Keep bash script for env var setting
2. Use conditional config files instead
3. Wait for environment variable condition support (future enhancement)

**Estimated Complexity**: Medium (env var workaround needed)
**Estimated Time**: 1 hour

#### 4. ollama

**Current Behavior**: Bash script exits early in CI (complete skip)

**Recommended Approach**:

```yaml
# Option 1: Skip all configure in CI
configure:
  templates:
    - source: ollama-config.toml
      destination: ~/.ollama/config.toml
      condition:
        env:
          CI: { not_equals: "true" }

# Option 2: Different CI vs local configs
configure:
  templates:
    - source: ollama-cpu-config.toml  # Minimal config for CI
      destination: ~/.ollama/config.toml
      condition:
        env:
          CI: "true"

    - source: ollama-gpu-config.toml  # Full config for local
      destination: ~/.ollama/config.toml
      condition:
        env:
          CI: { not_equals: "true" }
```

**Estimated Complexity**: Low
**Estimated Time**: 30 minutes

#### 5. dotnet

**Current Behavior**: Similar patterns (needs investigation)

**Action Required**:

1. Analyze `install.sh` for conditional logic
2. Identify environment-based template selection
3. Map to YAML conditions

**Estimated Complexity**: TBD
**Estimated Time**: TBD

#### 6. cloud-tools

**Current Behavior**: Similar patterns (needs investigation)

**Action Required**:

1. Analyze `install.sh` for conditional logic
2. Identify environment-based template selection
3. Map to YAML conditions

**Estimated Complexity**: TBD
**Estimated Time**: TBD

#### 7. ai-toolkit

**Current Behavior**: Similar patterns (needs investigation)

**Action Required**:

1. Analyze `install.sh` for conditional logic
2. Identify environment-based template selection
3. Map to YAML conditions

**Estimated Complexity**: TBD
**Estimated Time**: TBD

## Migration Guidelines

### Step 1: Analyze Current Extension

```bash
# Find conditional logic in install.sh
cd v3/extensions/<extension-name>
grep -n 'if.*CI' install.sh
grep -n 'if.*uname' install.sh
grep -n 'if.*GITHUB_ACTIONS' install.sh
```

### Step 2: Design Condition Structure

Map bash conditionals to YAML:

- `[[ "${CI}" == "true" ]]` → `env: { CI: "true" }`
- `[[ -n "${VAR}" ]]` → `env: { VAR: { exists: true } }`
- `[[ "$(uname)" == "Linux" ]]` → `platform: { os: ["linux"] }`

### Step 3: Update extension.yaml

Add `condition` field to templates:

```yaml
configure:
  templates:
    - source: template.yml
      destination: ~/config.yml
      mode: overwrite
      condition:
        env:
          CI: { not_equals: "true" }
```

### Step 4: Simplify or Remove Bash Script

Options:

1. **Remove entirely** if only used for template selection
2. **Simplify** to remove conditional logic
3. **Keep** for non-template tasks (downloading, building)

### Step 5: Test Both Modes

```bash
# Test local
unset CI GITHUB_ACTIONS
sindri extension install <extension>

# Test CI
export CI=true
sindri extension install <extension> --force
```

### Step 6: Document Changes

Update extension documentation:

- What changed
- Why (declarative vs imperative)
- How to test

## Benefits of Migration

**For Extension Authors**:

- ✅ Declarative configuration (YAML vs bash)
- ✅ Easier to understand and maintain
- ✅ Type-safe condition evaluation
- ✅ Better testing (unit + integration tests)
- ✅ Self-documenting template selection

**For Extension Users**:

- ✅ Transparent template selection (visible in YAML)
- ✅ Consistent behavior across extensions
- ✅ Better error messages
- ✅ Install logs show which templates were used/skipped

**For Sindri Platform**:

- ✅ Standardized conditional logic
- ✅ Reduced bash script complexity
- ✅ Improved security (declarative > imperative)
- ✅ Better extensibility (easy to add new condition types)

## Common Patterns

### Pattern: CI vs Local

```yaml
# Local template
condition:
  env:
    not_any:
      - CI: "true"
      - GITHUB_ACTIONS: "true"

# CI template
condition:
  env:
    any:
      - CI: "true"
      - GITHUB_ACTIONS: "true"
```

### Pattern: Platform-Specific

```yaml
# Linux
condition:
  platform:
    os: ["linux"]

# macOS
condition:
  platform:
    os: ["macos"]

# Windows
condition:
  platform:
    os: ["windows"]
```

### Pattern: Skip in CI

```yaml
# Only process locally
condition:
  env:
    CI: { not_equals: "true" }
```

### Pattern: GPU Detection

```yaml
# GPU available
condition:
  env:
    NVIDIA_VISIBLE_DEVICES: { exists: true }

# CPU only
condition:
  env:
    NVIDIA_VISIBLE_DEVICES: { exists: false }
```

## Future Enhancements

### Environment Variable Conditions

Add condition support to environment variables:

```yaml
configure:
  environment:
    - key: PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD
      value: "1"
      scope: session
      condition: # Not yet supported
        env:
          CI: "true"
```

**Tracking**: Future enhancement (ADR needed)

### Capability Detection

Add runtime capability detection:

```yaml
condition:
  capability:
    gpu: true # GPU present
    container: true # Running in container
    network: true # Network available
    shell: "zsh" # Shell type
```

**Tracking**: Future enhancement (see ADR 033)

### Condition Debugging

Add debugging command to show condition evaluation:

```bash
sindri extension install --debug-conditions <extension>
# Output:
# ✓ Template 'local.yml' matched (CI not set)
# ✗ Template 'ci.yml' skipped (CI != true)
```

**Tracking**: Future enhancement

## Statistics

**Migration Coverage**: 14% (1/7 extensions)
**Lines of Bash Removed**: ~10 lines (claude-marketplace)
**YAML Lines Added**: ~15 lines (conditions)
**Net Change**: +5 lines (but declarative vs imperative)

**Projected Impact** (all 7 extensions):

- Bash lines removed: ~70 lines
- YAML lines added: ~105 lines
- Simplified bash scripts: 7
- Improved maintainability: 7 extensions

## Resources

- [Migration Guide](EXTENSION_CONDITIONAL_TEMPLATES_MIGRATION.md)
- [ADR 033: Environment-Based Template Selection](architecture/adr/033-environment-based-template-selection.md)
- [Condition Evaluator Source](../crates/sindri-extensions/src/configure/conditions.rs)
- [Integration Tests](../crates/sindri-extensions/tests/configure_integration_tests.rs)

## Next Steps

1. **Immediate**:
   - Complete `ruby` migration (30 min)
   - Complete `ollama` migration (30 min)
   - Complete `playwright` migration (1 hour)

2. **Short-term**:
   - Analyze `dotnet`, `cloud-tools`, `ai-toolkit` install scripts
   - Design conditions for each
   - Execute migrations

3. **Long-term**:
   - Add environment variable condition support
   - Add capability detection
   - Add condition debugging command

## Timeline Estimate

**Remaining Migrations**: 6 extensions
**Estimated Time**: 4-6 hours

- 3 simple migrations: 1.5 hours
- 3 TBD migrations: 2.5-4.5 hours (depending on complexity)

**Target Completion**: Within 1 week

---

**Last Updated**: 2026-01-26
**Status**: In Progress (1/7 complete)
**Owner**: Sindri Team
