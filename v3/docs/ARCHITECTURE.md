# Sindri v3 Architecture Overview

> **Version**: v3 (Rust CLI)
> **Last Updated**: 2026-01-26
> **Status**: Active Development

## Overview

Sindri v3 is a complete rewrite of the Sindri CLI from ~52,000 lines of Bash to ~11,200 lines of Rust, achieving a 78% code reduction while adding new capabilities. The architecture follows a multi-crate workspace design with clear separation of concerns, type-safe configuration handling, and async operations throughout.

### Design Philosophy

- **Type Safety**: Compile-time guarantees via Rust's type system and serde deserialization
- **Async/Await**: Tokio runtime for non-blocking I/O and parallel operations
- **Provider Abstraction**: Trait-based polymorphism for consistent cross-platform support
- **Security First**: Memory zeroing (zeroize), client-side encryption, audit logging, path validation
- **Testability**: Mock-based testing, comprehensive unit tests, CI integration

## Key Components

### 1. CLI Module (`sindri`)

The main binary crate orchestrating all CLI commands.

```
crates/sindri/
├── src/
│   ├── main.rs           # Entry point, Tokio runtime
│   ├── cli.rs            # Clap argument parsing
│   └── commands/         # Subcommand implementations
│       ├── deploy.rs     # sindri deploy
│       ├── config.rs     # sindri config init/validate
│       ├── extension.rs  # sindri extension install/list/upgrade
│       ├── secrets.rs    # sindri secrets validate/test-vault
│       ├── backup.rs     # sindri backup/restore
│       ├── k8s.rs        # sindri k8s create/destroy/list
│       └── packer.rs     # sindri vm build/validate
```

### 2. Provider Abstraction Layer (`sindri-providers`)

Async trait-based abstraction supporting five deployment providers with **standardized image handling**.

**Supported Providers**:

- **Docker**: Local container-based development with Docker Compose v2
  - **Image Handling**: Pre-built images OR local Dockerfile builds
- **Fly.io**: Edge deployment with Fly Machines API
  - **Image Handling**: Pre-built images OR server-side Dockerfile builds
- **DevPod**: Multi-backend development (Kubernetes, AWS, Docker)
  - **Image Handling**: Smart builds (cloud=build+push, local=dockerfile)
- **E2B**: Ephemeral cloud sandboxes for AI agents
  - **Image Handling**: Dockerfile-based template builds (required)
- **Kubernetes**: Production cluster deployments
  - **Image Handling**: Pre-built images only (no builds)

**Image Resolution Priority** (all providers):

1. `image_config.digest` - Immutable (production-safe)
2. `image_config.tag_override` - Explicit tag
3. `image_config.version` - Semantic version constraint
4. `image` - Legacy full reference
5. Local Dockerfile - Build on-demand (provider-dependent)
6. Default - `ghcr.io/pacphi/sindri:latest`

**Dockerfile Path Standardization**: All providers search in priority order:

- `./Dockerfile` (project root - default)
- `./v3/Dockerfile` (Sindri v3 specific - fallback)
- `./deploy/Dockerfile` (deploy-specific - fallback)

