# ADR 029: Local Kubernetes Cluster Management Architecture

**Status**: Accepted
**Date**: 2026-01-23
**Deciders**: Core Team
**Related**: [ADR-002: Provider Abstraction](002-provider-abstraction-layer.md), [ADR-005: Provider-Specific](005-provider-specific-implementations.md)

## Context

The v2 bash CLI provides local Kubernetes cluster management via `sindri k8s` commands:

- Create/destroy kind and k3d clusters
- Multi-node cluster support
- K3d local registry integration
- Auto-detection of available tools

The v3 Rust CLI has a `kubernetes.rs` provider that deploys TO existing clusters, but lacks cluster lifecycle management. Users must manually create clusters before using `sindri deploy --provider kubernetes`.

**Problem**: No feature parity for local cluster creation/management in v3.

**Constraints**:

- No native Rust libraries for kind/k3d cluster management (both are Go CLIs)
- Must support Kubernetes 1.35+ (longest support window until Feb 2027)
- CI integration requires deterministic testing

## Decision

### Separate Crate Architecture

**Decision**: Create new `sindri-clusters` crate separate from `sindri-providers`

**Rationale**:

- **Separation of concerns**: Cluster lifecycle ≠ workload deployment
- **Optional dependency**: Not all users need local cluster management
- **Testability**: Independent unit testing
- **Feature flags**: Can be disabled for minimal builds

### Crate Structure

```
crates/sindri-clusters/
├── Cargo.toml
└── src/
    ├── lib.rs               # Module exports + factory
    ├── traits.rs            # ClusterProvider trait
    ├── kind.rs              # Kind provider (~350 lines)
    ├── k3d.rs               # K3d provider (~400 lines)
    ├── installer.rs         # Tool installation helpers
    ├── platform.rs          # OS/arch detection
    └── config.rs            # Cluster configuration types
```

### ClusterProvider Trait

**Decision**: Define trait similar to Provider but for cluster lifecycle

```rust
#[async_trait]
pub trait ClusterProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn check_installed(&self) -> bool;
    fn get_version(&self) -> Option<String>;
    async fn install(&self) -> Result<()>;
    async fn create(&self, config: &ClusterConfig) -> Result<ClusterInfo>;
    async fn destroy(&self, name: &str, force: bool) -> Result<()>;
    async fn exists(&self, name: &str) -> bool;
    async fn list(&self) -> Result<Vec<ClusterInfo>>;
    async fn status(&self, name: &str) -> Result<ClusterStatus>;
    async fn get_kubeconfig(&self, name: &str) -> Result<String>;
    fn context_name(&self, cluster_name: &str) -> String;
}
```

### CLI Shell-Out Pattern

**Decision**: Use `tokio::process::Command` for kind/k3d CLI orchestration

**Rationale**:

- No Rust-native alternatives exist
- v3 already uses this pattern in kubernetes.rs
- JSON output parsing for status/list (already implemented for k3d)
- Consistent with Provider implementations

### Multi-Node Configuration

**Decision**: Generate provider-specific config for multi-node clusters

**Kind**: Generate YAML config file

```yaml
kind: Cluster
apiVersion: kind.x-k8s.io/v1alpha4
nodes:
  - role: control-plane
  - role: worker # Added per node count
```

**K3d**: Use CLI flags

```bash
k3d cluster create <name> --agents <N-1>
```

### Context Naming Convention

**Decision**: Follow provider conventions exactly

- Kind: `kind-{cluster-name}`
- K3d: `k3d-{cluster-name}`

**Rationale**: Users expect standard context names for kubectl interop

### K3d Registry Integration

**Decision**: Support built-in k3d registry for local development

```bash
k3d cluster create <name> --registry-create <registry-name>:<port>
```

**Rationale**:

- Enables local image push without external registry
- Speeds up development workflow
- Maps to `localhost:<port>` for docker push

### CLI Commands

**Decision**: Add `sindri k8s` command group with subcommands

```
sindri k8s
├── create [--provider kind|k3d] [--name <name>] [--nodes <n>] [--k8s-version <ver>]
├── destroy [--name <name>] [--force]
├── list [--provider kind|k3d]
├── status [--name <name>]
├── config [--name <name>]
└── install <kind|k3d>
```

### Configuration in sindri.yaml

**Decision**: Add `providers.k8s` schema section

```yaml
providers:
  k8s:
    provider: kind # or k3d
    clusterName: sindri-local
    version: v1.35.0 # Latest LTS, supported until Feb 2027
    nodes: 1 # Multi-node support

    kind:
      image: kindest/node:v1.35.0
      configFile: kind-config.yaml # Optional custom config

    k3d:
      image: rancher/k3s:v1.35.0-k3s1
      registry:
        enabled: true
        name: k3d-registry
        port: 5000
```

### Integration with Kubernetes Provider

**Decision**: Update `sindri-providers/kubernetes.rs` to:

1. Suggest `sindri k8s create` when no cluster is available
2. Auto-detect local cluster type from context prefix
3. Provide actionable error messages for cluster setup

## Consequences

### Positive

1. Feature parity with v2 k8s commands
2. Clean separation from workload deployment
3. Testable in CI with matrix strategy
4. Cross-platform installation assistance
5. Consistent with existing v3 architecture patterns

### Negative

1. Additional crate to maintain (~1,200 lines)
2. Shell-out introduces external tool dependency
3. Version skew possible between CLI and cluster tools

### Trade-offs

- Chose shell-out over Go FFI: simpler, maintainable
- Chose separate crate over extending provider: cleaner architecture
- Chose K8s 1.35: longest support (Feb 2027) vs cutting-edge 1.36

## Implementation

### Files Created

- `crates/sindri-clusters/` - New crate
- `crates/sindri/src/commands/k8s.rs` - CLI commands
- `docs/architecture/adr/029-local-kubernetes-cluster-management.md` - This ADR

### Files Modified

- `v3/Cargo.toml` - Added sindri-clusters to workspace
- `crates/sindri/Cargo.toml` - Added sindri-clusters dependency
- `crates/sindri/src/cli.rs` - Added K8s command enum
- `crates/sindri/src/main.rs` - Added K8s command handler
- `crates/sindri-providers/src/kubernetes.rs` - Added cluster suggestions
- `schemas/sindri.schema.json` - Updated k8s version to v1.35.0

## References

- v2 implementation: `v2/deploy/adapters/k8s/`
- Kind releases: https://github.com/kubernetes-sigs/kind/releases
- K3d releases: https://github.com/k3d-io/k3d/releases
- K8s releases: https://kubernetes.io/releases/
