# ADR 015: Secrets Resolver Core Architecture

**Status**: Proposed
**Date**: 2026-01-21
**Deciders**: Core Team
**Related**: [ADR-001: Rust Migration](001-rust-migration-workspace-architecture.md), [ADR-002: Provider Abstraction](002-provider-abstraction-layer.md), [ADR-004: Async Runtime](004-async-runtime-command-execution.md)

## Context

Sindri's secrets management system provides **multi-source secret resolution** with provider-agnostic injection. The bash implementation (`cli/secrets-manager`, 823 lines) successfully handles:

1. **Three Secret Sources**:
   - `env`: Environment variables, `.env` files, file content via `fromFile`
   - `file`: Certificates, keys, configs mounted into containers
   - `vault`: HashiCorp Vault KV store for production secrets

2. **Complex Resolution Logic**:
   - Precedence: shell env > `.env.local` > `.env` > `fromFile` > vault
   - Required vs optional secret handling
   - Security: tmpfs storage, secure overwrite on cleanup, restrictive permissions
   - Validation: path traversal prevention, token renewal, dependency checks

3. **Provider Integration**:
   - Fly.io: `flyctl secrets import`, base64-encoded file secrets
   - Docker: `.env.secrets` + Docker secrets with volume mounts
   - Kubernetes: `Secret` resources with envFrom and volume mounts
   - DevPod: `containerEnv` with local environment passthrough

4. **Current Pain Points**:
   - String manipulation for parsing env files (sed/grep)
   - Sequential resolution (no parallelization)
   - Complex base64 encoding/decoding logic scattered across functions
   - Vault token validation in separate function (not integrated into flow)
   - Error accumulation via global counter (`VALIDATION_ERRORS`)
   - Secrets cached in plaintext temporary files

The Rust migration (Phase 5) requires:

- Type-safe secret metadata and resolution context
- Async resolution for Vault API calls and file I/O
- Secure in-memory caching with proper cleanup
- Clear error propagation without global state
- Extensible architecture for future sources (AWS Secrets Manager, GCP Secret Manager)

## Decision

### 1. Core Architecture: Source-Resolver Pattern

We implement a **polymorphic source-resolver architecture** with trait-based abstraction:

```rust
// crates/sindri-secrets/src/lib.rs

pub mod resolver;
pub mod sources;
pub mod context;
pub mod cache;
pub mod injection;

pub use resolver::{SecretResolver, ResolvedSecret};
pub use sources::{SecretSource, EnvSource, FileSource, VaultSource};
pub use context::ResolutionContext;
```

### 2. Type System

**Core Types**:

```rust
// crates/sindri-secrets/src/types.rs

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Secret definition from sindri.yaml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretDefinition {
    pub name: String,
    pub source: SecretSourceType,
    #[serde(default)]
    pub required: bool,

    // source: env fields
    #[serde(rename = "fromFile")]
    pub from_file: Option<String>,

    // source: file fields
    pub path: Option<String>,
    #[serde(rename = "mountPath")]
    pub mount_path: Option<String>,
    pub permissions: Option<String>,

    // source: vault fields
    #[serde(rename = "vaultPath")]
    pub vault_path: Option<String>,
    #[serde(rename = "vaultKey")]
    pub vault_key: Option<String>,
    #[serde(rename = "vaultMount")]
    pub vault_mount: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SecretSourceType {
    Env,
    File,
    Vault,
}

/// Resolved secret with metadata
#[derive(Debug, Clone)]
pub struct ResolvedSecret {
    pub name: String,
    pub value: SecretValue,
    pub metadata: SecretMetadata,
}

/// Secret value with automatic zeroing
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub enum SecretValue {
    /// Environment variable value (string)
    Env(String),
    /// File content with mount information
    File {
        content: Vec<u8>,
        mount_path: PathBuf,
        permissions: u32, // Octal as decimal (e.g., 0o600 = 384)
    },
}

impl std::fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecretValue::Env(_) => write!(f, "Env([REDACTED])"),
            SecretValue::File { mount_path, permissions, .. } => {
                write!(f, "File(path={}, perms={:o})", mount_path.display(), permissions)
            }
        }
    }
}

/// Metadata about secret resolution
#[derive(Debug, Clone)]
pub struct SecretMetadata {
    pub source_type: SecretSourceType,
    pub resolved_from: ResolvedFrom,
    pub size_bytes: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum ResolvedFrom {
    ShellEnv,
    EnvLocalFile,
    EnvFile,
    FromFile(PathBuf),
    Vault { path: String, mount: String },
    LocalFile(PathBuf),
}
```

