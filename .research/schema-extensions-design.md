# JSON Schema Extensions Design: RunPod & Northflank Providers

> Design Document for Sindri v3 Provider Schema Extensions
> Date: 2026-02-16
> Status: Draft

---

## 1. Overview

This document defines the JSON schema extensions required to support RunPod and Northflank as first-class providers in Sindri v3. The design follows the existing schema conventions established by Docker, Fly.io, DevPod, E2B, and Kubernetes providers in `v3/schemas/sindri.schema.json` and the corresponding Rust types in `v3/crates/sindri-core/src/types/config_types.rs`.

### Design Principles

1. **Consistency** -- Follow existing naming conventions (camelCase for JSON/YAML keys, snake_case with `#[serde(rename)]` in Rust)
2. **Progressive disclosure** -- Minimal required fields, sensible defaults for everything else
3. **Backward compatibility** -- Additive changes only; no modifications to existing provider schemas
4. **Platform fidelity** -- Schema fields map directly to platform API parameters where possible
5. **Validation at the boundary** -- Schema enforces constraints that prevent invalid API calls

---

## 2. Provider Enum Extension

### JSON Schema Change

**File**: `v3/schemas/sindri.schema.json` (line 24)

Current:

```json
"provider": {
  "type": "string",
  "enum": ["fly", "kubernetes", "docker-compose", "docker", "devpod", "e2b"]
}
```

Updated:

```json
"provider": {
  "type": "string",
  "enum": ["fly", "kubernetes", "docker-compose", "docker", "devpod", "e2b", "runpod", "northflank"]
}
```

### Rust Type Change

**File**: `v3/crates/sindri-core/src/types/config_types.rs` (line 171)

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
    Runpod,       // NEW
    Northflank,   // NEW
}
```

The `Display` impl, `normalized()`, and `supports_gpu()` methods must also be extended:

```rust
impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // ... existing arms ...
            Provider::Runpod => write!(f, "runpod"),
            Provider::Northflank => write!(f, "northflank"),
        }
    }
}

