# Sindri v3 Provider Adapter Architecture Analysis

## Executive Summary

Sindri v3 is a Rust-based CLI that uses a **trait-based provider adapter pattern** to deploy development environments across multiple cloud platforms. The architecture is defined in the `v3/crates/sindri-providers/` crate, with each provider implementing the `Provider` trait from `traits.rs`. Configuration is driven by `sindri.yaml` (validated against `v3/schemas/sindri.schema.json`), and the CLI routes commands through a factory function `create_provider()` in `lib.rs`.

---

## 1. Provider Trait Contract

**File**: `v3/crates/sindri-providers/src/traits.rs`

Every provider must implement the `Provider` trait:

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &'static str;
    fn check_prerequisites(&self) -> Result<PrerequisiteStatus>;
    async fn deploy(&self, config: &SindriConfig, opts: DeployOptions) -> Result<DeployResult>;
    async fn connect(&self, config: &SindriConfig) -> Result<()>;
    async fn status(&self, config: &SindriConfig) -> Result<DeploymentStatus>;
    async fn destroy(&self, config: &SindriConfig, force: bool) -> Result<()>;
    async fn plan(&self, config: &SindriConfig) -> Result<DeploymentPlan>;
    async fn start(&self, config: &SindriConfig) -> Result<()>;
    async fn stop(&self, config: &SindriConfig) -> Result<()>;

    // Optional capability flags (defaults to false)
    fn supports_gpu(&self) -> bool { false }
    fn supports_auto_suspend(&self) -> bool { false }
}
```

### Required Methods (9 total)

| Method                  | Purpose                    | Returns                      |
| ----------------------- | -------------------------- | ---------------------------- |
| `name()`                | Provider identifier string | `&'static str`               |
| `check_prerequisites()` | Verify CLI tools and auth  | `Result<PrerequisiteStatus>` |
| `deploy()`              | Full deployment lifecycle  | `Result<DeployResult>`       |
| `connect()`             | Interactive shell access   | `Result<()>`                 |
| `status()`              | Current deployment state   | `Result<DeploymentStatus>`   |
| `destroy()`             | Remove all resources       | `Result<()>`                 |
| `plan()`                | Dry-run deployment plan    | `Result<DeploymentPlan>`     |
| `start()`               | Start a stopped deployment | `Result<()>`                 |
| `stop()`                | Stop a running deployment  | `Result<()>`                 |

### Optional Capability Methods (2)

| Method                    | Default | Providers that override                                       |
| ------------------------- | ------- | ------------------------------------------------------------- |
| `supports_gpu()`          | `false` | Fly (true), DevPod (true), K8s (true), Docker (runtime check) |
| `supports_auto_suspend()` | `false` | Fly (true), E2B (true)                                        |

### Factory Pattern

**File**: `v3/crates/sindri-providers/src/lib.rs`

```rust
pub fn create_provider(provider: ProviderType) -> Result<Box<dyn Provider>> {
    match provider {
        ProviderType::Docker | ProviderType::DockerCompose => {
            Ok(Box::new(docker::DockerProvider::new()?))
        }
        ProviderType::Fly => Ok(Box::new(fly::FlyProvider::new()?)),
        ProviderType::Devpod => Ok(Box::new(devpod::DevPodProvider::new()?)),
        ProviderType::E2b => Ok(Box::new(e2b::E2bProvider::new()?)),
        ProviderType::Kubernetes => Ok(Box::new(kubernetes::KubernetesProvider::new()?)),
    }
}
```

**To add a new provider**, you must:

1. Add the variant to `ProviderType` enum in `sindri-core/src/types/config_types.rs`
2. Create a new module file (e.g., `runpod.rs`) in `sindri-providers/src/`
3. Add the module declaration and match arm in `lib.rs`
4. Implement the `Provider` trait

---

## 2. Provider Struct Pattern

Every provider follows a consistent struct pattern:

```rust
pub struct XxxProvider {
    templates: TemplateRegistry,   // For config file generation (Tera templates)
    output_dir: PathBuf,           // Where generated files go
}

impl XxxProvider {
    pub fn new() -> Result<Self> { ... }
    pub fn with_output_dir(output_dir: PathBuf) -> Result<Self> { ... }
    // Private helper methods...
}
```

### Common Fields

- **`templates: TemplateRegistry`** -- Tera-based template engine for generating provider config files (docker-compose.yml, fly.toml, devcontainer.json, k8s-deployment.yaml, e2b.toml)
- **`output_dir: PathBuf`** -- Default: `std::env::current_dir()`, configurable via `with_output_dir()`