**Why `zeroize`?**

- Automatic memory zeroing on Drop prevents secrets from lingering in memory
- Critical for production security (prevents heap inspection attacks)
- No performance overhead (compiler optimizes to single memset)

### 3. Async Source Trait

```rust
// crates/sindri-secrets/src/sources/mod.rs

use async_trait::async_trait;
use anyhow::Result;

#[async_trait]
pub trait SecretSource: Send + Sync {
    /// Resolve a secret from this source
    async fn resolve(&self, definition: &SecretDefinition, ctx: &ResolutionContext)
        -> Result<Option<ResolvedSecret>>;

    /// Validate this source is available (e.g., vault CLI installed)
    fn validate(&self) -> Result<()>;

    /// Source name for error messages
    fn name(&self) -> &'static str;
}
```

**Why async?**

- Vault API calls are network I/O (async HTTP via reqwest)
- File I/O can be async (tokio::fs) for large files
- Environment variable reads are sync but wrapped for consistency
- Enables future parallel resolution of independent secrets

### 4. Resolution Strategy

**Precedence-Based Resolution**:

```rust
// crates/sindri-secrets/src/resolver.rs

use std::sync::Arc;
use tokio::sync::RwLock;

pub struct SecretResolver {
    sources: Vec<Box<dyn SecretSource>>,
    cache: Arc<RwLock<SecretCache>>,
    context: ResolutionContext,
}

impl SecretResolver {
    pub fn new(context: ResolutionContext) -> Self {
        let sources: Vec<Box<dyn SecretSource>> = vec![
            Box::new(EnvSource::new()),
            Box::new(FileSource::new()),
            Box::new(VaultSource::new()),
        ];

        Self {
            sources,
            cache: Arc::new(RwLock::new(SecretCache::new())),
            context,
        }
    }

    /// Resolve all secrets from definitions
    pub async fn resolve_all(&self, definitions: &[SecretDefinition])
        -> Result<Vec<ResolvedSecret>> {
        let mut resolved = Vec::new();
        let mut errors = Vec::new();

        for definition in definitions {
            match self.resolve_one(definition).await {
                Ok(Some(secret)) => {
                    resolved.push(secret);
                }
                Ok(None) => {
                    if definition.required {
                        errors.push(format!(
                            "Required secret '{}' not found",
                            definition.name
                        ));
                    }
                }
                Err(e) => {
                    errors.push(format!(
                        "Failed to resolve '{}': {}",
                        definition.name, e
                    ));
                }
            }
        }

        if !errors.is_empty() {
            bail!("Secret resolution failed:\n  - {}", errors.join("\n  - "));
        }

        Ok(resolved)
    }

    /// Resolve a single secret with caching
    async fn resolve_one(&self, definition: &SecretDefinition)
        -> Result<Option<ResolvedSecret>> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(&definition.name) {
                return Ok(Some(cached.clone()));
            }
        }

        // Try each source appropriate for the definition type
        let source = self.get_source_for_type(&definition.source)?;
        let resolved = source.resolve(definition, &self.context).await?;

        // Cache if resolved
        if let Some(ref secret) = resolved {
            let mut cache = self.cache.write().await;
            cache.insert(secret.clone());
        }

        Ok(resolved)
    }

    fn get_source_for_type(&self, source_type: &SecretSourceType)
        -> Result<&dyn SecretSource> {
        match source_type {
            SecretSourceType::Env => self.sources.iter()
                .find(|s| s.name() == "env")
                .map(|s| s.as_ref())
                .ok_or_else(|| anyhow!("Env source not found")),
            SecretSourceType::File => self.sources.iter()
                .find(|s| s.name() == "file")
                .map(|s| s.as_ref())
                .ok_or_else(|| anyhow!("File source not found")),
            SecretSourceType::Vault => self.sources.iter()
                .find(|s| s.name() == "vault")
                .map(|s| s.as_ref())
                .ok_or_else(|| anyhow!("Vault source not found")),
        }
    }
}
```

