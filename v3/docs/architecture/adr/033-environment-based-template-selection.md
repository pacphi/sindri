# ADR 033: Environment-Based Template Selection

## Status

Accepted

## Context

The configure system implemented in ADR 032 processes templates unconditionally - there's no way to declaratively select different templates based on environment context (CI vs local, OS, architecture, etc.).

### Impact

**Current Limitations:**
- 7 extensions use imperative bash scripts for environment detection
- Install scripts contain duplicate logic that should be declarative
- Templates are selected via `if [[ "$CI" == "true" ]]` in bash, not YAML
- Extensions provision BOTH templates then rely on script logic to select
- No standardized way to express "use template-A in CI, template-B locally"

**Affected Extensions:**
- `claude-marketplace`: Bash script selects between `marketplaces.yml` vs `marketplaces.ci.yml`
- `ruby`: Bash script skips Rails install entirely in CI
- `playwright`: Bash script sets `PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1` in CI
- `ollama`: Bash script exits early in CI (complete skip)
- `dotnet`, `cloud-tools`, `ai-toolkit`: Similar patterns

### Requirements

From extension analysis and v2 migration patterns:

**Declarative Conditions:**
- Environment variable matching (equals, not_equals, exists, regex)
- Platform detection (OS, architecture)
- Logical operators (any, all, not) for complex conditions
- Backwards compatible (templates without conditions always process)

**Use Cases:**
- **CI/Local Split**: Different templates for CI (minimal) vs local (full featured)
- **Platform-Specific**: Linux/macOS/Windows specific configurations
- **GPU Awareness**: Different configs for GPU vs CPU-only environments
- **Multi-Environment**: Development/staging/production template selection

## Decision

### Architecture

Add **conditional template selection** to the configure system, allowing extensions to declaratively specify template processing conditions.

```
configure/
├── mod.rs           # ConfigureProcessor orchestrator (updated)
├── templates.rs     # TemplateProcessor (updated for conditions)
├── environment.rs   # EnvironmentProcessor
├── path.rs          # PathResolver
└── conditions.rs    # ConditionEvaluator (NEW)
```

### Type System

**New types in `sindri-core/types/extension_types.rs`:**

```rust
pub struct TemplateConfig {
    pub source: String,
    pub destination: String,
    pub mode: TemplateMode,
    pub condition: Option<TemplateCondition>,  // NEW
}

pub struct TemplateCondition {
    pub env: Option<EnvCondition>,
    pub platform: Option<PlatformCondition>,
    pub any: Option<Vec<TemplateCondition>>,  // OR logic
    pub all: Option<Vec<TemplateCondition>>,  // AND logic
    pub not: Option<Box<TemplateCondition>>,  // NOT logic
}

pub enum EnvCondition {
    Simple(HashMap<String, String>),           // { CI: "true" }
    Complex(HashMap<String, EnvConditionExpr>), // { CI: { equals: "true" } }
    Logical(EnvConditionLogical),              // { any: [...], all: [...] }
}

pub struct EnvConditionExpr {
    pub equals: Option<String>,
    pub not_equals: Option<String>,
    pub exists: Option<bool>,
    pub matches: Option<String>,  // regex
    pub in_list: Option<Vec<String>>,
}

pub struct PlatformCondition {
    pub os: Option<Vec<String>>,    // ["linux", "macos", "windows"]
    pub arch: Option<Vec<String>>,  // ["x86_64", "aarch64", "arm64"]
}
```

### Design Principles

**1. Declarative Over Imperative**
- Replace bash script logic with YAML conditions
- Self-documenting: condition intent is clear from YAML
- Easier to validate and test

**2. Flexibility**
- Simple conditions for common cases: `{ CI: "true" }`
- Complex conditions for advanced cases: regex, logical operators
- Composable: combine env + platform + logical operators

**3. Backwards Compatibility**
- Templates without `condition` field always process (current behavior)
- Existing extensions continue to work unchanged
- No breaking changes to extension.yaml schema