impl Provider {
    pub fn normalized(&self) -> &str {
        match self {
            // ... existing arms ...
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
                | Provider::Runpod       // GPU is RunPod's core feature
                | Provider::Northflank   // Supports H100, A100, L4, etc.
        )
    }
}
```

---

## 3. RunPod Provider Schema

### JSON Schema Definition

Added under `properties.providers.properties.runpod`:

```json
"runpod": {
  "type": "object",
  "description": "RunPod GPU cloud provider options",
  "properties": {
    "gpuType": {
      "type": "string",
      "description": "GPU type identifier (e.g., 'NVIDIA RTX A4000', 'NVIDIA A100 80GB', 'NVIDIA H100 80GB'). Required unless cpuOnly is true."
    },
    "gpuCount": {
      "type": "integer",
      "minimum": 1,
      "maximum": 8,
      "default": 1,
      "description": "Number of GPUs to attach to the pod"
    },
    "containerDiskGb": {
      "type": "integer",
      "minimum": 1,
      "maximum": 500,
      "default": 20,
      "description": "Container disk size in GB (ephemeral, lost on pod termination)"
    },
    "volumeSizeGb": {
      "type": "integer",
      "minimum": 0,
      "maximum": 2048,
      "default": 50,
      "description": "Network volume size in GB (persistent across pod restarts). Set to 0 to disable."
    },
    "volumeMountPath": {
      "type": "string",
      "default": "/workspace",
      "description": "Mount path for the network volume inside the container"
    },
    "cloudType": {
      "type": "string",
      "enum": ["SECURE", "COMMUNITY"],
      "default": "COMMUNITY",
      "description": "Cloud type: SECURE (Tier 1/2 data centers) or COMMUNITY (peer-hosted, lower cost)"
    },
    "region": {
      "type": "string",
      "description": "Datacenter region filter (e.g., 'US', 'EU', 'CA'). Leave empty for any region."
    },
    "spotBid": {
      "type": "number",
      "minimum": 0,
      "default": 0,
      "description": "Spot instance bid price in $/hr. 0 = on-demand pricing, >0 = spot with maximum bid price."
    },
    "exposePorts": {
      "type": "array",
      "items": {
        "type": "integer",
        "minimum": 1,
        "maximum": 65535
      },
      "description": "HTTP ports to expose via RunPod proxy URLs (e.g., [8080, 8888])"
    },
    "cpuOnly": {
      "type": "boolean",
      "default": false,
      "description": "Deploy a CPU-only pod (no GPU). Uses CPU instance types."
    },
    "cpuInstanceId": {
      "type": "string",
      "description": "CPU instance type ID when cpuOnly is true (e.g., 'cpu3c-2-4' for 2 vCPU, 4GB RAM)"
    },
    "startSsh": {
      "type": "boolean",
      "default": true,
      "description": "Enable SSH access on the pod"
    },
    "templateId": {
      "type": "string",
      "description": "RunPod template ID to use instead of a raw image. Overrides deployment.image."
    }
  },
  "if": {
    "properties": { "cpuOnly": { "const": true } }
  },
  "then": {
    "not": { "required": ["gpuType"] }
  },
  "additionalProperties": false
}
```

### Field Reference Table

| Field             | Type       | Required | Default      | Constraints                                 | RunPod API Mapping                   |
| ----------------- | ---------- | -------- | ------------ | ------------------------------------------- | ------------------------------------ |
| `gpuType`         | string     | No\*     | -            | Free text, must match RunPod GPU identifier | `gpuType` parameter in `create pods` |
| `gpuCount`        | integer    | No       | 1            | 1-8                                         | `gpuCount` parameter                 |
| `containerDiskGb` | integer    | No       | 20           | 1-500                                       | `containerDiskSize` parameter        |
| `volumeSizeGb`    | integer    | No       | 50           | 0-2048                                      | `volumeSize` parameter               |
| `volumeMountPath` | string     | No       | `/workspace` | Valid Unix path                             | `volumeMountPath` parameter          |
| `cloudType`       | enum       | No       | `COMMUNITY`  | `SECURE` or `COMMUNITY`                     | `cloudType` parameter                |
| `region`          | string     | No       | (any)        | Free text, datacenter filter                | Region filter in API                 |
| `spotBid`         | number     | No       | 0            | >= 0                                        | `--bid` flag on `start pod`          |
| `exposePorts`     | array[int] | No       | []           | Port range 1-65535                          | `--ports` parameter                  |
| `cpuOnly`         | boolean    | No       | false        | -                                           | Uses CPU instance endpoint           |
| `cpuInstanceId`   | string     | No       | -            | Free text                                   | CPU instance ID                      |
| `startSsh`        | boolean    | No       | true         | -                                           | `--startSSH` flag                    |
| `templateId`      | string     | No       | -            | Free text                                   | `--templateId` parameter             |

\*`gpuType` is effectively required when `cpuOnly` is false (validated at the adapter level, not schema level, to keep the schema simple).

### Rust Type Definition

```rust
/// RunPod provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunpodProviderConfig {
    /// GPU type identifier
    #[serde(default, rename = "gpuType")]
    pub gpu_type: Option<String>,

    /// Number of GPUs
    #[serde(default = "default_gpu_count_1", rename = "gpuCount")]
    pub gpu_count: u32,

    /// Container disk size in GB
    #[serde(default = "default_container_disk", rename = "containerDiskGb")]
    pub container_disk_gb: u32,

    /// Network volume size in GB (0 to disable)
    #[serde(default = "default_volume_size_50", rename = "volumeSizeGb")]
    pub volume_size_gb: u32,

    /// Volume mount path
    #[serde(default = "default_volume_mount_path", rename = "volumeMountPath")]
    pub volume_mount_path: String,

    /// Cloud type (SECURE or COMMUNITY)
    #[serde(default = "default_cloud_type", rename = "cloudType")]
    pub cloud_type: RunpodCloudType,

    /// Region filter
    #[serde(default)]
    pub region: Option<String>,

    /// Spot bid price (0 = on-demand)
    #[serde(default, rename = "spotBid")]
    pub spot_bid: f64,

    /// Ports to expose via RunPod proxy
    #[serde(default, rename = "exposePorts")]
    pub expose_ports: Vec<u16>,

    /// CPU-only mode (no GPU)
    #[serde(default, rename = "cpuOnly")]
    pub cpu_only: bool,

    /// CPU instance type ID (when cpuOnly is true)
    #[serde(default, rename = "cpuInstanceId")]
    pub cpu_instance_id: Option<String>,

    /// Enable SSH access
    #[serde(default = "default_true", rename = "startSsh")]
    pub start_ssh: bool,

    /// RunPod template ID (overrides image)
    #[serde(default, rename = "templateId")]
    pub template_id: Option<String>,
}

/// RunPod cloud types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum RunpodCloudType {
    Secure,
    #[default]
    Community,
}

