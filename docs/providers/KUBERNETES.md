# Kubernetes Provider

Enterprise orchestration with Kubernetes.

## Overview

The Kubernetes provider generates manifests (StatefulSet, PVC, Service, Ingress) for deploying Sindri to any Kubernetes cluster.

**Best for:** Enterprise teams, multi-tenant environments, compliance requirements

## Kubernetes Deployment Paths

Sindri supports two paths to Kubernetes deployment:

### Path 1: Direct Kubernetes (This Guide)

- Uses `deployment.provider: kubernetes`
- Generates native K8s manifests (StatefulSet, PVC, Service, Ingress)
- Full control over K8s resources
- Best for: Enterprise teams with existing K8s tooling

### Path 2: DevPod + Kubernetes

- Uses `deployment.provider: devpod` with `type: kubernetes`
- Deploys via DevPod's Kubernetes provider
- DevContainer compatibility, IDE integration
- Best for: VS Code Remote, CI testing, DevContainer workflows
- See [DevPod Provider Guide](DEVPOD.md)

**CI Testing Note:** The CI workflow uses DevPod+K8s (`devpod-k8s` provider) with
auto-provisioned kind clusters for testing. See [Testing Guide](../TESTING.md).

**Example directories:**

| Directory                     | Purpose                              |
| ----------------------------- | ------------------------------------ |
| `examples/devpod/kubernetes/` | DevPod+K8s configs (CI uses these)   |
| `examples/k8s/`               | Local cluster creation + deployment  |

## Prerequisites

- Kubernetes cluster (1.24+)
- kubectl configured with cluster access
- Storage class for persistent volumes
- (Optional) Ingress controller for external access

## Configuration

### Basic Configuration

```yaml
# sindri.yaml
version: 1.0
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
    namespace: dev-environments
```

### Advanced Configuration

```yaml
version: 1.0
name: sindri-k8s

deployment:
  provider: kubernetes
  resources:
    memory: 4GB
    cpus: 2
  volumes:
    workspace:
      size: 30GB

extensions:
  profile: enterprise

providers:
  kubernetes:
    namespace: dev-environments
    storageClass: fast-ssd
    ingressEnabled: true
    ingressHost: sindri.company.com
    ingressClass: nginx
    serviceType: ClusterIP
    nodeSelector:
      workload-type: development
    tolerations:
      - key: "dedicated"
        operator: "Equal"
        value: "dev-environments"
        effect: "NoSchedule"
```

**Generated:** StatefulSet, PVC, Service, Ingress, ConfigMap

## Deployment

### Deploy

```bash
./cli/sindri deploy --provider kubernetes
```

This will:

- Parse sindri.yaml
- Generate Kubernetes manifests
- Create namespace if needed
- Apply manifests to cluster
- Wait for StatefulSet to be ready

### Connect

```bash
# Interactive shell
kubectl exec -it sindri-k8s-0 -n dev-environments -- bash

# As developer user
kubectl exec -it sindri-k8s-0 -n dev-environments -- su - developer

# Port forward for local access
kubectl port-forward sindri-k8s-0 2222:2222 -n dev-environments
ssh developer@localhost -p 2222
```

### Lifecycle Management

```bash
# Scale down (stop)
kubectl scale statefulset sindri-k8s --replicas=0 -n dev-environments

# Scale up (start)
kubectl scale statefulset sindri-k8s --replicas=1 -n dev-environments

# View logs
kubectl logs -f sindri-k8s-0 -n dev-environments

# Describe pod
kubectl describe pod sindri-k8s-0 -n dev-environments

# Delete deployment
kubectl delete -f generated-manifests/ -n dev-environments
```

## Generated Manifests

### StatefulSet

```yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: sindri-k8s
  namespace: dev-environments
spec:
  serviceName: sindri-k8s
  replicas: 1
  selector:
    matchLabels:
      app: sindri-k8s
  template:
    metadata:
      labels:
        app: sindri-k8s
    spec:
      containers:
        - name: sindri
          image: ghcr.io/sindri/sindri:latest
          resources:
            limits:
              memory: "4Gi"
              cpu: "2"
            requests:
              memory: "2Gi"
              cpu: "1"
          volumeMounts:
            - name: workspace
              mountPath: /workspace
          ports:
            - containerPort: 2222
              name: ssh
  volumeClaimTemplates:
    - metadata:
        name: workspace
      spec:
        accessModes: ["ReadWriteOnce"]
        storageClassName: fast-ssd
        resources:
          requests:
            storage: 30Gi
```

### Service

```yaml
apiVersion: v1
kind: Service
metadata:
  name: sindri-k8s
  namespace: dev-environments
spec:
  selector:
    app: sindri-k8s
  ports:
    - port: 2222
      targetPort: 2222
      name: ssh
  type: ClusterIP
```

### Ingress (Optional)

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: sindri-k8s
  namespace: dev-environments
  annotations:
    nginx.ingress.kubernetes.io/backend-protocol: "TCP"