**4. Early Evaluation**
- Conditions evaluated before template processing
- Skipped templates logged for transparency
- Failed condition evaluations return errors (don't silently skip)

### Implementation

**Condition Evaluator (`configure/conditions.rs`):**
```rust
pub struct ConditionEvaluator {
    platform_info: PlatformInfo,
}

impl ConditionEvaluator {
    pub fn evaluate(&self, condition: &TemplateCondition) -> Result<bool> {
        // Evaluate env, platform, and logical operators
    }
}
```

**Template Processor Integration:**
```rust
pub async fn process_template(
    &self,
    extension_name: &str,
    template: &TemplateConfig,
) -> Result<Option<TemplateResult>> {  // Changed to Option
    // NEW: Evaluate condition if present
    if let Some(condition) = &template.condition {
        let evaluator = ConditionEvaluator::new();
        if !evaluator.evaluate(condition)? {
            return Ok(None);  // Skip this template
        }
    }

    // Existing template processing...
}
```

**Orchestrator Handling:**
```rust
async fn process_templates(...) -> Result<Vec<TemplateResult>> {
    for template in templates {
        match processor.process_template(extension_name, template).await? {
            Some(result) => results.push(result),
            None => {
                // Template skipped due to condition
                tracing::info!("Template {} skipped", template.source);
            }
        }
    }
}
```

## Usage Examples

### Example 1: CI vs Local Template Selection

**Before (install.sh + extension.yaml):**
```bash
# install.sh
if [[ "${CI:-}" == "true" ]] || [[ "${GITHUB_ACTIONS:-}" == "true" ]]; then
  YAML_FILE="marketplaces.ci.yml"
else
  YAML_FILE="marketplaces.yml"
fi
```

**After (extension.yaml only):**
```yaml
configure:
  templates:
    # Local environment gets full marketplace list
    - source: marketplaces.yml.example
      destination: ~/config/marketplaces.yml
      mode: overwrite
      condition:
        env:
          not_any:
            - CI: "true"
            - GITHUB_ACTIONS: "true"

    # CI environment gets minimal marketplace list
    - source: marketplaces.ci.yml.example
      destination: ~/config/marketplaces.yml  # Same destination!
      mode: overwrite
      condition:
        env:
          any:
            - CI: "true"
            - GITHUB_ACTIONS: "true"
```

### Example 2: Platform-Specific Templates

```yaml
configure:
  templates:
    # Linux-specific configuration
    - source: templates/linux-config.sh
      destination: ~/.config/app/config.sh
      mode: overwrite
      condition:
        platform:
          os: ["linux"]

    # macOS-specific configuration
    - source: templates/macos-config.sh
      destination: ~/.config/app/config.sh
      mode: overwrite
      condition:
        platform:
          os: ["macos"]
```

### Example 3: GPU-Aware Templates

```yaml
configure:
  templates:
    # GPU-accelerated config
    - source: ollama-gpu-config.toml
      destination: ~/.ollama/config.toml
      mode: overwrite
      condition:
        env:
          NVIDIA_VISIBLE_DEVICES: { exists: true }

    # CPU-only config
    - source: ollama-cpu-config.toml
      destination: ~/.ollama/config.toml
      mode: overwrite
      condition:
        env:
          NVIDIA_VISIBLE_DEVICES: { exists: false }
```

### Example 4: Complex Conditions

```yaml
configure:
  templates:
    # Only in CI on Linux x86_64
    - source: ci-linux-amd64-config.yml
      destination: ~/config.yml
      mode: overwrite
      condition:
        all:
          - env:
              any:
                - CI: "true"
                - GITHUB_ACTIONS: "true"
          - platform:
              os: ["linux"]
              arch: ["x86_64"]
```

### Example 5: Regex Matching

```yaml
configure:
  templates:
    # Only if workspace path matches pattern
    - source: workspace-config.yml
      destination: ~/.workspace/config.yml
      mode: overwrite
      condition:
        env:
          WORKSPACE: { matches: "^/home/.*workspace$" }
```

### Example 6: List Membership

```yaml
configure:
  templates:
    # Only in specific deployment environments
    - source: production-config.yml
      destination: ~/config.yml
      mode: overwrite
      condition:
        env:
          DEPLOY_ENV: { in_list: ["staging", "production"] }
```

## Consequences

### Positive

1. **Declarative Configuration**: Replace 7 extensions' bash scripts with YAML
2. **Type Safety**: Strongly typed conditions vs string comparison in bash
3. **Maintainability**: Easier to understand and modify than bash conditionals
4. **Testability**: Unit tests for condition evaluation (14 tests), integration tests (6 tests)
5. **Performance**: Condition evaluation adds <1ms per template
6. **Extensibility**: Easy to add new condition types (capability detection, etc.)
7. **Documentation**: Conditions self-document template selection logic
8. **Migration Path**: Clear path to remove bash script workarounds

### Negative

1. **Complexity**: Added ~700 lines of code (conditions.rs + types + tests)
2. **Dependencies**: New dependency on `regex = "1.10"` for pattern matching
3. **Learning Curve**: Extension authors need to learn condition syntax
4. **Testing Burden**: Conditions must be tested in both CI and local environments

### Neutral

1. **Backward Compatible**: Templates without conditions work unchanged
2. **Opt-In**: Only extensions that need conditional logic add conditions
3. **Future Work**: Capability detection (GPU, container, network), custom functions

## Implementation Notes

### Platform Detection

Uses Rust standard library constants:
```rust
fn detect_platform() -> PlatformInfo {
    let os = match std::env::consts::OS {
        "linux" => "linux",
        "macos" => "macos",
        "windows" => "windows",
        _ => "unknown",
    };

    let arch = match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        "arm" => "arm64",
        _ => "unknown",
    };

    PlatformInfo { os, arch }
}
```

### Condition Evaluation Order

1. Check `env` conditions (fail fast on mismatch)
2. Check `platform` conditions (fail fast on mismatch)
3. Evaluate `any` (OR - return true if any match)
4. Evaluate `all` (AND - return false if any fail)
5. Evaluate `not` (invert result)

### Template Selection Strategy

- **First match wins** for same destination
- Templates processed in YAML order
- Multiple templates can target different destinations
- Log warnings if multiple templates match same destination

### Testing Strategy

**Unit Tests (14 tests in conditions.rs):**
- Simple environment matching
- Complex expressions (not_equals, exists, regex, in_list)
- Platform detection
- Logical operators (any, all, not)
- Nested conditions
- Edge cases (missing vars, invalid regex)

**Integration Tests (6 tests):**
- CI mode vs local mode template selection
- Platform-specific templates
- Wrong platform skip
- Templates without conditions (backward compatibility)
- Complex multi-condition scenarios

**Test Isolation:**
- Uses `serial_test` crate to serialize tests that modify environment
- Cleans up environment variables after each test
- Validates platform detection on current OS

### Migration Guide for Extensions

**Step 1: Identify Current Logic**
```bash
# Find bash conditionals in install.sh
if [[ "${CI:-}" == "true" ]]; then
  # CI-specific behavior
else
  # Local-specific behavior
fi
```

**Step 2: Convert to Declarative Conditions**
```yaml
configure:
  templates:
    - source: local-template.yml
      destination: ~/config.yml
      condition:
        env:
          not_any:
            - CI: "true"

    - source: ci-template.yml
      destination: ~/config.yml
      condition:
        env:
          CI: "true"
```

**Step 3: Remove Bash Script Logic (Optional)**

After verification, bash scripts can be simplified or removed.

**Step 4: Test Both Modes**
```bash
# Test local
unset CI GITHUB_ACTIONS
sindri extension install <extension>

# Test CI
export CI=true
sindri extension install <extension> --force
```

## Performance Characteristics

**Condition Evaluation:**
- Average: <1ms per template
- Platform detection: One-time cost at evaluator creation
- Environment variable lookup: O(1) via std::env::var
- Regex compilation: Cached per pattern

**Memory Overhead:**
- ConditionEvaluator: ~200 bytes
- Platform info cached during evaluation
- No persistent state between templates

**Impact on Install Time:**
- Negligible for most extensions (<10 templates)
- Measurable only for extensions with >100 templates

## Security Considerations

**Path Traversal:**
- Conditions don't affect path validation
- All existing path security checks still apply

**Code Injection:**
- Conditions are declarative YAML, not executable code
- No eval or dynamic code execution
- Regex patterns validated before execution

**Environment Variable Exposure:**
- Conditions only read environment variables
- No modification of environment during evaluation
- Sensitive variables (passwords, tokens) not exposed in logs

## Dependencies

**New dependency:**
- `regex = "1.10"` - Pattern matching in environment variables

**Existing dependencies:**
- `serde`, `serde_yaml` - Condition deserialization
- `anyhow` - Error handling
- `tracing` - Logging

**Dev dependencies:**
- `serial_test` - Test isolation for environment variables

## Future Enhancements

1. **Capability Detection**
   ```yaml
   condition:
     capability:
       gpu: true
       container: true
       network: true
   ```

2. **Custom Condition Functions**
   ```yaml
   condition:
     function: shell_type  # bash/zsh/fish
     function: kernel_version  # version comparison
     function: file_exists  # check for files
   ```

3. **Condition Validation**
   - Warn if conditions are mutually exclusive
   - Suggest simplifications for complex conditions
   - Static analysis of condition coverage

4. **Template Selection Strategies**
   ```yaml
   strategy: first-match  # default
   strategy: all-match    # apply all matching templates
   strategy: require-one  # error if 0 or 2+ match
   ```

5. **Condition Debugging**
   ```bash
   sindri extension install --debug-conditions
   # Shows which conditions matched/didn't match
   # Explains why templates were skipped
   ```

## References

- ADR 032: Extension Configure Processing
- [Condition types](../../v3/crates/sindri-core/src/types/extension_types.rs)
- [Condition evaluator](../../v3/crates/sindri-extensions/src/configure/conditions.rs)
- [Integration tests](../../v3/crates/sindri-extensions/tests/configure_integration_tests.rs)

## Related ADRs

- ADR 032: Extension Configure Processing (foundation)
- ADR 026: Extension Version Lifecycle Management
- ADR 011: Multi-Method Extension Installation

## Decision Date

2026-01-26

## Authors

- Claude Sonnet 4.5 (implementation)
- Chris Phillipson (review and guidance)