// Default functions
fn default_gpu_count_1() -> u32 { 1 }
fn default_container_disk() -> u32 { 20 }
fn default_volume_size_50() -> u32 { 50 }
fn default_volume_mount_path() -> String { "/workspace".to_string() }
fn default_cloud_type() -> RunpodCloudType { RunpodCloudType::Community }
```

---

## 4. Northflank Provider Schema

### JSON Schema Definition

Added under `properties.providers.properties.northflank`:

```json
"northflank": {
  "type": "object",
  "description": "Northflank PaaS provider options",
  "required": ["projectName"],
  "properties": {
    "projectName": {
      "type": "string",
      "pattern": "^[a-z][a-z0-9-]*$",
      "description": "Northflank project name. Created if it does not exist."
    },
    "serviceName": {
      "type": "string",
      "pattern": "^[a-z][a-z0-9-]*$",
      "description": "Service name within the project (defaults to sindri.yaml name)"
    },
    "computePlan": {
      "type": "string",
      "pattern": "^nf-compute-\\d+(-\\d+)?$",
      "default": "nf-compute-50",
      "description": "Northflank compute plan (e.g., 'nf-compute-50', 'nf-compute-200-8'). See Northflank pricing for options."
    },
    "gpuType": {
      "type": "string",
      "description": "GPU model for GPU workloads (e.g., 'nvidia-h100', 'nvidia-a100-40gb', 'nvidia-l4')"
    },
    "instances": {
      "type": "integer",
      "minimum": 0,
      "maximum": 50,
      "default": 1,
      "description": "Number of service instances. Set to 0 to pause billing while preserving config."
    },
    "volumeSizeGb": {
      "type": "integer",
      "minimum": 0,
      "maximum": 1500,
      "default": 10,
      "description": "Persistent volume size in GB. 0 = no volume. Note: volumes limit service to 1 instance."
    },
    "volumeMountPath": {
      "type": "string",
      "default": "/workspace",
      "description": "Mount path for the persistent volume inside the container"
    },
    "region": {
      "type": "string",
      "description": "Northflank region slug (e.g., 'us-east', 'europe-west', 'asia-northeast'). Region is immutable after project creation."
    },
    "registryCredentials": {
      "type": "string",
      "description": "Northflank registry credential ID for pulling private images"
    },
    "ports": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["name", "internalPort"],
        "properties": {
          "name": {
            "type": "string",
            "pattern": "^[a-z][a-z0-9-]*$",
            "description": "Port name (used as identifier)"
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
            "description": "Expose this port publicly with auto-TLS"
          },
          "protocol": {
            "type": "string",
            "enum": ["HTTP", "TCP", "UDP"],
            "default": "HTTP",
            "description": "Protocol for this port"
          }
        },
        "additionalProperties": false
      },
      "description": "Port configuration for the service"
    },
    "autoScaling": {
      "type": "object",
      "description": "Horizontal auto-scaling configuration",
      "properties": {
        "enabled": {
          "type": "boolean",
          "default": false,
          "description": "Enable auto-scaling based on resource utilization"
        },
        "minInstances": {
          "type": "integer",
          "minimum": 1,
          "maximum": 50,
          "default": 1,
          "description": "Minimum number of instances (scale-down floor)"
        },
        "maxInstances": {
          "type": "integer",
          "minimum": 1,
          "maximum": 50,
          "default": 3,
          "description": "Maximum number of instances (scale-up ceiling)"
        },
        "targetCpuUtilization": {
          "type": "integer",
          "minimum": 1,
          "maximum": 100,
          "default": 70,
          "description": "Target CPU utilization percentage (scale up when exceeded)"
        },
        "targetMemoryUtilization": {
          "type": "integer",
          "minimum": 1,
          "maximum": 100,
          "default": 80,
          "description": "Target memory utilization percentage (scale up when exceeded)"
        }
      },
      "additionalProperties": false
    },
    "healthCheck": {
      "type": "object",
      "description": "Health check configuration for liveness probing",
      "properties": {
        "type": {
          "type": "string",
          "enum": ["http", "tcp", "command"],
          "default": "tcp",
          "description": "Health check method"
        },
        "path": {
          "type": "string",
          "description": "HTTP endpoint path (required when type is 'http')"
        },
        "port": {
          "type": "integer",
          "minimum": 1,
          "maximum": 65535,
          "description": "Port to check (required when type is 'http' or 'tcp')"
        },
        "command": {
          "type": "array",
          "items": { "type": "string" },
          "description": "Command to execute (required when type is 'command')"
        },
        "initialDelaySeconds": {
          "type": "integer",
          "minimum": 0,
          "default": 10,
          "description": "Seconds to wait before starting health checks"
        },
        "periodSeconds": {
          "type": "integer",
          "minimum": 1,
          "default": 15,
          "description": "Interval between health checks in seconds"
        },
        "failureThreshold": {
          "type": "integer",
          "minimum": 1,
          "default": 3,
          "description": "Number of consecutive failures before restart"
        }
      },
      "allOf": [
        {
          "if": {
            "properties": { "type": { "const": "http" } }
          },
          "then": {
            "required": ["path", "port"]
          }
        },
        {
          "if": {
            "properties": { "type": { "const": "tcp" } }
          },
          "then": {
            "required": ["port"]
          }
        },
        {
          "if": {
            "properties": { "type": { "const": "command" } }
          },
          "then": {
            "required": ["command"]
          }
        }
      ],
      "additionalProperties": false
    }
  },
  "additionalProperties": false
}
```

### Field Reference Table

| Field                 | Type    | Required | Default              | Constraints                      | Northflank API Mapping            |
| --------------------- | ------- | -------- | -------------------- | -------------------------------- | --------------------------------- |
| `projectName`         | string  | **Yes**  | -                    | lowercase alphanumeric + hyphens | Project `name`                    |
| `serviceName`         | string  | No       | (sindri.yaml name)   | lowercase alphanumeric + hyphens | Service `name`                    |
| `computePlan`         | string  | No       | `nf-compute-50`      | Pattern: `nf-compute-\d+(-\d+)?` | `billing.deploymentPlan`          |
| `gpuType`             | string  | No       | -                    | Free text                        | GPU workload config               |
| `instances`           | integer | No       | 1                    | 0-50                             | `deployment.instances`            |
| `volumeSizeGb`        | integer | No       | 10                   | 0-1500                           | Volume `size`                     |
| `volumeMountPath`     | string  | No       | `/workspace`         | Valid Unix path                  | `mountPaths[].containerMountPath` |
| `region`              | string  | No       | (Northflank default) | Must be valid region slug        | Project `region`                  |
| `registryCredentials` | string  | No       | -                    | Credential ID                    | `deployment.external.credentials` |
| `ports`               | array   | No       | []                   | See port object schema           | `ports` array                     |
| `autoScaling`         | object  | No       | (disabled)           | See auto-scaling schema          | `autoscaling` config              |
| `healthCheck`         | object  | No       | -                    | See health check schema          | `healthChecks.liveness`           |

### Rust Type Definitions

```rust
/// Northflank provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NorthflankProviderConfig {
    /// Northflank project name (required)
    #[serde(rename = "projectName")]
    pub project_name: String,

    /// Service name (defaults to sindri.yaml name)
    #[serde(default, rename = "serviceName")]
    pub service_name: Option<String>,

    /// Compute plan
    #[serde(default = "default_compute_plan", rename = "computePlan")]
    pub compute_plan: String,

    /// GPU type for GPU workloads
    #[serde(default, rename = "gpuType")]
    pub gpu_type: Option<String>,

    /// Number of service instances
    #[serde(default = "default_instances")]
    pub instances: u32,

    /// Volume size in GB (0 = no volume)
    #[serde(default = "default_volume_size_10", rename = "volumeSizeGb")]
    pub volume_size_gb: u32,

    /// Volume mount path
    #[serde(default = "default_volume_mount_path", rename = "volumeMountPath")]
    pub volume_mount_path: String,

    /// Region slug
    #[serde(default)]
    pub region: Option<String>,

    /// Registry credential ID for private images
    #[serde(default, rename = "registryCredentials")]
    pub registry_credentials: Option<String>,

    /// Port configuration
    #[serde(default)]
    pub ports: Vec<NorthflankPort>,

    /// Auto-scaling configuration
    #[serde(default, rename = "autoScaling")]
    pub auto_scaling: Option<NorthflankAutoScaling>,

    /// Health check configuration
    #[serde(default, rename = "healthCheck")]
    pub health_check: Option<NorthflankHealthCheck>,
}

