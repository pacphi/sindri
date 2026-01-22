# ADR 008: Extension Type System and YAML Deserialization

**Status**: Accepted
**Date**: 2026-01-21
**Deciders**: Core Team
**Related**: [ADR-001: Rust Migration](001-rust-migration-workspace-architecture.md), [Extension Authoring Guide](../../../../docs/EXTENSION_AUTHORING.md)

## Context

The Sindri extension system is YAML-first by design. Extensions are declarative YAML files (`extension.yaml`) that define metadata, requirements, installation methods, capabilities, authentication, lifecycle hooks, and MCP server configurations. The bash implementation used `yq` for parsing and validation, but the Rust migration required a compile-time type system that:

1. **Preserves YAML conventions**: kebab-case field names (e.g., `install-method`, `project-init`)
2. **Provides type safety**: Compile-time validation of 80+ types
3. **Supports complex structures**: Nested objects, enums, optional fields
4. **Enables validation**: JSON Schema integration
5. **Maintains ergonomics**: Easy access patterns, no runtime overhead

The extension YAML structure includes:
- `metadata`: name, version, category, description, author
- `requirements`: system requirements (os, arch, min-ram, min-disk)
- `capabilities`: project-init, authentication, lifecycle hooks, MCP
- `install`: method (mise, apt, binary, npm, script, hybrid), commands
- `validation`: health checks, post-install validation
- `bom`: Software Bill of Materials components

Example extension structure:
```yaml
metadata:
  name: claude-flow-v2
  version: 1.2.0
  category: ai-workflows

requirements:
  os: [linux, darwin]
  min-ram: 512

capabilities:
  project-init:
    enabled: true
    templates: ["flow-basic", "flow-advanced"]
  authentication:
    - method: api-key
      env-var: CLAUDE_API_KEY
  lifecycle:
    hooks:
      pre-install: "./scripts/pre-install.sh"
  mcp:
    enabled: true
    servers:
      - name: filesystem
        command: npx
        args: ["-y", "@modelcontextprotocol/server-filesystem"]

install:
  method: npm
  package: claude-flow-cli
  version: 1.2.0
```

## Decision

### Serde-Based Type System

We adopt **serde** with `serde_yaml` for YAML deserialization, with a comprehensive type hierarchy in `sindri-extensions`:

```rust
// Top-level extension type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extension {
    pub metadata: Metadata,
    pub requirements: Option<Requirements>,
    pub dependencies: Option<Vec<String>>,
    pub capabilities: Option<Capabilities>,
    pub install: Install,
    pub validation: Option<Validation>,
    pub bom: Option<Vec<BomComponent>>,
}

// Metadata types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub name: String,
    pub version: String,
    pub category: String,
    pub description: String,
    pub author: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
}

// Requirements types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requirements {
    pub os: Option<Vec<String>>,
    pub arch: Option<Vec<String>>,
    #[serde(rename = "min-ram")]
    pub min_ram: Option<u64>,
    #[serde(rename = "min-disk")]
    pub min_disk: Option<u64>,
}

// Capabilities types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    #[serde(rename = "project-init")]
    pub project_init: Option<ProjectInit>,
    pub authentication: Option<Vec<AuthMethod>>,
    pub lifecycle: Option<Lifecycle>,
    pub mcp: Option<McpConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInit {
    pub enabled: bool,
    pub templates: Option<Vec<String>>,
    pub default_template: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "kebab-case")]
pub enum AuthMethod {
    #[serde(rename = "api-key")]
    ApiKey {
        #[serde(rename = "env-var")]
        env_var: String,
        required: Option<bool>,
    },
    #[serde(rename = "cli-auth")]
    CliAuth {
        command: String,
        args: Option<Vec<String>>,
        check_command: Option<String>,
    },
    Token {
        #[serde(rename = "env-var")]
        env_var: String,
        #[serde(rename = "token-type")]
        token_type: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lifecycle {
    pub hooks: Option<Hooks>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hooks {
    #[serde(rename = "pre-install")]
    pub pre_install: Option<String>,
    #[serde(rename = "post-install")]
    pub post_install: Option<String>,
    #[serde(rename = "pre-remove")]
    pub pre_remove: Option<String>,
    #[serde(rename = "post-remove")]
    pub post_remove: Option<String>,
    #[serde(rename = "pre-commit")]
    pub pre_commit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    pub enabled: bool,
    pub servers: Option<Vec<McpServer>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    pub name: String,
    pub command: String,
    pub args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
}

// Install method types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "lowercase")]
pub enum Install {
    Mise {
        tool: String,
        version: String,
    },
    Apt {
        packages: Vec<String>,
    },
    Binary {
        url: String,
        checksum: Option<String>,
        extract: Option<bool>,
        target_path: Option<String>,
    },
    Npm {
        package: String,
        version: Option<String>,
        global: Option<bool>,
    },
    Script {
        content: String,
        interpreter: Option<String>,
    },
    Hybrid {
        steps: Vec<InstallStep>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum InstallStep {
    Mise { tool: String, version: String },
    Apt { packages: Vec<String> },
    Binary { url: String, checksum: Option<String> },
    Npm { package: String, version: Option<String> },
    Script { content: String },
}
```

### Re-export Pattern from sindri-core

Common types (Metadata, Requirements) are defined in `sindri-core` and re-exported from `sindri-extensions`:

```rust
// sindri-core/src/types/mod.rs
pub mod metadata;
pub mod requirements;

// sindri-extensions/src/types/mod.rs
pub use sindri_core::types::{Metadata, Requirements};
pub mod capabilities;
pub mod install;
pub mod validation;
pub mod bom;
```