### 5. Source Implementations

See full implementation details in the agent output above for:

- **Env Source** (with precedence chain: shell env > .env.local > .env > fromFile)
- **File Source** (with security validation and path traversal prevention)
- **Vault Source** (with HTTP API and token validation)

### 6. Resolution Context

```rust
// crates/sindri-secrets/src/context.rs

use std::path::PathBuf;

/// Context for secret resolution
#[derive(Debug, Clone)]
pub struct ResolutionContext {
    /// Directory containing sindri.yaml (for relative path resolution)
    pub config_dir: PathBuf,

    /// Whether to allow optional secrets to fail silently
    pub allow_optional_failures: bool,

    /// Validation mode (don't actually resolve, just check availability)
    pub validation_mode: bool,
}

impl ResolutionContext {
    pub fn new(config_path: &Path) -> Result<Self> {
        let config_dir = config_path.parent()
            .ok_or_else(|| anyhow!("Cannot determine config directory"))?
            .to_path_buf();

        Ok(Self {
            config_dir,
            allow_optional_failures: true,
            validation_mode: false,
        })
    }

    pub fn with_validation_mode(mut self, mode: bool) -> Self {
        self.validation_mode = mode;
        self
    }
}
```

### 7. In-Memory Cache with Automatic Cleanup

```rust
// crates/sindri-secrets/src/cache.rs

use std::collections::HashMap;

/// In-memory cache with automatic zeroing on drop
pub struct SecretCache {
    secrets: HashMap<String, ResolvedSecret>,
}

impl SecretCache {
    pub fn new() -> Self {
        Self {
            secrets: HashMap::new(),
        }
    }

    pub fn insert(&mut self, secret: ResolvedSecret) {
        self.secrets.insert(secret.name.clone(), secret);
    }

    pub fn get(&self, name: &str) -> Option<&ResolvedSecret> {
        self.secrets.get(name)
    }

    pub fn clear(&mut self) {
        // SecretValue implements ZeroizeOnDrop, so values are automatically zeroed
        self.secrets.clear();
    }
}

impl Drop for SecretCache {
    fn drop(&mut self) {
        // Explicit clear for paranoia (values already zero on drop)
        self.clear();
    }
}
```

### 8. Provider Injection Interface

```rust
// crates/sindri-secrets/src/injection.rs

/// Provider-specific secret injection strategies
pub trait SecretInjector {
    /// Inject environment secrets into provider
    fn inject_env_secrets(&self, secrets: &[ResolvedSecret]) -> Result<()>;

    /// Inject file secrets into provider
    fn inject_file_secrets(&self, secrets: &[ResolvedSecret]) -> Result<()>;
}

// Providers implement this trait:
// impl SecretInjector for FlyProvider { ... }
// impl SecretInjector for DockerProvider { ... }
```

### 9. CLI Integration

