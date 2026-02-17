# JSON Schema & Config Type Extensions for RunPod and Northflank

## 1. Overview

This document specifies all changes needed to the JSON schema (`sindri.schema.json`) and
Rust config types (`config_types.rs`) to support RunPod and Northflank providers. All changes
follow existing patterns established by Fly, Docker, Kubernetes, E2B, and DevPod providers.

---

## 2. Provider Enum Changes

### 2.1 JSON Schema (`v3/schemas/sindri.schema.json`)

**Current** (line 24):

```json
"provider": {
  "type": "string",
  "enum": ["fly", "kubernetes", "docker-compose", "docker", "devpod", "e2b"],
  "description": "Deployment provider (docker is an alias for docker-compose)"
}
```

**Updated**:

```json
"provider": {
  "type": "string",
  "enum": ["fly", "kubernetes", "docker-compose", "docker", "devpod", "e2b", "runpod", "northflank"],
  "description": "Deployment provider (docker is an alias for docker-compose)"
}
```

### 2.2 Rust Enum (`v3/crates/sindri-core/src/types/config_types.rs`)

**Add to `Provider` enum** (after line 178):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Provider {
    Docker,
    #[serde(alias = "docker-compose")]
    DockerCompose,
    Fly,
    Devpod,
    E2b,
    Kubernetes,
    Runpod,        // NEW
    Northflank,    // NEW
}
```

**Add to `Display` impl** (after line 190):

```rust
impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // ... existing ...
            Provider::Runpod => write!(f, "runpod"),
            Provider::Northflank => write!(f, "northflank"),
        }
    }
}
```

**Add to `normalized()` method** (after line 202):

```rust
impl Provider {
    pub fn normalized(&self) -> &str {
        match self {
            // ... existing ...
            Provider::Runpod => "runpod",
            Provider::Northflank => "northflank",
        }
    }

    pub fn supports_gpu(&self) -> bool {
        matches!(
            self,
            Provider::Docker
                | Provider::DockerCompose
                | Provider::Fly
                | Provider::Devpod
                | Provider::Kubernetes
                | Provider::Runpod        // NEW
                | Provider::Northflank    // NEW
        )
    }
}
```

---

## 3. RunPod Provider Config Schema

### 3.1 JSON Schema

Add inside `"providers"` object (after `"e2b"` section, around line 706):

```json
"runpod": {
  "type": "object",
  "description": "RunPod GPU cloud provider options",
  "properties": {
    "gpuTypeId": {
      "type": "string",
      "description": "RunPod GPU type identifier (e.g., 'NVIDIA RTX A4000', 'NVIDIA A100 80GB PCIe'). If omitted, derived from deployment.resources.gpu.tier."
    },
    "containerDiskGb": {
      "type": "integer",
      "minimum": 1,
      "maximum": 500,
      "default": 20,
      "description": "Container disk size in GB (temporary, lost on pod termination)"
    },
    "cloudType": {
      "type": "string",
      "enum": ["SECURE", "COMMUNITY"],
      "default": "COMMUNITY",
      "description": "Cloud type: SECURE (dedicated) or COMMUNITY (shared, cheaper)"
    },
    "region": {
      "type": "string",
      "description": "Datacenter region ID (optional, auto-selected if omitted)"
    },
    "exposePorts": {
      "type": "array",
      "items": {
        "type": "string",
        "pattern": "^\\d+$"
      },
      "default": [],
      "description": "HTTP ports to expose via RunPod proxy URLs (e.g., ['8080', '3000'])"
    },
    "spotBid": {
      "type": "number",
      "minimum": 0,
      "description": "Spot instance bid price ($/hr). Omit or 0 for on-demand pricing."
    },
    "startSsh": {
      "type": "boolean",
      "default": true,
      "description": "Enable SSH access on pod creation"
    }
  },
  "additionalProperties": false
}
```

### 3.2 Rust Config Struct

Add to `config_types.rs`:

```rust
/// RunPod provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunpodProviderConfig {
    /// RunPod GPU type identifier (e.g., "NVIDIA RTX A4000")
    #[serde(default, rename = "gpuTypeId")]
    pub gpu_type_id: Option<String>,

    /// Container disk size in GB
    #[serde(default = "default_runpod_container_disk", rename = "containerDiskGb")]
    pub container_disk_gb: u32,

    /// Cloud type: SECURE or COMMUNITY
    #[serde(default = "default_cloud_type", rename = "cloudType")]
    pub cloud_type: String,

    /// Datacenter region ID (optional)
    #[serde(default)]
    pub region: Option<String>,

    /// HTTP ports to expose via proxy
    #[serde(default, rename = "exposePorts")]
    pub expose_ports: Vec<String>,

    /// Spot instance bid price (None = on-demand)
    #[serde(default, rename = "spotBid")]
    pub spot_bid: Option<f64>,

    /// Enable SSH access
    #[serde(default = "default_true", rename = "startSsh")]
    pub start_ssh: bool,
}