### Provider-Specific Internal Config Structs

Each provider extracts a typed config from `SindriConfig`:

| Provider   | Config Struct         | Key Fields                                                                                  |
| ---------- | --------------------- | ------------------------------------------------------------------------------------------- |
| Docker     | (inline)              | dind_mode, compose_path                                                                     |
| Fly        | `FlyDeployConfig<'a>` | region, organization, cpu_kind, cpus, memory_mb, ssh_port, auto_stop, volume_size, gpu_tier |
| DevPod     | (inline)              | provider_type, build_repository                                                             |
| E2B        | `E2bDeployConfig`     | template_alias, profile, cpus, memory_mb, timeout, auto_pause, auto_resume, team            |
| Kubernetes | `K8sDeployConfig<'a>` | namespace, storage_class, volume_size, gpu_enabled                                          |

---

## 3. Configuration Parsing

### SindriConfig Loading

**File**: `v3/crates/sindri-core/src/config.rs`

The CLI loads config via:

```rust
let config = SindriConfig::load(config_path)?;
```

This reads `sindri.yaml`, validates against the JSON schema, and provides typed access to all fields.

### Key Config Access Patterns

```rust
// Get inner file data
let file = config.inner();

// Get provider name
config.provider()  // Returns ProviderType enum

// Get deployment name
config.name()  // Returns &str

// Get resources
config.resources()  // Returns ResourceConfig

// Resolve image (with image_config support)
config.resolve_image(resolver).await  // Returns Result<String>

// Get secrets
config.secrets()  // Returns Vec<SecretConfig>
```

### Provider-Specific Config Access

Provider-specific config is nested under `providers`:

```rust
let file = config.inner();

// Fly.io config
let fly = file.providers.fly.as_ref();
let region = fly.map(|f| f.region.as_str()).unwrap_or("sjc");

// Docker config
let docker = file.providers.docker.as_ref();
let privileged = docker.map(|d| d.privileged).unwrap_or(false);

// Kubernetes config
let k8s = file.providers.kubernetes.as_ref();
let namespace = k8s.map(|k| k.namespace.as_str()).unwrap_or("default");

// E2B config
let e2b = file.providers.e2b.as_ref();
let timeout = e2b.map(|e| e.timeout).unwrap_or(300);

// DevPod config
let devpod = file.providers.devpod.as_ref();
let provider_type = devpod.map(|d| d.r#type.clone());
```

### Default Values Pattern

All providers use a consistent pattern for defaults:

```rust
let value = config_section
    .map(|c| c.field_name)
    .unwrap_or(DEFAULT_VALUE);
```

---

## 4. CLI Integration

### Provider Type Enum

**File**: `v3/crates/sindri-core/src/types/config_types.rs`

```rust
pub enum Provider {
    Docker,
    #[serde(alias = "docker-compose")]
    DockerCompose,
    Fly,
    Devpod,
    E2b,
    Kubernetes,
}
```

This enum is deserialized from `deployment.provider` in `sindri.yaml`.

### Schema Validation

**File**: `v3/schemas/sindri.schema.json`

The provider field is validated as:

```json
"provider": {
  "type": "string",
  "enum": ["fly", "kubernetes", "docker-compose", "docker", "devpod", "e2b"]
}
```

### Command Flow (deploy example)

**File**: `v3/crates/sindri/src/commands/deploy.rs`

```
1. SindriConfig::load(config_path)       -- Load and validate YAML
2. check_env_files(&config, ...)         -- Preflight check for .env files
3. config.resolve_image(resolver).await  -- Resolve image reference
4. create_provider(config.provider())    -- Factory creates the right provider
5. provider.check_prerequisites()        -- Verify tools installed
6. provider.deploy(&config, opts).await  -- Execute deployment
7. output::success/error(...)            -- Display results
```

All commands (`deploy`, `connect`, `status`, `destroy`) follow this pattern:

```
load config -> create provider -> check prerequisites -> call provider method -> display results
```

### CLI Arguments

**File**: `v3/crates/sindri/src/cli.rs`

The CLI uses clap for argument parsing. Key deploy args:

```rust
pub struct DeployArgs {
    pub force: bool,
    pub dry_run: bool,
    pub wait: bool,
    pub timeout: u64,
    pub skip_validation: bool,
    pub from_source: bool,
    pub skip_image_verification: bool,
    pub env_file: Option<String>,
}
```

