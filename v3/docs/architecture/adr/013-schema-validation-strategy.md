# ADR 013: Schema Validation Strategy

**Status**: Accepted
**Date**: 2026-01-21
**Deciders**: Core Team
**Related**: [ADR-008: Extension Type System](008-extension-type-system-yaml-deserialization.md)

## Context

The Sindri extension system is declarative and YAML-first. Extensions must conform to strict schemas to ensure:

1. **Structural Validity**: YAML parses correctly and has required fields
2. **Type Safety**: Values match expected types (string, int, bool, array)
3. **Semantic Validity**: Values make sense (version is valid semver, URLs are valid, dependencies exist)
4. **Security**: No injection attacks via malicious YAML
5. **Evolution**: Schema changes don't break existing extensions
6. **Documentation**: Schema serves as specification for extension authors

The bash implementation used `yq` for basic YAML parsing and manual field validation, but lacked:
- Comprehensive schema validation
- Clear error messages for invalid extensions
- Schema versioning and evolution
- Automated validation in CI/CD

Types of validation needed:

**JSON Schema Validation** (structural):
- Required fields present
- Field types correct
- Enum values valid
- Format validation (URLs, semver)

**Semantic Validation** (business rules):
- Dependencies exist in registry
- Install method matches available tools
- Version compatibility constraints valid
- Hook scripts exist and are executable
- MCP server configurations valid

**Example Invalid Extensions**:

```yaml
# Missing required field
metadata:
  name: nodejs
  # Missing: version, category, description

# Invalid type
install:
  method: mise
  version: 123  # Should be string

# Invalid enum value
metadata:
  category: invalid-category  # Not in allowed categories

# Semantic error
dependencies:
  - nonexistent-extension  # Extension doesn't exist
```

## Decision

### Three-Level Validation Pipeline

We implement a **three-level validation strategy**:

1. **YAML Parsing** (serde_yaml) - Deserialize YAML to Rust types
2. **JSON Schema Validation** (jsonschema) - Validate against schema
3. **Semantic Validation** (custom) - Business rule checks

