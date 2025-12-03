# Kubernetes Deployment Guide

Deploy Sindri to Kubernetes clusters via DevPod.

## Overview

Sindri deploys to Kubernetes using **DevPod** as the orchestration layer. DevPod handles
Pod, PVC, and Service creation while providing DevContainer compatibility and IDE integration.

**Best for:** Enterprise teams, multi-tenant environments, CI/CD pipelines, IDE integration

**How it works:**

1. You configure `sindri.yaml` with `provider: devpod` and `type: kubernetes`
2. Sindri generates a `devcontainer.json` and invokes DevPod
3. DevPod creates the Kubernetes resources (Pod, PVC, Service)
4. You connect via SSH, VS Code, or the sindri CLI

## Quick Start

### Deploy to Existing Cluster

```bash
# 1. Create sindri.yaml (or use an example)
cp examples/devpod/kubernetes/minimal.sindri.yaml sindri.yaml

# 2. Deploy
./cli/sindri deploy

# 3. Connect
./cli/sindri connect

# 4. When done
./cli/sindri destroy
```

### Deploy to Local Cluster (kind/k3d)

```bash
# 1. Create a local kind cluster
./cli/sindri k8s create --config examples/k8s/kind-minimal.sindri.yaml

# 2. Deploy Sindri to it
./cli/sindri deploy --config examples/k8s/kind-minimal.sindri.yaml

# 3. Connect
./cli/sindri connect

# 4. Cleanup
./cli/sindri destroy
./cli/sindri k8s destroy --name sindri-kind-local
```

## Prerequisites

- Kubernetes cluster (1.24+) or ability to create local clusters (kind/k3d)
- `kubectl` configured with cluster access
- `devpod` CLI installed
- Storage class for persistent volumes
- (For external clusters) Container registry access with `buildRepository`

## Configuration

### Basic Configuration

```yaml
# sindri.yaml
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
```

### With Specific Context

```yaml
providers:
  devpod:
    type: kubernetes
    kubernetes:
      context: my-cluster-context # kubectl context to use
      namespace: dev-environments
      storageClass: fast-ssd
```

### GPU Workloads

```yaml
deployment:
  provider: devpod
  resources:
    memory: 32GB
    cpus: 8
    gpu:
      enabled: true
      type: nvidia
      tier: gpu-medium # Selects nodes with A10G GPUs
      count: 1
  volumes:
    workspace:
      size: 100GB

extensions:
  profile: ai-dev

providers:
  devpod:
    type: kubernetes
    kubernetes:
      namespace: gpu-workloads
      storageClass: fast-ssd
```

**GPU Tier Mapping:**

| Tier         | GPU      | Node Selector     |
| ------------ | -------- | ----------------- |
| `gpu-small`  | Tesla T4 | `nvidia-tesla-t4` |
| `gpu-medium` | A10G     | `nvidia-a10g`     |
| `gpu-large`  | L40S     | `nvidia-l40s`     |
| `gpu-xlarge` | A100     | `nvidia-a100`     |

## Deployment Commands

| Command          | Description                            |
| ---------------- | -------------------------------------- |
| `sindri deploy`  | Create/update the Kubernetes workspace |
| `sindri connect` | SSH into the workspace                 |
| `sindri status`  | Show workspace status                  |
| `sindri plan`    | Preview deployment plan                |
| `sindri destroy` | Delete the workspace                   |

## Image Handling

### Local Clusters (kind/k3d)

Sindri auto-detects local clusters and loads images directly:

```bash
# Image is built locally and loaded into the cluster
docker build -t sindri:latest .
kind load docker-image sindri:latest --name <cluster>
# or
k3d image import sindri:latest -c <cluster>
```

No registry configuration required.

### External Clusters

For remote/cloud clusters, specify a container registry:

```yaml
providers:
  devpod:
    type: kubernetes
    buildRepository: ghcr.io/your-org/sindri
    kubernetes:
      namespace: dev-environments
```

The image will be built and pushed to the registry, then pulled by the cluster.

## Local Cluster Management

Sindri can create and manage local Kubernetes clusters for development.

### Commands

```bash
# List local clusters (kind and k3d)
./cli/sindri k8s list

# Create cluster from config
./cli/sindri k8s create --config examples/k8s/kind-minimal.sindri.yaml

# Get kubeconfig for external tools
./cli/sindri k8s config --name sindri-kind-local

# Destroy cluster
./cli/sindri k8s destroy --name sindri-kind-local
```

### kind Cluster Configuration

```yaml
# examples/k8s/kind-minimal.sindri.yaml
version: "1.0"
name: sindri-kind-local

deployment:
  provider: devpod
  resources:
    memory: 4GB
    cpus: 2

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

### k3d Cluster with Registry

```yaml
providers:
  k8s:
    provider: k3d
    clusterName: sindri-k3d-local
    nodes: 1
    registry:
      enabled: true
      port: 5000

  devpod:
    type: kubernetes
    kubernetes:
      context: k3d-sindri-k3d-local
      namespace: sindri