---

## 5. Prerequisite Checks

### Pattern

Every provider implements `check_prerequisites()` following this pattern:

```rust
fn check_prerequisites(&self) -> Result<PrerequisiteStatus> {
    let mut missing = Vec::new();
    let mut available = Vec::new();

    // 1. Check CLI tool exists
    if command_exists("tool-name") {
        let version = get_command_version("tool-name", "--version")
            .unwrap_or_else(|_| "unknown".to_string());
        available.push(Prerequisite {
            name: "tool-name".to_string(),
            description: "Tool description".to_string(),
            install_hint: None,
            version: Some(version),
        });
    } else {
        missing.push(Prerequisite {
            name: "tool-name".to_string(),
            description: "Tool description".to_string(),
            install_hint: Some("Install instructions URL".to_string()),
            version: None,
        });
    }

    // 2. Check authentication (if applicable)
    if self.is_authenticated() { ... } else { missing.push(...) }

    // 3. Check API key (if applicable)
    if self.has_api_key() { ... } else { missing.push(...) }

    Ok(PrerequisiteStatus {
        satisfied: missing.is_empty(),
        missing,
        available,
    })
}
```

### Utility Functions

**File**: `v3/crates/sindri-providers/src/utils.rs`

```rust
pub fn command_exists(cmd: &str) -> bool;          // Uses which::which
pub fn get_command_version(cmd: &str, flag: &str) -> Result<String>;
```

### Prerequisites by Provider

| Provider   | Required CLI               | Auth Check           | API Key Check         |
| ---------- | -------------------------- | -------------------- | --------------------- |
| Docker     | `docker`, `docker compose` | N/A                  | N/A                   |
| Fly        | `flyctl`                   | `flyctl auth whoami` | N/A                   |
| DevPod     | `devpod`                   | N/A                  | N/A                   |
| E2B        | `e2b`                      | N/A                  | `E2B_API_KEY` env var |
| Kubernetes | `kubectl`                  | N/A                  | N/A                   |

---

## 6. Error Handling Patterns

### Error Types

All providers use `anyhow::Result` for error handling with contextual messages:

```rust
// Missing prerequisites
return Err(anyhow!("Missing prerequisites: {}", missing_names.join(", ")));

// Already exists
return Err(anyhow!("Container '{}' already exists. Use --force to recreate.", name));

// Command failure
if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr);
    return Err(anyhow!("Failed to start container: {}", stderr));
}

// Configuration errors
return Err(anyhow!(
    "No image configured. Please specify:\n\
    1. deployment.image or deployment.image_config in sindri.yaml, OR\n\
    2. Enable deployment.buildFromSource.enabled in sindri.yaml"
));
```

### Logging Patterns

Uses `tracing` crate consistently:

```rust
use tracing::{debug, info, warn};

info!("Deploying {} with Docker provider", name);
debug!("Running: docker compose -f {} {}", compose_file, args.join(" "));
warn!("Sysbox requested but not available on host");
```

### User Confirmation (Destroy)

**File**: `v3/crates/sindri/src/commands/destroy.rs`

```rust
if !args.force {
    let confirmed = Confirm::new()
        .with_prompt(format!("Are you sure you want to destroy '{}'?", config.name()))
        .default(false)
        .interact()?;
    if !confirmed {
        output::info("Cancelled");
        return Ok(());
    }
}
```

---

## 7. State Management

### No Persistent State Files

Unlike v2 (which used shell scripts and state files), v3 providers query live state directly from the platform API:

| Provider   | State Query Method                   |
| ---------- | ------------------------------------ |
| Docker     | `docker inspect`, `docker ps`        |
| Fly        | `flyctl machines list --json`        |
| DevPod     | `devpod status <name> --output json` |
| E2B        | `e2b sandbox list --json`            |
| Kubernetes | `kubectl get pods -o json`           |

### DeploymentState Enum

**File**: `v3/crates/sindri-core/src/types/provider_types.rs`

```rust
pub enum DeploymentState {
    NotDeployed,
    Creating,
    Running,
    Stopped,
    Suspended,  // Fly.io machines
    Paused,     // E2B sandboxes
    Error,
    Destroying,
    Unknown,
}
```

### Generated Artifacts

Providers generate config files in `output_dir`:

| Provider   | Generated Files                                          |
| ---------- | -------------------------------------------------------- |
| Docker     | `docker-compose.yml`, `.env.secrets`                     |
| Fly        | `fly.toml` (colocated with sindri.yaml)                  |
| DevPod     | `.devcontainer/devcontainer.json`                        |
| E2B        | `.e2b/template/e2b.Dockerfile`, `.e2b/template/e2b.toml` |
| Kubernetes | `k8s-deployment.yaml`                                    |

---

## 8. Output Formatting

### Output Module

**File**: `v3/crates/sindri/src/output.rs`

All user-facing output goes through the `output` module:

```rust
output::header("Deploying sindri to docker");
output::info("Using image: ghcr.io/org/sindri:latest");
output::success("Deployment complete");
output::error("Deployment failed");
output::warning("cosign not installed");
output::kv("SSH", "docker exec -it ...");
output::spinner("Destroying resources...");
```

### DeployResult Structure

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

pub struct ConnectionInfo {
    pub ssh_command: Option<String>,
    pub http_url: Option<String>,
    pub https_url: Option<String>,
    pub instructions: Option<String>,
}
```

### Status Display Pattern

```
Status: my-env
  Provider:    docker
  State:       Running
  Instance ID: abc123def456
  Image:       ghcr.io/org/sindri:latest

Resources:
  CPU:    45.2%
  Memory: 1.2 GB / 4.0 GB
```

---

## 9. Secrets Resolution

All providers follow the same pattern for secrets using `sindri-secrets` crate:

```rust
use sindri_secrets::{ResolutionContext, SecretResolver};

// 1. Get secrets from config
let secrets = config.secrets();

// 2. Create resolution context
let config_dir = config.config_path.parent();
let context = ResolutionContext::new(config_dir)
    .with_custom_env_file(custom_env_file);

// 3. Resolve all secrets
let resolver = SecretResolver::new(context);
let resolved = resolver.resolve_all(secrets).await?;

// 4. Provider-specific injection
// Docker: Write to .env.secrets file
// Fly: flyctl secrets import --stage
// DevPod: devcontainer.json containerEnv
// E2B: Dockerfile ENV statements
// K8s: Kubernetes Secret resource
```

---

## 10. Template System

### Template Registry

**File**: `v3/crates/sindri-providers/src/templates/mod.rs`

Templates are embedded at compile time using `include_str!()`:

```rust
tera.add_raw_template("docker-compose.yml", include_str!("docker-compose.yml.tera"))?;
tera.add_raw_template("fly.toml", include_str!("fly.toml.tera"))?;
tera.add_raw_template("devcontainer.json", include_str!("devcontainer.json.tera"))?;
tera.add_raw_template("k8s-deployment.yaml", include_str!("k8s-deployment.yaml.tera"))?;
tera.add_raw_template("e2b.toml", include_str!("e2b.toml.tera"))?;
```

### TemplateContext

**File**: `v3/crates/sindri-providers/src/templates/context.rs`

Shared context object that all templates use:

```rust
pub struct TemplateContext {
    pub name: String,
    pub profile: String,
    pub image: String,
    pub memory: String,
    pub cpus: u32,
    pub volume_size: String,
    pub gpu_enabled: bool,
    pub gpu_type: String,
    pub gpu_count: u32,
    pub custom_extensions: String,
    pub additional_extensions: String,
    pub skip_auto_install: bool,
    pub dind: DindConfig,
    pub runtime: Option<String>,
    pub privileged: bool,
    pub network_mode: String,
    pub extra_hosts: Vec<String>,
    pub ports: Vec<String>,
    pub has_secrets: bool,
    pub secrets_file: String,
    pub env_vars: HashMap<String, String>,  // Provider-specific extras
    pub ci_mode: bool,
}
```

### Adding a New Template

1. Create `new-provider-config.tera` in `v3/crates/sindri-providers/src/templates/`
2. Register it in `TemplateRegistry::new()` in `mod.rs`
3. Use `self.templates.render("new-provider-config", &context)?` in the provider

---

## 11. Image Resolution

### Priority Chain

Image resolution follows this priority:

1. `deployment.image_config` (registry-based resolution with version patterns)
2. `deployment.image` (static image reference)
3. Build from source (`deployment.buildFromSource.enabled`)

```rust
// In deploy command
let resolved_image = config.resolve_image(resolver).await?;
```

### Build-from-Source Flow

When no pre-built image is configured:

1. `fetch_sindri_build_context()` clones the Sindri GitHub repo
2. Selects `Dockerfile` (production) or `Dockerfile.dev` (development)
3. Builds locally with `docker build`
4. Tags as `sindri:{version}-{git_sha}`

---

## 12. Deploy Lifecycle (Common Pattern)

All providers follow the same deployment sequence:

```
1. check_prerequisites()
   - Verify CLI tools exist
   - Verify authentication