spec:
  ingressClassName: nginx
  rules:
    - host: sindri.company.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: sindri-k8s
                port:
                  number: 2222
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

# View secrets (names only)
kubectl get secret sindri-secrets -n dev-environments -o yaml
```

### Reference in sindri.yaml

```yaml
providers:
  kubernetes:
    secretName: sindri-secrets
```

### External Secrets Operator

For enterprise secret management:

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

## Storage

### Storage Classes

```yaml
providers:
  kubernetes:
    storageClass: fast-ssd # SSD-backed storage
    # storageClass: standard  # Default storage
    # storageClass: premium-rwo  # Premium storage
```

### Volume Size

```yaml
deployment:
  volumes:
    workspace:
      size: 50GB
```

### Volume Operations

```bash
# List PVCs
kubectl get pvc -n dev-environments

# Describe PVC
kubectl describe pvc workspace-sindri-k8s-0 -n dev-environments

# Expand volume (if storage class supports it)
kubectl patch pvc workspace-sindri-k8s-0 -n dev-environments \
  -p '{"spec":{"resources":{"requests":{"storage":"100Gi"}}}}'
```

## Networking

### Service Types

```yaml
providers:
  kubernetes:
    serviceType: ClusterIP # Internal only (default)
    # serviceType: NodePort   # Expose on node port
    # serviceType: LoadBalancer  # Cloud load balancer
```

### Network Policies

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: sindri-network-policy
  namespace: dev-environments
spec:
  podSelector:
    matchLabels:
      app: sindri-k8s
  policyTypes:
    - Ingress
    - Egress
  ingress:
    - from:
        - namespaceSelector:
            matchLabels:
              name: dev-environments
      ports:
        - port: 2222
  egress:
    - {} # Allow all egress
```

## Multi-Tenant Setup

### Per-User Namespaces

```yaml
# user-alice.sindri.yaml
version: 1.0
name: sindri-alice

providers:
  kubernetes:
    namespace: dev-alice
    resourceQuota:
      cpu: "4"
      memory: "8Gi"
      storage: "100Gi"
```

### Resource Quotas

```yaml
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

### Limit Ranges

```yaml
apiVersion: v1
kind: LimitRange
metadata:
  name: sindri-limits
  namespace: dev-environments
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

## Monitoring

### Prometheus Metrics

```yaml
providers:
  kubernetes:
    prometheusEnabled: true
    metricsPort: 9090
```

### Pod Annotations for Scraping

```yaml
template:
  metadata:
    annotations:
      prometheus.io/scrape: "true"
      prometheus.io/port: "9090"
      prometheus.io/path: "/metrics"
```

### Health Checks

```yaml
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
```

## Troubleshooting

### Pod Won't Start

```bash
# Check pod status
kubectl get pods -n dev-environments

# View pod events
kubectl describe pod sindri-k8s-0 -n dev-environments

# Check logs
kubectl logs sindri-k8s-0 -n dev-environments --previous
```

### PVC Pending

```bash
# Check PVC status
kubectl get pvc -n dev-environments

# Check storage class
kubectl get storageclass

# Check events
kubectl describe pvc workspace-sindri-k8s-0 -n dev-environments
```

### Connection Issues

```bash
# Test connectivity
kubectl exec -it sindri-k8s-0 -n dev-environments -- curl -v localhost:2222

# Check service
kubectl get svc -n dev-environments

# Check endpoints
kubectl get endpoints sindri-k8s -n dev-environments
```

### Resource Constraints

```bash
# Check resource usage
kubectl top pod sindri-k8s-0 -n dev-environments

# Check node resources
kubectl describe node | grep -A5 "Allocated resources"

# Scale resources
kubectl patch statefulset sindri-k8s -n dev-environments \
  -p '{"spec":{"template":{"spec":{"containers":[{"name":"sindri","resources":{"limits":{"memory":"8Gi"}}}]}}}}'
```

## Best Practices

1. **Use namespaces** - Isolate environments per user/team
2. **Set resource limits** - Prevent resource contention
3. **Enable network policies** - Secure inter-pod communication
4. **Use storage classes** - Match performance requirements
5. **Implement RBAC** - Limit access per user/role
6. **Monitor resources** - Track usage with Prometheus
7. **Regular backups** - Snapshot PVCs periodically

## Cost Estimates

**Varies by cluster:**

- Managed K8s (GKE, EKS, AKS): ~$70-150/month base cluster cost
- Self-hosted: Infrastructure costs only
- Per-environment: ~$5-20/month (resource share)

**Resource costs (typical cloud pricing):**

- 2 CPU, 4GB RAM: ~$50-80/month
- 30GB SSD storage: ~$5-10/month
- Load balancer: ~$15-25/month

## Related Documentation

- [Deployment Overview](../DEPLOYMENT.md)
- [Configuration Reference](../CONFIGURATION.md)
- [Security](../SECURITY.md)
- [Troubleshooting](../TROUBLESHOOTING.md)
