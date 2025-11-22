# Deployment Overview

Guide to deploying Sindri across different providers.

## Deployment Philosophy

Sindri uses a **provider-agnostic architecture** where a single `sindri.yaml` configuration can deploy to multiple platforms:

- **Docker** - Local development and testing
- **Fly.io** - Cloud deployment with auto-suspend and cost optimization
- **Kubernetes** - Enterprise orchestration
- **DevPod** - IDE-integrated containers (VS Code, Codespaces)

## Provider Comparison

| Feature                | Docker         | Fly.io                | Kubernetes       | DevPod       |
| ---------------------- | -------------- | --------------------- | ---------------- | ------------ |
| **Best For**           | Local dev      | Individual developers | Enterprise/teams | IDE users    |
| **Cost**               | Free (local)   | ~$6-50/mo             | Variable         | Free (local) |
| **Setup Time**         | < 1 min        | < 5 min               | 10-30 min        | < 2 min      |
| **Auto-Suspend**       | Manual         | Yes                   | Via HPA          | Manual       |
| **Persistent Storage** | Docker volumes | Fly volumes           | PVCs             | Volumes      |
| **Remote Access**      | Local only     | SSH/Web               | Ingress          | SSH/VSCode   |
| **Scaling**            | Manual         | Auto/Manual           | Auto             | Manual       |
| **Prerequisites**      | Docker         | flyctl                | kubectl, cluster | DevPod CLI   |

## Quick Deployment Comparison

### Docker (Local)

**Best for:** Testing, offline development, CI/CD

```bash
./cli/sindri deploy --provider docker
```

**Pros:**

- Instant startup
- No cost
- Works offline
- Full control

**Cons:**

- Local resources only
- No remote access
- Manual lifecycle management

### Fly.io (Cloud)

**Best for:** Remote development, team collaboration, cost-optimized cloud

```bash
./cli/sindri deploy --provider fly
```

**Pros:**

- Auto-suspend (pay per use)
- SSH access anywhere
- Persistent volumes
- Fast deployment

**Cons:**

- Requires internet
- Monthly cost (~$6-50)
- Single-region by default

See: [Fly.io Deployment Guide](FLY_DEPLOYMENT.md)

### Kubernetes (Enterprise)

**Best for:** Large teams, multi-tenant, enterprise compliance

```bash
./cli/sindri deploy --provider kubernetes
```

**Pros:**

- Multi-tenant isolation
- Advanced networking
- Enterprise features
- Horizontal scaling

**Cons:**

- Complex setup
- Requires cluster
- Higher operational overhead

### DevPod (IDE Integration)

**Best for:** VS Code users, GitHub Codespaces, local IDE containers

```bash
./cli/sindri deploy --provider devpod
```

**Pros:**

- Native VS Code integration
- Codespaces support
- IDE features preserved
- Local or remote

**Cons:**

- VS Code focused
- Limited to DevContainer spec
- Manual lifecycle

See: [DevPod Integration Guide](DEVPOD_INTEGRATION.md)

## Deployment Workflow

### 1. Initialize Configuration

```bash
# Create sindri.yaml
./cli/sindri config init

# Or copy from examples
cp examples/fly-minimal.sindri.yaml sindri.yaml
```

### 2. Edit Configuration

```yaml
# sindri.yaml
version: 1.0
name: my-dev-env

deployment:
  provider: fly # Choose provider

extensions:
  profile: fullstack
```

See: [Configuration Reference](CONFIGURATION.md)

### 3. Validate Configuration

```bash
./cli/sindri config validate
```

### 4. Deploy

```bash
# Use provider from config
./cli/sindri deploy

# Or override provider
./cli/sindri deploy --provider docker
```

### 5. Connect

```bash
# For Docker
docker exec -it my-dev-env bash

# For Fly.io
ssh developer@my-dev-env.fly.dev -p 10022

# For Kubernetes
kubectl exec -it my-dev-env-0 -n dev-envs -- bash

# For DevPod
devpod ssh .
```

## Provider-Specific Configuration

### Docker Configuration

```yaml
version: 1.0
name: sindri-docker

deployment:
  provider: docker
  resources:
    memory: 2GB
    cpus: 1

extensions:
  profile: minimal

providers:
  docker:
    ports:
      - "3000:3000"
      - "8080:8080"
```

**Generated:** `docker-compose.yml`

### Fly.io Configuration

```yaml
version: 1.0
name: sindri-fly

deployment:
  provider: fly
  resources:
    memory: 2GB
    cpus: 1
  volumes:
    workspace:
      size: 10GB

extensions:
  profile: fullstack

providers:
  fly:
    region: sjc
    autoStopMachines: true
    autoStartMachines: true
```

**Generated:** `fly.toml`

See: [Fly.io Deployment Guide](FLY_DEPLOYMENT.md)

### Kubernetes Configuration

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
```

**Generated:** Kubernetes manifests (StatefulSet, PVC, Service, Ingress)

### DevPod Configuration

```yaml
version: 1.0
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
    vscodeExtensions:
      - ms-python.python
      - golang.go
    forwardPorts:
      - 3000
      - 8080