2. Resolve image
   - config.resolve_image() for pre-built
   - OR fetch_sindri_build_context() + docker build

3. Handle dry_run
   - Return early with plan if opts.dry_run

4. Check existing resources
   - If exists && !force: error
   - If exists && force: destroy first

5. Create infrastructure
   - Provider-specific (volumes, apps, namespaces, etc.)

6. Resolve and inject secrets
   - Provider-specific mechanism

7. Deploy
   - Provider-specific deployment command

8. Wait (if opts.wait)
   - Poll until running or timeout

9. Return DeployResult
   - Success, connection info, messages, warnings
```

---

## 13. Testing Patterns

### Unit Tests

Each provider module includes `#[cfg(test)] mod tests`:

```rust
#[test]
fn test_provider_creation() {
    let provider = XxxProvider::new().unwrap();
    assert_eq!(provider.name(), "xxx");
}

#[test]
fn test_check_prerequisites() {
    let provider = XxxProvider::new().unwrap();
    let result = provider.check_prerequisites();
    assert!(result.is_ok());
}

#[test]
fn test_supports_gpu() {
    let provider = XxxProvider::new().unwrap();
    assert!(provider.supports_gpu()); // or !
}

// Provider-specific deserialization tests
#[test]
fn test_api_response_deserialization() {
    let json = r#"{"id": "abc", "state": "running"}"#;
    let response: ApiResponse = serde_json::from_str(json).unwrap();
    assert_eq!(response.id, "abc");
}
```

### Test Categories

1. **Provider creation**: `new()` and `with_output_dir()` constructors
2. **Capability flags**: `supports_gpu()`, `supports_auto_suspend()`
3. **Prerequisites**: `check_prerequisites()` doesn't panic
4. **Parsing**: Memory/size string parsing (e.g., "2GB" -> 2048)
5. **Deserialization**: API response JSON parsing
6. **Utility functions**: `command_exists`, `get_command_version`

### Integration Tests

**File**: `v3/tests/integration/`

Minimal integration tests exist; most testing is at the unit level within each provider module.

---

## 14. Dependencies (Cargo.toml)

**File**: `v3/crates/sindri-providers/Cargo.toml`

```toml
[dependencies]
sindri-core = { workspace = true }
sindri-secrets = { workspace = true }
tokio = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
async-trait = "0.1"
serde = { workspace = true }
serde_json = { workspace = true }
serde_yaml_ng = { workspace = true }
tera = { workspace = true }
tracing = { workspace = true }
duct = { workspace = true }
which = "8.0"
chrono = { workspace = true }
dirs = { workspace = true }
base64 = "0.22"

[dev-dependencies]
tempfile = { workspace = true }
```

---

## 15. JSON Schema Structure

**File**: `v3/schemas/sindri.schema.json`

### Provider Section

Provider-specific config lives under `providers` at the top level:

```yaml
# sindri.yaml structure
version: "3.0"
name: my-env
deployment:
  provider: fly # Selects which provider to use
  image: ghcr.io/... # Optional pre-built image
  resources:
    memory: 4GB
    cpus: 2
    gpu:
      enabled: false
  volumes:
    workspace:
      size: 10GB
extensions:
  profile: minimal
providers: # Provider-specific overrides
  fly:
    region: sjc
    autoStopMachines: true
  docker:
    privileged: false
    dind:
      enabled: false
  kubernetes:
    namespace: default
  e2b:
    timeout: 300
  devpod:
    type: docker
```

### Adding a New Provider to Schema

Add to `providers` object:

```json
"runpod": {
  "type": "object",
  "properties": {
    "gpuType": { "type": "string" },
    "region": { "type": "string" },
    ...
  }
}
```

And add to the provider enum:

```json
"provider": {
  "type": "string",
  "enum": ["fly", "kubernetes", "docker-compose", "docker", "devpod", "e2b", "runpod", "northflank"]
}
```

---

## 16. Checklist for Adding a New Provider

### Core Implementation Files

1. **`v3/crates/sindri-core/src/types/config_types.rs`**
   - Add variant to `Provider` enum (e.g., `Runpod`, `Northflank`)
   - Add `Display` implementation
   - Add serde alias if needed (e.g., `#[serde(alias = "run-pod")]`)
   - Add provider-specific config struct (e.g., `RunpodConfig`)
   - Add field to `ProvidersConfig` struct