```

## What DevPod Creates

When you deploy, DevPod creates these Kubernetes resources:

| Resource                  | Purpose                                        |
| ------------------------- | ---------------------------------------------- |
| **Pod**                   | Runs the Sindri container                      |
| **PersistentVolumeClaim** | Persists `/alt/home/developer` across restarts |
| **Service**               | Exposes SSH on port 2222                       |

You don't manage these directly - DevPod handles the lifecycle. To inspect:

```bash
# Find pods created by DevPod
kubectl get pods -n <namespace> -l devpod.sh/created=true

# View PVCs
kubectl get pvc -n <namespace>

# View services
kubectl get svc -n <namespace>
```

## Secrets Management

### Using Kubernetes Secrets

```bash
# Create secret
kubectl create secret generic sindri-secrets \
  --from-literal=ANTHROPIC_API_KEY=sk-ant-... \
  --from-literal=GITHUB_TOKEN=ghp_... \
  --from-literal=GIT_USER_NAME="Your Name" \
  --from-literal=GIT_USER_EMAIL="you@example.com" \
  -n dev-environments

# Update secret
kubectl create secret generic sindri-secrets \
  --from-literal=ANTHROPIC_API_KEY=sk-ant-new-key \
  -n dev-environments \
  --dry-run=client -o yaml | kubectl apply -f -
```

### External Secrets Operator

For enterprise secret management with Vault, AWS Secrets Manager, etc.:

```yaml
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: sindri-secrets
  namespace: dev-environments
spec:
  refreshInterval: 1h
  secretStoreRef:
    kind: ClusterSecretStore
    name: vault-backend
  target:
    name: sindri-secrets
  data:
    - secretKey: ANTHROPIC_API_KEY
      remoteRef:
        key: dev/sindri
        property: anthropic_api_key
```

## Troubleshooting

### Pod Won't Start

```bash
# Check pod status
kubectl get pods -n <namespace>

# View pod events
kubectl describe pod <pod-name> -n <namespace>

# Check logs
kubectl logs <pod-name> -n <namespace> --previous
```

### PVC Pending

```bash
# Check PVC status
kubectl get pvc -n <namespace>

# Check available storage classes
kubectl get storageclass

# Check events
kubectl describe pvc <pvc-name> -n <namespace>
```

### Connection Issues

```bash
# Test connectivity from within the cluster
kubectl exec -it <pod-name> -n <namespace> -- curl -v localhost:2222

# Check service endpoints
kubectl get endpoints -n <namespace>

# Port forward for direct access
kubectl port-forward <pod-name> 2222:2222 -n <namespace>
ssh developer@localhost -p 2222
```

### DevPod Issues

```bash
# Check DevPod workspace status
devpod list

# View DevPod logs
devpod logs <workspace-name>

# Force recreate
devpod up . --recreate
```

## Best Practices

1. **Use namespaces** - Isolate environments per user/team
2. **Set resource limits** - Prevent resource contention on shared clusters
3. **Use appropriate storage classes** - Match performance requirements (SSD for dev work)
4. **Configure RBAC** - Limit access per user/role in multi-tenant setups
5. **Use local clusters for testing** - kind/k3d are free and fast to create

## Example Configurations

| Example                                               | Use Case                |
| ----------------------------------------------------- | ----------------------- |
| `examples/devpod/kubernetes/minimal.sindri.yaml`      | Basic K8s deployment    |
| `examples/devpod/kubernetes/devops.sindri.yaml`       | DevOps tooling          |
| `examples/devpod/kubernetes/systems.sindri.yaml`      | Systems programming     |
| `examples/devpod/kubernetes/gpu-workload.sindri.yaml` | GPU/AI workloads        |
| `examples/k8s/kind-minimal.sindri.yaml`               | Local kind cluster      |
| `examples/k8s/k3d-with-registry.sindri.yaml`          | Local k3d with registry |

## Related Documentation

- [DevPod Provider Guide](DEVPOD.md) - Full DevPod documentation
- [Deployment Overview](../DEPLOYMENT.md) - All deployment options
- [Configuration Reference](../CONFIGURATION.md) - Complete sindri.yaml reference

---

## Appendix A: Manual Kubernetes Deployment

For users who need full control over Kubernetes resources or cannot use DevPod
(e.g., strict enterprise policies, GitOps workflows with ArgoCD/Flux).

### When to Use Manual Deployment

- Enterprise environments with strict deployment policies
- Integration with existing GitOps workflows
- Custom resource requirements DevPod doesn't support
- Air-gapped environments without DevPod

### Reference Manifests

These manifests can be customized and applied directly with `kubectl`.

#### StatefulSet

```yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: sindri
  namespace: dev-environments
  labels:
    app: sindri
