# Deployment Overview

Guide to deploying Sindri across different providers.

## Deployment Philosophy

Sindri uses a **provider-agnostic architecture** where a single `sindri.yaml` configuration can deploy to multiple platforms:

- **[Docker](providers/DOCKER.md)** - Local development and testing
- **[Fly.io](providers/FLY.md)** - Cloud deployment with auto-suspend and cost optimization
- **[Kubernetes](providers/KUBERNETES.md)** - Enterprise orchestration
- **[DevPod](providers/DEVPOD.md)** - IDE-integrated containers (VS Code, Codespaces)

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

## DevPod Multi-Backend Architecture

DevPod is unique - it's a **meta-provider** that can deploy to multiple backends:

| DevPod Backend | sindri.yaml Config   | CI Provider Name | Use Case           |
| -------------- | -------------------- | ---------------- | ------------------ |
| Docker (local) | `type: docker`       | N/A              | Local development  |
| AWS EC2        | `type: aws`          | `devpod-aws`     | Cloud dev on AWS   |
| GCP Compute    | `type: gcp`          | `devpod-gcp`     | Cloud dev on GCP   |
| Azure VMs      | `type: azure`        | `devpod-azure`   | Cloud dev on Azure |
| DigitalOcean   | `type: digitalocean` | `devpod-do`      | Budget cloud dev   |
| Kubernetes     | `type: kubernetes`   | `devpod-k8s`     | K8s pod-based dev  |
| SSH Host       | `type: ssh`          | `devpod-ssh`     | Any SSH server     |

**Example configuration:**

```yaml
deployment:
  provider: devpod        # Use DevPod as the deployment method

providers:
  devpod:
    type: kubernetes      # Target Kubernetes as the backend
    kubernetes:
      namespace: sindri-dev
      storageClass: standard
```

**CI Testing Note:** When using `devpod-k8s` in CI without a `KUBECONFIG` secret,
a local kind cluster is automatically created for testing.

## Quick Start

### 1. Initialize Configuration

```bash
./cli/sindri config init
```

### 2. Edit Configuration

```yaml
# sindri.yaml
version: 1.0
name: my-dev-env

deployment:
  provider: fly # docker | fly | kubernetes | devpod

extensions:
  profile: fullstack
```

### 3. Validate

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
# Docker
docker exec -it my-dev-env bash

# Fly.io
ssh developer@my-dev-env.fly.dev -p 10022

# Kubernetes
kubectl exec -it my-dev-env-0 -n dev-envs -- bash

# DevPod
devpod ssh .
```

## Choosing a Provider

### Docker

**Use when:**

- Testing locally before cloud deployment
- Working offline
- Running CI/CD pipelines
- Zero cost is required

[Docker Provider Guide](providers/DOCKER.md)

### Fly.io

**Use when:**

- Need remote access from anywhere
- Want cost-optimized cloud deployment
- Individual developer or small team
- Auto-suspend/resume is valuable

[Fly.io Provider Guide](providers/FLY.md)

### Kubernetes

**Use when:**

- Enterprise compliance requirements
- Multi-tenant environments
- Need horizontal scaling
- Existing cluster infrastructure

[Kubernetes Provider Guide](providers/KUBERNETES.md)

### DevPod

**Use when:**

- VS Code is primary editor
- Using GitHub Codespaces
- Want IDE-native container experience
- Need DevContainer compatibility

[DevPod Provider Guide](providers/DEVPOD.md)

## Secrets Management

Each provider handles secrets differently:

| Provider   | Method                   | Example                                    |
| ---------- | ------------------------ | ------------------------------------------ |
| Docker     | `.env` file              | `ANTHROPIC_API_KEY=sk-ant-...`             |
| Fly.io     | `flyctl secrets`         | `flyctl secrets set ANTHROPIC_API_KEY=...` |
| Kubernetes | Kubernetes secrets       | `kubectl create secret generic ...`        |
| DevPod     | IDE settings/environment | VS Code settings or environment variables  |

See provider-specific guides for details.

## Lifecycle Management

### Start/Stop

```bash
# Docker
docker start my-dev-env
docker stop my-dev-env

# Fly.io (automatic with auto-suspend)
flyctl machine start <id> -a my-app
flyctl machine stop <id> -a my-app

# Kubernetes
kubectl scale statefulset my-dev-env --replicas=0 -n dev-envs
kubectl scale statefulset my-dev-env --replicas=1 -n dev-envs
```

### Teardown

```bash
# Docker
docker compose down -v

# Fly.io
flyctl apps destroy my-app

# Kubernetes
kubectl delete namespace dev-envs

# DevPod
devpod delete .
```

## Migration Between Providers

Sindri's volume architecture makes migration straightforward:

1. Export workspace:

   ```bash
   tar -czf workspace-backup.tar.gz /workspace
   ```

2. Deploy to new provider:

   ```bash
   ./cli/sindri deploy --provider fly
   ```

3. Restore workspace:

   ```bash
   tar -xzf workspace-backup.tar.gz -C /workspace
   ```

All extensions and configurations are preserved.

## Hybrid Deployment

Use different providers for different purposes:

- **Local development:** Docker
- **Remote collaboration:** Fly.io
- **Production workloads:** Kubernetes
- **IDE integration:** DevPod

All use the same base image and extension system.

## Related Documentation

- [Quickstart](QUICKSTART.md)
- [Configuration Reference](CONFIGURATION.md)
- [Troubleshooting](TROUBLESHOOTING.md)

### Provider Guides

- [Docker](providers/DOCKER.md)
- [Fly.io](providers/FLY.md)
- [Kubernetes](providers/KUBERNETES.md)
- [DevPod](providers/DEVPOD.md)