```rust
pub struct ExtensionValidator {
    schema: Schema,
    registry: Registry,
}

impl ExtensionValidator {
    /// Full validation pipeline
    pub fn validate(&self, yaml_content: &str) -> Result<Extension> {
        // Level 1: YAML parsing
        let extension: Extension = serde_yaml::from_str(yaml_content)
            .context("Failed to parse extension YAML")?;

        // Level 2: JSON Schema validation
        self.validate_schema(&extension)
            .context("Schema validation failed")?;

        // Level 3: Semantic validation
        self.validate_semantics(&extension)
            .context("Semantic validation failed")?;

        Ok(extension)
    }

    /// Level 2: JSON Schema validation
    fn validate_schema(&self, extension: &Extension) -> Result<()> {
        // Convert Extension to serde_json::Value for jsonschema
        let json_value = serde_json::to_value(extension)?;

        // Validate against schema
        let compiled = jsonschema::JSONSchema::compile(&self.schema.json)
            .map_err(|e| anyhow!("Failed to compile schema: {}", e))?;

        if let Err(errors) = compiled.validate(&json_value) {
            let error_messages: Vec<String> = errors
                .map(|e| format!("  - {}", e))
                .collect();

            bail!(
                "Extension failed schema validation:\n{}",
                error_messages.join("\n")
            );
        }

        Ok(())
    }

    /// Level 3: Semantic validation
    fn validate_semantics(&self, extension: &Extension) -> Result<()> {
        let mut errors = Vec::new();

        // Validate dependencies exist
        if let Some(deps) = &extension.dependencies {
            for dep in deps {
                if !self.registry.extensions.contains_key(dep) {
                    errors.push(format!(
                        "Dependency '{}' not found in registry",
                        dep
                    ));
                }
            }
        }

        // Validate version is valid semver
        if let Err(e) = Version::parse(&extension.metadata.version) {
            errors.push(format!(
                "Invalid semantic version '{}': {}",
                extension.metadata.version,
                e
            ));
        }

        // Validate CLI version constraints
        if let Some(min_ver) = &extension.metadata.min_cli_version {
            if let Err(e) = Version::parse(min_ver) {
                errors.push(format!(
                    "Invalid min_cli_version '{}': {}",
                    min_ver,
                    e
                ));
            }
        }

        // Validate install method specifics
        match &extension.install {
            Install::Mise { tool, version } => {
                self.validate_mise_tool(tool, version, &mut errors);
            }
            Install::Binary { url, .. } => {
                self.validate_url(url, &mut errors);
            }
            Install::Npm { package, .. } => {
                self.validate_npm_package(package, &mut errors);
            }
            _ => {}
        }

        // Validate capabilities
        if let Some(capabilities) = &extension.capabilities {
            self.validate_capabilities(capabilities, &mut errors);
        }

        if !errors.is_empty() {
            bail!(
                "Extension failed semantic validation:\n  - {}",
                errors.join("\n  - ")
            );
        }

        Ok(())
    }

    fn validate_mise_tool(&self, tool: &str, version: &str, errors: &mut Vec<String>) {
        // Check tool name is valid
        if !tool.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            errors.push(format!("Invalid mise tool name: '{}'", tool));
        }

        // Validate version format (can be semver or mise-specific like "lts")
        if version != "latest" && version != "lts" {
            if let Err(e) = Version::parse(version) {
                errors.push(format!(
                    "Invalid version '{}' for mise tool '{}': {}",
                    version, tool, e
                ));
            }
        }
    }

    fn validate_url(&self, url: &str, errors: &mut Vec<String>) {
        match Url::parse(url) {
            Ok(parsed) => {
                if parsed.scheme() != "https" {
                    errors.push(format!(
                        "Binary URL must use HTTPS: '{}'",
                        url
                    ));
                }
            }
            Err(e) => {
                errors.push(format!("Invalid URL '{}': {}", url, e));
            }
        }
    }

    fn validate_npm_package(&self, package: &str, errors: &mut Vec<String>) {
        // npm package names: lowercase, may have @scope/
        if package.starts_with('@') {
            if !package.contains('/') {
                errors.push(format!(
                    "Invalid scoped npm package name: '{}'",
                    package
                ));
            }
        }

        // Check for invalid characters
        if !package.chars().all(|c| {
            c.is_alphanumeric() || matches!(c, '-' | '_' | '/' | '@' | '.')
        }) {
            errors.push(format!("Invalid npm package name: '{}'", package));
        }
    }

    fn validate_capabilities(&self, cap: &Capabilities, errors: &mut Vec<String>) {
        // Validate authentication methods
        if let Some(auth_methods) = &cap.authentication {
            for auth in auth_methods {
                match auth {
                    AuthMethod::ApiKey { env_var, .. } => {
                        self.validate_env_var_name(env_var, errors);
                    }
                    AuthMethod::CliAuth { command, .. } => {
                        self.validate_command_path(command, errors);
                    }
                    AuthMethod::Token { env_var, .. } => {
                        self.validate_env_var_name(env_var, errors);
                    }
                }
            }
        }

        // Validate MCP configuration
        if let Some(mcp) = &cap.mcp {
            if mcp.enabled {
                if let Some(servers) = &mcp.servers {
                    for server in servers {
                        self.validate_mcp_server(server, errors);
                    }
                }
            }
        }
    }

    fn validate_env_var_name(&self, name: &str, errors: &mut Vec<String>) {
        // Environment variable names: UPPERCASE, digits, underscore
        if !name.chars().all(|c| c.is_uppercase() || c.is_numeric() || c == '_') {
            errors.push(format!(
                "Invalid environment variable name: '{}' (should be UPPERCASE_WITH_UNDERSCORES)",
                name
            ));
        }
    }

    fn validate_command_path(&self, path: &str, errors: &mut Vec<String>) {
        // Command paths should be absolute or in PATH
        if path.starts_with('/') {
            // Absolute path - check reasonable
            if !path.starts_with("/usr/") && !path.starts_with("/bin/") {
                errors.push(format!(
                    "Suspicious absolute command path: '{}'",
                    path
                ));
            }
        } else {
            // Relative path - should be simple command name
            if path.contains('/') {
                errors.push(format!(
                    "Relative command path should not contain '/': '{}'",
                    path
                ));
            }
        }
    }

    fn validate_mcp_server(&self, server: &McpServer, errors: &mut Vec<String>) {
        self.validate_command_path(&server.command, errors);

        // Validate server name is valid
        if !server.name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            errors.push(format!(
                "Invalid MCP server name: '{}' (use alphanumeric, dash, underscore)",
                server.name
            ));
        }
    }
}
```