fn default_runpod_container_disk() -> u32 {
    20
}

fn default_cloud_type() -> String {
    "COMMUNITY".to_string()
}
```

### 3.3 Add to ProvidersConfig

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProvidersConfig {
    // ... existing fields ...

    /// RunPod specific config
    #[serde(default)]
    pub runpod: Option<RunpodProviderConfig>,

    /// Northflank specific config
    #[serde(default)]
    pub northflank: Option<NorthflankProviderConfig>,
}
```

---

## 4. Northflank Provider Config Schema

### 4.1 JSON Schema

Add inside `"providers"` object (after `"runpod"` section):

```json
"northflank": {
  "type": "object",
  "description": "Northflank PaaS provider options",
  "properties": {
    "projectName": {
      "type": "string",
      "pattern": "^[a-z][a-z0-9-]*$",
      "description": "Northflank project name (default: 'sindri-{name}')"
    },
    "serviceName": {
      "type": "string",
      "pattern": "^[a-z][a-z0-9-]*$",
      "description": "Northflank service name (default: sindri.yaml name)"
    },
    "computePlan": {
      "type": "string",
      "enum": ["nf-compute-10", "nf-compute-20", "nf-compute-50", "nf-compute-100", "nf-compute-200"],
      "description": "Compute plan (auto-selected from resources if omitted)"
    },
    "gpuType": {
      "type": "string",
      "description": "GPU type identifier (e.g., 'nvidia-a100', 'nvidia-a10g')"
    },
    "instances": {
      "type": "integer",
      "minimum": 1,
      "maximum": 10,
      "default": 1,
      "description": "Number of deployment instances"
    },
    "region": {
      "type": "string",
      "description": "Deployment region (optional)"
    },
    "ports": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["name", "internalPort"],
        "properties": {
          "name": {
            "type": "string",
            "description": "Port name identifier"
          },
          "internalPort": {
            "type": "integer",
            "minimum": 1,
            "maximum": 65535,
            "description": "Container-internal port number"
          },
          "public": {
            "type": "boolean",
            "default": false,
            "description": "Expose port publicly with auto-TLS"
          },
          "protocol": {
            "type": "string",
            "enum": ["TCP", "HTTP", "UDP"],
            "default": "TCP",
            "description": "Port protocol"
          }
        }
      },
      "description": "Port configuration (SSH port 22 is always included automatically)"
    },
    "healthCheck": {
      "type": "object",
      "description": "HTTP health check configuration",
      "properties": {
        "path": {
          "type": "string",
          "default": "/health",
          "description": "HTTP path to check"
        },
        "port": {
          "type": "integer",
          "minimum": 1,
          "maximum": 65535,
          "description": "Port to check"
        },
        "intervalSecs": {
          "type": "integer",
          "minimum": 5,
          "maximum": 300,
          "default": 30,
          "description": "Check interval in seconds"
        },
        "timeoutSecs": {
          "type": "integer",
          "minimum": 1,
          "maximum": 60,
          "default": 5,
          "description": "Check timeout in seconds"
        }
      }
    },
    "autoScaling": {
      "type": "object",
      "description": "Auto-scaling configuration",
      "properties": {
        "minInstances": {
          "type": "integer",
          "minimum": 1,
          "default": 1,
          "description": "Minimum number of instances"
        },
        "maxInstances": {
          "type": "integer",
          "minimum": 1,
          "maximum": 20,
          "default": 3,
          "description": "Maximum number of instances"
        },
        "cpuTargetPercent": {
          "type": "integer",
          "minimum": 10,
          "maximum": 95,
          "default": 80,
          "description": "CPU utilization target for scaling"
        }
      }
    }
  },
  "additionalProperties": false
}
```