2. **`v3/crates/sindri-providers/src/runpod.rs`** (new file)
   - Define `RunpodProvider` struct
   - Implement `Provider` trait (all 9 required methods + 2 optional)
   - Define internal config struct (e.g., `RunpodDeployConfig`)
   - Define API response deserialization structs
   - Add unit tests

3. **`v3/crates/sindri-providers/src/lib.rs`**
   - Add `pub mod runpod;`
   - Add match arm in `create_provider()`

4. **`v3/crates/sindri-providers/src/templates/`** (if needed)
   - Add Tera template file (e.g., `runpod.toml.tera`)
   - Register in `TemplateRegistry::new()`

### Schema and Validation

5. **`v3/schemas/sindri.schema.json`**
   - Add provider name to `deployment.provider` enum
   - Add provider-specific config under `providers`

### Documentation

6. **`v3/docs/providers/RUNPOD.md`** (new file)
   - Prerequisites, setup, configuration, examples

### Optional

7. **Example configs** in `examples/`
8. **Integration tests** in `v3/tests/`
9. **CI/CD updates** in `.github/workflows/`

---

## 17. Existing Provider Comparison Matrix

| Feature            | Docker             | Fly                | DevPod            | E2B                   | Kubernetes          |
| ------------------ | ------------------ | ------------------ | ----------------- | --------------------- | ------------------- |
| GPU Support        | Runtime check      | Yes (A100, L40s)   | Yes (cloud)       | No                    | Yes (node selector) |
| Auto-suspend       | No                 | Yes                | No                | Yes (pause)           | No                  |
| Persistent Volumes | Docker volumes     | Fly volumes        | DevPod-managed    | No                    | PVCs                |
| SSH Access         | docker exec        | flyctl ssh / SSH   | devpod ssh        | N/A (terminal)        | kubectl exec        |
| Build from Source  | Yes                | Yes                | Yes               | Yes                   | No (image only)     |
| Secrets            | .env.secrets file  | flyctl secrets     | containerEnv      | Dockerfile ENV        | K8s Secret          |
| Template File      | docker-compose.yml | fly.toml           | devcontainer.json | e2b.toml + Dockerfile | k8s-deployment.yaml |
| CLI Tool           | docker             | flyctl             | devpod            | e2b                   | kubectl             |
| Auth Method        | N/A                | flyctl auth login  | N/A               | E2B_API_KEY env       | kubectl config      |
| Connect Method     | docker exec -it    | flyctl ssh console | devpod ssh        | e2b sandbox terminal  | kubectl exec -it    |

---

## 18. Key Architectural Decisions

1. **Async-first**: All provider methods are `async` using `tokio` runtime
2. **CLI subprocess**: Providers shell out to CLI tools (`docker`, `flyctl`, etc.) rather than using SDK libraries
3. **Template-based config**: Uses Tera templates to generate provider config files, not raw string building
4. **No persistent state**: Queries live platform state rather than maintaining local state files
5. **Secrets resolution**: Centralized secret resolution via `sindri-secrets` crate, provider-specific injection
6. **Image resolution**: Centralized image resolution with registry version resolution support
7. **Build-from-source**: All container-based providers can build Sindri from the GitHub repository
8. **Trait objects**: Providers are used as `Box<dyn Provider>` for polymorphic dispatch

---

## 19. Code Quality Observations

### Strengths

- Consistent trait-based abstraction makes adding providers predictable
- Good separation of concerns (core types, providers, CLI, templates)
- Comprehensive unit tests for parsing and deserialization
- Proper error messages with user-actionable hints
- Secrets are handled securely (.env.secrets gets 0600 permissions)

### Areas for New Providers

- Some parsing functions are duplicated (e.g., `parse_memory_to_mb` in fly.rs and e2b.rs) -- consider using shared utility
- Provider-specific config structs use borrowed vs owned strings inconsistently
- The `env_vars: HashMap<String, String>` in `TemplateContext` is used as a catch-all for provider-specific data

### Patterns to Follow

- Always implement `with_output_dir()` constructor for testability
- Always include `#[cfg(test)] mod tests` with creation, prerequisites, and capability tests
- Use `tracing::{debug, info, warn}` for all logging
- Return `anyhow::Result` with descriptive error messages
- Check prerequisites before any operation in `deploy()`
- Handle `opts.dry_run` early in `deploy()` by calling `plan()` and returning
- Clean up generated files in `destroy()`