### Embedded Schemas with Runtime Fallback

Schemas are **embedded in the binary** at compile time using `rust-embed`, with runtime fallback to GitHub:

```rust
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "schemas/"]
pub struct SchemaAssets;

pub struct SchemaLoader {
    github_fallback: bool,
}

impl SchemaLoader {
    pub fn load_extension_schema(&self) -> Result<Schema> {
        // Try embedded schema first
        if let Some(content) = SchemaAssets::get("extension.schema.json") {
            let json: Value = serde_json::from_slice(&content.data)?;
            return Ok(Schema { json });
        }

        // Fallback to GitHub if embedded schema missing
        if self.github_fallback {
            return self.fetch_schema_from_github("extension.schema.json").await;
        }

        bail!("Extension schema not found (embedded or GitHub)")
    }

    async fn fetch_schema_from_github(&self, name: &str) -> Result<Schema> {
        let url = format!(
            "https://raw.githubusercontent.com/pacphi/sindri/main/docker/lib/schemas/{}",
            name
        );

        let response = reqwest::get(&url).await?;
        let json: Value = response.json().await?;

        Ok(Schema { json })
    }
}
```

### Decoupled Schema Repository

Schemas are maintained in **separate repository** (`docker/lib/schemas/`) but embedded at build time:

```
docker/lib/schemas/
├── extension.schema.json       # Main extension schema
├── metadata.schema.json        # Metadata subschema
├── install.schema.json         # Install methods subschema
├── capabilities.schema.json    # Capabilities subschema
├── validation.schema.json      # Validation subschema
├── bom.schema.json            # BOM subschema
└── README.md                  # Schema documentation
```

**Build-time Embedding** (build.rs):
```rust
fn main() {
    // Tell cargo to rebuild if schemas change
    println!("cargo:rerun-if-changed=schemas/");

    // Validate all schemas at build time
    validate_schemas().expect("Schema validation failed");
}

fn validate_schemas() -> Result<()> {
    for entry in fs::read_dir("schemas/")? {
        let entry = entry?;
        if entry.path().extension().map_or(false, |e| e == "json") {
            let content = fs::read_to_string(entry.path())?;
            let _schema: Value = serde_json::from_str(&content)
                .with_context(|| format!("Invalid JSON in {:?}", entry.path()))?;
            println!("Validated schema: {:?}", entry.path());
        }
    }
    Ok(())
}
```

### Schema Evolution and Versioning

Extensions declare schema version they conform to:

```yaml
# In extension.yaml
schema_version: "1.0.0"

metadata:
  name: nodejs
  version: 1.2.0
  # ...
```

**Version Compatibility Check**:
```rust
pub fn check_schema_compatibility(extension: &Extension) -> Result<()> {
    let ext_schema_version = extension.schema_version
        .as_ref()
        .map(|v| Version::parse(v))
        .transpose()?
        .unwrap_or_else(|| Version::parse("1.0.0").unwrap());

    let cli_schema_version = Version::parse(env!("SCHEMA_VERSION"))?;

    // Major version must match
    if ext_schema_version.major != cli_schema_version.major {
        bail!(
            "Incompatible schema version: extension uses {}, CLI expects {}",
            ext_schema_version,
            cli_schema_version
        );
    }

    // Minor version can be less than or equal
    if ext_schema_version.minor > cli_schema_version.minor {
        bail!(
            "Extension schema {} is newer than CLI schema {}",
            ext_schema_version,
            cli_schema_version
        );
    }

    Ok(())
}
```

## Consequences

### Positive

1. **Comprehensive Validation**: Three levels catch all error types
2. **Clear Error Messages**: Specific validation errors guide extension authors
3. **Type Safety**: Rust types + JSON Schema = double validation
4. **Embedded Schemas**: No external dependencies for validation
5. **Evolution Support**: Schema versioning enables gradual migration
6. **CI Integration**: Automated validation in CI/CD pipeline
7. **Offline Operation**: Embedded schemas work without network
8. **Performance**: Schemas compiled once at startup
9. **Documentation**: JSON Schema serves as machine-readable spec
10. **Security**: Validation prevents malicious YAML

