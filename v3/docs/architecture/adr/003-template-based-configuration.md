# ADR 003: Template-Based Configuration Generation

**Status**: Accepted
**Date**: 2026-01-21
**Deciders**: Core Team
**Related**: [ADR-002: Provider Abstraction](002-provider-abstraction-layer.md)

## Context

Each cloud provider requires a specific configuration file format:
- **Docker**: docker-compose.yml (YAML)
- **Fly.io**: fly.toml (TOML)
- **DevPod**: devcontainer.json (JSON)
- **E2B**: e2b.toml (TOML)
- **Kubernetes**: k8s-deployment.yaml (YAML)

The bash implementation used heredocs and string concatenation to generate these files, leading to:
1. **Fragile**: Easy to break YAML/TOML formatting
2. **Repetitive**: Similar logic duplicated across adapters
3. **Untestable**: Hard to validate generated configs
4. **Hard to Read**: Complex multi-line string building

Requirements:
- Generate valid provider configs from sindri.yaml
- Support conditional sections (GPU, DinD, CI mode)
- Embed templates in binary (no external files)
- Validate template rendering at compile time
- Reuse common configuration data

## Decision

### Tera Template Engine

We adopt **Tera** as the template engine for all provider configuration generation.

**Why Tera?**
- Jinja2-like syntax (familiar to many developers)
- Excellent error messages
- Compile-time template validation
- Supports YAML, TOML, JSON equally well
- Active maintenance and good performance

### Template Architecture

**1. Embedded Templates**

Templates are embedded in the binary at compile time:

```rust
pub fn new() -> Result<Self> {
    let mut tera = Tera::default();

    tera.add_raw_template("docker-compose.yml",
        include_str!("docker-compose.yml.tera"))?;
    tera.add_raw_template("fly.toml",
        include_str!("fly.toml.tera"))?;
    tera.add_raw_template("e2b.toml",
        include_str!("e2b.toml.tera"))?;
    tera.add_raw_template("devcontainer.json",
        include_str!("devcontainer.json.tera"))?;
    tera.add_raw_template("k8s-deployment.yaml",
        include_str!("k8s-deployment.yaml.tera"))?;

    Ok(Self { tera })
}
```

**2. Template Context**

A unified `TemplateContext` struct provides data to all templates:

```rust
pub struct TemplateContext {
    // Basic info
    pub name: String,
    pub profile: String,
    pub image: String,

    // Resources
    pub memory: String,
    pub cpus: u32,
    pub volume_size: String,

    // GPU
    pub gpu_enabled: bool,
    pub gpu_type: String,
    pub gpu_count: u32,

    // Extensions
    pub custom_extensions: String,
    pub additional_extensions: String,
    pub skip_auto_install: bool,

    // Provider-specific (key-value pairs)
    pub env_vars: HashMap<String, String>,

    // Modes
    pub ci_mode: bool,
}
```

**3. Data Flow**

```
SindriConfig (from sindri.yaml)
    ↓
TemplateContext::from_config(config, provider_mode)
    ↓
Provider adds specific variables (fly_region, e2b_template_alias, etc.)
    ↓
TemplateRegistry.render(template_name, &context)
    ↓
Validated provider config (docker-compose.yml, fly.toml, etc.)
```

### Template Examples

**Docker with DinD (docker-compose.yml.tera)**
```yaml
services:
  sindri:
    image: {{ image }}
    container_name: {{ name }}
{% if runtime %}
    runtime: {{ runtime }}
{% endif %}
{% if dind.mode == "privileged" %}
    privileged: true
    volumes:
      - {{ name }}_home:/alt/home/developer
      - {{ name }}_docker:/var/lib/docker
{% elif dind.mode == "socket" %}
    volumes:
      - {{ name }}_home:/alt/home/developer
      - /var/run/docker.sock:/var/run/docker.sock
{% endif %}
```

**Fly.io with Auto-Suspend (fly.toml.tera)**
```toml
app = "{{ name }}"
primary_region = "{{ fly_region }}"

[[services]]
  auto_stop_machines = "{{ fly_auto_stop_mode }}"
  auto_start_machines = {{ fly_auto_start }}

{% if gpu_enabled %}
[vm]
  guest_type = "{{ fly_gpu_guest_type }}"
  cpus = {{ fly_gpu_cpus }}
{% endif %}
```

### Refactoring History

**Initial Implementation**
- Docker: Used templates ✅
- Fly, E2B: Used inline string generation ❌

**After Refactoring**
- All providers now use Tera templates ✅
- Consistent data flow through TemplateContext ✅
- No inline string generation for configs ✅

## Consequences

### Positive

1. **Correctness**: Templates validate at compile time
2. **Maintainability**: Declarative templates easier to read than string building
3. **Testability**: Can test template rendering independently
4. **Consistency**: Same approach across all providers
5. **Separation**: Logic (Rust) separated from config format (templates)
6. **Reusability**: TemplateContext shared across providers
7. **Embedded**: Templates bundled in binary, no runtime file access

### Negative

1. **Learning Curve**: Team needs to learn Tera syntax
2. **Debugging**: Template errors slightly harder to debug than Rust code
3. **Binary Size**: Templates add ~20KB to binary (minimal impact)
4. **Compile Time**: Template parsing happens at runtime (cached)

### Trade-offs

**Tera vs Alternatives**

Considered:
- **Handlebars**: Less powerful conditionals
- **Askama**: Compile-time but less flexible
- **Format strings**: Fragile, hard to maintain
- **Inline generation**: What we had initially

Chose Tera for balance of power, flexibility, and familiarity.

**Runtime vs Compile-Time**

- Templates parsed at runtime (first use)
- Could use Askama for compile-time, but less flexible
- Trade-off: Minor runtime cost for better developer experience

## Validation

### Template Coverage

| Provider | Template | Size | Conditionals |
|----------|----------|------|--------------|
| Docker | docker-compose.yml.tera | 2.9KB | DinD modes, GPU, secrets |
| Fly.io | fly.toml.tera | 1.9KB | GPU, CI mode, services |
| E2B | e2b.toml.tera | 0.5KB | Resources |
| DevPod | devcontainer.json.tera | 1.0KB | GPU, extensions |
| Kubernetes | k8s-deployment.yaml.tera | 2.4KB | GPU, node selector, storage |

### Test Coverage

- `test_template_registry_creation`: Verifies all templates load
- `test_docker_template_render`: Validates rendering produces valid YAML
- Each provider tests template generation in their deploy tests

### Generated Config Validation

All generated configs tested against provider CLIs:
- `docker compose config` validates docker-compose.yml
- `flyctl config validate` for fly.toml
- `kubectl apply --dry-run` for Kubernetes manifests

## Migration Path

**Phase 1**: Docker provider (baseline)
- Implemented templates from start ✅

**Phase 2**: Initial implementations
- Fly, E2B used inline generation
- DevPod, K8s used templates

**Phase 3**: Refactoring
- Converted Fly.io to templates ✅
- Converted E2B to templates ✅
- Verified DevPod/K8s already compliant ✅

## Future Considerations

- Template inheritance (base template with provider overrides)
- User-provided custom templates (override embedded ones)
- Template linting in CI (validate against provider schemas)
- Hot reload for development (watch template changes)

## References

- Implementation: `crates/sindri-providers/src/templates/`
- Templates: `crates/sindri-providers/src/templates/*.tera`
- Context: `crates/sindri-providers/src/templates/context.rs`
- Tera Documentation: https://keats.github.io/tera/
