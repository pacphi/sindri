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

## Related Documentation

- [Provider Overview](README.md)
- [Kubernetes Provider](KUBERNETES.md)
- [Configuration Reference](../CONFIGURATION.md)
- [Secrets Management](../SECRETS_MANAGEMENT.md)
- [DevPod Documentation](https://devpod.sh/docs)