### Negative

1. **Duplication**: Schema exists in both JSON Schema and Rust types
2. **Maintenance**: Must keep JSON Schema and Rust types in sync
3. **Build Time**: Embedding schemas adds ~1 second to build
4. **Binary Size**: Embedded schemas add ~50KB to binary
5. **Error Verbosity**: JSON Schema errors can be verbose
6. **Schema Evolution**: Breaking changes require major version bump

### Neutral

1. **Validation Order**: Could reorder levels (schema before parsing, etc.)
2. **Schema Location**: Could move to separate crate
3. **Fallback Strategy**: GitHub fallback rarely used in practice

## Alternatives Considered

### 1. Compile-Time Validation Only (Rust Types)

**Description**: Skip JSON Schema, rely only on Rust type system for validation.

**Pros**:
- No schema duplication
- Simpler implementation
- Smaller binary (no jsonschema crate)
- Faster validation

**Cons**:
- No schema documentation for external tools
- Can't validate raw YAML without parsing
- Harder to generate schema documentation
- No standard format for extension specification

**Rejected**: JSON Schema provides valuable documentation and external tooling support.

### 2. Runtime-Only Schema (No Embedding)

**Description**: Always fetch schemas from GitHub at runtime.

**Pros**:
- Always latest schemas
- No build-time embedding
- Smaller binary

**Cons**:
- Requires network connection
- Slower validation (fetch overhead)
- Rate limit concerns
- Fails in offline environments

**Rejected**: Embedded schemas enable offline operation and better performance.

### 3. JSON Schema Generation from Rust Types

**Description**: Use `schemars` crate to generate JSON Schema from Rust types.

```rust
use schemars::{schema_for, JsonSchema};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Extension {
    // ...
}

// Generate schema at build time
let schema = schema_for!(Extension);
```

**Pros**:
- Single source of truth (Rust types)
- No schema duplication
- Automatic sync between types and schema
- Less maintenance

**Cons**:
- Generated schemas less human-readable
- Less control over schema details
- schemars doesn't support all JSON Schema features
- Harder to customize error messages

**Partially Adopted**: We use this for internal validation but maintain handwritten schemas for documentation.

### 4. YAML Schema (Not JSON Schema)

**Description**: Use YAML-specific schema language.

**Pros**:
- Native YAML validation
- More expressive for YAML-specific features

**Cons**:
- Less tooling support
- Not widely adopted
- Harder to integrate with existing tools
- No clear standard like JSON Schema

**Rejected**: JSON Schema is industry standard and has better tooling.

### 5. Relaxed Validation (Warnings Only)

**Description**: Make validation warnings instead of errors, allow invalid extensions.

**Pros**:
- More flexible
- Doesn't block experimentation
- Easier for extension authors

**Cons**:
- Invalid extensions could cause runtime failures
- Security risks (malicious YAML)
- Poor user experience (fails during installation)
- Harder to debug issues

**Rejected**: Strict validation catches errors early and improves quality.

## Compliance

- ✅ Three-level validation pipeline (parse, schema, semantic)
- ✅ JSON Schema validation with jsonschema crate
- ✅ Embedded schemas via rust-embed
- ✅ Runtime fallback to GitHub
- ✅ Schema versioning and compatibility checks
- ✅ Clear error messages for all validation failures
- ✅ Build-time schema validation
- ✅ 100% test coverage for validation logic

## Notes

The three-level validation is designed to be defensive: each level catches different error types, providing comprehensive coverage.

JSON Schema validation happens at deserialization time, not at runtime, for performance. Once an extension passes validation, it's guaranteed to be structurally valid.

Semantic validation is extensible - new business rules can be added without changing schemas or types.

Future enhancement: Add `--strict` flag for extra validation (e.g., warn about deprecated fields, suggest improvements).

## Related Decisions

- [ADR-008: Extension Type System](008-extension-type-system-yaml-deserialization.md) - Rust types for validation
- [ADR-010: GitHub Distribution](010-github-extension-distribution.md) - Registry includes checksums
- [ADR-011: Multi-Method Installation](011-multi-method-extension-installation.md) - Validates install configuration
- [ADR-012: Registry and Manifest Architecture](012-registry-manifest-dual-state-architecture.md) - Validates registry entries