/// Northflank port configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NorthflankPort {
    /// Port name
    pub name: String,

    /// Internal container port
    #[serde(rename = "internalPort")]
    pub internal_port: u16,

    /// Whether to expose publicly
    #[serde(default)]
    pub public: bool,

    /// Protocol
    #[serde(default = "default_http_protocol")]
    pub protocol: NorthflankProtocol,
}

/// Northflank port protocols
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum NorthflankProtocol {
    #[default]
    Http,
    Tcp,
    Udp,
}

/// Northflank auto-scaling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NorthflankAutoScaling {
    /// Enable auto-scaling
    #[serde(default)]
    pub enabled: bool,

    /// Minimum instances
    #[serde(default = "default_min_instances", rename = "minInstances")]
    pub min_instances: u32,

    /// Maximum instances
    #[serde(default = "default_max_instances", rename = "maxInstances")]
    pub max_instances: u32,

    /// Target CPU utilization percentage
    #[serde(default = "default_cpu_target", rename = "targetCpuUtilization")]
    pub target_cpu_utilization: u32,

    /// Target memory utilization percentage
    #[serde(default = "default_memory_target", rename = "targetMemoryUtilization")]
    pub target_memory_utilization: u32,
}

/// Northflank health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NorthflankHealthCheck {
    /// Check type
    #[serde(default = "default_health_check_type", rename = "type")]
    pub check_type: NorthflankHealthCheckType,

    /// HTTP path (for http type)
    #[serde(default)]
    pub path: Option<String>,

    /// Port to check (for http/tcp types)
    #[serde(default)]
    pub port: Option<u16>,

    /// Command to execute (for command type)
    #[serde(default)]
    pub command: Option<Vec<String>>,

    /// Initial delay before first check
    #[serde(default = "default_initial_delay", rename = "initialDelaySeconds")]
    pub initial_delay_seconds: u32,

    /// Interval between checks
    #[serde(default = "default_period", rename = "periodSeconds")]
    pub period_seconds: u32,

    /// Failures before restart
    #[serde(default = "default_failure_threshold", rename = "failureThreshold")]
    pub failure_threshold: u32,
}

