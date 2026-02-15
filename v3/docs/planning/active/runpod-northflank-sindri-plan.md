# RunPod & Northflank Provider Integration for Sindri

## Technical Research Findings & Implementation Plan

_Targeting sindri v3 adapter architecture_

---

## 1. Research Findings

### 1.1 Sindri's Current Architecture

Sindri uses a **provider adapter pattern** at `deploy/adapters/` with the CLI dispatching to provider-specific scripts based on `--provider <name>`. The current v3 providers are Docker, Fly.io, DevPod, E2B, and Kubernetes.

Key architectural traits:

- **CLI entry point**: `cli/sindri` (shell script) handles subcommands like `deploy`, `destroy`, `status`
- **YAML configuration**: `sindri.yaml` defines the environment (extensions, resources, secrets)
- **JSON Schema validation**: All YAML validated against schemas before deploy
- **Adapter contract**: Each provider implements `deploy`, `destroy`, and `status` operations, reading from the parsed `sindri.yaml` config
- **Docker image**: Sindri builds a Docker image from its Dockerfile + selected extensions, then the provider adapter handles pushing/deploying that image to the target platform
- **Volume architecture**: Immutable `/docker/lib` system layer, mutable `$HOME` volume for workspace persistence

### 1.2 RunPod Platform

**What it is**: GPU-focused cloud platform offering persistent Pods (dedicated containers) and Serverless endpoints. For Sindri's use case, **Pods** are the right abstraction — they run any Docker container with optional GPU access, SSH, and persistent volumes.

**CLI (`runpodctl`)**:

- Written in Go, available as a standalone binary
- Install: download from GitHub releases
- Auth: `runpodctl config --apiKey={key}`
- Key commands:
  - `runpodctl create pods --name <name> --gpuType "<type>" --imageName "<image>" --containerDiskSize <GB> --volumeSize <GB> --args "<cmd>"`
  - `runpodctl get pod` / `runpodctl get pod <podId>`
  - `runpodctl start pod <podId>` (on-demand or `--bid=<price>` for spot)
  - `runpodctl stop pod <podId>`
  - `runpodctl remove pod <podId>` (permanent deletion)

**Python SDK (`runpod`)**:

- `pip install runpod`
- `runpod.create_pod(name, image, gpu_type, ...)` → returns pod object with `.id`
- `runpod.stop_pod(id)`, `runpod.resume_pod(id)`, `runpod.terminate_pod(id)`

**GraphQL API**: Underlying API used by both CLI and SDK. Available at `https://api.runpod.io/graphql`.

**Key capabilities relevant to Sindri**:

- Deploy any Docker image with configurable disk and volume sizes
- SSH access built-in (all pods get `runpodctl` pre-installed)
- Network volumes for data persistence across pod terminations
- Environment variable injection
- Expose HTTP ports via proxy URLs (`<podId>-<port>.proxy.runpod.net`)
- GPU types: RTX 3070/3090/4090, A40, A100, H100, L4, L40S, etc.
- CPU-only pods also available (`instance_id="cpu3c-2-4"`)
- Spot pricing (bid-based) for cost savings

**Lifecycle model**:

```
create → RUNNING → stop → EXITED → start → RUNNING
                                  → remove (permanent)
```

Stopped pods retain container disk data. Network volumes persist independently.

### 1.3 Northflank Platform

**What it is**: Kubernetes-based PaaS supporting deployment of Docker containers with built-in CI/CD, auto-scaling, volumes, GPU support, and multi-region. Runs on managed K8s with an abstraction layer.

**CLI (`northflank`)**:

- Node.js-based, installed via npm: `npm install -g @northflank/cli`
- Auth: `northflank login` (opens browser for API token selection)
- Context-based: `northflank context use project|service` for setting defaults
- Fully interactive or scriptable with `--project`, `--service` flags
- Supports JSON/YAML resource definitions via `--file` or `--input`

**Key commands**:

- `northflank create project`
- `northflank create service deployment` — deploy an external Docker image
- `northflank create service combined` — build from git + deploy
- `northflank create volume`
- `northflank delete service --project <id> --service <id>`
- `northflank scale --project <id> --service <id>`
- `northflank pause` / `northflank resume`
- `northflank restart`
- `northflank exec` — shell access into running container
- `northflank forward` — port-forward for local dev access

