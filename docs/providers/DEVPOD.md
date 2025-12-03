# DevPod Integration Guide

## Overview

DevPod support enables Sindri environments to run as DevContainers with full VS Code, GitHub Codespaces, and DevPod CLI compatibility. The Sindri CLI provides full lifecycle management for DevPod workspaces.

## Quick Start

```bash
# Deploy to DevPod (auto-detects provider type from sindri.yaml)
./cli/sindri deploy --provider devpod

# Check status
./cli/sindri status

# Connect
./cli/sindri connect

# Destroy
./cli/sindri destroy
```

## What Happens on Deploy

When you run `sindri deploy --provider devpod`, the adapter:

1. **Generates** `.devcontainer/devcontainer.json` with VS Code extensions, volumes, and port forwarding
2. **Adds** the DevPod provider if not already configured (e.g., `devpod provider add kubernetes`)
3. **Configures** provider options (context, namespace, storage class for k8s)
4. **Creates** namespace if using Kubernetes backend
5. **Runs** `devpod up` to create the workspace

## Lifecycle Commands

| Command                           | Description             |
| --------------------------------- | ----------------------- |
| `sindri deploy --provider devpod` | Create/update workspace |
| `sindri connect`                  | SSH into workspace      |
| `sindri status`                   | Show workspace status   |
| `sindri plan`                     | Show deployment plan    |
| `sindri destroy`                  | Delete workspace        |

## Usage Methods

### 1. Sindri CLI (Recommended)

```bash
# Full deployment with automatic provider setup
./cli/sindri deploy --provider devpod

# Connect
./cli/sindri connect

# Or use devpod directly after sindri creates the workspace
devpod ssh my-sindri-dev
```

### 2. VS Code Dev Containers

```bash
# Generate config only (no deployment)
./deploy/adapters/devpod-adapter.sh deploy --config-only sindri.yaml

# Open VS Code
code .

# Open in container
# Ctrl+Shift+P -> "Dev Containers: Open Folder in Container"
```

### 3. GitHub Codespaces

```bash
# Generate DevContainer config
./deploy/adapters/devpod-adapter.sh deploy --config-only sindri.yaml

# Push repository with .devcontainer
git add .devcontainer
git commit -m "Add DevContainer configuration"
git push

# Create codespace from GitHub UI
```

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

### Configure Features

DevContainer features add capabilities:

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

## Multi-Provider Support

DevPod is a **meta-provider** that can deploy to multiple backends:

| Backend        | CLI Provider   | sindri.yaml `type` | Example Configs                 |
| -------------- | -------------- | ------------------ | ------------------------------- |
| Docker (local) | `docker`       | `docker`           | `examples/devpod/`              |
| AWS EC2        | `aws`          | `aws`              | `examples/devpod/aws/`          |
| GCP Compute    | `gcp`          | `gcp`              | `examples/devpod/gcp/`          |
| Azure VMs      | `azure`        | `azure`            | `examples/devpod/azure/`        |
| DigitalOcean   | `digitalocean` | `digitalocean`     | `examples/devpod/digitalocean/` |
| Kubernetes     | `kubernetes`   | `kubernetes`       | `examples/devpod/kubernetes/`   |
| SSH Host       | `ssh`          | `ssh`              | N/A                             |

### Docker (Local)

```bash
devpod up . --provider docker
```

### Kubernetes

```bash
devpod up . --provider kubernetes --options "NAMESPACE=dev"
```

**sindri.yaml configuration:**

```yaml
deployment:
  provider: devpod

providers:
  devpod:
    type: kubernetes
    kubernetes:
      namespace: sindri-dev
      storageClass: standard
      context: my-cluster # Optional: specific kubeconfig context
```

**CI Testing with Kind:**

When testing in CI without an external cluster:

| KUBECONFIG Secret    | CI Behavior               |
| -------------------- | ------------------------- |
| Not provided         | Auto-creates kind cluster |
| Provided (file path) | Uses that kubeconfig      |
| Provided (content)   | Writes to ~/.kube/config  |

