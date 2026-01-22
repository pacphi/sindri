# ADR 004: Async Runtime and Command Execution

**Status**: Accepted
**Date**: 2026-01-21
**Deciders**: Core Team
**Related**: [ADR-002: Provider Abstraction](002-provider-abstraction-layer.md)

## Context

Sindri's provider operations are fundamentally I/O bound:

- Shelling out to CLI tools (docker, flyctl, kubectl, devpod, e2b)
- Network API calls (Fly.io machines, E2B sandboxes, GitHub releases)
- File operations (reading configs, writing templates)
- Interactive sessions (SSH, docker exec)

The bash implementation executed these operations **synchronously**:

- Single-threaded, sequential execution
- No parallelization of independent operations
- Slow startup (parsing schemas, validating configs sequentially)

We needed an approach that:

1. Enables concurrent operations where possible
2. Maintains single-threaded simplicity where appropriate
3. Provides responsive user feedback during long operations
4. Supports both batch and interactive modes

## Decision

### Tokio Async Runtime

We adopt **Tokio** as the async runtime for all I/O operations.

```toml
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
```

### Async-First Provider Trait

All provider lifecycle methods are async:

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    async fn deploy(&self, config: &SindriConfig, opts: DeployOptions) -> Result<DeployResult>;
    async fn connect(&self, config: &SindriConfig) -> Result<()>;
    async fn status(&self, config: &SindriConfig) -> Result<DeploymentStatus>;
    async fn destroy(&self, config: &SindriConfig, force: bool) -> Result<()>;
    async fn plan(&self, config: &SindriConfig) -> Result<DeploymentPlan>;
    async fn start(&self, config: &SindriConfig) -> Result<()>;
    async fn stop(&self, config: &SindriConfig) -> Result<()>;
}
```

### Command Execution Patterns

**1. Tokio Process Spawning**

For CLI tool execution:

```rust
use tokio::process::Command;

let output = Command::new("docker")
    .args(["ps", "--format", "{{.Names}}"])
    .output()
    .await?;
```

**2. Interactive Commands**

For user interaction (SSH, shells):

```rust
let status = Command::new("docker")
    .args(["exec", "-it", name, "/bin/bash"])
    .stdin(Stdio::inherit())
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit())
    .status()
    .await?;
