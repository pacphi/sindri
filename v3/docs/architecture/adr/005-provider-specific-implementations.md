# ADR 005: Provider-Specific Implementation Patterns

**Status**: Accepted
**Date**: 2026-01-21
**Deciders**: Core Team
**Related**: [ADR-002: Provider Abstraction](002-provider-abstraction-layer.md), [ADR-003: Templates](003-template-based-configuration.md)

## Context

While the Provider trait provides a unified interface, each cloud provider has unique characteristics requiring specific implementation patterns:

**Docker**

- DinD (Docker-in-Docker) requires runtime detection
- Multiple isolation modes (sysbox, privileged, socket)
- Volume cleanup complexity with compose project names
- GPU requires NVIDIA runtime

**Fly.io**

- Machines can be suspended (unique state)
- Auto-wake on connection required
- Memory in MB, not GB strings
- GPU only in specific regions (ord, sjc)

**E2B**

- Template-based deployment model
- Sandboxes identified by metadata, not names
- Pause time = 4s per 1GB RAM
- GPU explicitly unsupported (critical validation)

**DevPod**

- 7 different backend providers (AWS, GCP, Azure, DO, K8s, SSH, Docker)
- Image must be pushed to registries (except Docker backend)
- Provider-specific configuration required

**Kubernetes**

- Local cluster detection (kind, k3d)
- Image loading for local development
- Manifest application and rollout tracking
- GPU via node selectors

We needed consistent patterns for handling these provider-specific concerns.

## Decision

### Pattern 1: Runtime Capability Detection

**Problem**: Features depend on host environment (sysbox runtime, NVIDIA, etc.)

**Solution**: Detect capabilities at runtime and adapt configuration

**Docker DinD Example:**

```rust
fn detect_dind_mode(&self, config: &SindriConfig) -> String {
    let dind_enabled = config.dind_enabled();
    if !dind_enabled { return "none"; }

    let requested = config.dind_mode(); // "auto", "sysbox", "privileged", "socket"
    let has_sysbox = self.has_sysbox();
    let privileged_ok = config.privileged();

    match requested {
        "sysbox" if has_sysbox => "sysbox",
        "sysbox" => { warn!("Sysbox unavailable"); "none" },
        "auto" if has_sysbox => "sysbox",
        "auto" if privileged_ok => "privileged",
        "auto" => { warn!("No DinD available"); "none" },
        mode => mode,
    }
}
```

**Benefits**:

- Graceful degradation (sysbox → privileged → none)
- User warnings when features unavailable
- Deterministic behavior

### Pattern 2: Provider-Specific Validation

**Problem**: Not all providers support all features (E2B has no GPU)

**Solution**: Validate early and provide helpful error messages

**E2B GPU Validation:**

```rust
async fn deploy(&self, config: &SindriConfig, opts: DeployOptions) -> Result<DeployResult> {
    // Validate GPU early
    if config.gpu_enabled() {
        return Err(anyhow!(
            "E2B does not support GPU deployments.\n\
             Use Fly.io (A100, L40s), DevPod (cloud GPUs), \
             Docker (NVIDIA runtime), or Kubernetes (GPU nodes)."
        ));
    }
    // ... proceed with deployment
}
```

**Benefits**:

- Fail fast before creating resources
- Helpful error messages suggest alternatives
- Prevents partial deployments

### Pattern 3: State Machine Mapping

**Problem**: Providers use different state terminologies

**Solution**: Map provider states to common DeploymentState enum

**State Mapping:**

```rust
pub enum DeploymentState {
    NotDeployed,
    Creating,
    Running,
    Stopped,
    Paused,      // E2B, Docker
    Suspended,   // Fly.io machines
    Destroying,
    Unknown,
}
```

**Provider Mappings:**

| Provider State    | Sindri State |
| ----------------- | ------------ |
| Docker: "running" | Running      |
| Docker: "exited"  | Stopped      |
| Docker: "paused"  | Paused       |
| Fly: "started"    | Running      |
| Fly: "suspended"  | Suspended    |
| E2B: "running"    | Running      |
| E2B: "paused"     | Paused       |
| K8s: "Running"    | Running      |
| K8s: "Pending"    | Creating     |

**Implementation:**

```rust
async fn get_container_state(&self, name: &str) -> DeploymentState {
    let output = Command::new("docker")
        .args(["inspect", "-f", "{{.State.Status}}", name])
        .output()
        .await?;

    match String::from_utf8_lossy(&output.stdout).trim() {
        "running" => DeploymentState::Running,
        "exited" | "stopped" => DeploymentState::Stopped,
        "paused" => DeploymentState::Paused,
        "created" => DeploymentState::Creating,
        _ => DeploymentState::Unknown,
    }
}
```

### Pattern 4: Resource Cleanup Strategies

**Problem**: Providers leave different artifacts on destroy

**Solution**: Provider-specific cleanup chains

**Docker Cleanup (Most Complex):**

```rust
async fn destroy(&self, config: &SindriConfig, force: bool) -> Result<()> {
    // 1. Docker compose down (handles most cleanup)
    self.docker_compose(&["down", "--volumes", "--remove-orphans"]).await;

    // 2. Fallback: manual container removal
    if self.container_exists(name).await {
        Command::new("docker").args(["stop", name]).await;
        Command::new("docker").args(["rm", name]).await;
    }

    // 3. Volume cleanup (handles compose project name variations)
    self.cleanup_volumes(name, project_name).await?;

    // 4. Network cleanup
    self.cleanup_networks(name, project_name).await?;

    // 5. Remove generated files
    std::fs::remove_file("docker-compose.yml")?;
}
```

**Fly.io Cleanup (Simplest):**

