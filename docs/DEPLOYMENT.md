# Deployment Overview

Guide to deploying Sindri across different providers.

## Deployment Philosophy

Sindri uses a **provider-agnostic architecture** where a single `sindri.yaml` configuration can deploy to multiple platforms:

- **[Docker](providers/DOCKER.md)** - Local development and testing
- **[Fly.io](providers/FLY.md)** - Cloud deployment with auto-suspend and cost optimization
- **[DevPod](providers/DEVPOD.md)** - IDE-integrated containers, multi-cloud, and Kubernetes
- **[E2B](providers/E2B.md)** - Ultra-fast cloud sandboxes for AI development

## Provider Comparison

| Feature                | Docker         | Fly.io                | DevPod                          | E2B                        |
| ---------------------- | -------------- | --------------------- | ------------------------------- | -------------------------- |
| **Best For**           | Local dev      | Individual developers | IDE users, K8s, multi-cloud     | AI sandboxes, prototyping  |
| **Cost**               | Free (local)   | ~$6-50/mo             | Varies by backend               | ~$0.13/hr (pay-per-second) |
| **Setup Time**         | < 1 min        | < 5 min               | < 2 min                         | < 1 sec (after template)   |
| **Auto-Suspend**       | Manual         | Yes                   | Backend-dependent               | Yes (pause/resume)         |
| **Persistent Storage** | Docker volumes | Fly volumes           | Volumes/PVCs                    | Pause snapshots (30 days)  |
| **Remote Access**      | Local only     | SSH/Web               | SSH/VSCode                      | WebSocket PTY              |
| **Scaling**            | Manual         | Auto/Manual           | Backend-dependent               | Per-sandbox                |
| **GPU Support**        | Yes            | Yes                   | Yes                             | No                         |
| **Prerequisites**      | Docker         | flyctl                | DevPod CLI + backend (optional) | E2B CLI + API key          |

**Note:** Kubernetes deployment is handled via DevPod with `type: kubernetes`.
See [Kubernetes Deployment Guide](providers/KUBERNETES.md).

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
  provider: devpod # Use DevPod as the deployment method

providers:
  devpod:
    type: kubernetes # Target Kubernetes as the backend
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
  provider: fly # docker | fly | devpod | e2b

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
# Universal (auto-detects provider)
./cli/sindri connect

# Or provider-specific commands:
# Docker: docker exec -it my-dev-env /docker/scripts/entrypoint.sh /bin/bash
# Fly.io: ssh developer@my-dev-env.fly.dev -p 10022
# DevPod: devpod ssh my-dev-env
```

### 6. Check Status

```bash
./cli/sindri status
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

### DevPod

**Use when:**

- VS Code is primary editor
- Using GitHub Codespaces
- Want IDE-native container experience
- Need DevContainer compatibility
- Deploying to Kubernetes clusters
- Using cloud providers (AWS, GCP, Azure, DigitalOcean)

[DevPod Provider Guide](providers/DEVPOD.md)

**For Kubernetes specifically:** See [Kubernetes Deployment Guide](providers/KUBERNETES.md)

### E2B

**Use when:**

- Need ultra-fast startup (~150ms)
- Building AI agents that need isolated sandboxes
- Want pay-per-second pricing
- Working behind corporate firewalls (WebSocket access)
- Need rapid prototyping environments
- Don't require GPU or SSH access

**Not recommended when:**

- Need GPU acceleration
- Require VS Code Remote SSH
- Need persistent storage beyond 30 days
- Prefer SSH access over WebSocket

[E2B Provider Guide](providers/E2B.md)

## Secrets Management

Each provider handles secrets differently:

| Provider   | Method                   | Example                                    |
| ---------- | ------------------------ | ------------------------------------------ |
| Docker     | `.env` file              | `ANTHROPIC_API_KEY=sk-ant-...`             |
| Fly.io     | `flyctl secrets`         | `flyctl secrets set ANTHROPIC_API_KEY=...` |
| Kubernetes | Kubernetes secrets       | `kubectl create secret generic ...`        |
| DevPod     | IDE settings/environment | VS Code settings or environment variables  |
| E2B        | Environment variables    | Injected at sandbox creation               |

See provider-specific guides for details.

## Lifecycle Management

### Common Commands

```bash
# Check status (any provider)
./cli/sindri status

# Connect to environment (any provider)
./cli/sindri connect

# Show deployment plan (dry-run)
./cli/sindri plan

# Teardown (any provider)
./cli/sindri destroy         # With confirmation
./cli/sindri destroy --force # Skip confirmation
```

### Provider-Specific Commands

**Docker:**

```bash
docker start my-dev-env
docker stop my-dev-env
docker compose down -v  # Removes volumes
```

**Fly.io:**

```bash
flyctl status -a my-app
flyctl machine start <id> -a my-app
flyctl machine stop <id> -a my-app
flyctl apps destroy my-app
```

**DevPod:**

```bash
devpod status my-dev-env
devpod stop my-dev-env
devpod delete my-dev-env
```

**E2B:**

```bash
# Pause sandbox (preserve state)
./cli/sindri pause

# E2B CLI commands
e2b sandbox list
e2b sandbox terminal <sandbox-id>
e2b sandbox pause <sandbox-id>
e2b sandbox kill <sandbox-id>
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
- **AI sandboxes/rapid prototyping:** E2B

All use the same base image and extension system.

## Related Documentation

- [Quickstart](QUICKSTART.md)
- [Configuration Reference](CONFIGURATION.md)
- [Troubleshooting](TROUBLESHOOTING.md)

### Provider Guides

- [Docker](providers/DOCKER.md)
- [Fly.io](providers/FLY.md)
- [DevPod](providers/DEVPOD.md)
- [E2B](providers/E2B.md)
- [Kubernetes (via DevPod)](providers/KUBERNETES.md)