```rust
// crates/sindri/src/commands/secrets.rs

use clap::Subcommand;

#[derive(Subcommand)]
pub enum SecretsCommands {
    /// Validate all secrets are resolvable
    Validate,

    /// List configured secrets
    List,

    /// Test Vault connection
    TestVault,

    /// Encode file to base64 for manual setting
    EncodeFile { file: PathBuf },
}

pub async fn run(cmd: SecretsCommands, config_path: PathBuf) -> Result<()> {
    let config = SindriConfig::load(&config_path)?;
    let context = ResolutionContext::new(&config_path)?
        .with_validation_mode(matches!(cmd, SecretsCommands::Validate));

    let resolver = SecretResolver::new(context);

    match cmd {
        SecretsCommands::Validate => {
            let resolved = resolver.resolve_all(&config.secrets).await?;

            println!("✓ All {} secret(s) validated successfully", resolved.len());
            for secret in resolved {
                println!("  ✓ {} ({})",
                    secret.name,
                    format_resolved_from(&secret.metadata.resolved_from)
                );
            }
        }

        SecretsCommands::List => {
            // ... list implementation
        }

        SecretsCommands::TestVault => {
            let vault = VaultSource::new();
            vault.validate()?;
            vault.validate_token().await?;
            println!("✓ Vault connection successful");
        }

        SecretsCommands::EncodeFile { file } => {
            // ... encode implementation
        }
    }

    Ok(())
}
```

## Consequences

### Positive

1. **Type Safety**: Compile-time validation of secret metadata and values
2. **Memory Security**: Automatic zeroing of sensitive data via `zeroize` crate
3. **Async Performance**: Parallel vault API calls, async file I/O
4. **Clear Errors**: Structured error propagation with context, no global state
5. **Extensibility**: Easy to add new sources (AWS Secrets Manager, GCP Secret Manager)
6. **Testability**: Mock sources for unit tests, no external dependencies
7. **Provider Decoupling**: Providers consume resolved secrets, no resolution logic
8. **Precedence Clarity**: Source-specific precedence rules encapsulated in implementations

### Negative

1. **Complexity**: 6 modules vs 1 bash script (823 lines → ~1500 lines Rust)
2. **Dependencies**: `zeroize`, `reqwest`, `tokio`, `shellexpand` (~500KB binary size)
3. **Async Overhead**: Tokio runtime required even for sync-only env secrets
4. **Cache Locking**: RwLock contention if resolving many secrets concurrently
5. **Learning Curve**: Understanding trait objects, async traits, zeroize semantics

### Neutral

1. **Caching Strategy**: In-memory only, no persistent cache (matches bash behavior)
2. **Vault HTTP**: Using reqwest instead of `vault` CLI (no subprocess overhead)
3. **String Interpolation**: Not implemented (bash: `${VAR}` expansion) - can add later

## Alternatives Considered

### 1. Sync-Only Resolution

**Rejected**: Async is required for Vault, and parallelization provides real performance benefit.

### 2. String-Based Secrets (No Zeroize)

**Rejected**: Security-first architecture requires zeroize.

### 3. Global Cache (Lazy Static)

**Rejected**: Explicit dependencies preferred, testability critical.

### 4. Vault CLI via subprocess

**Rejected**: HTTP API is faster, more testable, better for async.

## Design Decisions

### Should we use async or sync resolution?

**Decision**: **Async** (`async fn resolve()`)

### How do we handle secret interpolation (e.g., ${VAR})?

**Decision**: **Not implemented in Phase 5** - can be added later if needed

### What's the precedence order?

**Decision**: **shell env > .env.local > .env > fromFile > vault** (per source type)

### Should secrets be cached in memory?

**Decision**: **Yes, with RwLock and automatic zeroing**

## Compliance

- ✅ Multi-source resolution (env, file, vault)
- ✅ Precedence rules match bash implementation
- ✅ Security: zeroize, path validation, token validation
- ✅ Async vault API calls
- ✅ Provider-agnostic (injection interface)
- ✅ Error handling with context (no global state)
- ✅ CLI commands: validate, list, test-vault, encode-file

## Related Decisions

- [ADR-001: Rust Migration](001-rust-migration-workspace-architecture.md) - Workspace structure
- [ADR-002: Provider Abstraction](002-provider-abstraction-layer.md) - Provider injection interface
- [ADR-004: Async Runtime](004-async-runtime-command-execution.md) - Tokio usage patterns