**REST API**: `https://api.northflank.com/v1/projects/{projectId}/services`

- Auth via `Authorization: Bearer <NORTHFLANK_API_TOKEN>` header
- Create deployment service POST body:

```json
{
  "name": "sindri-dev",
  "billing": { "deploymentPlan": "nf-compute-20" },
  "deployment": {
    "instances": 1,
    "external": {
      "imagePath": "your-image:latest",
      "credentials": "optional-cred-id"
    },
    "docker": { "configType": "default" }
  },
  "ports": [{ "name": "ssh", "internalPort": 22, "public": false }]
}
```

- Delete: `DELETE /v1/projects/{projectId}/services/{serviceId}`

**Key capabilities relevant to Sindri**:

- Deploy any external Docker image from registries
- Persistent volumes attachable to services
- Health checks (HTTP, TCP, CMD)
- CMD/entrypoint override without rebuilding
- Environment variable injection and secret management
- GPU workloads supported (NVIDIA A100, etc.)
- Multiple compute plans (shared vCPU to dedicated CPU, various memory tiers)
- Port forwarding via CLI for local access
- Shell exec into running containers
- Auto-scaling with CPU/memory thresholds
- Pause/resume for cost management
- BYOC (Bring Your Own Cloud) — deploy to user's own GCP/AWS/Azure account

**Resource hierarchy**: Account/Team → Project → Services, Jobs, Volumes, Secrets

**Lifecycle model**:

```
create service → RUNNING → pause → PAUSED → resume → RUNNING
                                           → delete (permanent)
```

---

## 2. Comparative Analysis for Sindri Integration

| Aspect                 | RunPod                                | Northflank                                |
| ---------------------- | ------------------------------------- | ----------------------------------------- |
| **Primary CLI**        | `runpodctl` (Go binary)               | `northflank` (npm/Node.js)                |
| **Auth mechanism**     | API key via CLI config                | API token via browser login               |
| **Deploy model**       | Create pod with image                 | Create project + deployment service       |
| **Image source**       | Docker Hub / registry URL             | Docker Hub / any registry w/ credentials  |
| **Persistent storage** | Network volumes (cross-pod)           | Volumes (attached to service)             |
| **SSH access**         | Built-in on all pods                  | Via `exec` command or port forward        |
| **GPU support**        | Core feature, extensive GPU selection | Supported (NVIDIA A100 etc.)              |
| **Cost management**    | Stop pod (retain disk), spot pricing  | Pause/resume, auto-scaling                |
| **Port exposure**      | Proxy URL per port                    | Public/private ports with auto-TLS        |
| **Health checks**      | Not built-in for pods                 | HTTP, TCP, CMD configurable               |
| **Env vars**           | Via create command flags              | Via API/CLI, secret groups                |
| **Destroy**            | `remove pod` (permanent)              | `delete service` + optional volume delete |
| **Status check**       | `get pod <id>` (JSON output)          | `get service details`                     |
| **Best for**           | GPU-heavy dev environments            | K8s-native, enterprise, multi-region      |

---

## 3. Technical Implementation Plan

### 3.1 New Files to Create

```
deploy/adapters/
├── runpod/
│   ├── deploy_v3.sh        # Deploy implementation
│   ├── destroy_v3.sh       # Destroy implementation
│   ├── status_v3.sh        # Status check
│   ├── connect_v3.sh       # SSH/connect helper
│   └── README.md           # Provider-specific docs
├── northflank/
│   ├── deploy_v3.sh
│   ├── destroy_v3.sh
│   ├── status_v3.sh
│   ├── connect_v3.sh
│   └── README.md
docs/providers/
├── RUNPOD.md               # User-facing RunPod deployment guide
└── NORTHFLANK.md           # User-facing Northflank deployment guide
```

### 3.2 Prerequisite Detection

Both adapters need CLI availability checks added to the existing prereq system.

**RunPod**:

```bash
check_runpod_prerequisites() {
  command -v runpodctl >/dev/null 2>&1 || {
    echo "Error: runpodctl not installed."
    echo "Install: wget https://github.com/runpod/runpodctl/releases/latest/download/runpodctl-linux-amd64 -O runpodctl && chmod +x runpodctl && sudo mv runpodctl /usr/local/bin/"
    exit 1
  }
  # Verify API key is configured
  runpodctl get pod >/dev/null 2>&1 || {
    echo "Error: RunPod API key not configured."
    echo "Run: runpodctl config --apiKey=YOUR_API_KEY"
    exit 1
  }
}
```

**Northflank**:

```bash
check_northflank_prerequisites() {
  command -v northflank >/dev/null 2>&1 || {
    echo "Error: northflank CLI not installed."
    echo "Install: npm install -g @northflank/cli"
    exit 1
  }
  # Verify authentication
  northflank list projects >/dev/null 2>&1 || {
    echo "Error: Not authenticated with Northflank."
    echo "Run: northflank login"
    exit 1
  }
}
```

### 3.3 Configuration Extensions to `sindri.yaml`

Add provider-specific config blocks to the schema:

```yaml
# sindri.yaml — RunPod provider config
provider:
  name: runpod
  runpod:
    gpu_type: "NVIDIA RTX A4000"      # or "NVIDIA A100 80GB", etc.
    gpu_count: 1
    container_disk_gb: 20
    volume_size_gb: 50
    volume_mount_path: "/workspace"
    cloud_type: "SECURE"               # or "COMMUNITY" for cheaper spot
    region: ""                          # optional datacenter ID
    expose_ports:                       # optional HTTP ports to expose
      - 8080
    spot_bid: 0.0                       # 0 = on-demand, >0 = spot with bid

# sindri.yaml — Northflank provider config
provider:
  name: northflank
  northflank:
    project_name: "sindri-dev"
    service_name: "sindri-workspace"
    compute_plan: "nf-compute-50"       # CPU/memory tier
    gpu_type: ""                        # optional, e.g., "nvidia-a100"
    gpu_count: 0
    instances: 1
    volume_size_gb: 10
    volume_mount_path: "/workspace"
    region: ""                          # optional region code
    ports:
      - name: "ssh"
        internal_port: 22
        public: false
```

### 3.4 RunPod Adapter Implementation

#### `deploy/adapters/runpod/deploy_v3.sh`

Core flow:

1. Parse `sindri.yaml` for RunPod config (using `yq`)
2. Build Docker image locally (reuse existing Sindri Docker build pipeline)
3. Push image to registry (Docker Hub or specified registry)
4. Create RunPod pod via `runpodctl create pods`
5. Wait for pod to reach RUNNING state
6. Store pod ID in `.sindri/state/runpod.json` for subsequent operations
7. Output connection info (SSH proxy URL, HTTP port proxies)

```bash
deploy_runpod_v3() {
  local config_file="${1:-sindri.yaml}"
  local app_name=$(yq '.name' "$config_file")
  local image_name=$(yq '.provider.runpod.image // "sindri-dev"' "$config_file")
  local gpu_type=$(yq '.provider.runpod.gpu_type // "NVIDIA RTX A4000"' "$config_file")
  local gpu_count=$(yq '.provider.runpod.gpu_count // 1' "$config_file")
  local container_disk=$(yq '.provider.runpod.container_disk_gb // 20' "$config_file")
  local volume_size=$(yq '.provider.runpod.volume_size_gb // 50' "$config_file")
  local cloud_type=$(yq '.provider.runpod.cloud_type // "COMMUNITY"' "$config_file")

  # Step 1: Build Sindri Docker image
  echo "Building Sindri Docker image..."
  docker build -t "${image_name}:latest" .

  # Step 2: Push to registry
  echo "Pushing image to registry..."
  docker push "${image_name}:latest"

  # Step 3: Create pod
  echo "Creating RunPod pod..."
  local pod_json=$(runpodctl create pods \
    --name "$app_name" \
    --gpuType "$gpu_type" \
    --gpuCount "$gpu_count" \
    --imageName "${image_name}:latest" \
    --containerDiskSize "$container_disk" \
    --volumeSize "$volume_size" \
    --volumeMountPath "/workspace" \
    --startSSH \
    2>&1)

  local pod_id=$(echo "$pod_json" | grep -oP '"id"\s*:\s*"\K[^"]+')

  # Step 4: Save state
  mkdir -p .sindri/state
  cat > .sindri/state/runpod.json <<EOF
{
  "pod_id": "$pod_id",
  "app_name": "$app_name",
  "gpu_type": "$gpu_type",
  "created_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
EOF

  # Step 5: Wait for running status
  echo "Waiting for pod to reach RUNNING state..."
  local retries=0
  while [ $retries -lt 60 ]; do
    local status=$(runpodctl get pod "$pod_id" 2>/dev/null | grep -oP '"desiredStatus"\s*:\s*"\K[^"]+')
    if [ "$status" = "RUNNING" ]; then
      echo "Pod is running!"
      break
    fi
    sleep 5
    retries=$((retries + 1))
  done

  # Step 6: Output connection info
  echo ""
  echo "=== Sindri Dev Environment on RunPod ==="
  echo "Pod ID:    $pod_id"
  echo "SSH:       ssh root@<pod-ip> (check RunPod console for IP)"
  echo "HTTP:      https://${pod_id}-8080.proxy.runpod.net (if port exposed)"
  echo "State:     .sindri/state/runpod.json"
}
```