/// Northflank health check types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NorthflankHealthCheckType {
    Http,
    #[default]
    Tcp,
    Command,
}

// Default functions
fn default_compute_plan() -> String { "nf-compute-50".to_string() }
fn default_instances() -> u32 { 1 }
fn default_volume_size_10() -> u32 { 10 }
fn default_http_protocol() -> NorthflankProtocol { NorthflankProtocol::Http }
fn default_min_instances() -> u32 { 1 }
fn default_max_instances() -> u32 { 3 }
fn default_cpu_target() -> u32 { 70 }
fn default_memory_target() -> u32 { 80 }
fn default_health_check_type() -> NorthflankHealthCheckType { NorthflankHealthCheckType::Tcp }
fn default_initial_delay() -> u32 { 10 }
fn default_period() -> u32 { 15 }
fn default_failure_threshold() -> u32 { 3 }
```

### ProvidersConfig Extension

```rust
/// Provider-specific configurations
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

## 5. Example YAML Configurations

### 5.1 Minimal RunPod Config (CPU-Only)

```yaml
version: "3.0"
name: my-dev-cpu
deployment:
  provider: runpod
  image: ghcr.io/pacphi/sindri:latest
  resources:
    memory: 4GB
    cpus: 2
extensions:
  profile: minimal
providers:
  runpod:
    cpuOnly: true
    cpuInstanceId: "cpu3c-2-4"
    containerDiskGb: 20
    volumeSizeGb: 0
```

### 5.2 Full RunPod Config (GPU with All Options)

```yaml
version: "3.0"
name: my-gpu-env
deployment:
  provider: runpod
  image: ghcr.io/pacphi/sindri:latest
  resources:
    memory: 16GB
    cpus: 4
    gpu:
      enabled: true
      type: nvidia
      count: 1
extensions:
  profile: fullstack
  additional:
    - python
    - cuda
secrets:
  - name: GITHUB_TOKEN
    source: env
    required: true
providers:
  runpod:
    gpuType: "NVIDIA A100 80GB"
    gpuCount: 1
    containerDiskGb: 50
    volumeSizeGb: 100
    volumeMountPath: "/workspace"
    cloudType: SECURE
    region: "US"
    spotBid: 1.50
    exposePorts: [8080, 8888]
    startSsh: true
```

### 5.3 Minimal Northflank Config

```yaml
version: "3.0"
name: my-nf-env
deployment:
  provider: northflank
  image: ghcr.io/pacphi/sindri:latest
  resources:
    memory: 2GB
    cpus: 1
extensions:
  profile: minimal
providers:
  northflank:
    projectName: sindri-dev
```

### 5.4 Full Northflank Config (Auto-Scaling and Health Checks)

```yaml
version: "3.0"
name: my-nf-production
deployment:
  provider: northflank
  image: ghcr.io/pacphi/sindri:latest
  resources:
    memory: 8GB
    cpus: 4
    gpu:
      enabled: true
      type: nvidia
      count: 1
extensions:
  profile: enterprise
  additional:
    - python
    - cuda
secrets:
  - name: DATABASE_URL
    source: env
    required: true
  - name: API_SECRET
    source: vault
    vaultPath: secret/data/sindri
    vaultKey: api_secret
providers:
  northflank:
    projectName: sindri-production
    serviceName: sindri-workspace
    computePlan: nf-compute-400-16
    gpuType: nvidia-h100
    instances: 2
    volumeSizeGb: 50
    volumeMountPath: "/workspace"
    region: us-east
    registryCredentials: ghcr-creds-id
    ports:
      - name: http
        internalPort: 8080
        public: true
        protocol: HTTP
      - name: ssh
        internalPort: 22
        public: false
        protocol: TCP
    autoScaling:
      enabled: true
      minInstances: 1
      maxInstances: 5
      targetCpuUtilization: 70
      targetMemoryUtilization: 80
    healthCheck:
      type: http
      path: /healthz
      port: 8080
      initialDelaySeconds: 15
      periodSeconds: 10
      failureThreshold: 3
```