```rust
async fn destroy(&self, config: &SindriConfig, force: bool) -> Result<()> {
    // Single command destroys app, machines, volumes, secrets
    Command::new("flyctl")
        .args(["apps", "destroy", name, "--yes"])
        .await?;

    std::fs::remove_file("fly.toml")?;
}
```

### Pattern 5: JSON API Integration

**Problem**: Providers expose JSON APIs for status queries

**Solution**: Deserialize with serde

**Fly.io Machine Status:**

```rust
#[derive(Debug, Deserialize)]
struct FlyMachine {
    id: String,
    state: String,
}

async fn get_machine_state(&self, app_name: &str) -> Result<DeploymentState> {
    let output = Command::new("flyctl")
        .args(["machines", "list", "-a", app_name, "--json"])
        .output()
        .await?;

    let machines: Vec<FlyMachine> = serde_json::from_str(&stdout)?;
    // Map machine.state to DeploymentState
}
```

### Pattern 6: Auto-Wake on Connect

**Problem**: Fly.io machines and E2B sandboxes auto-suspend

**Solution**: Check state and wake before connecting

**Fly.io Implementation:**

```rust
async fn connect(&self, config: &SindriConfig) -> Result<()> {
    let (machine_id, state) = self.get_machine_state(name).await?;

    // Wake suspended/stopped machines
    if matches!(state, DeploymentState::Suspended | DeploymentState::Stopped) {
        if let Some(id) = &machine_id {
            info!("Machine is {:?}, waking up...", state);
            self.start_machine(name, id).await?;
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    // Connect
    Command::new("flyctl")
        .args(["ssh", "console", "-a", name])
        .status()
        .await?;
}
```

### Pattern 7: Provider Configuration Extraction

**Problem**: Each provider has unique configuration in providers.{provider}

**Solution**: Provider-specific config struct extraction

**Example:**

```rust
fn get_fly_config<'a>(&self, config: &'a SindriConfig) -> FlyDeployConfig<'a> {
    let file = config.inner();
    let fly = file.providers.fly.as_ref();

    FlyDeployConfig {
        region: fly.map(|f| f.region.as_str()).unwrap_or("sjc"),
        ssh_port: fly.map(|f| f.ssh_port).unwrap_or(10022),
        auto_stop: fly.map(|f| f.auto_stop_machines).unwrap_or(true),
        // ... extract all fly-specific settings
    }
}
```

## Consequences

### Positive

1. **Graceful Degradation**: Docker DinD falls back to safer modes
2. **Early Validation**: GPU/feature checks before resource creation
3. **Clean Shutdown**: Proper cleanup prevents orphaned resources
4. **Auto-Recovery**: Suspended resources wake automatically
5. **Type Safety**: JSON deserialization catches API changes
6. **User Guidance**: Helpful error messages with alternatives

### Negative

1. **Complexity**: Each provider has custom logic
2. **Code Duplication**: Some patterns repeated (state mapping, JSON parsing)
3. **Testing**: Provider-specific tests needed for each pattern
4. **Maintenance**: Must track provider API changes

### Trade-offs

**Shared vs Provider-Specific Logic**

- Some duplication accepted for provider autonomy
- Shared utilities (command_exists, get_command_version) for common operations
- Provider-specific implementations for unique features

**Early vs Late Validation**

- Chose early validation (before resource creation)
- Trade-off: Fail fast vs. partial deployment recovery

## Provider-Specific Decisions

### Docker: DinD Priority Order

**Decision**: sysbox > privileged > socket > none

**Rationale**:

- Sysbox is most secure (user namespaces)
- Privileged is fallback for compatibility
- Socket shares host daemon (least isolation)
- None is safest default

### Fly.io: Memory Format

**Decision**: Convert GB strings to MB integers for API

**Rationale**:

- Fly.io API expects memory_mb as integer
- Users think in GB (sindri.yaml: memory: 4GB)
- Convert at provider boundary: 4GB → 4096

### E2B: Metadata-Based Discovery

**Decision**: Use sandbox metadata for name lookups

**Rationale**:

- E2B doesn't support human-readable names
- Sandboxes identified by random IDs
- Add metadata: {sindri_name: "my-env"}
- Query: find sandbox where metadata.sindri_name == "my-env"

### DevPod: Image Push Decision

**Decision**: Push to registry for all non-Docker backends

**Rationale**:

- Cloud backends can't access local Docker images
- Docker backend can use local images
- Check backend type before building/pushing

### Kubernetes: Local vs Remote Cluster Detection

**Decision**: Auto-detect cluster type via config

**Rationale**:

- kind/k3d require image loading
- Remote clusters use image pull
- Check providers.k8s.provider to determine

## Validation

All patterns tested across 5 providers:

- ✅ 17 provider-specific tests passing
- ✅ DinD mode detection tested with 4 scenarios
- ✅ State mapping validated for all providers
- ✅ Cleanup verified via integration tests
- ✅ JSON parsing tested with sample responses

## Future Enhancements

1. **Provider Health Checks**: Periodic checks for suspended resources
2. **Cost Tracking**: Integrate cloud provider billing APIs
3. **Multi-Region**: Deploy to multiple regions simultaneously
4. **Hybrid**: Deploy to Docker + Fly for local + cloud workflow
5. **Metrics**: Track deployment success rates per provider

## References

- Docker: `crates/sindri-providers/src/docker.rs:83-142` (DinD detection)
- Fly.io: `crates/sindri-providers/src/fly.rs:563-603` (auto-wake)
- E2B: `crates/sindri-providers/src/e2b.rs:457-521` (metadata discovery)
- DevPod: `crates/sindri-providers/src/devpod.rs:125-172` (backend detection)
- Kubernetes: `crates/sindri-providers/src/kubernetes.rs:105-127` (cluster type)