#### `deploy/adapters/runpod/destroy_v3.sh`

```bash
destroy_runpod_v3() {
  local state_file=".sindri/state/runpod.json"

  if [ ! -f "$state_file" ]; then
    echo "Error: No RunPod state found. Nothing to destroy."
    exit 1
  fi

  local pod_id=$(jq -r '.pod_id' "$state_file")
  local app_name=$(jq -r '.app_name' "$state_file")

  echo "Destroying RunPod pod: $app_name ($pod_id)..."
  read -p "This will permanently delete the pod and all non-volume data. Continue? [y/N] " confirm
  if [ "$confirm" != "y" ] && [ "$confirm" != "Y" ]; then
    echo "Aborted."
    exit 0
  fi

  runpodctl remove pod "$pod_id"
  rm -f "$state_file"

  echo "Pod $pod_id destroyed and state cleaned up."
}
```

### 3.5 Northflank Adapter Implementation

#### `deploy/adapters/northflank/deploy_v3.sh`

Core flow:

1. Parse `sindri.yaml` for Northflank config
2. Build and push Docker image to registry
3. Create Northflank project (if not exists)
4. Create deployment service with external image
5. Optionally create and attach volume
6. Wait for service to reach RUNNING
7. Store project/service IDs in `.sindri/state/northflank.json`
8. Output connection info (port-forward command, exec command)

```bash
deploy_northflank_v3() {
  local config_file="${1:-sindri.yaml}"
  local app_name=$(yq '.name' "$config_file")
  local project_name=$(yq '.provider.northflank.project_name // "sindri-dev"' "$config_file")
  local service_name=$(yq '.provider.northflank.service_name // "sindri-workspace"' "$config_file")
  local compute_plan=$(yq '.provider.northflank.compute_plan // "nf-compute-50"' "$config_file")
  local image_name=$(yq '.provider.northflank.image // "sindri-dev"' "$config_file")
  local volume_size=$(yq '.provider.northflank.volume_size_gb // 10' "$config_file")

  # Step 1: Build and push Docker image
  echo "Building Sindri Docker image..."
  docker build -t "${image_name}:latest" .
  docker push "${image_name}:latest"

  # Step 2: Create project (idempotent — CLI handles existing)
  echo "Ensuring Northflank project exists..."
  northflank create project \
    --input "{\"name\": \"$project_name\", \"description\": \"Sindri dev environment\"}" \
    2>/dev/null || true

  # Step 3: Create deployment service
  echo "Creating deployment service..."
  local service_def=$(cat <<EOF
{
  "name": "$service_name",
  "description": "Sindri development environment",
  "billing": {
    "deploymentPlan": "$compute_plan"
  },
  "deployment": {
    "instances": 1,
    "external": {
      "imagePath": "${image_name}:latest"
    },
    "docker": {
      "configType": "default"
    },
    "storage": {
      "ephemeralStorage": {
        "storageSize": $((volume_size * 1024))
      }
    }
  },
  "ports": [
    {
      "name": "ssh",
      "internalPort": 22,
      "public": false,
      "protocol": "TCP"
    }
  ]
}
EOF
  )

  northflank create service deployment \
    --project "$project_name" \
    --input "$service_def"

  # Step 4: Save state
  local service_id=$(echo "$service_name" | tr '[:upper:]' '[:lower:]' | sed 's/ /-/g')
  mkdir -p .sindri/state
  cat > .sindri/state/northflank.json <<EOF
{
  "project_name": "$project_name",
  "service_name": "$service_name",
  "service_id": "$service_id",
  "compute_plan": "$compute_plan",
  "created_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
EOF

  # Step 5: Output connection info
  echo ""
  echo "=== Sindri Dev Environment on Northflank ==="
  echo "Project:      $project_name"
  echo "Service:      $service_name"
  echo "Connect:      northflank exec --project $project_name --service $service_id"
  echo "Port forward: northflank forward --project $project_name --service $service_id"
  echo "State:        .sindri/state/northflank.json"
}
```