---

## 6. Rust Type Mappings Summary

### Naming Convention Mapping

The existing codebase uses `camelCase` in YAML/JSON and `snake_case` in Rust, bridged by `#[serde(rename = "camelCase")]`. New types follow the same convention.

| JSON/YAML Key             | Rust Field                  | Rust Type                       | Serde Annotation                                                           |
| ------------------------- | --------------------------- | ------------------------------- | -------------------------------------------------------------------------- |
| `gpuType`                 | `gpu_type`                  | `Option<String>`                | `#[serde(default, rename = "gpuType")]`                                    |
| `gpuCount`                | `gpu_count`                 | `u32`                           | `#[serde(default = "default_gpu_count_1", rename = "gpuCount")]`           |
| `containerDiskGb`         | `container_disk_gb`         | `u32`                           | `#[serde(default = "default_container_disk", rename = "containerDiskGb")]` |
| `volumeSizeGb`            | `volume_size_gb`            | `u32`                           | `#[serde(default = "...", rename = "volumeSizeGb")]`                       |
| `volumeMountPath`         | `volume_mount_path`         | `String`                        | `#[serde(default = "...", rename = "volumeMountPath")]`                    |
| `cloudType`               | `cloud_type`                | `RunpodCloudType`               | `#[serde(default = "...", rename = "cloudType")]`                          |
| `spotBid`                 | `spot_bid`                  | `f64`                           | `#[serde(default, rename = "spotBid")]`                                    |
| `exposePorts`             | `expose_ports`              | `Vec<u16>`                      | `#[serde(default, rename = "exposePorts")]`                                |
| `cpuOnly`                 | `cpu_only`                  | `bool`                          | `#[serde(default, rename = "cpuOnly")]`                                    |
| `cpuInstanceId`           | `cpu_instance_id`           | `Option<String>`                | `#[serde(default, rename = "cpuInstanceId")]`                              |
| `startSsh`                | `start_ssh`                 | `bool`                          | `#[serde(default = "default_true", rename = "startSsh")]`                  |
| `templateId`              | `template_id`               | `Option<String>`                | `#[serde(default, rename = "templateId")]`                                 |
| `projectName`             | `project_name`              | `String`                        | `#[serde(rename = "projectName")]`                                         |
| `serviceName`             | `service_name`              | `Option<String>`                | `#[serde(default, rename = "serviceName")]`                                |
| `computePlan`             | `compute_plan`              | `String`                        | `#[serde(default = "...", rename = "computePlan")]`                        |
| `registryCredentials`     | `registry_credentials`      | `Option<String>`                | `#[serde(default, rename = "registryCredentials")]`                        |
| `internalPort`            | `internal_port`             | `u16`                           | `#[serde(rename = "internalPort")]`                                        |
| `autoScaling`             | `auto_scaling`              | `Option<NorthflankAutoScaling>` | `#[serde(default, rename = "autoScaling")]`                                |
| `healthCheck`             | `health_check`              | `Option<NorthflankHealthCheck>` | `#[serde(default, rename = "healthCheck")]`                                |
| `minInstances`            | `min_instances`             | `u32`                           | `#[serde(default = "...", rename = "minInstances")]`                       |
| `maxInstances`            | `max_instances`             | `u32`                           | `#[serde(default = "...", rename = "maxInstances")]`                       |
| `targetCpuUtilization`    | `target_cpu_utilization`    | `u32`                           | `#[serde(default = "...", rename = "targetCpuUtilization")]`               |
| `targetMemoryUtilization` | `target_memory_utilization` | `u32`                           | `#[serde(default = "...", rename = "targetMemoryUtilization")]`            |
| `initialDelaySeconds`     | `initial_delay_seconds`     | `u32`                           | `#[serde(default = "...", rename = "initialDelaySeconds")]`                |
| `periodSeconds`           | `period_seconds`            | `u32`                           | `#[serde(default = "...", rename = "periodSeconds")]`                      |
| `failureThreshold`        | `failure_threshold`         | `u32`                           | `#[serde(default = "...", rename = "failureThreshold")]`                   |

### Enum Type Mappings

| JSON/YAML Value                  | Rust Enum Variant                                     | Serde Strategy                       |
| -------------------------------- | ----------------------------------------------------- | ------------------------------------ |
| `"SECURE"` / `"COMMUNITY"`       | `RunpodCloudType::Secure` / `Community`               | `#[serde(rename_all = "UPPERCASE")]` |
| `"HTTP"` / `"TCP"` / `"UDP"`     | `NorthflankProtocol::Http` / `Tcp` / `Udp`            | `#[serde(rename_all = "UPPERCASE")]` |
| `"http"` / `"tcp"` / `"command"` | `NorthflankHealthCheckType::Http` / `Tcp` / `Command` | `#[serde(rename_all = "lowercase")]` |