```

**Generated:** `.devcontainer/devcontainer.json`

## Secrets Management

Secrets are handled differently by each provider:

### Docker

Use `.env` file (not committed):

```bash
# .env
ANTHROPIC_API_KEY=sk-ant-...
GITHUB_TOKEN=ghp_...
```

Reference in `docker-compose.yml`:

```yaml
services:
  sindri:
    env_file: .env
```

### Fly.io

Use Fly secrets:

```bash
flyctl secrets set ANTHROPIC_API_KEY=sk-ant-... -a my-app
flyctl secrets set GITHUB_TOKEN=ghp_... -a my-app
```

### Kubernetes Secrets

Use Kubernetes secrets:

```bash
kubectl create secret generic sindri-secrets \
  --from-literal=ANTHROPIC_API_KEY=sk-ant-... \
  --from-literal=GITHUB_TOKEN=ghp_... \
  --namespace=dev-envs
```

### DevPod Secrets

Handled by IDE settings or environment.

## Cost Considerations

### Docker (Local) Costs

**Cost:** $0 (uses local resources)

**Resource Requirements:**

- 2-8 GB RAM recommended
- 10-50 GB disk space
- 1-2 CPU cores

### Fly.io (Cloud) Costs

**Minimal Setup:**

- 1 GB RAM, 1 vCPU: ~$5-10/month
- 10 GB volume: ~$1.50/month
- **Total: ~$6.50-11.50/month**

**Standard Setup:**

- 2 GB RAM, 1 vCPU: ~$10-15/month
- 30 GB volume: ~$4.50/month
- **Total: ~$14.50-19.50/month**

**With auto-suspend:** Costs scale with usage time.

See: [Fly.io Deployment Guide](FLY_DEPLOYMENT.md) for detailed pricing.

### Kubernetes Costs

**Varies by cluster:**

- Managed K8s (GKE, EKS, AKS): ~$70-150/month base
- Self-hosted: Infrastructure costs only
- Per-environment: ~$5-20/month (resources)

### DevPod Costs

**Cost:** $0 (local) or Codespaces pricing (~$0.18/hour)

## Lifecycle Management

### Start/Stop

```bash
# Docker
docker start my-dev-env
docker stop my-dev-env

# Fly.io (automatic with auto-suspend)
flyctl machine start <machine-id> -a my-app
flyctl machine stop <machine-id> -a my-app

# Kubernetes
kubectl scale statefulset my-dev-env --replicas=0 -n dev-envs
kubectl scale statefulset my-dev-env --replicas=1 -n dev-envs
```

### Update Configuration

```bash
# Edit sindri.yaml
vim sindri.yaml

# Redeploy
./cli/sindri deploy
```

### Teardown

```bash
# Docker
docker-compose down -v

# Fly.io
flyctl apps destroy my-app

# Kubernetes
kubectl delete namespace dev-envs

# DevPod
devpod delete .
```

## Multi-Environment Strategy

Deploy multiple environments from one repo:

```bash
# Production
cp examples/fly-production.sindri.yaml sindri.yaml
./cli/sindri deploy

# Development
cp examples/fly-minimal.sindri.yaml sindri-dev.yaml
./cli/sindri deploy --config sindri-dev.yaml
```

## Hybrid Deployment

Use different providers for different purposes:

- **Local development:** Docker
- **Remote collaboration:** Fly.io
- **Production workloads:** Kubernetes
- **IDE integration:** DevPod

All use the same base image and extension system!

## Troubleshooting

### Connection Issues

```bash
# Check deployment status
./cli/sindri status

# View logs
./cli/sindri logs

# Restart
./cli/sindri restart
```

### Resource Constraints

Increase resources in `sindri.yaml`:

```yaml
deployment:
  resources:
    memory: 4GB # Increase from 2GB
    cpus: 2 # Increase from 1
```

Then redeploy:

```bash
./cli/sindri deploy
```

### Volume Issues

Check volume status:

```bash
# Docker
docker volume ls
docker volume inspect sindri-workspace

# Fly.io
flyctl volumes list -a my-app

# Kubernetes
kubectl get pvc -n dev-envs
```

## Migration Between Providers

Sindri's volume architecture makes migration straightforward:

1. Export workspace data:

   ```bash
   tar -czf workspace-backup.tar.gz /workspace
   ```

2. Deploy to new provider:

   ```bash
   # Update provider in sindri.yaml
   ./cli/sindri deploy --provider fly
   ```

3. Restore workspace:

   ```bash
   tar -xzf workspace-backup.tar.gz -C /workspace
   ```

All extensions and configurations are preserved!

## Best Practices

1. **Start Local** - Test with Docker before deploying to cloud
2. **Use Profiles** - Leverage extension profiles for consistency
3. **Version Control** - Commit `sindri.yaml` to version control
4. **Secrets Externally** - Never commit secrets to `sindri.yaml`
5. **Monitor Costs** - Use auto-suspend on Fly.io, set resource limits
6. **Regular Backups** - Snapshot volumes periodically
7. **Test Migrations** - Validate before switching providers

## Related Documentation

- [Quickstart](QUICKSTART.md)
- [Configuration Reference](CONFIGURATION.md)
- [Fly.io Deployment](FLY_DEPLOYMENT.md)
- [DevPod Integration](DEVPOD_INTEGRATION.md)
- [Troubleshooting](TROUBLESHOOTING.md)
