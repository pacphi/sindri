# DevPod Provider

> **Version:** 3.x
> **Last Updated:** 2026-01

Multi-cloud development environments with DevContainer compatibility.

## Overview

DevPod enables Sindri environments to run as DevContainers with full VS Code, GitHub Codespaces, and DevPod CLI compatibility. It acts as a meta-provider that can deploy to multiple backends.

**Best for:** IDE integration, multi-cloud flexibility, DevContainer standard compliance

## Supported Backends

| Backend        | DevPod Provider | Best For          |
| -------------- | --------------- | ----------------- |
| Docker (local) | `docker`        | Local development |
| Kubernetes     | `kubernetes`    | Enterprise, CI/CD |
| AWS EC2        | `aws`           | Cloud instances   |
| GCP Compute    | `gcp`           | Cloud instances   |
| Azure VMs      | `azure`         | Cloud instances   |
| DigitalOcean   | `digitalocean`  | Budget cloud      |
| SSH Host       | `ssh`           | Existing servers  |

## Prerequisites

| Requirement | Check Command      | Install                                                             |
| ----------- | ------------------ | ------------------------------------------------------------------- |
| devpod CLI  | `devpod version`   | [devpod.sh/install](https://devpod.sh/docs/getting-started/install) |
| Docker      | `docker --version` | Required for local provider and image building                      |
| kubectl     | `kubectl version`  | Required for Kubernetes backend                                     |

## Quick Start

### Local Docker Backend

```bash
# 1. Create configuration
cat > sindri.yaml << 'EOF'
version: "1.0"
name: sindri-devpod

deployment:
  provider: devpod
  resources:
    memory: 4GB
    cpus: 2

extensions:
  profile: fullstack

providers:
  devpod:
    type: docker
EOF

# 2. Deploy
sindri deploy

# 3. Connect
sindri connect
```

### Kubernetes Backend

```bash
# 1. Create configuration
cat > sindri.yaml << 'EOF'
version: "1.0"
name: sindri-k8s

deployment:
  provider: devpod
  resources:
    memory: 4GB
    cpus: 2
  volumes:
    workspace:
      size: 30GB

extensions:
  profile: fullstack

providers:
  devpod:
    type: kubernetes
    kubernetes:
      namespace: dev-environments
      storageClass: standard
      context: my-cluster  # Optional: specific kubeconfig context
EOF

# 2. Deploy
sindri deploy

# 3. Connect
sindri connect
```

### With Multi-Node Local Cluster

```bash
# 1. Create multi-node k3d cluster with registry
sindri k8s create --provider k3d --name sindri-local --nodes 3 --registry

# 2. Create configuration with minimal profile
cat > sindri.yaml << 'EOF'
version: "1.0"
name: sindri-minimal-devpod

deployment:
  provider: devpod
  resources:
    memory: 4GB
    cpus: 2
  volumes:
    workspace:
      size: 30GB

extensions:
  profile: minimal  # Lightweight: nodejs, python only

providers:
  devpod:
    type: kubernetes
    kubernetes:
      namespace: dev-environments
      storageClass: standard
      # DevPod auto-detects k3d-* contexts
EOF

# 3. Deploy
sindri deploy

# 4. Connect
sindri connect
```

## Configuration

### Basic Configuration

```yaml
version: "1.0"
name: sindri-devpod

deployment:
  provider: devpod
  resources:
    memory: 4GB
    cpus: 2

extensions:
  profile: fullstack

providers:
  devpod:
    type: docker # docker | kubernetes | aws | gcp | azure
```

### Kubernetes Configuration

```yaml
providers:
  devpod:
    type: kubernetes
    buildRepository: ghcr.io/myorg/sindri # Required for remote clusters
    kubernetes:
      context: my-cluster-context
      namespace: dev-environments
      storageClass: fast-ssd
```

### Cloud Provider Configuration

```yaml
providers:
  devpod:
    type: aws
    buildRepository: 123456789.dkr.ecr.us-west-2.amazonaws.com/sindri
    aws:
      region: us-west-2
      instanceType: t3.large
```

### GPU Configuration

```yaml
deployment:
  provider: devpod
  resources:
    memory: 32GB
    cpus: 8
    gpu:
      enabled: true
      type: nvidia
      tier: gpu-medium
      count: 1

providers:
  devpod:
    type: kubernetes
    kubernetes:
      namespace: gpu-workloads
```

### Image Deployment Options

DevPod builds Docker images for deployment. You can control which source to build from:

#### Option 1: Default Build (Recommended for Users)

```yaml
deployment:
  provider: devpod
  # Uses official Sindri Dockerfile from releases
```

#### Option 2: Build from Source (For Sindri Developers)

> **Important:** This clones from GitHub - your changes must be pushed first!
> For local Docker testing of uncommitted changes, use `make v3-cycle-fast` with the Docker provider.
> See [MAINTAINER_GUIDE.md](../MAINTAINER_GUIDE.md#two-development-paths) for the full guide.

**Using CLI flag:**

```bash
# First push your changes, then:
sindri deploy --from-source
```

**Using YAML configuration:**

```yaml
deployment:
  provider: devpod
  buildFromSource:
    enabled: true
    gitRef: "main"  # Optional: branch, tag, or commit SHA

# Test a specific pushed feature branch
deployment:
  provider: devpod
  buildFromSource:
    enabled: true
    gitRef: "feature/my-feature"
```

This clones from GitHub and builds from your specified branch, allowing you to test pushed code changes.

## Deployment Commands

```bash
# Deploy
sindri deploy

# Preview deployment plan
sindri plan

# Check status
sindri status

# Connect to workspace
sindri connect

# Stop workspace
sindri stop

# Start workspace
sindri start

# Destroy workspace
sindri destroy
```

## Image Handling

### Local Clusters (kind/k3d) - Zero Config

The V3 provider automatically detects local Kubernetes clusters and handles image loading:

```rust
// V3 detection logic
async fn detect_local_k8s_cluster(&self, context: Option<&str>) -> Option<LocalCluster> {
    let current_context = self.get_k8s_current_context(context).await.ok()?;

    if current_context.starts_with("kind-") {
        // Auto-detected kind cluster
        return Some(LocalCluster { cluster_type: LocalClusterType::Kind, ... });
    }
    if current_context.starts_with("k3d-") {
        // Auto-detected k3d cluster
        return Some(LocalCluster { cluster_type: LocalClusterType::K3d, ... });
    }
    None
}
```

**What happens:**

1. Detects `kind-*` or `k3d-*` context
2. Builds sindri:latest locally
3. Loads image into cluster (`kind load` / `k3d image import`)

No registry configuration required.

### Remote/Cloud Kubernetes

For external clusters, specify a container registry:

```yaml
providers:
  devpod:
    type: kubernetes
    buildRepository: ghcr.io/myorg/sindri
    kubernetes:
      namespace: dev-environments
```

**Registry credentials lookup order:**

1. Environment variables: `DOCKER_USERNAME`, `DOCKER_PASSWORD`
2. `.env.local` or `.env` files
3. Existing Docker login: `~/.docker/config.json`

**Special registry support:**

| Registry                        | Credential Source                     |
| ------------------------------- | ------------------------------------- |
| ghcr.io                         | `GITHUB_TOKEN`                        |
| ECR (_.dkr.ecr._.amazonaws.com) | AWS CLI                               |
| GCR (\*.gcr.io)                 | `GOOGLE_APPLICATION_CREDENTIALS`      |
| Other                           | `DOCKER_USERNAME` + `DOCKER_PASSWORD` |

## What Gets Generated

### devcontainer.json

```json
{
  "name": "sindri-devpod",
  "image": "sindri:latest",
  "remoteUser": "developer",
  "containerEnv": {
    "HOME": "/alt/home/developer",
    "WORKSPACE": "/alt/home/developer/workspace",
    "INSTALL_PROFILE": "fullstack"
  },
  "forwardPorts": [3000, 8080],
  "mounts": ["source=dev_home,target=/alt/home/developer,type=volume"],
  "customizations": {
    "vscode": {
      "extensions": ["ms-python.python", "golang.go", "rust-lang.rust-analyzer"]
    }
  }
}
```

## IDE Integration

### VS Code Dev Containers

1. Generate config only:

```bash
sindri deploy --dry-run
# Or generate devcontainer manually
sindri plan > /dev/null  # Creates .devcontainer/devcontainer.json
```

2. Open in VS Code:

```
Ctrl+Shift+P -> "Dev Containers: Open Folder in Container"
```

### VS Code Remote SSH (via DevPod)

```bash
# DevPod provides SSH access
devpod ssh <workspace-name>

# Or configure VS Code to use DevPod
# Install "DevPod" VS Code extension
```

### GitHub Codespaces

1. Generate DevContainer config
2. Push to repository:

```bash
git add .devcontainer
git commit -m "Add DevContainer configuration"
git push
```

3. Create Codespace from GitHub UI

### JetBrains Gateway

DevPod integrates with JetBrains Gateway for remote development.

## Customization

### Add VS Code Extensions

Edit `.devcontainer/devcontainer.json`:

```json
"customizations": {
  "vscode": {
    "extensions": [
      "ms-python.python",
      "golang.go",
      "rust-lang.rust-analyzer",
      "your-extension-id"
    ]
  }
}
```

### Add DevContainer Features

```json
"features": {
  "ghcr.io/devcontainers/features/github-cli:1": {},
  "ghcr.io/devcontainers/features/docker-in-docker:2": {},
  "ghcr.io/devcontainers/features/kubectl-helm-minikube:1": {}
}
```

### Environment Variables

```json
"containerEnv": {
  "ENVIRONMENT": "development",
  "DEBUG": "true"
}
```

### Port Forwarding

```json
"forwardPorts": [3000, 8080, 5432],
"portsAttributes": {
  "3000": {
    "label": "Application",
    "onAutoForward": "notify"
  },
  "8080": {
    "label": "API",
    "onAutoForward": "openBrowser"
  }
}
```

## DevPod CLI Commands

```bash
# List workspaces
devpod list

# Connect via SSH
devpod ssh <workspace>

# Stop workspace
devpod stop <workspace>

# Start workspace
devpod up <workspace>

# Delete workspace
devpod delete <workspace>

# View logs
devpod logs <workspace>

# List providers
devpod provider list

# Add provider
devpod provider add kubernetes
```

## Troubleshooting

### Workspace Not Found

```bash
# List existing workspaces
devpod list

# Check DevPod status
devpod status <workspace>
```

### Image Pull Errors

For Kubernetes with private registry:

```bash
# Verify registry login
docker login <registry>

# Check if secret exists in cluster
kubectl get secrets -n <namespace>

# Verify buildRepository is set
cat sindri.yaml | grep buildRepository
```

### Kubernetes Connection Issues

```bash
# Verify kubectl context
kubectl config current-context

# Test cluster access
kubectl cluster-info

# Check namespace
kubectl get namespace <namespace>
```

### DevPod Provider Issues

```bash
# List provider status
devpod provider list

# Update provider
devpod provider update kubernetes

# Remove and re-add provider
devpod provider delete kubernetes
devpod provider add kubernetes
```

### Build Failures

```bash
# View build logs
devpod logs <workspace>

# Force rebuild
devpod up . --recreate
```

## Best Practices

1. **Use named workspaces** - Easier to manage multiple environments
2. **Set resource limits** - Prevent resource contention
3. **Use buildRepository for remote K8s** - Required for image distribution
4. **Test with local Docker first** - Before deploying to cloud
5. **Commit .devcontainer** - For team consistency

## Benefits

| Benefit             | Description                     |
| ------------------- | ------------------------------- |
| **IDE Integration** | Full VS Code features           |
| **Reproducible**    | Same environment everywhere     |
| **Cloud-Ready**     | Works with Codespaces           |
| **Multi-Provider**  | Docker, K8s, Cloud              |
| **Feature-Rich**    | DevContainer features ecosystem |
| **Standardized**    | Industry-standard format        |

## Hardcoded Defaults

> **For maintainers:** The following values are either hardcoded in the generated `devcontainer.json` (not configurable via `sindri.yaml`) or are fallback defaults applied when a field is omitted from the config. DevPod supports 7 backend variants, each with its own defaults.

### Non-Configurable Template Values

**Source:** `sindri-providers/src/templates/devcontainer.json.tera`

| Value                                                  | Template Line | Description                     |
| ------------------------------------------------------ | ------------- | ------------------------------- |
| `INIT_WORKSPACE = "true"`                              | 5             | Always initializes workspace    |
| Volume mount target `/alt/home/developer`              | 22            | Home directory path             |
| `remoteUser: "developer"`                              | 25            | Fixed username                  |
| `workspaceFolder: "/alt/home/developer/workspace"`     | 26            | Fixed workspace path            |
| GPU run arg `"all"`                                    | 30            | All GPUs exposed (when enabled) |
| VS Code extension `ms-vscode-remote.remote-containers` | 36            | Required extension              |
| Terminal default profile `bash`                        | 39            | Default Linux shell             |
| `postStartCommand: "/docker/scripts/entrypoint.sh"`    | 43            | Fixed entrypoint path           |

**Source:** `sindri-providers/src/devpod.rs`

| Value                              | Line | Description                               |
| ---------------------------------- | ---- | ----------------------------------------- |
| IDE argument `"none"`              | 670  | Always passes `--ide none` to `devpod up` |
| Build arg `BUILD_FROM_SOURCE=true` | 255  | Always set for Docker builds              |
| Image tag for local K8s            | 395  | `sindri:local` (for kind/k3d clusters)    |
| Registry host fallback             | 414  | `docker.io` when not in buildRepository   |

### Configurable Fallback Defaults

**Source:** `sindri-providers/src/devpod.rs`

| Field                | Default    | Line | Override in `sindri.yaml` |
| -------------------- | ---------- | ---- | ------------------------- |
| DevPod provider type | `"docker"` | 54   | `providers.devpod.type`   |

**Source:** `sindri-providers/src/templates/context.rs`

| Field               | Default    | Line | Override in `sindri.yaml`             |
| ------------------- | ---------- | ---- | ------------------------------------- |
| Memory              | `"4GB"`    | 97   | `deployment.resources.memory`         |
| CPUs                | `2`        | 100  | `deployment.resources.cpus`           |
| Volume size         | `"10GB"`   | 109  | `deployment.volumes.workspace.size`   |
| GPU type            | `"nvidia"` | 118  | `deployment.resources.gpu.type`       |
| Network mode        | `"bridge"` | 133  | `providers.docker.network`            |
| DinD storage size   | `"20GB"`   | 153  | `providers.docker.dind.storageSize`   |
| DinD storage driver | `"auto"`   | 154  | `providers.docker.dind.storageDriver` |

### Variant-Specific Defaults

Each DevPod backend variant has its own defaults (applied when the corresponding field is omitted).

**Source:** `sindri-core/src/types/config_types.rs`

#### AWS (`providers.devpod.type: aws`)

| Field         | Default       | Line | Override                        |
| ------------- | ------------- | ---- | ------------------------------- |
| Region        | `"us-west-2"` | 773  | `providers.devpod.region`       |
| Instance type | `"c5.xlarge"` | 777  | `providers.devpod.instanceType` |
| Disk size     | `40` GB       | 780  | `providers.devpod.diskSize`     |

#### GCP (`providers.devpod.type: gcp`)

| Field        | Default           | Line | Override                       |
| ------------ | ----------------- | ---- | ------------------------------ |
| Zone         | `"us-central1-a"` | 800  | `providers.devpod.zone`        |
| Machine type | `"e2-standard-4"` | 804  | `providers.devpod.machineType` |
| Disk type    | `"pd-balanced"`   | 808  | `providers.devpod.diskType`    |
| Disk size    | `40` GB           | 780  | `providers.devpod.diskSize`    |

#### Azure (`providers.devpod.type: azure`)

| Field          | Default              | Line | Override                         |
| -------------- | -------------------- | ---- | -------------------------------- |
| Resource group | `"devpod-resources"` | 827  | `providers.devpod.resourceGroup` |
| Location       | `"eastus"`           | 831  | `providers.devpod.location`      |
| VM size        | `"Standard_D4s_v3"`  | 835  | `providers.devpod.vmSize`        |
| Disk size      | `40` GB              | 780  | `providers.devpod.diskSize`      |

#### DigitalOcean (`providers.devpod.type: digitalocean`)

| Field  | Default         | Line | Override                  |
| ------ | --------------- | ---- | ------------------------- |
| Region | `"nyc3"`        | 850  | `providers.devpod.region` |
| Size   | `"s-4vcpu-8gb"` | 854  | `providers.devpod.size`   |

#### Kubernetes (`providers.devpod.type: kubernetes`)

| Field     | Default    | Line | Override                     |
| --------- | ---------- | ---- | ---------------------------- |
| Namespace | `"devpod"` | 871  | `providers.devpod.namespace` |

#### SSH (`providers.devpod.type: ssh`)

| Field    | Default           | Line | Override                   |
| -------- | ----------------- | ---- | -------------------------- |
| User     | `"root"`          | 888  | `providers.devpod.user`    |
| Port     | `22`              | 892  | `providers.devpod.port`    |
| Key path | `"~/.ssh/id_rsa"` | 896  | `providers.devpod.keyPath` |

#### Docker (`providers.devpod.type: docker`)

No variant-specific defaults beyond the shared template context values above.

### Computed / Derived Values

| Value                 | Computation                                   | Source                      |
| --------------------- | --------------------------------------------- | --------------------------- |
| DevPod provider name  | Lowercased enum variant                       | `devpod.rs:53`              |
| K8s cluster detection | `kind-*` or `k3d-*` context prefix            | `devpod.rs:73, 89`          |
| SSH command           | `devpod ssh {name}`                           | `devpod.rs:795`             |
| Docker build args     | `SINDRI_VERSION`, `SINDRI_SOURCE_REF`         | `devpod.rs:244–245`         |
| Volume mount source   | `{name}_home` (computed from deployment name) | `devcontainer.json.tera:21` |

## Related Documentation

- [Provider Overview](README.md)
- [Kubernetes Provider](KUBERNETES.md)
- [Configuration Reference](../CONFIGURATION.md)
- [Secrets Management](../SECRETS_MANAGEMENT.md)
- [DevPod Documentation](https://devpod.sh/docs)