---

## 7. Backward Compatibility

### Assessment: No Breaking Changes

This design is purely additive:

1. **Provider enum** -- Two new values (`"runpod"`, `"northflank"`) added. Existing values unchanged.
2. **Providers object** -- Two new optional properties added. Existing provider schemas untouched.
3. **Rust types** -- New variants added to `Provider` enum with `#[serde(rename_all = "kebab-case")]`, which serializes as `"runpod"` and `"northflank"`.
4. **Config deserialization** -- New fields in `ProvidersConfig` use `#[serde(default)]`, so existing configs without `runpod` or `northflank` sections deserialize correctly as `None`.
5. **Factory pattern** -- New match arms added to `create_provider()` without modifying existing arms.

### Migration Path

- Existing `sindri.yaml` files with `provider: fly|docker|kubernetes|devpod|e2b` continue to work without changes.
- The JSON schema accepts the new provider values immediately.
- No configuration migration is required.

### Validation of Backward Compatibility

Any valid v3 `sindri.yaml` file will remain valid after this change. Test with:

```bash
# Existing configs should still validate
sindri validate sindri.yaml
```

---

## 8. Validation Error Messages

### RunPod Validation Errors

| Condition                                  | Error Message                                                                                                                               |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `gpuType` missing when `cpuOnly` is false  | `RunPod: gpuType is required for GPU pods. Set providers.runpod.gpuType (e.g., 'NVIDIA RTX A4000') or set cpuOnly: true for CPU-only pods.` |
| `containerDiskGb` < 1                      | `RunPod: containerDiskGb must be at least 1 GB. Got: {value}`                                                                               |
| `containerDiskGb` > 500                    | `RunPod: containerDiskGb exceeds maximum of 500 GB. Got: {value}`                                                                           |
| `gpuCount` < 1 when GPU pod                | `RunPod: gpuCount must be at least 1 for GPU pods. Got: {value}`                                                                            |
| `gpuCount` > 8                             | `RunPod: gpuCount exceeds maximum of 8. Got: {value}. Contact RunPod for larger allocations.`                                               |
| `spotBid` < 0                              | `RunPod: spotBid must be >= 0 (0 for on-demand, >0 for spot pricing). Got: {value}`                                                         |
| `cloudType` invalid                        | `RunPod: cloudType must be 'SECURE' or 'COMMUNITY'. Got: '{value}'`                                                                         |
| `cpuOnly` true but `gpuType` set           | `RunPod: Warning - gpuType is set but cpuOnly is true. gpuType will be ignored.`                                                            |
| `cpuOnly` true but `cpuInstanceId` missing | `RunPod: cpuInstanceId is recommended when cpuOnly is true (e.g., 'cpu3c-2-4'). Falling back to default CPU instance.`                      |
| Missing `RUNPOD_API_KEY`                   | `RunPod: API key not configured. Run: runpodctl config --apiKey=YOUR_KEY or set RUNPOD_API_KEY environment variable.`                       |
| Port in `exposePorts` out of range         | `RunPod: Port {port} in exposePorts is out of valid range (1-65535).`                                                                       |

### Northflank Validation Errors

| Condition                                      | Error Message                                                                                                                                           |
| ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `projectName` missing                          | `Northflank: projectName is required. Add providers.northflank.projectName to your sindri.yaml.`                                                        |
| `projectName` invalid pattern                  | `Northflank: projectName '{value}' is invalid. Must start with a lowercase letter and contain only lowercase letters, numbers, and hyphens.`            |
| `computePlan` invalid pattern                  | `Northflank: computePlan '{value}' is invalid. Must match pattern 'nf-compute-NNN' or 'nf-compute-NNN-M' (e.g., 'nf-compute-200', 'nf-compute-200-8').` |
| `instances` > 1 with volume                    | `Northflank: Warning - volumeSizeGb > 0 limits service to 1 instance. Current instances: {value}. The platform will enforce single-instance mode.`      |
| `autoScaling.maxInstances` < `minInstances`    | `Northflank: autoScaling.maxInstances ({max}) must be >= minInstances ({min}).`                                                                         |
| Health check type `http` without path          | `Northflank: healthCheck.path is required when type is 'http'.`                                                                                         |
| Health check type `http` or `tcp` without port | `Northflank: healthCheck.port is required when type is '{type}'.`                                                                                       |
| Health check type `command` without command    | `Northflank: healthCheck.command is required when type is 'command'.`                                                                                   |
| `region` immutability warning                  | `Northflank: Warning - Region '{region}' will be set on project creation and cannot be changed later.`                                                  |
| Missing `NORTHFLANK_API_TOKEN`                 | `Northflank: API token not configured. Run: northflank login or set NORTHFLANK_API_TOKEN environment variable.`                                         |
| `targetCpuUtilization` out of range            | `Northflank: targetCpuUtilization must be between 1 and 100. Got: {value}`                                                                              |