The CI workflow automatically handles cluster provisioning:

```yaml
# In GitHub Actions - no KUBECONFIG needed
providers: devpod-k8s # Kind cluster auto-created
```

**Example directories:**

| Directory                     | Purpose                        | Used By              |
| ----------------------------- | ------------------------------ | -------------------- |
| `examples/devpod/kubernetes/` | Deploy to existing K8s cluster | CI (`devpod-k8s`)    |
| `examples/k8s/`               | Create local cluster + deploy  | Manual local testing |

### AWS

```bash
devpod up . --provider aws --options "REGION=us-west-2,INSTANCE_TYPE=t3.medium"
```

### SSH

```bash
devpod up . --provider ssh --options "HOST=dev.example.com,USER=developer"
```

## Benefits

1. **IDE Integration** - Full VS Code features
2. **Reproducible** - Same environment everywhere
3. **Cloud-Ready** - Works with Codespaces
4. **Multi-Provider** - Docker, K8s, Cloud
5. **Feature-Rich** - DevContainer features ecosystem
6. **Standardized** - Industry-standard format

## Image Building for Kubernetes

When deploying to Kubernetes or cloud providers, Sindri automatically handles Docker image building and distribution.

### Image Strategy by Provider

| Provider Type         | Strategy                        | Configuration              |
| --------------------- | ------------------------------- | -------------------------- |
| Local Docker          | Build from Dockerfile           | None (automatic)           |
| kind/k3d              | Auto-detect & load into cluster | None (zero-config)         |
| External K8s          | Build & push to registry        | `buildRepository` required |
| Cloud (AWS/GCP/Azure) | Build & push to registry        | `buildRepository` required |

### Local Clusters (kind/k3d) - Zero Config

Sindri automatically detects local Kubernetes clusters and handles image loading:

```bash
# Just deploy - kind/k3d auto-detected
./cli/sindri deploy --provider devpod

# What happens behind the scenes:
# 1. Detects kind-* or k3d-* context
# 2. Builds sindri:latest locally
# 3. Loads image into cluster (kind load / k3d image import)
```

### Remote/Cloud Kubernetes - Build Repository Required

For external clusters, configure a registry where the image can be pushed:

#### Option 1: CLI flag

```bash
./cli/sindri deploy --provider devpod --build-repository ghcr.io/myorg/sindri
```

#### Option 2: sindri.yaml configuration

```yaml
providers:
  devpod:
    type: kubernetes
    buildRepository: ghcr.io/myorg/sindri
    kubernetes:
      namespace: sindri-dev
```

### Docker Registry Credentials

Sindri looks for registry credentials in this order:

1. **Environment variables**: `DOCKER_USERNAME`, `DOCKER_PASSWORD`
2. **.env.local** or **.env** files
3. **Existing Docker login**: `~/.docker/config.json`

**Special registry support:**

| Registry                        | Credential Source                      |
| ------------------------------- | -------------------------------------- |
| ghcr.io                         | `GITHUB_TOKEN` environment variable    |
| ECR (_.dkr.ecr._.amazonaws.com) | AWS CLI (`aws ecr get-login-password`) |
| GCR (\*.gcr.io)                 | `GOOGLE_APPLICATION_CREDENTIALS`       |
| Other                           | `DOCKER_USERNAME` + `DOCKER_PASSWORD`  |

### Viewing the Image Strategy

Use `sindri plan` to see what image strategy will be used:

```bash
./cli/sindri plan --provider devpod

# Output shows:
# Image Strategy:
#   Local cluster detected: kind:sindri-dev
#   → Build locally and load into cluster
```

## Related Documentation

- [Deployment Overview](../DEPLOYMENT.md)
- [Configuration Reference](../CONFIGURATION.md)
- [Troubleshooting](../TROUBLESHOOTING.md)