See [ADR-034](architecture/adr/034-image-handling-consistency-framework.md) and [ADR-035](architecture/adr/035-dockerfile-path-standardization.md) for details.

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    async fn deploy(&self, config: &SindriConfig) -> Result<DeployResult>;
    async fn status(&self, config: &SindriConfig) -> Result<StatusResult>;
    async fn connect(&self, config: &SindriConfig) -> Result<()>;
    async fn destroy(&self, config: &SindriConfig) -> Result<()>;
    fn check_prerequisites(&self) -> Result<PrerequisiteStatus>;
}
```

#### Image Resolution & Verification

**Image Resolution** (`sindri-core/src/config/loader.rs:185-297`):

- 6-level priority chain from digest to default
- Semantic version constraint resolution via `sindri-image` crate
- Registry API integration for tag enumeration
- GitHub token support for private registries

**Image Verification** (`sindri-image/src/verify.rs`):

- Cosign signature verification
- SLSA provenance attestation validation
- Certificate identity and OIDC issuer validation
- SBOM (Software Bill of Materials) fetching

**Dockerfile Discovery** (`sindri-providers/src/utils.rs`):

- Standardized path search across all providers
- Priority order: `./Dockerfile` → `./v3/Dockerfile` → `./deploy/Dockerfile`
- Clear error messages with searched paths

This architecture enables consistent image handling across all deployment providers while maintaining provider-specific optimization strategies.

### 3. Extension System (`sindri-extensions`)

YAML-first extension management with 80+ typed configurations.

**Core Features**:

- **DAG-based dependency resolution**: Topological sort with cycle detection
- **6 installation methods**: mise, apt, binary, npm, script, hybrid
- **Registry/Manifest dual-state**: Available vs installed extension tracking
- **Version lifecycle**: Enumeration, rollback, history tracking
- **Configure processing**: Template provisioning, environment variable management

```
crates/sindri-extensions/
├── src/
│   ├── registry.rs       # Extension registry (available extensions)
│   ├── manifest.rs       # Manifest management (installed state)
│   ├── resolver.rs       # Dependency resolution (DAG)
│   ├── executor.rs       # Installation execution
│   ├── distributor.rs    # GitHub release distribution
│   └── configure/        # Post-install configuration
│       ├── templates.rs  # File template processing
│       ├── environment.rs# Environment variable management
│       └── conditions.rs # Environment-based selection
```

### 4. Configuration Management (`sindri-core`)

Type-safe configuration loading, validation, and schema enforcement with **structured image handling**.

**Key Types**:

- `SindriConfig`: Root configuration from sindri.yaml
- `DeploymentConfig`: Provider-agnostic deployment settings with `image_config` support
- `ImageConfig`: Structured image configuration with semver resolution, verification
- `ExtensionConfig`: Extension system configuration
- `SecretsConfig`: Multi-source secret definitions

**Image Configuration Features**:

- Semantic versioning with constraint resolution (e.g., `^3.0.0`)
- Image signature verification via cosign
- SLSA provenance attestation verification
- Immutable digest pinning for production
- Pull policy control (Always, IfNotPresent, Never)

**Template Engine**: Tera templates for provider-specific configuration generation.

### 5. Secrets Management (`sindri-secrets`)

Multi-source async secret resolution with provider-agnostic injection.

**Secret Sources**:

- **Environment**: Shell env, .env files, fromFile references
- **File**: Certificates, keys, configs with mount paths
- **Vault**: HashiCorp Vault KV v1/v2 with token renewal
- **S3**: ChaCha20-Poly1305 + age encrypted storage

```rust
#[async_trait]
pub trait SecretSource: Send + Sync {
    async fn resolve(&self, definition: &SecretDefinition, ctx: &ResolutionContext)
        -> Result<Option<ResolvedSecret>>;
    fn validate(&self) -> Result<()>;
    fn name(&self) -> &'static str;
}
```

### 6. Backup/Restore System (`sindri-backup`)

Profile-based workspace backup with streaming tar.gz compression.

**Backup Profiles**:

| Profile   | Size      | Use Case                           |
| --------- | --------- | ---------------------------------- |
| user-data | 100MB-1GB | Migration to new provider          |
| standard  | 1-5GB     | Regular backups, disaster recovery |
| full      | 5-20GB    | Complete disaster recovery         |

**Restore Modes**: safe (non-destructive), merge (selective), full (complete overwrite)

## Architecture Decision Records

The v3 architecture is documented across 35 ADRs covering 8 development phases.

### Phase 1: Foundation

| ADR                                                                  | Title                                 | Status   |
| -------------------------------------------------------------------- | ------------------------------------- | -------- |
| [001](architecture/adr/001-rust-migration-workspace-architecture.md) | Rust Migration Workspace Architecture | Accepted |

### Phase 2: Provider Framework & Configuration

| ADR                                                                 | Title                                | Status   |
| ------------------------------------------------------------------- | ------------------------------------ | -------- |
| [002](architecture/adr/002-provider-abstraction-layer.md)           | Provider Abstraction Layer           | Accepted |
| [003](architecture/adr/003-template-based-configuration.md)         | Template-Based Configuration (Tera)  | Accepted |
| [004](architecture/adr/004-async-runtime-command-execution.md)      | Async Runtime Command Execution      | Accepted |
| [034](architecture/adr/034-image-handling-consistency-framework.md) | Image Handling Consistency Framework | Accepted |
| [035](architecture/adr/035-dockerfile-path-standardization.md)      | Dockerfile Path Standardization      | Accepted |

### Phase 3: Additional Providers

| ADR                                                              | Title                             | Status   |
| ---------------------------------------------------------------- | --------------------------------- | -------- |
| [005](architecture/adr/005-provider-specific-implementations.md) | Provider-Specific Implementations | Accepted |
| [006](architecture/adr/006-template-refactoring-consistency.md)  | Template Refactoring Consistency  | Accepted |
| [007](architecture/adr/007-phases-2-3-completion.md)             | Phases 2-3 Completion             | Accepted |

### Phase 4: Extension System

| ADR                                                                       | Title                                      | Status   |
| ------------------------------------------------------------------------- | ------------------------------------------ | -------- |
| [008](architecture/adr/008-extension-type-system-yaml-deserialization.md) | Extension Type System YAML Deserialization | Accepted |
| [009](architecture/adr/009-dependency-resolution-dag-topological-sort.md) | Dependency Resolution DAG Topological Sort | Accepted |
| [010](architecture/adr/010-github-extension-distribution.md)              | GitHub Extension Distribution              | Accepted |
| [011](architecture/adr/011-multi-method-extension-installation.md)        | Multi-Method Extension Installation        | Accepted |
| [012](architecture/adr/012-registry-manifest-dual-state-architecture.md)  | Registry Manifest Dual-State Architecture  | Accepted |
| [013](architecture/adr/013-schema-validation-strategy.md)                 | Schema Validation Strategy                 | Accepted |
| [014](architecture/adr/014-sbom-generation-industry-standards.md)         | SBOM Generation (CycloneDX/SPDX)           | Accepted |
| [026](architecture/adr/026-extension-version-lifecycle-management.md)     | Extension Version Lifecycle Management     | Accepted |
| [032](architecture/adr/032-extension-configure-processing.md)             | Extension Configure Processing             | Accepted |
| [033](architecture/adr/033-environment-based-template-selection.md)       | Environment-Based Template Selection       | Accepted |

### Phase 5: Secrets and Backup

| ADR                                                               | Title                              | Status   |
| ----------------------------------------------------------------- | ---------------------------------- | -------- |
| [015](architecture/adr/015-secrets-resolver-core-architecture.md) | Secrets Resolver Core Architecture | Accepted |
| [016](architecture/adr/016-vault-integration-architecture.md)     | Vault Integration Architecture     | Accepted |
| [017](architecture/adr/017-backup-system-architecture.md)         | Backup System Architecture         | Accepted |
| [018](architecture/adr/018-restore-system-architecture.md)        | Restore System Architecture        | Accepted |
| [019](architecture/adr/019-phase-5-secrets-backup-integration.md) | Phase 5 Integration Strategy       | Accepted |
| [020](architecture/adr/020-s3-encrypted-secret-storage.md)        | S3 Encrypted Secret Storage        | Proposed |

### Phase 6: CI/CD and Self-Update

| ADR                                                               | Title                             | Status   |
| ----------------------------------------------------------------- | --------------------------------- | -------- |
| [021](architecture/adr/021-bifurcated-ci-cd-v2-v3.md)             | Bifurcated CI/CD Pipeline (v2/v3) | Accepted |
| [022](architecture/adr/022-phase-6-self-update-implementation.md) | Self-Update Implementation        | Accepted |

### Phase 7: Project Management

| ADR                                                                    | Title                                    | Status   |
| ---------------------------------------------------------------------- | ---------------------------------------- | -------- |
| [023](architecture/adr/023-phase-7-project-management-architecture.md) | Project Management Architecture          | Accepted |
| [024](architecture/adr/024-template-based-project-scaffolding.md)      | Template-Based Project Scaffolding       | Accepted |
| [025](architecture/adr/025-git-operations-repository-management.md)    | Git Operations and Repository Management | Accepted |

### Phase 8: Tool Dependencies

| ADR                                                              | Title                             | Status   |
| ---------------------------------------------------------------- | --------------------------------- | -------- |
| [027](architecture/adr/027-tool-dependency-management-system.md) | Tool Dependency Management System | Accepted |
| [028](architecture/adr/028-config-init-template-generation.md)   | Config Init Template Generation   | Accepted |

### Kubernetes and VM Provisioning

| ADR                                                                | Title                               | Status   |
| ------------------------------------------------------------------ | ----------------------------------- | -------- |
| [029](architecture/adr/029-local-kubernetes-cluster-management.md) | Local Kubernetes Cluster Management | Accepted |
| [030](architecture/adr/030-kubernetes-ci-integration-testing.md)   | Kubernetes CI Integration Testing   | Accepted |
| [031](architecture/adr/031-packer-vm-provisioning-architecture.md) | Packer VM Provisioning Architecture | Accepted |

## Design Principles

### Type Safety

All configuration is deserialized through strongly-typed Rust structs with serde:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ExtensionDefinition {
    pub name: String,
    pub version: String,
    pub install: InstallConfig,
    #[serde(default)]
    pub dependencies: Vec<String>,
}
```