---

## 9. Schema Cross-Validation Rules

These rules span multiple schema sections and should be enforced at the adapter level (not JSON Schema level):

| Rule                                 | Severity | Description                                                                                                                                                                                    |
| ------------------------------------ | -------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| GPU consistency                      | Warning  | If `deployment.resources.gpu.enabled` is true but provider-specific `gpuType` is not set, warn that the generic GPU config may not map to a specific GPU on this provider.                     |
| Volume+scaling conflict (Northflank) | Warning  | If `volumeSizeGb > 0` and `instances > 1` (or `autoScaling.maxInstances > 1`), warn that Northflank limits services with volumes to 1 instance.                                                |
| Image required                       | Error    | Both RunPod and Northflank require a container image. If neither `deployment.image` nor `deployment.image_config` is set, and `deployment.buildFromSource.enabled` is not true, emit an error. |
| Region immutability (Northflank)     | Info     | On first deploy, inform user that the region cannot be changed after project creation.                                                                                                         |
| Spot pricing (RunPod)                | Info     | When `spotBid > 0`, inform user that spot pods may be preempted and should not be used for critical workloads.                                                                                 |

---

## 10. Files to Modify

### Schema File

- `v3/schemas/sindri.schema.json` -- Add `"runpod"` and `"northflank"` to provider enum; add provider config objects under `providers`

### Rust Types File

- `v3/crates/sindri-core/src/types/config_types.rs` -- Add `Runpod` and `Northflank` to `Provider` enum; add config structs; extend `ProvidersConfig`

### Provider Factory

- `v3/crates/sindri-providers/src/lib.rs` -- Add match arms for `ProviderType::Runpod` and `ProviderType::Northflank`

### New Provider Modules

- `v3/crates/sindri-providers/src/runpod.rs` -- RunPod provider implementation
- `v3/crates/sindri-providers/src/northflank.rs` -- Northflank provider implementation

---

## Appendix A: RunPod GPU Types Reference

Common GPU types accepted by the RunPod API:

| GPU Type String    | VRAM  | Use Case                |
| ------------------ | ----- | ----------------------- |
| `NVIDIA RTX 3070`  | 8 GB  | Light inference         |
| `NVIDIA RTX 3080`  | 10 GB | Dev/test                |
| `NVIDIA RTX 3090`  | 24 GB | ML training (small)     |
| `NVIDIA RTX 4090`  | 24 GB | ML training (efficient) |
| `NVIDIA RTX A4000` | 16 GB | Professional workloads  |
| `NVIDIA RTX A5000` | 24 GB | Professional workloads  |
| `NVIDIA A40`       | 48 GB | Enterprise inference    |
| `NVIDIA L4`        | 24 GB | Inference-optimized     |
| `NVIDIA L40S`      | 48 GB | Large inference         |
| `NVIDIA A100 40GB` | 40 GB | ML training             |
| `NVIDIA A100 80GB` | 80 GB | Large ML training       |
| `NVIDIA H100 80GB` | 80 GB | LLM training/inference  |

## Appendix B: Northflank Compute Plans Reference

| Plan                 | vCPU | Memory   | Monthly |
| -------------------- | ---- | -------- | ------- |
| `nf-compute-10`      | 0.1  | 256 MB   | $2.70   |
| `nf-compute-20`      | 0.2  | 512 MB   | $5.40   |
| `nf-compute-50`      | 0.5  | 1024 MB  | $12.00  |
| `nf-compute-100-2`   | 1.0  | 2048 MB  | $24.00  |
| `nf-compute-200`     | 2.0  | 4096 MB  | $48.00  |
| `nf-compute-200-8`   | 2.0  | 8192 MB  | $72.00  |
| `nf-compute-400`     | 4.0  | 8192 MB  | $96.00  |
| `nf-compute-400-16`  | 4.0  | 16384 MB | $144.00 |
| `nf-compute-800-16`  | 8.0  | 16384 MB | $192.00 |
| `nf-compute-800-32`  | 8.0  | 32768 MB | $288.00 |
| `nf-compute-1600-32` | 16.0 | 32768 MB | $384.00 |

## Appendix C: Northflank Region Slugs

Valid region values: `us-east`, `us-east-ohio`, `us-west`, `us-west-california`, `us-central`, `europe-west`, `europe-west-frankfurt`, `europe-west-netherlands`, `europe-west-zurich`, `canada-central`, `asia-east`, `asia-northeast`, `asia-southeast`, `australia-southeast`, `africa-south`, `southamerica-east`