```

**3. Prerequisite Checks (Sync)**

For fast startup checks, we use sync std::process:

```rust
fn check_prerequisites(&self) -> Result<PrerequisiteStatus> {
    // Sync - must complete before async operations
    if command_exists("docker") {
        let version = get_command_version("docker", "--version")?;
        // ...
    }
}
```

### CLI Entry Point

Main function uses tokio runtime:

```rust
#[tokio::main]
async fn main() -> Result<()> {
    match cli.command {
        Commands::Deploy(args) => commands::deploy::run(args).await?,
        Commands::Connect(args) => commands::connect::run(args).await?,
        Commands::Status(args) => commands::status::run(args).await?,
        // ...
    }
    Ok(())
}
```

## Implementation Details

### Async Where Needed

**Async Operations:**

- Provider deploy (long-running builds, deploys)
- Provider connect (wait for wake-up, establish connection)
- Provider status (query remote APIs, parse JSON)
- Provider destroy (cleanup multiple resources)
- File I/O for large operations

**Sync Operations:**

- Config parsing (fast, local)
- Schema validation (fast, CPU-bound)
- Prerequisite checks (fast, local commands)
- Version display (instant)

### Concurrency Opportunities

While we currently execute operations sequentially, the async foundation enables:

**Future: Parallel Prerequisite Checks**

```rust
let (docker_check, compose_check) = tokio::join!(
    check_docker(),
    check_compose_v2(),
);
```

**Future: Concurrent Provider Operations**

```rust
// Deploy to multiple providers simultaneously
let (docker_result, fly_result) = tokio::join!(
    docker.deploy(config, opts),
    fly.deploy(config, opts),
);
```

**Future: Background Tasks**

```rust
// Start extension installation in background while connecting
tokio::spawn(async move {
    extension_manager.install_profile("ai-dev").await
});
```

### Error Handling

Async operations propagate errors via Result:

```rust
async fn deploy(&self, config: &SindriConfig, opts: DeployOptions) -> Result<DeployResult> {
    // Check prerequisites
    let prereqs = self.check_prerequisites()?;

    // Generate config
    let config_path = self.generate_config(config).await?;

    // Deploy
    self.run_deploy(&config_path).await?;

    // Wait if requested
    if opts.wait {
        self.wait_for_ready(opts.timeout).await?;
    }

    Ok(result)
}
```

### Timeouts and Cancellation

Operations support timeout via DeployOptions:

```rust
if opts.wait {
    let timeout = opts.timeout.unwrap_or(60);
    let start = std::time::Instant::now();
    while start.elapsed().as_secs() < timeout {
        if self.is_ready().await {
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
```

## Consequences

### Positive

1. **Performance**: Enables concurrent operations (future)
2. **Responsive**: Non-blocking I/O during long operations
3. **Scalable**: Can handle multiple deployments concurrently
4. **Modern**: Async/await syntax is ergonomic
5. **Ecosystem**: Access to rich async ecosystem (reqwest, etc.)
6. **Cancellation**: Can implement timeout/cancellation cleanly

### Negative

1. **Complexity**: Async runtime adds mental overhead
2. **Binary Size**: Tokio adds ~2MB to binary
3. **Compile Time**: Tokio increases build time
4. **Color Functions**: Async/sync boundaries require care
5. **Testing**: Async tests slightly more complex

### Trade-offs

**Tokio vs async-std**

- Chose Tokio (larger ecosystem, better maintained)
- Trade-off: Slightly larger binary vs. more features

**Full vs Minimal Features**

- Chose `tokio = { features = ["full"] }`
- Trade-off: Binary size vs. flexibility for future features
- Could optimize to minimal features in future

**Async All vs Selective**

- Chose async for all provider methods
- Trade-off: Some operations don't benefit, but consistent interface
- Alternative: Split trait into sync/async, but adds complexity

## Validation

### Current Performance

| Operation          | Bash   | Rust  | Improvement    |
| ------------------ | ------ | ----- | -------------- |
| Config parsing     | ~50ms  | ~5ms  | 10x faster     |
| Schema validation  | ~200ms | ~20ms | 10x faster     |
| Prerequisite check | ~100ms | ~10ms | 10x faster     |
| Template rendering | N/A    | ~1ms  | New capability |

### Future Concurrent Operations

With async foundation, we can enable:

- Parallel provider health checks
- Concurrent volume cleanup
- Background extension installation
- Multi-provider deployments

## Migration Notes

**From Bash to Rust Async**

Bash pattern:

```bash
docker compose up -d
docker exec -it $NAME /bin/bash
```

Rust async pattern:

```rust
self.docker_compose(&["up", "-d"], &compose_path).await?;
Command::new("docker")
    .args(["exec", "-it", name, "/bin/bash"])
    .status()
    .await?;
```

**Key Differences**

1. Must `.await` all async operations
2. Functions marked `async fn`
3. Error propagation via `?` works the same
4. `tokio::process::Command` vs `std::process::Command`

## Future Considerations

- Structured concurrency with tokio::task::JoinSet
- Progress tracking with tokio::sync::watch channels
- Rate limiting for API calls
- Connection pooling for repeated operations
- Graceful shutdown handling

## References

- Tokio Documentation: https://tokio.rs/
- async-trait: https://docs.rs/async-trait/
- Main entry: `crates/sindri/src/main.rs`
- Provider trait: `crates/sindri-providers/src/traits.rs`
- Command execution: `crates/sindri-providers/src/utils.rs`
