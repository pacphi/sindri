# Kubernetes Provider

> **Version:** 3.x
> **Last Updated:** 2026-01

Container orchestration for enterprise, multi-tenant, and CI/CD environments.

## Overview

The Kubernetes provider deploys Sindri environments to any Kubernetes cluster with:

- **Local cluster support** - kind, k3d with automatic image loading
- **Remote cluster support** - EKS, GKE, AKS, self-hosted
- **Persistent volumes** - PVC-based storage
- **GPU support** - Via node selectors
- **Multi-tenant** - Namespace isolation

**Best for:** Enterprise environments, CI/CD pipelines, multi-tenant platforms

## Prerequisites

| Requirement    | Check Command          | Install                                                                   |
| -------------- | ---------------------- | ------------------------------------------------------------------------- |
| kubectl        | `kubectl version`      | [kubernetes.io/docs/tasks/tools](https://kubernetes.io/docs/tasks/tools/) |
| Cluster access | `kubectl cluster-info` | See cluster setup below                                                   |

**Optional (for local development):**

| Tool | Check Command  | Install                                       |
| ---- | -------------- | --------------------------------------------- |
| kind | `kind version` | [kind.sigs.k8s.io](https://kind.sigs.k8s.io/) |
| k3d  | `k3d version`  | [k3d.io](https://k3d.io/)                     |

## Quick Start

### With Local Cluster (kind)

```bash
# 1. Create local cluster
sindri k8s create --provider kind --name sindri-local

# 2. Create configuration
cat > sindri.yaml << 'EOF'
version: "1.0"
name: sindri-k8s

deployment:
  provider: kubernetes
  resources:
    memory: 4GB
    cpus: 2
  volumes:
    workspace:
      size: 10Gi

extensions:
  profile: fullstack

providers:
  kubernetes:
    namespace: sindri-dev
EOF

# 3. Deploy
sindri deploy

# 4. Connect
sindri connect
```

### With Multi-Node Local Cluster

```bash
# 1. Create 3-node kind cluster (1 control-plane + 2 workers)
sindri k8s create --provider kind --name sindri-multinode --nodes 3

# 2. Create configuration with minimal profile
cat > sindri.yaml << 'EOF'
version: "1.0"
name: sindri-minimal

deployment:
  provider: kubernetes
  resources:
    memory: 4GB
    cpus: 2
  volumes:
    workspace:
      size: 10Gi

extensions:
  profile: minimal  # Lightweight: nodejs, python only

providers:
  kubernetes:
    namespace: sindri-dev
EOF

# 3. Deploy
sindri deploy

# 4. Connect
sindri connect
```

### With Existing Cluster

```bash
# 1. Verify cluster access
kubectl cluster-info

# 2. Create namespace (optional)
kubectl create namespace sindri-dev

# 3. Create configuration
cat > sindri.yaml << 'EOF'
version: "1.0"
name: sindri-k8s

deployment:
  provider: kubernetes
  image: ghcr.io/my-org/sindri:latest  # Pre-pushed image
  resources:
    memory: 4GB
    cpus: 2
  volumes:
    workspace:
      size: 30Gi

extensions:
  profile: fullstack

providers:
  kubernetes:
    namespace: sindri-dev
    storageClass: standard
EOF

# 4. Deploy
sindri deploy
```

## Configuration

### Basic Configuration

```yaml
version: "1.0"
name: sindri-k8s

deployment:
  provider: kubernetes
  resources:
    memory: 2GB
    cpus: 1

extensions:
  profile: fullstack

providers:
  kubernetes:
    namespace: default
```

### Advanced Configuration

```yaml
version: "1.0"
name: sindri-k8s-prod

deployment:
  provider: kubernetes
  image: ghcr.io/my-org/sindri:v1.0.0
  resources:
    memory: 8GB
    cpus: 4
  volumes:
    workspace:
      size: 100Gi

extensions:
  profile: enterprise

providers:
  kubernetes:
    namespace: dev-environments
    storageClass: fast-ssd
    context: prod-cluster # Specific kubeconfig context
```

### GPU Configuration

```yaml
deployment:
  provider: kubernetes
  resources:
    memory: 32GB
    cpus: 8
    gpu:
      enabled: true
      type: nvidia
      count: 1

providers:
  kubernetes:
    namespace: gpu-workloads
```

GPU workloads automatically add node selector:

```yaml
# Generated manifest includes:
spec:
  nodeSelector:
    gpu: nvidia
```

## Image Requirements

Kubernetes provider does NOT build images during deployment (per K8s best practices).

Images should be:

1. Built in CI/CD pipelines
2. Pushed to accessible registries
3. Deployed via Sindri with explicit image reference

### Why No Build?

Kubernetes clusters are designed to pull images from registries, not build them. This approach ensures:

- **Immutable deployments** - Same image deploys the same way every time
- **Faster deployments** - No build step, just pull and run
- **GitOps compatibility** - Image tags can be tracked in version control
- **Security** - Images can be scanned before deployment

### Providing Images

```yaml
# Option 1: Explicit image reference
deployment:
  provider: kubernetes
  image: ghcr.io/myorg/app:v1.0.0

# Option 2: Use image_config for version resolution
deployment:
  provider: kubernetes
  image_config:
    registry: ghcr.io/myorg/app
    version: "^1.0.0"
    verify_signature: true

# Option 3: Immutable digest (best for production)
deployment:
  provider: kubernetes
  image_config:
    registry: ghcr.io/myorg/app
    digest: sha256:abc123...
```

### CI/CD Workflow for Kubernetes

Build images in your CI/CD pipeline, then deploy:

```yaml
# GitHub Actions example
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build and push
        run: |
          docker build -t ghcr.io/${{ github.repository }}:${{ github.sha }} .
          docker push ghcr.io/${{ github.repository }}:${{ github.sha }}

  deploy:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - name: Deploy to Kubernetes
        run: |
          cat > sindri.yaml << EOF
          deployment:
            provider: kubernetes
            image_config:
              registry: ghcr.io/${{ github.repository }}
              digest: sha256:${{ needs.build.outputs.digest }}
              verify_signature: true
          EOF
          sindri deploy
```

### Production Best Practices

For production deployments, use immutable digests:

```yaml
deployment:
  provider: kubernetes
  image_config:
    registry: ghcr.io/myorg/app
    digest: sha256:abc123... # Immutable - same image every time
    verify_signature: true # Verify image authenticity
    verify_provenance: true # Verify build provenance
```

This ensures:

- Reproducible deployments
- Rollback capability (just change the digest)
- Audit trail of what was deployed

## Deployment Commands

```bash
# Deploy (creates deployment, service, PVC)
sindri deploy

# Preview deployment plan
sindri plan

# Check status
sindri status

# Connect to pod
sindri connect

# Stop (scale to 0)
sindri stop

# Start (scale to 1)
sindri start

# Destroy (removes all resources)
sindri destroy
```

## Local Cluster Management

### Install Cluster Tool

```bash
# Install kind
sindri k8s install kind

# Or install k3d
sindri k8s install k3d
```

### Create Local Cluster

```bash
# Create single-node kind cluster
sindri k8s create --provider kind --name sindri-local

# Create multi-node kind cluster (1 control-plane + 2 workers)
sindri k8s create --provider kind --name sindri-local --nodes 3

# Create single-node k3d cluster
sindri k8s create --provider k3d --name sindri-local

# Create multi-node k3d cluster with registry
sindri k8s create --provider k3d --name sindri-local --nodes 3 --registry

# List clusters
sindri k8s list
```

### Delete Local Cluster

```bash
# Delete kind cluster
kind delete cluster --name sindri-local

# Delete k3d cluster
k3d cluster delete sindri-local
```

## Architecture

### Cluster Type Detection

V3 automatically detects cluster type from kubectl context:

```rust
// V3 detection logic
async fn detect_cluster_type(&self) -> ClusterType {
    let context = self.get_k8s_current_context().await;

    if context.starts_with("kind-") {
        ClusterType::Kind
    } else if context.starts_with("k3d-") {
        ClusterType::K3d
    } else {
        ClusterType::Remote
    }
}
```

### Image Loading for Local Clusters

For kind/k3d clusters, images are loaded directly without a registry:

```bash
# kind
kind load docker-image sindri:latest --name sindri-local

# k3d
k3d image import sindri:latest -c sindri-local
```

### Generated Resources

The provider creates these Kubernetes resources:

```yaml
# 1. Namespace (if needed)
apiVersion: v1
kind: Namespace
metadata:
  name: sindri-dev

---
# 2. PersistentVolumeClaim
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: sindri-k8s-home-pvc
  namespace: sindri-dev
spec:
  accessModes: [ReadWriteOnce]
  resources:
    requests:
      storage: 30Gi
  storageClassName: standard

---
# 3. Deployment
apiVersion: apps/v1
kind: Deployment
metadata:
  name: sindri-k8s
  namespace: sindri-dev
spec:
  replicas: 1
  selector:
    matchLabels:
      instance: sindri-k8s
  template:
    metadata:
      labels:
        instance: sindri-k8s
    spec:
      containers:
        - name: sindri
          image: sindri:latest
          env:
            - name: HOME
              value: /alt/home/developer
            - name: WORKSPACE
              value: /alt/home/developer/workspace
          volumeMounts:
            - name: home
              mountPath: /alt/home/developer
          resources:
            limits:
              cpu: "2"
              memory: 4Gi
            requests:
              cpu: "1"
              memory: 2Gi
      volumes:
        - name: home
          persistentVolumeClaim:
            claimName: sindri-k8s-home-pvc

---
# 4. Service
apiVersion: v1
kind: Service
metadata:
  name: sindri-k8s
  namespace: sindri-dev
spec:
  selector:
    instance: sindri-k8s
  ports:
    - port: 2222
      targetPort: 2222
```

## Connection Methods

### kubectl exec (Default)

```bash
# Via Sindri CLI
sindri connect

# Direct kubectl
kubectl exec -it <pod-name> -n sindri-dev -- /bin/bash

# Get pod name
kubectl get pods -n sindri-dev -l instance=sindri-k8s
```

### Port Forwarding

```bash
# Forward SSH port
kubectl port-forward pod/<pod-name> -n sindri-dev 10022:2222

# Connect via SSH
ssh developer@localhost -p 10022
```

### VS Code Kubernetes Extension

1. Install "Kubernetes" extension in VS Code
2. Connect to cluster
3. Right-click pod > "Attach Visual Studio Code"

## Secrets Management

### Kubernetes Secrets

```bash
# Create secret
kubectl create secret generic sindri-secrets \
  --from-literal=ANTHROPIC_API_KEY=sk-ant-... \
  --from-literal=GITHUB_TOKEN=ghp_... \
  -n sindri-dev

# Reference in sindri.yaml
providers:
  kubernetes:
    namespace: sindri-dev
    secrets:
      - sindri-secrets
```

### ImagePullSecret

For private registries, the provider auto-creates ImagePullSecret from `~/.docker/config.json`:

```bash
# Login to registry first
docker login ghcr.io

# Provider creates sindri-registry-creds secret automatically
```

Manual creation:

```bash
kubectl create secret docker-registry sindri-registry-creds \
  --docker-server=ghcr.io \
  --docker-username=<username> \
  --docker-password=<token> \
  -n sindri-dev
```

## Troubleshooting

### No Cluster Available

```bash
# Check kubectl context
kubectl config current-context

# List available contexts
kubectl config get-contexts

# Switch context
kubectl config use-context <context-name>

# Create local cluster
sindri k8s create --provider kind --name sindri-local
```

### Pod Not Starting

```bash
# Check pod status
kubectl get pods -n sindri-dev

# Describe pod for events
kubectl describe pod <pod-name> -n sindri-dev

# Check pod logs
kubectl logs <pod-name> -n sindri-dev

# Common issues:
# - ImagePullBackOff: Image not accessible
# - Pending: Insufficient resources or no nodes
# - CrashLoopBackOff: Container crashing
```

### Image Pull Errors

For local clusters (kind/k3d):

```bash
# Build image locally
docker build -t sindri:latest .

# Load into cluster
# For kind:
kind load docker-image sindri:latest --name sindri-local

# For k3d:
k3d image import sindri:latest -c sindri-local
```

For remote clusters:

```bash
# Push to registry
docker push ghcr.io/my-org/sindri:latest

# Verify ImagePullSecret
kubectl get secrets -n sindri-dev

# Create ImagePullSecret if needed
kubectl create secret docker-registry sindri-registry-creds \
  --docker-server=ghcr.io \
  --docker-username=$GITHUB_USER \
  --docker-password=$GITHUB_TOKEN \
  -n sindri-dev
```

### Volume Issues

```bash
# Check PVC status
kubectl get pvc -n sindri-dev

# Describe PVC
kubectl describe pvc sindri-k8s-home-pvc -n sindri-dev

# Check storage classes
kubectl get storageclass

# Common issues:
# - Pending: No storage provisioner
# - Volume not found: StorageClass doesn't exist
```

### Connection Refused

```bash
# Check pod is running
kubectl get pods -n sindri-dev

# Check if container is ready
kubectl get pods -n sindri-dev -o jsonpath='{.items[*].status.containerStatuses[*].ready}'

# Wait for pod to be ready
kubectl wait pod -n sindri-dev -l instance=sindri-k8s --for=condition=Ready --timeout=300s
```

### Resource Quota Issues

```bash
# Check namespace quotas
kubectl describe resourcequota -n sindri-dev

# Check limit ranges
kubectl describe limitrange -n sindri-dev
```

## kubectl Commands Reference

```bash
# Deployment management
kubectl get deployments -n sindri-dev
kubectl scale deployment sindri-k8s --replicas=0 -n sindri-dev
kubectl scale deployment sindri-k8s --replicas=1 -n sindri-dev
kubectl rollout restart deployment sindri-k8s -n sindri-dev

# Pod management
kubectl get pods -n sindri-dev
kubectl logs <pod-name> -n sindri-dev
kubectl describe pod <pod-name> -n sindri-dev
kubectl exec -it <pod-name> -n sindri-dev -- /bin/bash

# Volume management
kubectl get pvc -n sindri-dev
kubectl describe pvc sindri-k8s-home-pvc -n sindri-dev

# Resource cleanup
kubectl delete deployment sindri-k8s -n sindri-dev
kubectl delete pvc sindri-k8s-home-pvc -n sindri-dev
kubectl delete namespace sindri-dev
```

## Multi-Tenant Configuration

For shared clusters with multiple users:

```yaml
# User A
providers:
  kubernetes:
    namespace: dev-user-a

# User B
providers:
  kubernetes:
    namespace: dev-user-b
```

Apply RBAC for isolation:

```yaml
apiVersion: v1
kind: ResourceQuota
metadata:
  name: sindri-quota
  namespace: dev-user-a
spec:
  hard:
    cpu: "4"
    memory: 8Gi
    persistentvolumeclaims: "2"
```

## Cost Considerations

| Cluster Type | Compute Cost     | Storage Cost |
| ------------ | ---------------- | ------------ |
| kind (local) | Free             | Free         |
| k3d (local)  | Free             | Free         |
| EKS          | Per-hour + nodes | EBS pricing  |
| GKE          | Per-hour + nodes | PD pricing   |
| AKS          | Per-hour + nodes | Disk pricing |

**Local development:** Use kind or k3d for free local clusters.

**Cloud clusters:** Consider using:

- Spot instances for development
- Preemptible VMs where appropriate
- Right-sized node pools

## Related Documentation

- [Provider Overview](README.md)
- [DevPod Provider](DEVPOD.md) (K8s backend)
- [Configuration Reference](../CONFIGURATION.md)
- [CLI Reference](../CLI.md)
- [Local Cluster ADR](../architecture/adr/029-local-kubernetes-cluster-management.md)