### 4.2 Rust Config Struct

Add to `config_types.rs`:

```rust
/// Northflank provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NorthflankProviderConfig {
    /// Northflank project name
    #[serde(default, rename = "projectName")]
    pub project_name: Option<String>,

    /// Northflank service name
    #[serde(default, rename = "serviceName")]
    pub service_name: Option<String>,

    /// Compute plan
    #[serde(default, rename = "computePlan")]
    pub compute_plan: Option<String>,

    /// GPU type
    #[serde(default, rename = "gpuType")]
    pub gpu_type: Option<String>,

    /// Number of instances
    #[serde(default = "default_instances")]
    pub instances: u32,

    /// Deployment region
    #[serde(default)]
    pub region: Option<String>,

    /// Port configuration
    #[serde(default)]
    pub ports: Vec<NorthflankPortConfig>,

    /// Health check configuration
    #[serde(default, rename = "healthCheck")]
    pub health_check: Option<NorthflankHealthCheckConfig>,

    /// Auto-scaling configuration
    #[serde(default, rename = "autoScaling")]
    pub auto_scaling: Option<NorthflankAutoScalingConfig>,
}

fn default_instances() -> u32 {
    1
}

/// Northflank port configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NorthflankPortConfig {
    /// Port name
    pub name: String,

    /// Internal port number
    #[serde(rename = "internalPort")]
    pub internal_port: u16,

    /// Expose publicly
    #[serde(default)]
    pub public: bool,

    /// Protocol (TCP, HTTP, UDP)
    #[serde(default)]
    pub protocol: Option<String>,
}

/// Northflank health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NorthflankHealthCheckConfig {
    /// HTTP path
    pub path: String,

    /// Port to check
    pub port: u16,

    /// Check interval
    #[serde(default, rename = "intervalSecs")]
    pub interval_secs: Option<u32>,

    /// Check timeout
    #[serde(default, rename = "timeoutSecs")]
    pub timeout_secs: Option<u32>,
}

/// Northflank auto-scaling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NorthflankAutoScalingConfig {
    /// Minimum instances
    #[serde(default, rename = "minInstances")]
    pub min_instances: Option<u32>,

    /// Maximum instances
    #[serde(default, rename = "maxInstances")]
    pub max_instances: Option<u32>,

    /// CPU utilization target
    #[serde(default, rename = "cpuTargetPercent")]
    pub cpu_target_percent: Option<u32>,
}
```

---

## 5. Factory Function Update

### `v3/crates/sindri-providers/src/lib.rs`

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
        ProviderType::Runpod => Ok(Box::new(runpod::RunpodProvider::new()?)),         // NEW
        ProviderType::Northflank => Ok(Box::new(northflank::NorthflankProvider::new()?)), // NEW
    }
}
```

Module declarations (add after line 17):

```rust
pub mod northflank;
pub mod runpod;
```

---

## 6. Example sindri.yaml Configurations

### RunPod - GPU Development

```yaml
version: "3.0"
name: gpu-dev
deployment:
  provider: runpod
  image: ghcr.io/org/sindri:latest
  resources:
    memory: 16GB
    cpus: 4
    gpu:
      enabled: true
      type: nvidia
      count: 1
      tier: gpu-medium
  volumes:
    workspace:
      size: 50GB
extensions:
  profile: fullstack
providers:
  runpod:
    cloudType: COMMUNITY
    containerDiskGb: 30
    exposePorts:
      - "8080"
      - "3000"
```

### RunPod - CPU Only

```yaml
version: "3.0"
name: cpu-dev
deployment:
  provider: runpod
  image: ghcr.io/org/sindri:latest
  resources:
    memory: 8GB
    cpus: 4
  volumes:
    workspace:
      size: 20GB
extensions:
  profile: minimal
providers:
  runpod:
    cloudType: COMMUNITY
    containerDiskGb: 20
```

### Northflank - Basic

```yaml
version: "3.0"
name: nf-dev
deployment:
  provider: northflank
  image: ghcr.io/org/sindri:latest
  resources:
    memory: 4GB
    cpus: 2
  volumes:
    workspace:
      size: 10GB
extensions:
  profile: minimal
providers:
  northflank:
    computePlan: nf-compute-50
    instances: 1
