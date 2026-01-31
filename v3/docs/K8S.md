# Sindri V3 Kubernetes Guide

> **Version**: V3 | Last Updated: 2026-01-26

Complete guide for using Sindri with Kubernetes, including local cluster management and workload deployment.

## Overview

Sindri V3 provides comprehensive Kubernetes support through two complementary systems:

1. **Local Cluster Management** (`sindri k8s`) - Create and manage local Kubernetes clusters using kind or k3d
2. **Kubernetes Provider** (`--provider kubernetes`) - Deploy Sindri environments to any Kubernetes cluster

### Architecture

```
sindri-clusters crate          sindri-providers crate
       |                               |
       v                               v
+------------------+         +--------------------+
| ClusterProvider  |         | KubernetesProvider |
| - Kind           |         | - Deploy workloads |
| - K3d            |         | - Manage pods      |
+------------------+         | - Connect to envs  |
       |                     +--------------------+
       v                               |
+------------------+                   |
| Local K8s        | <-----------------+
| Cluster          |
+------------------+
```

## Prerequisites

### Required Tools

| Tool    | Purpose           | Installation                            |
| ------- | ----------------- | --------------------------------------- |
| kubectl | Kubernetes CLI    | https://kubernetes.io/docs/tasks/tools/ |
| Docker  | Container runtime | https://docs.docker.com/get-docker/     |

### Optional Tools (Local Clusters)

| Tool | Purpose              | Installation                                           |
| ---- | -------------------- | ------------------------------------------------------ |
| kind | Kubernetes IN Docker | `sindri k8s install kind` or https://kind.sigs.k8s.io/ |
| k3d  | K3s in Docker        | `sindri k8s install k3d` or https://k3d.io/            |

### Check Prerequisites

```bash
# Check all K8s-related tools
sindri doctor --provider k8s

# Verify Docker is running
docker info

# Check kubectl connectivity (if cluster exists)
kubectl cluster-info
```

## Local Cluster Management

The `sindri k8s` commands manage local Kubernetes clusters for development and testing.

### Supported Providers

| Provider | Description                                   | Best For                       |
| -------- | --------------------------------------------- | ------------------------------ |
| **kind** | Full Kubernetes in Docker containers          | Testing, CI, full K8s features |
| **k3d**  | Lightweight K3s in Docker with local registry | Fast development, local images |

### Create a Cluster

```bash
# Create with defaults (kind, single node)
sindri k8s create

# Create kind cluster
sindri k8s create --provider kind --name dev-cluster

# Create k3d cluster with local registry
sindri k8s create --provider k3d --name dev-cluster --registry

# Multi-node cluster
sindri k8s create --provider kind --name multi-node --nodes 3

# Specific Kubernetes version
sindri k8s create --k8s-version v1.34.0
```

#### CLI Options

| Option                    | Short | Default      | Description                      |
| ------------------------- | ----- | ------------ | -------------------------------- |
| `--provider <PROVIDER>`   | `-p`  | kind         | Cluster provider (kind, k3d)     |
| `--name <NAME>`           | `-n`  | sindri-local | Cluster name                     |
| `--nodes <N>`             | `-N`  | 1            | Number of nodes                  |
| `--k8s-version <VERSION>` | -     | v1.35.0      | Kubernetes version               |
| `--registry`              | -     | -            | Enable local registry (k3d only) |
| `--registry-port <PORT>`  | -     | 5000         | Registry port (k3d only)         |

### List Clusters

```bash
# List all clusters
sindri k8s list

# List only kind clusters
sindri k8s list --provider kind

# Output as JSON
sindri k8s list --json
```

### Check Cluster Status

```bash
# Check default cluster
sindri k8s status

# Check specific cluster
sindri k8s status --name dev-cluster

# JSON output
sindri k8s status --name dev-cluster --json
```

### Get Kubeconfig

```bash
# Print kubeconfig
sindri k8s config --name dev-cluster

# Save to file
sindri k8s config --name dev-cluster > ~/.kube/dev-cluster.yaml
export KUBECONFIG=~/.kube/dev-cluster.yaml
```

### Destroy Cluster

```bash
# Destroy with confirmation
sindri k8s destroy --name dev-cluster

# Force destroy without confirmation
sindri k8s destroy --name dev-cluster --force
```

### Install Cluster Tools

```bash
# Install kind
sindri k8s install kind

# Install k3d
sindri k8s install k3d -y
```

## Configuration

### sindri.yaml Configuration

Configure K8s settings in your `sindri.yaml`:

```yaml
version: "1.0"
name: my-project

deployment:
  provider: kubernetes # Use K8s provider
  resources:
    memory: 4GB
    cpus: 2
  volumes:
    workspace:
      size: 20GB

extensions:
  profile: fullstack

# Local cluster configuration
providers:
  k8s:
    provider: kind # or k3d
    clusterName: sindri-local
    nodes: 1

    # Kind-specific options
    kind:
      image: kindest/node:v1.35.0
      configFile: kind-config.yaml # Optional custom config

    # K3d-specific options
    k3d:
      image: rancher/k3s:v1.35.0-k3s1
      registry:
        enabled: true
        name: k3d-registry
        port: 5000

  # Kubernetes deployment options
  kubernetes:
    namespace: sindri
    storageClass: standard # Or local-path for k3d
```

### DevPod with Kubernetes Backend

Use DevPod to target a Kubernetes cluster:

```yaml
version: "1.0"
name: sindri-k8s-devpod

deployment:
  provider: devpod
  resources:
    memory: 4GB
    cpus: 2

providers:
  k8s:
    provider: kind
    clusterName: sindri-kind-local

  devpod:
    type: kubernetes
    kubernetes:
      context: kind-sindri-kind-local
      namespace: sindri
      storageClass: standard
```

### Context Naming Convention

Cluster tools create kubectl contexts with predictable names:

| Provider | Context Pattern       | Example             |
| -------- | --------------------- | ------------------- |
| kind     | `kind-{cluster-name}` | `kind-sindri-local` |
| k3d      | `k3d-{cluster-name}`  | `k3d-sindri-local`  |

## Deploying to Kubernetes

### Deploy Sindri Environment

```bash
# Initialize for Kubernetes
sindri config init --provider kubernetes --profile kubernetes

# Deploy to current kubectl context
sindri deploy

# Deploy with dry-run preview
sindri deploy --dry-run

# Force recreation
sindri deploy --force

# Wait for deployment with custom timeout
sindri deploy --wait --timeout 600
```

### Connect to Environment

```bash
# Interactive shell
sindri connect

# Run specific command
sindri connect -c "ls -la"

# Direct kubectl access
kubectl exec -it <pod-name> -n sindri -- /bin/bash
```

### Check Status

```bash
sindri status
sindri status --json
```

### Destroy Deployment

```bash
# Destroy with confirmation
sindri destroy

# Force destroy
sindri destroy --force

# Also remove volumes
sindri destroy --volumes --force
```

## Working with Local Registries

K3d supports a local Docker registry for pushing custom images:

### Enable Registry

```bash
# Create cluster with registry
sindri k8s create --provider k3d --name dev --registry --registry-port 5000
```

### Push Images to Registry

```bash
# Tag image for local registry
docker tag myimage localhost:5000/myimage:latest

# Push to local registry
docker push localhost:5000/myimage:latest

# Use in deployment
# Image: localhost:5000/myimage:latest
```

### Registry in sindri.yaml

```yaml
providers:
  k8s:
    provider: k3d
    clusterName: dev
    k3d:
      registry:
        enabled: true
        name: dev-registry
        port: 5000
```

## GPU Support

Sindri supports GPU workloads on Kubernetes clusters with NVIDIA GPUs:

### Configuration

```yaml
deployment:
  resources:
    memory: 8GB
    cpus: 4
    gpu:
      enabled: true
      count: 1
      type: nvidia

providers:
  kubernetes:
    namespace: sindri-gpu
```

### Requirements

- Kubernetes cluster with NVIDIA GPU nodes
- NVIDIA device plugin installed
- Nodes labeled with `gpu=nvidia`

## Examples

### Example: Kind Minimal Setup

File: `examples/v3/k8s/kind-minimal.sindri.yaml`

```yaml
version: "1.0"
name: sindri-kind-local

deployment:
  provider: devpod
  resources:
    memory: 4GB
    cpus: 2
  volumes:
    workspace:
      size: 20GB

extensions:
  profile: fullstack

providers:
  k8s:
    provider: kind
    clusterName: sindri-kind-local
    nodes: 1

  devpod:
    type: kubernetes
    kubernetes:
      context: kind-sindri-kind-local
      namespace: sindri
      storageClass: standard
```

**Usage:**

```bash
# Create cluster
sindri k8s create --config examples/v3/k8s/kind-minimal.sindri.yaml

# Deploy Sindri
sindri deploy --config examples/v3/k8s/kind-minimal.sindri.yaml

# Destroy
sindri k8s destroy --config examples/v3/k8s/kind-minimal.sindri.yaml
```

### Example: K3d with Registry

File: `examples/v3/k8s/k3d-with-registry.sindri.yaml`