#### `deploy/adapters/northflank/destroy_v3.sh`

```bash
destroy_northflank_v3() {
  local state_file=".sindri/state/northflank.json"

  if [ ! -f "$state_file" ]; then
    echo "Error: No Northflank state found. Nothing to destroy."
    exit 1
  fi

  local project_name=$(jq -r '.project_name' "$state_file")
  local service_id=$(jq -r '.service_id' "$state_file")

  echo "Destroying Northflank service: $service_id in project $project_name..."
  read -p "This will permanently delete the service and volumes. Continue? [y/N] " confirm
  if [ "$confirm" != "y" ] && [ "$confirm" != "Y" ]; then
    echo "Aborted."
    exit 0
  fi

  # Delete service (with volumes)
  northflank delete service \
    --project "$project_name" \
    --service "$service_id" \
    --cascade-volumes

  rm -f "$state_file"

  echo "Service destroyed and state cleaned up."
  echo "Note: The Northflank project '$project_name' was preserved."
  echo "To also delete the project: northflank delete project --project $project_name"
}
```

### 3.6 CLI Router Changes

In `cli/sindri`, the `deploy` and `destroy` subcommand handlers need to recognize the new providers:

```bash
case "$PROVIDER" in
  docker)    source "$ADAPTER_DIR/docker/deploy_v3.sh" ;;
  fly)       source "$ADAPTER_DIR/fly/deploy_v3.sh" ;;
  devpod)    source "$ADAPTER_DIR/devpod/deploy_v3.sh" ;;
  e2b)       source "$ADAPTER_DIR/e2b/deploy_v3.sh" ;;
  kubernetes) source "$ADAPTER_DIR/kubernetes/deploy_v3.sh" ;;
  runpod)    source "$ADAPTER_DIR/runpod/deploy_v3.sh" ;;
  northflank) source "$ADAPTER_DIR/northflank/deploy_v3.sh" ;;
  *)         echo "Unknown provider: $PROVIDER"; exit 1 ;;
esac
```

### 3.7 JSON Schema Updates

Extend the `sindri.yaml` JSON schema to validate RunPod and Northflank provider blocks:

```json
{
  "provider": {
    "oneOf": [
      { "$ref": "#/definitions/dockerProvider" },
      { "$ref": "#/definitions/flyProvider" },
      { "$ref": "#/definitions/runpodProvider" },
      { "$ref": "#/definitions/northflankProvider" }
    ]
  },
  "definitions": {
    "runpodProvider": {
      "type": "object",
      "properties": {
        "name": { "const": "runpod" },
        "runpod": {
          "type": "object",
          "properties": {
            "gpu_type": { "type": "string" },
            "gpu_count": { "type": "integer", "minimum": 0 },
            "container_disk_gb": { "type": "integer", "minimum": 1 },
            "volume_size_gb": { "type": "integer", "minimum": 0 },
            "cloud_type": { "enum": ["SECURE", "COMMUNITY"] },
            "spot_bid": { "type": "number", "minimum": 0 }
          },
          "required": ["gpu_type"]
        }
      }
    },
    "northflankProvider": {
      "type": "object",
      "properties": {
        "name": { "const": "northflank" },
        "northflank": {
          "type": "object",
          "properties": {
            "project_name": { "type": "string" },
            "service_name": { "type": "string" },
            "compute_plan": { "type": "string" },
            "gpu_type": { "type": "string" },
            "instances": { "type": "integer", "minimum": 1 },
            "volume_size_gb": { "type": "integer", "minimum": 0 },
            "region": { "type": "string" }
          },
          "required": ["project_name"]
        }
      }
    }
  }
}
```