### Async/Await

Tokio runtime enables non-blocking operations and parallel execution:

```rust
// Parallel secret resolution
let secrets = join_all(
    definitions.iter().map(|def| resolver.resolve_one(def))
).await;

// Parallel prerequisite checking
let statuses = join_all(
    tools.iter().map(|tool| checker.check_tool(tool))
).await;
```

### Security

- **Memory Zeroing**: Secrets use `zeroize` crate for automatic cleanup
- **Path Validation**: Multi-layer validation prevents traversal attacks
- **Encryption**: ChaCha20-Poly1305 + age for at-rest encryption
- **Audit Logging**: Structured logging for security events

### Provider Abstraction

Clean separation between provider-agnostic core and provider-specific implementations:

```
sindri-core (abstractions)
    |
    v
sindri-providers (implementations)
    |
    +-- docker.rs
    +-- fly.rs
    +-- devpod.rs
    +-- e2b.rs
    +-- kubernetes.rs
```

## Crate Dependency Graph

```
                    sindri (binary)
                         |
         +---------------+---------------+
         |               |               |
    sindri-core    sindri-providers  sindri-extensions
         |               |               |
         +-------+-------+               |
                 |                       |
            sindri-secrets          sindri-backup
                 |
            sindri-update
                 |
         +-------+-------+
         |               |
    sindri-clusters  sindri-packer
         |               |
    sindri-project  sindri-doctor
```