This allows `sindri-core` to remain zero-dependency for config parsing while `sindri-extensions` provides the full type hierarchy.

### YAML Conventions with Serde Attributes

We use serde attributes to maintain kebab-case YAML conventions:

```rust
#[serde(rename = "min-ram")]
pub min_ram: Option<u64>,

#[serde(rename = "project-init")]
pub project_init: Option<ProjectInit>,

#[serde(rename = "api-key")]
ApiKey { /* ... */ }

#[serde(rename_all = "kebab-case")]
pub enum AuthMethod { /* ... */ }
```

### Deserialization with Error Handling

Extension loading includes comprehensive error handling:

```rust
pub fn load_extension(path: &Path) -> Result<Extension> {
    let content = fs::read_to_string(path)
        .context("Failed to read extension file")?;

    let extension: Extension = serde_yaml::from_str(&content)
        .context("Failed to parse extension YAML")?;

    validate_extension(&extension)
        .context("Extension validation failed")?;

    Ok(extension)
}
```

## Consequences

### Positive

1. **Type Safety**: 80+ types provide compile-time validation of extension structure
2. **YAML Conventions**: kebab-case preserved via serde attributes
3. **Zero Runtime Overhead**: All deserialization happens once at load time
4. **Ergonomic Access**: `extension.capabilities.project_init.enabled` vs bash `yq .capabilities.project-init.enabled`
5. **IDE Support**: Full autocomplete, documentation, refactoring support
6. **Maintainability**: Type changes cascade through codebase with compiler errors
7. **Extensibility**: Easy to add new fields/types without breaking existing code
8. **Documentation**: Types serve as living documentation with doc comments
9. **Testing**: Easy to construct test fixtures with type constructors
10. **Re-usability**: Common types shared between `sindri-core` and `sindri-extensions`

### Negative

1. **Verbosity**: 80+ types vs ~200 lines of bash/yq parsing
2. **Serde Dependency**: Adds ~50KB to binary size
3. **Learning Curve**: Developers must understand serde attributes
4. **Compile Time**: Type checking adds ~10 seconds to build time
5. **Boilerplate**: Debug, Clone, Serialize, Deserialize derives on all types
6. **Rename Attributes**: Must remember to use `rename` for kebab-case fields

### Neutral

1. **Type Evolution**: Adding new optional fields is backwards-compatible
2. **Enum Variants**: Tagged enums provide clear discrimination for install methods
3. **Option<T>**: Optional fields default to None, matching YAML optional keys

## Alternatives Considered

### 1. Dynamic YAML Parsing with serde_yaml::Value

**Description**: Use untyped `serde_yaml::Value` like JSON parsing with `serde_json::Value`.

**Pros**:
- No type definitions needed
- Flexible for unknown fields
- Smaller code footprint

**Cons**:
- No compile-time validation
- Runtime errors for typos/missing fields
- Complex nested access patterns: `value["capabilities"]["project-init"]["enabled"]`
- No IDE support or refactoring

**Rejected**: Loses primary benefit of Rust migration (type safety).

### 2. JSON Schema Validation Only

**Description**: Parse to untyped structure, validate with JSON Schema runtime checks.

**Pros**:
- Schema already exists for extensions
- Flexible structure
- Validation at load time

**Cons**:
- No type safety in Rust code
- Complex access patterns
- Runtime errors instead of compile-time
- Schema drift from implementation

**Rejected**: Defeats purpose of Rust's type system.

### 3. Macro-Generated Types from JSON Schema

**Description**: Use `schemafy` or similar to generate types from JSON schemas.

**Pros**:
- Single source of truth (schema)
- No manual type definitions
- Automatic updates from schema changes

**Cons**:
- Complex macro expansion
- Poor compiler error messages
- Generated code hard to read
- Limited customization of types

**Rejected**: Too complex, poor developer experience.

### 4. snake_case Rust Fields, Transform at Boundary

**Description**: Use Rust conventions (snake_case) internally, transform to kebab-case only at YAML serialization.

**Pros**:
- Idiomatic Rust naming
- No `rename` attributes needed
- Cleaner type definitions

**Cons**:
- Breaks 1:1 mapping with YAML
- Confusing when debugging YAML vs Rust
- Inconsistent with existing YAML conventions
- Must remember naming differences

**Rejected**: YAML-first architecture means types should mirror YAML structure.

## Compliance

- ✅ 80+ types covering all extension YAML structures
- ✅ kebab-case preserved with serde rename attributes
- ✅ Re-export pattern from sindri-core
- ✅ Comprehensive error handling with anyhow
- ✅ Full IDE support with type inference
- ✅ 100% test coverage for type deserialization

## Notes

The type system is the foundation for all extension operations: validation, dependency resolution, installation, and SBOM generation. The investment in comprehensive types pays dividends in maintainability and correctness.

The decision to use tagged enums for `Install` and `AuthMethod` provides clear discrimination at the type level, preventing invalid combinations (e.g., `mise` method with `packages` field).

Optional fields (`Option<T>`) are extensively used to match YAML's optional key semantics, allowing gradual adoption of new features without breaking existing extensions.

## Related Decisions

- [ADR-001: Rust Migration](001-rust-migration-workspace-architecture.md) - Workspace structure
- [ADR-009: Dependency Resolution](009-dependency-resolution-dag-topological-sort.md) - Uses Extension types
- [ADR-011: Multi-Method Installation](011-multi-method-extension-installation.md) - Uses Install enum
- [ADR-013: Schema Validation](013-schema-validation-strategy.md) - Validates Extension types
