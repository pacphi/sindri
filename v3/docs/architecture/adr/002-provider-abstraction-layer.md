# ADR 002: Provider Abstraction Layer Design

**Status**: Accepted
**Date**: 2026-01-21
**Deciders**: Core Team
**Related**: [ADR-001: Rust Migration](001-rust-migration-workspace-architecture.md)

## Context

Sindri supports deployment to 5 different cloud providers:

- **Docker** (local development)
- **Fly.io** (cloud VMs with auto-suspend)
- **DevPod** (multi-cloud dev environments)
- **E2B** (cloud sandboxes)
- **Kubernetes** (container orchestration)

Each provider has different:

- CLI tools (docker, flyctl, devpod, e2b, kubectl)
- Resource models (containers, machines, sandboxes, pods)
- Lifecycle operations (create, pause, suspend, scale)
- Configuration formats (YAML, TOML, JSON)

We needed a unified abstraction that:

1. Allows seamless provider switching via sindri.yaml
2. Provides consistent CLI experience across providers
3. Enables provider-specific optimizations
4. Supports async operations for performance

## Decision

### Provider Trait Design

We define a **single async trait** that all providers must implement:

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    // Identity
    fn name(&self) -> &'static str;

    // Prerequisites
    fn check_prerequisites(&self) -> Result<PrerequisiteStatus>;

    // Core lifecycle (async)
    async fn deploy(&self, config: &SindriConfig, opts: DeployOptions) -> Result<DeployResult>;
    async fn connect(&self, config: &SindriConfig) -> Result<()>;
    async fn status(&self, config: &SindriConfig) -> Result<DeploymentStatus>;
    async fn destroy(&self, config: &SindriConfig, force: bool) -> Result<()>;

    // Planning and control
    async fn plan(&self, config: &SindriConfig) -> Result<DeploymentPlan>;
    async fn start(&self, config: &SindriConfig) -> Result<()>;
    async fn stop(&self, config: &SindriConfig) -> Result<()>;

    // Capabilities (sync)
    fn supports_gpu(&self) -> bool { false }
    fn supports_auto_suspend(&self) -> bool { false }
}
```

### Provider Factory

Dynamic provider selection based on config:

```rust
pub fn create_provider(provider_type: ProviderType) -> Result<Box<dyn Provider>> {
    match provider_type {
        ProviderType::Docker | ProviderType::DockerCompose =>
            Ok(Box::new(DockerProvider::new())),
        ProviderType::Fly =>
            Ok(Box::new(FlyProvider::new())),
        ProviderType::Devpod =>
            Ok(Box::new(DevPodProvider::new())),
        ProviderType::E2b =>
            Ok(Box::new(E2bProvider::new())),
        ProviderType::Kubernetes =>
            Ok(Box::new(KubernetesProvider::new())),
    }
}
```

### Common Types

All providers use shared types from `sindri-core`:

**DeployOptions**

```rust
pub struct DeployOptions {
    pub force: bool,
    pub dry_run: bool,
    pub wait: bool,
    pub timeout: Option<u64>,
    pub skip_validation: bool,
    pub verbose: bool,
}
```

**DeployResult**

```rust
pub struct DeployResult {
    pub success: bool,
    pub name: String,
    pub provider: String,
    pub instance_id: Option<String>,
    pub connection: Option<ConnectionInfo>,
    pub messages: Vec<String>,
    pub warnings: Vec<String>,
}
```

**DeploymentStatus**

```rust
pub struct DeploymentStatus {
    pub name: String,
    pub provider: String,
    pub state: DeploymentState,
    pub instance_id: Option<String>,
    pub image: Option<String>,
    pub addresses: Vec<String>,
    pub resources: Option<ResourceUsage>,
    pub timestamps: DeploymentTimestamps,
    pub details: HashMap<String, String>,
}
```

### Provider-Specific State

Each provider maintains:

- **TemplateRegistry**: For rendering provider-specific configs
- **output_dir**: Where to write generated files (docker-compose.yml, fly.toml, etc.)

Example:

```rust
pub struct DockerProvider {
    templates: TemplateRegistry,
    output_dir: PathBuf,
}
```

## Implementation Details

### Docker Provider

- Manages docker-compose.yml via Tera templates
- DinD mode detection (sysbox > privileged > socket > none)
- Volume and network cleanup on destroy
- Resource usage monitoring via `docker stats`

### Fly.io Provider

- Manages fly.toml via Tera templates
- App, volume, and machine lifecycle
- Auto-wake suspended machines on connect
- GPU tier mapping (A100, L40s)

### E2B Provider

- Manages e2b.toml via Tera templates
- Template building and caching
- Sandbox pause/resume (4s per 1GB RAM)
- WebSocket PTY connection
- GPU explicitly blocked with helpful error

### DevPod Provider

- Manages devcontainer.json via Tera templates
- Multi-cloud backend support (7 providers)
- Image build and registry push
- SSH-based workspace connection

### Kubernetes Provider

- Manages k8s-deployment.yaml via Tera templates
- Local cluster support (kind, k3d)
- Image loading for local development
- Namespace and PVC management

## Consequences

### Positive

1. **Unified Interface**: Single trait for all providers
2. **Testable**: Each provider can be unit tested independently
3. **Extensible**: New providers just implement the trait
4. **Type Safe**: Compiler enforces interface compliance
5. **Async**: Non-blocking operations for better UX
6. **Consistent**: All providers return same types
7. **Discoverable**: Trait methods show what's available

### Negative

1. **Trait Object Overhead**: Minor runtime cost for dynamic dispatch
2. **Async Complexity**: All providers must be async-aware
3. **Common Types**: Must balance flexibility vs. provider-specific features

### Trade-offs

**Trait Object vs Generics**

- Chose trait objects (`Box<dyn Provider>`) over generics
- Reason: Simpler factory pattern, runtime provider selection
- Trade-off: Small runtime cost vs. compile-time monomorphization

**Sync vs Async**

- Chose async trait for all lifecycle methods
- Reason: Deploy operations are I/O bound (network, CLI calls)
- Trade-off: Added tokio dependency, but enables concurrent operations

**Capabilities Pattern**

- Chose default trait methods for capabilities (supports_gpu, supports_auto_suspend)
- Reason: Not all providers support all features
- Trade-off: Runtime checks vs. compile-time type system

## Validation

All 5 providers successfully implement the trait:

- ✅ Docker: 864 LOC, full DinD support
- ✅ Fly.io: 855 LOC, auto-suspend machines
- ✅ E2B: 994 LOC, sandbox lifecycle
- ✅ DevPod: 945 LOC, 7 cloud backends
- ✅ Kubernetes: 948 LOC, kind/k3d support

17 provider tests passing, 100% interface compliance.

## Future Considerations

- Provider plugins via dynamic library loading (if needed for extensibility)
- Provider-specific optimizations (connection pooling, caching)
- Multi-provider deployments (deploy to Docker + Fly simultaneously)
- Provider migration tools (Docker → Fly, etc.)

## References

- Implementation: `crates/sindri-providers/src/traits.rs`
- Providers: `crates/sindri-providers/src/{docker,fly,devpod,e2b,kubernetes}.rs`
- Usage: `crates/sindri/src/commands/deploy.rs`