```yaml
version: "1.0"
name: sindri-k3d-dev

deployment:
  provider: devpod
  resources:
    memory: 4GB
    cpus: 2
  volumes:
    workspace:
      size: 30GB

extensions:
  profile: devops

secrets:
  - name: AWS_ACCESS_KEY_ID
    source: env
  - name: AWS_SECRET_ACCESS_KEY
    source: env

providers:
  k8s:
    provider: k3d
    clusterName: sindri-k3d-dev
    nodes: 3 # 1 server + 2 agents
    k3d:
      registry:
        enabled: true
        name: sindri-registry
        port: 5000

  devpod:
    type: kubernetes
    kubernetes:
      context: k3d-sindri-k3d-dev
      namespace: sindri
      storageClass: local-path
```

**Usage:**

```bash
# Create cluster with registry
sindri k8s create --config examples/v3/k8s/k3d-with-registry.sindri.yaml

# Push custom images
docker tag myapp localhost:5000/myapp
docker push localhost:5000/myapp

# Deploy
sindri deploy --config examples/v3/k8s/k3d-with-registry.sindri.yaml
```

## Troubleshooting

### No Kubernetes Cluster Available

```
Error: No Kubernetes cluster available
```

**Solution:**

```bash
# Install a cluster tool
sindri k8s install kind

# Create a cluster
sindri k8s create --name sindri-local
```

### Cluster Not Accessible (Docker Stopped)

```
Cluster not accessible. Docker may be stopped.
```

**Solution:**

```bash
# Start Docker
# macOS/Windows: Start Docker Desktop
# Linux:
sudo systemctl start docker

# Verify cluster status
sindri k8s status --name <cluster-name>
```

### Kind Not Installed

```
Error: kind is not installed. Run: sindri k8s install kind
```

**Solution:**

```bash
# Install via Sindri
sindri k8s install kind

# Or via Homebrew (macOS)
brew install kind

# Or direct download (Linux)
curl -Lo ./kind https://kind.sigs.k8s.io/dl/latest/kind-linux-amd64
chmod +x ./kind
sudo mv ./kind /usr/local/bin/kind
```

### K3d Not Installed

```
Error: k3d is not installed. Run: sindri k8s install k3d
```

**Solution:**

```bash
# Install via Sindri
sindri k8s install k3d

# Or via Homebrew (macOS)
brew install k3d

# Or via install script (Linux/macOS)
curl -s https://raw.githubusercontent.com/k3d-io/k3d/main/install.sh | bash
```

### Pod Not Starting

```
Error: Pod failed to become ready
```

**Diagnosis:**

```bash
# Check pod status
kubectl get pods -n sindri

# Check pod events
kubectl describe pod <pod-name> -n sindri

# Check pod logs
kubectl logs <pod-name> -n sindri
```

**Common Causes:**

- Image pull errors (check registry access)
- Insufficient resources (check node capacity)
- PVC binding issues (check storage class)

### Image Pull Errors

```
Failed to pull image: unauthorized
```

**Solution for private registries:**

```bash
# Login to registry
docker login ghcr.io

# Sindri will auto-create ImagePullSecret from ~/.docker/config.json
sindri deploy
```

### Storage Class Not Found

```
storageclass.storage.k8s.io "standard" not found
```

**Solution:**

```yaml
# For k3d, use local-path
providers:
  kubernetes:
    storageClass: local-path

# For kind, use standard (default)
providers:
  kubernetes:
    storageClass: standard
```

### Kubectl Context Issues

```
Error: context "kind-my-cluster" not found
```

**Solution:**

```bash
# List available contexts
kubectl config get-contexts

# Check if cluster exists
sindri k8s list

# Recreate cluster if needed
sindri k8s destroy --name my-cluster --force
sindri k8s create --name my-cluster
```

## Best Practices

### Development Workflow

1. **Use k3d with registry** for faster local iteration
2. **Single-node clusters** for development (faster startup)
3. **Multi-node clusters** for testing distributed scenarios
4. **Clean up regularly** to free Docker resources

### CI/CD Integration

1. **Use kind** for CI environments (more widely supported)
2. **Set deterministic versions** for reproducibility
3. **Use `--wait` flag** to ensure deployment readiness
4. **Clean up clusters** after test runs

### Resource Management

```bash
# Check Docker resource usage
docker system df

# Clean up unused resources
docker system prune

# Destroy unused clusters
sindri k8s list
sindri k8s destroy --name unused-cluster --force
```

## See Also

- [CLI Reference](./CLI.md) - Complete CLI documentation
- [Configuration Reference](./CONFIGURATION.md) - sindri.yaml options
- [Doctor Guide](./DOCTOR.md) - System diagnostics
- [ADR-029](./architecture/adr/029-local-kubernetes-cluster-management.md) - Architecture decisions