---

## 4. Testing Strategy

### 4.1 Unit Tests (Offline)

- YAML schema validation for new provider blocks
- State file generation and parsing
- Prerequisite check functions (mock CLI presence)
- Config parsing with `yq` (various valid/invalid inputs)

### 4.2 Integration Tests (Requires Accounts)

- **RunPod**: Create pod → verify status → SSH connectivity → destroy → verify cleanup
- **Northflank**: Create project + service → verify running → exec shell → destroy → verify cleanup
- Test with CPU-only configs (cheaper for CI)
- Verify state file lifecycle (create, read, delete)
- Test destroy idempotency (double-destroy should be safe)

### 4.3 CI Considerations

- RunPod and Northflank tests should be **optional/manual-trigger** in CI (require real accounts and incur cost)
- Add `--dry-run` flag to both adapters for CI validation without actual deployment
- Secrets: `RUNPOD_API_KEY` and `NORTHFLANK_API_TOKEN` as GitHub Actions secrets

---

## 5. Documentation Deliverables

| Document                       | Purpose                                                                             |
| ------------------------------ | ----------------------------------------------------------------------------------- |
| `docs/providers/RUNPOD.md`     | User-facing setup guide: prerequisites, config, deploy, connect, destroy, cost tips |
| `docs/providers/NORTHFLANK.md` | User-facing setup guide: prerequisites, config, deploy, connect, destroy, scaling   |
| `docs/DEPLOYMENT.md` update    | Add RunPod and Northflank to provider comparison table                              |
| `docs/CONFIGURATION.md` update | Document new YAML provider blocks                                                   |
| `CHANGELOG.md` update          | Feature entry for new providers                                                     |

---

## 6. Implementation Order

| Phase | Work                                                                 | Estimated Effort |
| ----- | -------------------------------------------------------------------- | ---------------- |
| **1** | Schema updates to `sindri.yaml` + JSON schema validation             | 2-3 hours        |
| **2** | RunPod adapter (`deploy_v3.sh`, `destroy_v3.sh`, `status_v3.sh`)     | 4-6 hours        |
| **3** | Northflank adapter (`deploy_v3.sh`, `destroy_v3.sh`, `status_v3.sh`) | 4-6 hours        |
| **4** | CLI router updates + prerequisite checks                             | 1-2 hours        |
| **5** | Example `sindri.yaml` configs for each provider                      | 1 hour           |
| **6** | Provider documentation (`RUNPOD.md`, `NORTHFLANK.md`)                | 2-3 hours        |
| **7** | Test suite (unit + integration stubs)                                | 3-4 hours        |
| **8** | CI workflow updates (optional provider test jobs)                    | 1-2 hours        |

**Total estimated effort: 18-27 hours**

---

## 7. Key Decisions Needed

1. **Image registry strategy**: Should Sindri push to Docker Hub by default, or support configurable registries (GHCR, etc.)? RunPod and Northflank both need a pullable image URL.

2. **GPU as default or opt-in**: RunPod is GPU-first. Should the sindri RunPod adapter default to a CPU instance (cheaper) with GPU as opt-in, or require GPU type specification?

3. **Northflank project lifecycle**: Should `destroy` also delete the Northflank project, or just the service? Projects are organizational containers that may hold multiple services.

4. **State storage location**: Currently using `.sindri/state/<provider>.json`. Should this be committed to repo or gitignored? (Recommend: gitignored, as it contains deployment-specific IDs.)

5. **Connection method**: Both platforms offer different connectivity approaches. Should the adapter provide a unified `sindri connect --provider <name>` command that abstracts SSH (RunPod) vs exec (Northflank)?

6. **Suspend/Resume support**: Both platforms support stop/start or pause/resume. Should these be first-class sindri operations alongside deploy/destroy?