## Key Dependencies

| Crate            | Purpose                           |
| ---------------- | --------------------------------- |
| tokio            | Async runtime                     |
| clap             | CLI argument parsing              |
| serde/serde_yaml | Configuration (de)serialization   |
| tera             | Template rendering                |
| git2             | Git operations (libgit2 bindings) |
| reqwest          | HTTP client for APIs              |
| vaultrs          | HashiCorp Vault client            |
| tar/flate2       | Backup compression                |
| zeroize          | Secure memory handling            |
| semver           | Version parsing and comparison    |

## See Also

- [ADR Index](architecture/adr/README.md) - Complete ADR listing with phase breakdown
- [CLI Reference](CLI.md) - Command-line interface documentation
- [Configuration Guide](CONFIGURATION.md) - sindri.yaml schema reference
- [Secrets Management](SECRETS_MANAGEMENT.md) - Multi-source secret resolution
- [Backup/Restore](BACKUP_RESTORE.md) - Workspace backup operations
- [Extension Development](./extensions/README.md) - Creating custom extensions

---

**Statistics**:

- **Total ADRs**: 35
- **ADR Documentation**: ~15,000 lines, ~490KB
- **Phases Covered**: 1-8 + K8s Cluster Management + Packer VM Provisioning
- **Implementation Status**: Phases 1-8 complete