```

### Northflank - Enterprise with Auto-Scaling

```yaml
version: "3.0"
name: team-dev
deployment:
  provider: northflank
  image: ghcr.io/org/sindri:latest
  resources:
    memory: 8GB
    cpus: 4
    gpu:
      enabled: true
      tier: gpu-large
  volumes:
    workspace:
      size: 50GB
extensions:
  profile: enterprise
providers:
  northflank:
    projectName: team-sindri
    computePlan: nf-compute-200
    instances: 2
    ports:
      - name: http
        internalPort: 8080
        public: true
        protocol: HTTP
    healthCheck:
      path: /health
      port: 8080
      intervalSecs: 30
      timeoutSecs: 5
    autoScaling:
      minInstances: 1
      maxInstances: 5
      cpuTargetPercent: 70
```

---

## 7. Updated Provider Comparison Matrix

| Feature      | Docker  | Fly      | DevPod       | E2B      | K8s      | **RunPod**    | **Northflank**  |
| ------------ | ------- | -------- | ------------ | -------- | -------- | ------------- | --------------- |
| GPU          | Runtime | Yes      | Yes          | No       | Yes      | **Primary**   | **Yes**         |
| Auto-suspend | No      | Yes      | No           | Yes      | No       | **No**        | **Yes**         |
| Volumes      | Docker  | Fly      | DevPod       | No       | PVCs     | **Network**   | **Attached**    |
| SSH          | exec    | ssh      | ssh          | N/A      | exec     | **Built-in**  | **exec**        |
| Secrets      | .env    | flyctl   | env          | ENV      | K8s      | **--env**     | **Groups**      |
| CLI          | docker  | flyctl   | devpod       | e2b      | kubectl  | **runpodctl** | **northflank**  |
| Auth         | N/A     | login    | N/A          | API_KEY  | config   | **API_KEY**   | **login/token** |
| Connect      | exec    | ssh      | ssh          | terminal | exec     | **connect**   | **exec**        |
| Config file  | compose | fly.toml | devcontainer | e2b.toml | k8s.yaml | **None**      | **None**        |
| Health check | No      | No       | No           | No       | Yes      | **No**        | **Yes**         |
| Auto-scale   | No      | No       | No           | No       | HPA      | **No**        | **Yes**         |

---

## 8. Complete File Change Summary

| File                                              | Type   | Description                                                                                                                                                                                              |
| ------------------------------------------------- | ------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `v3/crates/sindri-core/src/types/config_types.rs` | MODIFY | Add `Runpod`, `Northflank` to Provider enum; add `Display`, `normalized()`, `supports_gpu()` arms; add `RunpodProviderConfig`, `NorthflankProviderConfig` + sub-structs; add fields to `ProvidersConfig` |
| `v3/schemas/sindri.schema.json`                   | MODIFY | Add "runpod", "northflank" to provider enum; add `providers.runpod` and `providers.northflank` schema objects                                                                                            |
| `v3/crates/sindri-providers/src/runpod.rs`        | NEW    | Full RunpodProvider implementation (~500 lines)                                                                                                                                                          |
| `v3/crates/sindri-providers/src/northflank.rs`    | NEW    | Full NorthflankProvider implementation (~600 lines)                                                                                                                                                      |
| `v3/crates/sindri-providers/src/lib.rs`           | MODIFY | Add module declarations + factory match arms                                                                                                                                                             |

---

## 9. Validation Rules

### RunPod-specific validation:

- If `deployment.provider` is "runpod" and `deployment.resources.gpu.enabled` is false and no `gpuTypeId` is set, the adapter creates a CPU-only pod
- `containerDiskGb` must be >= 1 and <= 500
- `spotBid` if set must be > 0
- `cloudType` must be exactly "SECURE" or "COMMUNITY" (case-sensitive, matching RunPod API)

### Northflank-specific validation:

- If `deployment.provider` is "northflank", `projectName` defaults to "sindri-{name}" if omitted
- `instances` must be >= 1
- `computePlan` if specified must be a valid Northflank plan ID
- `ports[].internalPort` must be 1-65535
- `ports[].protocol` must be one of "TCP", "HTTP", "UDP"
- `healthCheck.port` must match one of the declared ports
- `autoScaling.minInstances` must be <= `autoScaling.maxInstances`
