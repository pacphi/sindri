# ADR 028: Template-Based Config Init Generation

**Status**: Accepted
**Date**: 2026-01-23
**Deciders**: Core Team
**Related**: [ADR-003: Template-Based Configuration](003-template-based-configuration.md)

## Context

The `sindri config init` command generates a new sindri.yaml configuration file. The v2 CLI generated ~400 lines of well-documented YAML serving as both configuration and documentation, while v3 initially generated minimal ~35 lines with almost no documentation.

### Problems with Initial v3 Implementation

1. **Minimal Documentation**: Users had no visibility into available options (GPU, DinD, secrets, profiles)
2. **Profile Bug**: The `--profile` CLI argument was accepted but ignored (always output "minimal")
3. **Version Misalignment**: Config output `version: "1.0"` but should be `version: "3.0"` for v3
4. **Hardcoded Format**: Used a format string in `loader.rs:203-243`, hard to maintain
5. **No Provider-Awareness**: All providers got identical output despite different capabilities

### Requirements

- Generate comprehensive, self-documenting configuration files
- Provider-aware content (show relevant options per provider)
- Fix `--profile` argument to actually affect output
- Align version to "3.0" for v3 CLI
- Leverage existing Tera template infrastructure (ADR-003)
- Embed profiles with descriptions for user reference

## Decision

### Template-Based Generation in sindri-core

We extend the template pattern from `sindri-providers` to `sindri-core` for config init generation.

**New Components**:

```
v3/crates/sindri-core/src/templates/
├── mod.rs              # ConfigTemplateRegistry
├── context.rs          # ConfigInitContext struct
└── sindri.yaml.tera    # Main template (~200 lines)
```

### ConfigInitContext

A specialized context for config generation, separate from provider template context:

```rust
pub struct ConfigInitContext {
    pub name: String,
    pub provider: String,
    pub profile: String,
    pub profiles: Vec<ProfileInfo>,      // All profiles with descriptions
    pub provider_supports_gpu: bool,     // Conditional GPU section
    pub provider_supports_dind: bool,    // Conditional DinD section
    pub provider_supports_ssh: bool,     // Conditional SSH setup guide
    pub default_region: String,          // Provider-specific default
}
```

### Provider Capability Matrix

| Provider   | GPU | DinD | SSH | Default Region |
| ---------- | --- | ---- | --- | -------------- |
| Fly        | ✓   | ✗    | ✓   | sjc            |
| Docker     | ✓   | ✓    | ✓   | (none)         |
| Kubernetes | ✓   | ✓    | ✓   | default        |
| DevPod     | ✓   | ✓    | ✓   | us-west-2      |
| E2B        | ✗   | ✗    | ✗   | (none)         |

### Template Design Principles

1. **Provider-Specific Sections**: Only show relevant provider config
2. **Self-Documenting**: Extensive comments explaining each option
3. **All Profiles Listed**: Users see all available profiles with descriptions
4. **Secrets Examples**: Common patterns (API keys, SSH keys, Vault)
5. **GPU Conditional**: Only shown for providers that support it
6. **Setup Guides**: Provider-specific instructions (VS Code SSH for Fly)

### API Changes

**Old API** (ignored profile):

```rust
pub fn generate_default_config(name: &str, provider: Provider) -> String
```

**New API** (uses profile):

```rust
pub fn generate_config(name: &str, provider: Provider, profile: &str) -> Result<String>
```

The old function is kept for backward compatibility, delegating to the new one.

### Version Alignment

All config generation now outputs `version: "3.0"` to align with:

- Workspace version: `3.0.0`
- `profiles.yaml`: `version: "3.0.0"`
- `registry.yaml`: `version: "3.0.0"`

## Consequences

### Positive

1. **Self-Documenting Configs**: Generated files serve as reference documentation
2. **Profile Selection Works**: `--profile anthropic-dev` actually outputs "anthropic-dev"
3. **Provider-Aware**: Fly users see Fly options, Docker users see Docker options
4. **Consistent Pattern**: Same template approach as provider configs (ADR-003)
5. **Embedded Templates**: No external files needed
6. **Version Consistency**: All v3 configs use version "3.0"

### Negative

1. **Larger Output**: ~200 lines vs ~35 lines (by design)
2. **Template Complexity**: More sophisticated Tera template with conditionals
3. **Hardcoded Profiles**: Profile list embedded in code (could sync from profiles.yaml)

### Trade-offs

**Comprehensive vs Minimal**

- v2: Very comprehensive but overwhelming
- Initial v3: Too minimal, unhelpful
- New v3: Balanced - provider-specific content only

**Embedded vs Dynamic**

- Could load profiles from profiles.yaml at runtime
- Chose embedded for reliability and binary self-containment
- Trade-off: Manual sync if profiles change

## Generated Output Examples

### Fly Provider

```yaml
# =============================================================================
# Sindri Configuration
# =============================================================================
version: "3.0"
name: my-project

deployment:
  provider: fly
  resources:
    memory: 4GB
    cpus: 2
    # gpu:
    #   enabled: true
    #   type: nvidia

extensions:
  profile: fullstack
  # Available profiles:
  #   minimal - Minimal development setup
  #   fullstack - Full-stack web development
  #   anthropic-dev - AI development with Anthropic toolset
  ...

providers:
  fly:
    region: sjc
    autoStopMachines: true
    sshPort: 10022

  # VS Code Remote SSH Setup:
  # 1. Deploy: sindri deploy
  # 2. Add to ~/.ssh/config:
  #    Host sindri-my-project
  #        HostName my-project.fly.dev
  #        Port 10022
```

### E2B Provider (No GPU)

```yaml
deployment:
  provider: e2b
  resources:
    memory: 4GB
    cpus: 2
    # Note: E2B does not support GPU

providers:
  e2b:
    timeout: 300
    autoPause: true

  # E2B Cost Optimization Tips:
  # - Use short timeouts (300s) for interactive development
  # - Enable autoPause to avoid paying for idle time
```

## Validation

### Test Coverage

- `test_render_config_docker`: Validates Docker template
- `test_render_config_fly`: Validates Fly template with region
- `test_render_config_e2b_no_gpu`: Ensures GPU section not shown for E2B
- `test_context_profiles_loaded`: Verifies profiles populated

### Manual Testing

```bash
# Test profile argument now works
sindri config init --provider fly --profile anthropic-dev
grep "profile: anthropic-dev" sindri.yaml  # ✓ Found

# Test version
grep 'version: "3.0"' sindri.yaml  # ✓ Found

# Test provider-specific content
sindri config init --provider e2b
grep "gpu:" sindri.yaml  # ✗ Not found (correct - E2B doesn't support GPU)
```

## Future Considerations

- **Interactive Mode**: Prompt for provider/profile selection
- **Profile Sync**: Auto-generate profile list from profiles.yaml
- **Schema Comments**: Extract documentation from JSON schema
- **Minimal Flag**: `--minimal` to output terse config without comments

## References

- Implementation: `crates/sindri-core/src/templates/`
- Template: `crates/sindri-core/src/templates/sindri.yaml.tera`
- Context: `crates/sindri-core/src/templates/context.rs`
- CLI Command: `crates/sindri/src/commands/config.rs`
- Related ADR: [003-template-based-configuration.md](003-template-based-configuration.md)