spec:
  serviceName: sindri
  replicas: 1
  selector:
    matchLabels:
      app: sindri
  template:
    metadata:
      labels:
        app: sindri
    spec:
      containers:
        - name: sindri
          image: ghcr.io/your-org/sindri:latest
          env:
            - name: HOME
              value: /alt/home/developer
            - name: WORKSPACE
              value: /alt/home/developer/workspace
            - name: INSTALL_PROFILE
              value: fullstack
          resources:
            requests:
              memory: "2Gi"
              cpu: "1"
            limits:
              memory: "4Gi"
              cpu: "2"
          volumeMounts:
            - name: home
              mountPath: /alt/home/developer
          ports:
            - containerPort: 2222
              name: ssh
          livenessProbe:
            tcpSocket:
              port: 2222
            initialDelaySeconds: 30
            periodSeconds: 10
          readinessProbe:
            tcpSocket:
              port: 2222
            initialDelaySeconds: 5
            periodSeconds: 5
  volumeClaimTemplates:
    - metadata:
        name: home
      spec:
        accessModes: ["ReadWriteOnce"]
        storageClassName: fast-ssd
        resources:
          requests:
            storage: 30Gi
```

#### Service

```yaml
apiVersion: v1
kind: Service
metadata:
  name: sindri
  namespace: dev-environments
spec:
  selector:
    app: sindri
  ports:
    - port: 2222
      targetPort: 2222
      name: ssh
  type: ClusterIP
```

#### Namespace and ResourceQuota (Optional)

```yaml
apiVersion: v1
kind: Namespace
metadata:
  name: dev-environments
---
apiVersion: v1
kind: ResourceQuota
metadata:
  name: sindri-quota
  namespace: dev-environments
spec:
  hard:
    requests.cpu: "4"
    requests.memory: "8Gi"
    limits.cpu: "8"
    limits.memory: "16Gi"
    persistentvolumeclaims: "5"
    requests.storage: "100Gi"
```

### Manual Deployment Steps

```bash
# 1. Create namespace
kubectl create namespace dev-environments

# 2. Create secrets
kubectl create secret generic sindri-secrets \
  --from-literal=ANTHROPIC_API_KEY=sk-ant-... \
  --from-literal=GITHUB_TOKEN=ghp_... \
  -n dev-environments

# 3. Apply manifests
kubectl apply -f sindri-statefulset.yaml
kubectl apply -f sindri-service.yaml

# 4. Wait for ready
kubectl rollout status statefulset/sindri -n dev-environments

# 5. Connect via port-forward
kubectl port-forward sindri-0 2222:2222 -n dev-environments
ssh developer@localhost -p 2222

# 6. Install extensions manually (inside container)
extension-manager install-profile fullstack
```

### Limitations of Manual Deployment

- No IDE integration (VS Code Remote Containers, etc.)
- Manual extension installation required after each pod recreation
- No `sindri` CLI lifecycle management
- User responsible for updates and maintenance
- No automatic image building/loading for local clusters

For most users, the DevPod-based deployment is recommended.

---

## Appendix B: Multi-Tenant Reference Architecture

For platform teams deploying Sindri across multiple users/teams.

### Per-User Namespace Strategy

```yaml
# Create namespace per user
apiVersion: v1
kind: Namespace
metadata:
  name: dev-alice
  labels:
    user: alice
    team: engineering
```

### Resource Quotas per Namespace

```yaml
apiVersion: v1
kind: ResourceQuota
metadata:
  name: user-quota
  namespace: dev-alice
spec:
  hard:
    requests.cpu: "4"
    requests.memory: "8Gi"
    limits.cpu: "8"
    limits.memory: "16Gi"
    persistentvolumeclaims: "3"
    requests.storage: "100Gi"
```

### Limit Ranges

```yaml
apiVersion: v1
kind: LimitRange
metadata:
  name: default-limits
  namespace: dev-alice
spec:
  limits:
    - default:
        cpu: "2"
        memory: "4Gi"
      defaultRequest:
        cpu: "1"
        memory: "2Gi"
      type: Container
```

### Network Policy (Namespace Isolation)

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: namespace-isolation
  namespace: dev-alice
spec:
  podSelector: {}
  policyTypes:
    - Ingress
    - Egress
  ingress:
    - from:
        - namespaceSelector:
            matchLabels:
              kubernetes.io/metadata.name: dev-alice
  egress:
    - {} # Allow all egress (internet access for development)
```

### RBAC for Users

```yaml
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: sindri-user
  namespace: dev-alice
subjects:
  - kind: User
    name: alice@company.com
    apiGroup: rbac.authorization.k8s.io
roleRef:
  kind: ClusterRole
  name: edit
  apiGroup: rbac.authorization.k8s.io
```
